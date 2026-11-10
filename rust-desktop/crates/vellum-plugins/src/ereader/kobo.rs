use std::collections::HashMap;
use std::path::{Path, PathBuf};
use chrono::{DateTime, Utc};
use serde_json::Value;

use vellum_domain::{
    Book, ConfigField, DeviceAnnotation, DeviceBookInfo, DeviceKind, DeviceReadingProgress,
    DeviceSyncPlugin, PluginDescriptor, PluginType, TestResult, VellumError, VellumPlugin,
    VellumResult,
};

use super::kepubify;

pub struct KoboSyncPlugin {
    descriptor: PluginDescriptor,
}

impl Default for KoboSyncPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl KoboSyncPlugin {
    pub fn new() -> Self {
        let descriptor = PluginDescriptor {
            id: "device.kobo".to_string(),
            name: "Kobo eReader Sync (Import & Export)".to_string(),
            description: "Synchronisation complète avec les liseuses Kobo : import/export de livres, conversion automatique KEPUB, et synchronisation de l'avancement de lecture.".to_string(),
            version: "1.0.0".to_string(),
            plugin_type: PluginType::Device,
            external: false,
            command: None,
            supported_extensions: vec![
                "epub".to_string(),
                "kepub.epub".to_string(),
                "pdf".to_string(),
                "cbz".to_string(),
                "cbr".to_string(),
                "txt".to_string(),
            ],
            source_format: None,
            target_format: Some("kepub".to_string()),
            config_schema: vec![
                ConfigField::new(
                    "customMountPath",
                    "Point de montage personnalisé",
                    "string",
                    Value::String(String::new()),
                    "Chemin vers la liseuse si non détectée automatiquement (ex: /Volumes/KOBOeReader)",
                ),
                ConfigField::new(
                    "exportFormat",
                    "Format d'exportation",
                    "select",
                    Value::String("kepub".to_string()),
                    "Format privilégié lors de l'envoi vers Kobo (KEPUB active la pagination rapide par chapitre)",
                )
                .with_options(vec!["kepub", "epub"]),
                ConfigField::new(
                    "targetDirectory",
                    "Dossier de stockage sur la Kobo",
                    "string",
                    Value::String("Books".to_string()),
                    "Sous-dossier où déposer les livres sur la liseuse (par défaut: Books)",
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
}

impl VellumPlugin for KoboSyncPlugin {
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
                "Liseuse Kobo détectée avec succès sur: {}",
                mount.display()
            ))
        } else {
            TestResult::error(
                "Aucune liseuse Kobo détectée. Vérifiez la connexion USB ou spécifiez le point de montage.",
            )
        }
    }
}

impl DeviceSyncPlugin for KoboSyncPlugin {
    fn device_kind(&self) -> DeviceKind {
        DeviceKind::Kobo
    }

    fn default_mount_paths(&self) -> Vec<PathBuf> {
        let mut paths = vec![
            PathBuf::from("/Volumes/KOBOeReader"),
            PathBuf::from("/media/KOBOeReader"),
        ];

        // Linux dynamic mounts
        if let Ok(user) = std::env::var("USER") {
            paths.push(PathBuf::from(format!("/media/{user}/KOBOeReader")));
            paths.push(PathBuf::from(format!("/run/media/{user}/KOBOeReader")));
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
                && (custom.join(".kobo/KoboReader.sqlite").exists()
                    || custom.join(".kobo/version").exists())
            {
                return Some(custom.to_path_buf());
            }
        }

        for p in self.default_mount_paths() {
            if p.exists()
                && (p.join(".kobo/KoboReader.sqlite").exists()
                    || p.join(".kobo/version").exists())
            {
                return Some(p);
            }
        }

        None
    }

    fn list_books(&self, mount_point: &Path) -> VellumResult<Vec<DeviceBookInfo>> {
        let db_path = mount_point.join(".kobo/KoboReader.sqlite");
        if db_path.exists() {
            if let Ok(conn) = rusqlite::Connection::open_with_flags(
                &db_path,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            ) {
                let mut stmt = conn
                    .prepare(
                        "SELECT ContentID, Title, Attribution, ___PercentRead, MimeType 
                         FROM content 
                         WHERE ContentType = 6",
                    )
                    .map_err(|e| VellumError::Plugin(format!("Erreur SQL Kobo: {e}")))?;

                let rows = stmt
                    .query_map([], |row| {
                        let content_id: String = row.get(0)?;
                        let title: Option<String> = row.get(1)?;
                        let author: Option<String> = row.get(2)?;
                        let pct: Option<f64> = row.get(3)?;
                        let _mime: Option<String> = row.get(4)?;
                        Ok((content_id, title, author, pct))
                    })
                    .map_err(|e| VellumError::Plugin(format!("Erreur lecture content Kobo: {e}")))?;

                let mut books = Vec::new();
                for r in rows.flatten() {
                    let (content_id, title, author, pct) = r;
                    let file_path = PathBuf::from(&content_id);
                    let full_path = if file_path.is_absolute() {
                        file_path.clone()
                    } else {
                        mount_point.join(&file_path)
                    };

                    let size = std::fs::metadata(&full_path).map(|m| m.len()).unwrap_or(0);
                    let ext = full_path
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("epub")
                        .to_string();

                    let clean_title = title
                        .filter(|t| !t.trim().is_empty())
                        .unwrap_or_else(|| {
                            full_path
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("Sans titre")
                                .to_string()
                        });

                    books.push(DeviceBookInfo {
                        title: clean_title,
                        author: author.unwrap_or_else(|| "Auteur inconnu".to_string()),
                        filename: full_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("book")
                            .to_string(),
                        relative_path: file_path,
                        file_size: size,
                        format: ext,
                        progress_percent: pct,
                    });
                }

                if !books.is_empty() {
                    return Ok(books);
                }
            }
        }

        // Fallback: directory filesystem scan
        let mut books = Vec::new();
        let target_dir = mount_point.join("Books");
        let scan_roots = if target_dir.exists() {
            vec![target_dir, mount_point.to_path_buf()]
        } else {
            vec![mount_point.to_path_buf()]
        };

        for root in scan_roots {
            self.scan_directory(&root, mount_point, &mut books)?;
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
        let target_folder_name = config
            .get("targetDirectory")
            .and_then(|v| v.as_str())
            .unwrap_or("Books");

        let export_fmt = config
            .get("exportFormat")
            .and_then(|v| v.as_str())
            .unwrap_or("kepub");

        let template_str = config
            .get("namingTemplate")
            .and_then(|v| v.as_str())
            .unwrap_or(vellum_domain::DEFAULT_NAMING_TEMPLATE);

        let src_ext = source_file
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        let is_kepub = export_fmt == "kepub" && src_ext == "epub";
        let target_ext = if is_kepub { "kepub.epub" } else { &src_ext };

        let relative_dest = vellum_domain::NamingTemplate::format_filename(book, template_str, target_ext);
        let dest_file = mount_point.join(target_folder_name).join(relative_dest);

        if let Some(parent) = dest_file.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                VellumError::Plugin(format!(
                    "Impossible de créer le dossier cible sur la Kobo: {e}"
                ))
            })?;
        }

        if is_kepub {
            kepubify::convert_epub_to_kepub(source_file, &dest_file)?;
        } else {
            std::fs::copy(source_file, &dest_file).map_err(|e| {
                VellumError::Plugin(format!("Erreur lors de la copie sur la Kobo: {e}"))
            })?;
        }

        Ok(dest_file)
    }

    fn get_reading_progress(&self, mount_point: &Path) -> VellumResult<Vec<DeviceReadingProgress>> {
        let db_path = mount_point.join(".kobo/KoboReader.sqlite");
        if !db_path.exists() {
            return Ok(Vec::new());
        }

        let conn = rusqlite::Connection::open_with_flags(
            &db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .map_err(|e| VellumError::Plugin(format!("Erreur accès KoboReader.sqlite: {e}")))?;

        let mut stmt = conn
            .prepare(
                "SELECT Title, Attribution, ContentID, ___PercentRead, ReadStatus, DateLastRead 
                 FROM content 
                 WHERE ContentType = 6 AND ___PercentRead > 0",
            )
            .map_err(|e| VellumError::Plugin(format!("Erreur SQL Kobo: {e}")))?;

        let rows = stmt
            .query_map([], |row| {
                let title: Option<String> = row.get(0)?;
                let author: Option<String> = row.get(1)?;
                let content_id: Option<String> = row.get(2)?;
                let pct: f64 = row.get(3).unwrap_or(0.0);
                let read_status: i32 = row.get(4).unwrap_or(0);
                let date_str: Option<String> = row.get(5)?;
                Ok((title, author, content_id, pct, read_status, date_str))
            })
            .map_err(|e| VellumError::Plugin(format!("Erreur lecture avancement: {e}")))?;

        let mut list = Vec::new();
        for r in rows.flatten() {
            let (title, author, content_id, pct, status, date_str) = r;
            let last_read = date_str.and_then(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .map(|dt| dt.with_timezone(&Utc))
                    .ok()
            });

            if let Some(t) = title {
                list.push(DeviceReadingProgress {
                    book_title: t,
                    book_author: author,
                    file_path: content_id,
                    progress_percent: pct,
                    last_read,
                    is_finished: status == 2 || pct >= 99.0,
                });
            }
        }

        Ok(list)
    }

    fn get_annotations(&self, mount_point: &Path) -> VellumResult<Vec<DeviceAnnotation>> {
        let db_path = mount_point.join(".kobo/KoboReader.sqlite");
        if !db_path.exists() {
            return Ok(Vec::new());
        }

        let conn = rusqlite::Connection::open_with_flags(
            &db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .map_err(|e| VellumError::Plugin(format!("Erreur accès KoboReader.sqlite: {e}")))?;

        let mut stmt = conn
            .prepare(
                "SELECT VolumeID, Text, Annotation, ChapterProgress, DateCreated 
                 FROM Bookmark 
                 WHERE Text IS NOT NULL OR Annotation IS NOT NULL",
            )
            .map_err(|e| VellumError::Plugin(format!("Erreur SQL Bookmarks: {e}")))?;

        let rows = stmt
            .query_map([], |row| {
                let volume_id: String = row.get(0)?;
                let text: Option<String> = row.get(1)?;
                let note: Option<String> = row.get(2)?;
                let progress: Option<f64> = row.get(3)?;
                let date_str: Option<String> = row.get(4)?;
                Ok((volume_id, text, note, progress, date_str))
            })
            .map_err(|e| VellumError::Plugin(format!("Erreur lecture Bookmark: {e}")))?;

        let mut list = Vec::new();
        for r in rows.flatten() {
            let (volume_id, text, note, progress, date_str) = r;
            let created_at = date_str.and_then(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .map(|dt| dt.with_timezone(&Utc))
                    .ok()
            });

            // VolumeID is often the path or file stem
            let title = Path::new(&volume_id)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(&volume_id)
                .to_string();

            list.push(DeviceAnnotation {
                book_title: title,
                book_author: None,
                text: text.unwrap_or_default(),
                note,
                location: progress.map(|p| format!("{:.0}%", p * 100.0)),
                created_at,
            });
        }

        Ok(list)
    }
}

impl KoboSyncPlugin {
    fn scan_directory(
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
                    if !dir_name.starts_with('.') {
                        let _ = self.scan_directory(&p, mount, out);
                    }
                } else if p.is_file() {
                    let ext = p
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    if matches!(
                        ext.as_str(),
                        "epub" | "pdf" | "cbz" | "cbr" | "txt" | "mobi"
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
