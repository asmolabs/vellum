use roxmltree::Document;
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use vellum_domain::{
    BookFormatPlugin, ExtractedMetadata, PluginDescriptor, PluginType, VellumError, VellumPlugin,
    VellumResult,
};
use zip::ZipArchive;

pub struct ComicFormatPlugin {
    descriptor: PluginDescriptor,
}

impl Default for ComicFormatPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl ComicFormatPlugin {
    pub fn new() -> Self {
        Self {
            descriptor: PluginDescriptor {
                id: "format-comic".to_string(),
                name: "Lecteur de Bandes Dessinées (CBZ / CBR)".to_string(),
                description: "Support des archives de bande dessinée CBZ et CBR avec extraction de ComicInfo.xml et de la première page en couverture.".to_string(),
                version: "1.0.0".to_string(),
                plugin_type: PluginType::Format,
                external: false,
                command: None,
                supported_extensions: vec!["cbz".to_string(), "cbr".to_string()],
                source_format: None,
                target_format: None,
                config_schema: Vec::new(),
            },
        }
    }
}

impl VellumPlugin for ComicFormatPlugin {
    fn descriptor(&self) -> &PluginDescriptor {
        &self.descriptor
    }

    fn as_format_plugin(&self) -> Option<&dyn BookFormatPlugin> {
        Some(self)
    }
}

impl BookFormatPlugin for ComicFormatPlugin {
    fn supported_extensions(&self) -> Vec<String> {
        vec!["cbz".to_string(), "cbr".to_string()]
    }

    fn extract_metadata(
        &self,
        file: &Path,
        _config: &HashMap<String, Value>,
    ) -> VellumResult<ExtractedMetadata> {
        let mut title = None;
        let mut author = None;
        let mut description = None;
        let mut tags = vec!["BD / Manga".to_string()];
        let mut cover_data = None;
        let mut cover_ext = Some(".jpg".to_string());

        let reader = File::open(file)?;
        if let Ok(mut zip) = ZipArchive::new(reader) {
            let mut entry_names: Vec<String> = (0..zip.len())
                .filter_map(|i| zip.by_index(i).ok().and_then(|e| e.name().ok().map(|n| n.to_string())))
                .collect();

            // 1. Check ComicInfo.xml
            if let Some(ci_name) = entry_names
                .iter()
                .find(|n| n.eq_ignore_ascii_case("ComicInfo.xml"))
                .cloned()
            {
                if let Ok(mut entry) = zip.by_name(&ci_name) {
                    let mut xml = String::new();
                    if entry.read_to_string(&mut xml).is_ok() {
                        if let Ok(doc) = Document::parse(&xml) {
                            let series = get_elem(&doc, "Series");
                            let number = get_elem(&doc, "Number");
                            let xml_title = get_elem(&doc, "Title");
                            let writer = get_elem(&doc, "Writer");
                            let summary = get_elem(&doc, "Summary");
                            let genre = get_elem(&doc, "Genre");

                            if let Some(t) = xml_title {
                                let prefix = series.map(|s| format!("{s} - ")).unwrap_or_default();
                                let suffix = number.map(|n| format!(" #{n}")).unwrap_or_default();
                                title = Some(format!("{prefix}{t}{suffix}"));
                            } else if let Some(s) = series {
                                let suffix = number.map(|n| format!(" #{n}")).unwrap_or_default();
                                title = Some(format!("{s}{suffix}"));
                            }

                            if writer.is_some() {
                                author = writer;
                            }
                            if summary.is_some() {
                                description = summary;
                            }
                            if let Some(g_str) = genre {
                                for g in g_str.split([',', ';']) {
                                    let trimmed = g.trim();
                                    if !trimmed.is_empty() && !tags.iter().any(|t| t == trimmed) {
                                        tags.push(trimmed.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // 2. First image as cover
            entry_names.retain(|n| {
                let l = n.to_lowercase();
                l.ends_with(".jpg")
                    || l.ends_with(".jpeg")
                    || l.ends_with(".png")
                    || l.ends_with(".webp")
            });
            entry_names.sort();

            if let Some(first_img) = entry_names.first() {
                if let Ok(mut entry) = zip.by_name(first_img) {
                    let mut buf = Vec::new();
                    if entry.read_to_end(&mut buf).is_ok() && !buf.is_empty() {
                        let ext = Path::new(first_img)
                            .extension()
                            .and_then(|e| e.to_str())
                            .map(|e| format!(".{}", e.to_lowercase()))
                            .unwrap_or_else(|| ".jpg".to_string());
                        cover_data = Some(buf);
                        cover_ext = Some(ext);
                    }
                }
            }
        } else {
            return Err(VellumError::InvalidArgument(
                "Archive CBZ/CBR invalide".to_string(),
            ));
        }

        Ok(ExtractedMetadata {
            title,
            author,
            publisher: None,
            publish_date: None,
            description,
            tags,
            cover_data,
            cover_ext,
        })
    }
}

fn get_elem(doc: &Document, name: &str) -> Option<String> {
    doc.descendants()
        .find(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case(name))
        .and_then(|n| n.text())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
}
