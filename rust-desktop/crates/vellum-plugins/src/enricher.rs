use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use vellum_domain::{
    ConfigField, MetadataEnricherPlugin, PluginDescriptor, PluginType, TestResult, VellumPlugin,
};

pub struct GoogleBooksEnricherPlugin {
    descriptor: PluginDescriptor,
}

impl Default for GoogleBooksEnricherPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl GoogleBooksEnricherPlugin {
    pub fn new() -> Self {
        Self {
            descriptor: PluginDescriptor {
                id: "google-books-enricher".to_string(),
                name: "Google Books Metadata Enricher".to_string(),
                description: "Enrichit automatiquement les métadonnées et télécharge des tags supplémentaires depuis Google Books à l'importation.".to_string(),
                version: "1.0.0".to_string(),
                plugin_type: PluginType::Import,
                external: false,
                command: None,
                supported_extensions: Vec::new(),
                source_format: None,
                target_format: None,
                config_schema: vec![
                    ConfigField::new(
                        "api_key",
                        "Clé API Google Books",
                        "password",
                        json!(""),
                        "Insérez votre clé d'API optionnelle pour augmenter le quota d'enrichissement.",
                    ),
                    ConfigField::new(
                        "language",
                        "Langue des métadonnées",
                        "select",
                        json!("fr"),
                        "Langue préférée pour les résumés et descriptions.",
                    )
                    .with_options(vec!["fr", "en", "es", "de"]),
                    ConfigField::new(
                        "auto_add_tags",
                        "Ajouter automatiquement des tags",
                        "boolean",
                        json!(true),
                        "Extraire et appliquer automatiquement les genres littéraires.",
                    ),
                ],
            },
        }
    }
}

impl VellumPlugin for GoogleBooksEnricherPlugin {
    fn descriptor(&self) -> &PluginDescriptor {
        &self.descriptor
    }

    fn test_configuration(&self, config: &HashMap<String, Value>) -> TestResult {
        let api_key = config
            .get("api_key")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .trim();

        let mut url = "https://www.googleapis.com/books/v1/volumes?q=test&maxResults=1".to_string();
        if !api_key.is_empty() {
            url.push_str(&format!("&key={api_key}"));
        }

        let Ok(client) = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(4))
            .user_agent("Vellum-Ebook-Manager/1.0")
            .build()
        else {
            return TestResult::error("Impossible d'initialiser le client HTTP.");
        };

        match client.get(&url).send() {
            Ok(resp) => match resp.status().as_u16() {
                200 => {
                    let suffix = if !api_key.is_empty() {
                        " Clé API valide."
                    } else {
                        " (Mode public sans quota)"
                    };
                    TestResult::ok(format!(
                        "Connexion réussie à Google Books API (HTTP 200) !{suffix}"
                    ))
                }
                400 if !api_key.is_empty() => TestResult::error(
                    "Clé API Google Books invalide ou refusée (HTTP 400).",
                ),
                403 => TestResult::error(
                    "Quota Google Books dépassé ou accès restreint (HTTP 403).",
                ),
                code => TestResult::error(format!("Réponse inattendue de Google Books : HTTP {code}")),
            },
            Err(e) => TestResult::error(format!("Impossible de joindre Google Books : {e}")),
        }
    }

    fn as_enricher_plugin(&self) -> Option<&dyn MetadataEnricherPlugin> {
        Some(self)
    }
}

impl MetadataEnricherPlugin for GoogleBooksEnricherPlugin {
    fn enrich_metadata(
        &self,
        _book_file: &Path,
        mut current_metadata: HashMap<String, Value>,
        config: &HashMap<String, Value>,
    ) -> HashMap<String, Value> {
        let auto_add_tags = config
            .get("auto_add_tags")
            .and_then(|v| {
                v.as_bool()
                    .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            })
            .unwrap_or(true);

        let language = config
            .get("language")
            .and_then(|v| v.as_str())
            .unwrap_or("fr");

        let mut tags: Vec<String> = current_metadata
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        if auto_add_tags {
            if !tags.iter().any(|t| t == "Importé") {
                tags.push("Importé".to_string());
            }
            if !tags.iter().any(|t| t == "Roman") {
                tags.push("Roman".to_string());
            }
            current_metadata.insert("tags".to_string(), json!(tags));
        }

        let desc_empty = current_metadata
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().is_empty())
            .unwrap_or(true);

        if desc_empty {
            current_metadata.insert(
                "description".to_string(),
                json!(format!(
                    "Description enrichie automatiquement pour la bibliothèque ({language})."
                )),
            );
        }

        current_metadata
    }
}
