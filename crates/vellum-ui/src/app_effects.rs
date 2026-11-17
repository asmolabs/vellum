//! Effets réactifs globaux, raccourcis clavier, drag & drop et actions complémentaires du tiroir de détail.

use std::path::PathBuf;
use std::sync::Arc;

use dioxus::html::HasFileData;
use dioxus::prelude::*;
use vellum_core::book_service::BookService;
use vellum_core::litestream_service::LitestreamStatus;
use vellum_core::{translate, translate_fmt, CustomColumnService, Locale, PolishOptions};
use vellum_domain::{Book, VirtualLibrary};

use crate::app_actions;
use crate::library_logic;
use crate::state::{FilterStatus, ReaderSession, SortOption};

#[derive(Clone)]
pub(crate) struct KeyboardNavContext {
    pub books: Signal<Vec<Book>>,
    pub virtual_libraries: Signal<Vec<VirtualLibrary>>,
    pub active_virtual_library_id: Signal<Option<String>>,
    pub current_filter: Signal<FilterStatus>,
    pub current_sort: Signal<SortOption>,
    pub selected_tag: Signal<Option<String>>,
    pub selected_series: Signal<Option<String>>,
    pub search_query: Signal<String>,
    pub active_reader: Signal<Option<ReaderSession>>,
    pub focused_book_id: Signal<Option<String>>,
    pub selected_book: Signal<Option<Book>>,
    pub palette_open: Signal<bool>,
    pub settings_open: Signal<bool>,
    pub bulk_edit_open: Signal<bool>,
    pub bulk_convert_open: Signal<bool>,
    pub bulk_regex_open: Signal<bool>,
    pub content_search_open: Signal<bool>,
    pub import_rules_open: Signal<bool>,
    pub news_recipes_open: Signal<bool>,
    pub audit_open: Signal<bool>,
    pub stats_open: Signal<bool>,
    pub translating_book: Signal<Option<Book>>,
    pub epub_editing_book: Signal<Option<Book>>,
    pub on_read: EventHandler<Book>,
}

pub(crate) struct DetailExtraHandlers {
    pub on_export_notes_md: EventHandler<String>,
    pub on_polish_epub: EventHandler<String>,
    pub on_save_custom_value: EventHandler<(String, String, String)>,
}

/// Observe le défilement et le redimensionnement du conteneur principal pour virtualiser la grille.
pub(crate) fn use_grid_scroll_observer(mut grid_scroll_metrics: Signal<(f64, f64, f64)>) {
    use_effect(move || {
        let mut eval = document::eval(
            r#"
            const scroller = document.getElementById('vellum-library-scroll');
            if (!scroller) return;
            let rafId = 0;
            const emit = () => {
                rafId = 0;
                const grid = document.getElementById('vellum-books-grid');
                const width = grid ? grid.clientWidth : Math.max(scroller.clientWidth - 48, 200);
                dioxus.send([scroller.scrollTop, width, scroller.clientHeight]);
            };
            const schedule = () => {
                if (!rafId) rafId = requestAnimationFrame(emit);
            };
            scroller.addEventListener('scroll', schedule, { passive: true });
            const ro = new ResizeObserver(schedule);
            ro.observe(scroller);
            emit();
        "#,
        );
        spawn(async move {
            while let Ok(vals) = eval.recv::<Vec<f64>>().await {
                if vals.len() == 3 {
                    grid_scroll_metrics.set((vals[0], vals[1], vals[2]));
                }
            }
        });
    });
}

fn current_filtered(ctx: &KeyboardNavContext) -> Vec<Book> {
    let active_vl_query = ctx
        .active_virtual_library_id
        .peek()
        .as_ref()
        .and_then(|id| {
            ctx.virtual_libraries
                .peek()
                .iter()
                .find(|vl| &vl.id == id)
                .map(|vl| vl.query.clone())
        });
    library_logic::filter_and_sort_books(
        &ctx.books.peek(),
        *ctx.current_filter.peek(),
        *ctx.current_sort.peek(),
        ctx.selected_tag.peek().as_deref(),
        ctx.selected_series.peek().as_deref(),
        &ctx.search_query.peek(),
        active_vl_query.as_deref(),
    )
}

fn active_book(ctx: &KeyboardNavContext, list: &[Book]) -> Option<Book> {
    let cur_focused = ctx.focused_book_id.peek();
    let sel_book = ctx.selected_book.peek();
    let target_id = cur_focused
        .as_deref()
        .or_else(|| sel_book.as_ref().map(|b| b.id.as_str()));
    library_logic::resolve_active_index(list, target_id).and_then(|i| list.get(i).cloned())
}

fn close_top_overlay(ctx: &mut KeyboardNavContext) {
    if *ctx.palette_open.peek() {
        ctx.palette_open.set(false);
    } else if *ctx.bulk_convert_open.peek() {
        ctx.bulk_convert_open.set(false);
    } else if *ctx.bulk_edit_open.peek() {
        ctx.bulk_edit_open.set(false);
    } else if *ctx.bulk_regex_open.peek() {
        ctx.bulk_regex_open.set(false);
    } else if *ctx.settings_open.peek() {
        ctx.settings_open.set(false);
    } else if *ctx.audit_open.peek() {
        ctx.audit_open.set(false);
    } else if *ctx.stats_open.peek() {
        ctx.stats_open.set(false);
    } else if *ctx.content_search_open.peek() {
        ctx.content_search_open.set(false);
    } else if *ctx.import_rules_open.peek() {
        ctx.import_rules_open.set(false);
    } else if *ctx.news_recipes_open.peek() {
        ctx.news_recipes_open.set(false);
    } else if ctx.epub_editing_book.peek().is_some() {
        ctx.epub_editing_book.set(None);
    } else if ctx.translating_book.peek().is_some() {
        ctx.translating_book.set(None);
    } else if ctx.selected_book.peek().is_some() {
        ctx.selected_book.set(None);
    }
}

/// Enregistre les raccourcis clavier globaux (`Cmd/Ctrl+K`, `Escape`, flèches, `Enter`, `Space`).
pub(crate) fn use_keyboard_shortcuts(ctx: KeyboardNavContext) {
    use_effect(move || {
        let mut ctx_spawn = ctx.clone();
        let mut eval = document::eval(
            r#"
            window.addEventListener('keydown', (e) => {
                if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
                    e.preventDefault();
                    dioxus.send('toggle_palette');
                    return;
                }
                const tag = e.target?.tagName;
                const isEditable = tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT' || e.target?.isContentEditable;
                if (e.key === 'Escape') {
                    dioxus.send('escape');
                    return;
                }
                if (isEditable) return;
                if (document.querySelector('.reader-modal, .modal-backdrop, .palette-overlay')) return;
                if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') {
                    e.preventDefault();
                    dioxus.send('prev');
                } else if (e.key === 'ArrowRight' || e.key === 'ArrowDown') {
                    e.preventDefault();
                    dioxus.send('next');
                } else if (e.key === 'Enter' && tag !== 'BUTTON' && tag !== 'A') {
                    e.preventDefault();
                    dioxus.send('open_reader');
                } else if (e.key === ' ' && tag !== 'BUTTON' && tag !== 'A') {
                    e.preventDefault();
                    dioxus.send('toggle_detail');
                }
            });
        "#,
        );
        spawn(async move {
            while let Ok(cmd) = eval.recv::<String>().await {
                if ctx_spawn.active_reader.peek().is_some() {
                    continue;
                }
                match cmd.as_str() {
                    "toggle_palette" => {
                        let cur = *ctx_spawn.palette_open.peek();
                        ctx_spawn.palette_open.set(!cur);
                    }
                    "escape" => close_top_overlay(&mut ctx_spawn),
                    "prev" | "next" => {
                        let delta = if cmd == "prev" { -1 } else { 1 };
                        let list = current_filtered(&ctx_spawn);
                        let next_id = {
                            let cur_focused = ctx_spawn.focused_book_id.peek();
                            let sel_book = ctx_spawn.selected_book.peek();
                            let fallback_id = cur_focused
                                .as_deref()
                                .or_else(|| sel_book.as_ref().map(|b| b.id.as_str()));
                            library_logic::next_focused_book_id(&list, fallback_id, delta)
                        };
                        if let Some(ref id) = next_id {
                            ctx_spawn.focused_book_id.set(Some(id.clone()));
                            if ctx_spawn.selected_book.peek().is_some() {
                                if let Some(b) = list.iter().find(|b| &b.id == id).cloned() {
                                    ctx_spawn.selected_book.set(Some(b));
                                }
                            }
                        }
                    }
                    "open_reader" => {
                        let list = current_filtered(&ctx_spawn);
                        if let Some(book) = active_book(&ctx_spawn, &list) {
                            ctx_spawn.focused_book_id.set(Some(book.id.clone()));
                            ctx_spawn.on_read.call(book);
                        }
                    }
                    "toggle_detail" => {
                        let list = current_filtered(&ctx_spawn);
                        if let Some(book) = active_book(&ctx_spawn, &list) {
                            ctx_spawn.focused_book_id.set(Some(book.id.clone()));
                            let already_open = ctx_spawn
                                .selected_book
                                .peek()
                                .as_ref()
                                .is_some_and(|b| b.id == book.id);
                            if already_open {
                                ctx_spawn.selected_book.set(None);
                            } else {
                                ctx_spawn.selected_book.set(Some(book));
                            }
                        }
                    }
                    _ => {}
                }
            }
        });
    });
}

/// Gère l'événement `keydown` sur le conteneur principal (`Escape` et `Cmd/Ctrl+K`).
pub(crate) fn handle_container_keydown(evt: KeyboardEvent, mut ctx: KeyboardNavContext) {
    if evt.key() == Key::Escape {
        ctx.palette_open.set(false);
        ctx.selected_book.set(None);
        ctx.settings_open.set(false);
        ctx.bulk_edit_open.set(false);
        ctx.bulk_convert_open.set(false);
        ctx.bulk_regex_open.set(false);
        ctx.content_search_open.set(false);
        ctx.import_rules_open.set(false);
        ctx.news_recipes_open.set(false);
        ctx.audit_open.set(false);
        ctx.stats_open.set(false);
        ctx.translating_book.set(None);
        ctx.epub_editing_book.set(None);
    } else if (evt.modifiers().meta() || evt.modifiers().ctrl())
        && matches!(evt.key(), Key::Character(ref c) if c.eq_ignore_ascii_case("k"))
    {
        evt.prevent_default();
        let cur = *ctx.palette_open.read();
        ctx.palette_open.set(!cur);
    }
}

/// Construit le gestionnaire d'import par glisser-déposer.
pub(crate) fn build_drop_handler(
    book_service: Arc<BookService>,
    mut is_dragging_files: Signal<bool>,
    ui_locale: Signal<Locale>,
    toast_msg: Signal<Option<(String, bool)>>,
    on_refresh: EventHandler<()>,
) -> EventHandler<DragEvent> {
    EventHandler::new(move |evt: DragEvent| {
        evt.prevent_default();
        is_dragging_files.set(false);
        if let Some(file_engine) = evt.files() {
            let paths: Vec<PathBuf> = file_engine.files().into_iter().map(PathBuf::from).collect();
            if !paths.is_empty() {
                let bs_task = book_service.clone();
                spawn(async move {
                    let (ok_count, err_count) = tokio::task::spawn_blocking(move || {
                        let mut ok = 0usize;
                        let mut err = 0usize;
                        for path in paths {
                            match bs_task.import_book(&path) {
                                Ok(_) => ok += 1,
                                Err(e) => {
                                    err += 1;
                                    tracing::error!(
                                        "Drag & drop import failed for {}: {}",
                                        path.display(),
                                        e
                                    );
                                }
                            }
                        }
                        (ok, err)
                    })
                    .await
                    .unwrap_or((0, 1));
                    on_refresh.call(());
                    let loc = *ui_locale.read();
                    if err_count == 0 {
                        app_actions::show_toast(
                            toast_msg,
                            translate_fmt(
                                loc,
                                "toolbar.drop_ok",
                                &[("count", &ok_count.to_string())],
                            ),
                            true,
                        );
                    } else {
                        app_actions::show_toast(
                            toast_msg,
                            translate_fmt(
                                loc,
                                "toolbar.drop_partial",
                                &[
                                    ("ok", &ok_count.to_string()),
                                    ("err", &err_count.to_string()),
                                ],
                            ),
                            false,
                        );
                    }
                });
            }
        }
    })
}

/// Retourne le libellé localisé du statut Litestream.
pub(crate) fn litestream_status_label(locale: Locale, status: LitestreamStatus) -> String {
    translate(
        locale,
        match status {
            LitestreamStatus::Running => "litestream.running",
            LitestreamStatus::Stopped => "litestream.stopped",
            LitestreamStatus::Disabled => "litestream.disabled",
            LitestreamStatus::BinaryNotFound => "litestream.binary_not_found",
            LitestreamStatus::Error => "litestream.error",
        },
    )
    .to_string()
}

/// Construit les actions secondaires du tiroir de détail (export notes Markdown, polish EPUB, colonnes custom).
pub(crate) fn build_detail_extra_handlers(
    book_service: Arc<BookService>,
    custom_column_service: Arc<CustomColumnService>,
    ui_locale: Signal<Locale>,
    toast_msg: Signal<Option<(String, bool)>>,
) -> DetailExtraHandlers {
    let bs_notes = book_service.clone();
    let on_export_notes_md = EventHandler::new(move |b_id: String| {
        let locale = *ui_locale.read();
        if let Some(dest) =
            crate::platform::save_file("Markdown", "notes.md", &[("Markdown", &["md"])])
        {
            match bs_notes.export_book_notes_markdown(&b_id, Some(&dest)) {
                Ok(_) => app_actions::show_toast(
                    toast_msg,
                    translate_fmt(
                        locale,
                        "actions.notes_exported",
                        &[("path", &dest.display().to_string())],
                    ),
                    true,
                ),
                Err(e) => app_actions::show_toast(
                    toast_msg,
                    translate_fmt(
                        locale,
                        "actions.notes_export_err",
                        &[("error", &e.to_string())],
                    ),
                    false,
                ),
            }
        }
    });

    let bs_polish = book_service;
    let on_polish_epub = EventHandler::new(move |b_id: String| {
        let locale = *ui_locale.read();
        match bs_polish.polish_book(&b_id, PolishOptions::default()) {
            Ok(report) => app_actions::show_toast(
                toast_msg,
                translate_fmt(
                    locale,
                    "actions.polish_ok",
                    &[
                        ("meta", &report.metadata_updated.to_string()),
                        ("cover", &report.cover_embedded.to_string()),
                        ("chapters", &report.chapters_smartened.to_string()),
                        ("toc", &report.toc_rebuilt.to_string()),
                    ],
                ),
                true,
            ),
            Err(e) => app_actions::show_toast(
                toast_msg,
                translate_fmt(locale, "actions.polish_err", &[("error", &e.to_string())]),
                false,
            ),
        }
    });

    let on_save_custom_value =
        EventHandler::new(move |(b_id, key, val): (String, String, String)| {
            let locale = *ui_locale.read();
            if let Err(e) = custom_column_service.set_book_value(&b_id, &key, &val) {
                app_actions::show_toast(
                    toast_msg,
                    translate_fmt(
                        locale,
                        "actions.custom_column_err",
                        &[("key", &key), ("error", &e.to_string())],
                    ),
                    false,
                );
            }
        });

    DetailExtraHandlers {
        on_export_notes_md,
        on_polish_epub,
        on_save_custom_value,
    }
}
