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

#[component]
pub fn AnnotationsView(
    books: Vec<Book>,
    data_dir: std::path::PathBuf,
    on_select: EventHandler<Book>,
    on_read: EventHandler<Book>,
) -> Element {
    use crate::components::{IconKind, LucideIcon};
    use std::collections::HashMap;
    use vellum_core::reader_store::{BookUserData, Highlight};

    let mut query = use_signal(String::new);
    let mut color_filter = use_signal(|| None::<String>);

    let book_map: HashMap<String, Book> = books.iter().map(|b| (b.id.clone(), b.clone())).collect();
    let all_highlights: Vec<Highlight> = BookUserData::load_all_highlights(&data_dir);
    let q_lower = query.read().trim().to_lowercase();
    let cur_color = color_filter.read().clone();

    let filtered: Vec<Highlight> = all_highlights
        .into_iter()
        .filter(|h| {
            if let Some(ref c) = cur_color {
                if &h.color != c {
                    return false;
                }
            }
            if !q_lower.is_empty() {
                let book_title = book_map
                    .get(&h.book_id)
                    .map(|b| b.title.to_lowercase())
                    .unwrap_or_default();
                let matches_text = h.selected_text.to_lowercase().contains(&q_lower)
                    || h.note
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&q_lower)
                    || book_title.contains(&q_lower);
                if !matches_text {
                    return false;
                }
            }
            true
        })
        .collect();

    rsx! {
        div { style: "padding: 8px 24px 32px; display: flex; flex-direction: column; gap: 14px;",
            div { style: "display: flex; align-items: center; justify-content: space-between; gap: 12px; flex-wrap: wrap;",
                div { style: "display: flex; align-items: center; gap: 10px;",
                    span {
                        style: "width: 30px; height: 30px; border-radius: 8px; background: rgba(250, 204, 21, 0.16); color: #facc15; display: inline-flex; align-items: center; justify-content: center;",
                        LucideIcon { kind: IconKind::Highlighter, size: 16 }
                    }
                    h2 { style: "font-size: 20px; font-weight: 700; margin: 0;", "Cahier d'annotations ({filtered.len()})" }
                }

                div { style: "display: flex; align-items: center; gap: 8px;",
                    input {
                        r#type: "text",
                        value: "{query}",
                        placeholder: "Filtrer surlignages, notes ou livres...",
                        style: "width: 240px; padding: 6px 10px; font-size: 12.5px; border-radius: 8px; background: rgba(255,255,255,0.04); color: var(--text-primary); border: 1px solid var(--border-subtle); outline: none;",
                        oninput: move |e| query.set(e.value()),
                    }
                    for (label, val) in [
                        ("Toutes", None),
                        ("Jaune", Some("yellow")),
                        ("Vert", Some("green")),
                        ("Bleu", Some("blue")),
                        ("Rose", Some("pink")),
                    ] {
                        {
                            let is_on = cur_color.as_deref() == val;
                            rsx! {
                                button {
                                    style: if is_on {
                                        "padding: 5px 10px; border-radius: 7px; background: rgba(99, 102, 241, 0.25); color: #c7d2fe; border: none; font-size: 12px; font-weight: 600; cursor: pointer;"
                                    } else {
                                        "padding: 5px 10px; border-radius: 7px; background: rgba(255,255,255,0.04); color: var(--text-muted); border: 1px solid var(--border-subtle); font-size: 12px; cursor: pointer;"
                                    },
                                    onclick: move |_| color_filter.set(val.map(str::to_string)),
                                    "{label}"
                                }
                            }
                        }
                    }
                }
            }

            if filtered.is_empty() {
                div { style: "padding: 48px; text-align: center; color: var(--text-muted); background: rgba(255,255,255,0.02); border-radius: 12px; border: 1px dashed var(--border-subtle);",
                    "Aucun surlignage ou note pour le moment. Sélectionnez du texte dans le lecteur pour créer des annotations."
                }
            } else {
                div { style: "display: flex; flex-direction: column; gap: 10px;",
                    for h in filtered {
                        {
                            let book_opt = book_map.get(&h.book_id).cloned();
                            let book_title = book_opt.as_ref().map(|b| b.title.clone()).unwrap_or_else(|| "Livre".to_string());
                            let book_author = book_opt.as_ref().map(|b| b.author.clone()).unwrap_or_default();
                            let bg = h.color_rgba();
                            rsx! {
                                div {
                                    key: "ann-{h.id}",
                                    style: "padding: 14px 16px; border-radius: 10px; background: rgba(255,255,255,0.03); border: 1px solid var(--border-subtle); display: flex; flex-direction: column; gap: 8px;",
                                    div { style: "display: flex; justify-content: space-between; align-items: center; gap: 8px;",
                                        div {
                                            style: "font-size: 12.5px; font-weight: 600; color: #a5b4fc; cursor: pointer;",
                                            onclick: {
                                                let b = book_opt.clone();
                                                move |_| {
                                                    if let Some(ref book) = b {
                                                        on_select.call(book.clone());
                                                    }
                                                }
                                            },
                                            "{book_title} — {book_author}"
                                        }
                                        div { style: "display: flex; align-items: center; gap: 8px;",
                                            span { style: "font-size: 11px; color: var(--text-muted);", "{h.chapter_title}" }
                                            if let Some(book) = book_opt.clone() {
                                                button {
                                                    class: "btn btn-secondary",
                                                    style: "padding: 3px 8px; font-size: 11px;",
                                                    onclick: move |_| on_read.call(book.clone()),
                                                    "Ouvrir"
                                                }
                                            }
                                        }
                                    }
                                    blockquote {
                                        style: "margin: 0; padding: 8px 12px; border-radius: 6px; background: {bg}; font-size: 13px; line-height: 1.5; color: var(--text-primary);",
                                        "{h.selected_text}"
                                    }
                                    if let Some(ref note) = h.note {
                                        if !note.trim().is_empty() {
                                            div { style: "font-size: 12px; color: var(--text-secondary); padding-left: 8px; border-left: 2px solid var(--border-active);",
                                                "Note : {note}"
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
}
