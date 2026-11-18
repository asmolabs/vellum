//! Gestionnaires d'actions liées au catalogue, aux métadonnées, à la lecture et aux exports fichiers.

use std::collections::HashSet;
use std::path::PathBuf;

use dioxus::prelude::*;
use vellum_core::{translate, translate_fmt};
use vellum_domain::Book;

use crate::components::bulk_edit_modal::BulkEditRequest;
use crate::components::detail_drawer::SaveMetadataRequest;
use crate::components::{KanbanTargetStatus, TranslationRequest};
use crate::cover_cache;
use crate::state::ReaderSession;

use super::{show_toast, AppActionContext};

pub(super) struct LibraryHandlers {
    pub on_import: EventHandler<()>,
    pub on_import_folder: EventHandler<()>,
    pub on_import_calibre: EventHandler<()>,
    pub on_attach_format: EventHandler<String>,
    pub on_read: EventHandler<Book>,
    pub on_progress: EventHandler<(usize, usize, f64)>,
    pub on_delete: EventHandler<String>,
    pub on_toggle_select: EventHandler<String>,
    pub on_bulk_edit_save: EventHandler<BulkEditRequest>,
    pub on_bulk_delete: EventHandler<()>,
    pub on_bulk_export: EventHandler<()>,
    pub on_change_book_status: EventHandler<(String, KanbanTargetStatus)>,
    pub on_convert: EventHandler<(String, String)>,
    pub on_save_metadata: EventHandler<SaveMetadataRequest>,
    pub on_enrich_metadata: EventHandler<String>,
    pub on_change_cover: EventHandler<(String, PathBuf)>,
    pub on_start_translate: EventHandler<TranslationRequest>,
    pub on_export_file: EventHandler<(String, Option<String>)>,
}

pub(super) fn build_library_handlers(
    mut ctx: AppActionContext,
    on_refresh: EventHandler<()>,
) -> LibraryHandlers {
    let on_import = EventHandler::new({
        let bs = ctx.book_service.clone();
        move |()| {
            let locale = *ctx.ui_locale.read();
            let files = crate::platform::pick_files(
                translate(locale, "actions.import_title"),
                &[(
                    translate(locale, "actions.import_filter"),
                    &[
                        "epub", "pdf", "cbz", "cbr", "mobi", "azw3", "txt", "mp3", "m4b",
                    ],
                )],
            );

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
            let folder =
                crate::platform::pick_folder(translate(locale, "actions.import_calibre_folder"));

            let target_path = if let Some(f) = folder {
                Some(f)
            } else {
                crate::platform::pick_file(
                    translate(locale, "actions.import_calibre_db"),
                    &[(translate(locale, "actions.import_calibre_filter"), &["db"])],
                )
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

    let on_export_file = EventHandler::new({
        let bs = ctx.book_service.clone();
        move |(b_id, pref_fmt): (String, Option<String>)| {
            let locale = *ctx.ui_locale.read();
            let folder =
                crate::platform::pick_folder(translate(locale, "actions.export_folder_title"));

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

    LibraryHandlers {
        on_import,
        on_import_calibre,
        on_read,
        on_progress,
        on_delete,
        on_toggle_select,
        on_bulk_edit_save,
        on_bulk_delete,
        on_change_book_status,
        on_convert,
        on_save_metadata,
        on_enrich_metadata,
        on_change_cover,
        on_start_translate,
        on_export_file,
        on_import_folder: EventHandler::new({
            let bs = ctx.book_service.clone();
            move |()| {
                let locale = *ctx.ui_locale.read();
                if let Some(folder) =
                    crate::platform::pick_folder("Importer un dossier (Audio / Comics / Livres)")
                {
                    let bs_c = bs.clone();
                    spawn(async move {
                        let res =
                            tokio::task::spawn_blocking(move || bs_c.import_folder(&folder)).await;
                        match res {
                            Ok(Ok(imported)) => {
                                on_refresh.call(());
                                show_toast(
                                    ctx.toast_msg,
                                    format!(
                                        "{} élément(s) importé(s) depuis le dossier.",
                                        imported.len()
                                    ),
                                    true,
                                );
                            }
                            Ok(Err(e)) => show_toast(
                                ctx.toast_msg,
                                translate_fmt(locale, "common.error", &[("error", &e.to_string())]),
                                false,
                            ),
                            Err(e) => show_toast(
                                ctx.toast_msg,
                                translate_fmt(
                                    locale,
                                    "common.task_interrupted",
                                    &[("error", &e.to_string())],
                                ),
                                false,
                            ),
                        }
                    });
                }
            }
        }),
        on_attach_format: EventHandler::new({
            let bs = ctx.book_service.clone();
            let mut sel_book = ctx.selected_book;
            move |b_id: String| {
                let locale = *ctx.ui_locale.read();
                let picked = crate::platform::pick_file(
                    "Ajouter un format au livre",
                    &[(
                        "Livres, Audio & Comics",
                        &[
                            "epub", "pdf", "cbz", "cbr", "cb7", "mobi", "azw3", "m4b", "mp3",
                            "m4a", "flac", "opus", "ogg",
                        ],
                    )],
                );
                if let Some(path) = picked {
                    let bs_c = bs.clone();
                    spawn(async move {
                        let res = tokio::task::spawn_blocking(move || {
                            bs_c.attach_format_to_book(&b_id, &path)
                        })
                        .await;
                        match res {
                            Ok(Ok(updated)) => {
                                sel_book.set(Some(updated));
                                on_refresh.call(());
                                show_toast(
                                    ctx.toast_msg,
                                    "Format ajouté au livre.".to_string(),
                                    true,
                                );
                            }
                            Ok(Err(e)) => show_toast(
                                ctx.toast_msg,
                                translate_fmt(locale, "common.error", &[("error", &e.to_string())]),
                                false,
                            ),
                            Err(e) => show_toast(
                                ctx.toast_msg,
                                translate_fmt(
                                    locale,
                                    "common.task_interrupted",
                                    &[("error", &e.to_string())],
                                ),
                                false,
                            ),
                        }
                    });
                }
            }
        }),
        on_bulk_export: EventHandler::new({
            let bs = ctx.book_service.clone();
            let sel_ids = ctx.selected_ids;
            move |()| {
                let locale = *ctx.ui_locale.read();
                let ids: Vec<String> = sel_ids.read().iter().cloned().collect();
                if ids.is_empty() {
                    return;
                }
                if let Some(dest_dir) =
                    crate::platform::pick_folder(translate(locale, "actions.export_folder_title"))
                {
                    let bs_c = bs.clone();
                    spawn(async move {
                        let count = ids.len();
                        let res = tokio::task::spawn_blocking(move || {
                            let mut ok = 0usize;
                            for id in ids {
                                if bs_c
                                    .export_book_to_directory(&id, &dest_dir, None, None)
                                    .is_ok()
                                {
                                    ok += 1;
                                }
                            }
                            ok
                        })
                        .await;
                        match res {
                            Ok(ok) => show_toast(
                                ctx.toast_msg,
                                format!("{ok}/{count} fichier(s) téléchargé(s) dans le dossier."),
                                ok > 0,
                            ),
                            Err(e) => show_toast(
                                ctx.toast_msg,
                                translate_fmt(
                                    locale,
                                    "common.task_interrupted",
                                    &[("error", &e.to_string())],
                                ),
                                false,
                            ),
                        }
                    });
                }
            }
        }),
    }
}
