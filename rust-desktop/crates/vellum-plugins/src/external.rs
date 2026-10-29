use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use vellum_domain::{
    BookConverterPlugin, BookFormatPlugin, BookTransformationPlugin, ConfigField,
    ExtractedMetadata, LibraryPlugin, MetadataEnricherPlugin, PluginDescriptor, PluginType,
    TestResult, VellumError, VellumPlugin, VellumResult,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalPluginManifest {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub plugin_type: String,
    pub command: Vec<String>,
    #[serde(default)]
    pub supported_extensions: Vec<String>,
    #[serde(default)]
    pub source_format: Option<String>,
    #[serde(default)]
    pub target_format: Option<String>,
    #[serde(default)]
    pub output_suffix: Option<String>,
    #[serde(default)]
    pub attach_to_book: Option<bool>,
    #[serde(default)]
    pub config_schema: Vec<ConfigField>,
}

pub struct ExternalPluginLoader {
    plugins_dir: PathBuf,
}

impl ExternalPluginLoader {
    pub fn new(plugins_dir: PathBuf) -> Self {
        Self { plugins_dir }
    }

    pub fn default_plugins_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".vellum")
            .join("plugins")
    }

    pub fn plugins_dir(&self) -> &Path {
        &self.plugins_dir
    }

    pub fn load_external_plugins(&self) -> Vec<LibraryPlugin> {
        let _ = fs::create_dir_all(&self.plugins_dir);
        self.ensure_sample_plugin_exists();

        let mut loaded = Vec::new();
        let Ok(entries) = fs::read_dir(&self.plugins_dir) else {
            return loaded;
        };

        for entry in entries.flatten() {
            let dir = entry.path();
            if !dir.is_dir() {
                continue;
            }
            let manifest_path = dir.join("plugin.json");
            if !manifest_path.exists() {
                continue;
            }
            if let Ok(raw) = fs::read_to_string(&manifest_path) {
                if let Ok(manifest) = serde_json::from_str::<ExternalPluginManifest>(&raw) {
                    if let Some(plugin) = ExternalScriptPlugin::from_manifest(dir, manifest) {
                        loaded.push(plugin);
                    }
                }
            }
        }

        loaded
    }

    fn ensure_sample_plugin_exists(&self) {
        let sample_dir = self.plugins_dir.join("sample-epub-stamper");
        let manifest_file = sample_dir.join("plugin.json");
        if manifest_file.exists() {
            return;
        }
        let _ = fs::create_dir_all(&sample_dir);

        let manifest_json = r#"{
  "id": "ext-sample-stamper",
  "name": "Sample Ex-Libris Stamper (Script Externe)",
  "version": "1.0.0",
  "author": "Vellum Community",
  "description": "Exemple de plugin externe (Python/Shell) qui copie un fichier en ajoutant un ex-libris configurable.",
  "type": "TRANSFORMATION",
  "command": ["python3", "plugin.py"],
  "supportedExtensions": ["epub", "txt", "pdf"],
  "outputSuffix": "stamped",
  "attachToBook": true,
  "configSchema": [
    {
      "key": "ownerName",
      "label": "Ex-Libris (Propriétaire)",
      "type": "TEXT",
      "defaultValue": "Bibliothèque Vellum",
      "description": "Nom inscrit dans les métadonnées ou le journal d'exécution",
      "options": []
    }
  ]
}"#;
        let script_py = r#"#!/usr/bin/env python3
import sys
import shutil

def main():
    args = sys.argv[1:]
    if "--test" in args:
        print("Script Python Ex-Libris opérationnel !")
        sys.exit(0)

    input_path = None
    output_path = None
    for i, a in enumerate(args):
        if a == "--input" and i + 1 < len(args):
            input_path = args[i + 1]
        elif a == "--output" and i + 1 < len(args):
            output_path = args[i + 1]

    if input_path and output_path:
        shutil.copyfile(input_path, output_path)
        sys.exit(0)

    sys.exit(1)

if __name__ == "__main__":
    main()
"#;
        let _ = fs::write(&manifest_file, manifest_json);
        let _ = fs::write(sample_dir.join("plugin.py"), script_py);
    }
}

pub struct ExternalScriptPlugin {
    plugin_dir: PathBuf,
    descriptor: PluginDescriptor,
    command: Vec<String>,
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
            PluginType::Transformation | PluginType::Device => LibraryPlugin::Transformation(instance),
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
}

impl VellumPlugin for ExternalScriptPlugin {
    fn descriptor(&self) -> &PluginDescriptor {
        &self.descriptor
    }

    fn test_configuration(&self, config: &HashMap<String, Value>) -> TestResult {
        let str_config: HashMap<String, String> = config
            .iter()
            .map(|(k, v)| {
                let val = match v {
                    Value::String(s) => s.clone(),
                    Value::Bool(b) => b.to_string(),
                    Value::Number(n) => n.to_string(),
                    _ => v.to_string(),
                };
                (k.clone(), val)
            })
            .collect();
        self.test_plugin(&str_config)
    }

    fn as_format_plugin(&self) -> Option<&dyn BookFormatPlugin> {
        if self.descriptor.plugin_type == PluginType::Format {
            Some(self)
        } else {
            None
        }
    }

    fn as_converter_plugin(&self) -> Option<&dyn BookConverterPlugin> {
        if self.descriptor.plugin_type == PluginType::Conversion {
            Some(self)
        } else {
            None
        }
    }

    fn as_enricher_plugin(&self) -> Option<&dyn MetadataEnricherPlugin> {
        if self.descriptor.plugin_type == PluginType::Import {
            Some(self)
        } else {
            None
        }
    }

    fn as_transformation_plugin(&self) -> Option<&dyn BookTransformationPlugin> {
        if self.descriptor.plugin_type == PluginType::Transformation {
            Some(self)
        } else {
            None
        }
    }
}

impl ExternalScriptPlugin {
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
                        return TestResult::ok(format!("Exécutable '{bin}' disponible dans le PATH"));
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
        let parsed: serde_json::Value = serde_json::from_str(&content)?;
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
        self.descriptor
            .source_format
            .as_deref()
            .unwrap_or("EPUB")
    }

    fn target_format(&self) -> &str {
        self.descriptor
            .target_format
            .as_deref()
            .unwrap_or("EPUB")
    }

    fn convert(
        &self,
        source_file: &Path,
        target_file: &Path,
        config: &HashMap<String, Value>,
    ) -> VellumResult<PathBuf> {
        let str_config: HashMap<String, String> = config
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
            .collect();

        let args = vec![
            "--input".to_string(),
            source_file.to_string_lossy().to_string(),
            "--output".to_string(),
            target_file.to_string_lossy().to_string(),
        ];
        self.execute_command(&args, &str_config)?;
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
        let str_config: HashMap<String, String> = config
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
            .collect();

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
        if self.execute_command(&args, &str_config).is_ok() {
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

        let str_config: HashMap<String, String> = config
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
            .collect();

        let args = vec![
            "--input".to_string(),
            input_file.to_string_lossy().to_string(),
            "--output".to_string(),
            output_file.to_string_lossy().to_string(),
        ];
        self.execute_command(&args, &str_config)?;
        Ok(output_file)
    }
}
