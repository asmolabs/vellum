//! Gestionnaires d'actions applicatives (import, métadonnées, conversion, synchro et envoi liseuse).

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use dioxus::prelude::*;
use vellum_core::book_service::BookService;
use vellum_core::{
    translate, translate_fmt, DeviceService, FolderSyncBackend, Locale, MailService, SyncService,
};
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
    pub ui_locale: Signal<Locale>,
    pub toast_msg: Signal<Option<(String, bool)>>,
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

pub(crate) fn show_toast(mut toast_msg: Signal<Option<(String, bool)>>, msg: String, ok: bool) {
    let expected = msg.clone();
    toast_msg.set(Some((msg, ok)));
    spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(4)).await;
        if toast_msg
            .read()
            .as_ref()
            .is_some_and(|(cur, _)| cur == &expected)
        {
            toast_msg.set(None);
        }
    });
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
            let locale = *ctx.ui_locale.read();
            let files = rfd::FileDialog::new()
                .set_title(translate(locale, "actions.import_title"))
                .add_filter(
                    translate(locale, "actions.import_filter"),
                    &[
                        "epub", "pdf", "cbz", "cbr", "mobi", "azw3", "txt", "mp3", "m4b",
                    ],
                )
                .pick_files();

            if let Some(paths) = files {
                let mut ok_count = 0usize;
                let mut err_count = 0usize;
                let mut last_title = String::new();
                let mut last_err = String::new();
                for path in paths {
                    tracing::info!("Importing file: {:?}", path);
                    match bs.import_book(&path) {
                        Ok(book) => {
                            ok_count += 1;
                            last_title = book.title;
                        }
                        Err(e) => {
                            err_count += 1;
                            last_err = e.to_string();
                            tracing::error!("Import error for {:?}: {}", path, e);
                        }
                    }
                }
                on_refresh.call(());
                if err_count == 0 && ok_count == 1 {
                    show_toast(
                        ctx.toast_msg,
                        translate_fmt(locale, "actions.import_one_ok", &[("title", &last_title)]),
                        true,
                    );
                } else if err_count == 0 && ok_count > 1 {
                    show_toast(
                        ctx.toast_msg,
                        translate_fmt(
                            locale,
                            "toolbar.drop_ok",
                            &[("count", &ok_count.to_string())],
                        ),
                        true,
                    );
                } else if ok_count == 0 && err_count > 0 {
                    show_toast(
                        ctx.toast_msg,
                        translate_fmt(locale, "common.error", &[("error", &last_err)]),
                        false,
                    );
                } else if err_count > 0 {
                    show_toast(
                        ctx.toast_msg,
                        translate_fmt(
                            locale,
                            "toolbar.drop_partial",
                            &[
                                ("ok", &ok_count.to_string()),
                                ("err", &err_count.to_string()),
                            ],
                        ),
                        false,
                    );
                }
            }
        }
    });

    let on_import_calibre = EventHandler::new({
        let bs = ctx.book_service.clone();
        move |()| {
            let locale = *ctx.ui_locale.read();
            let folder = rfd::FileDialog::new()
                .set_title(translate(locale, "actions.import_calibre_folder"))
                .pick_folder();

            let target_path = if let Some(f) = folder {
                Some(f)
            } else {
                rfd::FileDialog::new()
                    .set_title(translate(locale, "actions.import_calibre_db"))
                    .add_filter(translate(locale, "actions.import_calibre_filter"), &["db"])
                    .pick_file()
            };

            if let Some(path) = target_path {
                tracing::info!("Importing Calibre library from: {:?}", path);
                let bs_cloned = bs.clone();
                spawn(async move {
                    let path_cloned = path.clone();
                    let res = tokio::task::spawn_blocking(move || {
                        bs_cloned.import_calibre_library(&path_cloned, true)
                    })
                    .await;

                    match res {
                        Ok(Ok(report)) => {
                            show_toast(
                                ctx.toast_msg,
                                translate_fmt(
                                    locale,
                                    "actions.import_calibre_ok",
                                    &[
                                        ("books", &report.books_imported.to_string()),
                                        ("formats", &report.formats_imported.to_string()),
                                    ],
                                ),
                                true,
                            );
                            on_refresh.call(());
                        }
                        Ok(Err(e)) => {
                            tracing::error!("Calibre import error: {e}");
                            show_toast(
                                ctx.toast_msg,
                                translate_fmt(locale, "common.error", &[("error", &e.to_string())]),
                                false,
                            );
                        }
                        Err(e) => {
                            tracing::error!("Background task interrupted: {e}");
                            show_toast(
                                ctx.toast_msg,
                                translate_fmt(
                                    locale,
                                    "common.task_interrupted",
                                    &[("error", &e.to_string())],
                                ),
                                false,
                            );
                        }
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
            let locale = *ctx.ui_locale.read();
            match bs.delete_book(&id, true) {
                Ok(()) => {
                    cover_cache::invalidate_book_cover(&id);
                    ctx.selected_book.set(None);
                    on_refresh.call(());
                    show_toast(
                        ctx.toast_msg,
                        translate(locale, "actions.deleted_one").to_string(),
                        true,
                    );
                }
                Err(e) => show_toast(
                    ctx.toast_msg,
                    translate_fmt(locale, "common.error", &[("error", &e.to_string())]),
                    false,
                ),
            }
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
            let locale = *ctx.ui_locale.read();
            let set = ctx.selected_ids.read().clone();
            let all = ctx.books.read();
            let ordered_ids: Vec<String> = all
                .iter()
                .filter(|b| set.contains(&b.id))
                .map(|b| b.id.clone())
                .collect();

            if !ordered_ids.is_empty() {
                let count = ordered_ids.len().to_string();
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
                show_toast(
                    ctx.toast_msg,
                    translate_fmt(locale, "actions.bulk_updated", &[("count", &count)]),
                    true,
                );
            }
        }
    });

    let on_bulk_delete = EventHandler::new({
        let bs = ctx.book_service.clone();
        move |()| {
            let locale = *ctx.ui_locale.read();
            let set = ctx.selected_ids.read().clone();
            let ids: Vec<String> = set.into_iter().collect();
            if !ids.is_empty() {
                let count = ids.len().to_string();
                let _ = bs.bulk_delete_books(&ids, true);
                for id in &ids {
                    cover_cache::invalidate_book_cover(id);
                }
                ctx.selected_ids.set(HashSet::new());
                ctx.selected_book.set(None);
                on_refresh.call(());
                show_toast(
                    ctx.toast_msg,
                    translate_fmt(locale, "actions.bulk_deleted", &[("count", &count)]),
                    true,
                );
            }
        }
    });

    let on_change_book_status = EventHandler::new({
        let bs = ctx.book_service.clone();
        move |(book_id, target_status): (String, KanbanTargetStatus)| {
            let locale = *ctx.ui_locale.read();
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
                        let fav_name = translate(locale, "actions.favorite_tag");
                        let fav_tag = vellum_domain::Tag::new(fav_name);
                        let _ = bs.tag_repo().save(&fav_tag);
                        if !b.tags.iter().any(|t| t.name == fav_name) {
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
        move |(b_id, target_fmt): (String, String)| {
            let locale = *ctx.ui_locale.read();
            match bs.convert_book_to_format(&b_id, &target_fmt) {
                Ok(updated) => {
                    show_toast(
                        ctx.toast_msg,
                        translate_fmt(
                            locale,
                            "actions.convert_ok",
                            &[("format", &target_fmt), ("title", &updated.title)],
                        ),
                        true,
                    );
                    ctx.selected_book.set(Some(updated));
                    on_refresh.call(());
                }
                Err(e) => {
                    tracing::error!("Conversion error to {target_fmt}: {e}");
                    show_toast(
                        ctx.toast_msg,
                        translate_fmt(locale, "common.error", &[("error", &e.to_string())]),
                        false,
                    );
                }
            }
        }
    });

    let on_save_metadata = EventHandler::new({
        let bs = ctx.book_service.clone();
        move |req: SaveMetadataRequest| {
            let locale = *ctx.ui_locale.read();
            match bs.update_book_metadata_full(vellum_core::UpdateMetadataRequest {
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
            }) {
                Ok(updated) => {
                    cover_cache::invalidate_book_cover(&updated.id);
                    show_toast(
                        ctx.toast_msg,
                        translate_fmt(
                            locale,
                            "actions.metadata_saved",
                            &[("title", &updated.title)],
                        ),
                        true,
                    );
                    ctx.selected_book.set(Some(updated));
                    on_refresh.call(());
                }
                Err(e) => {
                    tracing::error!("Metadata update error: {e}");
                    show_toast(
                        ctx.toast_msg,
                        translate_fmt(locale, "common.error", &[("error", &e.to_string())]),
                        false,
                    );
                }
            }
        }
    });

    let on_enrich_metadata = EventHandler::new({
        let bs = ctx.book_service.clone();
        move |b_id: String| {
            let locale = *ctx.ui_locale.read();
            match bs.enrich_book(&b_id) {
                Ok(updated) => {
                    cover_cache::invalidate_book_cover(&updated.id);
                    show_toast(
                        ctx.toast_msg,
                        translate_fmt(locale, "actions.enriched_ok", &[("title", &updated.title)]),
                        true,
                    );
                    ctx.selected_book.set(Some(updated));
                    on_refresh.call(());
                }
                Err(e) => {
                    tracing::error!("Metadata enrich error: {e}");
                    show_toast(
                        ctx.toast_msg,
                        translate_fmt(locale, "common.error", &[("error", &e.to_string())]),
                        false,
                    );
                }
            }
        }
    });

    let on_change_cover = EventHandler::new({
        let bs = ctx.book_service.clone();
        move |(b_id, path): (String, PathBuf)| {
            let locale = *ctx.ui_locale.read();
            match bs.update_book_cover(&b_id, &path) {
                Ok(updated) => {
                    cover_cache::invalidate_book_cover(&updated.id);
                    show_toast(
                        ctx.toast_msg,
                        translate_fmt(
                            locale,
                            "actions.cover_updated",
                            &[("title", &updated.title)],
                        ),
                        true,
                    );
                    ctx.selected_book.set(Some(updated));
                    on_refresh.call(());
                }
                Err(e) => {
                    tracing::error!("Cover update error: {e}");
                    show_toast(
                        ctx.toast_msg,
                        translate_fmt(locale, "common.error", &[("error", &e.to_string())]),
                        false,
                    );
                }
            }
        }
    });

    let on_start_translate = EventHandler::new({
        let bs = ctx.book_service.clone();
        move |req: TranslationRequest| {
            let locale = *ctx.ui_locale.read();
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
                        show_toast(
                            ctx.toast_msg,
                            translate_fmt(
                                locale,
                                "actions.translated_ok",
                                &[("title", &translated_book.title)],
                            ),
                            true,
                        );
                        ctx.translating_book.set(None);
                        ctx.selected_book.set(Some(translated_book));
                        on_refresh.call(());
                    }
                    Ok(Err(e)) => {
                        tracing::error!("Translation error: {e}");
                        show_toast(
                            ctx.toast_msg,
                            translate_fmt(locale, "common.error", &[("error", &e.to_string())]),
                            false,
                        );
                    }
                    Err(e) => {
                        tracing::error!("Background translation task error: {e}");
                        show_toast(
                            ctx.toast_msg,
                            translate_fmt(
                                locale,
                                "common.async_error",
                                &[("error", &e.to_string())],
                            ),
                            false,
                        );
                    }
                }
            });
        }
    });

    let on_export_to_device = EventHandler::new({
        let ds = ctx.device_service.clone();
        move |(b_id, kind): (String, DeviceKind)| {
            let locale = *ctx.ui_locale.read();
            match ds.export_book_to_device(&b_id, kind, None) {
                Ok(_) => show_toast(
                    ctx.toast_msg,
                    translate_fmt(
                        locale,
                        "actions.device_exported",
                        &[("device", &format!("{kind:?}"))],
                    ),
                    true,
                ),
                Err(e) => {
                    tracing::error!("Device export error ({kind:?}): {e}");
                    show_toast(
                        ctx.toast_msg,
                        translate_fmt(locale, "common.error", &[("error", &e.to_string())]),
                        false,
                    );
                }
            }
        }
    });

    let on_export_file = EventHandler::new({
        let bs = ctx.book_service.clone();
        move |(b_id, pref_fmt): (String, Option<String>)| {
            let locale = *ctx.ui_locale.read();
            let folder = rfd::FileDialog::new()
                .set_title(translate(locale, "actions.export_folder_title"))
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
                            let name = out_path
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_else(|| out_path.display().to_string());
                            show_toast(
                                ctx.toast_msg,
                                translate_fmt(locale, "actions.file_exported", &[("name", &name)]),
                                true,
                            );
                        }
                        Ok(Err(e)) => {
                            tracing::error!("File export error: {e}");
                            show_toast(
                                ctx.toast_msg,
                                translate_fmt(locale, "common.error", &[("error", &e.to_string())]),
                                false,
                            );
                        }
                        Err(e) => {
                            tracing::error!("Export task interrupted: {e}");
                            show_toast(
                                ctx.toast_msg,
                                translate_fmt(
                                    locale,
                                    "common.task_interrupted",
                                    &[("error", &e.to_string())],
                                ),
                                false,
                            );
                        }
                    }
                });
            }
        }
    });

    let on_sync_device = EventHandler::new({
        let ds = ctx.device_service.clone();
        move |kind: DeviceKind| {
            let locale = *ctx.ui_locale.read();
            let _ = ds.sync_reading_progress(kind, None);
            let _ = ds.import_books_from_device(kind, None);
            on_refresh.call(());
            ctx.connected_devices.set(ds.detect_connected_devices());
            show_toast(
                ctx.toast_msg,
                translate_fmt(
                    locale,
                    "actions.device_synced",
                    &[("device", &format!("{kind:?}"))],
                ),
                true,
            );
        }
    });

    let on_sync_now = EventHandler::new({
        let ss = ctx.sync_service.clone();
        let pr = ctx.book_service.param_repo().clone();
        move |()| {
            if *ctx.is_syncing.read() {
                return;
            }
            let locale = *ctx.ui_locale.read();
            ctx.is_syncing.set(true);
            ctx.sync_message
                .set(Some(translate(locale, "actions.sync_running").to_string()));
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
                        let msg = translate_fmt(
                            locale,
                            "actions.sync_ok",
                            &[
                                ("uploaded", &report.uploaded_books.to_string()),
                                ("downloaded", &report.downloaded_books.to_string()),
                                ("events", &report.pulled_events.to_string()),
                            ],
                        );
                        tracing::info!("{msg}");
                        ctx.sync_message.set(Some(msg.clone()));
                        show_toast(ctx.toast_msg, msg, true);
                        on_refresh.call(());
                    }
                    Ok((Err(e), _)) => {
                        let err_msg =
                            translate_fmt(locale, "actions.sync_err", &[("error", &e.to_string())]);
                        tracing::error!("{err_msg}");
                        ctx.sync_message.set(Some(err_msg.clone()));
                        show_toast(ctx.toast_msg, err_msg, false);
                    }
                    Err(e) => {
                        tracing::error!("Async sync task error: {e}");
                        let msg = translate(locale, "actions.sync_unexpected").to_string();
                        ctx.sync_message.set(Some(msg.clone()));
                        show_toast(ctx.toast_msg, msg, false);
                    }
                }
            });
        }
    });

    let on_pick_sync_folder = EventHandler::new({
        let ss = ctx.sync_service.clone();
        let bs = ctx.book_service.clone();
        move |()| {
            let locale = *ctx.ui_locale.read();
            if let Some(folder) = rfd::FileDialog::new()
                .set_title(translate(locale, "actions.sync_folder_title"))
                .pick_folder()
            {
                if ss.set_sync_folder(folder).is_ok() {
                    persist_sync_config(&ss, &bs, ctx.sync_config);
                    ctx.sync_message.set(Some(
                        translate(locale, "actions.sync_folder_ok").to_string(),
                    ));
                }
            }
        }
    });

    let on_auto_detect_folder = EventHandler::new({
        let ss = ctx.sync_service.clone();
        let bs = ctx.book_service.clone();
        move |()| {
            let locale = *ctx.ui_locale.read();
            if let Some(folder) = FolderSyncBackend::auto_detect_google_drive() {
                if ss.set_sync_folder(folder).is_ok() {
                    persist_sync_config(&ss, &bs, ctx.sync_config);
                    ctx.sync_message.set(Some(
                        translate(locale, "actions.sync_folder_detected").to_string(),
                    ));
                }
            } else {
                ctx.sync_message.set(Some(
                    translate(locale, "actions.sync_folder_not_found").to_string(),
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
            let locale = *ctx.ui_locale.read();
            ctx.is_testing_smtp.set(true);
            ctx.smtp_test_message.set(None);
            let ms_cloned = ms.clone();
            spawn(async move {
                let res = ms_cloned.test_connection(&cfg).await;
                ctx.is_testing_smtp.set(false);
                match res {
                    Ok(()) => ctx.smtp_test_message.set(Some((
                        true,
                        translate(locale, "actions.smtp_ok").to_string(),
                    ))),
                    Err(e) => {
                        ctx.smtp_test_message.set(Some((
                            false,
                            translate_fmt(locale, "actions.smtp_err", &[("error", &e.to_string())]),
                        )));
                    }
                }
            });
        }
    });

    let on_send_to_kindle = EventHandler::new({
        let ms = ctx.mail_service.clone();
        move |b: Book| {
            let locale = *ctx.ui_locale.read();
            let cfg = ctx.smtp_config.read().clone();
            ctx.is_sending_kindle.set(true);
            let ms_cloned = ms.clone();
            spawn(async move {
                let res = ms_cloned.send_to_kindle(&b, &cfg, None).await;
                ctx.is_sending_kindle.set(false);
                match res {
                    Ok(()) => show_toast(
                        ctx.toast_msg,
                        translate_fmt(locale, "actions.kindle_sent", &[("title", &b.title)]),
                        true,
                    ),
                    Err(e) => {
                        tracing::error!("Kindle send error: {e}");
                        show_toast(
                            ctx.toast_msg,
                            translate_fmt(locale, "common.error", &[("error", &e.to_string())]),
                            false,
                        );
                    }
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
