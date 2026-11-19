//! Chargement des plugins externes (scripts CLI `plugin.json` et modules WASM `plugin.wasm.json`).

mod sample;
mod script_plugin;
mod wasm_plugin;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use vellum_domain::{ConfigField, LibraryPlugin};

use crate::wasm::WasmPluginRunner;

pub use script_plugin::ExternalScriptPlugin;
pub use wasm_plugin::WasmExternalPlugin;

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
        sample::ensure_sample_plugin_exists(&self.plugins_dir);

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
            if manifest_path.exists() {
                if let Ok(raw) = fs::read_to_string(&manifest_path) {
                    if let Ok(manifest) = serde_json::from_str::<ExternalPluginManifest>(&raw) {
                        if let Some(plugin) =
                            ExternalScriptPlugin::from_manifest(dir.clone(), manifest)
                        {
                            loaded.push(plugin);
                        }
                    }
                }
            }

            let wasm_manifest_path = dir.join("plugin.wasm.json");
            if wasm_manifest_path.exists() {
                if let Ok(manifest) = WasmPluginRunner::load_manifest(&wasm_manifest_path) {
                    let wasm_path = dir.join(&manifest.wasm_file);
                    let plugin = Arc::new(WasmExternalPlugin::new(wasm_path, manifest));
                    loaded.push(LibraryPlugin::Enricher(plugin));
                }
            }
        }

        loaded
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::collections::HashMap;

    #[test]
    fn test_load_external_wasm_plugin_from_manifest() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let plugin_dir = tmp.path().join("my-wasm-enricher");
        fs::create_dir_all(&plugin_dir).unwrap();

        let wat = r#"
            (module
                (memory (export "memory") 1)
                (func (export "alloc") (param $len i32) (result i32)
                    i32.const 1024
                )
                (func (export "run") (param $ptr i32) (param $len i32) (result i64)
                    local.get $ptr
                    i64.extend_i32_u
                    i64.const 32
                    i64.shl
                    local.get $len
                    i64.extend_i32_u
                    i64.or
                )
            )
        "#;
        fs::write(plugin_dir.join("plugin.wat"), wat).unwrap();
        let manifest = r#"{
            "id": "wasm-echo-enricher",
            "name": "WASM Echo Enricher",
            "version": "1.0.0",
            "description": "Test WASM enricher",
            "wasm_file": "plugin.wat",
            "entrypoint": "run"
        }"#;
        fs::write(plugin_dir.join("plugin.wasm.json"), manifest).unwrap();

        let loader = ExternalPluginLoader::new(tmp.path().to_path_buf());
        let plugins = loader.load_external_plugins();
        let wasm_plugin = plugins
            .iter()
            .find(|p| p.descriptor().id == "wasm-echo-enricher")
            .expect("WASM plugin should be discovered");

        let test_res = match wasm_plugin {
            LibraryPlugin::Enricher(p) => p.test_configuration(&HashMap::new()),
            _ => panic!("Expected Enricher variant"),
        };
        assert!(test_res.success, "{}", test_res.message);

        let mut meta = HashMap::new();
        meta.insert("title".to_string(), Value::String("Dune".to_string()));
        meta.insert(
            "author".to_string(),
            Value::String("Frank Herbert".to_string()),
        );
        if let LibraryPlugin::Enricher(p) = wasm_plugin {
            let enriched = p.enrich_metadata(Path::new("dune.epub"), meta, &HashMap::new());
            assert_eq!(enriched.get("title").and_then(|v| v.as_str()), Some("Dune"));
        }
    }
}
