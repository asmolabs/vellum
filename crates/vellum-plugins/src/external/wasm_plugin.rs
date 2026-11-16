//! Wrapper d'exécution sandboxée WebAssembly pour les plugins `plugin.wasm.json`.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use vellum_domain::{
    MetadataEnricherPlugin, PluginDescriptor, PluginType, TestResult, VellumPlugin,
};

use crate::wasm::{WasmPluginManifest, WasmPluginRunner};

/// Plugin externe sandboxé WebAssembly chargé depuis un manifeste `plugin.wasm.json`.
pub struct WasmExternalPlugin {
    wasm_path: PathBuf,
    manifest: WasmPluginManifest,
    descriptor: PluginDescriptor,
}

impl WasmExternalPlugin {
    pub fn new(wasm_path: PathBuf, manifest: WasmPluginManifest) -> Self {
        let descriptor = PluginDescriptor {
            id: manifest.id.clone(),
            name: manifest.name.clone(),
            version: manifest.version.clone(),
            description: manifest.description.clone(),
            plugin_type: PluginType::Import,
            supported_extensions: Vec::new(),
            source_format: None,
            target_format: None,
            external: true,
            command: Some(format!("wasm:{}", manifest.wasm_file)),
            config_schema: Vec::new(),
        };
        Self {
            wasm_path,
            manifest,
            descriptor,
        }
    }
}

impl VellumPlugin for WasmExternalPlugin {
    fn descriptor(&self) -> &PluginDescriptor {
        &self.descriptor
    }

    fn test_configuration(&self, _config: &HashMap<String, Value>) -> TestResult {
        let Ok(bytes) = fs::read(&self.wasm_path) else {
            return TestResult {
                success: false,
                message: format!("Fichier WASM introuvable : {}", self.wasm_path.display()),
            };
        };
        let runner = WasmPluginRunner::new(self.manifest.limits.clone());
        match runner.validate_module(&bytes) {
            Ok(()) => TestResult {
                success: true,
                message: format!(
                    "Module WASM '{}' valide (sandbox wasmi prêt).",
                    self.manifest.wasm_file
                ),
            },
            Err(e) => TestResult {
                success: false,
                message: format!("Module WASM invalide : {e}"),
            },
        }
    }

    fn as_enricher_plugin(&self) -> Option<&dyn MetadataEnricherPlugin> {
        Some(self)
    }
}

impl MetadataEnricherPlugin for WasmExternalPlugin {
    fn enrich_metadata(
        &self,
        _book_file: &Path,
        current_metadata: HashMap<String, Value>,
        _config: &HashMap<String, Value>,
    ) -> HashMap<String, Value> {
        let Ok(bytes) = fs::read(&self.wasm_path) else {
            return current_metadata;
        };
        let Ok(input_json) = serde_json::to_string(&current_metadata) else {
            return current_metadata;
        };
        let runner = WasmPluginRunner::new(self.manifest.limits.clone());
        match runner.run_json_hook(&bytes, "alloc", &self.manifest.entrypoint, &input_json) {
            Ok(res) => serde_json::from_str(&res.value).unwrap_or(current_metadata),
            Err(_) => current_metadata,
        }
    }
}
