//! Surveillance de dossier (Auto-Add / Watch Folder) pour importer automatiquement les nouveaux ebooks.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use vellum_domain::VellumResult;

use crate::BookService;

const SUPPORTED_EXTENSIONS: &[&str] = &["epub", "pdf", "cbz", "cbr", "mobi", "azw3", "txt"];

/// Rapport d'exécution d'un scan de dossier surveillé.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatchFolderReport {
    pub scanned_files: usize,
    pub imported_book_ids: Vec<String>,
    pub skipped_duplicates: usize,
    pub errors: Vec<String>,
}

/// Service d'import automatique depuis un dossier surveillé.
pub struct WatchFolderService;

impl WatchFolderService {
    /// Scanne `watch_dir` (non récursif) et importe les nouveaux livres reconnus.
    /// Si `move_to_processed` est vrai, les fichiers importés sont déplacés dans `watch_dir/processed/`.
    pub fn scan_once(
        book_service: &BookService,
        watch_dir: &Path,
        move_to_processed: bool,
    ) -> VellumResult<WatchFolderReport> {
        let mut report = WatchFolderReport::default();
        if !watch_dir.exists() || !watch_dir.is_dir() {
            return Ok(report);
        }

        let entries = fs::read_dir(watch_dir)?;
        let mut candidate_files: Vec<PathBuf> = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && Self::is_supported_ebook(&path) {
                candidate_files.push(path);
            }
        }
        candidate_files.sort();
        report.scanned_files = candidate_files.len();

        for file_path in candidate_files {
            let meta = book_service.metadata_service().extract_metadata(&file_path);
            let fallback_title = file_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            let title = meta.title.as_deref().unwrap_or(&fallback_title);
            let author = meta.author.as_deref().unwrap_or("Auteur inconnu");

            if let Ok(Some(_)) = book_service.check_duplicate(title, author) {
                report.skipped_duplicates += 1;
                continue;
            }

            match book_service.import_book(&file_path) {
                Ok(book) => {
                    report.imported_book_ids.push(book.id);
                    if move_to_processed {
                        let processed_dir = watch_dir.join("processed");
                        if fs::create_dir_all(&processed_dir).is_ok() {
                            if let Some(fname) = file_path.file_name() {
                                let safe_name =
                                    BookService::sanitize_filename(&fname.to_string_lossy());
                                let dest = processed_dir.join(safe_name);
                                let _ = fs::rename(&file_path, &dest);
                            }
                        }
                    }
                }
                Err(e) => {
                    report.errors.push(format!("{}: {e}", file_path.display()));
                }
            }
        }

        Ok(report)
    }

    fn is_supported_ebook(path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|ext| {
                let lower = ext.to_ascii_lowercase();
                SUPPORTED_EXTENSIONS.contains(&lower.as_str())
            })
            .unwrap_or(false)
    }
}
