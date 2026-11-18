//! Modales applicatives, barre d'actions de sélection flottante et bannières toast de Vellum.

use std::collections::HashSet;
use std::sync::Arc;

use dioxus::prelude::*;
use vellum_core::book_service::BookService;
use vellum_core::{
    translate, translate_fmt, CatalogExporter, CatalogFormat, CustomColumnService, Locale,
    NewsService, SyncService,
};
use vellum_domain::{
    Book, CloudSyncConfig, CustomColumnDefinition, CustomColumnType, PluginDescriptor,
    ReadingStats, SmtpConfig,
};

use crate::components::bulk_edit_modal::BulkEditRequest;
use crate::components::{
    BulkConvertModal, BulkEditModal, BulkRegexModal, CommandPalette, ContentSearchModal,
    EpubEditorModal, ImportRulesModal, LibraryAuditModal, NewsRecipesModal, PaletteAction,
    ReaderModal, ReadingStatsModal, SettingsModal, TranslateModal, TranslationRequest,
};
use crate::state::ReaderSession;

/// Services et métadonnées dérivées nécessaires au rendu des modales.
pub(crate) struct OverlayServices {
    pub book_service: Arc<BookService>,
    pub news_service: Arc<NewsService>,
    pub custom_column_service: Arc<CustomColumnService>,
    pub sync_service: Arc<SyncService>,
    pub opds_port: u16,
    pub opds_running: bool,
    pub litestream_status_str: String,
    pub plugin_descriptors: Vec<PluginDescriptor>,
    pub reading_stats: Option<ReadingStats>,
    pub series_names: Vec<String>,
}

/// Signaux réactifs de visibilité et d'état des modales.
#[derive(Clone, Copy)]
pub(crate) struct OverlayState {
    pub books: Signal<Vec<Book>>,
    pub custom_columns: Signal<Vec<CustomColumnDefinition>>,
    pub watch_folder_path: Signal<String>,
    pub ui_locale: Signal<Locale>,
    pub ui_font_size: Signal<String>,
    pub sync_config: Signal<CloudSyncConfig>,
    pub opds_enabled: Signal<bool>,
    pub is_syncing: Signal<bool>,
    pub sync_message: Signal<Option<String>>,
    pub smtp_config: Signal<SmtpConfig>,
    pub is_testing_smtp: Signal<bool>,
    pub smtp_test_message: Signal<Option<(bool, String)>>,
    pub selected_book: Signal<Option<Book>>,
    pub active_reader: Signal<Option<ReaderSession>>,
    pub settings_open: Signal<bool>,
    pub palette_open: Signal<bool>,
    pub bulk_edit_open: Signal<bool>,
    pub bulk_convert_open: Signal<bool>,
    pub selected_ids: Signal<HashSet<String>>,
    pub translating_book: Signal<Option<Book>>,
    pub is_translating: Signal<bool>,
    pub epub_editing_book: Signal<Option<Book>>,
    pub news_recipes_open: Signal<bool>,
    pub audit_open: Signal<bool>,
    pub stats_open: Signal<bool>,
    pub content_search_open: Signal<bool>,
    pub bulk_regex_open: Signal<bool>,
    pub import_rules_open: Signal<bool>,
    pub toast_msg: Signal<Option<(String, bool)>>,
}

/// Callbacks encapsulés pour les actions déclenchées depuis les modales.
#[derive(Clone, Copy)]
pub(crate) struct OverlayHandlers {
    pub on_refresh: EventHandler<()>,
    pub on_read: EventHandler<Book>,
    pub on_progress: EventHandler<(usize, usize, f64)>,
    pub on_start_translate: EventHandler<TranslationRequest>,
    pub on_palette_action: EventHandler<PaletteAction>,
    pub on_bulk_edit_save: EventHandler<BulkEditRequest>,
    pub on_bulk_delete: EventHandler<()>,
    pub on_bulk_export: EventHandler<()>,
    pub on_bulk_merge: EventHandler<()>,
    pub on_sync_now: EventHandler<()>,
    pub on_pick_sync_folder: EventHandler<()>,
    pub on_auto_detect_folder: EventHandler<()>,
    pub on_toggle_auto_download: EventHandler<bool>,
    pub on_toggle_sync_enabled: EventHandler<bool>,
    pub on_set_sync_mode: EventHandler<vellum_domain::CloudSyncMode>,
    pub on_save_gdrive_creds: EventHandler<(String, String, String)>,
    pub on_connect_gdrive: EventHandler<()>,
    pub on_disconnect_gdrive: EventHandler<()>,
    pub on_save_smtp: EventHandler<SmtpConfig>,
    pub on_test_smtp: EventHandler<SmtpConfig>,
}

/// Rend l'ensemble des modales, du lecteur plein écran, de la barre de sélection et des toasts.
pub(crate) fn render_app_overlays(
    svc: OverlayServices,
    mut st: OverlayState,
    hd: OverlayHandlers,
) -> Element {
    let book_service = svc.book_service;
    let news_service = svc.news_service;
    let custom_column_service = svc.custom_column_service;
    let sync_service = svc.sync_service;

    rsx! {
        // EPUB Inspector & TOC Editor Modal
        if let Some(book) = st.epub_editing_book.read().as_ref() {
            EpubEditorModal {
                locale: *st.ui_locale.read(),
                book: book.clone(),
                book_service: book_service.clone(),
                on_close: move |_| st.epub_editing_book.set(None),
                on_updated: move |_| hd.on_refresh.call(()),
            }
        }

        // Translate Modal
        if let Some(book) = st.translating_book.read().as_ref() {
            TranslateModal {
                locale: *st.ui_locale.read(),
                book: book.clone(),
                is_translating: *st.is_translating.read(),
                on_close: move |_| st.translating_book.set(None),
                on_start_translate: hd.on_start_translate,
            }
        }

        // News Recipes Modal (RSS to EPUB)
        if *st.news_recipes_open.read() {
            NewsRecipesModal {
                locale: *st.ui_locale.read(),
                news_service: news_service.clone(),
                on_close: move |_| st.news_recipes_open.set(false),
                on_imported: move |_| hd.on_refresh.call(()),
            }
        }

        // Library Health & Audit Modal
        if *st.audit_open.read() {
            LibraryAuditModal {
                locale: *st.ui_locale.read(),
                book_service: book_service.clone(),
                on_close: move |_| st.audit_open.set(false),
                on_updated: move |_| hd.on_refresh.call(()),
            }
        }

        // Reading Stats & Annual Heatmap Modal
        if *st.stats_open.read() {
            if let Some(ref reading_stats) = svc.reading_stats {
                ReadingStatsModal {
                    locale: *st.ui_locale.read(),
                    stats: reading_stats.clone(),
                    on_close: move |_| st.stats_open.set(false),
                    on_read_book: hd.on_read,
                }
            }
        }

        // Fullscreen Reader
        if st.active_reader.read().is_some() {
            ReaderModal {
                session: st.active_reader,
                locale: *st.ui_locale.read(),
                on_close: move |_| {
                    st.active_reader.set(None);
                    hd.on_refresh.call(());
                },
                on_progress_update: hd.on_progress,
            }
        }

        // Settings Modal
        if *st.settings_open.read() {
            SettingsModal {
                locale: *st.ui_locale.read(),
                on_change_locale: {
                    let bs = book_service.clone();
                    move |new_loc| {
                        let _ = bs.set_ui_locale(new_loc);
                        st.ui_locale.set(new_loc);
                    }
                },
                ui_font_size: st.ui_font_size.read().clone(),
                on_change_font_size: {
                    let bs = book_service.clone();
                    move |sz: String| {
                        let _ = bs.param_repo().set_value("ui_font_size", &sz);
                        st.ui_font_size.set(sz);
                    }
                },
                opds_port: svc.opds_port,
                opds_running: svc.opds_running,
                opds_enabled: *st.opds_enabled.read(),
                on_toggle_opds: {
                    let bs = book_service.clone();
                    move |enabled: bool| {
                        let _ = bs.param_repo().set_value(
                            "opds_enabled",
                            if enabled { "true" } else { "false" },
                        );
                        st.opds_enabled.set(enabled);
                        let loc = *st.ui_locale.read();
                        let msg = if enabled {
                            translate(loc, "actions.opds_enabled_on")
                        } else {
                            translate(loc, "actions.opds_enabled_off")
                        };
                        st.toast_msg.set(Some((msg.to_string(), true)));
                    }
                },
                litestream_status_text: svc.litestream_status_str.clone(),
                plugins: svc.plugin_descriptors.clone(),
                sync_config: st.sync_config.read().clone(),
                sync_available: sync_service.is_backend_available(),
                is_syncing: *st.is_syncing.read(),
                sync_message: st.sync_message.read().clone(),
                smtp_config: st.smtp_config.read().clone(),
                is_testing_smtp: *st.is_testing_smtp.read(),
                smtp_test_message: st.smtp_test_message.read().clone(),
                custom_columns: st.custom_columns.read().clone(),
                on_create_custom_column: {
                    let ccs = custom_column_service.clone();
                    move |(key, label, col_type): (String, String, CustomColumnType)| {
                        match ccs.define_column(&key, &label, col_type) {
                            Ok(_) => {
                                st.custom_columns.set(ccs.list_columns().unwrap_or_default());
                            }
                            Err(e) => {
                                st.sync_message.set(Some(format!("Erreur colonne : {e}")));
                            }
                        }
                    }
                },
                on_delete_custom_column: {
                    let ccs = custom_column_service.clone();
                    move |id: String| {
                        if ccs.delete_column(&id).is_ok() {
                            st.custom_columns.set(ccs.list_columns().unwrap_or_default());
                        }
                    }
                },
                watch_folder_path: st.watch_folder_path.read().clone(),
                on_scan_watch_folder: {
                    let bs = book_service.clone();
                    move |dir: String| {
                        let locale = *st.ui_locale.read();
                        let _ = bs.param_repo().set_value("watch_folder_path", &dir);
                        st.watch_folder_path.set(dir.clone());
                        match vellum_core::WatchFolderService::scan_once(
                            &bs,
                            std::path::Path::new(&dir),
                            true,
                        ) {
                            Ok(report) => {
                                st.sync_message.set(Some(translate_fmt(
                                    locale,
                                    "actions.watch_folder_ok",
                                    &[
                                        ("imported", &report.imported_book_ids.len().to_string()),
                                        ("skipped", &report.skipped_duplicates.to_string()),
                                    ],
                                )));
                                hd.on_refresh.call(());
                            }
                            Err(e) => {
                                st.sync_message.set(Some(translate_fmt(
                                    locale,
                                    "actions.watch_folder_err",
                                    &[("error", &e.to_string())],
                                )));
                            }
                        }
                    }
                },
                on_export_catalog: move |fmt: CatalogFormat| {
                    let locale = *st.ui_locale.read();
                    let (default_name, ext) = match fmt {
                        CatalogFormat::Csv => ("vellum-catalogue.csv", "csv"),
                        CatalogFormat::Json => ("vellum-catalogue.json", "json"),
                        CatalogFormat::Bibtex => ("vellum-catalogue.bib", "bib"),
                    };
                    if let Some(dest) = crate::platform::save_file(
                        default_name,
                        default_name,
                        &[(translate(locale, "actions.catalog_filter"), &[ext])],
                    ) {
                        let all_books = st.books.read().clone();
                        match CatalogExporter::export_to_file(&all_books, fmt, &dest) {
                            Ok(()) => {
                                st.sync_message.set(Some(translate_fmt(
                                    locale,
                                    "actions.catalog_exported",
                                    &[
                                        ("count", &all_books.len().to_string()),
                                        ("path", &dest.display().to_string()),
                                    ],
                                )));
                            }
                            Err(e) => {
                                st.sync_message.set(Some(translate_fmt(
                                    locale,
                                    "actions.catalog_export_err",
                                    &[("error", &e.to_string())],
                                )));
                            }
                        }
                    }
                },
                on_sync_now: hd.on_sync_now,
                on_select_folder: hd.on_pick_sync_folder,
                on_auto_detect_folder: hd.on_auto_detect_folder,
                on_toggle_auto_download: hd.on_toggle_auto_download,
                on_toggle_enabled: hd.on_toggle_sync_enabled,
                on_set_sync_mode: hd.on_set_sync_mode,
                on_save_gdrive_creds: hd.on_save_gdrive_creds,
                on_connect_gdrive: hd.on_connect_gdrive,
                on_disconnect_gdrive: hd.on_disconnect_gdrive,
                on_save_smtp: hd.on_save_smtp,
                on_test_smtp: hd.on_test_smtp,
                on_close: move |_| st.settings_open.set(false),
            }
        }

        // Command Palette (Cmd + K)
        if *st.palette_open.read() {
            CommandPalette {
                locale: *st.ui_locale.read(),
                books: st.books.read().clone(),
                series_list: svc.series_names.clone(),
                on_close: move |_| st.palette_open.set(false),
                on_action: hd.on_palette_action,
            }
        }

        // Floating Selection Toolbar (BookOrbit SelectionActionBar style)
        if !st.selected_ids.read().is_empty() {
            {
                let count = st.selected_ids.read().len();
                let count_str = count.to_string();
                let loc = *st.ui_locale.read();
                rsx! {
                    div {
                        style: "position: fixed; bottom: 24px; left: 50%; transform: translateX(-50%); background: var(--bg-surface); border: 1px solid var(--border-active); box-shadow: 0 18px 48px rgba(0,0,0,0.65); padding: 8px 16px; border-radius: 9999px; display: flex; align-items: center; gap: 10px; z-index: 1000;",
                        div {
                            style: "display: flex; align-items: center; gap: 8px; font-size: 12.5px; font-weight: 600; color: var(--text-primary); padding-left: 4px;",
                            span {
                                style: "background: var(--accent-primary); color: #fff; min-width: 22px; height: 22px; padding: 0 6px; border-radius: 999px; display: inline-flex; align-items: center; justify-content: center; font-size: 11px; font-weight: 700; font-variant-numeric: tabular-nums;",
                                "{count}"
                            }
                            span { {vellum_core::translate(loc, "selection.count_suffix")} }
                        }
                        div { style: "width: 1px; height: 20px; background: var(--border-subtle);" }
                        button {
                            class: "btn btn-primary",
                            style: "padding: 5px 12px; font-size: 12px; border-radius: 20px; display: flex; align-items: center; gap: 6px; cursor: pointer;",
                            onclick: move |_| st.bulk_edit_open.set(true),
                            crate::components::LucideIcon { kind: crate::components::IconKind::Edit, size: 13 }
                            {vellum_core::translate(loc, "selection.bulk_edit")}
                        }
                        button {
                            class: "btn btn-secondary",
                            style: "padding: 5px 12px; font-size: 12px; border-radius: 20px; display: flex; align-items: center; gap: 6px; cursor: pointer;",
                            onclick: move |_| st.bulk_convert_open.set(true),
                            crate::components::LucideIcon { kind: crate::components::IconKind::RefreshCw, size: 13 }
                            {vellum_core::translate(loc, "selection.convert")}
                        }
                        button {
                            class: "btn btn-secondary",
                            style: "padding: 5px 12px; font-size: 12px; border-radius: 20px; display: flex; align-items: center; gap: 6px; cursor: pointer;",
                            onclick: move |_| st.bulk_regex_open.set(true),
                            crate::components::LucideIcon { kind: crate::components::IconKind::Wrench, size: 13 }
                            {vellum_core::translate(loc, "selection.regex")}
                        }
                        if count >= 2 {
                            button {
                                class: "btn btn-secondary",
                                style: "padding: 5px 12px; font-size: 12px; border-radius: 20px; display: flex; align-items: center; gap: 6px; cursor: pointer;",
                                onclick: move |_| hd.on_bulk_merge.call(()),
                                crate::components::LucideIcon { kind: crate::components::IconKind::Layers, size: 13 }
                                "Fusionner"
                            }
                        }
                        button {
                            class: "btn btn-secondary",
                            style: "padding: 5px 12px; font-size: 12px; border-radius: 20px; display: flex; align-items: center; gap: 6px; cursor: pointer;",
                            onclick: move |_| hd.on_bulk_export.call(()),
                            crate::components::LucideIcon { kind: crate::components::IconKind::Download, size: 13 }
                            "Exporter"
                        }
                        button {
                            class: "btn btn-danger",
                            style: "padding: 5px 12px; font-size: 12px; border-radius: 20px; display: flex; align-items: center; gap: 6px; cursor: pointer;",
                            onclick: move |_| hd.on_bulk_delete.call(()),
                            crate::components::LucideIcon { kind: crate::components::IconKind::Trash, size: 13 }
                            {vellum_core::translate_fmt(loc, "selection.delete_count", &[("count", &count_str)])}
                        }
                        button {
                            class: "btn btn-secondary",
                            style: "padding: 5px 10px; font-size: 12px; border-radius: 20px; color: var(--text-muted); cursor: pointer;",
                            onclick: move |_| st.selected_ids.set(HashSet::new()),
                            "✕"
                        }
                    }
                }
            }
        }

        // Bulk Edit Modal
        if *st.bulk_edit_open.read() {
            {
                let count = st.selected_ids.read().len();
                rsx! {
                    BulkEditModal {
                        locale: *st.ui_locale.read(),
                        selected_count: count,
                        on_save: move |data| {
                            st.bulk_edit_open.set(false);
                            hd.on_bulk_edit_save.call(data);
                        },
                        on_close: move |_| st.bulk_edit_open.set(false),
                    }
                }
            }
        }

        // Full-Text Content Search Modal (FTS5)
        if *st.content_search_open.read() {
            ContentSearchModal {
                locale: *st.ui_locale.read(),
                book_service: book_service.clone(),
                books: st.books.read().clone(),
                on_close: move |_| st.content_search_open.set(false),
                on_select_book: move |b| st.selected_book.set(Some(b)),
            }
        }

        // Bulk Convert Modal
        if *st.bulk_convert_open.read() {
            {
                let ids: Vec<String> = st.selected_ids.read().iter().cloned().collect();
                rsx! {
                    BulkConvertModal {
                        locale: *st.ui_locale.read(),
                        book_service: book_service.clone(),
                        target_book_ids: ids,
                        on_close: move |_| st.bulk_convert_open.set(false),
                        on_converted: move |_| hd.on_refresh.call(()),
                    }
                }
            }
        }

        // Bulk Regex Search & Replace Modal
        if *st.bulk_regex_open.read() {
            {
                let ids: Vec<String> = if st.selected_ids.read().is_empty() {
                    st.books.read().iter().map(|b| b.id.clone()).collect()
                } else {
                    st.selected_ids.read().iter().cloned().collect()
                };
                rsx! {
                    BulkRegexModal {
                        locale: *st.ui_locale.read(),
                        book_service: book_service.clone(),
                        target_book_ids: ids,
                        on_close: move |_| st.bulk_regex_open.set(false),
                        on_updated: move |_| hd.on_refresh.call(()),
                    }
                }
            }
        }

        // Automatic Import Rules Modal
        if *st.import_rules_open.read() {
            ImportRulesModal {
                locale: *st.ui_locale.read(),
                book_service: book_service.clone(),
                on_close: move |_| st.import_rules_open.set(false),
            }
        }

        // Toast Banner
        if let Some((msg, ok)) = st.toast_msg.read().as_ref() {
            div {
                class: "toast-banner",
                style: if *ok {
                    "background: rgba(20, 83, 45, 0.92); color: #dcfce7; border: 1px solid rgba(34, 197, 94, 0.45);"
                } else {
                    "background: rgba(127, 29, 29, 0.92); color: #fee2e2; border: 1px solid rgba(239, 68, 68, 0.45);"
                },
                span { "{msg}" }
                button {
                    style: "background: transparent; border: none; color: inherit; cursor: pointer; font-size: 13px; padding: 0 2px;",
                    onclick: move |_| st.toast_msg.set(None),
                    "✕"
                }
            }
        }
    }
}
