//! # `vellum-ui`
//!
//! Interface desktop native de Vellum basée sur Dioxus.
//!
//! Initialise les services applicatifs (`vellum-core`), démarre le serveur OPDS/Web Companion
//! en tâche de fond et rend l'interface réactive (bibliothèque grille/liste/Kanban, lecteur intégré,
//! éditeur EPUB, palette de commandes, statistiques et modales de configuration).

use std::path::PathBuf;
use std::sync::Arc;

use dioxus::desktop::{Config, WindowBuilder};
use dioxus::prelude::*;
use tracing::info;

mod components;
mod state;
mod styles;

use components::{
    BookCard, BulkEditModal, CommandPalette, DetailDrawer, EpubEditorModal, Header,
    KanbanBoard, KanbanTargetStatus, LibraryAuditModal, NewsRecipesModal, PaletteAction,
    ReaderModal, ReadingStatsModal, SettingsModal, Sidebar, TranslateModal, TranslationRequest,
};
use state::{FilterStatus, ReaderSession, SortOption, ViewMode};
use styles::APP_CSS;

use vellum_core::book_service::BookService;
use vellum_core::device_service::DeviceService;
use vellum_core::litestream_service::LitestreamService;
use vellum_core::mail_service::MailService;
use vellum_core::news_service::NewsService;
use vellum_core::{init_services, AppServices, FolderSyncBackend, SearchQuery, SyncService};
use vellum_domain::{Book, DeviceKind, SmtpConfig};

fn main() {
    tracing_subscriber::fmt::init();

    let cfg = Config::new().with_window(
        WindowBuilder::new()
            .with_title("Vellum — Bibliothèque Personnelle & Liseuse")
            .with_inner_size(dioxus::desktop::LogicalSize::new(1200.0, 800.0))
            .with_min_inner_size(dioxus::desktop::LogicalSize::new(960.0, 640.0)),
    );

    dioxus::LaunchBuilder::desktop().with_cfg(cfg).launch(App);
}

#[component]
fn App() -> Element {
    let services = use_hook(init_services);
    let book_service = services.book_service.clone();
    let sync_service = services.sync_service.clone();
    let litestream = services.litestream_service.clone();
    let opds_port = services.opds_port;
    let device_service = services.device_service.clone();
    let mut connected_devices = use_signal(|| device_service.detect_connected_devices());

    let mut sync_config = use_signal(|| sync_service.config());
    let mut is_syncing = use_signal(|| false);
    let mut sync_message = use_signal(|| None::<String>);

    let mut books = use_signal(|| {
        book_service
            .book_repo()
            .find_all()
            .unwrap_or_default()
    });


    let current_filter = use_signal(|| FilterStatus::All);
    let mut current_sort = use_signal(|| SortOption::RecentAdded);
    let mut view_mode = use_signal(|| ViewMode::Grid);
    let mut selected_tag = use_signal(|| None::<String>);
    let mut selected_series = use_signal(|| None::<String>);
    let search_query = use_signal(String::new);
    let mut selected_book = use_signal(|| None::<Book>);
    let mut active_reader = use_signal(|| None::<ReaderSession>);
    let mut settings_open = use_signal(|| false);
    let mut palette_open = use_signal(|| false);
    let mut bulk_edit_open = use_signal(|| false);
    let mut selected_ids = use_signal(std::collections::HashSet::<String>::new);
    let mut translating_book = use_signal(|| None::<Book>);
    let mut is_translating = use_signal(|| false);
    let mut epub_editing_book = use_signal(|| None::<Book>);
    let mut news_recipes_open = use_signal(|| false);
    let mut audit_open = use_signal(|| false);
    let mut stats_open = use_signal(|| false);

    let news_service = services.news_service.clone();
    let mail_service = services.mail_service.clone();
    let mut smtp_config = use_signal(|| services.smtp_config.clone());
    let mut is_testing_smtp = use_signal(|| false);
    let mut smtp_test_message = use_signal(|| None::<(bool, String)>);
    let mut is_sending_kindle = use_signal(|| false);

    // Refresh function helper
    let refresh_library = {
        let bs = book_service.clone();
        move || {
            if let Ok(all) = bs.book_repo().find_all() {
                books.set(all);
            }
        }
    };


    // Calculate tag counts
    let tags_counts: Vec<(String, usize)> = {
        let all_books = books.read();
        let mut map = std::collections::HashMap::new();
        for b in all_books.iter() {
            for t in &b.tags {
                *map.entry(t.name.clone()).or_insert(0) += 1;
            }
        }
        let mut list: Vec<_> = map.into_iter().collect();
        list.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        list
    };

    // Calculate series counts
    let series_counts: Vec<(String, usize)> = {
        let all_books = books.read();
        let mut map = std::collections::HashMap::new();
        for b in all_books.iter() {
            if let Some(ref s) = b.series {
                let trimmed = s.trim();
                if !trimmed.is_empty() {
                    *map.entry(trimmed.to_string()).or_insert(0) += 1;
                }
            }
        }
        let mut list: Vec<_> = map.into_iter().collect();
        list.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        list
    };
    let series_names: Vec<String> = series_counts.iter().map(|(n, _)| n.clone()).collect();

    // Reading stats
    let reading_stats = book_service.get_reading_stats().ok();

    // Filter and sort books
    let filtered_books: Vec<Book> = {
        let all = books.read();
        let filter = *current_filter.read();
        let sort = *current_sort.read();
        let tag_filter = selected_tag.read().clone();
        let series_filter = selected_series.read().clone();
        let raw_query = search_query.read();
        let parsed_query = SearchQuery::parse(&raw_query);

        let mut list: Vec<Book> = all
            .iter()
            .filter(|b| {
                // Status Filter
                let matches_status = match filter {
                    FilterStatus::All => true,
                    FilterStatus::Reading => b.progress_percent > 0.0 && !b.is_finished,
                    FilterStatus::Finished => b.is_finished,
                    FilterStatus::ToRead => b.progress_percent == 0.0 && !b.is_finished,
                };
                if !matches_status {
                    return false;
                }

                // Tag Filter
                if let Some(ref t) = tag_filter {
                    if !b.tags.iter().any(|bt| &bt.name == t) {
                        return false;
                    }
                }

                // Series Filter
                if let Some(ref s) = series_filter {
                    if b.series.as_deref().map(str::trim) != Some(s.as_str()) {
                        return false;
                    }
                }

                // Calibre Advanced Query Search (author:, series:, tag:, format:, not, quotes...)
                if !parsed_query.is_empty() && !parsed_query.matches(b) {
                    return false;
                }

                true
            })
            .cloned()
            .collect();

        match sort {
            SortOption::TitleAsc => list.sort_by_key(|a| a.title.to_lowercase()),
            SortOption::TitleDesc => list.sort_by(|a, b| b.title.to_lowercase().cmp(&a.title.to_lowercase())),
            SortOption::AuthorAsc => list.sort_by(|a, b| a.author.to_lowercase().cmp(&b.author.to_lowercase()).then_with(|| a.title.cmp(&b.title))),
            SortOption::RecentAdded => list.sort_by(|a, b| b.added_at.cmp(&a.added_at)),
            SortOption::ProgressDesc => list.sort_by(|a, b| b.progress_percent.partial_cmp(&a.progress_percent).unwrap_or(std::cmp::Ordering::Equal)),
        }

        list
    };

    // Handle book file import via native file picker
    let handle_import = {
        let bs = book_service.clone();
        let mut refresh = refresh_library.clone();
        move |()| {
            let files = rfd::FileDialog::new()
                .set_title("Importer des livres ou bandes dessinées dans Vellum")
                .add_filter(
                    "Fichiers pris en charge",
                    &["epub", "pdf", "cbz", "cbr", "mobi", "azw3", "txt", "mp3", "m4b"],
                )
                .pick_files();

            if let Some(paths) = files {
                for path in paths {
                    tracing::info!("Importing file: {:?}", path);
                    match bs.import_book(&path) {
                        Ok(book) => {
                            tracing::info!("Livre importé avec succès: {} par {}", book.title, book.author);
                        }
                        Err(e) => {
                            tracing::error!("Erreur lors de l'import de {:?}: {}", path, e);
                        }
                    }
                }
                refresh();
            }
        }
    };

    // Handle Calibre library import via native folder/file picker
    let handle_import_calibre = {
        let bs = book_service.clone();
        let refresh = refresh_library.clone();
        move |()| {
            let folder = rfd::FileDialog::new()
                .set_title("Sélectionner le dossier de votre bibliothèque Calibre (ou fichier metadata.db)")
                .pick_folder();

            let target_path = if let Some(f) = folder {
                Some(f)
            } else {
                rfd::FileDialog::new()
                    .set_title("Sélectionner le fichier metadata.db de Calibre")
                    .add_filter("Base de données Calibre", &["db"])
                    .pick_file()
            };

            if let Some(path) = target_path {
                tracing::info!("Importation de la bibliothèque Calibre depuis: {:?}", path);
                let bs_cloned = bs.clone();
                let mut refresh_cloned = refresh.clone();
                spawn(async move {
                    let path_cloned = path.clone();
                    let res = tokio::task::spawn_blocking(move || {
                        bs_cloned.import_calibre_library(&path_cloned, true)
                    })
                    .await;

                    match res {
                        Ok(Ok(report)) => {
                            tracing::info!(
                                "Migration Calibre terminée avec succès : {} livres importés, {} formats, {} séries, {} tags",
                                report.books_imported,
                                report.formats_imported,
                                report.series_count,
                                report.tags_count
                            );
                            refresh_cloned();
                        }
                        Ok(Err(e)) => {
                            tracing::error!("Erreur lors de l'import Calibre: {e}");
                        }
                        Err(e) => {
                            tracing::error!("Tâche de fond interrompue: {e}");
                        }
                    }
                });
            }
        }
    };

    // Handle read book launcher
    let handle_read = {
        let bs = book_service.clone();
        move |book: Book| {
            if let Some(first_fmt) = book.formats.first() {
                let abs_path = bs.library_path().join(&first_fmt.filepath);
                if abs_path.exists() {
                    let session = ReaderSession::new(book, abs_path);
                    active_reader.set(Some(session));
                    selected_book.set(None);
                }
            }
        }
    };

    // Handle progress update from reader
    let handle_progress = {
        let bs = book_service.clone();
        move |(page, total, pct): (usize, usize, f64)| {
            if let Some(ref sess) = *active_reader.read() {
                let book_id = &sess.book.id;
                let finished = page + 1 >= total && total > 0;
                let _ = bs.update_reading_progress(
                    book_id,
                    page as i32,
                    total as i32,
                    pct,
                    finished,
                );
            }
        }
    };

    // Handle book delete
    let handle_delete = {
        let bs = book_service.clone();
        let mut refresh = refresh_library.clone();
        move |id: String| {
            let _ = bs.delete_book(&id, true);
            selected_book.set(None);
            refresh();
        }
    };

    // Handle toggle book selection
    let handle_toggle_select = {
        move |id: String| {
            let mut set = selected_ids.read().clone();
            if set.contains(&id) {
                set.remove(&id);
            } else {
                set.insert(id);
            }
            selected_ids.set(set);
        }
    };

    // Handle bulk edit save
    let handle_bulk_edit_save = {
        let bs = book_service.clone();
        let mut refresh = refresh_library.clone();
        move |(author, series, auto_index, add_tags, remove_tags): (
            Option<String>,
            Option<String>,
            bool,
            Vec<String>,
            Vec<String>,
        )| {
            let set = selected_ids.read().clone();
            let all = books.read();
            let ordered_ids: Vec<String> = all
                .iter()
                .filter(|b| set.contains(&b.id))
                .map(|b| b.id.clone())
                .collect();

            if !ordered_ids.is_empty() {
                let _ = bs.bulk_update_books(
                    &ordered_ids,
                    author.as_deref(),
                    series.as_deref(),
                    auto_index,
                    &add_tags,
                    &remove_tags,
                );
                refresh();
                selected_ids.set(std::collections::HashSet::new());
            }
        }
    };

    // Handle bulk delete
    let handle_bulk_delete = {
        let bs = book_service.clone();
        let mut refresh = refresh_library.clone();
        move |()| {
            let set = selected_ids.read().clone();
            let ids: Vec<String> = set.into_iter().collect();
            if !ids.is_empty() {
                let _ = bs.bulk_delete_books(&ids, true);
                selected_ids.set(std::collections::HashSet::new());
                selected_book.set(None);
                refresh();
            }
        }
    };

    // Handle Kanban status transition
    let handle_change_book_status = {
        let bs = book_service.clone();
        let mut refresh = refresh_library.clone();
        move |(book_id, target_status): (String, KanbanTargetStatus)| {
            if let Ok(Some(mut b)) = bs.book_repo().find_by_id(&book_id) {
                match target_status {
                    KanbanTargetStatus::ToRead => {
                        b.progress_percent = 0.0;
                        b.current_chapter_index = 0;
                        b.is_finished = false;
                    }
                    KanbanTargetStatus::Reading => {
                        if b.progress_percent == 0.0 {
                            b.progress_percent = 5.0;
                        }
                        b.is_finished = false;
                        b.last_read_at = Some(chrono::Utc::now().to_rfc3339());
                    }
                    KanbanTargetStatus::Finished => {
                        b.progress_percent = 100.0;
                        b.is_finished = true;
                        b.last_read_at = Some(chrono::Utc::now().to_rfc3339());
                    }
                    KanbanTargetStatus::Favorite => {
                        b.rating = Some(5);
                        let fav_tag = vellum_domain::Tag::new("Coup de cœur");
                        let _ = bs.tag_repo().save(&fav_tag);
                        if !b.tags.iter().any(|t| t.name == "Coup de cœur") {
                            b.add_tag(fav_tag);
                        }
                    }
                }
                let _ = bs.update_book(&b);
                refresh();
            }
        }
    };

    // Handle book format conversion via pivot pipeline
    let handle_convert = {
        let bs = book_service.clone();
        let mut refresh = refresh_library.clone();
        move |(b_id, target_fmt): (String, String)| {
            match bs.convert_book_to_format(&b_id, &target_fmt) {
                Ok(updated) => {
                    tracing::info!("Conversion réussie vers le format {target_fmt}");
                    selected_book.set(Some(updated));
                    refresh();
                }
                Err(e) => {
                    tracing::error!("Erreur lors de la conversion en {target_fmt}: {e}");
                }
            }
        }
    };

    // Handle book metadata save
    let handle_save_metadata = {
        let bs = book_service.clone();
        let mut refresh = refresh_library.clone();
        move |(b_id, title, author, publ, desc, isbn, pdate, series, series_idx, tags): (
            String,
            String,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<f64>,
            Vec<String>,
        )| {
            match bs.update_book_metadata_full(vellum_core::UpdateMetadataRequest {
                book_id: &b_id,
                title: &title,
                author: &author,
                publisher: publ.as_deref(),
                description: desc.as_deref(),
                isbn: isbn.as_deref(),
                publish_date: pdate.as_deref(),
                series: series.as_deref(),
                series_index: series_idx,
                tag_names: tags,
            }) {
                Ok(updated) => {
                    tracing::info!("Métadonnées mises à jour avec succès pour: {}", updated.title);
                    selected_book.set(Some(updated));
                    refresh();
                }
                Err(e) => {
                    tracing::error!("Erreur lors de la mise à jour des métadonnées: {e}");
                }
            }
        }
    };

    // Handle book automatic enrichment
    let handle_enrich_metadata = {
        let bs = book_service.clone();
        let mut refresh = refresh_library.clone();
        move |b_id: String| {
            match bs.enrich_book(&b_id) {
                Ok(updated) => {
                    tracing::info!("Métadonnées enrichies avec succès pour: {}", updated.title);
                    selected_book.set(Some(updated));
                    refresh();
                }
                Err(e) => {
                    tracing::error!("Erreur lors de l'enrichissement des métadonnées: {e}");
                }
            }
        }
    };

    // Handle cover replacement
    let handle_change_cover = {
        let bs = book_service.clone();
        let mut refresh = refresh_library.clone();
        move |(b_id, path): (String, PathBuf)| {
            match bs.update_book_cover(&b_id, &path) {
                Ok(updated) => {
                    tracing::info!("Couverture mise à jour avec succès pour: {}", updated.title);
                    selected_book.set(Some(updated));
                    refresh();
                }
                Err(e) => {
                    tracing::error!("Erreur lors du changement de couverture: {e}");
                }
            }
        }
    };

    // Handle AI translation launch
    let handle_start_translate = {
        let bs = book_service.clone();
        let refresh = refresh_library.clone();
        move |req: TranslationRequest| {
            is_translating.set(true);
            let bs_cloned = bs.clone();
            let mut refresh_cloned = refresh.clone();
            dioxus::prelude::spawn(async move {
                let res = tokio::task::spawn_blocking(move || {
                    bs_cloned.translate_book(vellum_core::TranslateBookRequest {
                        book_id: &req.book_id,
                        target_lang: &req.target_lang,
                        provider: &req.provider,
                        model: &req.model,
                        base_url: req.base_url.as_deref(),
                        api_key: req.api_key.as_deref(),
                        bilingual_mode: req.bilingual_mode,
                    })
                })
                .await;

                is_translating.set(false);
                match res {
                    Ok(Ok(translated_book)) => {
                        tracing::info!("Livre traduit avec succès: {}", translated_book.title);
                        translating_book.set(None);
                        selected_book.set(Some(translated_book));
                        refresh_cloned();
                    }
                    Ok(Err(e)) => {
                        tracing::error!("Erreur lors de la traduction: {e}");
                    }
                    Err(e) => {
                        tracing::error!("Erreur de tâche en arrière-plan: {e}");
                    }
                }
            });
        }
    };


    // Handle eReader export
    let handle_export_to_device = {
        let ds = device_service.clone();
        move |(b_id, kind): (String, DeviceKind)| {
            match ds.export_book_to_device(&b_id, kind, None) {
                Ok(dest) => {
                    tracing::info!("Livre exporté avec succès vers {:?}: {:?}", kind, dest);
                }
                Err(e) => {
                    tracing::error!("Erreur lors de l'exportation vers {:?}: {e}", kind);
                }
            }
        }
    };

    // Handle book file export to directory via template
    let handle_export_file = {
        let bs = book_service.clone();
        move |(b_id, pref_fmt): (String, Option<String>)| {
            let folder = rfd::FileDialog::new()
                .set_title("Choisir le dossier de destination pour l'export")
                .pick_folder();

            if let Some(dest_dir) = folder {
                let bs_cloned = bs.clone();
                spawn(async move {
                    let res = tokio::task::spawn_blocking(move || {
                        bs_cloned.export_book_to_directory(&b_id, &dest_dir, pref_fmt.as_deref(), None)
                    })
                    .await;

                    match res {
                        Ok(Ok(out_path)) => {
                            tracing::info!("Livre exporté avec succès dans: {:?}", out_path);
                        }
                        Ok(Err(e)) => {
                            tracing::error!("Erreur lors de l'export: {e}");
                        }
                        Err(e) => {
                            tracing::error!("Tâche de fond interrompue: {e}");
                        }
                    }
                });
            }
        }
    };

    // Handle eReader sync (import new books & sync reading progress)
    let handle_sync_device = {
        let ds = device_service.clone();
        let mut refresh = refresh_library.clone();
        move |kind: DeviceKind| {
            let _ = ds.sync_reading_progress(kind, None);
            let _ = ds.import_books_from_device(kind, None);
            refresh();
            connected_devices.set(ds.detect_connected_devices());
        }
    };

    // Handle manual cloud sync trigger
    let handle_sync_now = {
        let ss = sync_service.clone();
        let pr = services.book_service.param_repo().clone();
        let refresh = refresh_library.clone();
        move |()| {
            if *is_syncing.read() {
                return;
            }
            is_syncing.set(true);
            sync_message.set(Some("Synchronisation avec Google Drive en cours...".to_string()));
            let ss_cloned = ss.clone();
            let pr_cloned = pr.clone();
            let mut refresh_cloned = refresh.clone();
            dioxus::prelude::spawn(async move {
                let res = tokio::task::spawn_blocking(move || {
                    let r = ss_cloned.sync_now();
                    let cfg = ss_cloned.config();
                    if let Ok(cfg_json) = serde_json::to_string(&cfg) {
                        let _ = pr_cloned.set_value("cloud_sync_config", &cfg_json);
                    }
                    (r, cfg)
                })
                .await;

                is_syncing.set(false);
                match res {
                    Ok((Ok(report), updated_cfg)) => {
                        sync_config.set(updated_cfg);
                        let msg = format!(
                            "Synchro réussie : {} envoyé(s), {} reçu(s), {} événement(s)",
                            report.uploaded_books, report.downloaded_books, report.pulled_events
                        );
                        tracing::info!("{msg}");
                        sync_message.set(Some(msg));
                        refresh_cloned();
                    }
                    Ok((Err(e), _)) => {
                        let err_msg = format!("Erreur de synchronisation : {e}");
                        tracing::error!("{err_msg}");
                        sync_message.set(Some(err_msg));
                    }
                    Err(e) => {
                        tracing::error!("Erreur de tâche asynchrone : {e}");
                        sync_message.set(Some("Erreur inattendue lors de la synchro".to_string()));
                    }
                }
            });
        }
    };

    // Handle picking Google Drive folder via file dialog
    let handle_pick_sync_folder = {
        let ss = sync_service.clone();
        let pr = services.book_service.param_repo().clone();
        move |()| {
            if let Some(folder) = rfd::FileDialog::new()
                .set_title("Sélectionner le dossier de synchronisation Google Drive")
                .pick_folder()
            {
                if let Ok(()) = ss.set_sync_folder(folder) {
                    let cfg = ss.config();
                    if let Ok(cfg_json) = serde_json::to_string(&cfg) {
                        let _ = pr.set_value("cloud_sync_config", &cfg_json);
                    }
                    sync_config.set(cfg);
                    sync_message.set(Some("Dossier Google Drive configuré avec succès !".to_string()));
                }
            }
        }
    };

    // Handle auto-detecting Google Drive folder
    let handle_auto_detect_folder = {
        let ss = sync_service.clone();
        let pr = services.book_service.param_repo().clone();
        move |()| {
            if let Some(folder) = FolderSyncBackend::auto_detect_google_drive() {
                if let Ok(()) = ss.set_sync_folder(folder) {
                    let cfg = ss.config();
                    if let Ok(cfg_json) = serde_json::to_string(&cfg) {
                        let _ = pr.set_value("cloud_sync_config", &cfg_json);
                    }
                    sync_config.set(cfg);
                    sync_message.set(Some("Dossier Google Drive détecté et configuré !".to_string()));
                }
            } else {
                sync_message.set(Some("Google Drive n'a pas été détecté automatiquement dans les chemins standards.".to_string()));
            }
        }
    };

    // Handle toggle auto-download books
    let handle_toggle_auto_download = {
        let ss = sync_service.clone();
        let pr = services.book_service.param_repo().clone();
        move |auto_dl: bool| {
            ss.set_auto_download_books(auto_dl);
            let cfg = ss.config();
            if let Ok(cfg_json) = serde_json::to_string(&cfg) {
                let _ = pr.set_value("cloud_sync_config", &cfg_json);
            }
            sync_config.set(cfg);
        }
    };

    // Handle toggle cloud sync enabled
    let handle_toggle_sync_enabled = {
        let ss = sync_service.clone();
        let pr = services.book_service.param_repo().clone();
        move |enabled: bool| {
            ss.set_enabled(enabled);
            let cfg = ss.config();
            if let Ok(cfg_json) = serde_json::to_string(&cfg) {
                let _ = pr.set_value("cloud_sync_config", &cfg_json);
            }
            sync_config.set(cfg);
        }
    };

    // Handle save SMTP configuration
    let handle_save_smtp = {
        let pr = services.book_service.param_repo().clone();
        move |cfg: SmtpConfig| {
            smtp_config.set(cfg.clone());
            if let Ok(cfg_json) = serde_json::to_string(&cfg) {
                let _ = pr.set_value("smtp_config", &cfg_json);
            }
        }
    };

    // Handle test SMTP connection
    let handle_test_smtp = {
        let ms = mail_service.clone();
        move |cfg: SmtpConfig| {
            is_testing_smtp.set(true);
            smtp_test_message.set(None);
            let ms_cloned = ms.clone();
            spawn(async move {
                let res = ms_cloned.test_connection(&cfg).await;
                is_testing_smtp.set(false);
                match res {
                    Ok(()) => smtp_test_message.set(Some((true, "Connexion SMTP établie et authentifiée avec succès !".into()))),
                    Err(e) => smtp_test_message.set(Some((false, format!("Échec du test SMTP : {e}")))),
                }
            });
        }
    };

    // Handle send book to Kindle
    let handle_send_to_kindle = {
        let ms = mail_service.clone();
        move |b: Book| {
            let cfg = smtp_config.read().clone();
            is_sending_kindle.set(true);
            let ms_cloned = ms.clone();
            spawn(async move {
                let res = ms_cloned.send_to_kindle(&b, &cfg, None).await;
                is_sending_kindle.set(false);
                match res {
                    Ok(()) => {
                        tracing::info!("Livre '{}' envoyé avec succès à la Kindle", b.title);
                    }
                    Err(e) => {
                        tracing::error!("Erreur lors de l'envoi vers Kindle: {e}");
                    }
                }
            });
        }
    };

    // Global keyboard shortcut Cmd + K (or Ctrl + K) for Command Palette
    use_effect(move || {
        let mut eval = document::eval(r#"
            window.addEventListener('keydown', (e) => {
                if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
                    e.preventDefault();
                    dioxus.send('toggle_palette');
                }
            });
        "#);
        spawn(async move {
            while let Ok(_) = eval.recv::<String>().await {
                let cur = *palette_open.peek();
                palette_open.set(!cur);
            }
        });
    });

    let handle_palette_action = {
        let mut cf = current_filter;
        let mut sb = selected_book;
        let mut st = selected_tag;
        let mut ss = selected_series;
        let mut hr = handle_read.clone();
        let mut hi = handle_import.clone();
        let hic = handle_import_calibre.clone();
        let mut hsc = handle_sync_now.clone();
        let mut so = settings_open;
        let mut nro = news_recipes_open;
        let mut ao = audit_open;
        let mut sto = stats_open;
        let mut vm = view_mode;
        move |act: PaletteAction| {
            match act {
                PaletteAction::SelectBook(b) => sb.set(Some(b)),
                PaletteAction::ReadBook(b) => hr(b),
                PaletteAction::SetViewGrid => vm.set(ViewMode::Grid),
                PaletteAction::SetViewKanban => vm.set(ViewMode::Kanban),
                PaletteAction::FilterAll => {
                    cf.set(FilterStatus::All);
                    st.set(None);
                    ss.set(None);
                }
                PaletteAction::FilterReading => {
                    cf.set(FilterStatus::Reading);
                    st.set(None);
                    ss.set(None);
                }
                PaletteAction::FilterFinished => {
                    cf.set(FilterStatus::Finished);
                    st.set(None);
                    ss.set(None);
                }
                PaletteAction::FilterToRead => {
                    cf.set(FilterStatus::ToRead);
                    st.set(None);
                    ss.set(None);
                }
                PaletteAction::ImportBooks => hi(()),
                PaletteAction::ImportCalibre => hic(()),
                PaletteAction::OpenNewsRecipes => nro.set(true),
                PaletteAction::OpenLibraryAudit => ao.set(true),
                PaletteAction::OpenReadingStats => sto.set(true),
                PaletteAction::OpenWebCompanion => {
                    let _ = open::that(format!("http://localhost:{opds_port}"));
                }
                PaletteAction::SyncCloud => hsc(()),
                PaletteAction::OpenSettings => so.set(true),
            }
        }
    };

    let handle_open_web_companion = {
        let port = opds_port;
        move |_| {
            let _ = open::that(format!("http://localhost:{port}"));
        }
    };

    let litestream_status_str = match litestream.status() {
        vellum_core::litestream_service::LitestreamStatus::Running => "En cours de synchronisation",
        vellum_core::litestream_service::LitestreamStatus::Stopped => "Arrêté (en attente)",
        vellum_core::litestream_service::LitestreamStatus::Disabled => "Désactivé",
        vellum_core::litestream_service::LitestreamStatus::BinaryNotFound => "Exécutable introuvable",
        vellum_core::litestream_service::LitestreamStatus::Error => "Erreur de processus",
    }
    .to_string();

    let plugin_descriptors = book_service.plugin_service().list_descriptors();

    rsx! {
        style { "{APP_CSS}" }

        div { class: "app-container",
            // Header
            Header {
                search_query: search_query,
                connected_devices: connected_devices.read().clone(),
                sync_available: sync_service.is_backend_available(),
                is_syncing: *is_syncing.read(),
                on_import: handle_import.clone(),
                on_import_calibre: handle_import_calibre.clone(),
                on_open_audit: move |_| audit_open.set(true),
                on_open_news: move |_| news_recipes_open.set(true),
                on_open_web_companion: handle_open_web_companion,
                on_sync_device: handle_sync_device.clone(),
                on_sync_cloud: handle_sync_now.clone(),
                on_open_palette: move |_| palette_open.set(true),
                on_open_settings: move |_| settings_open.set(true),
            }

            // Main Layout
            div { class: "main-content",
                // Sidebar
                Sidebar {
                    current_filter: current_filter,
                    selected_tag: selected_tag,
                    selected_series: selected_series,
                    tags_counts: tags_counts,
                    series_counts: series_counts,
                    stats: reading_stats.clone(),
                    on_open_stats: move |_| stats_open.set(true),
                }

                // Books Library
                main { class: "library-area",
                    div { class: "library-header",
                        div { class: "library-title-group",
                            div { style: "display: flex; align-items: center; gap: 10px;",
                                h2 { "{current_filter.read().label()}" }
                                if let Some(ref s) = *selected_series.read() {
                                    div {
                                        style: "display: flex; align-items: center; gap: 6px; padding: 2px 8px; border-radius: 6px; background: rgba(167, 139, 250, 0.2); border: 1px solid rgba(167, 139, 250, 0.4); font-size: 12px; color: #c4b5fd;",
                                        span { "📚 {s}" }
                                        button {
                                            style: "background: none; border: none; color: inherit; cursor: pointer; padding: 0 2px;",
                                            onclick: move |_| selected_series.set(None),
                                            "✕"
                                        }
                                    }
                                }
                                if let Some(ref t) = *selected_tag.read() {
                                    div {
                                        style: "display: flex; align-items: center; gap: 6px; padding: 2px 8px; border-radius: 6px; background: rgba(99, 102, 241, 0.2); border: 1px solid rgba(99, 102, 241, 0.4); font-size: 12px; color: #a5b4fc;",
                                        span { "🏷️ {t}" }
                                        button {
                                            style: "background: none; border: none; color: inherit; cursor: pointer; padding: 0 2px;",
                                            onclick: move |_| selected_tag.set(None),
                                            "✕"
                                        }
                                    }
                                }
                            }
                            div { class: "library-count",
                                "{filtered_books.len()} livre(s) affiché(s)"
                            }
                        }

                        div { style: "display: flex; align-items: center; gap: 8px;",
                            // View Mode Toggle (Grid vs Kanban PAL)
                            div { style: "display: flex; align-items: center; background: rgba(255,255,255,0.06); border-radius: 8px; padding: 2px; border: 1px solid var(--border-subtle); margin-right: 4px;",
                                button {
                                    style: if *view_mode.read() == ViewMode::Grid {
                                        "padding: 4px 10px; font-size: 12px; border-radius: 6px; background: #6366f1; color: #fff; border: none; font-weight: 600; cursor: pointer;"
                                    } else {
                                        "padding: 4px 10px; font-size: 12px; border-radius: 6px; background: transparent; color: var(--text-secondary); border: none; cursor: pointer;"
                                    },
                                    onclick: move |_| view_mode.set(ViewMode::Grid),
                                    "⊞ Grille"
                                }
                                button {
                                    style: if *view_mode.read() == ViewMode::Kanban {
                                        "padding: 4px 10px; font-size: 12px; border-radius: 6px; background: #6366f1; color: #fff; border: none; font-weight: 600; cursor: pointer;"
                                    } else {
                                        "padding: 4px 10px; font-size: 12px; border-radius: 6px; background: transparent; color: var(--text-secondary); border: none; cursor: pointer;"
                                    },
                                    onclick: move |_| view_mode.set(ViewMode::Kanban),
                                    "📋 Kanban PAL"
                                }
                            }

                            if !filtered_books.is_empty() {
                                {
                                    let all_selected = filtered_books.iter().all(|b| selected_ids.read().contains(&b.id));
                                    let current_ids: Vec<String> = filtered_books.iter().map(|b| b.id.clone()).collect();
                                    rsx! {
                                        button {
                                            class: "btn btn-secondary",
                                            style: "padding: 6px 12px; font-size: 12px; border-radius: 8px; display: flex; align-items: center; gap: 6px; cursor: pointer;",
                                            onclick: move |_| {
                                                let mut set = selected_ids.read().clone();
                                                if all_selected {
                                                    for id in &current_ids {
                                                        set.remove(id);
                                                    }
                                                } else {
                                                    for id in &current_ids {
                                                        set.insert(id.clone());
                                                    }
                                                }
                                                selected_ids.set(set);
                                            },
                                            if all_selected { "Désélectionner tout" } else { "Tout sélectionner" }
                                        }
                                    }
                                }
                            }
                            span { style: "font-size: 12px; color: var(--text-muted); font-weight: 500;", "Trier par :" }
                            select {
                                style: "padding: 6px 12px; font-size: 12px; border-radius: 8px; background: rgba(255,255,255,0.05); color: var(--text-primary); border: 1px solid var(--border-subtle); outline: none; cursor: pointer;",
                                value: match *current_sort.read() {
                                    SortOption::RecentAdded => "recent",
                                    SortOption::TitleAsc => "title_asc",
                                    SortOption::TitleDesc => "title_desc",
                                    SortOption::AuthorAsc => "author_asc",
                                    SortOption::ProgressDesc => "progress",
                                },
                                onchange: move |evt| {
                                    let val = evt.value();
                                    let sort = match val.as_str() {
                                        "title_asc" => SortOption::TitleAsc,
                                        "title_desc" => SortOption::TitleDesc,
                                        "author_asc" => SortOption::AuthorAsc,
                                        "progress" => SortOption::ProgressDesc,
                                        _ => SortOption::RecentAdded,
                                    };
                                    current_sort.set(sort);
                                },
                                option { value: "recent", "Récemment ajoutés" }
                                option { value: "title_asc", "Titre (A → Z)" }
                                option { value: "title_desc", "Titre (Z → A)" }
                                option { value: "author_asc", "Auteur (A → Z)" }
                                option { value: "progress", "Progression de lecture" }
                            }
                        }
                    }

                    if *view_mode.read() == ViewMode::Kanban {
                        KanbanBoard {
                            books: filtered_books.clone(),
                            on_select: move |selected| selected_book.set(Some(selected)),
                            on_read: handle_read.clone(),
                            on_change_status: handle_change_book_status.clone(),
                        }
                    } else if filtered_books.is_empty() {
                        div {
                            style: "flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 12px; color: var(--text-muted); padding: 60px 0;",
                            span { style: "font-size: 40px;", "📚" }
                            p { style: "font-size: 15px; font-weight: 500;", "Aucun livre trouvé dans cette vue" }
                            p { style: "font-size: 13px;", "Glissez-déposez des fichiers EPUB, PDF, CBZ ou cliquez sur 'Importer'." }
                            button {
                                class: "btn btn-primary",
                                style: "margin-top: 8px;",
                                onclick: {
                                    let mut h = handle_import.clone();
                                    move |_| h(())
                                },
                                "Ajouter un livre"
                            }
                        }
                    } else {
                        div { class: "books-grid",
                            for b in filtered_books.iter() {
                                {
                                    let id = b.id.clone();
                                    let is_selected = selected_ids.read().contains(&id);
                                    let selection_mode = !selected_ids.read().is_empty();
                                    rsx! {
                                        BookCard {
                                            key: "{b.id}",
                                            book: b.clone(),
                                            is_selected: is_selected,
                                            selection_mode: selection_mode,
                                            on_toggle_select: handle_toggle_select,
                                            on_select: move |selected| selected_book.set(Some(selected)),
                                            on_read: handle_read.clone(),
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Detail Drawer
            if let Some(book) = selected_book.read().as_ref() {
                {
                    let convertible = book_service.get_convertible_formats_for_book(book);
                    rsx! {
                        DetailDrawer {
                            book: book.clone(),
                            convertible_formats: convertible,
                            connected_devices: connected_devices.read().clone(),
                            on_close: move |_| selected_book.set(None),
                            on_read: handle_read.clone(),
                            on_delete: handle_delete.clone(),
                            on_convert: handle_convert.clone(),
                            on_change_cover: handle_change_cover.clone(),
                            on_save_metadata: handle_save_metadata.clone(),
                            on_enrich_metadata: handle_enrich_metadata.clone(),
                            on_open_translate: move |b| translating_book.set(Some(b)),
                            on_export_to_device: handle_export_to_device.clone(),
                            can_send_kindle: smtp_config.read().enabled && !smtp_config.read().kindle_email.is_empty(),
                            is_sending_kindle: *is_sending_kindle.read(),
                            on_send_to_kindle: handle_send_to_kindle.clone(),
                            on_export_file: handle_export_file.clone(),
                            on_open_epub_editor: move |b| epub_editing_book.set(Some(b)),
                        }

                    }
                }
            }

            // EPUB Inspector & TOC Editor Modal
            if let Some(book) = epub_editing_book.read().as_ref() {
                EpubEditorModal {
                    book: book.clone(),
                    book_service: book_service.clone(),
                    on_close: move |_| epub_editing_book.set(None),
                    on_updated: {
                        let mut refresh = refresh_library.clone();
                        move |_| {
                            refresh();
                        }
                    },
                }
            }

            // Translate Modal
            if let Some(book) = translating_book.read().as_ref() {
                TranslateModal {
                    book: book.clone(),
                    is_translating: *is_translating.read(),
                    on_close: move |_| translating_book.set(None),
                    on_start_translate: handle_start_translate.clone(),
                }
            }

            // News Recipes Modal (RSS to EPUB)
            if *news_recipes_open.read() {
                NewsRecipesModal {
                    news_service: news_service.clone(),
                    on_close: move |_| news_recipes_open.set(false),
                    on_imported: {
                        let mut refresh = refresh_library.clone();
                        move |_| {
                            refresh();
                        }
                    },
                }
            }

            // Library Health & Audit Modal
            if *audit_open.read() {
                LibraryAuditModal {
                    book_service: book_service.clone(),
                    on_close: move |_| audit_open.set(false),
                    on_updated: {
                        let mut refresh = refresh_library.clone();
                        move |_| {
                            refresh();
                        }
                    },
                }
            }

            // Reading Stats & Annual Heatmap Modal
            if *stats_open.read() {
                if let Some(ref st) = reading_stats {
                    ReadingStatsModal {
                        stats: st.clone(),
                        on_close: move |_| stats_open.set(false),
                        on_read_book: handle_read.clone(),
                    }
                }
            }

            // Fullscreen Reader
            if active_reader.read().is_some() {
                ReaderModal {
                    session: active_reader,
                    on_close: {
                        let mut refresh = refresh_library.clone();
                        move |_| {
                            active_reader.set(None);
                            refresh();
                        }
                    },
                    on_progress_update: handle_progress,
                }
            }

            // Settings Modal
            if *settings_open.read() {
                SettingsModal {
                    opds_port: opds_port,
                    opds_running: true,
                    litestream_status_text: litestream_status_str,
                    plugins: plugin_descriptors,
                    sync_config: sync_config.read().clone(),
                    sync_available: sync_service.is_backend_available(),
                    is_syncing: *is_syncing.read(),
                    sync_message: sync_message.read().clone(),
                    smtp_config: smtp_config.read().clone(),
                    is_testing_smtp: *is_testing_smtp.read(),
                    smtp_test_message: smtp_test_message.read().clone(),
                    on_sync_now: handle_sync_now.clone(),
                    on_select_folder: handle_pick_sync_folder.clone(),
                    on_auto_detect_folder: handle_auto_detect_folder.clone(),
                    on_toggle_auto_download: handle_toggle_auto_download.clone(),
                    on_toggle_enabled: handle_toggle_sync_enabled.clone(),
                    on_save_smtp: handle_save_smtp.clone(),
                    on_test_smtp: handle_test_smtp.clone(),
                    on_close: move |_| settings_open.set(false),
                }
            }

            // Command Palette (Cmd + K)
            if *palette_open.read() {
                CommandPalette {
                    books: books.read().clone(),
                    series_list: series_names.clone(),
                    on_close: move |_| palette_open.set(false),
                    on_action: handle_palette_action.clone(),
                }
            }

            // Floating Selection Toolbar
            if !selected_ids.read().is_empty() {
                {
                    let count = selected_ids.read().len();
                    rsx! {
                        div {
                            style: "position: fixed; bottom: 28px; left: 50%; transform: translateX(-50%); background: rgba(24, 24, 27, 0.95); backdrop-filter: blur(16px); border: 1px solid rgba(255, 255, 255, 0.15); box-shadow: 0 12px 32px rgba(0,0,0,0.6); padding: 10px 20px; border-radius: 9999px; display: flex; align-items: center; gap: 14px; z-index: 1000;",
                            div {
                                style: "display: flex; align-items: center; gap: 8px; font-size: 13px; font-weight: 600; color: #fff;",
                                span {
                                    style: "background: var(--accent-primary); color: #fff; width: 22px; height: 22px; border-radius: 50%; display: inline-flex; align-items: center; justify-content: center; font-size: 11px; font-weight: 700;",
                                    "{count}"
                                }
                                span { "sélectionné(s)" }
                            }
                            div { style: "width: 1px; height: 20px; background: rgba(255,255,255,0.15);" }
                            button {
                                class: "btn btn-primary",
                                style: "padding: 6px 14px; font-size: 13px; border-radius: 20px; display: flex; align-items: center; gap: 6px; cursor: pointer;",
                                onclick: move |_| bulk_edit_open.set(true),
                                span { "✏️" }
                                "Modifier en masse"
                            }
                            button {
                                class: "btn btn-danger",
                                style: "padding: 6px 14px; font-size: 13px; border-radius: 20px; display: flex; align-items: center; gap: 6px; cursor: pointer;",
                                onclick: {
                                    let mut h = handle_bulk_delete.clone();
                                    move |_| h(())
                                },
                                span { "🗑️" }
                                "Supprimer ({count})"
                            }
                            button {
                                class: "btn btn-secondary",
                                style: "padding: 6px 12px; font-size: 12px; border-radius: 20px; color: var(--text-muted); cursor: pointer;",
                                onclick: move |_| selected_ids.set(std::collections::HashSet::new()),
                                "Annuler"
                            }
                        }
                    }
                }
            }

            // Bulk Edit Modal
            if *bulk_edit_open.read() {
                {
                    let count = selected_ids.read().len();
                    rsx! {
                        BulkEditModal {
                            selected_count: count,
                            on_save: {
                                let mut h = handle_bulk_edit_save.clone();
                                move |data| {
                                    bulk_edit_open.set(false);
                                    h(data);
                                }
                            },
                            on_close: move |_| bulk_edit_open.set(false),
                        }
                    }
                }
            }
        }

    }
}
