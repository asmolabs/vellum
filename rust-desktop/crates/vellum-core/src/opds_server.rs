use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{Path as AxPath, Query, State};
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use chrono::Utc;
use serde::Deserialize;
use tower_http::cors::CorsLayer;

use crate::book_service::BookService;
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

impl OpdsServer {
    pub fn new(book_service: BookService) -> Self {
        Self { book_service }
    }

    pub fn router(book_service: BookService) -> Router {
        let shared = Arc::new(book_service);
        Router::new()
            .route("/opds", get(root_catalog_handler))
            .route("/opds/books", get(books_catalog_handler))
            .route("/opds/covers/:id", get(cover_handler))
            .route("/opds/download/:id/:format", get(download_handler))
            .layer(CorsLayer::permissive())
            .with_state(shared)
    }

    pub async fn run_server(book_service: BookService, port: u16) -> VellumResult<()> {
        let app = Self::router(book_service);
        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;
        Ok(())
    }

    pub async fn start(&self, port: u16) -> VellumResult<()> {
        Self::run_server(self.book_service.clone(), port).await
    }
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
  <link rel="start" href="/opds" type="application/atom+xml;profile=opds-catalog"/>
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
                    let disp = format!("attachment; filename=\"{}\"", fmt.filename);
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
