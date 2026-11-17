use crate::sync::CloudSyncConfig;
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
pub struct SmtpConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from_address: String,
    pub kindle_email: String,
}

impl Default for SmtpConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            host: "smtp.gmail.com".to_string(),
            port: 587,
            username: String::new(),
            password: String::new(),
            from_address: String::new(),
            kindle_email: String::new(),
        }
    }
}

fn default_opds_enabled() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfig {
    pub library_path: PathBuf,
    pub plugins_path: PathBuf,
    pub database_path: PathBuf,
    pub opds_port: u16,
    #[serde(default = "default_opds_enabled")]
    pub opds_enabled: bool,
    pub litestream: LitestreamConfig,
    pub cloud_sync: CloudSyncConfig,
    pub smtp: SmtpConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let vellum_dir = home.join(".vellum");
        Self::with_dirs(vellum_dir.clone(), vellum_dir.join("library"))
    }
}

impl AppConfig {
    pub fn with_dirs(data_dir: PathBuf, library_path: PathBuf) -> Self {
        Self {
            library_path,
            plugins_path: data_dir.join("plugins"),
            database_path: data_dir.join("metadata.db"),
            opds_port: 8085,
            opds_enabled: true,
            litestream: LitestreamConfig::default(),
            cloud_sync: CloudSyncConfig::default(),
            smtp: SmtpConfig::default(),
        }
    }

    pub fn from_env() -> Self {
        let data_dir = std::env::var("VELLUM_DATA_DIR")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".vellum")
            });

        let library_path = std::env::var("VELLUM_LIBRARY_DIR")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| data_dir.join("library"));

        let mut cfg = Self::with_dirs(data_dir, library_path);

        if let Some(port) = std::env::var("VELLUM_OPDS_PORT")
            .ok()
            .and_then(|s| s.trim().parse::<u16>().ok())
        {
            cfg.opds_port = port;
        }

        if let Ok(val) = std::env::var("VELLUM_OPDS_ENABLED") {
            let lower = val.trim().to_ascii_lowercase();
            cfg.opds_enabled = !matches!(lower.as_str(), "0" | "false" | "no" | "off");
        }

        cfg
    }
}
