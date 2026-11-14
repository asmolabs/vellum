use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use regex::Regex;
use vellum_domain::{VellumError, VellumResult};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use super::charter_planner::EditorialCharterPlanner;
use super::glossary_manager::GlossaryManager;
use super::html_parser::{
    build_bilingual_chapter, extract_paragraphs, rebuild_chapter_html, restore_inline_tags,
};
use super::llm_client::{parse_numbered_response, LlmTranslationClient};
use super::qa_validator::QualityAssuranceValidator;
use super::segment_cache::TranslationSegmentCache;

#[derive(Debug, Default, Clone)]
pub struct PipelineStats {
    pub chapters_processed: usize,
    pub paragraphs_translated: usize,
    pub cache_hits: usize,
    pub qa_warnings: usize,
}

pub fn build_llm_client(config: &HashMap<String, String>) -> LlmTranslationClient {
    let provider = config
        .get("provider")
        .map(String::as_str)
        .unwrap_or("Ollama (Local - Gratuit)");
    let base_url = config
        .get("baseUrl")
        .map(String::as_str)
        .unwrap_or("http://localhost:11434");
    let model = config
        .get("model")
        .map(String::as_str)
        .unwrap_or("qwen2.5:14b");
    let api_key = config.get("apiKey").cloned();
    let temperature = config
        .get("temperature")
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.25);

    LlmTranslationClient::new(provider, base_url, model, api_key, temperature)
}

pub fn translate_epub(
    input_file: &Path,
    output_file: &Path,
    config: &HashMap<String, String>,
) -> VellumResult<PipelineStats> {
    let llm = build_llm_client(config);
    let target_lang = config
        .get("targetLanguage")
        .map(String::as_str)
        .unwrap_or("Français");
    let bilingual_mode = config
        .get("bilingualMode")
        .map(|s| s.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let style_notes = config.get("styleNotes").map(String::as_str).unwrap_or("");
    let max_chapters = config
        .get("maxChapters")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0);
    let enable_charter = config
        .get("enableCharter")
        .map(|s| !s.eq_ignore_ascii_case("false"))
        .unwrap_or(true);
    let enable_qa = config
        .get("enableQa")
        .map(|s| !s.eq_ignore_ascii_case("false"))
        .unwrap_or(true);
    let enable_cache = config
        .get("enableCache")
        .map(|s| !s.eq_ignore_ascii_case("false"))
        .unwrap_or(true);

    let cache_dir = if enable_cache {
        Some(
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".vellum")
                .join("translation-cache"),
        )
    } else {
        None
    };

    let model_key = config
        .get("model")
        .map(String::as_str)
        .unwrap_or("qwen2.5:14b");
    let cache = TranslationSegmentCache::new(cache_dir, model_key, target_lang);

    let mut glossary = GlossaryManager::new();
    if let Some(raw_glossary) = config.get("glossary") {
        glossary.load_from_config(raw_glossary);
    }

    // 1. Read all entries from source EPUB
    let src_file = File::open(input_file)?;
    let mut archive = ZipArchive::new(src_file).map_err(|e| VellumError::Zip(e.to_string()))?;

    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
    let mut sample_paragraphs: Vec<String> = Vec::new();

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| VellumError::Zip(e.to_string()))?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().map(|n| n.to_string()).unwrap_or_default();
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf)?;

        if is_translatable_xhtml(&name) && sample_paragraphs.len() < 15 {
            if let Ok(html) = String::from_utf8(buf.clone()) {
                for p in extract_paragraphs(&html) {
                    if sample_paragraphs.len() < 15 {
                        sample_paragraphs.push(p.marked_text);
                    }
                }
            }
        }
        entries.push((name, buf));
    }

    // 2. Phase 1: Pre-translation Editorial Charter & Initial Glossary
    let charter = if enable_charter {
        EditorialCharterPlanner::generate_editorial_charter(
            &llm,
            &sample_paragraphs,
            target_lang,
            style_notes,
        )
        .unwrap_or_else(|_| EditorialCharterPlanner::default_charter(target_lang, style_notes))
    } else {
        EditorialCharterPlanner::default_charter(target_lang, style_notes)
    };

    if glossary.terms().is_empty() && enable_charter {
        glossary.extract_initial_glossary(&llm, &sample_paragraphs, target_lang);
    }

    let mut stats = PipelineStats::default();
    let mut rolling_context: Vec<String> = Vec::new();

    if let Some(parent) = output_file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let out_file = File::create(output_file)?;
    let mut writer = ZipWriter::new(out_file);

    // Write mimetype first if present
    if let Some((_, mime_bytes)) = entries.iter().find(|(n, _)| n == "mimetype") {
        let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        writer
            .start_file("mimetype", stored)
            .map_err(|e| VellumError::Zip(e.to_string()))?;
        writer.write_all(mime_bytes)?;
    }

    let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let lang_re =
        Regex::new(r"(?is)(<dc:language[^>]*>)(.*?)(</dc:language>)").expect("valid regex");

    for (name, bytes) in entries {
        if name == "mimetype" {
            continue;
        }

        writer
            .start_file(&name, deflated)
            .map_err(|e| VellumError::Zip(e.to_string()))?;

        if name.to_lowercase().ends_with(".opf") {
            if let Ok(opf_str) = String::from_utf8(bytes.clone()) {
                let lang_code = language_to_iso_code(target_lang);
                let updated_opf = lang_re
                    .replace_all(&opf_str, format!("${{1}}{lang_code}${{3}}"))
                    .into_owned();
                writer.write_all(updated_opf.as_bytes())?;
            } else {
                writer.write_all(&bytes)?;
            }
            continue;
        }

        if !is_translatable_xhtml(&name)
            || (max_chapters > 0 && stats.chapters_processed >= max_chapters)
        {
            writer.write_all(&bytes)?;
            continue;
        }

        let Ok(html) = String::from_utf8(bytes.clone()) else {
            writer.write_all(&bytes)?;
            continue;
        };

        let paragraphs = extract_paragraphs(&html);
        if paragraphs.is_empty() {
            writer.write_all(&bytes)?;
            continue;
        }

        stats.chapters_processed += 1;

        let mut translated_inners = vec![String::new(); paragraphs.len()];
        let mut uncached_indices: Vec<usize> = Vec::new();

        for (idx, p) in paragraphs.iter().enumerate() {
            let cached_opt = if enable_cache {
                cache.get(&p.marked_text)
            } else {
                None
            };

            if let Some(cached_marked) = cached_opt {
                stats.cache_hits += 1;
                translated_inners[idx] = restore_inline_tags(&cached_marked, &p.inline_tags);
                rolling_context.push(cached_marked);
                if rolling_context.len() > 3 {
                    rolling_context.remove(0);
                }
            } else {
                uncached_indices.push(idx);
            }
        }

        // Batch translate uncached paragraphs in chunks of 8
        for batch in uncached_indices.chunks(8) {
            let mut batch_prompt = String::new();
            for (local_i, &p_idx) in batch.iter().enumerate() {
                let num = local_i + 1;
                batch_prompt.push_str(&format!("[{num}] {}\n\n", paragraphs[p_idx].marked_text));
            }

            let glossary_section = glossary.format_for_prompt(&batch_prompt);
            let context_section = if rolling_context.is_empty() {
                String::new()
            } else {
                format!(
                    "CONTEXTE PRÉCÉDENT (pour continuité narrative, NE PAS RETRADUIRE) :\n{}\n",
                    rolling_context.join("\n")
                )
            };

            let system_prompt = format!(
                "Tu es un traducteur littéraire professionnel vers le {target_lang}.\n\
                 {charter}\n\n\
                 {glossary_section}\n\n\
                 {context_section}\n\
                 RÈGLES STRICTES DE FORMATAGE :\n\
                 1. Traduis chaque paragraphe numéroté [1], [2], etc. en {target_lang}.\n\
                 2. Conserve impérativement les numéros [1], [2]... au début de chaque paragraphe traduit.\n\
                 3. Conserve tous les marqueurs inline {{0}}...{{/0}}, {{1}}...{{/1}} exactement autour des mots correspondants.\n\
                 4. Ne rajoute aucun commentaire ni introduction."
            );

            let raw_response = llm.chat(&system_prompt, &batch_prompt).unwrap_or_default();
            let parsed = parse_numbered_response(&raw_response, batch.len());

            for (local_i, &p_idx) in batch.iter().enumerate() {
                let source_marked = &paragraphs[p_idx].marked_text;
                let mut candidate = parsed
                    .get(local_i)
                    .cloned()
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or_else(|| source_marked.clone());

                if enable_qa {
                    let issues = QualityAssuranceValidator::validate_paragraph(
                        p_idx,
                        source_marked,
                        &candidate,
                        Some(&glossary),
                    );
                    if !issues.is_empty() {
                        stats.qa_warnings += issues.len();
                        candidate = QualityAssuranceValidator::repair_missing_markers(
                            source_marked,
                            &candidate,
                        );
                    }
                }

                if enable_cache && candidate != *source_marked {
                    cache.put(source_marked, &candidate);
                }

                rolling_context.push(candidate.clone());
                if rolling_context.len() > 3 {
                    rolling_context.remove(0);
                }

                translated_inners[p_idx] =
                    restore_inline_tags(&candidate, &paragraphs[p_idx].inline_tags);
                stats.paragraphs_translated += 1;
            }
        }

        let new_html = if bilingual_mode {
            build_bilingual_chapter(&html, &paragraphs, &translated_inners)
        } else {
            rebuild_chapter_html(&html, &paragraphs, &translated_inners)
        };

        writer.write_all(new_html.as_bytes())?;
    }

    writer
        .finish()
        .map_err(|e| VellumError::Zip(e.to_string()))?;
    Ok(stats)
}

fn is_translatable_xhtml(name: &str) -> bool {
    let lower = name.to_lowercase();
    if !(lower.ends_with(".xhtml") || lower.ends_with(".html") || lower.ends_with(".htm")) {
        return false;
    }
    let file_part = lower.rsplit('/').next().unwrap_or(&lower);
    !(file_part.contains("nav") || file_part.contains("toc") || file_part.contains("cover"))
}

fn language_to_iso_code(target_language: &str) -> &'static str {
    let lower = target_language.to_lowercase();
    if lower.starts_with("fran") || lower == "fr" {
        "fr"
    } else if lower.starts_with("angl") || lower.starts_with("engl") || lower == "en" {
        "en"
    } else if lower.starts_with("esp") || lower.starts_with("span") || lower == "es" {
        "es"
    } else if lower.starts_with("all") || lower.starts_with("deut") || lower == "de" {
        "de"
    } else if lower.starts_with("ital") || lower == "it" {
        "it"
    } else if lower.starts_with("port") || lower == "pt" {
        "pt"
    } else if lower.starts_with("jap") || lower == "ja" {
        "ja"
    } else {
        "fr"
    }
}
