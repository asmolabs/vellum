//! Endpoints compatibles avec le plugin Progress Sync de KOReader.

use std::sync::Arc;

use axum::extract::{Path as AxPath, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::book_service::BookService;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KoreaderProgressPayload {
    pub document: String,
    pub progress: String,
    pub percentage: f64,
    #[serde(default)]
    pub device: Option<String>,
    #[serde(default)]
    pub device_id: Option<String>,
    #[serde(default)]
    pub timestamp: Option<i64>,
}

fn sanitize_doc_key(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect()
}

pub(super) async fn users_auth_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(serde_json::json!({ "authorized": "OK" })),
    )
}

pub(super) async fn users_create_handler() -> impl IntoResponse {
    (
        StatusCode::CREATED,
        Json(serde_json::json!({ "username": "vellum" })),
    )
}

pub(super) async fn put_progress_handler(
    State(service): State<Arc<BookService>>,
    Json(mut payload): Json<KoreaderProgressPayload>,
) -> Response {
    let doc_key = sanitize_doc_key(&payload.document);
    if doc_key.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Invalid document identifier" })),
        )
            .into_response();
    }

    let ts = payload
        .timestamp
        .unwrap_or_else(|| chrono::Utc::now().timestamp());
    payload.timestamp = Some(ts);

    let param_key = format!("koreader_progress:{doc_key}");
    if let Ok(json_str) = serde_json::to_string(&payload) {
        let _ = service.param_repo().set_value(&param_key, &json_str);
    }

    // Si le document correspond à un book_id Vellum ou à un hash/titre connu, on répercute la progression
    if let Ok(Some(_book)) = service.get_book_by_id(&doc_key) {
        let pct = (payload.percentage * 100.0).clamp(0.0, 100.0);
        let finished = pct >= 99.0;
        let _ = service.update_reading_progress(&doc_key, 0, 0, pct, finished);
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "document": doc_key,
            "timestamp": ts
        })),
    )
        .into_response()
}

pub(super) async fn get_progress_handler(
    State(service): State<Arc<BookService>>,
    AxPath(document): AxPath<String>,
) -> Response {
    let doc_key = sanitize_doc_key(&document);
    if doc_key.is_empty() {
        return StatusCode::BAD_REQUEST.into_response();
    }

    let param_key = format!("koreader_progress:{doc_key}");
    if let Ok(Some(raw)) = service.param_repo().get_value(&param_key) {
        if let Ok(payload) = serde_json::from_str::<KoreaderProgressPayload>(&raw) {
            return (StatusCode::OK, Json(payload)).into_response();
        }
    }

    // Repli : si le document est un book_id Vellum existant
    if let Ok(Some(book)) = service.get_book_by_id(&doc_key) {
        let fallback = KoreaderProgressPayload {
            document: doc_key,
            progress: format!("/body/chapter/{}", book.current_chapter_index),
            percentage: (book.progress_percent / 100.0).clamp(0.0, 1.0),
            device: Some("vellum".to_string()),
            device_id: Some("vellum".to_string()),
            timestamp: Some(chrono::Utc::now().timestamp()),
        };
        return (StatusCode::OK, Json(fallback)).into_response();
    }

    StatusCode::NOT_FOUND.into_response()
}
