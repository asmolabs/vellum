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
) -> Element {
    let title = book.title.clone();
    let author = book.author.clone();
    let formats = book.formats.clone();
    let progress = book.progress_percent;
    let cover_path = book.cover_path.clone();
    let book_id = book.id.clone();

    use base64::Engine;
    let cover_src = cover_path.as_ref().and_then(|p| {
        if p.starts_with("data:") || p.starts_with("http") {
            Some(p.clone())
        } else {
            let full_path = if std::path::Path::new(p).is_absolute() {
                std::path::PathBuf::from(p)
            } else {
                dirs::home_dir().map(|h| h.join(".vellum").join("library").join(p))?
            };

            if full_path.exists() {
                if let Ok(bytes) = std::fs::read(&full_path) {
                    let ext = full_path
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("jpeg")
                        .to_lowercase();
                    let mime = match ext.as_str() {
                        "png" => "image/png",
                        "webp" => "image/webp",
                        _ => "image/jpeg",
                    };
                    return Some(format!(
                        "data:{mime};base64,{}",
                        base64::engine::general_purpose::STANDARD.encode(&bytes)
                    ));
                }
            }
            None
        }
    });

    rsx! {
        div {
            class: if is_selected { "book-card selected" } else { "book-card" },
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

            div { class: "cover-container",
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

                // Format Badges
                div { class: "format-badges",
                    for fmt in formats.iter() {
                        span { class: "format-pill", "{fmt.format.to_uppercase()}" }
                    }
                }
            }

            div { class: "card-body",
                div { class: "book-title", title: "{title}", "{title}" }

                if let Some(ref s) = book.series {
                    {
                        let series_str = if let Some(idx) = book.series_index {
                            if idx.fract() == 0.0 {
                                format!("📚 {s} • T.{}", idx as i64)
                            } else {
                                format!("📚 {s} • T.{idx}")
                            }
                        } else {
                            format!("📚 {s}")
                        };
                        rsx! {
                            div {
                                style: "font-size: 11px; font-weight: 600; color: #a78bfa; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;",
                                title: "{series_str}",
                                "{series_str}"
                            }
                        }
                    }
                }

                div { class: "book-author", title: "{author}", "{author}" }

                if progress > 0.0 {
                    div { class: "progress-track",
                        div {
                            class: "progress-bar",
                            style: "width: {progress.clamp(0.0, 100.0)}%;",
                        }
                    }
                }
            }
        }
    }
}
