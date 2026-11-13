//! # `vellum-domain`
//!
//! Couche Domaine pure de Vellum (Clean Architecture).
//!
//! Ce crate ne contient aucune dépendance d'infrastructure (pas de SQLite, HTTP, UI ou ZIP).
//! Il définit :
//! - Les entités métier ([`Book`], [`BookFormat`], [`Tag`], [`Param`], [`ReadingStats`])
//! - Les types de diagnostic et d'édition EPUB ([`EpubDiagnostics`], [`EpubTocEntry`])
//! - Les modèles de santé de bibliothèque et de flux d'actualités ([`LibraryHealthReport`], [`NewsRecipe`])
//! - Les contrats de plugins ([`BookFormatPlugin`], [`BookConverterPlugin`], [`DeviceSyncPlugin`], [`MetadataEnricherPlugin`])
//! - Les configurations ([`AppConfig`], [`SmtpConfig`], [`LitestreamConfig`], [`CloudSyncConfig`]) et erreurs ([`VellumError`])

pub mod config;
pub mod entity;
pub mod epub;
pub mod error;
pub mod health;
pub mod news;
pub mod plugin;
pub mod sync;
pub mod template;

pub use config::{AppConfig, LitestreamConfig, SmtpConfig};
pub use entity::{
    Book, BookFormat, ContentSearchHit, CustomColumnDefinition, CustomColumnType, Param,
    ReadingStats, Tag, VirtualLibrary,
};
pub use epub::{EpubDiagnostics, EpubIssue, EpubIssueSeverity, EpubTocEntry};
pub use error::{VellumError, VellumResult};
pub use health::{DuplicatePair, IncompleteBookIssue, LibraryHealthReport, MissingFileIssue};
pub use news::{NewsArticle, NewsFeedConfig, NewsRecipe};
pub use plugin::{
    BookConverterPlugin, BookFormatPlugin, BookTransformationPlugin, ConfigField,
    DeviceAnnotation, DeviceBookInfo, DeviceKind, DeviceReadingProgress, DeviceSyncPlugin,
    ExtractedMetadata, LibraryPlugin, LibraryPluginInfo, MetadataEnricherPlugin, PluginDescriptor,
    PluginType, TestResult, VellumPlugin,
};
pub use sync::{
    CloudSyncConfig, CloudSyncMode, SyncAction, SyncDelta, SyncEvent, SyncReport,
};
pub use template::{NamingTemplate, DEFAULT_NAMING_TEMPLATE};

