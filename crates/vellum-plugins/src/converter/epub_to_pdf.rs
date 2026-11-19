use crate::converter::epub_to_text::html_to_plain_text;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use vellum_domain::{
    BookConverterPlugin, ConfigField, PluginDescriptor, PluginType, TestResult, VellumError,
    VellumPlugin, VellumResult,
};
use zip::ZipArchive;

pub struct EpubToPdfConverterPlugin {
    descriptor: PluginDescriptor,
}

impl Default for EpubToPdfConverterPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl EpubToPdfConverterPlugin {
    pub fn new() -> Self {
        Self {
            descriptor: PluginDescriptor {
                id: "converter-epub-to-pdf".to_string(),
                name: "Générateur Document (EPUB ➔ PDF)".to_string(),
                description: "Convertit et pagine le texte d'un livre EPUB en document PDF A4 prêt à l'impression avec gestion des sauts de page.".to_string(),
                version: "1.0.0".to_string(),
                plugin_type: PluginType::Conversion,
                external: false,
                command: None,
                supported_extensions: Vec::new(),
                source_format: Some("epub".to_string()),
                target_format: Some("pdf".to_string()),
                config_schema: vec![ConfigField::new(
                    "font_size",
                    "Taille du texte",
                    "number",
                    json!(10.0),
                    "Taille de police en points (8 à 16).",
                )
                .with_range(8.0, 16.0, 1.0)],
            },
        }
    }
}

impl VellumPlugin for EpubToPdfConverterPlugin {
    fn descriptor(&self) -> &PluginDescriptor {
        &self.descriptor
    }

    fn test_configuration(&self, _config: &HashMap<String, Value>) -> TestResult {
        TestResult::ok("Moteur de pagination PDF prêt (génération A4 vectorielle).")
    }
}

impl BookConverterPlugin for EpubToPdfConverterPlugin {
    fn source_format(&self) -> &str {
        "epub"
    }

    fn target_format(&self) -> &str {
        "pdf"
    }

    fn convert(
        &self,
        input_file: &Path,
        output_file: &Path,
        config: &HashMap<String, Value>,
    ) -> VellumResult<PathBuf> {
        let font_size = config
            .get("font_size")
            .and_then(|v| {
                v.as_f64()
                    .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            })
            .unwrap_or(10.0) as f32;

        let lines = extract_lines_from_epub(input_file)?;
        let pdf_bytes = generate_a4_text_pdf(&lines, font_size);
        let mut out = File::create(output_file)?;
        out.write_all(&pdf_bytes)?;
        Ok(output_file.to_path_buf())
    }
}

fn extract_lines_from_epub(epub_file: &Path) -> VellumResult<Vec<String>> {
    let reader = File::open(epub_file)?;
    let mut zip =
        ZipArchive::new(reader).map_err(|e| VellumError::InvalidArgument(e.to_string()))?;

    let mut names: Vec<String> = (0..zip.len())
        .filter_map(|i| {
            zip.by_index(i)
                .ok()
                .and_then(|e| e.name().ok().map(|n| n.to_string()))
        })
        .filter(|n| n.ends_with(".xhtml") || n.ends_with(".html"))
        .collect();
    names.sort();

    let mut lines = Vec::new();
    for name in names {
        if let Ok(mut entry) = zip.by_name(&name) {
            let mut html = String::new();
            if entry.read_to_string(&mut html).is_ok() {
                let text = html_to_plain_text(&html);
                for l in text.lines() {
                    lines.push(l.to_string());
                }
            }
        }
    }
    Ok(lines)
}

fn generate_a4_text_pdf(lines: &[String], font_size: f32) -> Vec<u8> {
    let leading = font_size * 1.4;
    let margin = 50.0f32;
    let page_height = 841.89f32;
    let max_chars = ((595.28 - 2.0 * margin) / (font_size * 0.52)).max(30.0) as usize;

    let mut pages_streams: Vec<String> = Vec::new();
    let mut cur_stream = String::new();
    let mut y = page_height - margin;

    for raw in lines {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            y -= leading * 0.7;
            continue;
        }
        for wrapped in wrap_text(trimmed, max_chars) {
            if y - leading < margin {
                pages_streams.push(std::mem::take(&mut cur_stream));
                y = page_height - margin;
            }
            let safe = sanitize_pdf_string(&wrapped);
            cur_stream.push_str(&format!(
                "BT /F1 {font_size:.1} Tf {margin:.1} {y:.1} Td ({safe}) Tj ET\n"
            ));
            y -= leading;
        }
    }
    if !cur_stream.is_empty() || pages_streams.is_empty() {
        pages_streams.push(cur_stream);
    }

    let mut out = Vec::new();
    out.extend_from_slice(b"%PDF-1.4\n");

    let total_objs = 3 + pages_streams.len() * 2;
    let mut offsets = vec![0usize; total_objs + 1];

    offsets[1] = out.len();
    out.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    let kids: Vec<String> = (0..pages_streams.len())
        .map(|i| format!("{} 0 R", 4 + i * 2))
        .collect();
    offsets[2] = out.len();
    out.extend_from_slice(
        format!(
            "2 0 obj\n<< /Type /Pages /Kids [{}] /Count {} >>\nendobj\n",
            kids.join(" "),
            pages_streams.len()
        )
        .as_bytes(),
    );

    offsets[3] = out.len();
    out.extend_from_slice(
        b"3 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n",
    );

    for (i, stream) in pages_streams.iter().enumerate() {
        let page_id = 4 + i * 2;
        let content_id = 5 + i * 2;

        offsets[page_id] = out.len();
        out.extend_from_slice(
            format!(
                "{page_id} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595.28 841.89] /Resources << /Font << /F1 3 0 R >> >> /Contents {content_id} 0 R >>\nendobj\n"
            )
            .as_bytes(),
        );

        offsets[content_id] = out.len();
        out.extend_from_slice(
            format!(
                "{content_id} 0 obj\n<< /Length {} >>\nstream\n{}endstream\nendobj\n",
                stream.len(),
                stream
            )
            .as_bytes(),
        );
    }

    let xref_pos = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", total_objs + 1).as_bytes());
    for off in offsets.iter().skip(1) {
        out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_pos}\n%%EOF\n",
            total_objs + 1
        )
        .as_bytes(),
    );
    out
}

fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
    let mut res = Vec::new();
    let mut cur = String::new();
    for word in text.split_whitespace() {
        if cur.is_empty() {
            cur.push_str(word);
        } else if cur.len() + 1 + word.len() <= max_chars {
            cur.push(' ');
            cur.push_str(word);
        } else {
            res.push(std::mem::take(&mut cur));
            cur.push_str(word);
        }
    }
    if !cur.is_empty() {
        res.push(cur);
    }
    res
}

fn sanitize_pdf_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '(' | ')' | '\\' => {
                out.push('\\');
                out.push(c);
            }
            c if (c as u32) >= 32 && (c as u32) <= 126 => out.push(c),
            _ => out.push(' '),
        }
    }
    out
}
