//! Stockage sécurisé des secrets locaux (trousseau système avec repli fichier `0600`).

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use tracing::{debug, warn};
use uuid::Uuid;
use vellum_domain::VellumResult;

const SERVICE_NAME: &str = "vellum";

/// Chemin par défaut du fichier de repli pour les environnements sans trousseau (Docker / headless).
pub fn default_secrets_path() -> PathBuf {
    let base = std::env::var("VELLUM_DATA_DIR")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".vellum")
        });
    base.join("secrets.json")
}

/// Gestionnaire de secrets utilisant le trousseau OS si disponible, sinon un fichier `0600`.
#[derive(Debug, Clone)]
pub struct SecretStore {
    fallback_path: PathBuf,
    use_os_keyring: bool,
}

impl Default for SecretStore {
    fn default() -> Self {
        Self::new(default_secrets_path())
    }
}

impl SecretStore {
    pub fn new(fallback_path: PathBuf) -> Self {
        Self {
            fallback_path,
            use_os_keyring: true,
        }
    }

    /// Constructeur pour tests ou mode headless forçant le stockage fichier `0600`.
    pub fn file_only(fallback_path: PathBuf) -> Self {
        Self {
            fallback_path,
            use_os_keyring: false,
        }
    }

    pub fn set_secret(&self, key: &str, value: &str) -> VellumResult<()> {
        if value.trim().is_empty() {
            return self.delete_secret(key);
        }

        if self.use_os_keyring && try_keyring_set(key, value) {
            debug!("Secret '{key}' enregistré dans le trousseau système.");
            let _ = self.delete_from_fallback(key);
            return Ok(());
        }

        self.set_in_fallback(key, value)
    }

    pub fn get_secret(&self, key: &str) -> Option<String> {
        if self.use_os_keyring {
            if let Some(val) = try_keyring_get(key) {
                return Some(val);
            }
        }
        self.get_from_fallback(key)
    }

    pub fn delete_secret(&self, key: &str) -> VellumResult<()> {
        if self.use_os_keyring {
            let _ = try_keyring_delete(key);
        }
        self.delete_from_fallback(key)
    }

    fn load_fallback_map(&self) -> HashMap<String, String> {
        let Ok(content) = fs::read_to_string(&self.fallback_path) else {
            return HashMap::new();
        };
        serde_json::from_str(&content).unwrap_or_default()
    }

    fn save_fallback_map(&self, map: &HashMap<String, String>) -> VellumResult<()> {
        save_map_atomic(&self.fallback_path, map)
    }

    fn set_in_fallback(&self, key: &str, value: &str) -> VellumResult<()> {
        let mut map = self.load_fallback_map();
        map.insert(key.to_string(), value.to_string());
        self.save_fallback_map(&map)
    }

    fn get_from_fallback(&self, key: &str) -> Option<String> {
        self.load_fallback_map().remove(key)
    }

    fn delete_from_fallback(&self, key: &str) -> VellumResult<()> {
        if !self.fallback_path.exists() {
            return Ok(());
        }
        let mut map = self.load_fallback_map();
        if map.remove(key).is_some() {
            self.save_fallback_map(&map)?;
        }
        Ok(())
    }
}

fn save_map_atomic(path: &Path, map: &HashMap<String, String>) -> VellumResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = serde_json::to_string_pretty(map)?;
    let temp_path = path.with_extension(format!("tmp_{}", Uuid::new_v4()));
    fs::write(&temp_path, content)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o600);
        if let Err(e) = fs::set_permissions(&temp_path, perms) {
            warn!("Impossible d'appliquer les permissions 0600 sur secrets.json: {e}");
        }
    }

    fs::rename(temp_path, path)?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn has_dbus_session() -> bool {
    std::env::var("DBUS_SESSION_BUS_ADDRESS")
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false)
}

#[cfg(target_os = "linux")]
fn try_keyring_set(key: &str, value: &str) -> bool {
    if !has_dbus_session() {
        return false;
    }
    let label = format!("Vellum: {key}");
    let Ok(mut child) = Command::new("secret-tool")
        .args([
            "store",
            "--label",
            &label,
            "service",
            SERVICE_NAME,
            "account",
            key,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        return false;
    };

    if let Some(mut stdin) = child.stdin.take() {
        if stdin.write_all(value.as_bytes()).is_err() {
            return false;
        }
    }
    child.wait().map(|s| s.success()).unwrap_or(false)
}

#[cfg(target_os = "linux")]
fn try_keyring_get(key: &str) -> Option<String> {
    if !has_dbus_session() {
        return None;
    }
    let output = Command::new("secret-tool")
        .args(["lookup", "service", SERVICE_NAME, "account", key])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let val = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if val.is_empty() {
        None
    } else {
        Some(val)
    }
}

#[cfg(target_os = "linux")]
fn try_keyring_delete(key: &str) -> bool {
    if !has_dbus_session() {
        return false;
    }
    Command::new("secret-tool")
        .args(["clear", "service", SERVICE_NAME, "account", key])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(target_os = "macos")]
fn try_keyring_set(key: &str, value: &str) -> bool {
    Command::new("security")
        .args([
            "add-generic-password",
            "-U",
            "-s",
            SERVICE_NAME,
            "-a",
            key,
            "-w",
            value,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(target_os = "macos")]
fn try_keyring_get(key: &str) -> Option<String> {
    let output = Command::new("security")
        .args(["find-generic-password", "-s", SERVICE_NAME, "-a", key, "-w"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let val = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if val.is_empty() {
        None
    } else {
        Some(val)
    }
}

#[cfg(target_os = "macos")]
fn try_keyring_delete(key: &str) -> bool {
    Command::new("security")
        .args(["delete-generic-password", "-s", SERVICE_NAME, "-a", key])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn try_keyring_set(_key: &str, _value: &str) -> bool {
    false
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn try_keyring_get(_key: &str) -> Option<String> {
    None
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn try_keyring_delete(_key: &str) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_store_file_fallback_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("secrets.json");
        let store = SecretStore::file_only(path.clone());

        assert_eq!(store.get_secret("smtp_password"), None);
        store.set_secret("smtp_password", "super-secret").unwrap();
        assert_eq!(
            store.get_secret("smtp_password").as_deref(),
            Some("super-secret")
        );

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }

        store.delete_secret("smtp_password").unwrap();
        assert_eq!(store.get_secret("smtp_password"), None);
    }
}
