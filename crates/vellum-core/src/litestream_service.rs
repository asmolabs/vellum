use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::{Arc, Mutex, RwLock};

use serde::{Deserialize, Serialize};
use vellum_domain::{LitestreamConfig, VellumError, VellumResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LitestreamStatus {
    Running,
    Stopped,
    BinaryNotFound,
    Disabled,
    Error,
}

#[derive(Clone)]
pub struct LitestreamService {
    config: LitestreamConfig,
    db_path: PathBuf,
    process: Arc<Mutex<Option<Child>>>,
    last_status_message: Arc<RwLock<String>>,
}

impl LitestreamService {
    pub fn new(config: LitestreamConfig, db_path: PathBuf) -> Self {
        let msg = if !config.enabled {
            "Litestream désactivé dans la configuration".to_string()
        } else {
            "Litestream initialisé".to_string()
        };

        Self {
            config,
            db_path,
            process: Arc::new(Mutex::new(None)),
            last_status_message: Arc::new(RwLock::new(msg)),
        }
    }

    pub fn is_installed(&self) -> bool {
        Command::new(&self.config.binary_path)
            .arg("version")
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false)
    }

    pub fn status(&self) -> LitestreamStatus {
        if !self.config.enabled {
            return LitestreamStatus::Disabled;
        }
        if !self.is_installed() {
            return LitestreamStatus::BinaryNotFound;
        }
        if let Ok(mut guard) = self.process.lock() {
            if let Some(child) = guard.as_mut() {
                match child.try_wait() {
                    Ok(None) => return LitestreamStatus::Running,
                    Ok(Some(_)) => {
                        *guard = None;
                        return LitestreamStatus::Stopped;
                    }
                    Err(_) => return LitestreamStatus::Error,
                }
            }
        }
        LitestreamStatus::Stopped
    }

    pub fn last_status_message(&self) -> String {
        self.last_status_message
            .read()
            .map(|s| s.clone())
            .unwrap_or_else(|_| "Inconnu".to_string())
    }

    pub fn generate_config_file(&self) -> VellumResult<PathBuf> {
        let config_dir = self.db_path.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(config_dir)?;

        let config_file = config_dir.join("litestream.yml");
        fs::create_dir_all(&self.config.replica_path)?;

        let db_str = self.db_path.to_string_lossy().replace('\\', "/");
        let replica_str = self
            .config
            .replica_path
            .to_string_lossy()
            .replace('\\', "/");

        let yaml = format!(
            "# Litestream SQLite Continuous Replication Configuration\n\
             dbs:\n\
             \x20 - path: \"{db_str}\"\n\
             \x20   replicas:\n\
             \x20     - path: \"{replica_str}\"\n\
             \x20       sync-interval: 1s\n"
        );

        fs::write(&config_file, yaml)?;
        Ok(config_file)
    }

    pub fn start_replication(&self) -> VellumResult<bool> {
        if !self.config.enabled {
            let msg = "Litestream est désactivé dans la configuration.";
            if let Ok(mut w) = self.last_status_message.write() {
                *w = msg.to_string();
            }
            return Ok(false);
        }

        if !self.is_installed() {
            let msg = format!(
                "Binaire '{}' introuvable dans le PATH.",
                self.config.binary_path
            );
            if let Ok(mut w) = self.last_status_message.write() {
                *w = msg.clone();
            }
            return Err(VellumError::ExternalCommand(msg));
        }

        let mut guard = self
            .process
            .lock()
            .map_err(|e| VellumError::System(e.to_string()))?;

        if let Some(child) = guard.as_mut() {
            if child.try_wait()?.is_none() {
                return Ok(true);
            }
        }

        let config_file = self.generate_config_file()?;
        let child = Command::new(&self.config.binary_path)
            .arg("replicate")
            .arg("-config")
            .arg(&config_file)
            .spawn()
            .map_err(|e| {
                VellumError::ExternalCommand(format!("Échec du lancement de Litestream: {e}"))
            })?;

        *guard = Some(child);
        let msg = format!(
            "Réplication active vers {}",
            self.config.replica_path.display()
        );
        if let Ok(mut w) = self.last_status_message.write() {
            *w = msg;
        }

        Ok(true)
    }

    pub fn stop_replication(&self) -> VellumResult<()> {
        let mut guard = self
            .process
            .lock()
            .map_err(|e| VellumError::System(e.to_string()))?;

        if let Some(mut child) = guard.take() {
            let _ = child.kill();
            let _ = child.wait();
        }

        let msg = "Litestream s'est arrêté.".to_string();
        if let Ok(mut w) = self.last_status_message.write() {
            *w = msg;
        }

        Ok(())
    }
}
