use roxmltree::Document;
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use vellum_domain::{
    BookFormatPlugin, ExtractedMetadata, PluginDescriptor, PluginType, VellumError, VellumPlugin,
    VellumResult,
};
use zip::ZipArchive;

pub struct EpubFormatPlugin {
    descriptor: PluginDescriptor,
}

impl Default for EpubFormatPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl EpubFormatPlugin {
    pub fn new() -> Self {
        Self {
            descriptor: PluginDescriptor {
                id: "format-epub".to_string(),
                name: "Lecteur de format EPUB (Standard)".to_string(),
                description: "Support natif des livres numériques au format EPUB 2 et EPUB 3 avec extraction des métadonnées Dublin Core et couvertures.".to_string(),
                version: "1.0.0".to_string(),
                plugin_type: PluginType::Format,
                external: false,
                command: None,
                supported_extensions: vec!["epub".to_string()],
                source_format: None,
                target_format: None,
                config_schema: Vec::new(),
            },
        }
    }
}

impl VellumPlugin for EpubFormatPlugin {
    fn descriptor(&self) -> &PluginDescriptor {
        &self.descriptor
    }

    fn as_format_plugin(&self) -> Option<&dyn BookFormatPlugin> {
        Some(self)
    }
}

impl BookFormatPlugin for EpubFormatPlugin {
    fn supported_extensions(&self) -> Vec<String> {
        vec!["epub".to_string()]
    }

    fn extract_metadata(
        &self,
        file: &Path,
        _config: &HashMap<String, Value>,
    ) -> VellumResult<ExtractedMetadata> {
        let reader = File::open(file)?;
        let mut zip =
            ZipArchive::new(reader).map_err(|e| VellumError::InvalidArgument(e.to_string()))?;

        // 1. Read META-INF/container.xml
        let mut container_xml = String::new();
        {
            let mut entry = zip.by_name("META-INF/container.xml").map_err(|_| {
                VellumError::InvalidArgument(
                    "Invalid EPUB: missing META-INF/container.xml".to_string(),
                )
            })?;
            entry.read_to_string(&mut container_xml)?;
        }

        let container_doc = Document::parse(&container_xml)
            .map_err(|e| VellumError::InvalidArgument(e.to_string()))?;
        let opf_path = container_doc
            .descendants()
            .find(|n| n.has_tag_name("rootfile"))
            .and_then(|n| n.attribute("full-path"))
            .ok_or_else(|| {
                VellumError::InvalidArgument("Invalid EPUB: no rootfile in container.xml".into())
            })?
            .to_string();

        // 2. Read OPF file
        let mut opf_xml = String::new();
        {
            let mut entry = zip.by_name(&opf_path).map_err(|_| {
                VellumError::InvalidArgument(format!("OPF file not found at {opf_path}"))
            })?;
            entry.read_to_string(&mut opf_xml)?;
        }

        let opf_doc =
            Document::parse(&opf_xml).map_err(|e| VellumError::InvalidArgument(e.to_string()))?;

        let title = find_first_element_text(&opf_doc, "title");
        let author = find_first_element_text(&opf_doc, "creator");
        let publisher = find_first_element_text(&opf_doc, "publisher");
        let publish_date = find_first_element_text(&opf_doc, "date");
        let description = find_first_element_text(&opf_doc, "description");
        let isbn = find_isbn(&opf_doc);
        let (series, series_index) = find_series(&opf_doc);

        let mut tags = Vec::new();
        for node in opf_doc
            .descendants()
            .filter(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("subject"))
        {
            if let Some(txt) = node.text().map(str::trim).filter(|s| !s.is_empty()) {
                if !tags.iter().any(|t| t == txt) {
                    tags.push(txt.to_string());
                }
            }
        }

        // 3. Extract cover image if present
        let mut cover_data = None;
        let mut cover_ext = None;

        if let Some(cover_href) = find_cover_href(&opf_doc) {
            let resolved_cover_path = Path::new(&opf_path)
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .map(|p| p.join(&cover_href))
                .unwrap_or_else(|| PathBuf::from(&cover_href))
                .to_string_lossy()
                .replace('\\', "/");

            if let Ok(mut entry) = zip.by_name(&resolved_cover_path) {
                let mut buf = Vec::new();
                if entry.read_to_end(&mut buf).is_ok() && !buf.is_empty() {
                    let name_str = entry.name().map(|n| n.to_string()).unwrap_or_default();
                    let ext = Path::new(&name_str)
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|e| format!(".{}", e.to_lowercase()))
                        .unwrap_or_else(|| ".jpg".to_string());
                    cover_data = Some(buf);
                    cover_ext = Some(ext);
                }
            }
        }

        Ok(ExtractedMetadata {
            title,
            author,
            publisher,
            publish_date,
            isbn,
            series,
            series_index,
            description,
            tags,
            cover_data,
            cover_ext,
        })
    }
}

fn find_isbn(doc: &Document) -> Option<String> {
    for node in doc
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("identifier"))
    {
        let text = node.text().map(str::trim).unwrap_or("");
        let scheme = node
            .attribute("scheme")
            .or_else(|| node.attribute("opf:scheme"))
            .unwrap_or("");
        let id_attr = node.attribute("id").unwrap_or("");

        let is_isbn = scheme.eq_ignore_ascii_case("isbn")
            || id_attr.to_lowercase().contains("isbn")
            || text.to_lowercase().contains("isbn")
            || text.starts_with("978")
            || text.starts_with("979");

        if is_isbn || (!text.is_empty() && (text.len() == 10 || text.len() == 13 || text.len() == 14 || text.len() == 17)) {
            let clean: String = text
                .chars()
                .filter(|c| c.is_ascii_digit() || *c == 'X' || *c == 'x')
                .collect();
            if clean.len() == 10 || clean.len() == 13 {
                return Some(clean);
            }
        }
    }
    None
}

fn find_first_element_text(doc: &Document, local_name: &str) -> Option<String> {
    doc.descendants()
        .find(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case(local_name))
        .and_then(|n| n.text())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
}

fn find_cover_href(doc: &Document) -> Option<String> {
    // Strategy A: <item properties="cover-image" ...>
    for item in doc
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("item"))
    {
        if let Some(props) = item.attribute("properties") {
            if props.contains("cover-image") {
                if let Some(href) = item.attribute("href") {
                    return Some(href.to_string());
                }
            }
        }
    }

    // Strategy B: <meta name="cover" content="item_id" />
    let cover_id = doc
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("meta"))
        .find(|m| {
            m.attribute("name")
                .map(|v| v.eq_ignore_ascii_case("cover"))
                .unwrap_or(false)
        })
        .and_then(|m| m.attribute("content"));

    if let Some(cid) = cover_id {
        for item in doc
            .descendants()
            .filter(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("item"))
        {
            if item.attribute("id") == Some(cid) {
                if let Some(href) = item.attribute("href") {
                    return Some(href.to_string());
                }
            }
        }
    }

    // Strategy C: item with id containing "cover" and image media-type
    for item in doc
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("item"))
    {
        let id = item.attribute("id").unwrap_or_default().to_lowercase();
        let mt = item.attribute("media-type").unwrap_or_default();
        if id.contains("cover") && mt.starts_with("image/") {
            if let Some(href) = item.attribute("href") {
                return Some(href.to_string());
            }
        }
    }

    None
}

fn find_series(doc: &Document) -> (Option<String>, Option<f64>) {
    let mut series = None;
    let mut index = None;

    // 1. Calibre standard metadata (<meta name="calibre:series" content="Dune"/>)
    for node in doc
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("meta"))
    {
        if let Some(name) = node.attribute("name") {
            if name.eq_ignore_ascii_case("calibre:series") {
                if let Some(content) = node.attribute("content") {
                    let trimmed = content.trim();
                    if !trimmed.is_empty() {
                        series = Some(trimmed.to_string());
                    }
                }
            } else if name.eq_ignore_ascii_case("calibre:series_index") {
                if let Some(content) = node.attribute("content") {
                    if let Ok(val) = content.trim().parse::<f64>() {
                        index = Some(val);
                    }
                }
            }
        }
    }

    // 2. EPUB 3 belongs-to-collection fallback
    if series.is_none() {
        for node in doc
            .descendants()
            .filter(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("meta"))
        {
            if let Some(prop) = node.attribute("property") {
                if prop.eq_ignore_ascii_case("belongs-to-collection") {
                    if let Some(txt) = node.text().map(str::trim).filter(|s| !s.is_empty()) {
                        series = Some(txt.to_string());
                        if let Some(id) = node.attribute("id") {
                            let target_refine = format!("#{id}");
                            for refine in doc.descendants().filter(|n| {
                                n.is_element() && n.tag_name().name().eq_ignore_ascii_case("meta")
                            }) {
                                if refine.attribute("refines") == Some(&target_refine)
                                    && refine.attribute("property") == Some("group-position")
                                {
                                    if let Some(pos_txt) = refine.text().map(str::trim) {
                                        if let Ok(val) = pos_txt.parse::<f64>() {
                                            index = Some(val);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    (series, index)
}
