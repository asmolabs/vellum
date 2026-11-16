use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use vellum_domain::{
    Book, ConfigField, DeviceAnnotation, DeviceBookInfo, DeviceKind, DeviceReadingProgress,
    DeviceSyncPlugin, PluginDescriptor, PluginType, TestResult, VellumError, VellumPlugin,
    VellumResult,
};

use super::clippings;

pub struct KindleSyncPlugin {
    descriptor: PluginDescriptor,
}

impl Default for KindleSyncPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl KindleSyncPlugin {
    pub fn new() -> Self {
        let descriptor = PluginDescriptor {
            id: "device.kindle".to_string(),
            name: "Amazon Kindle Sync (Import & Export)".to_string(),
            description: "Synchronisation complète avec les liseuses Amazon Kindle : import/export de livres (AZW3, MOBI, PDF), génération des vignettes de couvertures et synchronisation des surlignages/notes (My Clippings).".to_string(),
            version: "1.0.0".to_string(),
            plugin_type: PluginType::Device,
            external: false,
            command: None,
            supported_extensions: vec![
                "azw3".to_string(),
                "mobi".to_string(),
                "pdf".to_string(),
                "txt".to_string(),
                "prc".to_string(),
                "kfx".to_string(),
            ],
            source_format: None,
            target_format: Some("azw3".to_string()),
            config_schema: vec![
                ConfigField::new(
                    "customMountPath",
                    "Point de montage personnalisé",
                    "string",
                    Value::String(String::new()),
                    "Chemin vers la liseuse si non détectée automatiquement (ex: /Volumes/Kindle)",
                ),
                ConfigField::new(
                    "targetDirectory",
                    "Dossier de stockage sur le Kindle",
                    "string",
                    Value::String("documents".to_string()),
                    "Dossier où placer les livres sur la liseuse (par défaut: documents)",
                ),
                ConfigField::new(
                    "namingTemplate",
                    "Gabarit de nommage des fichiers",
                    "string",
                    Value::String(vellum_domain::DEFAULT_NAMING_TEMPLATE.to_string()),
                    "Modèle utilisé pour nommer les dossiers et fichiers sur la liseuse (ex: {author}/{series}/T.{series_index:02} - {title})",
                ),
                ConfigField::new(
                    "generateThumbnails",
                    "Générer les vignettes de couverture",
                    "boolean",
                    Value::Bool(true),
                    "Injecte les miniatures dans system/thumbnails pour affichage sur l'écran d'accueil du Kindle",
                ),
            ],
        };

        Self { descriptor }
    }
}

impl VellumPlugin for KindleSyncPlugin {
    fn descriptor(&self) -> &PluginDescriptor {
        &self.descriptor
    }

    fn as_device_plugin(&self) -> Option<&dyn DeviceSyncPlugin> {
        Some(self)
    }

    fn test_configuration(&self, config: &HashMap<String, Value>) -> TestResult {
        let custom = config
            .get("customMountPath")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .map(Path::new);

        if let Some(mount) = self.detect_device(custom) {
            TestResult::ok(format!(
                "Liseuse Amazon Kindle détectée avec succès sur: {}",
                mount.display()
            ))
        } else {
            TestResult::error(
                "Aucune liseuse Kindle détectée. Vérifiez la connexion USB ou spécifiez le point de montage.",
            )
        }
    }
}

impl DeviceSyncPlugin for KindleSyncPlugin {
    fn device_kind(&self) -> DeviceKind {
        DeviceKind::Kindle
    }

    fn default_mount_paths(&self) -> Vec<PathBuf> {
        let mut paths = vec![
            PathBuf::from("/Volumes/Kindle"),
            PathBuf::from("/media/Kindle"),
        ];

        // Linux dynamic mounts
        if let Ok(user) = std::env::var("USER") {
            paths.push(PathBuf::from(format!("/media/{user}/Kindle")));
            paths.push(PathBuf::from(format!("/run/media/{user}/Kindle")));
        }

        // Windows drives
        for drive in ['D', 'E', 'F', 'G', 'H', 'I'] {
            paths.push(PathBuf::from(format!("{drive}:\\")));
        }

        paths
    }

    fn detect_device(&self, custom_mount: Option<&Path>) -> Option<PathBuf> {
        if let Some(custom) = custom_mount {
            if custom.exists()
                && custom.join("documents").exists()
                && (custom.join("system").exists()
                    || custom
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .contains("Kindle"))
            {
                return Some(custom.to_path_buf());
            }
        }

        self.default_mount_paths()
            .into_iter()
            .find(|p| p.exists() && p.join("documents").exists())
    }

    fn list_books(&self, mount_point: &Path) -> VellumResult<Vec<DeviceBookInfo>> {
        let mut books = Vec::new();
        let docs_dir = mount_point.join("documents");

        if docs_dir.exists() {
            self.scan_documents_dir(&docs_dir, mount_point, &mut books)?;
        }

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
            .unwrap_or("documents");

        let gen_thumbnails = config
            .get("generateThumbnails")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let template_str = config
            .get("namingTemplate")
            .and_then(|v| v.as_str())
            .unwrap_or(vellum_domain::DEFAULT_NAMING_TEMPLATE);

        let src_ext = source_file
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("mobi")
            .to_lowercase();

        let relative_dest =
            vellum_domain::NamingTemplate::format_filename(book, template_str, &src_ext);
        let dest_file = mount_point.join(target_dir_name).join(relative_dest);

        if let Some(parent) = dest_file.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                VellumError::Plugin(format!(
                    "Impossible de créer le dossier cible sur le Kindle: {e}"
                ))
            })?;
        }

        std::fs::copy(source_file, &dest_file).map_err(|e| {
            VellumError::Plugin(format!("Erreur lors de la copie sur le Kindle: {e}"))
        })?;

        // Optional Kindle cover thumbnail generation
        if gen_thumbnails {
            if let Some(ref cov_path_str) = book.cover_path {
                let cov_path = Path::new(cov_path_str);
                if cov_path.exists() {
                    let thumbs_dir = mount_point.join("system").join("thumbnails");
                    let _ = std::fs::create_dir_all(&thumbs_dir);

                    // Kindle searches thumbnails by hash/ASIN: thumbnail_<UUID>_EBOK_portrait.jpg
                    let asin_or_uuid = &book.id;
                    let thumb_dest =
                        thumbs_dir.join(format!("thumbnail_{asin_or_uuid}_EBOK_portrait.jpg"));
                    let _ = std::fs::copy(cov_path, thumb_dest);
                }
            }
        }

        Ok(dest_file)
    }

    fn get_reading_progress(&self, mount_point: &Path) -> VellumResult<Vec<DeviceReadingProgress>> {
        // Kindle does not store reading progress in a simple sqlite DB like Kobo,
        // but recent positions are reflected in My Clippings or .sdr sidecar files.
        let clippings_file = mount_point.join("documents").join("My Clippings.txt");
        if !clippings_file.exists() {
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(&clippings_file).unwrap_or_default();
        let annotations = clippings::parse_kindle_clippings(&content);

        let mut map: HashMap<String, (Option<String>, Option<chrono::DateTime<chrono::Utc>>)> =
            HashMap::new();
        for ann in annotations {
            let entry = map
                .entry(ann.book_title.clone())
                .or_insert((ann.book_author, ann.created_at));
            if ann.created_at.is_some() && ann.created_at > entry.1 {
                entry.1 = ann.created_at;
            }
        }

        let mut list = Vec::new();
        for (title, (author, last_read)) in map {
            list.push(DeviceReadingProgress {
                book_title: title,
                book_author: author,
                file_path: None,
                progress_percent: 50.0, // Indication that book is being read
                last_read,
                is_finished: false,
            });
        }

        Ok(list)
    }

    fn get_annotations(&self, mount_point: &Path) -> VellumResult<Vec<DeviceAnnotation>> {
        let clippings_file = mount_point.join("documents").join("My Clippings.txt");
        if !clippings_file.exists() {
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(&clippings_file)
            .map_err(|e| VellumError::Plugin(format!("Erreur lecture My Clippings.txt: {e}")))?;

        Ok(clippings::parse_kindle_clippings(&content))
    }
}

impl KindleSyncPlugin {
    fn scan_documents_dir(
        &self,
        dir: &Path,
        mount: &Path,
        out: &mut Vec<DeviceBookInfo>,
    ) -> VellumResult<()> {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    let dir_name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    // Skip hidden dirs and sidecar annotation folders (.sdr)
                    if !dir_name.starts_with('.') && !dir_name.ends_with(".sdr") {
                        let _ = self.scan_documents_dir(&p, mount, out);
                    }
                } else if p.is_file() {
                    let fname = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if fname.eq_ignore_ascii_case("My Clippings.txt")
                        || fname.to_lowercase().starts_with("my clippings")
                    {
                        continue;
                    }

                    let ext = p
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    if matches!(
                        ext.as_str(),
                        "azw3" | "mobi" | "pdf" | "txt" | "prc" | "kfx"
                    ) {
                        let stem = p
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("book")
                            .to_string();
                        let size = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
                        let rel = p.strip_prefix(mount).unwrap_or(&p).to_path_buf();

                        out.push(DeviceBookInfo {
                            title: stem,
                            author: "Inconnu".to_string(),
                            filename: p
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("book")
                                .to_string(),
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
