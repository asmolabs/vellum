//! Gestionnaires d'actions applicatives (import, métadonnées, conversion, synchro et envoi liseuse).

mod library_ops;
mod sync_ops;

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use dioxus::prelude::*;
use vellum_core::book_service::BookService;
use vellum_core::{DeviceService, Locale, MailService, SyncService};
use vellum_domain::{Book, DeviceKind, SmtpConfig};

use crate::app_modals::OverlayHandlers;
use crate::components::detail_drawer::SaveMetadataRequest;
use crate::components::{KanbanTargetStatus, PaletteAction};
use crate::state::{FilterStatus, ReaderSession, ViewMode};

#[derive(Clone)]
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
    pub search_query: Signal<String>,
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
    pub on_import_folder: EventHandler<()>,
    pub on_import_calibre: EventHandler<()>,
    pub on_attach_format: EventHandler<String>,
    pub on_read: EventHandler<Book>,
    pub on_delete: EventHandler<String>,
    pub on_toggle_select: EventHandler<String>,
    pub on_change_book_status: EventHandler<(String, KanbanTargetStatus)>,
    pub on_rate_book: EventHandler<(String, Option<i32>)>,
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

    let lib = library_ops::build_library_handlers(ctx.clone(), on_refresh);
    let sync = sync_ops::build_sync_handlers(ctx.clone(), on_refresh);
    let on_read = lib.on_read;
    let on_import = lib.on_import;
    let on_import_calibre = lib.on_import_calibre;
    let on_sync_now = sync.on_sync_now;
    let opds_port = ctx.opds_port;
    let on_palette_action = EventHandler::new(move |act: PaletteAction| match act {
        PaletteAction::SelectBook(b) => ctx.selected_book.set(Some(b)),
        PaletteAction::ReadBook(b) => on_read.call(b),
        PaletteAction::SetViewGrid => ctx.view_mode.set(ViewMode::Grid),
        PaletteAction::SetViewTable => ctx.view_mode.set(ViewMode::Table),
        PaletteAction::FilterAll => {
            ctx.current_filter.set(FilterStatus::All);
            ctx.selected_tag.set(None);
            ctx.selected_series.set(None);
        }
        PaletteAction::FilterAuthor(author) => {
            ctx.current_filter.set(FilterStatus::All);
            ctx.selected_tag.set(None);
            ctx.selected_series.set(None);
            ctx.search_query.set(format!("author:\"{author}\""));
            ctx.view_mode.set(ViewMode::Grid);
        }
        PaletteAction::FilterSeries(series) => {
            ctx.current_filter.set(FilterStatus::All);
            ctx.selected_tag.set(None);
            ctx.selected_series.set(Some(series));
            ctx.view_mode.set(ViewMode::Grid);
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
            crate::platform::open_url(&format!("http://localhost:{opds_port}"));
        }
        PaletteAction::SyncCloud => on_sync_now.call(()),
        PaletteAction::OpenSettings => ctx.settings_open.set(true),
    });

    let on_open_web_companion = EventHandler::new(move |_| {
        crate::platform::open_url(&format!("http://localhost:{opds_port}"));
    });

    let overlay_handlers = OverlayHandlers {
        on_refresh,
        on_read,
        on_progress: lib.on_progress,
        on_start_translate: lib.on_start_translate,
        on_palette_action,
        on_bulk_edit_save: lib.on_bulk_edit_save,
        on_bulk_delete: lib.on_bulk_delete,
        on_bulk_export: lib.on_bulk_export,
        on_bulk_merge: lib.on_bulk_merge,
        on_sync_now,
        on_pick_sync_folder: sync.on_pick_sync_folder,
        on_auto_detect_folder: sync.on_auto_detect_folder,
        on_toggle_auto_download: sync.on_toggle_auto_download,
        on_toggle_sync_enabled: sync.on_toggle_sync_enabled,
        on_set_sync_mode: sync.on_set_sync_mode,
        on_save_gdrive_creds: sync.on_save_gdrive_creds,
        on_connect_gdrive: sync.on_connect_gdrive,
        on_disconnect_gdrive: sync.on_disconnect_gdrive,
        on_save_smtp: sync.on_save_smtp,
        on_test_smtp: sync.on_test_smtp,
    };

    AppActions {
        on_import,
        on_import_folder: lib.on_import_folder,
        on_import_calibre,
        on_attach_format: lib.on_attach_format,
        on_read,
        on_delete: lib.on_delete,
        on_toggle_select: lib.on_toggle_select,
        on_change_book_status: lib.on_change_book_status,
        on_rate_book: lib.on_rate_book,
        on_convert: lib.on_convert,
        on_save_metadata: lib.on_save_metadata,
        on_enrich_metadata: lib.on_enrich_metadata,
        on_change_cover: lib.on_change_cover,
        on_export_to_device: sync.on_export_to_device,
        on_export_file: lib.on_export_file,
        on_sync_device: sync.on_sync_device,
        on_sync_cloud: on_sync_now,
        on_send_to_kindle: sync.on_send_to_kindle,
        on_open_web_companion,
        overlay_handlers,
    }
}
