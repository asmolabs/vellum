use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{info, warn};

use vellum_domain::{
    Book, DeviceAnnotation, DeviceBookInfo, DeviceKind, DeviceReadingProgress, DeviceSyncPlugin,
    VellumError, VellumResult,
};
use vellum_storage::BookRepository;

use crate::book_service::BookService;
use crate::plugin_service::PluginService;

#[derive(Debug, Clone, PartialEq)]
pub struct ConnectedDevice {
    pub kind: DeviceKind,
    pub name: String,
    pub mount_point: PathBuf,
    pub book_count: usize,
}

#[derive(Clone)]
pub struct DeviceService {
    book_service: Arc<BookService>,
    plugin_service: PluginService,
}

impl PartialEq for DeviceService {
    fn eq(&self, other: &Self) -> bool {
        self.book_service == other.book_service
    }
}

impl DeviceService {
    pub fn new(book_service: Arc<BookService>, plugin_service: PluginService) -> Self {
        Self {
            book_service,
            plugin_service,
        }
    }

    pub fn book_repo(&self) -> &BookRepository {
        self.book_service.book_repo()
    }

    /// Detects all currently mounted eReaders (Kobo, Kindle)
    pub fn detect_connected_devices(&self) -> Vec<ConnectedDevice> {
        let mut list = Vec::new();
        let plugins = self.plugin_service.get_device_plugins();

        for p in plugins {
            let custom_mount = self
                .plugin_service
                .get_plugin_config(p.id())
                .ok()
                .and_then(|cfg| {
                    cfg.get("customMountPath")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.trim().is_empty())
                        .map(PathBuf::from)
                });

            if let Some(mount) = p.detect_device(custom_mount.as_deref()) {
                list.push(ConnectedDevice {
                    kind: p.device_kind(),
                    name: p.name().to_string(),
                    mount_point: mount,
                    book_count: 0,
                });
            }
        }

        list
    }

    /// Finds the plugin responsible for a specific device kind
    pub fn get_plugin_for_kind(&self, kind: DeviceKind) -> Option<Arc<dyn DeviceSyncPlugin>> {
        self.plugin_service
            .get_device_plugins()
            .into_iter()
            .find(|p| p.device_kind() == kind)
    }

    /// Lists all books present on a given connected device
    pub fn list_device_books(
        &self,
        kind: DeviceKind,
        custom_mount: Option<&Path>,
    ) -> VellumResult<Vec<DeviceBookInfo>> {
        let plugin = self.get_plugin_for_kind(kind).ok_or_else(|| {
            VellumError::NotFound(format!("Aucun plugin trouvé pour l'appareil {:?}", kind))
        })?;

        let mount = plugin.detect_device(custom_mount).ok_or_else(|| {
            VellumError::NotFound(format!("L'appareil {:?} n'est pas connecté", kind))
        })?;

        plugin.list_books(&mount)
    }

    /// Exports a book from Vellum to an eReader (with auto-conversion to preferred format if needed)
    pub fn export_book_to_device(
        &self,
        book_id: &str,
        kind: DeviceKind,
        custom_mount: Option<&Path>,
    ) -> VellumResult<PathBuf> {
        let plugin = self.get_plugin_for_kind(kind).ok_or_else(|| {
            VellumError::NotFound(format!("Plugin introuvable pour l'appareil {:?}", kind))
        })?;

        let mount = plugin.detect_device(custom_mount).ok_or_else(|| {
            VellumError::NotFound(format!("L'appareil {:?} n'est pas connecté", kind))
        })?;

        let book = self
            .book_repo()
            .find_by_id(book_id)?
            .ok_or_else(|| VellumError::NotFound(format!("Livre introuvable: {book_id}")))?;

        let config = self
            .plugin_service
            .get_plugin_config(plugin.id())
            .unwrap_or_default();

        // Target format preference:
        // - Kobo: prefers EPUB or KEPUB
        // - Kindle: prefers AZW3, MOBI, or PDF
        let preferred_format = match kind {
            DeviceKind::Kobo => "epub",
            DeviceKind::Kindle => "azw3",
            DeviceKind::Generic => "epub",
        };

        // Check if we already have the preferred format or need conversion
        let mut source_file = None;

        // 1. Exact match
        if let Some(fmt) = book
            .formats
            .iter()
            .find(|f| f.format.eq_ignore_ascii_case(preferred_format))
        {
            source_file = Some(self.book_service.library_path().join(&fmt.filepath));
        }

        // 2. Kindle fallback: check for MOBI or PDF
        if source_file.is_none() && kind == DeviceKind::Kindle {
            if let Some(fmt) = book.formats.iter().find(|f| {
                f.format.eq_ignore_ascii_case("mobi") || f.format.eq_ignore_ascii_case("pdf")
            }) {
                source_file = Some(self.book_service.library_path().join(&fmt.filepath));
            }
        }

        // 3. If still none, convert on-the-fly using Vellum's pivot conversion pipeline
        let final_source_path = if let Some(path) = source_file {
            path
        } else {
            info!(
                "Conversion automatique du livre '{}' vers le format optimal pour {:?}",
                book.title, kind
            );
            let updated_book = self
                .book_service
                .convert_book_to_format(book_id, preferred_format)?;
            let fmt = updated_book
                .formats
                .iter()
                .find(|f| f.format.eq_ignore_ascii_case(preferred_format))
                .ok_or_else(|| {
                    VellumError::Unsupported(format!(
                        "Échec de la conversion vers le format {preferred_format}"
                    ))
                })?;

            self.book_service.library_path().join(&fmt.filepath)
        };

        plugin.export_book(&book, &final_source_path, &mount, &config)
    }

    /// Imports all new books found on the device into the Vellum library
    pub fn import_books_from_device(
        &self,
        kind: DeviceKind,
        custom_mount: Option<&Path>,
    ) -> VellumResult<Vec<Book>> {
        let plugin = self.get_plugin_for_kind(kind).ok_or_else(|| {
            VellumError::NotFound(format!("Plugin introuvable pour l'appareil {:?}", kind))
        })?;

        let mount = plugin.detect_device(custom_mount).ok_or_else(|| {
            VellumError::NotFound(format!("L'appareil {:?} n'est pas connecté", kind))
        })?;

        let device_books = plugin.list_books(&mount)?;
        let mut imported = Vec::new();

        for d_book in device_books {
            let full_device_file = if d_book.relative_path.is_absolute() {
                d_book.relative_path.clone()
            } else {
                mount.join(&d_book.relative_path)
            };

            if !full_device_file.exists() {
                continue;
            }

            // Check if already in library by title
            let all = self.book_repo().find_all().unwrap_or_default();
            let already_exists = all
                .iter()
                .any(|b| b.title.eq_ignore_ascii_case(&d_book.title));

            if !already_exists {
                info!("Importation du livre depuis {:?}: {}", kind, d_book.title);
                match self.book_service.import_book(&full_device_file) {
                    Ok(b) => imported.push(b),
                    Err(e) => {
                        warn!(
                            "Impossible d'importer {:?} depuis l'appareil: {e}",
                            full_device_file
                        );
                    }
                }
            }
        }

        Ok(imported)
    }

    /// Syncs reading progress between device and Vellum database
    pub fn sync_reading_progress(
        &self,
        kind: DeviceKind,
        custom_mount: Option<&Path>,
    ) -> VellumResult<Vec<DeviceReadingProgress>> {
        let plugin = self.get_plugin_for_kind(kind).ok_or_else(|| {
            VellumError::NotFound(format!("Plugin introuvable pour l'appareil {:?}", kind))
        })?;

        let mount = plugin.detect_device(custom_mount).ok_or_else(|| {
            VellumError::NotFound(format!("L'appareil {:?} n'est pas connecté", kind))
        })?;

        let states = plugin.get_reading_progress(&mount)?;
        let all_books = self.book_repo().find_all()?;

        for state in &states {
            for b in &all_books {
                if b.title.eq_ignore_ascii_case(&state.book_title)
                    && state.progress_percent > b.progress_percent
                {
                    let _ = self.book_service.update_reading_progress(
                        &b.id,
                        (state.progress_percent * 10.0) as i32,
                        1000,
                        state.progress_percent,
                        state.is_finished,
                    );
                }
            }
        }

        Ok(states)
    }

    /// Fetches all highlights, bookmarks and notes from the device
    pub fn get_annotations(
        &self,
        kind: DeviceKind,
        custom_mount: Option<&Path>,
    ) -> VellumResult<Vec<DeviceAnnotation>> {
        let plugin = self.get_plugin_for_kind(kind).ok_or_else(|| {
            VellumError::NotFound(format!("Plugin introuvable pour l'appareil {:?}", kind))
        })?;

        let mount = plugin.detect_device(custom_mount).ok_or_else(|| {
            VellumError::NotFound(format!("L'appareil {:?} n'est pas connecté", kind))
        })?;

        plugin.get_annotations(&mount)
    }

    /// Compares books on a connected device with the Vellum library.
    /// Categorizes books into:
    /// - present on device (matched)
    /// - in Vellum but not yet on device (candidates to transfer)
    /// - on device but not in Vellum (candidates to import)
    pub fn compare_device(
        &self,
        kind: DeviceKind,
        custom_mount: Option<&Path>,
    ) -> VellumResult<DeviceComparison> {
        let plugin = self.get_plugin_for_kind(kind).ok_or_else(|| {
            VellumError::NotFound(format!("Plugin introuvable pour l'appareil {:?}", kind))
        })?;

        let mount = plugin.detect_device(custom_mount).ok_or_else(|| {
            VellumError::NotFound(format!("L'appareil {:?} n'est pas connecté", kind))
        })?;

        let device_books = plugin.list_books(&mount)?;
        let all_books = self.book_repo().find_all()?;

        let mut matched_device_books = Vec::new();
        let mut in_vellum_not_on_device = Vec::new();
        let mut matched_device_indices = std::collections::HashSet::new();

        for book in all_books {
            let norm_title = normalize_for_match(&book.title);
            let mut found = false;

            for (idx, d_book) in device_books.iter().enumerate() {
                let norm_d_title = normalize_for_match(&d_book.title);
                let norm_d_stem = normalize_for_match(
                    Path::new(&d_book.filename)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or(""),
                );

                if !norm_title.is_empty()
                    && (norm_title == norm_d_title
                        || norm_d_stem.contains(&norm_title)
                        || norm_title.contains(&norm_d_title))
                {
                    matched_device_indices.insert(idx);
                    found = true;
                    break;
                }
            }

            if found {
                // Book is already on device
            } else {
                in_vellum_not_on_device.push(book);
            }
        }

        let mut on_device_not_in_vellum = Vec::new();
        for (idx, d_book) in device_books.into_iter().enumerate() {
            if matched_device_indices.contains(&idx) {
                matched_device_books.push(d_book);
            } else {
                on_device_not_in_vellum.push(d_book);
            }
        }

        let connected_device = ConnectedDevice {
            kind,
            name: plugin.name().to_string(),
            mount_point: mount,
            book_count: matched_device_books.len() + on_device_not_in_vellum.len(),
        };

        Ok(DeviceComparison {
            device: connected_device,
            present_on_device: matched_device_books,
            in_vellum_not_on_device,
            on_device_not_in_vellum,
        })
    }

    /// Batch exports books from Vellum to a connected device.
    pub fn export_books_batch(
        &self,
        book_ids: &[String],
        kind: DeviceKind,
        custom_mount: Option<&Path>,
    ) -> VellumResult<usize> {
        let mut count = 0;
        for id in book_ids {
            match self.export_book_to_device(id, kind, custom_mount) {
                Ok(_) => count += 1,
                Err(e) => {
                    warn!("Échec de l'export du livre {id} vers {:?}: {e}", kind);
                }
            }
        }
        Ok(count)
    }

    /// Batch imports selected books from the device into Vellum.
    pub fn import_device_books_batch(
        &self,
        relative_paths: &[PathBuf],
        kind: DeviceKind,
        custom_mount: Option<&Path>,
    ) -> VellumResult<Vec<Book>> {
        let plugin = self.get_plugin_for_kind(kind).ok_or_else(|| {
            VellumError::NotFound(format!("Plugin introuvable pour l'appareil {:?}", kind))
        })?;

        let mount = plugin.detect_device(custom_mount).ok_or_else(|| {
            VellumError::NotFound(format!("L'appareil {:?} n'est pas connecté", kind))
        })?;

        let mut imported = Vec::new();
        for rel in relative_paths {
            let full_device_file = if rel.is_absolute() {
                rel.clone()
            } else {
                mount.join(rel)
            };

            if full_device_file.exists() {
                match self.book_service.import_book(&full_device_file) {
                    Ok(b) => imported.push(b),
                    Err(e) => {
                        warn!("Erreur lors de l'import de {:?}: {e}", full_device_file);
                    }
                }
            }
        }

        Ok(imported)
    }
}

fn normalize_for_match(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeviceComparison {
    pub device: ConnectedDevice,
    pub present_on_device: Vec<DeviceBookInfo>,
    pub in_vellum_not_on_device: Vec<Book>,
    pub on_device_not_in_vellum: Vec<DeviceBookInfo>,
}
