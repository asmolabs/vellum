use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use vellum_domain::{
    BookConverterPlugin, PluginDescriptor, PluginType, TestResult, VellumPlugin, VellumResult,
};

use crate::ereader::convert_epub_to_kepub;

pub struct EpubToKepubConverterPlugin {
    descriptor: PluginDescriptor,
}

impl Default for EpubToKepubConverterPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl EpubToKepubConverterPlugin {
    pub fn new() -> Self {
        Self {
            descriptor: PluginDescriptor {
                id: "converter-epub-to-kepub".to_string(),
                name: "Convertisseur Kobo EPUB (EPUB ➔ KEPUB)".to_string(),
                description: "Transforme un EPUB standard en KEPUB Kobo (injection des balises koboSpan pour pagination rapide et statistiques).".to_string(),
                version: "1.0.0".to_string(),
                plugin_type: PluginType::Conversion,
                external: false,
                command: None,
                supported_extensions: Vec::new(),
                source_format: Some("epub".to_string()),
                target_format: Some("kepub".to_string()),
                config_schema: Vec::new(),
            },
        }
    }
}

impl VellumPlugin for EpubToKepubConverterPlugin {
    fn descriptor(&self) -> &PluginDescriptor {
        &self.descriptor
    }

    fn test_configuration(&self, _config: &HashMap<String, Value>) -> TestResult {
        TestResult::ok("Convertisseur KEPUB prêt (injection koboSpan intégrée).")
    }

    fn as_converter_plugin(&self) -> Option<&dyn BookConverterPlugin> {
        Some(self)
    }
}

impl BookConverterPlugin for EpubToKepubConverterPlugin {
    fn source_format(&self) -> &str {
        "epub"
    }

    fn target_format(&self) -> &str {
        "kepub"
    }

    fn can_convert(&self, from_ext: &str, to_ext: &str) -> bool {
        from_ext.eq_ignore_ascii_case("epub")
            && (to_ext.eq_ignore_ascii_case("kepub") || to_ext.eq_ignore_ascii_case("kepub.epub"))
    }

    fn convert(
        &self,
        input_file: &Path,
        output_file: &Path,
        _config: &HashMap<String, Value>,
    ) -> VellumResult<PathBuf> {
        convert_epub_to_kepub(input_file, output_file)?;
        Ok(output_file.to_path_buf())
    }
}
