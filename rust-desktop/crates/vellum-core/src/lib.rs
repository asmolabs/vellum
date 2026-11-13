//! # `vellum-core`
//!
//! Couche Application / Cas d'usage de Vellum.
//!
//! Orchestre les entités de `vellum-domain`, la persistance de `vellum-storage` et les adaptateurs
//! de `vellum-plugins` pour fournir les services métier :
//! - [`BookService`] : import, organisation par templates, recherche Calibre-like, fusion et suppression
//! - [`ReaderContent`] / [`BookContentProvider`] : lecture EPUB/CBZ/PDF/TXT avec sanitisation HTML `ammonia`
//! - [`OpdsServer`] : serveur HTTP Axum pour catalogues OPDS 1.2 et Web Companion mobile
//! - [`EpubEditor`] : inspection, réparation et édition de table des matières EPUB
//! - [`NewsService`] : téléchargement RSS/Atom, sanitisation `ammonia` et génération d'EPUB de presse
//! - [`SyncService`], [`DeviceService`], [`MailService`], [`ConversionService`], [`LitestreamService`]

pub mod app_services;
pub mod book_service;
pub mod calibre_importer;
pub mod catalog_exporter;
pub mod conversion_service;
pub mod custom_column_service;
pub mod device_service;
pub mod epub_editor;
pub mod i18n;
pub mod library_audit;
pub mod litestream_service;
pub mod mail_service;
pub mod metadata_service;
pub mod naming_template;
pub mod news_service;
pub mod opds_server;
pub mod plugin_service;
pub mod reader_provider;
pub mod reader_store;
pub mod search_query;
pub mod sync_service;
pub mod virtual_library_service;
pub mod watch_folder;
pub mod web_companion_assets;

pub use app_services::{define_passage_with_ai, init_services, scan_and_index_library, AppServices};
pub use book_service::{
    BookService, DuplicateBookInfo, TranslateBookRequest, UpdateMetadataRequest,
};
pub use calibre_importer::{CalibreImportReport, CalibreImporter};
pub use catalog_exporter::{CatalogExporter, CatalogFormat};
pub use conversion_service::ConversionService;
pub use custom_column_service::CustomColumnService;
pub use device_service::{ConnectedDevice, DeviceService};
pub use epub_editor::{EpubEditor, PolishOptions, PolishReport};
pub use i18n::{catalog_for, translate, translate_fmt, Locale};
pub use library_audit::{audit_library, compute_similarity, normalize_string};
pub use litestream_service::LitestreamService;
pub use mail_service::MailService;
pub use metadata_service::MetadataService;
pub use naming_template::{NamingTemplate, DEFAULT_NAMING_TEMPLATE};
pub use news_service::NewsService;
pub use opds_server::OpdsServer;
pub use plugin_service::PluginService;
pub use reader_provider::{BookContentProvider, ReaderContent, ReaderSection};
pub use reader_store::*;
pub use search_query::SearchQuery;
pub use sync_service::{CloudSyncBackend, FolderSyncBackend, SyncService};
pub use virtual_library_service::VirtualLibraryService;
pub use watch_folder::{WatchFolderReport, WatchFolderService};

