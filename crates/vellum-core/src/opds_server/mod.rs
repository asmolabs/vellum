//! Serveur OPDS 1.2, Web Companion et API REST embarquée avec authentification par jeton.

mod koreader_sync;
mod web_api;
mod xml_feed;

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use axum::extract::{DefaultBodyLimit, Request, State};
use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
use axum::http::{HeaderName, HeaderValue, Method, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Extension, Router};
use tower_http::cors::{Any, CorsLayer};

use crate::book_service::BookService;
use crate::sync_service::SyncService;
use vellum_domain::VellumResult;

pub use web_api::{
    BookDto, ChapterContentDto, ContentSearchQuery, ExportQuery, FormatDto, ProgressUpdateDto,
    TocItemDto,
};
pub use xml_feed::{escape_xml, get_mime_type, CatalogQuery};

#[derive(Clone)]
pub struct OpdsServer {
    book_service: BookService,
}

impl OpdsServer {
    pub fn new(book_service: BookService) -> Self {
        Self { book_service }
    }

    pub fn router(book_service: BookService) -> Router {
        Self::router_with_token(book_service, None)
    }

    pub fn router_with_token(book_service: BookService, auth_token: Option<String>) -> Router {
        Self::router_with_services(book_service, None, auth_token)
    }

    pub fn router_with_services(
        book_service: BookService,
        sync_service: Option<Arc<SyncService>>,
        auth_token: Option<String>,
    ) -> Router {
        let shared = Arc::new(book_service);
        let expected_token = Arc::new(auth_token.filter(|t| !t.trim().is_empty()));
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods([Method::GET, Method::POST, Method::PUT, Method::OPTIONS])
            .allow_headers([
                CONTENT_TYPE,
                AUTHORIZATION,
                HeaderName::from_static("x-vellum-user"),
            ]);

        let protected = Router::new()
            // 1. Web Companion & Mobile Reader UI
            .route("/", get(web_api::web_library_handler))
            .route("/read/:id", get(web_api::web_reader_handler))
            // 2. REST API for Web & Mobile Clients
            .route("/api/status", get(web_api::api_status_handler))
            .route("/api/users", get(web_api::api_users_handler))
            .route("/api/stats", get(web_api::api_stats_handler))
            .route("/api/sync/status", get(web_api::api_sync_status_handler))
            .route("/api/sync", post(web_api::api_sync_now_handler))
            .route("/api/i18n", get(web_api::api_i18n_handler))
            .route("/api/export", get(web_api::api_export_handler))
            .route(
                "/api/search-content",
                get(web_api::api_search_content_handler),
            )
            .route("/api/books", get(web_api::api_books_handler))
            .route(
                "/api/books/upload",
                post(web_api::api_book_upload_handler)
                    .layer(DefaultBodyLimit::max(256 * 1024 * 1024)),
            )
            .route("/api/books/:id", get(web_api::api_book_detail_handler))
            .route(
                "/api/books/:id/metadata",
                post(web_api::api_book_metadata_handler),
            )
            .route("/api/books/:id/notes", get(web_api::api_book_notes_handler))
            .route("/api/books/:id/cover", get(xml_feed::cover_handler))
            .route("/api/books/:id/toc", get(web_api::api_book_toc_handler))
            .route(
                "/api/books/:id/chapter/:idx",
                get(web_api::api_book_chapter_handler),
            )
            .route(
                "/api/books/:id/progress",
                post(web_api::api_book_progress_handler),
            )
            .route(
                "/api/books/:id/download/:format",
                get(xml_feed::download_handler),
            )
            .route("/api/books/:id/audio", get(web_api::api_book_audio_handler))
            // 3. OPDS 1.2 Catalog Feed (for KOReader, Moon+, etc.)
            .route("/opds", get(xml_feed::root_catalog_handler))
            .route("/opds/books", get(xml_feed::books_catalog_handler))
            .route("/opds/covers/:id", get(xml_feed::cover_handler))
            .route(
                "/opds/download/:id/:format",
                get(xml_feed::download_handler),
            )
            // 4. KOReader Progress Sync API
            .route("/users/auth", get(koreader_sync::users_auth_handler))
            .route("/users/create", post(koreader_sync::users_create_handler))
            .route("/syncs/progress", put(koreader_sync::put_progress_handler))
            .route(
                "/syncs/progress/:document",
                get(koreader_sync::get_progress_handler),
            )
            .layer(Extension(sync_service))
            .layer(middleware::from_fn_with_state(
                expected_token,
                auth_and_security_headers,
            ))
            .with_state(shared);

        Router::new()
            .route("/api/v1/health", get(health_handler))
            .merge(protected)
            .layer(cors)
    }

    pub async fn run_server(book_service: BookService, port: u16) -> VellumResult<()> {
        Self::run_server_on(book_service, "0.0.0.0", port).await
    }

    pub async fn run_server_on(
        book_service: BookService,
        host: &str,
        port: u16,
    ) -> VellumResult<()> {
        let token = std::env::var("VELLUM_OPDS_TOKEN")
            .ok()
            .filter(|t| !t.trim().is_empty());
        Self::run_server_with_token_on(book_service, host, port, token).await
    }

    pub async fn run_server_with_token(
        book_service: BookService,
        port: u16,
        auth_token: Option<String>,
    ) -> VellumResult<()> {
        Self::run_server_with_token_on(book_service, "0.0.0.0", port, auth_token).await
    }

    pub async fn run_server_with_token_on(
        book_service: BookService,
        host: &str,
        port: u16,
        auth_token: Option<String>,
    ) -> VellumResult<()> {
        Self::run_server_with_services_on(book_service, None, host, port, auth_token).await
    }

    pub async fn run_server_with_services_on(
        book_service: BookService,
        sync_service: Option<Arc<SyncService>>,
        host: &str,
        port: u16,
        auth_token: Option<String>,
    ) -> VellumResult<()> {
        let ip: IpAddr = host.parse().unwrap_or_else(|_| IpAddr::from([0, 0, 0, 0]));
        let app = Self::router_with_services(book_service, sync_service, auth_token);
        let addr = SocketAddr::new(ip, port);
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;
        Ok(())
    }

    pub async fn start(&self, port: u16) -> VellumResult<()> {
        Self::run_server(self.book_service.clone(), port).await
    }
}

async fn health_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        [
            (
                axum::http::header::X_CONTENT_TYPE_OPTIONS,
                HeaderValue::from_static("nosniff"),
            ),
            (
                axum::http::header::X_FRAME_OPTIONS,
                HeaderValue::from_static("SAMEORIGIN"),
            ),
        ],
        axum::Json(serde_json::json!({
            "status": "ok",
            "service": "vellum"
        })),
    )
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
                [(
                    axum::http::header::WWW_AUTHENTICATE,
                    "Bearer realm=\"Vellum\"",
                )],
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
