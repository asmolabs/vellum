use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use tempfile::NamedTempFile;
use vellum_domain::{VellumError, VellumResult};
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

/// Transforms a standard EPUB into a KEPUB (Kobo EPUB) by injecting koboSpan tags
/// into paragraph elements (<p>), allowing the Kobo ACCESS rendering engine to handle
/// per-chapter page numbering, micro-measurements, and instant bookmarking.
pub fn convert_epub_to_kepub(input_epub: &Path, output_kepub: &Path) -> VellumResult<()> {
    let file = File::open(input_epub)
        .map_err(|e| VellumError::Plugin(format!("Impossible d'ouvrir l'EPUB source: {e}")))?;
    let mut zip_in = ZipArchive::new(file)
        .map_err(|e| VellumError::Plugin(format!("Archive EPUB invalide: {e}")))?;

    let temp_file = NamedTempFile::new().map_err(|e| {
        VellumError::Plugin(format!("Impossible de créer un fichier temporaire: {e}"))
    })?;
    let mut zip_out = ZipWriter::new(
        temp_file
            .reopen()
            .map_err(|e| VellumError::Plugin(format!("Erreur d'accès fichier temporaire: {e}")))?,
    );

    let mut chapter_idx = 0usize;

    for i in 0..zip_in.len() {
        let mut entry = zip_in
            .by_index(i)
            .map_err(|e| VellumError::Plugin(format!("Erreur lecture entrée zip {i}: {e}")))?;
        let entry_name = entry.name().map(|n| n.to_string()).unwrap_or_default();

        if entry.is_dir() {
            zip_out
                .add_directory(&entry_name, SimpleFileOptions::default())
                .map_err(|e| VellumError::Plugin(format!("Erreur ajout dossier zip: {e}")))?;
            continue;
        }

        let is_html = entry_name.ends_with(".xhtml")
            || entry_name.ends_with(".html")
            || entry_name.ends_with(".htm");

        // Keep mimetype uncompressed as first entry
        let options = if entry_name == "mimetype" {
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored)
        } else {
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated)
        };

        zip_out
            .start_file(&entry_name, options)
            .map_err(|e| VellumError::Plugin(format!("Erreur écriture entrée zip: {e}")))?;

        if is_html {
            let mut content = String::new();
            if entry.read_to_string(&mut content).is_ok() {
                chapter_idx += 1;
                let modified = inject_kobo_spans(&content, chapter_idx);
                zip_out
                    .write_all(modified.as_bytes())
                    .map_err(|e| VellumError::Plugin(format!("Erreur écriture xhtml: {e}")))?;
            } else {
                let mut buf = Vec::new();
                let _ = entry.read_to_end(&mut buf);
                let _ = zip_out.write_all(&buf);
            }
        } else {
            let mut buf = Vec::new();
            entry
                .read_to_end(&mut buf)
                .map_err(|e| VellumError::Plugin(format!("Erreur lecture buffer: {e}")))?;
            zip_out
                .write_all(&buf)
                .map_err(|e| VellumError::Plugin(format!("Erreur écriture binaire: {e}")))?;
        }
    }

    zip_out
        .finish()
        .map_err(|e| VellumError::Plugin(format!("Erreur finalisation zip kepub: {e}")))?;

    if let Some(parent) = output_kepub.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            VellumError::Plugin(format!("Impossible de créer le dossier de sortie: {e}"))
        })?;
    }

    std::fs::copy(temp_file.path(), output_kepub)
        .map_err(|e| VellumError::Plugin(format!("Impossible de copier le KEPUB final: {e}")))?;

    Ok(())
}

/// Injects `<span class="koboSpan" id="kobo.{chapter}.{span_idx}">...</span>` into paragraph tags
fn inject_kobo_spans(html: &str, chapter_idx: usize) -> String {
    let mut result = String::with_capacity(html.len() + 1024);
    let mut span_idx = 0usize;
    let mut cursor = 0;

    let p_regex = regex::Regex::new(r"(?is)(<p\b[^>]*>)(.*?)(</p>)").unwrap();

    for cap in p_regex.captures_iter(html) {
        let whole_match = cap.get(0).unwrap();
        // Append text prior to this <p>
        result.push_str(&html[cursor..whole_match.start()]);

        let p_start = cap.get(1).unwrap().as_str();
        let p_body = cap.get(2).unwrap().as_str();
        let p_end = cap.get(3).unwrap().as_str();

        span_idx += 1;
        result.push_str(p_start);
        if !p_body.trim().is_empty() && !p_body.contains("koboSpan") {
            result.push_str(&format!(
                r#"<span class="koboSpan" id="kobo.{chapter_idx}.{span_idx}">{p_body}</span>"#
            ));
        } else {
            result.push_str(p_body);
        }
        result.push_str(p_end);

        cursor = whole_match.end();
    }

    result.push_str(&html[cursor..]);
    result
}
