use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use chrono::Utc;
use tracing::{debug, info, warn};
use vellum_domain::{
    CloudSyncConfig, SyncAction, SyncDelta, SyncEvent, SyncReport, VellumError, VellumResult,
};
use vellum_storage::{BookRepository, SyncRepository, TagRepository};

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

/// Backend utilisant un dossier partagé synchronisé par Google Drive Desktop
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
        let temp_path = target_path.parent().unwrap().join(temp_filename);
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

/// Service principal de synchronisation décentralisée
#[derive(Clone)]
pub struct SyncService {
    config: Arc<RwLock<CloudSyncConfig>>,
    sync_repo: SyncRepository,
    book_repo: BookRepository,
    tag_repo: TagRepository,
    library_path: PathBuf,
    backend: Arc<RwLock<Option<Box<dyn CloudSyncBackend>>>>,
}

impl SyncService {
    pub fn new(
        config: CloudSyncConfig,
        sync_repo: SyncRepository,
        book_repo: BookRepository,
        tag_repo: TagRepository,
        library_path: PathBuf,
    ) -> Self {
        let mut config = config;
        let mut backend_opt: Option<Box<dyn CloudSyncBackend>> = None;

        if let Some(ref folder) = config.sync_folder_path {
            backend_opt = Some(Box::new(FolderSyncBackend::new(folder.clone())));
        } else if let Some(detected) = FolderSyncBackend::auto_detect_google_drive() {
            config.sync_folder_path = Some(detected.clone());
            config.enabled = true;
            backend_opt = Some(Box::new(FolderSyncBackend::new(detected)));
        }

        Self {
            config: Arc::new(RwLock::new(config)),
            sync_repo,
            book_repo,
            tag_repo,
            library_path,
            backend: Arc::new(RwLock::new(backend_opt)),
        }
    }

    pub fn config(&self) -> CloudSyncConfig {
        self.config.read().map(|c| c.clone()).unwrap_or_default()
    }

    pub fn set_sync_folder(&self, folder: PathBuf) -> VellumResult<()> {
        let backend = FolderSyncBackend::new(folder.clone());
        if let Ok(mut cfg) = self.config.write() {
            cfg.sync_folder_path = Some(folder);
            cfg.enabled = true;
        }
        if let Ok(mut b) = self.backend.write() {
            *b = Some(Box::new(backend));
        }
        Ok(())
    }

    pub fn set_enabled(&self, enabled: bool) {
        if let Ok(mut cfg) = self.config.write() {
            cfg.enabled = enabled;
        }
    }

    pub fn set_auto_download_books(&self, auto_download: bool) {
        if let Ok(mut cfg) = self.config.write() {
            cfg.auto_download_books = auto_download;
        }
    }

    pub fn is_backend_available(&self) -> bool {
        let Ok(guard) = self.backend.read() else {
            return false;
        };
        guard.as_ref().map(|b| b.is_available()).unwrap_or(false)
    }

    pub fn client_id(&self) -> String {
        self.config
            .read()
            .map(|c| c.client_id.clone())
            .unwrap_or_else(|_| "unknown-client".to_string())
    }

    pub fn record_event(&self, action: SyncAction) -> VellumResult<()> {
        let client_id = self.client_id();
        let event = SyncEvent::new(client_id, action);
        self.sync_repo.record_event(&event)
    }

    /// Exécute la synchronisation bidirectionnelle complète
    pub fn sync_now(&self) -> VellumResult<SyncReport> {
        let client_id = self.client_id();
        let auto_download = self.config().auto_download_books;

        let backend_guard = self.backend.read().map_err(|e| {
            VellumError::System(format!("Échec de verrouillage du backend de synchro: {e}"))
        })?;

        let Some(backend) = backend_guard.as_ref() else {
            return Err(VellumError::NotFound(
                "Aucun backend Google Drive configuré ou disponible.".to_string(),
            ));
        };

        if !backend.is_available() {
            return Err(VellumError::NotFound(
                "Le dossier Google Drive est inaccessible ou non monté.".to_string(),
            ));
        }

        let mut report = SyncReport {
            timestamp: Utc::now().to_rfc3339(),
            ..Default::default()
        };

        // -------------------------------------------------------------
        // Phase 1 : Pousser les nouveaux livres locaux & les événements
        // -------------------------------------------------------------
        let unsynced_events = self.sync_repo.get_unsynced_events()?;

        if !unsynced_events.is_empty() {
            info!(
                "Sync: {} événements locaux à pousser vers Google Drive",
                unsynced_events.len()
            );

            // Pour chaque création de livre, assurer l'upload des binaires (EPUB/PDF/Cover)
            for ev in &unsynced_events {
                if let SyncAction::BookCreated { book, formats, .. } = &ev.action {
                    // Upload des fichiers de formats
                    for fmt in formats {
                        let local_file = self.library_path.join(&fmt.filepath);
                        if local_file.exists() {
                            if !backend.file_exists(&fmt.filepath)? {
                                backend.upload_file(&local_file, &fmt.filepath)?;
                                report.uploaded_books += 1;
                                info!("Sync: Upload du livre '{}' ({})", fmt.filename, fmt.format);
                            }
                        }
                    }

                    // Upload de la couverture si présente
                    if let Some(ref cover_rel) = book.cover_path {
                        let local_cover = self.library_path.join(cover_rel);
                        if local_cover.exists() {
                            if !backend.file_exists(cover_rel)? {
                                let _ = backend.upload_file(&local_cover, cover_rel);
                            }
                        }
                    }
                }
            }

            // Générer le paquet de delta
            let delta = SyncDelta {
                client_id: client_id.clone(),
                created_at: Utc::now().to_rfc3339(),
                events: unsynced_events.clone(),
            };

            let delta_filename = backend.write_delta(&delta)?;
            let event_ids: Vec<String> = unsynced_events.iter().map(|e| e.id.clone()).collect();
            self.sync_repo
                .mark_events_synced(&event_ids, &report.timestamp)?;
            self.sync_repo
                .mark_delta_applied(&delta_filename, &client_id)?;

            report.pushed_events = unsynced_events.len();
        }

        // -------------------------------------------------------------
        // Phase 2 : Récupérer les deltas distants des autres clients
        // -------------------------------------------------------------
        let all_remote_deltas = backend.list_deltas()?;

        for delta_name in all_remote_deltas {
            // Ignorer si déjà appliqué
            if self.sync_repo.is_delta_applied(&delta_name)? {
                continue;
            }

            // Ignorer nos propres deltas
            if delta_name.starts_with(&client_id) {
                let _ = self.sync_repo.mark_delta_applied(&delta_name, &client_id);
                continue;
            }

            debug!("Sync: Application du delta distant: {}", delta_name);
            let remote_delta = match backend.read_delta(&delta_name) {
                Ok(d) => d,
                Err(e) => {
                    warn!("Impossible de lire le delta {}: {}", delta_name, e);
                    continue;
                }
            };

            for ev in remote_delta.events {
                match ev.action {
                    SyncAction::BookCreated {
                        mut book,
                        formats,
                        tags,
                    } => {
                        let existing = self.book_repo.find_by_id(&book.id)?;
                        if existing.is_none() {
                            // Enregistrer tags
                            for tag in &tags {
                                let _ = self.tag_repo.save(tag);
                            }

                            // Téléchargement automatique des formats si demandé
                            if auto_download {
                                for fmt in &formats {
                                    let target_local = self.library_path.join(&fmt.filepath);
                                    if !target_local.exists() {
                                        if let Ok(true) =
                                            backend.download_file(&fmt.filepath, &target_local)
                                        {
                                            report.downloaded_books += 1;
                                            info!(
                                                "Sync: Téléchargé depuis Google Drive: {}",
                                                fmt.filename
                                            );
                                        }
                                    }
                                }

                                if let Some(ref cover_rel) = book.cover_path {
                                    let target_cover = self.library_path.join(cover_rel);
                                    if !target_cover.exists() {
                                        let _ = backend.download_file(cover_rel, &target_cover);
                                    }
                                }
                            }

                            book.formats = formats;
                            book.tags = tags;
                            let _ = self.book_repo.save(&book);
                            info!(
                                "Sync: Nouveau livre intégré depuis un autre client: '{}' par {}",
                                book.title, book.author
                            );
                        }
                    }

                    SyncAction::BookUpdated {
                        book: updated_book,
                        tags,
                    } => {
                        if let Ok(Some(mut local_book)) = self.book_repo.find_by_id(&updated_book.id) {
                            local_book.title = updated_book.title;
                            local_book.author = updated_book.author;
                            local_book.publisher = updated_book.publisher;
                            local_book.publish_date = updated_book.publish_date;
                            local_book.isbn = updated_book.isbn;
                            local_book.description = updated_book.description;

                            local_book.tags.clear();
                            for tag in tags {
                                let _ = self.tag_repo.save(&tag);
                                local_book.add_tag(tag);
                            }
                            let _ = self.book_repo.save(&local_book);
                        }
                    }

                    SyncAction::BookDeleted { book_id } => {
                        let _ = self.book_repo.delete(&book_id);
                        info!("Sync: Suppression du livre {} répercutée localement", book_id);
                    }

                    SyncAction::ProgressUpdated {
                        book_id,
                        progress_percent,
                        current_chapter_index,
                        total_chapters,
                        last_read_at,
                        is_finished,
                    } => {
                        if let Ok(Some(mut local_book)) = self.book_repo.find_by_id(&book_id) {
                            // Comparer les horodatages de lecture (LWW)
                            let should_update = match &local_book.last_read_at {
                                None => true,
                                Some(local_time) => last_read_at >= *local_time,
                            };

                            if should_update {
                                local_book.progress_percent = progress_percent;
                                local_book.current_chapter_index = current_chapter_index;
                                local_book.total_chapters = total_chapters;
                                local_book.last_read_at = Some(last_read_at);
                                local_book.is_finished = is_finished;
                                let _ = self.book_repo.save(&local_book);
                                debug!(
                                    "Sync: Progression mise à jour pour '{}' -> {:.1}%",
                                    local_book.title, progress_percent
                                );
                            }
                        }
                    }
                }
                report.pulled_events += 1;
            }

            let _ = self
                .sync_repo
                .mark_delta_applied(&delta_name, &remote_delta.client_id);
        }

        // Mettre à jour l'horodatage de dernière synchro
        if let Ok(mut cfg) = self.config.write() {
            cfg.last_synced_at = Some(report.timestamp.clone());
        }

        info!(
            "Sync terminé avec succès : +{} livres envoyés, +{} livres reçus, {} événements traités",
            report.uploaded_books, report.downloaded_books, report.pulled_events
        );

        Ok(report)
    }
}
