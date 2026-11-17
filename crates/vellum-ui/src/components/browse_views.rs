//! Écrans dédiés inspirés de BookOrbit : Auteurs, Séries/Sagas et Historique de lecture.

use std::collections::BTreeMap;

use dioxus::prelude::*;
use vellum_core::{translate, Locale};
use vellum_domain::Book;

use crate::cover_cache::resolve_cover_src;
use crate::state::ViewMode;

#[component]
pub fn AuthorsView(
    locale: Locale,
    books: Vec<Book>,
    mut search_query: Signal<String>,
    mut view_mode: Signal<ViewMode>,
) -> Element {
    let mut by_author: BTreeMap<String, Vec<Book>> = BTreeMap::new();
    for b in books {
        let key = if b.author.trim().is_empty() {
            "Inconnu".to_string()
        } else {
            b.author.trim().to_string()
        };
        by_author.entry(key).or_default().push(b);
    }

    rsx! {
        div { style: "padding: 8px 24px 32px; display: flex; flex-direction: column; gap: 16px;",
            h2 { style: "font-size: 18px; font-weight: 700; margin: 0;", "{translate(locale, \"browse.authors_title\")}" }
            div { style: "display: grid; grid-template-columns: repeat(auto-fill, minmax(220px, 1fr)); gap: 12px;",
                for (author, list) in by_author {
                    {
                        let author_q = author.clone();
                        let count = list.len();
                        let cover = list
                            .iter()
                            .find_map(|b| resolve_cover_src(&b.id, b.cover_path.as_deref(), false));
                        rsx! {
                            div {
                                key: "auth-{author}",
                                style: "display: flex; align-items: center; gap: 12px; padding: 12px; border-radius: 12px; background: rgba(255,255,255,0.03); border: 1px solid var(--border-subtle); cursor: pointer;",
                                onclick: move |_| {
                                    search_query.set(format!("author:\"{author_q}\""));
                                    view_mode.set(ViewMode::Grid);
                                },
                                div { style: "width: 44px; height: 44px; border-radius: 50%; overflow: hidden; background: #1e293b; display: flex; align-items: center; justify-content: center; flex-shrink: 0;",
                                    if let Some(src) = cover {
                                        img { src: "{src}", style: "width: 100%; height: 100%; object-fit: cover;" }
                                    } else {
                                        span { style: "font-size: 16px; font-weight: 700; color: #a5b4fc;", "{author.chars().next().unwrap_or('A')}" }
                                    }
                                }
                                div { style: "min-width: 0; flex: 1;",
                                    div { style: "font-size: 13px; font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;", "{author}" }
                                    div { style: "font-size: 11px; color: var(--text-muted);", "{count} livre(s)" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn SeriesView(
    locale: Locale,
    books: Vec<Book>,
    mut selected_series: Signal<Option<String>>,
    mut view_mode: Signal<ViewMode>,
) -> Element {
    let mut by_series: BTreeMap<String, Vec<Book>> = BTreeMap::new();
    for b in books {
        if let Some(ref s) = b.series {
            let trimmed = s.trim();
            if !trimmed.is_empty() {
                by_series.entry(trimmed.to_string()).or_default().push(b);
            }
        }
    }

    rsx! {
        div { style: "padding: 8px 24px 32px; display: flex; flex-direction: column; gap: 16px;",
            h2 { style: "font-size: 18px; font-weight: 700; margin: 0;", "{translate(locale, \"browse.series_title\")}" }
            if by_series.is_empty() {
                div { style: "padding: 40px; text-align: center; color: var(--text-muted);",
                    "{translate(locale, \"browse.series_empty\")}"
                }
            } else {
                div { style: "display: grid; grid-template-columns: repeat(auto-fill, minmax(240px, 1fr)); gap: 14px;",
                    for (series_name, list) in by_series {
                        {
                            let s_clone = series_name.clone();
                            let count = list.len();
                            let finished_count = list.iter().filter(|b| b.is_finished).count();
                            let pct = if count > 0 { (finished_count * 100) / count } else { 0 };
                            let cover = list
                                .iter()
                                .find_map(|b| resolve_cover_src(&b.id, b.cover_path.as_deref(), false));
                            rsx! {
                                div {
                                    key: "ser-{series_name}",
                                    style: "display: flex; gap: 12px; padding: 12px; border-radius: 12px; background: rgba(255,255,255,0.03); border: 1px solid var(--border-subtle); cursor: pointer; align-items: center;",
                                    onclick: move |_| {
                                        selected_series.set(Some(s_clone.clone()));
                                        view_mode.set(ViewMode::Grid);
                                    },
                                    div { style: "width: 52px; height: 76px; border-radius: 6px; overflow: hidden; background: #1e293b; flex-shrink: 0; display: flex; align-items: center; justify-content: center;",
                                        if let Some(src) = cover {
                                            img { src: "{src}", style: "width: 100%; height: 100%; object-fit: cover;" }
                                        } else {
                                            span { style: "font-size: 20px;", "📚" }
                                        }
                                    }
                                    div { style: "flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 6px;",
                                        div { style: "font-size: 13px; font-weight: 700; color: #c4b5fd; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;", "{series_name}" }
                                        div { style: "font-size: 11px; color: var(--text-muted);", "{count} vol. • {finished_count} terminé(s)" }
                                        div { style: "height: 4px; background: rgba(255,255,255,0.1); border-radius: 4px; overflow: hidden;",
                                            div { style: "width: {pct}%; height: 100%; background: #8b5cf6;" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn ReadingHistoryView(
    locale: Locale,
    books: Vec<Book>,
    on_select: EventHandler<Book>,
    on_read: EventHandler<Book>,
) -> Element {
    let mut history: Vec<Book> = books
        .into_iter()
        .filter(|b| b.last_read_at.is_some() || b.progress_percent > 0.0 || b.is_finished)
        .collect();
    history.sort_by(|a, b| b.last_read_at.cmp(&a.last_read_at));

    rsx! {
        div { style: "padding: 8px 24px 32px; display: flex; flex-direction: column; gap: 14px;",
            h2 { style: "font-size: 18px; font-weight: 700; margin: 0;", "{translate(locale, \"browse.history_title\")}" }
            if history.is_empty() {
                div { style: "padding: 40px; text-align: center; color: var(--text-muted);",
                    "{translate(locale, \"browse.history_empty\")}"
                }
            } else {
                div { style: "display: flex; flex-direction: column; gap: 10px;",
                    for b in history {
                        {
                            let b_sel = b.clone();
                            let b_read = b.clone();
                            let pct = b.progress_percent.round() as i32;
                            let date_str = b.last_read_at.as_deref().unwrap_or("-");
                            rsx! {
                                div {
                                    key: "hist-{b.id}",
                                    style: "display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 12px 16px; border-radius: 10px; background: rgba(255,255,255,0.03); border: 1px solid var(--border-subtle);",
                                    div {
                                        style: "flex: 1; min-width: 0; cursor: pointer;",
                                        onclick: move |_| on_select.call(b_sel.clone()),
                                        div { style: "font-size: 13px; font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;", "{b.title}" }
                                        div { style: "font-size: 11px; color: var(--text-muted);", "{b.author} • {date_str}" }
                                    }
                                    div { style: "display: flex; align-items: center; gap: 12px;",
                                        span { style: "font-size: 12px; font-weight: 600; color: #a5b4fc;", "{pct}%" }
                                        button {
                                            class: "btn btn-primary",
                                            style: "font-size: 11px; padding: 5px 12px;",
                                            onclick: move |_| on_read.call(b_read.clone()),
                                            "{translate(locale, \"home.resume_btn\")}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
