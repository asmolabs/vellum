use std::path::Path;

use vellum_domain::{ExtractedMetadata, VellumResult};

use crate::plugin_service::PluginService;

#[derive(Clone)]
pub struct MetadataService {
    plugin_service: PluginService,
}

impl MetadataService {
    pub fn new(plugin_service: PluginService) -> Self {
        Self { plugin_service }
    }

    pub fn extract_metadata(&self, file_path: &Path) -> ExtractedMetadata {
        let ext = file_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        let mut meta = if let Some(plugin) =
            self.plugin_service.find_format_plugin_for_extension(&ext)
        {
            plugin
                .extract_metadata(file_path, &std::collections::HashMap::new())
                .unwrap_or_default()
        } else {
            ExtractedMetadata::default()
        };

        let stem = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown Book")
            .trim();

        let (fallback_author, fallback_title) = Self::parse_author_and_title_from_stem(stem);

        if meta.title.as_deref().unwrap_or("").trim().is_empty() {
            meta.title = Some(fallback_title);
        }
        if meta.author.as_deref().unwrap_or("").trim().is_empty()
            || meta
                .author
                .as_deref()
                .map(|a| a.eq_ignore_ascii_case("unknown") || a.eq_ignore_ascii_case("unknown author"))
                .unwrap_or(false)
        {
            meta.author = Some(fallback_author);
        }

        meta
    }

    pub fn extract_cover(&self, file_path: &Path) -> VellumResult<Option<Vec<u8>>> {
        let ext = file_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if let Some(plugin) = self.plugin_service.find_format_plugin_for_extension(&ext) {
            let meta = plugin.extract_metadata(file_path, &std::collections::HashMap::new())?;
            Ok(meta.cover_data)
        } else {
            Ok(None)
        }
    }

    pub fn parse_author_and_title_from_stem(stem: &str) -> (String, String) {
        if let Some((author_part, title_part)) = stem.split_once(" - ") {
            let author = author_part.trim();
            let title = title_part.trim();
            if !author.is_empty() && !title.is_empty() {
                return (author.to_string(), title.to_string());
            }
        }
        ("Unknown Author".to_string(), stem.to_string())
    }
}
