//! Initialisation et assemblage des services applicatifs de Vellum.

use std::path::PathBuf;
use std::sync::Arc;

use tracing::info;
use vellum_domain::{AppConfig, Book, CloudSyncConfig, SmtpConfig};
use vellum_storage::{
    BookRepository, CustomColumnRepository, Database, ParamRepository, SyncRepository,
    TagRepository, VirtualLibraryRepository,
};

use crate::{
    BookService, ConversionService, CustomColumnService, DeviceService, LitestreamService,
    MailService, MetadataService, NewsService, OpdsServer, PluginService, SyncService,
    VirtualLibraryService,
};

/// Conteneur d'injection de dépendances regroupant les services métier de l'application.
#[derive(Clone)]
pub struct AppServices {
    pub book_service: Arc<BookService>,
    pub sync_service: Arc<SyncService>,
    pub device_service: Arc<DeviceService>,
    pub litestream_service: Arc<LitestreamService>,
    pub mail_service: Arc<MailService>,
    pub news_service: Arc<NewsService>,
    pub virtual_library_service: Arc<VirtualLibraryService>,
    pub custom_column_service: Arc<CustomColumnService>,
    pub smtp_config: SmtpConfig,
    pub opds_port: u16,
}

/// Initialise la base SQLite WAL, les services métier et les tâches d'arrière-plan (sync, OPDS).
pub fn init_services() -> AppServices {
    let app_cfg = AppConfig::default();
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let vellum_dir = home.join(".vellum");
    let _ = std::fs::create_dir_all(&vellum_dir);
    let _ = std::fs::create_dir_all(&app_cfg.library_path);
    let _ = std::fs::create_dir_all(&app_cfg.plugins_path);

    let db = Database::open(&app_cfg.database_path).expect("open vellum sqlite database");
    let book_repo = BookRepository::new(db.clone());
    let param_repo = ParamRepository::new(db.clone());
    let tag_repo = TagRepository::new(db.clone());
    let sync_repo = SyncRepository::new(db.clone());
    let vl_repo = VirtualLibraryRepository::new(db.clone());
    let custom_col_repo = CustomColumnRepository::new(db.clone());

    let mut cloud_sync_cfg = app_cfg.cloud_sync.clone();
    if let Ok(Some(saved_json)) = param_repo.get_value("cloud_sync_config") {
        if let Ok(loaded) = serde_json::from_str::<CloudSyncConfig>(&saved_json) {
            cloud_sync_cfg = loaded;
        }
    }

    let sync_service = Arc::new(SyncService::new(
        cloud_sync_cfg,
        sync_repo.clone(),
        book_repo.clone(),
        tag_repo.clone(),
        app_cfg.library_path.clone(),
    ));

    let plugin_service = PluginService::new(param_repo.clone(), Some(app_cfg.plugins_path.clone()));
    let metadata_service = MetadataService::new(plugin_service.clone());
    let conversion_service = ConversionService::new(plugin_service.clone());

    let book_service = Arc::new(
        BookService::new(
            book_repo,
            param_repo.clone(),
            tag_repo,
            metadata_service,
            plugin_service,
            conversion_service,
            app_cfg.library_path.clone(),
        )
        .with_sync_repo(sync_repo, sync_service.client_id()),
    );

    scan_and_index_library(&book_service);

    let litestream_service = Arc::new(LitestreamService::new(
        app_cfg.litestream.clone(),
        app_cfg.database_path.clone(),
    ));

    let periodic_sync = sync_service.clone();
    let periodic_watch_bs = book_service.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            if periodic_sync.is_backend_available() && periodic_sync.config().enabled {
                let s_clone = periodic_sync.clone();
                let res = tokio::task::spawn_blocking(move || s_clone.sync_now()).await;
                if let Ok(Ok(report)) = res {
                    if report.pulled_events > 0 || report.downloaded_books > 0 {
                        info!(
                            "Synchro périodique Google Drive : {} événements appliqués, {} livres reçus",
                            report.pulled_events, report.downloaded_books
                        );
                    }
                }
            }
            if let Ok(Some(watch_dir)) = periodic_watch_bs.param_repo().get_value("watch_folder_path") {
                let trimmed = watch_dir.trim().to_string();
                if !trimmed.is_empty() && std::path::Path::new(&trimmed).exists() {
                    let bs_clone = periodic_watch_bs.clone();
                    let res = tokio::task::spawn_blocking(move || {
                        crate::WatchFolderService::scan_once(
                            &bs_clone,
                            std::path::Path::new(&trimmed),
                            true,
                        )
                    })
                    .await;
                    if let Ok(Ok(report)) = res {
                        if !report.imported_book_ids.is_empty() {
                            info!(
                                "Watch Folder périodique : {} nouveau(x) livre(s) importé(s)",
                                report.imported_book_ids.len()
                            );
                        }
                    }
                }
            }
        }
    });

    let opds_bs = book_service.clone();
    let opds_port = app_cfg.opds_port;
    tokio::spawn(async move {
        info!("Starting OPDS server on port {}", opds_port);
        let _ = OpdsServer::run_server((*opds_bs).clone(), opds_port).await;
    });

    let device_service = Arc::new(DeviceService::new(
        book_service.clone(),
        book_service.plugin_service().clone(),
    ));

    let mut smtp_cfg = app_cfg.smtp.clone();
    if let Ok(Some(saved_json)) = param_repo.get_value("smtp_config") {
        if let Ok(loaded) = serde_json::from_str::<SmtpConfig>(&saved_json) {
            smtp_cfg = loaded;
        }
    }
    let mail_service = Arc::new(MailService::new(app_cfg.library_path.clone()));
    let news_service = Arc::new(NewsService::new(
        (*book_service).clone(),
        param_repo,
        Some(mail_service.clone()),
    ));
    let virtual_library_service = Arc::new(VirtualLibraryService::new(vl_repo));
    let custom_column_service = Arc::new(CustomColumnService::new(custom_col_repo));

    AppServices {
        book_service,
        sync_service,
        device_service,
        litestream_service,
        mail_service,
        news_service,
        virtual_library_service,
        custom_column_service,
        smtp_config: smtp_cfg,
        opds_port,
    }
}

/// Indexe automatiquement les livres présents sur disque si la base SQLite est vide.
pub fn scan_and_index_library(book_service: &BookService) {
    if let Ok(count) = book_service.book_repo().count() {
        if count == 0 {
            let library_path = book_service.library_path();
            if let Ok(authors) = std::fs::read_dir(library_path) {
                for author_res in authors.flatten() {
                    let author_path = author_res.path();
                    if author_path.is_dir() {
                        if let Ok(books_dir) = std::fs::read_dir(&author_path) {
                            for book_res in books_dir.flatten() {
                                let book_path = book_res.path();
                                if book_path.is_dir() {
                                    if let Ok(files) = std::fs::read_dir(&book_path) {
                                        for file_res in files.flatten() {
                                            let file_p = file_res.path();
                                            let ext = file_p
                                                .extension()
                                                .and_then(|e| e.to_str())
                                                .unwrap_or("")
                                                .to_lowercase();
                                            if matches!(
                                                ext.as_str(),
                                                "epub" | "pdf" | "cbz" | "cbr" | "mobi" | "azw3"
                                            ) {
                                                let meta = book_service
                                                    .metadata_service()
                                                    .extract_metadata(&file_p);
                                                let title = meta.title.unwrap_or_else(|| {
                                                    file_p
                                                        .file_stem()
                                                        .and_then(|s| s.to_str())
                                                        .unwrap_or("Sans titre")
                                                        .to_string()
                                                });
                                                let author = meta.author.unwrap_or_else(|| {
                                                    author_path
                                                        .file_name()
                                                        .and_then(|s| s.to_str())
                                                        .unwrap_or("Auteur inconnu")
                                                        .to_string()
                                                });

                                                let file_rel = file_p
                                                    .strip_prefix(library_path)
                                                    .unwrap_or(&file_p)
                                                    .to_string_lossy()
                                                    .to_string();

                                                let cover_path = if book_path.join("cover.jpg").exists() {
                                                    Some(
                                                        book_path
                                                            .join("cover.jpg")
                                                            .strip_prefix(library_path)
                                                            .unwrap_or(&book_path.join("cover.jpg"))
                                                            .to_string_lossy()
                                                            .to_string(),
                                                    )
                                                } else if book_path.join("cover.jpeg").exists() {
                                                    Some(
                                                        book_path
                                                            .join("cover.jpeg")
                                                            .strip_prefix(library_path)
                                                            .unwrap_or(&book_path.join("cover.jpeg"))
                                                            .to_string_lossy()
                                                            .to_string(),
                                                    )
                                                } else {
                                                    None
                                                };

                                                let file_size = std::fs::metadata(&file_p)
                                                    .map(|m| m.len())
                                                    .unwrap_or(0);
                                                let mut book = Book::new(title, author);
                                                book.description = meta.description;
                                                book.publisher = meta.publisher;
                                                book.publish_date = meta.publish_date;
                                                book.isbn = meta.isbn;
                                                book.series = meta.series;
                                                book.series_index = meta.series_index;
                                                book.cover_path = cover_path;
                                                for t in meta.tags {
                                                    let tag = vellum_domain::Tag::new(t);
                                                    let _ = book_service.tag_repo().save(&tag);
                                                    book.add_tag(tag);
                                                }
                                                let fmt = vellum_domain::BookFormat::new(
                                                    ext,
                                                    file_p
                                                        .file_name()
                                                        .and_then(|n| n.to_str())
                                                        .unwrap_or("book")
                                                        .to_string(),
                                                    file_rel,
                                                    file_size,
                                                );
                                                book.add_format(fmt);
                                                let _ = book_service.book_repo().save(&book);
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Fournit une définition ou traduction littéraire d'un passage via le client LLM local (Ollama).
pub fn define_passage_with_ai(prompt_text: &str) -> Result<String, String> {
    let client = vellum_plugins::translator::llm_client::LlmClient::new(
        "Ollama (Local - Gratuit)",
        "http://localhost:11434/v1",
        "qwen2.5:14b",
        None,
        0.3,
    );
    client
        .chat(
            "Tu es un dictionnaire et assistant littéraire concis en français. Fournis la définition claire, le sens littéraire ou la traduction élégante du terme ou passage sélectionné, avec un exemple si opportun. Reste concis.",
            prompt_text,
        )
        .map_err(|e| e.to_string())
}

