//! # `vellum-ui`
//!
//! Interface desktop native de Vellum basée sur Dioxus.
//!
//! Initialise les services applicatifs (`vellum-core`), démarre le serveur OPDS/Web Companion
//! en tâche de fond et rend l'interface réactive (bibliothèque grille/liste/Kanban, lecteur intégré,
//! éditeur EPUB, palette de commandes, statistiques et modales de configuration).

#![cfg_attr(test, allow(clippy::unwrap_used))]

#[cfg(feature = "desktop")]
use dioxus::desktop::{Config, WindowBuilder};
use dioxus::prelude::*;
use vellum_core::{init_services, translate, Locale};

mod app_actions;
mod app_effects;
mod app_modals;
mod components;
mod cover_cache;
mod library_logic;
mod platform;
mod state;
mod styles;

use components::{
    AnnotationsView, AudioMiniPlayer, AuthorsView, BookCard, BookQuickView, BooksTable,
    DetailDrawer, HomeDashboard, KanbanBoard, LibraryToolbar, ReadingHistoryView, SeriesView,
    Sidebar,
};
use state::{FilterStatus, MediaType, ReaderSession, SortOption, ViewMode};
use styles::APP_CSS;

use vellum_domain::Book;

fn main() {
    tracing_subscriber::fmt::init();

    #[cfg(feature = "desktop")]
    {
        let window_title = translate(Locale::detect_system(), "app.title");
        let cfg = Config::new().with_menu(None).with_window(
            WindowBuilder::new()
                .with_title(window_title)
                .with_inner_size(dioxus::desktop::LogicalSize::new(1200.0, 800.0))
                .with_min_inner_size(dioxus::desktop::LogicalSize::new(960.0, 640.0)),
        );

        dioxus::LaunchBuilder::desktop().with_cfg(cfg).launch(App);
    }

    #[cfg(not(feature = "desktop"))]
    {
        dioxus::LaunchBuilder::mobile().launch(App);
    }
}

#[component]
fn App() -> Element {
    let services = use_hook(init_services);
    let book_service = services.book_service.clone();
    let sync_service = services.sync_service.clone();
    let litestream = services.litestream_service.clone();
    let opds_port = services.opds_port;
    let opds_running = services.opds_running;
    let device_service = services.device_service.clone();
    let virtual_library_service = services.virtual_library_service.clone();
    let custom_column_service = services.custom_column_service.clone();
    let connected_devices = use_signal(|| device_service.detect_connected_devices());

    let sync_config = use_signal(|| sync_service.config());
    let opds_enabled = use_signal(|| opds_running);
    let is_syncing = use_signal(|| false);
    let sync_message = use_signal(|| None::<String>);
    let ui_locale = use_signal(|| book_service.get_ui_locale());
    let mut ui_theme = use_signal(|| {
        book_service
            .param_repo()
            .get_value("ui_theme")
            .ok()
            .flatten()
            .unwrap_or_else(|| "midnight".to_string())
    });

    let books = use_signal(|| book_service.book_repo().find_all().unwrap_or_default());
    let mut virtual_libraries = use_signal(|| virtual_library_service.list().unwrap_or_default());
    let mut active_virtual_library_id = use_signal(|| None::<String>);
    let custom_columns = use_signal(|| custom_column_service.list_columns().unwrap_or_default());
    let watch_folder_path = use_signal(|| {
        book_service
            .param_repo()
            .get_value("watch_folder_path")
            .ok()
            .flatten()
            .unwrap_or_default()
    });

    let current_filter = use_signal(|| FilterStatus::All);
    let media_type = use_signal(|| MediaType::All);
    let format_filter = use_signal(|| None::<String>);
    let min_rating = use_signal(|| None::<i32>);
    let collapse_series = use_signal(|| false);
    let mut quick_view_book = use_signal(|| None::<Book>);
    let mut audio_player_book = use_signal(|| None::<Book>);
    let current_sort = use_signal(|| SortOption::RecentAdded);
    let view_mode = use_signal(|| ViewMode::Home);
    let sidebar_collapsed = use_signal(|| false);
    let selected_tag = use_signal(|| None::<String>);
    let mut selected_series = use_signal(|| None::<String>);
    let search_query = use_signal(String::new);
    let mut selected_book = use_signal(|| None::<Book>);
    let focused_book_id = use_signal(|| None::<String>);
    let grid_scroll_metrics = use_signal(|| (0.0_f64, 920.0_f64, 720.0_f64));
    let active_reader = use_signal(|| None::<ReaderSession>);
    let mut settings_open = use_signal(|| false);
    let mut palette_open = use_signal(|| false);
    let bulk_edit_open = use_signal(|| false);
    let bulk_convert_open = use_signal(|| false);
    let selected_ids = use_signal(std::collections::HashSet::<String>::new);
    let mut translating_book = use_signal(|| None::<Book>);
    let is_translating = use_signal(|| false);
    let mut epub_editing_book = use_signal(|| None::<Book>);
    let mut news_recipes_open = use_signal(|| false);
    let mut audit_open = use_signal(|| false);
    let mut stats_open = use_signal(|| false);
    let mut content_search_open = use_signal(|| false);
    let mut bulk_regex_open = use_signal(|| false);
    let mut import_rules_open = use_signal(|| false);
    let mut is_dragging_files = use_signal(|| false);
    let toast_msg = use_signal(|| None::<(String, bool)>);

    let news_service = services.news_service.clone();
    let mail_service = services.mail_service.clone();
    let smtp_config = use_signal(|| services.smtp_config.clone());
    let is_testing_smtp = use_signal(|| false);
    let smtp_test_message = use_signal(|| None::<(bool, String)>);
    let is_sending_kindle = use_signal(|| false);

    // Calculate media counts (books, audiobooks, comics)
    let media_counts = library_logic::compute_media_counts(&books.read());

    // Calculate tag counts
    let tags_counts = library_logic::compute_tags_counts(&books.read());

    // Calculate series counts
    let series_counts = library_logic::compute_series_counts(&books.read());
    let series_names: Vec<String> = series_counts.iter().map(|(n, _)| n.clone()).collect();

    // Reading stats
    let reading_stats = book_service.get_reading_stats().ok();

    // Filter and sort books
    let filtered_books: Vec<Book> = {
        let all = books.read();
        let active_vl_query = active_virtual_library_id.read().as_ref().and_then(|id| {
            virtual_libraries
                .read()
                .iter()
                .find(|vl| &vl.id == id)
                .map(|vl| vl.query.clone())
        });
        let mut list = library_logic::filter_and_sort_books_by_media(
            &all,
            *current_filter.read(),
            *media_type.read(),
            *current_sort.read(),
            selected_tag.read().as_deref(),
            selected_series.read().as_deref(),
            &search_query.read(),
            active_vl_query.as_deref(),
        );
        if let Some(ref fmt) = *format_filter.read() {
            list.retain(|b| {
                b.formats.iter().any(|f| match fmt.as_str() {
                    "cbz" => matches!(
                        f.format.to_ascii_lowercase().as_str(),
                        "cbz" | "cbr" | "cb7" | "cbt"
                    ),
                    "m4b" => matches!(
                        f.format.to_ascii_lowercase().as_str(),
                        "m4b" | "m4a" | "mp3" | "opus" | "ogg" | "flac"
                    ),
                    other => f.format.eq_ignore_ascii_case(other),
                })
            });
        }
        if let Some(min_r) = *min_rating.read() {
            list.retain(|b| b.rating.is_some_and(|r| r >= min_r));
        }
        list
    };

    let actions = app_actions::build_app_actions(app_actions::AppActionContext {
        book_service: book_service.clone(),
        sync_service: sync_service.clone(),
        device_service: device_service.clone(),
        mail_service: mail_service.clone(),
        opds_port,
        books,
        selected_book,
        active_reader,
        selected_ids,
        connected_devices,
        sync_config,
        is_syncing,
        sync_message,
        is_translating,
        translating_book,
        smtp_config,
        is_testing_smtp,
        smtp_test_message,
        is_sending_kindle,
        current_filter,
        selected_tag,
        selected_series,
        search_query,
        view_mode,
        settings_open,
        news_recipes_open,
        audit_open,
        stats_open,
        content_search_open,
        bulk_regex_open,
        import_rules_open,
        ui_locale,
        toast_msg,
    });

    let kb_ctx = app_effects::KeyboardNavContext {
        books,
        virtual_libraries,
        active_virtual_library_id,
        current_filter,
        current_sort,
        selected_tag,
        selected_series,
        search_query,
        active_reader,
        focused_book_id,
        selected_book,
        palette_open,
        settings_open,
        bulk_edit_open,
        bulk_convert_open,
        bulk_regex_open,
        content_search_open,
        import_rules_open,
        news_recipes_open,
        audit_open,
        stats_open,
        translating_book,
        epub_editing_book,
        on_read: actions.on_read,
    };

    app_effects::use_grid_scroll_observer(grid_scroll_metrics);
    app_effects::use_keyboard_shortcuts(kb_ctx.clone());

    let cur_locale = *ui_locale.read();
    let litestream_status_str =
        app_effects::litestream_status_label(cur_locale, litestream.status());
    let detail_extra = app_effects::build_detail_extra_handlers(
        book_service.clone(),
        custom_column_service.clone(),
        ui_locale,
        toast_msg,
    );
    let drop_handler = app_effects::build_drop_handler(
        book_service.clone(),
        is_dragging_files,
        ui_locale,
        toast_msg,
        actions.overlay_handlers.on_refresh,
    );

    let plugin_descriptors = book_service.plugin_service().list_descriptors();

    let overlay_services = app_modals::OverlayServices {
        book_service: book_service.clone(),
        news_service: news_service.clone(),
        custom_column_service: custom_column_service.clone(),
        sync_service: sync_service.clone(),
        opds_port,
        opds_running,
        litestream_status_str,
        plugin_descriptors,
        reading_stats: reading_stats.clone(),
        series_names: series_names.clone(),
    };
    let overlay_state = app_modals::OverlayState {
        books,
        custom_columns,
        watch_folder_path,
        ui_locale,
        sync_config,
        opds_enabled,
        is_syncing,
        sync_message,
        smtp_config,
        is_testing_smtp,
        smtp_test_message,
        selected_book,
        active_reader,
        settings_open,
        palette_open,
        bulk_edit_open,
        bulk_convert_open,
        selected_ids,
        translating_book,
        is_translating,
        epub_editing_book,
        news_recipes_open,
        audit_open,
        stats_open,
        content_search_open,
        bulk_regex_open,
        import_rules_open,
        toast_msg,
    };
    rsx! {
        style { "{APP_CSS}" }

        div {
            class: "app-container",
            "data-theme": "{ui_theme.read()}",
            tabindex: "0",
            onkeydown: move |evt: KeyboardEvent| {
                app_effects::handle_container_keydown(evt, kb_ctx.clone());
            },
            ondragenter: move |evt: DragEvent| {
                evt.prevent_default();
                is_dragging_files.set(true);
            },
            ondragover: move |evt: DragEvent| {
                evt.prevent_default();
            },
            ondragleave: move |_| {
                is_dragging_files.set(false);
            },
            ondrop: move |evt| drop_handler.call(evt),
            if *is_dragging_files.read() {
                div { class: "drop-zone-overlay",
                    div { class: "drop-zone-box",
                        div { style: "font-size: 36px; margin-bottom: 10px;", "📥" }
                        h3 { style: "margin: 0 0 6px 0; font-size: 18px;", {translate(*ui_locale.read(), "toolbar.drop_title")} }
                        p { style: "margin: 0; font-size: 13px; color: var(--text-secondary);", "EPUB, PDF, CBZ, CBR, MOBI, AZW3, TXT, MP3, M4B" }
                    }
                }
            }
            // Sidebar flottante plein-hauteur (façon BookOrbit)
            Sidebar {
                locale: *ui_locale.read(),
                current_filter: current_filter,
                media_type,
                view_mode,
                sidebar_collapsed,
                search_query,
                on_open_palette: move |_| palette_open.set(true),
                on_open_content_search: move |_| content_search_open.set(true),
                selected_tag: selected_tag,
                selected_series: selected_series,
                media_counts,
                tags_counts: tags_counts,
                series_counts: series_counts,
                virtual_libraries: virtual_libraries.read().clone(),
                active_virtual_library_id: active_virtual_library_id,
                on_create_virtual_library: {
                    let vls = virtual_library_service.clone();
                    move |(name, query): (String, String)| {
                        if vls.create(&name, &query, None).is_ok() {
                            virtual_libraries.set(vls.list().unwrap_or_default());
                        }
                    }
                },
                on_delete_virtual_library: {
                    let vls = virtual_library_service.clone();
                    move |id: String| {
                        if active_virtual_library_id.read().as_ref() == Some(&id) {
                            active_virtual_library_id.set(None);
                        }
                        if vls.delete(&id).is_ok() {
                            virtual_libraries.set(vls.list().unwrap_or_default());
                        }
                    }
                },
                stats: reading_stats.clone(),
                on_open_stats: move |_| stats_open.set(true),
                ui_theme: ui_theme.read().clone(),
                on_change_theme: {
                    let bs = book_service.clone();
                    move |t: String| {
                        let _ = bs.param_repo().set_value("ui_theme", &t);
                        ui_theme.set(t);
                    }
                },
                connected_devices: connected_devices.read().clone(),
                sync_available: sync_service.is_backend_available(),
                is_syncing: *is_syncing.read(),
                on_import: actions.on_import,
                on_import_calibre: actions.on_import_calibre,
                on_open_audit: move |_| audit_open.set(true),
                on_open_import_rules: move |_| import_rules_open.set(true),
                on_open_bulk_regex: move |_| bulk_regex_open.set(true),
                on_open_news: move |_| news_recipes_open.set(true),
                on_open_web_companion: actions.on_open_web_companion,
                on_sync_device: actions.on_sync_device,
                on_sync_cloud: actions.on_sync_cloud,
                on_open_settings: move |_| settings_open.set(true),
            }

            // Main Right Column (sans barre supérieure, façon BookOrbit)
            div { class: "main-column",
                // Books Library
                main {
                    id: "vellum-library-scroll",
                    class: "library-area",
                    if matches!(
                        *view_mode.read(),
                        ViewMode::Grid | ViewMode::Table | ViewMode::TableCarousel | ViewMode::Kanban
                    ) {
                        LibraryToolbar {
                            locale: *ui_locale.read(),
                            current_filter,
                            media_type,
                            search_query,
                            selected_series,
                            selected_tag,
                            format_filter,
                            min_rating,
                            collapse_series,
                            on_shuffle: {
                                let pool = filtered_books.clone();
                                move |_| {
                                    if !pool.is_empty() {
                                        let seed = std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .map(|d| d.subsec_nanos() as usize)
                                            .unwrap_or(0);
                                        let pick = pool[seed % pool.len()].clone();
                                        quick_view_book.set(Some(pick));
                                    }
                                }
                            },
                            filtered_count: filtered_books.len(),
                            filtered_ids: filtered_books.iter().map(|b| b.id.clone()).collect(),
                            selected_ids,
                            view_mode,
                            current_sort,
                        }
                    }

                    if books.read().is_empty() {
                        div {
                            style: "flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 14px; padding: 40px 24px; max-width: 560px; margin: 0 auto; text-align: center;",
                            span { style: "font-size: 46px;", "📚" }
                            h2 { style: "font-size: 20px; font-weight: 700; margin: 0;", {translate(*ui_locale.read(), "onboarding.welcome")} }
                            p { style: "font-size: 13px; color: var(--text-secondary); line-height: 1.5; margin: 0;", {translate(*ui_locale.read(), "onboarding.subtitle")} }
                            div { style: "display: flex; flex-direction: column; gap: 8px; width: 100%; margin-top: 8px;",
                                button {
                                    class: "btn btn-primary",
                                    style: "justify-content: flex-start; padding: 10px 16px;",
                                    onclick: move |_| actions.on_import.call(()),
                                    {translate(*ui_locale.read(), "onboarding.step_import")}
                                }
                                button {
                                    class: "btn btn-secondary",
                                    style: "justify-content: flex-start; padding: 10px 16px;",
                                    onclick: move |_| actions.on_import_calibre.call(()),
                                    {translate(*ui_locale.read(), "onboarding.step_calibre")}
                                }
                                button {
                                    class: "btn btn-secondary",
                                    style: "justify-content: flex-start; padding: 10px 16px;",
                                    onclick: move |_| settings_open.set(true),
                                    {translate(*ui_locale.read(), "onboarding.step_gdrive")}
                                }
                            }
                        }
                    } else if *view_mode.read() == ViewMode::Home {
                        HomeDashboard {
                            locale: *ui_locale.read(),
                            books: books.read().clone(),
                            stats: reading_stats.clone(),
                            current_filter,
                            selected_series,
                            view_mode,
                            on_select: move |selected| selected_book.set(Some(selected)),
                            on_read: actions.on_read,
                        }
                    } else if *view_mode.read() == ViewMode::Authors {
                        AuthorsView {
                            locale: *ui_locale.read(),
                            books: books.read().clone(),
                            search_query,
                            view_mode,
                        }
                    } else if *view_mode.read() == ViewMode::Series {
                        SeriesView {
                            locale: *ui_locale.read(),
                            books: books.read().clone(),
                            selected_series,
                            view_mode,
                        }
                    } else if *view_mode.read() == ViewMode::History {
                        ReadingHistoryView {
                            locale: *ui_locale.read(),
                            books: books.read().clone(),
                            on_select: move |selected| selected_book.set(Some(selected)),
                            on_read: actions.on_read,
                        }
                    } else if *view_mode.read() == ViewMode::Annotations {
                        AnnotationsView {
                            books: books.read().clone(),
                            data_dir: book_service.library_path().join(".vellum"),
                            on_select: move |selected| selected_book.set(Some(selected)),
                            on_read: actions.on_read,
                        }
                    } else if *view_mode.read() == ViewMode::Kanban {
                        KanbanBoard {
                            locale: *ui_locale.read(),
                            books: filtered_books.clone(),
                            on_select: move |selected| selected_book.set(Some(selected)),
                            on_read: actions.on_read,
                            on_change_status: actions.on_change_book_status,
                        }
                    } else if filtered_books.is_empty() {
                        div {
                            style: "flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 12px; color: var(--text-muted); padding: 60px 0;",
                            span { style: "font-size: 40px;", "📚" }
                            p { style: "font-size: 15px; font-weight: 500;", {translate(*ui_locale.read(), "toolbar.empty_title")} }
                            p { style: "font-size: 13px;", {translate(*ui_locale.read(), "toolbar.empty_subtitle")} }
                            button {
                                class: "btn btn-primary",
                                style: "margin-top: 8px;",
                                onclick: move |_| actions.on_import.call(()),
                                {translate(*ui_locale.read(), "toolbar.add_book")}
                            }
                        }
                    } else if *view_mode.read() == ViewMode::Table || *view_mode.read() == ViewMode::TableCarousel {
                        BooksTable {
                            locale: *ui_locale.read(),
                            books: filtered_books.clone(),
                            show_carousel: *view_mode.read() == ViewMode::TableCarousel,
                            selected_ids: selected_ids.read().clone(),
                            focused_book_id,
                            on_toggle_select: actions.on_toggle_select,
                            on_select: move |selected| selected_book.set(Some(selected)),
                            on_read: actions.on_read,
                        }
                    } else {
                        {
                            let continue_books: Vec<vellum_domain::Book> = books
                                .read()
                                .iter()
                                .filter(|b| !b.is_finished && b.progress_percent > 0.0)
                                .take(5)
                                .cloned()
                                .collect();
                            let (scroll_top, grid_w, view_h) = *grid_scroll_metrics.read();
                            let grid_win = library_logic::compute_grid_window(
                                filtered_books.len(),
                                grid_w,
                                scroll_top,
                                view_h,
                            );
                            rsx! {
                                if !continue_books.is_empty() {
                                    div {
                                        style: "display: flex; flex-direction: column; gap: 8px; padding: 0 24px 12px;",
                                        span { style: "font-size: 12px; font-weight: 700; color: var(--text-secondary);", {translate(*ui_locale.read(), "library.continue_reading")} }
                                        div { style: "display: flex; gap: 10px; overflow-x: auto; padding-bottom: 4px;",
                                            for cb in continue_books {
                                                {
                                                    let cb_clone = cb.clone();
                                                    let pct = cb.progress_percent.round() as i32;
                                                    rsx! {
                                                        button {
                                                            key: "cont-{cb.id}",
                                                            class: "btn btn-secondary",
                                                            style: "display: flex; flex-direction: column; align-items: flex-start; gap: 3px; min-width: 190px; max-width: 220px; padding: 8px 12px; text-align: left;",
                                                            onclick: move |_| actions.on_read.call(cb_clone.clone()),
                                                            span { style: "font-size: 12px; font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; width: 100%;", "{cb.title}" }
                                                            span { style: "font-size: 11px; color: var(--text-muted);", "{pct}%" }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                if grid_win.top_spacer_px > 0.0 {
                                    div {
                                        key: "grid-top-spacer",
                                        style: "height: {grid_win.top_spacer_px:.1}px; flex-shrink: 0;",
                                    }
                                }
                                div {
                                    id: "vellum-books-grid",
                                    class: "books-grid",
                                    if *collapse_series.read() {
                                        for (b, stack) in library_logic::collapse_books_by_series(&filtered_books) {
                                            {
                                                let id = b.id.clone();
                                                let is_selected = selected_ids.read().contains(&id);
                                                let selection_mode = !selected_ids.read().is_empty();
                                                let stack_series = stack.as_ref().map(|(s, _)| s.clone());
                                                rsx! {
                                                    BookCard {
                                                        key: "col-{b.id}",
                                                        book: b.clone(),
                                                        is_selected: is_selected,
                                                        selection_mode: selection_mode,
                                                        series_stack: stack,
                                                        on_toggle_select: actions.on_toggle_select,
                                                        on_select: move |selected| {
                                                            if let Some(ref s_name) = stack_series {
                                                                selected_series.set(Some(s_name.clone()));
                                                            } else {
                                                                selected_book.set(Some(selected));
                                                            }
                                                        },
                                                        on_read: actions.on_read,
                                                        on_quick_view: move |qv| quick_view_book.set(Some(qv)),
                                                    }
                                                }
                                            }
                                        }
                                    } else {
                                        for b in filtered_books[grid_win.start_idx..grid_win.end_idx].iter() {
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
                                                        on_toggle_select: actions.on_toggle_select,
                                                        on_select: move |selected| selected_book.set(Some(selected)),
                                                        on_read: actions.on_read,
                                                        on_quick_view: move |qv| quick_view_book.set(Some(qv)),
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                if grid_win.bottom_spacer_px > 0.0 {
                                    div {
                                        key: "grid-bottom-spacer",
                                        style: "height: {grid_win.bottom_spacer_px:.1}px; flex-shrink: 0;",
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
                    let custom_vals = custom_column_service
                        .get_book_values(&book.id)
                        .unwrap_or_default();
                    rsx! {
                        DetailDrawer {
                            locale: *ui_locale.read(),
                            book: book.clone(),
                            convertible_formats: convertible,
                            connected_devices: connected_devices.read().clone(),
                            on_close: move |_| selected_book.set(None),
                            on_read: actions.on_read,
                            on_delete: actions.on_delete,
                            on_convert: actions.on_convert,
                            on_change_cover: actions.on_change_cover,
                            on_save_metadata: actions.on_save_metadata,
                            on_enrich_metadata: actions.on_enrich_metadata,
                            on_open_translate: move |b| translating_book.set(Some(b)),
                            on_export_to_device: actions.on_export_to_device,
                            can_send_kindle: smtp_config.read().enabled && !smtp_config.read().kindle_email.is_empty(),
                            is_sending_kindle: *is_sending_kindle.read(),
                            on_send_to_kindle: actions.on_send_to_kindle,
                            on_export_file: actions.on_export_file,
                            on_export_notes_md: detail_extra.on_export_notes_md,
                            on_open_epub_editor: move |b| epub_editing_book.set(Some(b)),
                            on_polish_epub: detail_extra.on_polish_epub,
                            custom_columns: custom_columns.read().clone(),
                            custom_values: custom_vals,
                            on_save_custom_value: detail_extra.on_save_custom_value,
                        }

                    }
                }
            }

            if let Some(qv_book) = quick_view_book.read().as_ref() {
                BookQuickView {
                    book: qv_book.clone(),
                    on_close: move |_| quick_view_book.set(None),
                    on_read: move |b: Book| {
                        if MediaType::Audiobooks.matches_book(&b) && !MediaType::Books.matches_book(&b) {
                            audio_player_book.set(Some(b));
                        } else {
                            actions.on_read.call(b);
                        }
                    },
                    on_open_details: move |b| selected_book.set(Some(b)),
                }
            }

            if let Some(ab_book) = audio_player_book.read().as_ref() {
                AudioMiniPlayer {
                    book: ab_book.clone(),
                    on_close: move |_| audio_player_book.set(None),
                    on_open_reader: actions.on_read,
                }
            }

            {app_modals::render_app_overlays(overlay_services, overlay_state, actions.overlay_handlers)}
        }

    }
}
