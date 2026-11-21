use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use vellum_domain::{
    Book, ConfigField, DeviceAnnotation, DeviceBookInfo, DeviceKind, DeviceReadingProgress,
    DeviceSyncPlugin, PluginDescriptor, PluginType, TestResult, VellumError, VellumPlugin,
    VellumResult,
};

pub struct GenericSyncPlugin {
    descriptor: PluginDescriptor,
}

impl Default for GenericSyncPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl GenericSyncPlugin {
    pub fn new() -> Self {
        let descriptor = PluginDescriptor {
            id: "device.generic".to_string(),
            name: "Liseuse USB Générique / Xteink / Carte SD".to_string(),
            description: "Synchronisation avec les liseuses USB génériques (Xteink, Boox, Tolino, PocketBook, clé USB ou carte SD).".to_string(),
            version: "1.0.0".to_string(),
            plugin_type: PluginType::Device,
            external: false,
            command: None,
            supported_extensions: vec![
                "epub".to_string(),
                "pdf".to_string(),
                "txt".to_string(),
                "cbz".to_string(),
                "cbr".to_string(),
                "mobi".to_string(),
                "azw3".to_string(),
            ],
            source_format: None,
            target_format: Some("epub".to_string()),
            config_schema: vec![
                ConfigField::new(
                    "customMountPath",
                    "Point de montage personnalisé",
                    "string",
                    Value::String(String::new()),
                    "Chemin vers la liseuse ou le dossier monté (ex: /Volumes/XTEINK ou /Volumes/NO NAME)",
                ),
                ConfigField::new(
                    "targetDirectory",
                    "Dossier de stockage",
                    "string",
                    Value::String("books".to_string()),
                    "Sous-dossier où déposer les livres sur l'appareil (ex: books, ebooks, ou laisser vide pour la racine)",
                ),
                ConfigField::new(
                    "namingTemplate",
                    "Gabarit de nommage des fichiers",
                    "string",
                    Value::String(vellum_domain::DEFAULT_NAMING_TEMPLATE.to_string()),
                    "Modèle utilisé pour nommer les dossiers et fichiers sur la liseuse (ex: {author}/{series}/T.{series_index:02} - {title})",
                ),
            ],
        };

        Self { descriptor }
    }

    fn scan_directory(
        &self,
        dir: &Path,
        mount: &Path,
        depth: usize,
        out: &mut Vec<DeviceBookInfo>,
    ) -> VellumResult<()> {
        if depth > 5 {
            return Ok(());
        }

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                let file_name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");

                // Skip hidden and system folders
                if file_name.starts_with('.')
                    || file_name.eq_ignore_ascii_case("System Volume Information")
                    || file_name.eq_ignore_ascii_case("$RECYCLE.BIN")
                    || file_name.eq_ignore_ascii_case(".Spotlight-V100")
                    || file_name.eq_ignore_ascii_case(".Trashes")
                    || file_name.eq_ignore_ascii_case(".fseventsd")
                {
                    continue;
                }

                if p.is_dir() {
                    let _ = self.scan_directory(&p, mount, depth + 1, out);
                } else if p.is_file() {
                    let ext = p
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("")
                        .to_lowercase();

                    if matches!(
                        ext.as_str(),
                        "epub" | "pdf" | "cbz" | "cbr" | "txt" | "mobi" | "azw3"
                    ) {
                        let stem = p
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("book")
                            .to_string();

                        // Try to split "Author - Title" if present
                        let (author, title) = if let Some((a, t)) = stem.split_once(" - ") {
                            (a.trim().to_string(), t.trim().to_string())
                        } else {
                            ("Auteur inconnu".to_string(), stem)
                        };

                        let size = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
                        let rel = p.strip_prefix(mount).unwrap_or(&p).to_path_buf();

                        out.push(DeviceBookInfo {
                            title,
                            author,
                            filename: file_name.to_string(),
                            relative_path: rel,
                            file_size: size,
                            format: ext,
                            progress_percent: None,
                        });
                    }
                }
            }
        }

        Ok(())
    }
}

impl VellumPlugin for GenericSyncPlugin {
    fn descriptor(&self) -> &PluginDescriptor {
        &self.descriptor
    }

    fn test_configuration(&self, config: &HashMap<String, Value>) -> TestResult {
        let custom = config
            .get("customMountPath")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .map(Path::new);

        if let Some(mount) = self.detect_device(custom) {
            TestResult::ok(format!(
                "Périphérique générique / Xteink détecté avec succès sur: {}",
                mount.display()
            ))
        } else {
            TestResult::error(
                "Aucun périphérique USB / carte SD détecté. Spécifiez le point de montage dans la configuration.",
            )
        }
    }
}

impl DeviceSyncPlugin for GenericSyncPlugin {
    fn device_kind(&self) -> DeviceKind {
        DeviceKind::Generic
    }

    fn default_mount_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // macOS: scan /Volumes for external volumes
        if let Ok(entries) = std::fs::read_dir("/Volumes") {
            for entry in entries.flatten() {
                let p = entry.path();
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name.is_empty()
                    || name == "Macintosh HD"
                    || name == "Recovery"
                    || name.starts_with('.')
                {
                    continue;
                }
                // Check if it looks like an e-reader or storage device
                let lower = name.to_lowercase();
                if lower.contains("xteink")
                    || lower.contains("xte")
                    || lower.contains("eink")
                    || lower.contains("reader")
                    || lower.contains("no name")
                    || lower.contains("untitled")
                    || p.join("books").exists()
                    || p.join("Books").exists()
                    || p.join("ebooks").exists()
                    || p.join("documents").exists()
                {
                    paths.push(p);
                }
            }
        }

        // Linux dynamic mounts
        if let Ok(user) = std::env::var("USER") {
            for base in [format!("/media/{user}"), format!("/run/media/{user}")] {
                let bp = PathBuf::from(base);
                if let Ok(entries) = std::fs::read_dir(&bp) {
                    for entry in entries.flatten() {
                        paths.push(entry.path());
                    }
                }
            }
        }

        // Windows drives
        for drive in ['D', 'E', 'F', 'G', 'H', 'I'] {
            let p = PathBuf::from(format!("{drive}:\\"));
            if p.exists() {
                paths.push(p);
            }
        }

        paths
    }

    fn detect_device(&self, custom_mount: Option<&Path>) -> Option<PathBuf> {
        if let Some(custom) = custom_mount {
            if custom.exists() {
                return Some(custom.to_path_buf());
            }
        }

        self.default_mount_paths().into_iter().find(|p| p.exists())
    }

    fn list_books(&self, mount_point: &Path) -> VellumResult<Vec<DeviceBookInfo>> {
        let mut books = Vec::new();
        self.scan_directory(mount_point, mount_point, 0, &mut books)?;
        Ok(books)
    }

    fn export_book(
        &self,
        book: &Book,
        source_file: &Path,
        mount_point: &Path,
        config: &HashMap<String, Value>,
    ) -> VellumResult<PathBuf> {
        let target_dir_name = config
            .get("targetDirectory")
            .and_then(|v| v.as_str())
            .unwrap_or("books")
            .trim();

        let template_str = config
            .get("namingTemplate")
            .and_then(|v| v.as_str())
            .unwrap_or(vellum_domain::DEFAULT_NAMING_TEMPLATE);

        let src_ext = source_file
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("epub")
            .to_lowercase();

        let relative_dest =
            vellum_domain::NamingTemplate::format_filename(book, template_str, &src_ext);

        let dest_file = if target_dir_name.is_empty() {
            mount_point.join(relative_dest)
        } else {
            mount_point.join(target_dir_name).join(relative_dest)
        };

        if let Some(parent) = dest_file.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                VellumError::Plugin(format!(
                    "Impossible de créer le dossier cible sur la liseuse: {e}"
                ))
            })?;
        }

        std::fs::copy(source_file, &dest_file).map_err(|e| {
            VellumError::Plugin(format!("Erreur lors de la copie sur la liseuse: {e}"))
        })?;

        Ok(dest_file)
    }

    fn get_reading_progress(
        &self,
        _mount_point: &Path,
    ) -> VellumResult<Vec<DeviceReadingProgress>> {
        Ok(Vec::new())
    }

    fn get_annotations(&self, _mount_point: &Path) -> VellumResult<Vec<DeviceAnnotation>> {
        Ok(Vec::new())
    }
}
