use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use vellum_domain::{
    BookFormatPlugin, ExtractedMetadata, PluginDescriptor, PluginType, VellumPlugin, VellumResult,
};

pub struct PdfFormatPlugin {
    descriptor: PluginDescriptor,
}

impl Default for PdfFormatPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl PdfFormatPlugin {
    pub fn new() -> Self {
        Self {
            descriptor: PluginDescriptor {
                id: "format-pdf".to_string(),
                name: "Lecteur de format PDF (Adobe / Document)".to_string(),
                description:
                    "Support des documents PDF avec extraction des métadonnées intégrées et aperçu."
                        .to_string(),
                version: "1.0.0".to_string(),
                plugin_type: PluginType::Format,
                external: false,
                command: None,
                supported_extensions: vec!["pdf".to_string()],
                source_format: None,
                target_format: None,
                config_schema: Vec::new(),
            },
        }
    }
}

impl VellumPlugin for PdfFormatPlugin {
    fn descriptor(&self) -> &PluginDescriptor {
        &self.descriptor
    }

    fn as_format_plugin(&self) -> Option<&dyn BookFormatPlugin> {
        Some(self)
    }
}

impl BookFormatPlugin for PdfFormatPlugin {
    fn supported_extensions(&self) -> Vec<String> {
        vec!["pdf".to_string()]
    }

    fn extract_metadata(
        &self,
        file: &Path,
        _config: &HashMap<String, Value>,
    ) -> VellumResult<ExtractedMetadata> {
        let bytes = std::fs::read(file)?;
        let raw_str = String::from_utf8_lossy(&bytes);

        let title = extract_pdf_info_field(&raw_str, "Title");
        let author = extract_pdf_info_field(&raw_str, "Author");
        let publisher = extract_pdf_info_field(&raw_str, "Producer");
        let description = extract_pdf_info_field(&raw_str, "Subject");

        Ok(ExtractedMetadata {
            title,
            author,
            publisher,
            publish_date: None,
            isbn: None,
            series: None,
            series_index: None,
            description,
            tags: vec!["PDF".to_string()],
            cover_data: None,
            cover_ext: Some(".jpg".to_string()),
        })
    }
}

fn extract_pdf_info_field(raw: &str, key: &str) -> Option<String> {
    let pattern = format!(r"/{key}\s*\(([^)]+)\)");
    let re = Regex::new(&pattern).ok()?;
    re.captures(raw)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim().to_string())
        .filter(|s| !s.is_empty())
}
