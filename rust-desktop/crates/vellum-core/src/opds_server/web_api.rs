//! Endpoints Web Companion (HTML) et API REST JSON du serveur embarqué.

use std::sync::Arc;

use axum::extract::{Path as AxPath, Query, State};
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

use super::xml_feed::CatalogQuery;
use crate::book_service::BookService;
use crate::catalog_exporter::{CatalogExporter, CatalogFormat};
use crate::reader_provider::create_content_provider;
use crate::web_companion_assets::{library_page_html, reader_page_html};
use vellum_domain::Book;

#[derive(Debug, Deserialize)]
pub struct ExportQuery {
    pub format: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ContentSearchQuery {
    pub q: Option<String>,
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

impl BookDto {
    fn from_book(b: Book) -> Self {
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
        Self {
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
    }
}

pub(super) async fn web_library_handler() -> Response {
    let html = library_page_html();
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    (StatusCode::OK, headers, html).into_response()
}

pub(super) async fn web_reader_handler(AxPath(_book_id): AxPath<String>) -> Response {
    let html = reader_page_html();
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    (StatusCode::OK, headers, html).into_response()
}

pub(super) async fn api_status_handler(State(service): State<Arc<BookService>>) -> Response {
    let count = service.book_repo().count().unwrap_or(0);
    Json(serde_json::json!({
        "status": "ok",
        "version": "1.0.0",
        "total_books": count,
    }))
    .into_response()
}

pub(super) async fn api_export_handler(
    State(service): State<Arc<BookService>>,
    Query(query): Query<ExportQuery>,
) -> Response {
    let books = service.get_books(None, None).unwrap_or_default();
    let raw_fmt = query.format.as_deref().unwrap_or("csv").to_lowercase();
    let (fmt, content_type, filename) = match raw_fmt.as_str() {
        "json" => (
            CatalogFormat::Json,
            "application/json; charset=utf-8",
            "vellum-catalogue.json",
        ),
        "bib" | "bibtex" => (
            CatalogFormat::Bibtex,
            "application/x-bibtex; charset=utf-8",
            "vellum-catalogue.bib",
        ),
        _ => (
            CatalogFormat::Csv,
            "text/csv; charset=utf-8",
            "vellum-catalogue.csv",
        ),
    };

    match CatalogExporter::export_to_string(&books, fmt) {
        Ok(body) => (
            [
                (CONTENT_TYPE, content_type),
                (
                    CONTENT_DISPOSITION,
                    &format!("attachment; filename=\"{filename}\""),
                ),
            ],
            body,
        )
            .into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub(super) async fn api_search_content_handler(
    State(service): State<Arc<BookService>>,
    Query(query): Query<ContentSearchQuery>,
) -> Response {
    let q = query.q.unwrap_or_default();
    match service.search_inside_books(&q, 25) {
        Ok(hits) => Json(hits).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub(super) async fn api_book_notes_handler(
    State(service): State<Arc<BookService>>,
    AxPath(book_id): AxPath<String>,
) -> Response {
    match service.export_book_notes_markdown(&book_id, None) {
        Ok(md) => ([(CONTENT_TYPE, "text/markdown; charset=utf-8")], md).into_response(),
        Err(e) => (StatusCode::NOT_FOUND, e.to_string()).into_response(),
    }
}

pub(super) async fn api_books_handler(
    State(service): State<Arc<BookService>>,
    Query(query): Query<CatalogQuery>,
) -> Response {
    let books = service
        .get_books(query.search.as_deref(), query.tag.as_deref())
        .unwrap_or_default();

    let dtos: Vec<BookDto> = books.into_iter().map(BookDto::from_book).collect();
    Json(dtos).into_response()
}

pub(super) async fn api_book_detail_handler(
    State(service): State<Arc<BookService>>,
    AxPath(book_id): AxPath<String>,
) -> Response {
    if let Ok(Some(b)) = service.get_book_by_id(&book_id) {
        Json(BookDto::from_book(b)).into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

pub(super) async fn api_book_toc_handler(
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

pub(super) async fn api_book_chapter_handler(
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

pub(super) async fn api_book_progress_handler(
    State(service): State<Arc<BookService>>,
    AxPath(book_id): AxPath<String>,
    Json(body): Json<ProgressUpdateDto>,
) -> Response {
    let percent = body.percentage.unwrap_or(0.0) * 100.0;
    let finished = percent >= 99.0;
    if service
        .update_reading_progress(&book_id, body.chapter_index as i32, 0, percent, finished)
        .is_ok()
    {
        return Json(serde_json::json!({ "status": "ok" })).into_response();
    }
    StatusCode::NOT_FOUND.into_response()
}

pub(super) async fn api_i18n_handler(State(service): State<Arc<BookService>>) -> impl IntoResponse {
    let locale = service.get_ui_locale();
    let payload = serde_json::json!({
        "locale": locale.as_code(),
        "messages": crate::i18n::catalog_for(locale),
    });
    Json(payload)
}
