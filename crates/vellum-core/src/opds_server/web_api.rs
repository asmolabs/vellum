//! Endpoints Web Companion (HTML) et API REST JSON du serveur embarqué.

use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Extension, Path as AxPath, Query, State};
use axum::http::header::{
    ACCEPT_RANGES, CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE, RANGE,
};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

use super::xml_feed::CatalogQuery;
use crate::book_service::{BookService, UpdateMetadataRequest};
use crate::catalog_exporter::{CatalogExporter, CatalogFormat};
use crate::reader_provider::create_content_provider;
use crate::sync_service::SyncService;
use crate::web_companion_assets::{library_page_html, reader_page_html};
use vellum_domain::{Book, CloudSyncMode};

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
    pub progress_percent: f64,
    pub is_finished: bool,
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

#[derive(Debug, Deserialize)]
pub struct UploadQuery {
    pub filename: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MetadataUpdateDto {
    pub title: String,
    pub author: String,
    pub series: Option<String>,
    pub series_index: Option<f64>,
    pub publisher: Option<String>,
    pub publish_date: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
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
            progress_percent: b.progress_percent,
            is_finished: b.is_finished,
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

pub(super) async fn api_book_upload_handler(
    State(service): State<Arc<BookService>>,
    Query(query): Query<UploadQuery>,
    body: Bytes,
) -> Response {
    if body.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Empty upload body" })),
        )
            .into_response();
    }

    let raw_name = query
        .filename
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("book.epub");
    let safe_name = BookService::sanitize_filename(raw_name);

    let res = tokio::task::spawn_blocking(move || -> Result<Book, String> {
        let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
        let file_path = dir.path().join(&safe_name);
        std::fs::write(&file_path, &body).map_err(|e| e.to_string())?;
        service.import_book(&file_path).map_err(|e| e.to_string())
    })
    .await;

    match res {
        Ok(Ok(book)) => (StatusCode::CREATED, Json(BookDto::from_book(book))).into_response(),
        Ok(Err(err)) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": err })),
        )
            .into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": err.to_string() })),
        )
            .into_response(),
    }
}

pub(super) async fn api_book_metadata_handler(
    State(service): State<Arc<BookService>>,
    AxPath(book_id): AxPath<String>,
    Json(body): Json<MetadataUpdateDto>,
) -> Response {
    let req = UpdateMetadataRequest {
        book_id: &book_id,
        title: &body.title,
        author: &body.author,
        series: body.series.as_deref(),
        series_index: body.series_index,
        publisher: body.publisher.as_deref(),
        publish_date: body.publish_date.as_deref(),
        isbn: None,
        description: body.description.as_deref(),
        tag_names: body.tags,
    };

    match service.update_book_metadata_full(req) {
        Ok(updated) => (StatusCode::OK, Json(BookDto::from_book(updated))).into_response(),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": err.to_string() })),
        )
            .into_response(),
    }
}

pub(super) async fn api_stats_handler(State(service): State<Arc<BookService>>) -> Response {
    let books = service.book_repo().find_all().unwrap_or_default();
    let total_books = books.len();
    let finished_books = books.iter().filter(|b| b.is_finished).count();
    let reading_books = books
        .iter()
        .filter(|b| !b.is_finished && b.progress_percent > 0.0)
        .count();
    let unread_books = total_books.saturating_sub(finished_books + reading_books);

    let mut series_set = std::collections::HashSet::new();
    let mut format_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for b in &books {
        if let Some(ref s) = b.series {
            let trimmed = s.trim();
            if !trimmed.is_empty() {
                series_set.insert(trimmed.to_string());
            }
        }
        for f in &b.formats {
            *format_counts.entry(f.format.to_uppercase()).or_insert(0) += 1;
        }
    }

    Json(serde_json::json!({
        "total_books": total_books,
        "reading_books": reading_books,
        "finished_books": finished_books,
        "unread_books": unread_books,
        "total_series": series_set.len(),
        "formats": format_counts,
    }))
    .into_response()
}

pub(super) async fn api_sync_status_handler(
    Extension(sync_opt): Extension<Option<Arc<SyncService>>>,
) -> Response {
    let Some(sync) = sync_opt else {
        return Json(serde_json::json!({
            "configured": false,
            "available": false,
            "enabled": false,
            "mode": "none",
            "last_synced_at": null,
        }))
        .into_response();
    };

    let cfg = sync.config();
    let mode = match cfg.mode {
        CloudSyncMode::GoogleDriveApi => "google_drive_api",
        CloudSyncMode::GoogleDriveFolder => "google_drive_folder",
    };

    Json(serde_json::json!({
        "configured": true,
        "available": sync.is_backend_available(),
        "enabled": cfg.enabled,
        "mode": mode,
        "account_email": cfg.google_account_email,
        "last_synced_at": cfg.last_synced_at,
    }))
    .into_response()
}

pub(super) async fn api_book_audio_handler(
    State(service): State<Arc<BookService>>,
    AxPath(book_id): AxPath<String>,
    req_headers: HeaderMap,
) -> Response {
    let Ok(Some(book)) = service.get_book_by_id(&book_id) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Some(fmt) = book.formats.iter().find(|f| {
        matches!(
            f.format.to_ascii_lowercase().as_str(),
            "mp3" | "m4b" | "m4a" | "ogg" | "flac" | "opus"
        )
    }) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let path = service.get_file_for_format(fmt);
    let Ok(data) = std::fs::read(&path) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let total_len = data.len();
    let mime = super::xml_feed::get_mime_type(&fmt.format);

    if let Some(range_val) = req_headers.get(RANGE).and_then(|v| v.to_str().ok()) {
        if let Some(spec) = range_val.strip_prefix("bytes=") {
            let mut parts = spec.splitn(2, '-');
            let start = parts
                .next()
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0);
            let end = parts
                .next()
                .and_then(|s| {
                    if s.is_empty() {
                        None
                    } else {
                        s.parse::<usize>().ok()
                    }
                })
                .unwrap_or_else(|| total_len.saturating_sub(1))
                .min(total_len.saturating_sub(1));

            if start <= end && start < total_len {
                let slice = data[start..=end].to_vec();
                let mut headers = HeaderMap::new();
                if let Ok(ct) = HeaderValue::from_str(mime) {
                    headers.insert(CONTENT_TYPE, ct);
                }
                headers.insert(ACCEPT_RANGES, HeaderValue::from_static("bytes"));
                if let Ok(cr) = HeaderValue::from_str(&format!("bytes {start}-{end}/{total_len}")) {
                    headers.insert(CONTENT_RANGE, cr);
                }
                if let Ok(cl) = HeaderValue::from_str(&slice.len().to_string()) {
                    headers.insert(CONTENT_LENGTH, cl);
                }
                return (StatusCode::PARTIAL_CONTENT, headers, slice).into_response();
            }
        }
    }

    let mut headers = HeaderMap::new();
    if let Ok(ct) = HeaderValue::from_str(mime) {
        headers.insert(CONTENT_TYPE, ct);
    }
    headers.insert(ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    if let Ok(cl) = HeaderValue::from_str(&total_len.to_string()) {
        headers.insert(CONTENT_LENGTH, cl);
    }
    (StatusCode::OK, headers, data).into_response()
}

pub(super) async fn api_sync_now_handler(
    Extension(sync_opt): Extension<Option<Arc<SyncService>>>,
) -> Response {
    let Some(sync) = sync_opt else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "SyncService not attached to server" })),
        )
            .into_response();
    };

    let res = tokio::task::spawn_blocking(move || sync.sync_now()).await;
    match res {
        Ok(Ok(report)) => (StatusCode::OK, Json(report)).into_response(),
        Ok(Err(e)) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}
