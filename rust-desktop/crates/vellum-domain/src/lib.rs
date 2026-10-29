pub mod config;
pub mod entity;
pub mod error;
pub mod plugin;

pub use config::{AppConfig, LitestreamConfig};
pub use entity::{Book, BookFormat, Param, ReadingStats, Tag};
pub use error::{VellumError, VellumResult};
pub use plugin::{
    BookConverterPlugin, BookFormatPlugin, BookTransformationPlugin, ConfigField,
    DeviceAnnotation, DeviceBookInfo, DeviceKind, DeviceReadingProgress, DeviceSyncPlugin,
    ExtractedMetadata, LibraryPlugin, LibraryPluginInfo, MetadataEnricherPlugin, PluginDescriptor,
    PluginType, TestResult, VellumPlugin,
};

