//! Trait de stockage cloud et implémentation basée sur un dossier synchronisé (Google Drive).

use std::fs;
use std::path::{Path, PathBuf};

use vellum_domain::{SyncDelta, VellumError, VellumResult};

pub trait CloudSyncBackend: Send + Sync {
    fn name(&self) -> &str;
    fn is_available(&self) -> bool;
    fn list_deltas(&self) -> VellumResult<Vec<String>>;
    fn read_delta(&self, filename: &str) -> VellumResult<SyncDelta>;
    fn write_delta(&self, delta: &SyncDelta) -> VellumResult<String>;
    fn upload_file(&self, local_path: &Path, rel_path: &str) -> VellumResult<()>;
    fn download_file(&self, rel_path: &str, local_path: &Path) -> VellumResult<bool>;
    fn file_exists(&self, rel_path: &str) -> VellumResult<bool>;
}

/// Backend utilisant un dossier partagé synchronisé par Google Drive Desktop.
pub struct FolderSyncBackend {
    root_dir: PathBuf,
}

impl FolderSyncBackend {
    pub fn new(root_dir: PathBuf) -> Self {
        let backend = Self { root_dir };
        let _ = backend.ensure_directories();
        backend
    }

    fn ensure_directories(&self) -> VellumResult<()> {
        let deltas_dir = self.root_dir.join("sync").join("deltas");
        let files_dir = self.root_dir.join("files");
        fs::create_dir_all(&deltas_dir)?;
        fs::create_dir_all(&files_dir)?;
        Ok(())
    }

    pub fn auto_detect_google_drive() -> Option<PathBuf> {
        let home = dirs::home_dir()?;

        // macOS: ~/Library/CloudStorage/GoogleDrive-*
        let cloud_storage = home.join("Library").join("CloudStorage");
        if cloud_storage.exists() {
            if let Ok(entries) = fs::read_dir(&cloud_storage) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or_default();
                    if name.starts_with("GoogleDrive-") {
                        // Check for "My Drive" or "Mon Drive"
                        let my_drive = path.join("My Drive").join("Vellum");
                        if my_drive.parent().map(|p| p.exists()).unwrap_or(false) {
                            return Some(my_drive);
                        }
                        let mon_drive = path.join("Mon Drive").join("Vellum");
                        if mon_drive.parent().map(|p| p.exists()).unwrap_or(false) {
                            return Some(mon_drive);
                        }
                        return Some(path.join("Vellum"));
                    }
                }
            }
        }

        // Standard ~/Google Drive
        let direct_gd = home.join("Google Drive").join("Vellum");
        if direct_gd.parent().map(|p| p.exists()).unwrap_or(false) {
            return Some(direct_gd);
        }

        // ~/GoogleDrive
        let direct_gd2 = home.join("GoogleDrive").join("Vellum");
        if direct_gd2.parent().map(|p| p.exists()).unwrap_or(false) {
            return Some(direct_gd2);
        }

        None
    }
}

impl CloudSyncBackend for FolderSyncBackend {
    fn name(&self) -> &str {
        "Google Drive Folder"
    }

    fn is_available(&self) -> bool {
        self.root_dir.exists()
    }

    fn list_deltas(&self) -> VellumResult<Vec<String>> {
        let deltas_dir = self.root_dir.join("sync").join("deltas");
        if !deltas_dir.exists() {
            return Ok(Vec::new());
        }

        let mut list = Vec::new();
        for entry in fs::read_dir(deltas_dir)?.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if ext.eq_ignore_ascii_case("json") {
                        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            list.push(name.to_string());
                        }
                    }
                }
            }
        }
        list.sort();
        Ok(list)
    }

    fn read_delta(&self, filename: &str) -> VellumResult<SyncDelta> {
        let path = self.root_dir.join("sync").join("deltas").join(filename);
        let content = fs::read_to_string(&path)?;
        let delta: SyncDelta = serde_json::from_str(&content)?;
        Ok(delta)
    }

    fn write_delta(&self, delta: &SyncDelta) -> VellumResult<String> {
        let deltas_dir = self.root_dir.join("sync").join("deltas");
        fs::create_dir_all(&deltas_dir)?;

        let filename = format!(
            "{}_{}.json",
            delta.client_id,
            delta.created_at.replace([':', '-'], "").replace('.', "_")
        );
        let target_path = deltas_dir.join(&filename);
        let temp_path = deltas_dir.join(format!(".tmp_{filename}"));

        let content = serde_json::to_string_pretty(delta)?;
        fs::write(&temp_path, content)?;
        fs::rename(temp_path, &target_path)?;

        Ok(filename)
    }

    fn upload_file(&self, local_path: &Path, rel_path: &str) -> VellumResult<()> {
        if !local_path.exists() {
            return Err(VellumError::NotFound(format!(
                "Fichier source introuvable: {}",
                local_path.display()
            )));
        }

        let target_path = self.root_dir.join("files").join(rel_path);
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Atomic write via temp file
        let temp_filename = format!(".tmp_{}", uuid::Uuid::new_v4());
        let parent_dir = target_path.parent().ok_or_else(|| {
            VellumError::InvalidArgument(format!(
                "Chemin cible de synchronisation invalide: {}",
                target_path.display()
            ))
        })?;
        let temp_path = parent_dir.join(temp_filename);
        fs::copy(local_path, &temp_path)?;
        fs::rename(temp_path, target_path)?;

        Ok(())
    }

    fn download_file(&self, rel_path: &str, local_path: &Path) -> VellumResult<bool> {
        let remote_path = self.root_dir.join("files").join(rel_path);
        if !remote_path.exists() {
            return Ok(false);
        }

        if let Some(parent) = local_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let temp_path = local_path.with_extension("tmp_download");
        fs::copy(&remote_path, &temp_path)?;
        fs::rename(temp_path, local_path)?;

        Ok(true)
    }

    fn file_exists(&self, rel_path: &str) -> VellumResult<bool> {
        let remote_path = self.root_dir.join("files").join(rel_path);
        Ok(remote_path.exists())
    }
}
