//! Exécution des plugins externes basés sur scripts CLI (`plugin.json`).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use serde_json::Value;
use vellum_domain::{
    BookConverterPlugin, BookFormatPlugin, BookTransformationPlugin, ExtractedMetadata,
    LibraryPlugin, MetadataEnricherPlugin, PluginDescriptor, PluginType, TestResult, VellumError,
    VellumPlugin, VellumResult,
};

use super::ExternalPluginManifest;

pub struct ExternalScriptPlugin {
    plugin_dir: PathBuf,
    descriptor: PluginDescriptor,
    command: Vec<String>,
}

fn stringify_config(config: &HashMap<String, Value>) -> HashMap<String, String> {
    config
        .iter()
        .map(|(k, v)| {
            let s = match v {
                Value::String(s) => s.clone(),
                Value::Bool(b) => b.to_string(),
                Value::Number(n) => n.to_string(),
                _ => v.to_string(),
            };
            (k.clone(), s)
        })
        .collect()
}

impl ExternalScriptPlugin {
    pub fn from_manifest(dir: PathBuf, manifest: ExternalPluginManifest) -> Option<LibraryPlugin> {
        if manifest.id.trim().is_empty() || manifest.command.is_empty() {
            return None;
        }

        let plugin_type = match manifest.plugin_type.trim().to_uppercase().as_str() {
            "FORMAT" => PluginType::Format,
            "CONVERTER" | "CONVERSION" => PluginType::Conversion,
            "ENRICHER" | "IMPORT" => PluginType::Import,
            "TRANSFORMATION" => PluginType::Transformation,
            _ => return None,
        };

        let descriptor = PluginDescriptor {
            id: manifest.id,
            name: manifest.name,
            version: manifest.version.unwrap_or_else(|| "1.0.0".to_string()),
            description: manifest.description.unwrap_or_default(),
            plugin_type,
            supported_extensions: manifest.supported_extensions,
            source_format: manifest.source_format,
            target_format: manifest.target_format,
            external: true,
            command: Some(manifest.command.join(" ")),
            config_schema: manifest.config_schema,
        };

        let instance = Arc::new(Self {
            plugin_dir: dir,
            descriptor,
            command: manifest.command,
        });

        Some(match plugin_type {
            PluginType::Format => LibraryPlugin::Format(instance),
            PluginType::Conversion => LibraryPlugin::Converter(instance),
            PluginType::Import => LibraryPlugin::Enricher(instance),
            PluginType::Transformation | PluginType::Device => {
                LibraryPlugin::Transformation(instance)
            }
        })
    }

    fn execute_command(
        &self,
        extra_args: &[String],
        config: &HashMap<String, String>,
    ) -> VellumResult<String> {
        let config_file = tempfile::Builder::new()
            .prefix("vellum_ext_cfg_")
            .suffix(".json")
            .tempfile()?;
        let cfg_json = serde_json::to_string_pretty(config)?;
        fs::write(config_file.path(), cfg_json)?;

        let mut cmd = Command::new(&self.command[0]);
        cmd.current_dir(&self.plugin_dir);
        for arg in &self.command[1..] {
            cmd.arg(arg);
        }
        for arg in extra_args {
            cmd.arg(arg);
        }
        cmd.arg("--config").arg(config_file.path());

        for (k, v) in config {
            let env_key = format!(
                "VELLUM_{}",
                k.chars()
                    .map(|c| if c.is_ascii_alphanumeric() {
                        c.to_ascii_uppercase()
                    } else {
                        '_'
                    })
                    .collect::<String>()
            );
            cmd.env(env_key, v);
        }

        let output = cmd.output()?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !output.status.success() {
            return Err(VellumError::Plugin(format!(
                "External plugin '{}' failed (exit {:?}): {} {}",
                self.descriptor.id,
                output.status.code(),
                stdout.trim(),
                stderr.trim()
            )));
        }

        Ok(if stdout.trim().is_empty() {
            stderr
        } else {
            stdout
        })
    }

    fn test_plugin(&self, config: &HashMap<String, String>) -> TestResult {
        let args = vec!["--test".to_string()];
        match self.execute_command(&args, config) {
            Ok(out) => {
                let first_line = out.lines().next().unwrap_or("OK").trim();
                TestResult::ok(format!("Plugin externe opérationnel : {first_line}"))
            }
            Err(_) => {
                let bin = &self.command[0];
                let local_bin = self.plugin_dir.join(bin);
                if local_bin.exists() {
                    return TestResult::ok(format!("Script trouvé : {}", local_bin.display()));
                }
                if let Ok(status) = Command::new("which").arg(bin).status() {
                    if status.success() {
                        return TestResult::ok(format!(
                            "Exécutable '{bin}' disponible dans le PATH"
                        ));
                    }
                }
                TestResult::fail(format!(
                    "Impossible d'exécuter la commande '{}'",
                    self.command.join(" ")
                ))
            }
        }
    }
}

impl VellumPlugin for ExternalScriptPlugin {
    fn descriptor(&self) -> &PluginDescriptor {
        &self.descriptor
    }

    fn test_configuration(&self, config: &HashMap<String, Value>) -> TestResult {
        self.test_plugin(&stringify_config(config))
    }

    fn as_format_plugin(&self) -> Option<&dyn BookFormatPlugin> {
        (self.descriptor.plugin_type == PluginType::Format).then_some(self)
    }

    fn as_converter_plugin(&self) -> Option<&dyn BookConverterPlugin> {
        (self.descriptor.plugin_type == PluginType::Conversion).then_some(self)
    }

    fn as_enricher_plugin(&self) -> Option<&dyn MetadataEnricherPlugin> {
        (self.descriptor.plugin_type == PluginType::Import).then_some(self)
    }

    fn as_transformation_plugin(&self) -> Option<&dyn BookTransformationPlugin> {
        (self.descriptor.plugin_type == PluginType::Transformation).then_some(self)
    }
}

impl BookFormatPlugin for ExternalScriptPlugin {
    fn supported_extensions(&self) -> Vec<String> {
        self.descriptor.supported_extensions.clone()
    }

    fn extract_metadata(
        &self,
        file_path: &Path,
        _config: &HashMap<String, Value>,
    ) -> VellumResult<ExtractedMetadata> {
        let meta_out = tempfile::Builder::new()
            .prefix("vellum_meta_out_")
            .suffix(".json")
            .tempfile()?;
        let args = vec![
            "--input".to_string(),
            file_path.to_string_lossy().to_string(),
            "--metadata-out".to_string(),
            meta_out.path().to_string_lossy().to_string(),
        ];
        self.execute_command(&args, &HashMap::new())?;
        let content = fs::read_to_string(meta_out.path())?;
        let parsed: Value = serde_json::from_str(&content)?;
        Ok(ExtractedMetadata {
            title: parsed
                .get("title")
                .and_then(|v| v.as_str())
                .map(String::from),
            author: parsed
                .get("author")
                .and_then(|v| v.as_str())
                .map(String::from),
            publisher: parsed
                .get("publisher")
                .and_then(|v| v.as_str())
                .map(String::from),
            publish_date: parsed
                .get("publishedDate")
                .or_else(|| parsed.get("publishDate"))
                .and_then(|v| v.as_str())
                .map(String::from),
            isbn: parsed
                .get("isbn")
                .and_then(|v| v.as_str())
                .map(String::from),
            series: parsed
                .get("series")
                .and_then(|v| v.as_str())
                .map(String::from),
            series_index: parsed
                .get("seriesIndex")
                .or_else(|| parsed.get("series_index"))
                .and_then(|v| v.as_f64()),
            description: parsed
                .get("description")
                .and_then(|v| v.as_str())
                .map(String::from),
            tags: Vec::new(),
            cover_data: None,
            cover_ext: None,
        })
    }
}

impl BookConverterPlugin for ExternalScriptPlugin {
    fn source_format(&self) -> &str {
        self.descriptor.source_format.as_deref().unwrap_or("EPUB")
    }

    fn target_format(&self) -> &str {
        self.descriptor.target_format.as_deref().unwrap_or("EPUB")
    }

    fn convert(
        &self,
        source_file: &Path,
        target_file: &Path,
        config: &HashMap<String, Value>,
    ) -> VellumResult<PathBuf> {
        let args = vec![
            "--input".to_string(),
            source_file.to_string_lossy().to_string(),
            "--output".to_string(),
            target_file.to_string_lossy().to_string(),
        ];
        self.execute_command(&args, &stringify_config(config))?;
        Ok(target_file.to_path_buf())
    }
}

impl MetadataEnricherPlugin for ExternalScriptPlugin {
    fn enrich_metadata(
        &self,
        book_file: &Path,
        current_metadata: HashMap<String, Value>,
        config: &HashMap<String, Value>,
    ) -> HashMap<String, Value> {
        let meta_in = match tempfile::Builder::new()
            .prefix("vellum_meta_in_")
            .suffix(".json")
            .tempfile()
        {
            Ok(f) => f,
            Err(_) => return current_metadata,
        };
        let meta_out = match tempfile::Builder::new()
            .prefix("vellum_meta_out_")
            .suffix(".json")
            .tempfile()
        {
            Ok(f) => f,
            Err(_) => return current_metadata,
        };

        if let Ok(in_json) = serde_json::to_string_pretty(&current_metadata) {
            let _ = fs::write(meta_in.path(), in_json);
        }

        let args = vec![
            "--input".to_string(),
            book_file.to_string_lossy().to_string(),
            "--metadata-in".to_string(),
            meta_in.path().to_string_lossy().to_string(),
            "--metadata-out".to_string(),
            meta_out.path().to_string_lossy().to_string(),
        ];
        if self
            .execute_command(&args, &stringify_config(config))
            .is_ok()
        {
            if let Ok(out_raw) = fs::read_to_string(meta_out.path()) {
                if let Ok(enriched) = serde_json::from_str(&out_raw) {
                    return enriched;
                }
            }
        }
        current_metadata
    }
}

impl BookTransformationPlugin for ExternalScriptPlugin {
    fn transform(
        &self,
        input_file: &Path,
        config: &HashMap<String, Value>,
    ) -> VellumResult<PathBuf> {
        let stem = input_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("book");
        let ext = input_file
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let suffix = "processed";
        let parent = input_file.parent().unwrap_or_else(|| Path::new("."));
        let output_file = parent.join(format!("{stem}_{suffix}.{ext}"));

        let args = vec![
            "--input".to_string(),
            input_file.to_string_lossy().to_string(),
            "--output".to_string(),
            output_file.to_string_lossy().to_string(),
        ];
        self.execute_command(&args, &stringify_config(config))?;
        Ok(output_file)
    }
}
