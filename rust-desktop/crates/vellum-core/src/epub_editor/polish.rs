use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use regex::Regex;
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;
use vellum_domain::{Book, VellumError, VellumResult};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use super::{escape_xml, map_zip, EpubEditor};

/// Options pour l'opération « Polish Book » sur un EPUB.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolishOptions {
    pub embed_metadata: bool,
    pub embed_cover: bool,
    pub smarten_punctuation: bool,
    pub repair_structure: bool,
    pub rebuild_toc_if_empty: bool,
}

impl Default for PolishOptions {
    fn default() -> Self {
        Self {
            embed_metadata: true,
            embed_cover: true,
            smarten_punctuation: true,
            repair_structure: true,
            rebuild_toc_if_empty: true,
        }
    }
}

/// Rapport détaillant les actions effectuées par « Polish Book ».
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PolishReport {
    pub metadata_updated: bool,
    pub cover_embedded: bool,
    pub chapters_smartened: usize,
    pub toc_rebuilt: bool,
    pub repaired: bool,
}

impl EpubEditor {
    /// Exécute l'opération « Polish Book » sur un fichier EPUB (métadonnées, couverture, typographie, TOC, réparation).
    pub fn polish(
        epub_path: &Path,
        book: &Book,
        cover_path: Option<&Path>,
        options: PolishOptions,
    ) -> VellumResult<PolishReport> {
        let mut report = PolishReport::default();
        let lacked_toc = if options.rebuild_toc_if_empty {
            let d = Self::inspect(epub_path)?;
            d.toc_entries_count == 0
        } else {
            false
        };

        if options.repair_structure {
            let _ = Self::clean_and_repair(epub_path)?;
            report.repaired = true;
        }

        if options.rebuild_toc_if_empty {
            let d = Self::inspect(epub_path)?;
            if d.toc_entries_count == 0 {
                let generated = Self::generate_toc_from_headings(epub_path).unwrap_or_default();
                if !generated.is_empty() {
                    Self::save_toc(epub_path, &generated)?;
                    report.toc_rebuilt = true;
                }
            } else if lacked_toc {
                report.toc_rebuilt = true;
            }
        }

        let need_zip_rewrite =
            options.embed_metadata || options.embed_cover || options.smarten_punctuation;
        if !need_zip_rewrite {
            return Ok(report);
        }

        let cover_bytes_and_ext: Option<(Vec<u8>, &'static str, &'static str)> =
            if options.embed_cover {
                if let Some(cp) = cover_path {
                    if cp.exists() {
                        let bytes = std::fs::read(cp)?;
                        let ext = cp
                            .extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("jpg")
                            .to_lowercase();
                        if ext == "png" {
                            Some((bytes, "png", "image/png"))
                        } else {
                            Some((bytes, "jpg", "image/jpeg"))
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

        let file = File::open(epub_path)?;
        let mut zip = map_zip(ZipArchive::new(file))?;
        let opf_path = Self::find_opf_path(&mut zip)?;
        let opf_dir = Path::new(&opf_path)
            .parent()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();

        let cover_rel_href = cover_bytes_and_ext
            .as_ref()
            .map(|(_, ext, _)| format!("images/vellum-cover.{ext}"));
        let cover_zip_path = cover_rel_href.as_ref().map(|href| {
            if opf_dir.is_empty() {
                href.clone()
            } else {
                format!("{opf_dir}/{href}")
            }
        });

        let mut entries: Vec<(String, Vec<u8>)> = Vec::with_capacity(zip.len());
        for i in 0..zip.len() {
            let mut entry = map_zip(zip.by_index(i))?;
            if entry.is_dir() {
                continue;
            }
            let name = entry.name().map(|n| n.to_string()).unwrap_or_default();
            if cover_zip_path.as_deref() == Some(name.as_str()) {
                continue;
            }
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;

            if name == opf_path && (options.embed_metadata || cover_bytes_and_ext.is_some()) {
                let mut xml = String::from_utf8_lossy(&buf).to_string();
                if options.embed_metadata {
                    xml = inject_opf_metadata(&xml, book);
                    report.metadata_updated = true;
                }
                if let (Some(href), Some((_, _, mime))) =
                    (cover_rel_href.as_deref(), cover_bytes_and_ext.as_ref())
                {
                    xml = inject_opf_cover(&xml, href, mime);
                    report.cover_embedded = true;
                }
                buf = xml.into_bytes();
            } else if options.smarten_punctuation && is_xhtml_entry(&name) {
                let original = String::from_utf8_lossy(&buf).to_string();
                let smartened = smarten_xhtml_prose(&original);
                if smartened != original {
                    report.chapters_smartened += 1;
                    buf = smartened.into_bytes();
                }
            }

            entries.push((name, buf));
        }

        if let (Some(zip_path), Some((bytes, _, _))) = (cover_zip_path, cover_bytes_and_ext) {
            entries.push((zip_path, bytes));
        }

        // Write updated EPUB atomically via temporary file
        let parent = epub_path.parent().unwrap_or_else(|| Path::new("."));
        let tmp = NamedTempFile::new_in(parent)?;
        {
            let mut writer = ZipWriter::new(tmp.reopen()?);
            let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
            let deflated =
                SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

            if let Some((_, mime_bytes)) = entries.iter().find(|(n, _)| n == "mimetype") {
                map_zip(writer.start_file("mimetype", stored))?;
                writer.write_all(mime_bytes)?;
            } else {
                map_zip(writer.start_file("mimetype", stored))?;
                writer.write_all(b"application/epub+zip")?;
            }

            for (name, data) in &entries {
                if name == "mimetype" {
                    continue;
                }
                map_zip(writer.start_file(name, deflated))?;
                writer.write_all(data)?;
            }
            map_zip(writer.finish())?;
        }

        let tmp_path = tmp.into_temp_path();
        if std::fs::rename(&tmp_path, epub_path).is_err() {
            std::fs::copy(&tmp_path, epub_path).map_err(VellumError::Io)?;
            let _ = std::fs::remove_file(&tmp_path);
        }

        Ok(report)
    }
}

fn is_xhtml_entry(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.ends_with(".xhtml") || lower.ends_with(".html") || lower.ends_with(".htm")
}

fn upsert_dc_tag(metadata_inner: &str, tag: &str, value: &str) -> String {
    let escaped = escape_xml(value);
    let re = Regex::new(&format!(r#"(?is)<dc:{tag}(?:\s+[^>]*)?>.*?</dc:{tag}>"#)).unwrap();
    let replacement = format!("<dc:{tag}>{escaped}</dc:{tag}>");
    if re.is_match(metadata_inner) {
        re.replace(metadata_inner, replacement.as_str()).to_string()
    } else {
        format!("{metadata_inner}\n    {replacement}")
    }
}

fn inject_opf_metadata(opf_xml: &str, book: &Book) -> String {
    let meta_re = Regex::new(r#"(?is)(<metadata\b[^>]*>)(.*?)(</metadata>)"#).unwrap();
    let Some(caps) = meta_re.captures(opf_xml) else {
        return opf_xml.to_string();
    };

    let mut inner = caps[2].to_string();
    inner = upsert_dc_tag(&inner, "title", &book.title);
    inner = upsert_dc_tag(&inner, "creator", &book.author);
    if let Some(pub_name) = book.publisher.as_deref().filter(|s| !s.trim().is_empty()) {
        inner = upsert_dc_tag(&inner, "publisher", pub_name);
    }
    if let Some(desc) = book.description.as_deref().filter(|s| !s.trim().is_empty()) {
        inner = upsert_dc_tag(&inner, "description", desc);
    }
    if let Some(date) = book
        .publish_date
        .as_deref()
        .filter(|s| !s.trim().is_empty())
    {
        inner = upsert_dc_tag(&inner, "date", date);
    }
    if let Some(isbn) = book.isbn.as_deref().filter(|s| !s.trim().is_empty()) {
        if !inner.contains(isbn) {
            inner.push_str(&format!(
                "\n    <dc:identifier opf:scheme=\"ISBN\">{}</dc:identifier>",
                escape_xml(isbn)
            ));
        }
    }

    let rebuilt = format!("{}{}\n  {}", &caps[1], inner, &caps[3]);
    meta_re.replace(opf_xml, rebuilt.as_str()).to_string()
}

fn inject_opf_cover(opf_xml: &str, cover_href: &str, mime: &str) -> String {
    let mut out = opf_xml.to_string();

    // Ensure <meta name="cover" content="vellum-cover-img"/> in <metadata>
    let cover_meta_re = Regex::new(r#"(?is)<meta\b[^>]*\bname=["']cover["'][^>]*/?>"#).unwrap();
    let meta_tag = r#"<meta name="cover" content="vellum-cover-img"/>"#;
    if cover_meta_re.is_match(&out) {
        out = cover_meta_re.replace(&out, meta_tag).to_string();
    } else if let Some(pos) = out.to_lowercase().find("</metadata>") {
        out.insert_str(pos, &format!("  {meta_tag}\n  "));
    }

    // Ensure <item id="vellum-cover-img" .../> in <manifest>
    let item_re = Regex::new(r#"(?is)<item\b[^>]*\bid=["']vellum-cover-img["'][^>]*/?>"#).unwrap();
    let item_tag = format!(
        r#"<item id="vellum-cover-img" href="{}" media-type="{}" properties="cover-image"/>"#,
        escape_xml(cover_href),
        mime
    );
    if item_re.is_match(&out) {
        out = item_re.replace(&out, item_tag.as_str()).to_string();
    } else if let Some(pos) = out.to_lowercase().find("</manifest>") {
        out.insert_str(pos, &format!("  {item_tag}\n  "));
    }

    out
}

/// Applique une typographie soignée uniquement sur le texte hors balises `<...>`.
fn smarten_xhtml_prose(xhtml: &str) -> String {
    let mut result = String::with_capacity(xhtml.len());
    let mut in_tag = false;
    let mut text_buf = String::new();

    for ch in xhtml.chars() {
        match ch {
            '<' if !in_tag => {
                if !text_buf.is_empty() {
                    result.push_str(&smarten_text_segment(&text_buf));
                    text_buf.clear();
                }
                in_tag = true;
                result.push(ch);
            }
            '>' if in_tag => {
                in_tag = false;
                result.push(ch);
            }
            _ if in_tag => result.push(ch),
            _ => text_buf.push(ch),
        }
    }
    if !text_buf.is_empty() {
        result.push_str(&smarten_text_segment(&text_buf));
    }
    result
}

fn smarten_text_segment(text: &str) -> String {
    text.replace("...", "…")
        .replace("---", "—")
        .replace("--", "–")
}
