//! Enrichisseur de métadonnées pour bandes dessinées / comics (CBZ/CBR) via ComicVine.

use std::collections::HashMap;
use std::path::Path;

use serde_json::{json, Value};
use tracing::warn;
use vellum_domain::{
    ConfigField, MetadataEnricherPlugin, PluginDescriptor, PluginType, TestResult, VellumPlugin,
};

use super::helpers::{clean_string, is_generic_value, url_encode};
use super::{BookMetadataDto, GoogleBooksEnricherPlugin};

const COMICVINE_SEARCH_URL: &str = "https://comicvine.gamespot.com/api/search/";

/// Enrichisseur ComicVine pour CBZ / CBR / Comics.
pub struct ComicVineEnricherPlugin {
    descriptor: PluginDescriptor,
}

impl Default for ComicVineEnricherPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl ComicVineEnricherPlugin {
    pub fn new() -> Self {
        Self {
            descriptor: PluginDescriptor {
                id: "comicvine-enricher".to_string(),
                name: "ComicVine Metadata Enricher".to_string(),
                description:
                    "Enrichit les métadonnées de comics, mangas et BD (CBZ/CBR) via l'API ComicVine."
                        .to_string(),
                version: "1.0.0".to_string(),
                plugin_type: PluginType::Import,
                external: false,
                command: None,
                supported_extensions: vec!["cbz".to_string(), "cbr".to_string()],
                source_format: None,
                target_format: None,
                config_schema: vec![ConfigField::new(
                    "api_key",
                    "Clé API ComicVine",
                    "password",
                    json!(""),
                    "Clé API gratuite ComicVine (comicvine.gamespot.com/api).",
                )],
            },
        }
    }
}

impl VellumPlugin for ComicVineEnricherPlugin {
    fn descriptor(&self) -> &PluginDescriptor {
        &self.descriptor
    }

    fn test_configuration(&self, config: &HashMap<String, Value>) -> TestResult {
        let api_key = config
            .get("api_key")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .trim();
        if api_key.is_empty() {
            return TestResult::error("Clé API ComicVine manquante.");
        }

        let Some(client) = GoogleBooksEnricherPlugin::shared_http_client() else {
            return TestResult::error("Impossible d'initialiser le client HTTP.");
        };

        let url = format!(
            "{COMICVINE_SEARCH_URL}?api_key={}&format=json&resources=issue&query=batman&limit=1",
            url_encode(api_key)
        );
        match client.get(&url).send() {
            Ok(resp) if resp.status().is_success() => {
                TestResult::ok("Connexion ComicVine réussie !")
            }
            Ok(resp) => TestResult::error(format!("Erreur ComicVine HTTP {}", resp.status())),
            Err(e) => TestResult::error(format!("Erreur réseau ComicVine: {e}")),
        }
    }
}

impl MetadataEnricherPlugin for ComicVineEnricherPlugin {
    fn enrich_metadata(
        &self,
        book_file: &Path,
        mut current_metadata: HashMap<String, Value>,
        config: &HashMap<String, Value>,
    ) -> HashMap<String, Value> {
        let api_key = config
            .get("api_key")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .trim()
            .to_string();
        if api_key.is_empty() {
            return current_metadata;
        }

        let mut title = clean_string(current_metadata.get("title"));
        if title.is_empty() {
            if let Some(stem) = book_file.file_stem().and_then(|s| s.to_str()) {
                title = stem.to_string();
            }
        }
        if title.is_empty() || is_generic_value(&title) {
            return current_metadata;
        }

        let Some(client) = GoogleBooksEnricherPlugin::shared_http_client() else {
            warn!("ComicVineEnricherPlugin: client HTTP indisponible.");
            return current_metadata;
        };

        let url = format!(
            "{COMICVINE_SEARCH_URL}?api_key={}&format=json&resources=volume,issue&query={}&limit=1",
            url_encode(&api_key),
            url_encode(&title)
        );
        let Ok(resp) = client.get(&url).send() else {
            return current_metadata;
        };
        if !resp.status().is_success() {
            return current_metadata;
        }
        let Ok(root) = resp.json::<Value>() else {
            return current_metadata;
        };

        if let Some(dto) = parse_comicvine_response(&root) {
            if let Some(t) = dto.title {
                if clean_string(current_metadata.get("title")).is_empty() {
                    current_metadata.insert("title".to_string(), json!(t));
                }
            }
            if let Some(pub_name) = dto.publisher {
                if clean_string(current_metadata.get("publisher")).is_empty() {
                    current_metadata.insert("publisher".to_string(), json!(pub_name));
                }
            }
            if let Some(desc) = dto.description {
                if clean_string(current_metadata.get("description")).is_empty() {
                    current_metadata.insert("description".to_string(), json!(desc));
                }
            }
            if let Some(date) = dto.published_date {
                if clean_string(current_metadata.get("publish_date")).is_empty() {
                    current_metadata.insert("publish_date".to_string(), json!(date));
                }
            }
        }

        current_metadata
    }
}

pub(crate) fn parse_comicvine_response(root: &Value) -> Option<BookMetadataDto> {
    let results = root.get("results").and_then(|v| v.as_array())?;
    let first = results.first()?;

    let title = first
        .get("name")
        .and_then(|v| v.as_str())
        .or_else(|| first.pointer("/volume/name").and_then(|v| v.as_str()))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from);

    let publisher = first
        .pointer("/publisher/name")
        .and_then(|v| v.as_str())
        .map(String::from);

    let description = first
        .get("deck")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from);

    let published_date = first
        .get("cover_date")
        .or_else(|| first.get("start_year"))
        .and_then(|v| v.as_str())
        .map(String::from);

    let cover_url = first
        .pointer("/image/medium_url")
        .and_then(|v| v.as_str())
        .map(String::from);

    Some(BookMetadataDto {
        title,
        authors: Vec::new(),
        publisher,
        published_date,
        description,
        categories: vec!["Comics".to_string()],
        cover_url,
        isbn: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_comicvine_search_response() {
        let sample = json!({
            "status_code": 1,
            "results": [{
                "name": "Batman: Year One",
                "deck": "The origin of Batman.",
                "cover_date": "1987-02-01",
                "publisher": { "name": "DC Comics" },
                "image": { "medium_url": "https://comicvine.gamespot.com/a/uploads/year_one.jpg" }
            }]
        });

        let dto = parse_comicvine_response(&sample).unwrap();
        assert_eq!(dto.title.as_deref(), Some("Batman: Year One"));
        assert_eq!(dto.publisher.as_deref(), Some("DC Comics"));
        assert_eq!(dto.published_date.as_deref(), Some("1987-02-01"));
    }
}
