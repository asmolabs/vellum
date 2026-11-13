use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{Path as AxPath, Query, Request, State};
use axum::http::header::{AUTHORIZATION, CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tower_http::cors::{Any, CorsLayer};

use crate::book_service::BookService;
use crate::reader_provider::create_content_provider;
use crate::web_companion_assets::{library_page_html, reader_page_html};
use vellum_domain::VellumResult;

#[derive(Clone)]
pub struct OpdsServer {
    book_service: BookService,
}

#[derive(Debug, Deserialize)]
pub struct CatalogQuery {
    pub search: Option<String>,
    pub tag: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookDto {
    pub id: String,
    pub title: String,
    pub author: String,
    pub series: Option<String>,
    pub series_index: Option<f64>,
    pub publisher: Option<String>,
    pub publish_date: Option<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub formats: Vec<FormatDto>,
    pub has_cover: bool,
    pub added_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatDto {
    pub format: String,
    pub filename: String,
    pub filesize: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TocItemDto {
    pub index: usize,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterContentDto {
    pub chapter_index: usize,
    pub total_chapters: usize,
    pub title: String,
    pub html: String,
}

#[derive(Debug, Deserialize)]
pub struct ProgressUpdateDto {
    pub chapter_index: usize,
    pub percentage: Option<f64>,
}

impl OpdsServer {
    pub fn new(book_service: BookService) -> Self {
        Self { book_service }
    }

    pub fn router(book_service: BookService) -> Router {
        Self::router_with_token(book_service, None)
    }

    pub fn router_with_token(book_service: BookService, auth_token: Option<String>) -> Router {
        let shared = Arc::new(book_service);
        let expected_token = Arc::new(auth_token.filter(|t| !t.trim().is_empty()));
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
            .allow_headers([CONTENT_TYPE, AUTHORIZATION]);

        Router::new()
            // 1. Web Companion & Mobile Reader UI
            .route("/", get(web_library_handler))
            .route("/read/:id", get(web_reader_handler))
            // 2. REST API for Web & Mobile Clients
            .route("/api/status", get(api_status_handler))
            .route("/api/books", get(api_books_handler))
            .route("/api/books/:id", get(api_book_detail_handler))
            .route("/api/books/:id/cover", get(cover_handler))
            .route("/api/books/:id/toc", get(api_book_toc_handler))
            .route("/api/books/:id/chapter/:idx", get(api_book_chapter_handler))
            .route("/api/books/:id/progress", post(api_book_progress_handler))
            .route("/api/books/:id/download/:format", get(download_handler))
            // 3. OPDS 1.2 Catalog Feed (for KOReader, Moon+, etc.)
            .route("/opds", get(root_catalog_handler))
            .route("/opds/books", get(books_catalog_handler))
            .route("/opds/covers/:id", get(cover_handler))
            .route("/opds/download/:id/:format", get(download_handler))
            .layer(middleware::from_fn_with_state(expected_token, auth_and_security_headers))
            .layer(cors)
            .with_state(shared)
    }

    pub async fn run_server(book_service: BookService, port: u16) -> VellumResult<()> {
        let token = std::env::var("VELLUM_OPDS_TOKEN")
            .ok()
            .filter(|t| !t.trim().is_empty());
        Self::run_server_with_token(book_service, port, token).await
    }

    pub async fn run_server_with_token(
        book_service: BookService,
        port: u16,
        auth_token: Option<String>,
    ) -> VellumResult<()> {
        let app = Self::router_with_token(book_service, auth_token);
        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;
        Ok(())
    }

    pub async fn start(&self, port: u16) -> VellumResult<()> {
        Self::run_server(self.book_service.clone(), port).await
    }
}

async fn auth_and_security_headers(
    State(expected_token): State<Arc<Option<String>>>,
    req: Request,
    next: Next,
) -> Response {
    if let Some(ref token) = *expected_token {
        let header_ok = req
            .headers()
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "))
            .is_some_and(|t| t == token);

        let query_ok = req.uri().query().is_some_and(|q| {
            q.split('&')
                .any(|pair| pair.strip_prefix("token=").is_some_and(|t| t == token))
        });

        if !header_ok && !query_ok {
            return (
                StatusCode::UNAUTHORIZED,
                [(axum::http::header::WWW_AUTHENTICATE, "Bearer realm=\"Vellum\"")],
                "Unauthorized",
            )
                .into_response();
        }
    }

    let mut resp = next.run(req).await;
    let headers = resp.headers_mut();
    headers.insert(
        axum::http::header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        axum::http::header::X_FRAME_OPTIONS,
        HeaderValue::from_static("SAMEORIGIN"),
    );
    resp
}

pub fn escape_xml(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub fn get_mime_type(format: &str) -> &'static str {
    match format.trim().to_lowercase().as_str() {
        "epub" => "application/epub+zip",
        "pdf" => "application/pdf",
        "cbz" => "application/vnd.comicbook+zip",
        "cbr" => "application/vnd.comicbook-rar",
        "mobi" => "application/x-mobipocket-ebook",
        "azw" | "azw3" => "application/vnd.amazon.ebook",
        "mp3" => "audio/mpeg",
        "m4b" | "m4a" => "audio/mp4",
        "txt" | "md" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

// --- Web Handlers ---

async fn web_library_handler() -> Response {
    let html = library_page_html();
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    (StatusCode::OK, headers, html).into_response()
}

async fn web_reader_handler(AxPath(_book_id): AxPath<String>) -> Response {
    let html = reader_page_html();
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    (StatusCode::OK, headers, html).into_response()
}

// --- REST API Handlers ---

async fn api_status_handler(State(service): State<Arc<BookService>>) -> Response {
    let count = service.book_repo().count().unwrap_or(0);
    Json(serde_json::json!({
        "status": "ok",
        "version": "1.0.0",
        "total_books": count,
    }))
    .into_response()
}

async fn api_books_handler(
    State(service): State<Arc<BookService>>,
    Query(query): Query<CatalogQuery>,
) -> Response {
    let books = service
        .get_books(query.search.as_deref(), query.tag.as_deref())
        .unwrap_or_default();

    let dtos: Vec<BookDto> = books
        .into_iter()
        .map(|b| {
            let formats = b
                .formats
                .iter()
                .map(|f| FormatDto {
                    format: f.format.clone(),
                    filename: f.filename.clone(),
                    filesize: f.filesize,
                })
                .collect();
            let tags = b.tags.into_iter().map(|t| t.name).collect();
            let has_cover = b.cover_path.is_some();
            BookDto {
                id: b.id,
                title: b.title,
                author: b.author,
                series: b.series,
                series_index: b.series_index,
                publisher: b.publisher,
                publish_date: b.publish_date,
                description: b.description,
                tags,
                formats,
                has_cover,
                added_at: b.added_at,
            }
        })
        .collect();

    Json(dtos).into_response()
}

async fn api_book_detail_handler(
    State(service): State<Arc<BookService>>,
    AxPath(book_id): AxPath<String>,
) -> Response {
    if let Ok(Some(b)) = service.get_book_by_id(&book_id) {
        let formats = b
            .formats
            .iter()
            .map(|f| FormatDto {
                format: f.format.clone(),
                filename: f.filename.clone(),
                filesize: f.filesize,
            })
            .collect();
        let tags = b.tags.into_iter().map(|t| t.name).collect();
        let has_cover = b.cover_path.is_some();
        let dto = BookDto {
            id: b.id,
            title: b.title,
            author: b.author,
            series: b.series,
            series_index: b.series_index,
            publisher: b.publisher,
            publish_date: b.publish_date,
            description: b.description,
            tags,
            formats,
            has_cover,
            added_at: b.added_at,
        };
        Json(dto).into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

async fn api_book_toc_handler(
    State(service): State<Arc<BookService>>,
    AxPath(book_id): AxPath<String>,
) -> Response {
    if let Ok(Some(book)) = service.get_book_by_id(&book_id) {
        if let Some(fmt) = book
            .formats
            .iter()
            .find(|f| f.format.eq_ignore_ascii_case("epub"))
            .or_else(|| book.formats.first())
        {
            let path = service.get_file_for_format(fmt);
            if path.exists() {
                let provider = create_content_provider(&path);
                let toc_raw = provider.get_table_of_contents();
                let toc_dtos: Vec<TocItemDto> = toc_raw
                    .into_iter()
                    .map(|(index, title)| TocItemDto { index, title })
                    .collect();
                return Json(toc_dtos).into_response();
            }
        }
    }
    StatusCode::NOT_FOUND.into_response()
}

async fn api_book_chapter_handler(
    State(service): State<Arc<BookService>>,
    AxPath((book_id, chapter_idx)): AxPath<(String, usize)>,
) -> Response {
    if let Ok(Some(book)) = service.get_book_by_id(&book_id) {
        if let Some(fmt) = book
            .formats
            .iter()
            .find(|f| f.format.eq_ignore_ascii_case("epub"))
            .or_else(|| book.formats.first())
        {
            let path = service.get_file_for_format(fmt);
            if path.exists() {
                let provider = create_content_provider(&path);
                let total = provider.chapter_count();
                if chapter_idx < total {
                    let title = provider.chapter_title(chapter_idx);
                    let html = provider.chapter_html(chapter_idx);
                    return Json(ChapterContentDto {
                        chapter_index: chapter_idx,
                        total_chapters: total,
                        title,
                        html,
                    })
                    .into_response();
                }
            }
        }
    }
    StatusCode::NOT_FOUND.into_response()
}

async fn api_book_progress_handler(
    State(service): State<Arc<BookService>>,
    AxPath(book_id): AxPath<String>,
    Json(body): Json<ProgressUpdateDto>,
) -> Response {
    let percent = body.percentage.unwrap_or(0.0) * 100.0;
    let finished = percent >= 99.0;
    if service.update_reading_progress(
        &book_id,
        body.chapter_index as i32,
        0,
        percent,
        finished,
    ).is_ok() {
        return Json(serde_json::json!({ "status": "ok" })).into_response();
    }
    StatusCode::NOT_FOUND.into_response()
}

// --- OPDS Catalog Handlers ---

async fn root_catalog_handler() -> Response {
    let now = Utc::now().to_rfc3339();
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom"
      xmlns:dc="http://purl.org/dc/elements/1.1/"
      xmlns:opds="http://opds-spec.org/2010/catalog">
  <id>urn:vellum:root</id>
  <title>Vellum Desktop Ebook Library</title>
  <updated>{now}</updated>
  <author><name>Vellum</name></author>
  <link rel="self" href="/opds" type="application/atom+xml;profile=opds-catalog"/>
  <link rel="start" href="/opds" type="application/atom+xml;profile=opds-catalog"/>

  <entry>
    <title>Tous les livres</title>
    <id>urn:vellum:all</id>
    <updated>{now}</updated>
    <content type="text">Parcourir l'ensemble des livres du catalogue</content>
    <link rel="subsection" href="/opds/books" type="application/atom+xml;profile=opds-catalog"/>
  </entry>
</feed>"#
    );

    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/atom+xml;profile=opds-catalog;charset=UTF-8"),
    );
    (StatusCode::OK, headers, xml).into_response()
}

async fn books_catalog_handler(
    State(service): State<Arc<BookService>>,
    Query(query): Query<CatalogQuery>,
) -> Response {
    let books = service
        .get_books(query.search.as_deref(), query.tag.as_deref())
        .unwrap_or_default();

    let now = Utc::now().to_rfc3339();
    let mut xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom"
      xmlns:dc="http://purl.org/dc/elements/1.1/"
      xmlns:opds="http://opds-spec.org/2010/catalog">
  <id>urn:vellum:books</id>
  <title>Vellum - Catalogue</title>
  <updated>{now}</updated>
  <author><name>Vellum</name></author>
  <link rel="self" href="/opds/books" type="application/atom+xml;profile=opds-catalog"/>
  <link rel="start" href="/opds/books" type="application/atom+xml;profile=opds-catalog"/>
"#
    );

    for b in books {
        xml.push_str("  <entry>\n");
        xml.push_str(&format!("    <title>{}</title>\n", escape_xml(&b.title)));
        xml.push_str(&format!("    <id>urn:vellum:book:{}</id>\n", b.id));
        xml.push_str(&format!("    <updated>{}</updated>\n", b.added_at));
        xml.push_str(&format!("    <author><name>{}</name></author>\n", escape_xml(&b.author)));

        if let Some(ref publ) = b.publisher {
            if !publ.trim().is_empty() {
                xml.push_str(&format!("    <dc:publisher>{}</dc:publisher>\n", escape_xml(publ)));
            }
        }
        if let Some(ref date) = b.publish_date {
            if !date.trim().is_empty() {
                xml.push_str(&format!("    <dc:date>{}</dc:date>\n", escape_xml(date)));
            }
        }
        if let Some(ref desc) = b.description {
            if !desc.trim().is_empty() {
                xml.push_str(&format!("    <summary type=\"text\">{}</summary>\n", escape_xml(desc)));
            }
        }

        for t in &b.tags {
            xml.push_str(&format!(
                "    <category term=\"{}\" label=\"{}\"/>\n",
                escape_xml(&t.name),
                escape_xml(&t.name)
            ));
        }

        if b.cover_path.is_some() {
            xml.push_str(&format!(
                "    <link rel=\"http://opds-spec.org/image\" href=\"/opds/covers/{}\" type=\"image/jpeg\"/>\n",
                b.id
            ));
            xml.push_str(&format!(
                "    <link rel=\"http://opds-spec.org/image/thumbnail\" href=\"/opds/covers/{}\" type=\"image/jpeg\"/>\n",
                b.id
            ));
        }

        for fmt in &b.formats {
            let mime = get_mime_type(&fmt.format);
            xml.push_str(&format!(
                "    <link rel=\"http://opds-spec.org/acquisition\" href=\"/opds/download/{}/{}\" type=\"{}\" length=\"{}\"/>\n",
                b.id,
                fmt.format.to_lowercase(),
                mime,
                fmt.filesize
            ));
        }

        xml.push_str("  </entry>\n");
    }

    xml.push_str("</feed>");

    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/atom+xml;profile=opds-catalog;charset=UTF-8"),
    );
    (StatusCode::OK, headers, xml).into_response()
}

async fn cover_handler(
    State(service): State<Arc<BookService>>,
    AxPath(book_id): AxPath<String>,
) -> Response {
    if let Ok(Some(book)) = service.get_book_by_id(&book_id) {
        if let Some(cover_file) = service.get_cover_file(&book) {
            if let Ok(bytes) = tokio::fs::read(cover_file).await {
                let mut headers = HeaderMap::new();
                headers.insert(CONTENT_TYPE, HeaderValue::from_static("image/jpeg"));
                return (StatusCode::OK, headers, bytes).into_response();
            }
        }
    }
    StatusCode::NOT_FOUND.into_response()
}

async fn download_handler(
    State(service): State<Arc<BookService>>,
    AxPath((book_id, fmt_param)): AxPath<(String, String)>,
) -> Response {
    if let Ok(Some(book)) = service.get_book_by_id(&book_id) {
        if let Some(fmt) = book
            .formats
            .iter()
            .find(|f| f.format.eq_ignore_ascii_case(&fmt_param))
        {
            let path = service.get_file_for_format(fmt);
            if path.exists() {
                if let Ok(bytes) = tokio::fs::read(&path).await {
                    let mut headers = HeaderMap::new();
                    let mime = get_mime_type(&fmt.format);
                    if let Ok(val) = HeaderValue::from_str(mime) {
                        headers.insert(CONTENT_TYPE, val);
                    }
                    let safe_name = BookService::sanitize_filename(&fmt.filename);
                    let disp = format!("attachment; filename=\"{safe_name}\"");
                    if let Ok(val) = HeaderValue::from_str(&disp) {
                        headers.insert(CONTENT_DISPOSITION, val);
                    }
                    return (StatusCode::OK, headers, bytes).into_response();
                }
            }
        }
    }
    StatusCode::NOT_FOUND.into_response()
}
