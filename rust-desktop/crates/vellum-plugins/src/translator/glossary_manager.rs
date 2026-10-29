use std::collections::BTreeMap;

use super::llm_client::LlmTranslationClient;

#[derive(Debug, Default, Clone)]
pub struct GlossaryManager {
    terms: BTreeMap<String, String>,
}

impl GlossaryManager {
    pub fn new() -> Self {
        Self {
            terms: BTreeMap::new(),
        }
    }

    pub fn load_from_config(&mut self, raw_glossary: &str) {
        for line in raw_glossary.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if let Some((src, dst)) = trimmed
                .split_once("->")
                .or_else(|| trimmed.split_once('='))
                .or_else(|| trimmed.split_once(':'))
            {
                let s = src.trim();
                let d = dst.trim();
                if !s.is_empty() && !d.is_empty() {
                    self.terms.insert(s.to_string(), d.to_string());
                }
            }
        }
    }

    pub fn extract_initial_glossary(
        &mut self,
        llm: &LlmTranslationClient,
        sample_paragraphs: &[String],
        target_language: &str,
    ) {
        if sample_paragraphs.is_empty() {
            return;
        }
        let sample_count = sample_paragraphs.len().min(15);
        let sample = sample_paragraphs[..sample_count].join("\n\n");

        let system_prompt = format!(
            "Tu es un terminologue littéraire.\n\
             Extrait jusqu'à 20 termes récurrents importants (noms de personnages, lieux fictifs, \
             titres, concepts spécifiques, objets récurrents) de cet extrait et propose leur équivalent \
             recommandé en {target_language} (ou indique le même nom s'il doit rester invariant).\n\
             Réponds UNIQUEMENT sous la forme d'une liste avec une entrée par ligne au format :\n\
             Terme Source -> Terme Cible"
        );

        if let Ok(resp) = llm.generate_completion(&system_prompt, &sample) {
            self.load_from_config(&resp);
        }
    }

    pub fn put_term(&mut self, source: &str, target: &str) {
        let s = source.trim();
        let t = target.trim();
        if !s.is_empty() && !t.is_empty() {
            self.terms.insert(s.to_string(), t.to_string());
        }
    }

    pub fn terms(&self) -> &BTreeMap<String, String> {
        &self.terms
    }

    pub fn format_for_prompt(&self, chapter_text: &str) -> String {
        if self.terms.is_empty() {
            return String::new();
        }
        let lower_chapter = chapter_text.to_lowercase();
        let mut matching = Vec::new();

        for (src, dst) in &self.terms {
            if chapter_text.is_empty() || lower_chapter.contains(&src.to_lowercase()) {
                matching.push(format!("- {src} -> {dst}"));
            }
        }

        if matching.is_empty() {
            for (src, dst) in self.terms.iter().take(15) {
                matching.push(format!("- {src} -> {dst}"));
            }
        }

        format!("GLOSSAIRE TERMINOLOGIQUE OBLIGATOIRE :\n{}", matching.join("\n"))
    }
}
