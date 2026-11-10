pub mod config;
pub mod entity;
pub mod error;
pub mod plugin;
pub mod sync;
pub mod template;

pub use config::{AppConfig, LitestreamConfig, SmtpConfig};
pub use entity::{Book, BookFormat, Param, ReadingStats, Tag};
pub use error::{VellumError, VellumResult};
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

