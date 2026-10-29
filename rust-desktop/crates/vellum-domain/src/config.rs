use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LitestreamConfig {
    pub enabled: bool,
    pub replica_type: String,
    pub replica_path: PathBuf,
    pub binary_path: String,
}

impl Default for LitestreamConfig {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        Self {
            enabled: false,
            replica_type: "file".to_string(),
            replica_path: home.join(".vellum").join("backups"),
            binary_path: "litestream".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfig {
    pub library_path: PathBuf,
    pub plugins_path: PathBuf,
    pub database_path: PathBuf,
    pub opds_port: u16,
    pub litestream: LitestreamConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let vellum_dir = home.join(".vellum");
        Self {
            library_path: vellum_dir.join("library"),
            plugins_path: vellum_dir.join("plugins"),
            database_path: vellum_dir.join("metadata.db"),
            opds_port: 8085,
            litestream: LitestreamConfig::default(),
        }
    }
}
