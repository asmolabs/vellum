use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use vellum_domain::{
    BookConverterPlugin, BookFormatPlugin, BookTransformationPlugin, ConfigField, DeviceSyncPlugin,
    LibraryPlugin, MetadataEnricherPlugin, PluginDescriptor, TestResult, VellumError, VellumResult,
};
use vellum_plugins::{
    built_in_converter_plugins, built_in_device_plugins, built_in_format_plugins,
    cleaner::OceanOfPdfCleanerPlugin,
    enricher::{ComicVineEnricherPlugin, GoogleBooksEnricherPlugin, HardcoverEnricherPlugin},
    translator::EpubTranslatorPlugin,
    ExternalPluginLoader,
};
use vellum_storage::ParamRepository;

#[derive(Clone)]
pub struct PluginService {
    param_repo: ParamRepository,
    external_loader: Arc<ExternalPluginLoader>,
    plugins: Arc<RwLock<Vec<LibraryPlugin>>>,
}

impl PluginService {
    pub fn new(param_repo: ParamRepository, external_dir: Option<PathBuf>) -> Self {
        let dir = external_dir.unwrap_or_else(ExternalPluginLoader::default_plugins_dir);
        let external_loader = Arc::new(ExternalPluginLoader::new(dir));
        let service = Self {
            param_repo,
            external_loader,
            plugins: Arc::new(RwLock::new(Vec::new())),
        };
        service.reload_all_plugins();
        service
    }

    pub fn reload_all_plugins(&self) {
        let mut all: Vec<LibraryPlugin> = Vec::new();

        for fp in built_in_format_plugins() {
            all.push(LibraryPlugin::Format(fp));
        }
        for cp in built_in_converter_plugins() {
            all.push(LibraryPlugin::Converter(cp));
        }
        for dp in built_in_device_plugins() {
            all.push(LibraryPlugin::Device(dp));
        }
        all.push(LibraryPlugin::Enricher(Arc::new(
            GoogleBooksEnricherPlugin::new(),
        )));
        all.push(LibraryPlugin::Enricher(Arc::new(
            HardcoverEnricherPlugin::new(),
        )));
        all.push(LibraryPlugin::Enricher(Arc::new(
            ComicVineEnricherPlugin::new(),
        )));
        all.push(LibraryPlugin::Transformation(Arc::new(
            OceanOfPdfCleanerPlugin::new(),
        )));
        all.push(LibraryPlugin::Transformation(Arc::new(
            EpubTranslatorPlugin::new(),
        )));

        for ext in self.external_loader.load_external_plugins() {
            all.push(ext);
        }

        if let Ok(mut guard) = self.plugins.write() {
            *guard = all;
        }
    }

    pub fn external_plugins_dir(&self) -> PathBuf {
        self.external_loader.plugins_dir().to_path_buf()
    }

    pub fn list_descriptors(&self) -> Vec<PluginDescriptor> {
        let Ok(guard) = self.plugins.read() else {
            return Vec::new();
        };
        guard.iter().map(|p| p.descriptor().clone()).collect()
    }

    pub fn get_format_plugins(&self) -> Vec<Arc<dyn BookFormatPlugin>> {
        let Ok(guard) = self.plugins.read() else {
            return Vec::new();
        };
        guard
            .iter()
            .filter_map(|p| match p {
                LibraryPlugin::Format(fp) => Some(Arc::clone(fp)),
                _ => None,
            })
            .collect()
    }

    pub fn get_converter_plugins(&self) -> Vec<Arc<dyn BookConverterPlugin>> {
        let Ok(guard) = self.plugins.read() else {
            return Vec::new();
        };
        guard
            .iter()
            .filter_map(|p| match p {
                LibraryPlugin::Converter(cp) => Some(Arc::clone(cp)),
                _ => None,
            })
            .collect()
    }

    pub fn get_enricher_plugins(&self) -> Vec<Arc<dyn MetadataEnricherPlugin>> {
        let Ok(guard) = self.plugins.read() else {
            return Vec::new();
        };
        guard
            .iter()
            .filter_map(|p| match p {
                LibraryPlugin::Enricher(ep) => Some(Arc::clone(ep)),
                _ => None,
            })
            .collect()
    }

    pub fn get_transformation_plugins(&self) -> Vec<Arc<dyn BookTransformationPlugin>> {
        let Ok(guard) = self.plugins.read() else {
            return Vec::new();
        };
        guard
            .iter()
            .filter_map(|p| match p {
                LibraryPlugin::Transformation(tp) => Some(Arc::clone(tp)),
                _ => None,
            })
            .collect()
    }

    pub fn get_device_plugins(&self) -> Vec<Arc<dyn DeviceSyncPlugin>> {
        let Ok(guard) = self.plugins.read() else {
            return Vec::new();
        };
        guard
            .iter()
            .filter_map(|p| match p {
                LibraryPlugin::Device(dp) => Some(Arc::clone(dp)),
                _ => None,
            })
            .collect()
    }

    pub fn find_device_plugin(&self, plugin_id: &str) -> Option<Arc<dyn DeviceSyncPlugin>> {
        self.get_device_plugins()
            .into_iter()
            .find(|p| p.descriptor().id == plugin_id)
    }

    pub fn find_format_plugin_for_extension(
        &self,
        extension: &str,
    ) -> Option<Arc<dyn BookFormatPlugin>> {
        let ext_lower = extension.trim_start_matches('.').to_lowercase();
        self.get_format_plugins().into_iter().find(|p| {
            p.supported_extensions()
                .iter()
                .any(|e| e.eq_ignore_ascii_case(&ext_lower))
        })
    }

    pub fn find_transformation_plugin(
        &self,
        plugin_id: &str,
    ) -> Option<Arc<dyn BookTransformationPlugin>> {
        self.get_transformation_plugins()
            .into_iter()
            .find(|p| p.descriptor().id == plugin_id)
    }

    pub fn find_enricher_plugin(&self, plugin_id: &str) -> Option<Arc<dyn MetadataEnricherPlugin>> {
        self.get_enricher_plugins()
            .into_iter()
            .find(|p| p.descriptor().id == plugin_id)
    }

    pub fn get_plugin_config(
        &self,
        plugin_id: &str,
    ) -> VellumResult<HashMap<String, serde_json::Value>> {
        let mut map = HashMap::new();
        let descriptors = self.list_descriptors();
        let schema: Vec<ConfigField> = descriptors
            .into_iter()
            .find(|d| d.id == plugin_id)
            .map(|d| d.config_schema)
            .unwrap_or_default();

        for field in schema {
            let db_key = format!("plugin.{plugin_id}.{}", field.key);
            if let Some(val) = self.param_repo.get_value(&db_key)? {
                let parsed: serde_json::Value =
                    serde_json::from_str(&val).unwrap_or(serde_json::Value::String(val));
                map.insert(field.key, parsed);
            } else {
                map.insert(field.key, field.default_value);
            }
        }
        Ok(map)
    }

    pub fn save_plugin_config(
        &self,
        plugin_id: &str,
        config: &HashMap<String, serde_json::Value>,
    ) -> VellumResult<()> {
        for (k, v) in config {
            let db_key = format!("plugin.{plugin_id}.{k}");
            let val_str = match v {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            self.param_repo.set_value(&db_key, &val_str)?;
        }
        Ok(())
    }

    pub fn test_plugin(
        &self,
        plugin_id: &str,
        override_config: Option<&HashMap<String, serde_json::Value>>,
    ) -> VellumResult<TestResult> {
        let mut config = self.get_plugin_config(plugin_id)?;
        if let Some(overrides) = override_config {
            for (k, v) in overrides {
                config.insert(k.clone(), v.clone());
            }
        }

        let guard = self
            .plugins
            .read()
            .map_err(|e| VellumError::Plugin(format!("Plugin lock poisoned: {e}")))?;
        let plugin = guard
            .iter()
            .find(|p| p.descriptor().id == plugin_id)
            .ok_or_else(|| VellumError::NotFound(format!("Plugin '{plugin_id}' not found")))?;

        Ok(plugin.test_plugin(&config))
    }

    pub fn is_plugin_enabled(&self, plugin_id: &str) -> bool {
        let key = format!("plugin.{plugin_id}.enabled");
        self.param_repo
            .get_value(&key)
            .ok()
            .flatten()
            .and_then(|v| v.parse::<bool>().ok())
            .unwrap_or(true)
    }

    pub fn run_import_enrichers(
        &self,
        book_file: Option<&std::path::Path>,
        current_metadata: HashMap<String, serde_json::Value>,
    ) -> HashMap<String, serde_json::Value> {
        let mut meta = current_metadata;
        for enricher in self.get_enricher_plugins() {
            let id = enricher.descriptor().id.clone();
            if !self.is_plugin_enabled(&id) {
                continue;
            }
            let config = self.get_plugin_config(&id).unwrap_or_default();
            let dummy_path = std::path::PathBuf::from("book");
            let file_ref = book_file.unwrap_or(&dummy_path);
            meta = enricher.enrich_metadata(file_ref, meta, &config);
        }
        meta
    }
}
