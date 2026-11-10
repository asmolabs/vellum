use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use regex::Regex;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use tracing::{debug, info, warn};
use vellum_domain::{
    ConfigField, MetadataEnricherPlugin, PluginDescriptor, PluginType, TestResult, VellumPlugin,
};

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

    fn build_http_client() -> Option<reqwest::blocking::Client> {
        reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(6))
            .user_agent("Vellum-Ebook-Manager/1.0 (https://github.com/asmolabs/library)")
            .build()
            .ok()
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
        });

        handle.join().unwrap_or_else(|_| TestResult::error("Le thread réseau s'est arrêté inopinément."))
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
            .and_then(|v| v.as_bool().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
            .unwrap_or(true);

        let fetch_cover = config
            .get("fetch_cover")
            .and_then(|v| v.as_bool().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
            .unwrap_or(true);

        let overwrite_existing = config
            .get("overwrite_existing")
            .and_then(|v| v.as_bool().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
            .unwrap_or(false);

        let fallback_openlibrary = config
            .get("fallback_openlibrary")
            .and_then(|v| v.as_bool().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
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
            let Some(client) = Self::build_http_client() else {
                warn!("GoogleBooksEnricherPlugin: Unable to construct HTTP client.");
                return (None, None);
            };

            let mut matched: Option<BookMetadataDto> = None;

            // 1. Try Google Books
            match query_google_books(
                &client,
                if cur_isbn_clone.is_empty() { None } else { Some(&cur_isbn_clone) },
                if cur_title_clone.is_empty() { None } else { Some(&cur_title_clone) },
                if cur_author_clone.is_empty() { None } else { Some(&cur_author_clone) },
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
                    &client,
                    if cur_isbn_clone.is_empty() { None } else { Some(&cur_isbn_clone) },
                    if cur_title_clone.is_empty() { None } else { Some(&cur_title_clone) },
                    if cur_author_clone.is_empty() { None } else { Some(&cur_author_clone) },
                ) {
                    Ok(Some(dto)) => {
                        info!("Open Library fallback matched: {:?}", dto.title);
                        matched = Some(dto);
                    }
                    Ok(None) => {}
                    Err(e) => {
                        warn!("Open Library fallback failed for '{}': {}", cur_title_clone, e);
                    }
                }
            }

            let mut cover_bytes_opt = None;
            if let Some(ref m) = matched {
                if fetch_cover {
                    if let Some(ref url) = m.cover_url {
                        cover_bytes_opt = download_image(&client, url);
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
            if !t.trim().is_empty() && (overwrite_existing || current_title.is_empty() || is_generic_value(&current_title)) {
                current_metadata.insert("title".to_string(), json!(t));
            }
        }

        if !m.authors.is_empty() {
            let joined_authors = m.authors.join(", ");
            if overwrite_existing || current_author.is_empty() || is_generic_value(&current_author) {
                current_metadata.insert("author".to_string(), json!(joined_authors));
            }
        }

        if let Some(ref p) = m.publisher {
            if !p.trim().is_empty() && (overwrite_existing || clean_string(current_metadata.get("publisher")).is_empty()) {
                current_metadata.insert("publisher".to_string(), json!(p));
            }
        }

        if let Some(ref d) = m.published_date {
            if !d.trim().is_empty() && (overwrite_existing || clean_string(current_metadata.get("publishDate")).is_empty()) {
                current_metadata.insert("publishDate".to_string(), json!(d));
            }
        }

        if let Some(ref desc) = m.description {
            if !desc.trim().is_empty() && (overwrite_existing || clean_string(current_metadata.get("description")).is_empty()) {
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
        let has_existing_cover = current_metadata.get("coverData").is_some();
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

// ---------------------------------------------------------------------------
// Google Books API query & parsing
// ---------------------------------------------------------------------------

fn query_google_books(
    client: &reqwest::blocking::Client,
    isbn: Option<&str>,
    title: Option<&str>,
    author: Option<&str>,
    language: &str,
    api_key: &str,
) -> Result<Option<BookMetadataDto>, String> {
    let clean_isbn = isbn
        .map(|s| s.chars().filter(|c| c.is_ascii_digit() || *c == 'X' || *c == 'x').collect::<String>())
        .unwrap_or_default();

    let query_param = if clean_isbn.len() == 10 || clean_isbn.len() == 13 {
        format!("isbn:{clean_isbn}")
    } else if let Some(t) = title.filter(|s| !s.trim().is_empty() && !is_generic_value(s)) {
        let clean_t = clean_term(t);
        if let Some(a) = author.filter(|s| !s.trim().is_empty() && !is_generic_value(s)) {
            format!("intitle:\"{}\"+inauthor:\"{}\"", clean_t, clean_term(a))
        } else {
            format!("intitle:\"{clean_t}\"")
        }
    } else {
        return Ok(None);
    };

    let execute_search = |q: &str| -> Result<Option<Value>, String> {
        let mut url = format!(
            "https://www.googleapis.com/books/v1/volumes?q={}&maxResults=5",
            url_encode(q)
        );
        if !language.is_empty() && !language.eq_ignore_ascii_case("all") {
            url.push_str(&format!("&langRestrict={}", url_encode(language)));
        }
        if !api_key.is_empty() {
            url.push_str(&format!("&key={}", url_encode(api_key)));
        }

        let resp = client
            .get(&url)
            .send()
            .map_err(|e| format!("HTTP request failed: {e}"))?;

        let status = resp.status().as_u16();
        if status == 429 || status == 403 {
            warn!("Google Books quota exceeded / restricted (HTTP {status})");
            return Ok(None);
        }
        if status != 200 {
            warn!("Google Books returned unexpected HTTP {status}");
            return Ok(None);
        }

        let body: Value = resp
            .json()
            .map_err(|e| format!("Failed to parse Google Books JSON: {e}"))?;
        Ok(Some(body))
    };

    let mut body = execute_search(&query_param)?;

    // Relaxed search if combined query yielded 0 results
    if let Some(ref b) = body {
        let items_len = b.get("items").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
        if items_len == 0 && query_param.contains("+inauthor:") {
            if let Some(t) = title {
                let relaxed_query = format!("intitle:\"{}\"", clean_term(t));
                if let Ok(Some(rel_body)) = execute_search(&relaxed_query) {
                    body = Some(rel_body);
                }
            }
        }
    }

    let Some(root) = body else {
        return Ok(None);
    };

    let Some(items) = root.get("items").and_then(|v| v.as_array()) else {
        return Ok(None);
    };

    if items.is_empty() {
        return Ok(None);
    }

    let volume_info = &items[0]["volumeInfo"];
    Ok(Some(parse_google_volume_info(volume_info)))
}

fn parse_google_volume_info(volume_info: &Value) -> BookMetadataDto {
    let raw_title = volume_info.get("title").and_then(|v| v.as_str());
    let subtitle = volume_info.get("subtitle").and_then(|v| v.as_str());
    let title = match (raw_title, subtitle) {
        (Some(t), Some(s)) if !s.trim().is_empty() => Some(format!("{t} : {s}")),
        (Some(t), _) => Some(t.to_string()),
        _ => None,
    };

    let authors: Vec<String> = volume_info
        .get("authors")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|a| a.as_str().map(str::trim))
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();

    let publisher = volume_info
        .get("publisher")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let published_date = volume_info
        .get("publishedDate")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let description = volume_info
        .get("description")
        .and_then(|v| v.as_str())
        .map(strip_html)
        .filter(|s| !s.trim().is_empty());

    // ISBN_13 or ISBN_10
    let mut found_isbn = None;
    if let Some(id_list) = volume_info.get("industryIdentifiers").and_then(|v| v.as_array()) {
        for id_node in id_list {
            let id_type = id_node.get("type").and_then(|v| v.as_str()).unwrap_or("");
            let val = id_node.get("identifier").and_then(|v| v.as_str()).unwrap_or("").trim();
            if id_type.eq_ignore_ascii_case("ISBN_13") {
                found_isbn = Some(val.to_string());
                break;
            } else if id_type.eq_ignore_ascii_case("ISBN_10") && found_isbn.is_none() {
                found_isbn = Some(val.to_string());
            }
        }
    }

    // Categories / Tags
    let mut categories = Vec::new();
    if let Some(cat_node) = volume_info.get("categories").and_then(|v| v.as_array()) {
        for c in cat_node {
            if let Some(text) = c.as_str() {
                for part in text.split(&['/', ';', ','][..]) {
                    let clean = clean_tag(part);
                    if !clean.is_empty() && !categories.iter().any(|existing| existing == &clean) {
                        categories.push(clean);
                    }
                }
            }
        }
    }

    // Image links (HD prioritized)
    let cover_url = volume_info
        .get("imageLinks")
        .and_then(|links| {
            links.get("extraLarge")
                .or_else(|| links.get("large"))
                .or_else(|| links.get("medium"))
                .or_else(|| links.get("thumbnail"))
                .or_else(|| links.get("smallThumbnail"))
        })
        .and_then(|v| v.as_str())
        .map(|u| u.replace("http://", "https://"));

    BookMetadataDto {
        title,
        authors,
        publisher,
        published_date,
        description,
        categories,
        cover_url,
        isbn: found_isbn,
    }
}

// ---------------------------------------------------------------------------
// Open Library fallback
// ---------------------------------------------------------------------------

fn query_open_library(
    client: &reqwest::blocking::Client,
    isbn: Option<&str>,
    title: Option<&str>,
    author: Option<&str>,
) -> Result<Option<BookMetadataDto>, String> {
    let clean_isbn = isbn
        .map(|s| s.chars().filter(|c| c.is_ascii_digit() || *c == 'X' || *c == 'x').collect::<String>())
        .unwrap_or_default();

    // Strategy A: By ISBN
    if clean_isbn.len() == 10 || clean_isbn.len() == 13 {
        let url = format!(
            "https://openlibrary.org/api/books?bibkeys=ISBN:{}&format=json&jscmd=data",
            clean_isbn
        );
        if let Ok(resp) = client.get(&url).send() {
            if resp.status().is_success() {
                if let Ok(root) = resp.json::<Value>() {
                    let key = format!("ISBN:{clean_isbn}");
                    if let Some(data) = root.get(&key) {
                        return Ok(Some(parse_open_library_data(data, &clean_isbn)));
                    }
                }
            }
        }
    }

    // Strategy B: By Title + Author search
    let Some(t) = title.filter(|s| !s.trim().is_empty() && !is_generic_value(s)) else {
        return Ok(None);
    };

    let mut search_url = format!(
        "https://openlibrary.org/search.json?title={}&limit=3",
        url_encode(t)
    );
    if let Some(a) = author.filter(|s| !s.trim().is_empty() && !is_generic_value(s)) {
        search_url.push_str(&format!("&author={}", url_encode(a)));
    }

    let resp = client
        .get(&search_url)
        .send()
        .map_err(|e| format!("Open Library search failed: {e}"))?;

    if !resp.status().is_success() {
        return Ok(None);
    }

    let root: Value = resp
        .json()
        .map_err(|e| format!("Failed to parse Open Library JSON: {e}"))?;

    let Some(docs) = root.get("docs").and_then(|v| v.as_array()) else {
        return Ok(None);
    };

    if docs.is_empty() {
        return Ok(None);
    }

    Ok(Some(parse_open_library_doc(&docs[0])))
}

fn parse_open_library_data(data: &Value, fallback_isbn: &str) -> BookMetadataDto {
    let title = data.get("title").and_then(|v| v.as_str()).map(String::from);
    let authors: Vec<String> = data
        .get("authors")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|a| a.get("name").and_then(|n| n.as_str()).map(str::trim))
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();

    let publisher = data
        .get("publishers")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|p| p.get("name").and_then(|n| n.as_str()))
        .map(String::from);

    let published_date = data.get("publish_date").and_then(|v| v.as_str()).map(String::from);

    let cover_url = data
        .get("cover")
        .and_then(|c| c.get("large").or_else(|| c.get("medium")).or_else(|| c.get("small")))
        .and_then(|v| v.as_str())
        .map(String::from);

    let mut categories = Vec::new();
    if let Some(subjects) = data.get("subjects").and_then(|v| v.as_array()) {
        for s in subjects {
            if let Some(name) = s.get("name").and_then(|n| n.as_str()) {
                let clean = clean_tag(name);
                if !clean.is_empty() && categories.len() < 8 && !categories.iter().any(|c| c == &clean) {
                    categories.push(clean);
                }
            }
        }
    }

    BookMetadataDto {
        title,
        authors,
        publisher,
        published_date,
        description: None,
        categories,
        cover_url,
        isbn: Some(fallback_isbn.to_string()),
    }
}

fn parse_open_library_doc(doc: &Value) -> BookMetadataDto {
    let title = doc.get("title").and_then(|v| v.as_str()).map(String::from);

    let authors: Vec<String> = doc
        .get("author_name")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|a| a.as_str().map(str::trim))
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();

    let publisher = doc
        .get("publisher")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.as_str())
        .map(String::from);

    let published_date = doc
        .get("first_publish_year")
        .and_then(|v| v.as_i64())
        .map(|y| y.to_string());

    let found_isbn = doc
        .get("isbn")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.as_str())
        .map(String::from);

    let mut categories = Vec::new();
    if let Some(subjects) = doc.get("subject").and_then(|v| v.as_array()) {
        for s in subjects {
            if let Some(name) = s.as_str() {
                let clean = clean_tag(name);
                if !clean.is_empty() && categories.len() < 8 && !categories.iter().any(|c| c == &clean) {
                    categories.push(clean);
                }
            }
        }
    }

    let cover_url = doc
        .get("cover_i")
        .and_then(|v| v.as_i64())
        .filter(|id| *id > 0)
        .map(|id| format!("https://covers.openlibrary.org/b/id/{id}-L.jpg"));

    BookMetadataDto {
        title,
        authors,
        publisher,
        published_date,
        description: None,
        categories,
        cover_url,
        isbn: found_isbn,
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn download_image(client: &reqwest::blocking::Client, url: &str) -> Option<Vec<u8>> {
    let resp = client.get(url).send().ok()?;
    if !resp.status().is_success() {
        return None;
    }
    resp.bytes().ok().map(|b| b.to_vec())
}

fn detect_image_extension(bytes: &[u8], url: &str) -> &'static str {
    if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        ".png"
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        ".jpg"
    } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        ".webp"
    } else if url.contains(".png") {
        ".png"
    } else if url.contains(".webp") {
        ".webp"
    } else {
        ".jpg"
    }
}

fn clean_string(val: Option<&Value>) -> String {
    val.and_then(|v| v.as_str()).unwrap_or("").trim().to_string()
}

fn clean_term(s: &str) -> String {
    s.replace(['"', '\'', '(', ')', '[', ']', '{', '}'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn clean_tag(s: &str) -> String {
    let t = s.trim().trim_matches(&['"', '\'', '#'][..]);
    if t.is_empty() {
        return String::new();
    }
    let mut chars = t.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => {
            let mut capitalized = first.to_uppercase().to_string();
            capitalized.push_str(chars.as_str());
            capitalized
        }
    }
}

fn is_generic_value(s: &str) -> bool {
    let lower = s.trim().to_lowercase();
    matches!(
        lower.as_str(),
        "unknown"
            | "unknown author"
            | "sans titre"
            | "inconnu"
            | "auteur inconnu"
            | "n/a"
            | "none"
            | "livre"
            | "book"
    )
}

fn strip_html(raw: &str) -> String {
    let tag_re = Regex::new(r"<[^>]*>").unwrap();
    let stripped = tag_re.replace_all(raw, " ");
    stripped
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn url_encode(s: &str) -> String {
    let mut encoded = String::new();
    for b in s.as_bytes() {
        match *b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(*b as char);
            }
            b' ' => encoded.push('+'),
            _ => {
                encoded.push_str(&format!("%{:02X}", b));
            }
        }
    }
    encoded
}
