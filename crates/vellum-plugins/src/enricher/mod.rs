mod google_books;
mod helpers;
mod open_library;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;
use std::time::Duration;
use tracing::{debug, info, warn};
use vellum_domain::{
    ConfigField, MetadataEnricherPlugin, PluginDescriptor, PluginType, TestResult, VellumPlugin,
};

use self::google_books::query_google_books;
use self::helpers::{clean_string, detect_image_extension, download_image, is_generic_value};
use self::open_library::query_open_library;

#[derive(Debug, Clone, Default)]
pub struct BookMetadataDto {
    pub title: Option<String>,
    pub authors: Vec<String>,
    pub publisher: Option<String>,
    pub published_date: Option<String>,
    pub description: Option<String>,
    pub categories: Vec<String>,
    pub cover_url: Option<String>,
    pub isbn: Option<String>,
}

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
                description: "Enrichit automatiquement les métadonnées (titre, auteur, résumé, éditeur, date, tags, couverture HD) via Google Books API avec secours Open Library.".to_string(),
                version: "1.1.0".to_string(),
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
                        "Insérez votre clé d'API Google Books (optionnelle, recommandée pour quotas élevés).",
                    ),
                    ConfigField::new(
                        "language",
                        "Langue des métadonnées",
                        "select",
                        json!("fr"),
                        "Langue préférée pour les résumés et résultats.",
                    )
                    .with_options(vec!["fr", "en", "es", "de", "all"]),
                    ConfigField::new(
                        "auto_add_tags",
                        "Ajouter automatiquement des tags",
                        "boolean",
                        json!(true),
                        "Extraire et appliquer automatiquement les catégories littéraires comme tags.",
                    ),
                    ConfigField::new(
                        "fetch_cover",
                        "Télécharger la couverture",
                        "boolean",
                        json!(true),
                        "Télécharger automatiquement la jaquette haute résolution si absente.",
                    ),
                    ConfigField::new(
                        "overwrite_existing",
                        "Écraser les champs existants",
                        "boolean",
                        json!(false),
                        "Remplacer les champs déjà renseignés (titre, description, etc.).",
                    ),
                    ConfigField::new(
                        "fallback_openlibrary",
                        "Secours Open Library",
                        "boolean",
                        json!(true),
                        "Interroger Open Library si Google Books est indisponible ou sans résultat.",
                    ),
                ],
            },
        }
    }

    fn shared_http_client() -> Option<&'static reqwest::blocking::Client> {
        static HTTP_CLIENT: LazyLock<Option<reqwest::blocking::Client>> = LazyLock::new(|| {
            reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(6))
                .user_agent("Vellum-Ebook-Manager/1.0 (https://github.com/asmolabs/library)")
                .build()
                .ok()
        });
        HTTP_CLIENT.as_ref()
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
            .trim()
            .to_string();

        let handle = std::thread::spawn(move || {
            let mut url =
                "https://www.googleapis.com/books/v1/volumes?q=test&maxResults=1".to_string();
            if !api_key.is_empty() {
                url.push_str(&format!("&key={api_key}"));
            }

            let Some(client) = Self::shared_http_client() else {
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
                    400 if !api_key.is_empty() => {
                        TestResult::error("Clé API Google Books invalide ou refusée (HTTP 400).")
                    }
                    403 => TestResult::error(
                        "Quota Google Books dépassé ou accès restreint (HTTP 403).",
                    ),
                    code => TestResult::error(format!(
                        "Réponse inattendue de Google Books : HTTP {code}"
                    )),
                },
                Err(e) => TestResult::error(format!("Impossible de joindre Google Books : {e}")),
            }
        });

        handle
            .join()
            .unwrap_or_else(|_| TestResult::error("Le thread réseau s'est arrêté inopinément."))
    }

    fn as_enricher_plugin(&self) -> Option<&dyn MetadataEnricherPlugin> {
        Some(self)
    }
}

impl MetadataEnricherPlugin for GoogleBooksEnricherPlugin {
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
            .trim();

        let language = config
            .get("language")
            .and_then(|v| v.as_str())
            .unwrap_or("fr")
            .trim();

        let auto_add_tags = config
            .get("auto_add_tags")
            .and_then(|v| {
                v.as_bool()
                    .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            })
            .unwrap_or(true);

        let fetch_cover = config
            .get("fetch_cover")
            .and_then(|v| {
                v.as_bool()
                    .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            })
            .unwrap_or(true);

        let overwrite_existing = config
            .get("overwrite_existing")
            .and_then(|v| {
                v.as_bool()
                    .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            })
            .unwrap_or(false);

        let fallback_openlibrary = config
            .get("fallback_openlibrary")
            .and_then(|v| {
                v.as_bool()
                    .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            })
            .unwrap_or(true);

        let mut current_title = clean_string(current_metadata.get("title"));
        let current_author = clean_string(current_metadata.get("author"));
        let current_isbn = clean_string(current_metadata.get("isbn"));

        if current_title.is_empty() && book_file.file_name().is_some() {
            if let Some(stem) = book_file.file_stem().and_then(|s| s.to_str()) {
                current_title = stem.to_string();
            }
        }

        let api_key_str = api_key.to_string();
        let lang_str = language.to_string();
        let cur_isbn_clone = current_isbn.clone();
        let cur_title_clone = current_title.clone();
        let cur_author_clone = current_author.clone();

        let handle = std::thread::spawn(move || {
            let Some(client) = Self::shared_http_client() else {
                warn!("GoogleBooksEnricherPlugin: Unable to construct HTTP client.");
                return (None, None);
            };

            let mut matched: Option<BookMetadataDto> = None;

            // 1. Try Google Books
            match query_google_books(
                client,
                if cur_isbn_clone.is_empty() {
                    None
                } else {
                    Some(&cur_isbn_clone)
                },
                if cur_title_clone.is_empty() {
                    None
                } else {
                    Some(&cur_title_clone)
                },
                if cur_author_clone.is_empty() {
                    None
                } else {
                    Some(&cur_author_clone)
                },
                &lang_str,
                &api_key_str,
            ) {
                Ok(Some(dto)) => {
                    debug!("Google Books matched: {:?}", dto.title);
                    matched = Some(dto);
                }
                Ok(None) => {
                    debug!("Google Books returned 0 matches for '{}'", cur_title_clone);
                }
                Err(e) => {
                    warn!("Google Books query failed for '{}': {}", cur_title_clone, e);
                }
            }

            // 2. Fallback to Open Library
            if matched.is_none() && fallback_openlibrary {
                match query_open_library(
                    client,
                    if cur_isbn_clone.is_empty() {
                        None
                    } else {
                        Some(&cur_isbn_clone)
                    },
                    if cur_title_clone.is_empty() {
                        None
                    } else {
                        Some(&cur_title_clone)
                    },
                    if cur_author_clone.is_empty() {
                        None
                    } else {
                        Some(&cur_author_clone)
                    },
                ) {
                    Ok(Some(dto)) => {
                        info!("Open Library fallback matched: {:?}", dto.title);
                        matched = Some(dto);
                    }
                    Ok(None) => {}
                    Err(e) => {
                        warn!(
                            "Open Library fallback failed for '{}': {}",
                            cur_title_clone, e
                        );
                    }
                }
            }

            let mut cover_bytes_opt = None;
            if let Some(ref m) = matched {
                if fetch_cover {
                    if let Some(ref url) = m.cover_url {
                        cover_bytes_opt = download_image(client, url);
                    }
                }
            }

            (matched, cover_bytes_opt)
        });

        let (matched, downloaded_cover) = handle.join().unwrap_or((None, None));

        let Some(m) = matched else {
            return current_metadata;
        };

        // Apply metadata fields
        if let Some(ref t) = m.title {
            if !t.trim().is_empty()
                && (overwrite_existing
                    || current_title.is_empty()
                    || is_generic_value(&current_title))
            {
                current_metadata.insert("title".to_string(), json!(t));
            }
        }

        if !m.authors.is_empty() {
            let joined_authors = m.authors.join(", ");
            if overwrite_existing || current_author.is_empty() || is_generic_value(&current_author)
            {
                current_metadata.insert("author".to_string(), json!(joined_authors));
            }
        }

        if let Some(ref p) = m.publisher {
            if !p.trim().is_empty()
                && (overwrite_existing
                    || clean_string(current_metadata.get("publisher")).is_empty())
            {
                current_metadata.insert("publisher".to_string(), json!(p));
            }
        }

        if let Some(ref d) = m.published_date {
            if !d.trim().is_empty()
                && (overwrite_existing
                    || clean_string(current_metadata.get("publishDate")).is_empty())
            {
                current_metadata.insert("publishDate".to_string(), json!(d));
            }
        }

        if let Some(ref desc) = m.description {
            if !desc.trim().is_empty()
                && (overwrite_existing
                    || clean_string(current_metadata.get("description")).is_empty())
            {
                current_metadata.insert("description".to_string(), json!(desc));
            }
        }

        if let Some(ref isbn_val) = m.isbn {
            if !isbn_val.trim().is_empty() && (overwrite_existing || current_isbn.is_empty()) {
                current_metadata.insert("isbn".to_string(), json!(isbn_val));
            }
        }

        // Apply tags / categories
        if auto_add_tags && !m.categories.is_empty() {
            let mut tags: Vec<String> = current_metadata
                .get("tags")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            for cat in m.categories {
                if !cat.is_empty() && !tags.iter().any(|t| t.eq_ignore_ascii_case(&cat)) {
                    tags.push(cat);
                }
            }
            current_metadata.insert("tags".to_string(), json!(tags));
        }

        // Apply downloaded cover image if present
        let has_existing_cover = current_metadata.contains_key("coverData");
        if fetch_cover && (!has_existing_cover || overwrite_existing) {
            if let Some(cover_bytes) = downloaded_cover {
                if cover_bytes.len() > 500 {
                    let cover_url = m.cover_url.as_deref().unwrap_or("");
                    let ext = detect_image_extension(&cover_bytes, cover_url);
                    let b64 = BASE64.encode(&cover_bytes);
                    current_metadata.insert("coverData".to_string(), json!(b64));
                    current_metadata.insert("coverExt".to_string(), json!(ext));
                    info!("Downloaded cover image ({} bytes)", cover_bytes.len());
                }
            }
        }

        current_metadata
    }
}
