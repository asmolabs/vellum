use crate::entity::Book;
use crate::error::VellumResult;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum PluginType {
    Import,
    #[default]
    Transformation,
    Format,
    Conversion,
    Device,
}

impl PluginType {
    pub fn as_str(&self) -> &'static str {
        match self {
            PluginType::Import => "import",
            PluginType::Transformation => "transformation",
            PluginType::Format => "format",
            PluginType::Conversion => "conversion",
            PluginType::Device => "device",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigField {
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(default)]
    pub default_value: Value,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub options: Vec<String>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub step: Option<f64>,
}

impl ConfigField {
    pub fn new(
        key: impl Into<String>,
        label: impl Into<String>,
        field_type: impl Into<String>,
        default_value: Value,
        description: impl Into<String>,
    ) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            field_type: field_type.into(),
            default_value,
            description: description.into(),
            options: Vec::new(),
            min: None,
            max: None,
            step: None,
        }
    }

    pub fn with_options(mut self, options: Vec<&str>) -> Self {
        self.options = options.into_iter().map(String::from).collect();
        self
    }

    pub fn with_range(mut self, min: f64, max: f64, step: f64) -> Self {
        self.min = Some(min);
        self.max = Some(max);
        self.step = Some(step);
        self
    }
}

fn default_version() -> String {
    "1.0.0".to_string()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginDescriptor {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(rename = "type", default)]
    pub plugin_type: PluginType,
    #[serde(default)]
    pub external: bool,
    pub command: Option<String>,
    #[serde(default)]
    pub supported_extensions: Vec<String>,
    pub source_format: Option<String>,
    pub target_format: Option<String>,
    #[serde(default)]
    pub config_schema: Vec<ConfigField>,
}

impl PluginDescriptor {
    pub fn type_string(&self) -> &'static str {
        self.plugin_type.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestResult {
    pub success: bool,
    pub message: String,
}

impl TestResult {
    pub fn ok(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
        }
    }

    pub fn fail(message: impl Into<String>) -> Self {
        Self::error(message)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtractedMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub publisher: Option<String>,
    pub publish_date: Option<String>,
    pub isbn: Option<String>,
    pub series: Option<String>,
    pub series_index: Option<f64>,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub cover_data: Option<Vec<u8>>,
    pub cover_ext: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeviceKind {
    Kobo,
    Kindle,
    Generic,
}

impl DeviceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            DeviceKind::Kobo => "kobo",
            DeviceKind::Kindle => "kindle",
            DeviceKind::Generic => "generic",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceBookInfo {
    pub title: String,
    pub author: String,
    pub filename: String,
    pub relative_path: PathBuf,
    pub file_size: u64,
    pub format: String,
    pub progress_percent: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceReadingProgress {
    pub book_title: String,
    pub book_author: Option<String>,
    pub file_path: Option<String>,
    pub progress_percent: f64,
    pub last_read: Option<chrono::DateTime<chrono::Utc>>,
    pub is_finished: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceAnnotation {
    pub book_title: String,
    pub book_author: Option<String>,
    pub text: String,
    pub note: Option<String>,
    pub location: Option<String>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

use std::sync::Arc;

#[derive(Clone)]
pub enum LibraryPlugin {
    Format(Arc<dyn BookFormatPlugin>),
    Converter(Arc<dyn BookConverterPlugin>),
    Enricher(Arc<dyn MetadataEnricherPlugin>),
    Transformation(Arc<dyn BookTransformationPlugin>),
    Device(Arc<dyn DeviceSyncPlugin>),
}

impl LibraryPlugin {
    pub fn descriptor(&self) -> &PluginDescriptor {
        match self {
            LibraryPlugin::Format(p) => p.descriptor(),
            LibraryPlugin::Converter(p) => p.descriptor(),
            LibraryPlugin::Enricher(p) => p.descriptor(),
            LibraryPlugin::Transformation(p) => p.descriptor(),
            LibraryPlugin::Device(p) => p.descriptor(),
        }
    }

    pub fn to_info(&self, enabled: bool) -> LibraryPluginInfo {
        let d = self.descriptor();
        LibraryPluginInfo {
            id: d.id.clone(),
            name: d.name.clone(),
            description: d.description.clone(),
            version: d.version.clone(),
            plugin_type: d.plugin_type.as_str().to_string(),
            external: d.external,
            enabled,
            config_schema: d.config_schema.clone(),
        }
    }

    pub fn test_configuration(&self, config: &HashMap<String, Value>) -> TestResult {
        match self {
            LibraryPlugin::Format(p) => p.test_configuration(config),
            LibraryPlugin::Converter(p) => p.test_configuration(config),
            LibraryPlugin::Enricher(p) => p.test_configuration(config),
            LibraryPlugin::Transformation(p) => p.test_configuration(config),
            LibraryPlugin::Device(p) => p.test_configuration(config),
        }
    }

    pub fn test_plugin(&self, config: &HashMap<String, Value>) -> TestResult {
        self.test_configuration(config)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LibraryPluginInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub plugin_type: String,
    pub external: bool,
    pub enabled: bool,
    pub config_schema: Vec<ConfigField>,
}

pub trait VellumPlugin: Send + Sync {
    fn descriptor(&self) -> &PluginDescriptor;

    fn id(&self) -> &str {
        &self.descriptor().id
    }

    fn name(&self) -> &str {
        &self.descriptor().name
    }

    fn is_external(&self) -> bool {
        self.descriptor().external
    }

    fn test_configuration(&self, _config: &HashMap<String, Value>) -> TestResult {
        TestResult::ok("Configuration opérationnelle.")
    }

    fn as_format_plugin(&self) -> Option<&dyn BookFormatPlugin> {
        None
    }

    fn as_converter_plugin(&self) -> Option<&dyn BookConverterPlugin> {
        None
    }

    fn as_enricher_plugin(&self) -> Option<&dyn MetadataEnricherPlugin> {
        None
    }

    fn as_transformation_plugin(&self) -> Option<&dyn BookTransformationPlugin> {
        None
    }

    fn as_device_plugin(&self) -> Option<&dyn DeviceSyncPlugin> {
        None
    }
}

pub trait BookFormatPlugin: VellumPlugin {
    fn supported_extensions(&self) -> Vec<String>;

    fn can_handle(&self, file: &Path) -> bool {
        let Some(ext) = file.extension().and_then(|e| e.to_str()) else {
            return false;
        };
        let ext_lower = ext.to_lowercase();
        self.supported_extensions()
            .iter()
            .any(|supported| supported.eq_ignore_ascii_case(&ext_lower))
    }

    fn extract_metadata(
        &self,
        file: &Path,
        config: &HashMap<String, Value>,
    ) -> VellumResult<ExtractedMetadata>;
}

pub trait BookConverterPlugin: VellumPlugin {
    fn source_format(&self) -> &str;
    fn target_format(&self) -> &str;

    fn can_convert(&self, from_ext: &str, to_ext: &str) -> bool {
        self.source_format().eq_ignore_ascii_case(from_ext)
            && self.target_format().eq_ignore_ascii_case(to_ext)
    }

    fn convert(
        &self,
        input_file: &Path,
        output_file: &Path,
        config: &HashMap<String, Value>,
    ) -> VellumResult<PathBuf>;
}

pub trait MetadataEnricherPlugin: VellumPlugin {
    fn enrich_metadata(
        &self,
        book_file: &Path,
        current_metadata: HashMap<String, Value>,
        config: &HashMap<String, Value>,
    ) -> HashMap<String, Value>;
}

pub trait BookTransformationPlugin: VellumPlugin {
    fn transform(
        &self,
        input_file: &Path,
        config: &HashMap<String, Value>,
    ) -> VellumResult<PathBuf>;
}

pub trait DeviceSyncPlugin: VellumPlugin {
    fn device_kind(&self) -> DeviceKind;
    fn default_mount_paths(&self) -> Vec<PathBuf>;
    fn detect_device(&self, custom_mount: Option<&Path>) -> Option<PathBuf>;
    fn list_books(&self, mount_point: &Path) -> VellumResult<Vec<DeviceBookInfo>>;
    fn export_book(
        &self,
        book: &Book,
        source_file: &Path,
        mount_point: &Path,
        config: &HashMap<String, Value>,
    ) -> VellumResult<PathBuf>;
    fn get_reading_progress(&self, mount_point: &Path) -> VellumResult<Vec<DeviceReadingProgress>>;
    fn get_annotations(&self, mount_point: &Path) -> VellumResult<Vec<DeviceAnnotation>>;
}
