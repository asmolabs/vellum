//! Service principal de synchronisation décentralisée (journal d'événements et deltas).

mod folder_backend;
pub mod gdrive_api_backend;
pub mod gdrive_oauth;

use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use chrono::Utc;
use tracing::{debug, info, warn};
use vellum_domain::{
    CloudSyncConfig, CloudSyncMode, SyncAction, SyncDelta, SyncEvent, SyncReport, VellumError,
    VellumResult,
};
use vellum_storage::{BookRepository, SyncRepository, TagRepository};

pub use folder_backend::{CloudSyncBackend, FolderSyncBackend};
pub use gdrive_api_backend::GoogleDriveApiBackend;
pub use gdrive_oauth::GoogleDriveTokens;

/// Service principal de synchronisation décentralisée
#[derive(Clone)]
pub struct SyncService {
    config: Arc<RwLock<CloudSyncConfig>>,
    sync_repo: SyncRepository,
    book_repo: BookRepository,
    tag_repo: TagRepository,
    library_path: PathBuf,
    tokens_path: PathBuf,
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
        let tokens_path = gdrive_oauth::default_tokens_path();

        if config.mode == CloudSyncMode::GoogleDriveFolder && config.sync_folder_path.is_none() {
            if let Some(detected) = FolderSyncBackend::auto_detect_google_drive() {
                config.sync_folder_path = Some(detected);
                config.enabled = true;
            }
        }

        let backend_opt = Self::build_backend(&config, &tokens_path);

        Self {
            config: Arc::new(RwLock::new(config)),
            sync_repo,
            book_repo,
            tag_repo,
            library_path,
            tokens_path,
            backend: Arc::new(RwLock::new(backend_opt)),
        }
    }

    pub fn with_tokens_path(mut self, tokens_path: PathBuf) -> Self {
        self.tokens_path = tokens_path;
        self.refresh_backend();
        self
    }

    fn build_backend(
        config: &CloudSyncConfig,
        tokens_path: &Path,
    ) -> Option<Box<dyn CloudSyncBackend>> {
        match config.mode {
            CloudSyncMode::GoogleDriveFolder => config.sync_folder_path.as_ref().map(|folder| {
                Box::new(FolderSyncBackend::new(folder.clone())) as Box<dyn CloudSyncBackend>
            }),
            CloudSyncMode::GoogleDriveApi => {
                let client_id = config
                    .google_client_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())?;
                GoogleDriveApiBackend::new(
                    client_id.to_string(),
                    config.google_client_secret.clone(),
                    config.google_drive_folder_name.clone(),
                    tokens_path.to_path_buf(),
                )
                .ok()
                .map(|b| Box::new(b) as Box<dyn CloudSyncBackend>)
            }
        }
    }

    fn refresh_backend(&self) {
        let cfg = self.config();
        let new_backend = Self::build_backend(&cfg, &self.tokens_path);
        if let Ok(mut b) = self.backend.write() {
            *b = new_backend;
        }
    }

    pub fn config(&self) -> CloudSyncConfig {
        self.config.read().map(|c| c.clone()).unwrap_or_default()
    }

    pub fn set_sync_mode(&self, mode: CloudSyncMode) {
        if let Ok(mut cfg) = self.config.write() {
            cfg.mode = mode;
        }
        self.refresh_backend();
    }

    pub fn set_google_drive_credentials(
        &self,
        client_id: String,
        client_secret: Option<String>,
        folder_name: String,
    ) {
        if let Ok(mut cfg) = self.config.write() {
            let trimmed_id = client_id.trim().to_string();
            cfg.google_client_id = if trimmed_id.is_empty() {
                None
            } else {
                Some(trimmed_id)
            };
            cfg.google_client_secret = client_secret
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            let trimmed_folder = folder_name.trim().to_string();
            cfg.google_drive_folder_name = if trimmed_folder.is_empty() {
                "Vellum".to_string()
            } else {
                trimmed_folder
            };
        }
        self.refresh_backend();
    }

    pub fn connect_google_drive(&self) -> VellumResult<Option<String>> {
        let cfg = self.config();
        let client_id = cfg.google_client_id.unwrap_or_default();
        let tokens = gdrive_oauth::authorize_interactive(
            &client_id,
            cfg.google_client_secret.as_deref(),
            &self.tokens_path,
        )?;
        let email = tokens.account_email.clone();

        if let Ok(mut write_cfg) = self.config.write() {
            write_cfg.mode = CloudSyncMode::GoogleDriveApi;
            write_cfg.enabled = true;
            write_cfg.google_account_email = email.clone();
        }
        self.refresh_backend();
        Ok(email)
    }

    pub fn disconnect_google_drive(&self) -> VellumResult<()> {
        gdrive_oauth::delete_tokens(&self.tokens_path)?;
        if let Ok(mut cfg) = self.config.write() {
            cfg.google_account_email = None;
        }
        self.refresh_backend();
        Ok(())
    }

    pub fn set_sync_folder(&self, folder: PathBuf) -> VellumResult<()> {
        let backend = FolderSyncBackend::new(folder.clone());
        if let Ok(mut cfg) = self.config.write() {
            cfg.mode = CloudSyncMode::GoogleDriveFolder;
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

    /// Exécute la synchronisation bidirectionnelle complète.
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

        self.push_local_events(&**backend, &client_id, &mut report)?;
        self.pull_remote_deltas(&**backend, &client_id, auto_download, &mut report)?;

        if let Ok(mut cfg) = self.config.write() {
            cfg.last_synced_at = Some(report.timestamp.clone());
        }

        info!(
            "Sync terminé avec succès : +{} livres envoyés, +{} livres reçus, {} événements traités",
            report.uploaded_books, report.downloaded_books, report.pulled_events
        );

        Ok(report)
    }

    fn push_local_events(
        &self,
        backend: &dyn CloudSyncBackend,
        client_id: &str,
        report: &mut SyncReport,
    ) -> VellumResult<()> {
        let unsynced_events = self.sync_repo.get_unsynced_events()?;
        if unsynced_events.is_empty() {
            return Ok(());
        }

        info!(
            "Sync: {} événements locaux à pousser vers Google Drive",
            unsynced_events.len()
        );

        for ev in &unsynced_events {
            if let SyncAction::BookCreated { book, formats, .. } = &ev.action {
                for fmt in formats {
                    let local_file = self.library_path.join(&fmt.filepath);
                    if local_file.exists() && !backend.file_exists(&fmt.filepath)? {
                        backend.upload_file(&local_file, &fmt.filepath)?;
                        report.uploaded_books += 1;
                        info!("Sync: Upload du livre '{}' ({})", fmt.filename, fmt.format);
                    }
                }

                if let Some(ref cover_rel) = book.cover_path {
                    let local_cover = self.library_path.join(cover_rel);
                    if local_cover.exists() && !backend.file_exists(cover_rel)? {
                        let _ = backend.upload_file(&local_cover, cover_rel);
                    }
                }
            }
        }

        let delta = SyncDelta {
            client_id: client_id.to_string(),
            created_at: Utc::now().to_rfc3339(),
            events: unsynced_events.clone(),
        };

        let delta_filename = backend.write_delta(&delta)?;
        let event_ids: Vec<String> = unsynced_events.iter().map(|e| e.id.clone()).collect();
        self.sync_repo
            .mark_events_synced(&event_ids, &report.timestamp)?;
        self.sync_repo
            .mark_delta_applied(&delta_filename, client_id)?;

        report.pushed_events = unsynced_events.len();
        Ok(())
    }

    fn pull_remote_deltas(
        &self,
        backend: &dyn CloudSyncBackend,
        client_id: &str,
        auto_download: bool,
        report: &mut SyncReport,
    ) -> VellumResult<()> {
        let all_remote_deltas = backend.list_deltas()?;

        for delta_name in all_remote_deltas {
            if self.sync_repo.is_delta_applied(&delta_name)? {
                continue;
            }

            if delta_name.starts_with(client_id) {
                let _ = self.sync_repo.mark_delta_applied(&delta_name, client_id);
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
                self.apply_remote_action(backend, ev.action, auto_download, report)?;
                report.pulled_events += 1;
            }

            let _ = self
                .sync_repo
                .mark_delta_applied(&delta_name, &remote_delta.client_id);
        }
        Ok(())
    }

    fn apply_remote_action(
        &self,
        backend: &dyn CloudSyncBackend,
        action: SyncAction,
        auto_download: bool,
        report: &mut SyncReport,
    ) -> VellumResult<()> {
        match action {
            SyncAction::BookCreated {
                mut book,
                formats,
                tags,
            } => {
                if self.book_repo.find_by_id(&book.id)?.is_none() {
                    for tag in &tags {
                        let _ = self.tag_repo.save(tag);
                    }

                    if auto_download {
                        for fmt in &formats {
                            let target_local = self.library_path.join(&fmt.filepath);
                            if !target_local.exists()
                                && backend
                                    .download_file(&fmt.filepath, &target_local)
                                    .unwrap_or(false)
                            {
                                report.downloaded_books += 1;
                                info!("Sync: Téléchargé depuis Google Drive: {}", fmt.filename);
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
                info!(
                    "Sync: Suppression du livre {} répercutée localement",
                    book_id
                );
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
        Ok(())
    }
}
