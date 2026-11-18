use crate::components::{IconKind, LucideIcon};
use crate::cover_cache::resolve_cover_src;
use crate::state::MediaType;
use dioxus::prelude::*;
use vellum_domain::Book;

#[component]
pub fn BookCard(
    book: Book,
    is_selected: bool,
    selection_mode: bool,
    on_toggle_select: EventHandler<String>,
    on_select: EventHandler<Book>,
    on_read: EventHandler<Book>,
    #[props(default)] series_stack: Option<(String, usize)>,
    #[props(default)] on_quick_view: Option<EventHandler<Book>>,
    #[props(default = false)] show_label: bool,
) -> Element {
    let title = book.title.clone();
    let author = book.author.clone();
    let formats = book.formats.clone();
    let progress = book.progress_percent;
    let is_finished = book.is_finished;
    let book_id = book.id.clone();

    let is_audiobook_only =
        MediaType::Audiobooks.matches_book(&book) && !MediaType::Books.matches_book(&book);
    let cover_ratio = if is_audiobook_only {
        "aspect-ratio: 1 / 1;"
    } else {
        "aspect-ratio: 2 / 3;"
    };
    let cover_src = resolve_cover_src(&book_id, book.cover_path.as_deref(), true);

    rsx! {
        div {
            class: if is_selected { "book-card selected" } else { "book-card" },
            title: "{title} — {author}",
            style: if is_selected { "border-color: #818cf8; box-shadow: 0 0 12px rgba(129, 140, 248, 0.4); transform: scale(1.02);" } else { "" },
            onclick: {
                let book = book.clone();
                let b_id = book.id.clone();
                move |_e: Event<MouseData>| {
                    if selection_mode {
                        on_toggle_select.call(b_id.clone());
                    } else {
                        on_select.call(book.clone());
                    }
                }
            },
            ondoubleclick: {
                let book = book.clone();
                move |_| on_read.call(book.clone())
            },

            div { class: "cover-container", style: "{cover_ratio}",
                // Selection Checkbox Button
                div {
                    class: if is_selected { "card-select-btn selected" } else { "card-select-btn" },
                    style: if is_selected || selection_mode {
                        "position: absolute; top: 8px; left: 8px; width: 22px; height: 22px; border-radius: 50%; display: flex; align-items: center; justify-content: center; z-index: 5; cursor: pointer; background: #6366f1; border: 2px solid #fff; box-shadow: 0 2px 6px rgba(0,0,0,0.5); font-size: 11px; font-weight: bold; color: #fff;"
                    } else {
                        "position: absolute; top: 8px; left: 8px; width: 22px; height: 22px; border-radius: 50%; display: flex; align-items: center; justify-content: center; z-index: 5; cursor: pointer; background: rgba(0,0,0,0.5); border: 2px solid rgba(255,255,255,0.4); opacity: 0; transition: opacity 0.2s;"
                    },
                    onclick: {
                        let b_id = book_id.clone();
                        move |e: Event<MouseData>| {
                            e.stop_propagation();
                            on_toggle_select.call(b_id.clone());
                        }
                    },
                    if is_selected { "✓" } else { "" }
                }

                // Series stack badge OR Reading status badge (top-right, BookOrbit style)
                if let Some((_, count)) = series_stack {
                    div {
                        style: "position: absolute; top: 8px; right: 8px; padding: 2px 7px; border-radius: 999px; background: rgba(99, 102, 241, 0.92); color: #fff; font-size: 10px; font-weight: 700; display: flex; align-items: center; gap: 4px; z-index: 5; box-shadow: 0 2px 6px rgba(0,0,0,0.4);",
                        "{count} tomes"
                    }
                } else if is_finished {
                    div {
                        style: "position: absolute; top: 8px; right: 8px; padding: 2px 6px; border-radius: 999px; background: rgba(16, 185, 129, 0.9); color: #fff; font-size: 10px; font-weight: 700; display: flex; align-items: center; gap: 3px; z-index: 4; box-shadow: 0 2px 6px rgba(0,0,0,0.35);",
                        "✓"
                    }
                } else if progress > 0.0 {
                    div {
                        style: "position: absolute; top: 8px; right: 8px; padding: 2px 6px; border-radius: 999px; background: rgba(15, 23, 42, 0.82); border: 1px solid rgba(255,255,255,0.16); color: #e2e8f0; font-size: 10px; font-weight: 700; font-variant-numeric: tabular-nums; z-index: 4;",
                        "{progress.round() as u32}%"
                    }
                }

                if let Some(src) = cover_src {
                    img {
                        class: "cover-image",
                        src: "{src}",
                        alt: "{title}",
                        loading: "lazy",
                    }
                } else {
                    div { class: "cover-fallback",
                        span { class: "fallback-title", "{title}" }
                        span { class: "fallback-author", "{author}" }
                    }
                }

                // Format Badges (bottom-left, BookOrbit style)
                div {
                    class: "format-badges",
                    style: "top: auto; right: auto; bottom: 8px; left: 8px;",
                    for fmt in formats.iter().take(2) {
                        span { class: "format-pill", "{fmt.format.to_uppercase()}" }
                    }
                }

                // Hover quick-view & quick-read buttons (bottom-right, BookOrbit style)
                if let Some(qv) = on_quick_view {
                    button {
                        class: "book-card-play",
                        style: "right: 48px; background: rgba(15, 23, 42, 0.85);",
                        title: "Aperçu rapide",
                        onclick: {
                            let book = book.clone();
                            move |e: Event<MouseData>| {
                                e.stop_propagation();
                                qv.call(book.clone());
                            }
                        },
                        LucideIcon { kind: IconKind::Eye, size: 14 }
                    }
                }
                button {
                    class: "book-card-play",
                    title: "{title}",
                    onclick: {
                        let book = book.clone();
                        move |e: Event<MouseData>| {
                            e.stop_propagation();
                            on_read.call(book.clone());
                        }
                    },
                    if is_audiobook_only {
                        LucideIcon { kind: IconKind::Headphones, size: 15 }
                    } else {
                        LucideIcon { kind: IconKind::BookOpen, size: 15 }
                    }
                }

                if progress > 0.0 {
                    div {
                        style: "position: absolute; bottom: 0; left: 0; right: 0; height: 3px; background: rgba(0, 0, 0, 0.45);",
                        div {
                            class: "progress-bar",
                            style: "width: {progress.clamp(0.0, 100.0)}%;",
                        }
                    }
                }
            }

            if show_label {
                div { class: "book-card-meta",
                    div { class: "book-card-title", "{title}" }
                    div { class: "book-card-author",
                        if author.is_empty() { "—" } else { "{author}" }
                    }
                }
            }
        }
    }
}
