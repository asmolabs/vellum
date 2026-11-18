//! Aperçu rapide d'un livre (inspiré de BookQuickView dans BookOrbit).

use crate::components::{IconKind, LucideIcon};
use crate::cover_cache::resolve_cover_src;
use crate::library_logic::compute_metadata_score;
use dioxus::prelude::*;
use vellum_domain::Book;

#[component]
pub fn BookQuickView(
    book: Book,
    on_close: EventHandler<()>,
    on_read: EventHandler<Book>,
    on_open_details: EventHandler<Book>,
    #[props(default)] on_download: Option<EventHandler<Book>>,
) -> Element {
    let cover_src = resolve_cover_src(&book.id, book.cover_path.as_deref(), true);
    let (score, missing) = compute_metadata_score(&book);
    let score_color = if score >= 80 {
        "#34d399"
    } else if score >= 50 {
        "#fbbf24"
    } else {
        "#f87171"
    };

    rsx! {
        div {
            class: "modal-overlay",
            onclick: move |_| on_close.call(()),
            div {
                style: "width: min(640px, 94vw); background: var(--bg-card); border: 1px solid var(--border-active); border-radius: 14px; padding: 20px; box-shadow: 0 24px 60px rgba(0,0,0,0.55); display: flex; flex-direction: column; gap: 16px;",
                onclick: move |e| e.stop_propagation(),

                div { style: "display: flex; gap: 18px; align-items: flex-start;",
                    // Cover
                    div {
                        style: "width: 120px; flex-shrink: 0; aspect-ratio: 2 / 3; border-radius: 10px; overflow: hidden; background: #181b2c; border: 1px solid var(--border-subtle);",
                        if let Some(src) = cover_src {
                            img { src: "{src}", style: "width: 100%; height: 100%; object-fit: cover;" }
                        } else {
                            div { class: "cover-fallback",
                                span { class: "fallback-title", "{book.title}" }
                            }
                        }
                    }

                    // Details
                    div { style: "flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 8px;",
                        div { style: "display: flex; justify-content: space-between; align-items: flex-start; gap: 8px;",
                            div {
                                h3 { style: "font-size: 18px; font-weight: 700; margin: 0; color: var(--text-primary);", "{book.title}" }
                                div { style: "font-size: 13px; color: var(--text-secondary); margin-top: 2px;", "{book.author}" }
                            }
                            button {
                                style: "background: none; border: none; color: var(--text-muted); cursor: pointer; font-size: 16px; padding: 2px 6px;",
                                onclick: move |_| on_close.call(()),
                                "✕"
                            }
                        }

                        // Metadata pills: Series, Rating, Formats, Metadata Score
                        div { style: "display: flex; flex-wrap: wrap; gap: 6px; align-items: center;",
                            if let Some(ref s) = book.series {
                                if !s.trim().is_empty() {
                                    span {
                                        style: "padding: 2px 8px; border-radius: 6px; background: rgba(167, 139, 250, 0.16); color: #c4b5fd; font-size: 11px; font-weight: 600;",
                                        if let Some(idx) = book.series_index { "{s} #{idx}" } else { "{s}" }
                                    }
                                }
                            }
                            if let Some(r) = book.rating {
                                span { style: "padding: 2px 8px; border-radius: 6px; background: rgba(251, 191, 36, 0.16); color: #fde68a; font-size: 11px; font-weight: 600;",
                                    "★ {r:.1}"
                                }
                            }
                            for fmt in &book.formats {
                                span { class: "format-pill", "{fmt.format.to_uppercase()}" }
                            }
                            span {
                                title: if missing.is_empty() { "Métadonnées complètes".to_string() } else { format!("Manquant : {}", missing.join(", ")) },
                                style: "padding: 2px 8px; border-radius: 6px; background: rgba(255,255,255,0.06); font-size: 11px; font-weight: 600; color: {score_color};",
                                "Score {score}%"
                            }
                        }

                        // Synopsis
                        div {
                            style: "font-size: 12.5px; line-height: 1.5; color: var(--text-secondary); max-height: 130px; overflow-y: auto;",
                            if let Some(ref desc) = book.description {
                                if !desc.trim().is_empty() {
                                    "{desc}"
                                } else {
                                    span { style: "color: var(--text-muted); font-style: italic;", "Aucune description disponible." }
                                }
                            } else {
                                span { style: "color: var(--text-muted); font-style: italic;", "Aucune description disponible." }
                            }
                        }
                    }
                }

                // Footer actions
                div { style: "display: flex; justify-content: flex-end; gap: 10px; border-top: 1px solid var(--border-subtle); padding-top: 12px;",
                    if let Some(dl_cb) = on_download {
                        button {
                            class: "btn btn-secondary",
                            style: "display: inline-flex; align-items: center; gap: 6px; margin-right: auto;",
                            onclick: {
                                let b = book.clone();
                                move |_| dl_cb.call(b.clone())
                            },
                            LucideIcon { kind: IconKind::Download, size: 14 }
                            span { "Télécharger" }
                        }
                    }
                    button {
                        class: "btn btn-secondary",
                        onclick: {
                            let b = book.clone();
                            move |_| {
                                on_open_details.call(b.clone());
                                on_close.call(());
                            }
                        },
                        "Détails & Édition"
                    }
                    button {
                        class: "btn btn-primary",
                        style: "display: inline-flex; align-items: center; gap: 6px;",
                        onclick: {
                            let b = book.clone();
                            move |_| {
                                on_read.call(b.clone());
                                on_close.call(());
                            }
                        },
                        LucideIcon { kind: IconKind::BookOpen, size: 15 }
                        span { "Lire" }
                    }
                }
            }
        }
    }
}
