//! Gestionnaires d'actions applicatives (import, métadonnées, conversion, synchro et envoi liseuse).

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use dioxus::prelude::*;
use vellum_core::book_service::BookService;
use vellum_core::{DeviceService, FolderSyncBackend, MailService, SyncService};
use vellum_domain::{Book, DeviceKind, SmtpConfig};

use crate::app_modals::OverlayHandlers;
use crate::components::bulk_edit_modal::BulkEditRequest;
use crate::components::detail_drawer::SaveMetadataRequest;
use crate::components::{KanbanTargetStatus, PaletteAction, TranslationRequest};
use crate::cover_cache;
use crate::state::{FilterStatus, ReaderSession, ViewMode};

pub(crate) struct AppActionContext {
    pub book_service: Arc<BookService>,
    pub sync_service: Arc<SyncService>,
    pub device_service: Arc<DeviceService>,
    pub mail_service: Arc<MailService>,
    pub opds_port: u16,
    pub books: Signal<Vec<Book>>,
    pub selected_book: Signal<Option<Book>>,
    pub active_reader: Signal<Option<ReaderSession>>,
    pub selected_ids: Signal<HashSet<String>>,
    pub connected_devices: Signal<Vec<vellum_core::ConnectedDevice>>,
    pub sync_config: Signal<vellum_domain::CloudSyncConfig>,
    pub is_syncing: Signal<bool>,
    pub sync_message: Signal<Option<String>>,
    pub is_translating: Signal<bool>,
    pub translating_book: Signal<Option<Book>>,
    pub smtp_config: Signal<SmtpConfig>,
    pub is_testing_smtp: Signal<bool>,
    pub smtp_test_message: Signal<Option<(bool, String)>>,
    pub is_sending_kindle: Signal<bool>,
    pub current_filter: Signal<FilterStatus>,
    pub selected_tag: Signal<Option<String>>,
    pub selected_series: Signal<Option<String>>,
    pub view_mode: Signal<ViewMode>,
    pub settings_open: Signal<bool>,
    pub news_recipes_open: Signal<bool>,
    pub audit_open: Signal<bool>,
    pub stats_open: Signal<bool>,
    pub content_search_open: Signal<bool>,
    pub bulk_regex_open: Signal<bool>,
    pub import_rules_open: Signal<bool>,
}

pub(crate) struct AppActions {
    pub on_import: EventHandler<()>,
    pub on_import_calibre: EventHandler<()>,
    pub on_read: EventHandler<Book>,
    pub on_delete: EventHandler<String>,
    pub on_toggle_select: EventHandler<String>,
    pub on_change_book_status: EventHandler<(String, KanbanTargetStatus)>,
    pub on_convert: EventHandler<(String, String)>,
    pub on_save_metadata: EventHandler<SaveMetadataRequest>,
    pub on_enrich_metadata: EventHandler<String>,
    pub on_change_cover: EventHandler<(String, PathBuf)>,
    pub on_export_to_device: EventHandler<(String, DeviceKind)>,
    pub on_export_file: EventHandler<(String, Option<String>)>,
    pub on_sync_device: EventHandler<DeviceKind>,
    pub on_sync_cloud: EventHandler<()>,
    pub on_send_to_kindle: EventHandler<Book>,
    pub on_open_web_companion: EventHandler<()>,
    pub overlay_handlers: OverlayHandlers,
}

fn persist_sync_config(
    ss: &SyncService,
    bs: &BookService,
    mut sync_config: Signal<vellum_domain::CloudSyncConfig>,
) {
    let cfg = ss.config();
    if let Ok(cfg_json) = serde_json::to_string(&cfg) {
        let _ = bs.param_repo().set_value("cloud_sync_config", &cfg_json);
    }
    sync_config.set(cfg);
}

pub(crate) fn build_app_actions(mut ctx: AppActionContext) -> AppActions {
    let bs = ctx.book_service.clone();
    let mut books_sig = ctx.books;
    let on_refresh = EventHandler::new(move |()| {
        if let Ok(all) = bs.book_repo().find_all() {
            books_sig.set(all);
        }
    });

    let on_import = EventHandler::new({
        let bs = ctx.book_service.clone();
        move |()| {
            let files = rfd::FileDialog::new()
                .set_title("Importer des livres ou bandes dessinées dans Vellum")
                .add_filter(
                    "Fichiers pris en charge",
                    &[
                        "epub", "pdf", "cbz", "cbr", "mobi", "azw3", "txt", "mp3", "m4b",
                    ],
                )
                .pick_files();

            if let Some(paths) = files {
                for path in paths {
                    tracing::info!("Importing file: {:?}", path);
                    match bs.import_book(&path) {
                        Ok(book) => tracing::info!(
                            "Livre importé avec succès: {} par {}",
                            book.title,
                            book.author
                        ),
                        Err(e) => tracing::error!("Erreur lors de l'import de {:?}: {}", path, e),
                    }
                }
                on_refresh.call(());
            }
        }
    });

    let on_import_calibre = EventHandler::new({
        let bs = ctx.book_service.clone();
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
                            on_refresh.call(());
                        }
                        Ok(Err(e)) => tracing::error!("Erreur lors de l'import Calibre: {e}"),
                        Err(e) => tracing::error!("Tâche de fond interrompue: {e}"),
                    }
                });
            }
        }
    });

    let on_read = EventHandler::new({
        let bs = ctx.book_service.clone();
        move |book: Book| {
            if let Some(first_fmt) = book.formats.first() {
                let abs_path = bs.library_path().join(&first_fmt.filepath);
                if abs_path.exists() {
                    let session = ReaderSession::new(book, abs_path);
                    ctx.active_reader.set(Some(session));
                    ctx.selected_book.set(None);
                }
            }
        }
    });

    let on_progress = EventHandler::new({
        let bs = ctx.book_service.clone();
        move |(page, total, pct): (usize, usize, f64)| {
            if let Some(ref sess) = *ctx.active_reader.read() {
                let book_id = &sess.book.id;
                let finished = page + 1 >= total && total > 0;
                let _ =
                    bs.update_reading_progress(book_id, page as i32, total as i32, pct, finished);
            }
        }
    });

    let on_delete = EventHandler::new({
        let bs = ctx.book_service.clone();
        move |id: String| {
            let _ = bs.delete_book(&id, true);
            cover_cache::invalidate_book_cover(&id);
            ctx.selected_book.set(None);
            on_refresh.call(());
        }
    });

    let on_toggle_select = EventHandler::new({
        move |id: String| {
            let mut set = ctx.selected_ids.read().clone();
            if set.contains(&id) {
                set.remove(&id);
            } else {
                set.insert(id);
            }
            ctx.selected_ids.set(set);
        }
    });

    let on_bulk_edit_save = EventHandler::new({
        let bs = ctx.book_service.clone();
        move |req: BulkEditRequest| {
            let set = ctx.selected_ids.read().clone();
            let all = ctx.books.read();
            let ordered_ids: Vec<String> = all
                .iter()
                .filter(|b| set.contains(&b.id))
                .map(|b| b.id.clone())
                .collect();

            if !ordered_ids.is_empty() {
                let _ = bs.bulk_update_books(
                    &ordered_ids,
                    req.author.as_deref(),
                    req.series.as_deref(),
                    req.auto_index,
                    &req.add_tags,
                    &req.remove_tags,
                );
                on_refresh.call(());
                ctx.selected_ids.set(HashSet::new());
            }
        }
    });

    let on_bulk_delete = EventHandler::new({
        let bs = ctx.book_service.clone();
        move |()| {
            let set = ctx.selected_ids.read().clone();
            let ids: Vec<String> = set.into_iter().collect();
            if !ids.is_empty() {
                let _ = bs.bulk_delete_books(&ids, true);
                for id in &ids {
                    cover_cache::invalidate_book_cover(id);
                }
                ctx.selected_ids.set(HashSet::new());
                ctx.selected_book.set(None);
                on_refresh.call(());
            }
        }
    });

    let on_change_book_status = EventHandler::new({
        let bs = ctx.book_service.clone();
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
                on_refresh.call(());
            }
        }
    });

    let on_convert = EventHandler::new({
        let bs = ctx.book_service.clone();
        move |(b_id, target_fmt): (String, String)| match bs
            .convert_book_to_format(&b_id, &target_fmt)
        {
            Ok(updated) => {
                tracing::info!("Conversion réussie vers le format {target_fmt}");
                ctx.selected_book.set(Some(updated));
                on_refresh.call(());
            }
            Err(e) => tracing::error!("Erreur lors de la conversion en {target_fmt}: {e}"),
        }
    });

    let on_save_metadata = EventHandler::new({
        let bs = ctx.book_service.clone();
        move |req: SaveMetadataRequest| match bs.update_book_metadata_full(
            vellum_core::UpdateMetadataRequest {
                book_id: &req.book_id,
                title: &req.title,
                author: &req.author,
                publisher: req.publisher.as_deref(),
                description: req.description.as_deref(),
                isbn: req.isbn.as_deref(),
                publish_date: req.publish_date.as_deref(),
                series: req.series.as_deref(),
                series_index: req.series_index,
                tag_names: req.tags,
            },
        ) {
            Ok(updated) => {
                tracing::info!(
                    "Métadonnées mises à jour avec succès pour: {}",
                    updated.title
                );
                cover_cache::invalidate_book_cover(&updated.id);
                ctx.selected_book.set(Some(updated));
                on_refresh.call(());
            }
            Err(e) => tracing::error!("Erreur lors de la mise à jour des métadonnées: {e}"),
        }
    });

    let on_enrich_metadata = EventHandler::new({
        let bs = ctx.book_service.clone();
        move |b_id: String| match bs.enrich_book(&b_id) {
            Ok(updated) => {
                tracing::info!("Métadonnées enrichies avec succès pour: {}", updated.title);
                cover_cache::invalidate_book_cover(&updated.id);
                ctx.selected_book.set(Some(updated));
                on_refresh.call(());
            }
            Err(e) => tracing::error!("Erreur lors de l'enrichissement des métadonnées: {e}"),
        }
    });

    let on_change_cover = EventHandler::new({
        let bs = ctx.book_service.clone();
        move |(b_id, path): (String, PathBuf)| match bs.update_book_cover(&b_id, &path) {
            Ok(updated) => {
                tracing::info!("Couverture mise à jour avec succès pour: {}", updated.title);
                cover_cache::invalidate_book_cover(&updated.id);
                ctx.selected_book.set(Some(updated));
                on_refresh.call(());
            }
            Err(e) => tracing::error!("Erreur lors du changement de couverture: {e}"),
        }
    });

    let on_start_translate = EventHandler::new({
        let bs = ctx.book_service.clone();
        move |req: TranslationRequest| {
            ctx.is_translating.set(true);
            let bs_cloned = bs.clone();
            spawn(async move {
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

                ctx.is_translating.set(false);
                match res {
                    Ok(Ok(translated_book)) => {
                        tracing::info!("Livre traduit avec succès: {}", translated_book.title);
                        ctx.translating_book.set(None);
                        ctx.selected_book.set(Some(translated_book));
                        on_refresh.call(());
                    }
                    Ok(Err(e)) => tracing::error!("Erreur lors de la traduction: {e}"),
                    Err(e) => tracing::error!("Erreur de tâche en arrière-plan: {e}"),
                }
            });
        }
    });

    let on_export_to_device = EventHandler::new({
        let ds = ctx.device_service.clone();
        move |(b_id, kind): (String, DeviceKind)| match ds.export_book_to_device(&b_id, kind, None)
        {
            Ok(dest) => tracing::info!("Livre exporté avec succès vers {:?}: {:?}", kind, dest),
            Err(e) => tracing::error!("Erreur lors de l'exportation vers {:?}: {e}", kind),
        }
    });

    let on_export_file = EventHandler::new({
        let bs = ctx.book_service.clone();
        move |(b_id, pref_fmt): (String, Option<String>)| {
            let folder = rfd::FileDialog::new()
                .set_title("Choisir le dossier de destination pour l'export")
                .pick_folder();

            if let Some(dest_dir) = folder {
                let bs_cloned = bs.clone();
                spawn(async move {
                    let res = tokio::task::spawn_blocking(move || {
                        bs_cloned.export_book_to_directory(
                            &b_id,
                            &dest_dir,
                            pref_fmt.as_deref(),
                            None,
                        )
                    })
                    .await;

                    match res {
                        Ok(Ok(out_path)) => {
                            tracing::info!("Livre exporté avec succès dans: {:?}", out_path)
                        }
                        Ok(Err(e)) => tracing::error!("Erreur lors de l'export: {e}"),
                        Err(e) => tracing::error!("Tâche de fond interrompue: {e}"),
                    }
                });
            }
        }
    });

    let on_sync_device = EventHandler::new({
        let ds = ctx.device_service.clone();
        move |kind: DeviceKind| {
            let _ = ds.sync_reading_progress(kind, None);
            let _ = ds.import_books_from_device(kind, None);
            on_refresh.call(());
            ctx.connected_devices.set(ds.detect_connected_devices());
        }
    });

    let on_sync_now = EventHandler::new({
        let ss = ctx.sync_service.clone();
        let pr = ctx.book_service.param_repo().clone();
        move |()| {
            if *ctx.is_syncing.read() {
                return;
            }
            ctx.is_syncing.set(true);
            ctx.sync_message.set(Some(
                "Synchronisation avec Google Drive en cours...".to_string(),
            ));
            let ss_cloned = ss.clone();
            let pr_cloned = pr.clone();
            spawn(async move {
                let res = tokio::task::spawn_blocking(move || {
                    let r = ss_cloned.sync_now();
                    let cfg = ss_cloned.config();
                    if let Ok(cfg_json) = serde_json::to_string(&cfg) {
                        let _ = pr_cloned.set_value("cloud_sync_config", &cfg_json);
                    }
                    (r, cfg)
                })
                .await;

                ctx.is_syncing.set(false);
                match res {
                    Ok((Ok(report), updated_cfg)) => {
                        ctx.sync_config.set(updated_cfg);
                        let msg = format!(
                            "Synchro réussie : {} envoyé(s), {} reçu(s), {} événement(s)",
                            report.uploaded_books, report.downloaded_books, report.pulled_events
                        );
                        tracing::info!("{msg}");
                        ctx.sync_message.set(Some(msg));
                        on_refresh.call(());
                    }
                    Ok((Err(e), _)) => {
                        let err_msg = format!("Erreur de synchronisation : {e}");
                        tracing::error!("{err_msg}");
                        ctx.sync_message.set(Some(err_msg));
                    }
                    Err(e) => {
                        tracing::error!("Erreur de tâche asynchrone : {e}");
                        ctx.sync_message
                            .set(Some("Erreur inattendue lors de la synchro".to_string()));
                    }
                }
            });
        }
    });

    let on_pick_sync_folder = EventHandler::new({
        let ss = ctx.sync_service.clone();
        let bs = ctx.book_service.clone();
        move |()| {
            if let Some(folder) = rfd::FileDialog::new()
                .set_title("Sélectionner le dossier de synchronisation Google Drive")
                .pick_folder()
            {
                if ss.set_sync_folder(folder).is_ok() {
                    persist_sync_config(&ss, &bs, ctx.sync_config);
                    ctx.sync_message.set(Some(
                        "Dossier Google Drive configuré avec succès !".to_string(),
                    ));
                }
            }
        }
    });

    let on_auto_detect_folder = EventHandler::new({
        let ss = ctx.sync_service.clone();
        let bs = ctx.book_service.clone();
        move |()| {
            if let Some(folder) = FolderSyncBackend::auto_detect_google_drive() {
                if ss.set_sync_folder(folder).is_ok() {
                    persist_sync_config(&ss, &bs, ctx.sync_config);
                    ctx.sync_message.set(Some(
                        "Dossier Google Drive détecté et configuré !".to_string(),
                    ));
                }
            } else {
                ctx.sync_message.set(Some(
                    "Google Drive n'a pas été détecté automatiquement dans les chemins standards."
                        .to_string(),
                ));
            }
        }
    });

    let on_toggle_auto_download = EventHandler::new({
        let ss = ctx.sync_service.clone();
        let bs = ctx.book_service.clone();
        move |auto_dl: bool| {
            ss.set_auto_download_books(auto_dl);
            persist_sync_config(&ss, &bs, ctx.sync_config);
        }
    });

    let on_toggle_sync_enabled = EventHandler::new({
        let ss = ctx.sync_service.clone();
        let bs = ctx.book_service.clone();
        move |enabled: bool| {
            ss.set_enabled(enabled);
            persist_sync_config(&ss, &bs, ctx.sync_config);
        }
    });

    let on_save_smtp = EventHandler::new({
        let pr = ctx.book_service.param_repo().clone();
        move |cfg: SmtpConfig| {
            ctx.smtp_config.set(cfg.clone());
            if let Ok(cfg_json) = serde_json::to_string(&cfg) {
                let _ = pr.set_value("smtp_config", &cfg_json);
            }
        }
    });

    let on_test_smtp = EventHandler::new({
        let ms = ctx.mail_service.clone();
        move |cfg: SmtpConfig| {
            ctx.is_testing_smtp.set(true);
            ctx.smtp_test_message.set(None);
            let ms_cloned = ms.clone();
            spawn(async move {
                let res = ms_cloned.test_connection(&cfg).await;
                ctx.is_testing_smtp.set(false);
                match res {
                    Ok(()) => ctx.smtp_test_message.set(Some((
                        true,
                        "Connexion SMTP établie et authentifiée avec succès !".into(),
                    ))),
                    Err(e) => {
                        ctx.smtp_test_message
                            .set(Some((false, format!("Échec du test SMTP : {e}"))));
                    }
                }
            });
        }
    });

    let on_send_to_kindle = EventHandler::new({
        let ms = ctx.mail_service.clone();
        move |b: Book| {
            let cfg = ctx.smtp_config.read().clone();
            ctx.is_sending_kindle.set(true);
            let ms_cloned = ms.clone();
            spawn(async move {
                let res = ms_cloned.send_to_kindle(&b, &cfg, None).await;
                ctx.is_sending_kindle.set(false);
                match res {
                    Ok(()) => tracing::info!("Livre '{}' envoyé avec succès à la Kindle", b.title),
                    Err(e) => tracing::error!("Erreur lors de l'envoi vers Kindle: {e}"),
                }
            });
        }
    });

    let opds_port = ctx.opds_port;
    let on_palette_action = EventHandler::new(move |act: PaletteAction| match act {
        PaletteAction::SelectBook(b) => ctx.selected_book.set(Some(b)),
        PaletteAction::ReadBook(b) => on_read.call(b),
        PaletteAction::SetViewGrid => ctx.view_mode.set(ViewMode::Grid),
        PaletteAction::SetViewTable => ctx.view_mode.set(ViewMode::Table),
        PaletteAction::SetViewTableCarousel => ctx.view_mode.set(ViewMode::TableCarousel),
        PaletteAction::SetViewKanban => ctx.view_mode.set(ViewMode::Kanban),
        PaletteAction::FilterAll => {
            ctx.current_filter.set(FilterStatus::All);
            ctx.selected_tag.set(None);
            ctx.selected_series.set(None);
        }
        PaletteAction::FilterReading => {
            ctx.current_filter.set(FilterStatus::Reading);
            ctx.selected_tag.set(None);
            ctx.selected_series.set(None);
        }
        PaletteAction::FilterFinished => {
            ctx.current_filter.set(FilterStatus::Finished);
            ctx.selected_tag.set(None);
            ctx.selected_series.set(None);
        }
        PaletteAction::FilterToRead => {
            ctx.current_filter.set(FilterStatus::ToRead);
            ctx.selected_tag.set(None);
            ctx.selected_series.set(None);
        }
        PaletteAction::ImportBooks => on_import.call(()),
        PaletteAction::ImportCalibre => on_import_calibre.call(()),
        PaletteAction::OpenNewsRecipes => ctx.news_recipes_open.set(true),
        PaletteAction::OpenLibraryAudit => ctx.audit_open.set(true),
        PaletteAction::OpenReadingStats => ctx.stats_open.set(true),
        PaletteAction::OpenContentSearch => ctx.content_search_open.set(true),
        PaletteAction::OpenBulkRegex => ctx.bulk_regex_open.set(true),
        PaletteAction::OpenImportRules => ctx.import_rules_open.set(true),
        PaletteAction::OpenWebCompanion => {
            let _ = open::that(format!("http://localhost:{opds_port}"));
        }
        PaletteAction::SyncCloud => on_sync_now.call(()),
        PaletteAction::OpenSettings => ctx.settings_open.set(true),
    });

    let on_open_web_companion = EventHandler::new(move |_| {
        let _ = open::that(format!("http://localhost:{opds_port}"));
    });

    let overlay_handlers = OverlayHandlers {
        on_refresh,
        on_read,
        on_progress,
        on_start_translate,
        on_palette_action,
        on_bulk_edit_save,
        on_bulk_delete,
        on_sync_now,
        on_pick_sync_folder,
        on_auto_detect_folder,
        on_toggle_auto_download,
        on_toggle_sync_enabled,
        on_save_smtp,
        on_test_smtp,
    };

    AppActions {
        on_import,
        on_import_calibre,
        on_read,
        on_delete,
        on_toggle_select,
        on_change_book_status,
        on_convert,
        on_save_metadata,
        on_enrich_metadata,
        on_change_cover,
        on_export_to_device,
        on_export_file,
        on_sync_device,
        on_sync_cloud: on_sync_now,
        on_send_to_kindle,
        on_open_web_companion,
        overlay_handlers,
    }
}
