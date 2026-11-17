//! Enrichisseur de métadonnées via l'API GraphQL Hardcover.

use std::collections::HashMap;
use std::path::Path;

use serde_json::{json, Value};
use tracing::warn;
use vellum_domain::{
    ConfigField, MetadataEnricherPlugin, PluginDescriptor, PluginType, TestResult, VellumPlugin,
};

use super::helpers::{clean_string, clean_tag, is_generic_value};
use super::{BookMetadataDto, GoogleBooksEnricherPlugin};

const HARDCOVER_GRAPHQL_URL: &str = "https://api.hardcover.app/v1/graphql";

/// Enrichisseur Hardcover (titre, auteur, synopsis, date, tags, couverture).
pub struct HardcoverEnricherPlugin {
    descriptor: PluginDescriptor,
}

impl Default for HardcoverEnricherPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl HardcoverEnricherPlugin {
    pub fn new() -> Self {
        Self {
            descriptor: PluginDescriptor {
                id: "hardcover-enricher".to_string(),
                name: "Hardcover Metadata Enricher".to_string(),
                description:
                    "Enrichit les métadonnées de romans (résumé, tags, date, couverture) via l'API GraphQL Hardcover."
                        .to_string(),
                version: "1.0.0".to_string(),
                plugin_type: PluginType::Import,
                external: false,
                command: None,
                supported_extensions: Vec::new(),
                source_format: None,
                target_format: None,
                config_schema: vec![ConfigField::new(
                    "api_key",
                    "Jeton Bearer Hardcover",
                    "password",
                    json!(""),
                    "Jeton API Hardcover (Account > API).",
                )],
            },
        }
    }
}

impl VellumPlugin for HardcoverEnricherPlugin {
    fn descriptor(&self) -> &PluginDescriptor {
        &self.descriptor
    }

    fn test_configuration(&self, config: &HashMap<String, Value>) -> TestResult {
        let token = config
            .get("api_key")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .trim();
        if token.is_empty() {
            return TestResult::error("Jeton API Hardcover manquant.");
        }

        let Some(client) = GoogleBooksEnricherPlugin::shared_http_client() else {
            return TestResult::error("Impossible d'initialiser le client HTTP.");
        };

        let body = json!({ "query": "{ me { id username } }" });
        match client
            .post(HARDCOVER_GRAPHQL_URL)
            .bearer_auth(token)
            .json(&body)
            .send()
        {
            Ok(resp) if resp.status().is_success() => {
                TestResult::ok("Connexion Hardcover GraphQL réussie !")
            }
            Ok(resp) => TestResult::error(format!("Erreur Hardcover HTTP {}", resp.status())),
            Err(e) => TestResult::error(format!("Erreur réseau Hardcover: {e}")),
        }
    }
}

impl MetadataEnricherPlugin for HardcoverEnricherPlugin {
    fn enrich_metadata(
        &self,
        book_file: &Path,
        mut current_metadata: HashMap<String, Value>,
        config: &HashMap<String, Value>,
    ) -> HashMap<String, Value> {
        let token = config
            .get("api_key")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .trim()
            .to_string();
        if token.is_empty() {
            return current_metadata;
        }

        let mut title = clean_string(current_metadata.get("title"));
        if title.is_empty() {
            if let Some(stem) = book_file.file_stem().and_then(|s| s.to_str()) {
                title = stem.to_string();
            }
        }
        let author = clean_string(current_metadata.get("author"));
        if title.is_empty() || is_generic_value(&title) {
            return current_metadata;
        }

        let search_query = if !author.is_empty() && !is_generic_value(&author) {
            format!("{title} {author}")
        } else {
            title
        };

        let Some(client) = GoogleBooksEnricherPlugin::shared_http_client() else {
            warn!("HardcoverEnricherPlugin: client HTTP indisponible.");
            return current_metadata;
        };

        let gql = json!({
            "query": "query Search($q: String!) { search(query: $q, query_type: \"books\", per_page: 1) { results } }",
            "variables": { "q": search_query }
        });

        let Ok(resp) = client
            .post(HARDCOVER_GRAPHQL_URL)
            .bearer_auth(&token)
            .json(&gql)
            .send()
        else {
            return current_metadata;
        };

        if !resp.status().is_success() {
            return current_metadata;
        }

        let Ok(root) = resp.json::<Value>() else {
            return current_metadata;
        };

        if let Some(dto) = parse_hardcover_response(&root) {
            if let Some(t) = dto.title {
                if clean_string(current_metadata.get("title")).is_empty() {
                    current_metadata.insert("title".to_string(), json!(t));
                }
            }
            if !dto.authors.is_empty() && clean_string(current_metadata.get("author")).is_empty() {
                current_metadata.insert("author".to_string(), json!(dto.authors.join(", ")));
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
            if !dto.categories.is_empty() {
                current_metadata.insert("tags".to_string(), json!(dto.categories));
            }
        }

        current_metadata
    }
}

pub(crate) fn parse_hardcover_response(root: &Value) -> Option<BookMetadataDto> {
    let hits = root
        .pointer("/data/search/results/hits")
        .and_then(|v| v.as_array())?;
    let doc = hits.first()?.get("document")?;

    let title = doc
        .get("title")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from);

    let authors = doc
        .get("author_names")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let description = doc
        .get("description")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from);

    let published_date = doc
        .get("release_year")
        .and_then(|v| v.as_i64())
        .map(|y| y.to_string());

    let categories = doc
        .get("genres")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(clean_tag)
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();

    let cover_url = doc
        .pointer("/image/url")
        .and_then(|v| v.as_str())
        .map(String::from);

    Some(BookMetadataDto {
        title,
        authors,
        publisher: None,
        published_date,
        description,
        categories,
        cover_url,
        isbn: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hardcover_graphql_response() {
        let sample = json!({
            "data": {
                "search": {
                    "results": {
                        "hits": [{
                            "document": {
                                "title": "Dune",
                                "author_names": ["Frank Herbert"],
                                "description": "Desert planet Arrakis.",
                                "release_year": 1965,
                                "genres": ["Science Fiction", "Classic"],
                                "image": { "url": "https://assets.hardcover.app/dune.jpg" }
                            }
                        }]
                    }
                }
            }
        });

        let dto = parse_hardcover_response(&sample).unwrap();
        assert_eq!(dto.title.as_deref(), Some("Dune"));
        assert_eq!(dto.authors, vec!["Frank Herbert"]);
        assert_eq!(dto.published_date.as_deref(), Some("1965"));
        assert_eq!(dto.categories, vec!["Science Fiction", "Classic"]);
    }
}
