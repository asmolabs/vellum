//! Écrans dédiés inspirés de BookOrbit : Auteurs, Séries/Sagas, Historique de lecture et Annotations.

use std::collections::{BTreeMap, HashMap};

use dioxus::prelude::*;
use vellum_core::reader_store::{BookUserData, Highlight};
use vellum_core::{translate, Locale};
use vellum_domain::Book;

use crate::components::{IconKind, LucideIcon};
use crate::cover_cache::resolve_cover_src;
use crate::state::ViewMode;

#[component]
pub fn AuthorsView(
    locale: Locale,
    books: Vec<Book>,
    mut search_query: Signal<String>,
    mut view_mode: Signal<ViewMode>,
) -> Element {
    let mut local_q = use_signal(String::new);
    let mut sort_by_count = use_signal(|| false);
    let mut active_letter = use_signal(|| None::<char>);

    let mut by_author: BTreeMap<String, Vec<Book>> = BTreeMap::new();
    for b in books {
        let key = if b.author.trim().is_empty() {
            "Inconnu".to_string()
        } else {
            b.author.trim().to_string()
        };
        by_author.entry(key).or_default().push(b);
    }

    let q_lower = local_q.read().trim().to_lowercase();
    let letter_filter = *active_letter.read();
    let mut list: Vec<(String, Vec<Book>)> = by_author
        .into_iter()
        .filter(|(a, _)| {
            if !q_lower.is_empty() && !a.to_lowercase().contains(&q_lower) {
                return false;
            }
            if let Some(ch) = letter_filter {
                let first = a
                    .chars()
                    .next()
                    .map(|c| c.to_ascii_uppercase())
                    .unwrap_or('#');
                if first != ch {
                    return false;
                }
            }
            true
        })
        .collect();

    if *sort_by_count.read() {
        list.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then_with(|| a.0.cmp(&b.0)));
    }

    rsx! {
        div { style: "padding: 4px 4px 32px; display: flex; flex-direction: column; gap: 16px;",
            // BrowseHeader (BookOrbit style)
            div { class: "library-header",
                div { class: "browse-title-row",
                    div { class: "library-title-group",
                        div { class: "browse-eyebrow", {translate(locale, "nav.browse")} }
                        div { style: "display: flex; align-items: center; gap: 10px;",
                            span {
                                style: "width: 30px; height: 30px; border-radius: 8px; background: var(--accent-glow); color: var(--accent-primary); display: inline-flex; align-items: center; justify-content: center;",
                                LucideIcon { kind: IconKind::Users, size: 16 }
                            }
                            h2 { "{translate(locale, \"browse.authors_title\")}" }
                            span { class: "library-count", style: "margin-top: 0;", "{list.len()}" }
                        }
                    }
                }

                div { class: "browse-command-bar",
                    div {
                        style: "flex: 1; min-width: 180px; display: flex; align-items: center; gap: 8px; padding: 0 8px; height: 32px; border-radius: 7px; background: var(--bg-primary);",
                        span { style: "color: var(--text-muted); display: flex; align-items: center;",
                            LucideIcon { kind: IconKind::Search, size: 14 }
                        }
                        input {
                            r#type: "text",
                            value: "{local_q}",
                            placeholder: translate(locale, "action.search"),
                            style: "flex: 1; background: transparent; border: none; outline: none; color: var(--text-primary); font-size: 13px;",
                            oninput: move |e| local_q.set(e.value()),
                        }
                        if !local_q.read().is_empty() {
                            button {
                                style: "background: none; border: none; color: var(--text-muted); cursor: pointer;",
                                onclick: move |_| local_q.set(String::new()),
                                "✕"
                            }
                        }
                    }

                    // A-Z JumpRail pills
                    div { style: "display: flex; align-items: center; gap: 2px; flex-wrap: wrap;",
                        button {
                            style: if letter_filter.is_none() {
                                "padding: 3px 7px; font-size: 11px; border-radius: 5px; background: var(--accent-primary); color: #fff; border: none; font-weight: 700; cursor: pointer;"
                            } else {
                                "padding: 3px 7px; font-size: 11px; border-radius: 5px; background: transparent; color: var(--text-muted); border: none; cursor: pointer;"
                            },
                            onclick: move |_| active_letter.set(None),
                            "Tous"
                        }
                        for ch in 'A'..='Z' {
                            button {
                                style: if letter_filter == Some(ch) {
                                    "width: 22px; height: 22px; font-size: 10.5px; border-radius: 5px; background: var(--accent-primary); color: #fff; border: none; font-weight: 700; cursor: pointer;"
                                } else {
                                    "width: 22px; height: 22px; font-size: 10.5px; border-radius: 5px; background: transparent; color: var(--text-muted); border: none; cursor: pointer;"
                                },
                                onclick: move |_| {
                                    if letter_filter == Some(ch) {
                                        active_letter.set(None);
                                    } else {
                                        active_letter.set(Some(ch));
                                    }
                                },
                                "{ch}"
                            }
                        }
                    }

                    select {
                        style: "height: 30px; padding: 0 10px; font-size: 12px; border-radius: 7px; background: var(--bg-primary); color: var(--text-primary); border: 1px solid var(--border-subtle); outline: none; cursor: pointer;",
                        value: if *sort_by_count.read() { "count" } else { "name" },
                        onchange: move |e| sort_by_count.set(e.value() == "count"),
                        option { value: "name", "Nom (A→Z)" }
                        option { value: "count", "Nombre de livres" }
                    }
                }
            }

            div { style: "display: grid; grid-template-columns: repeat(auto-fill, minmax(220px, 1fr)); gap: 14px;",
                for (author, author_books) in list {
                    {
                        let author_q = author.clone();
                        let count = author_books.len();
                        let cover = author_books
                            .iter()
                            .find_map(|b| resolve_cover_src(&b.id, b.cover_path.as_deref(), false));
                        rsx! {
                            div {
                                key: "auth-{author}",
                                style: "display: flex; align-items: center; gap: 12px; padding: 12px 14px; border-radius: 12px; background: var(--bg-card); border: 1px solid var(--border-subtle); cursor: pointer; transition: transform 0.15s, border-color 0.15s;",
                                onclick: move |_| {
                                    search_query.set(format!("author:\"{author_q}\""));
                                    view_mode.set(ViewMode::Grid);
                                },
                                div {
                                    style: "width: 46px; height: 46px; border-radius: 50%; overflow: hidden; background: var(--accent-glow); border: 1px solid var(--border-subtle); display: flex; align-items: center; justify-content: center; flex-shrink: 0;",
                                    if let Some(src) = cover {
                                        img { src: "{src}", style: "width: 100%; height: 100%; object-fit: cover;" }
                                    } else {
                                        span { style: "font-size: 16px; font-weight: 700; color: var(--accent-primary);", "{author.chars().next().unwrap_or('A')}" }
                                    }
                                }
                                div { style: "min-width: 0; flex: 1;",
                                    div { style: "font-size: 13.5px; font-weight: 600; color: var(--text-primary); white-space: nowrap; overflow: hidden; text-overflow: ellipsis;", "{author}" }
                                    div { style: "font-size: 11.5px; color: var(--text-muted);", "{count} livre(s)" }
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
    let mut local_q = use_signal(String::new);
    let mut sort_by_count = use_signal(|| false);

    let mut by_series: BTreeMap<String, Vec<Book>> = BTreeMap::new();
    for b in books {
        if let Some(ref s) = b.series {
            let trimmed = s.trim();
            if !trimmed.is_empty() {
                by_series.entry(trimmed.to_string()).or_default().push(b);
            }
        }
    }

    let q_lower = local_q.read().trim().to_lowercase();
    let mut series_list: Vec<(String, Vec<Book>)> = by_series
        .into_iter()
        .filter(|(s, _)| q_lower.is_empty() || s.to_lowercase().contains(&q_lower))
        .collect();

    if *sort_by_count.read() {
        series_list.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then_with(|| a.0.cmp(&b.0)));
    }

    rsx! {
        div { style: "padding: 4px 4px 32px; display: flex; flex-direction: column; gap: 16px;",
            div { class: "library-header",
                div { class: "browse-title-row",
                    div { class: "library-title-group",
                        div { class: "browse-eyebrow", {translate(locale, "nav.browse")} }
                        div { style: "display: flex; align-items: center; gap: 10px;",
                            span {
                                style: "width: 30px; height: 30px; border-radius: 8px; background: var(--accent-glow); color: var(--accent-primary); display: inline-flex; align-items: center; justify-content: center;",
                                LucideIcon { kind: IconKind::Layers, size: 16 }
                            }
                            h2 { "{translate(locale, \"browse.series_title\")}" }
                            span { class: "library-count", style: "margin-top: 0;", "{series_list.len()}" }
                        }
                    }
                }

                div { class: "browse-command-bar",
                    div {
                        style: "flex: 1; min-width: 180px; display: flex; align-items: center; gap: 8px; padding: 0 8px; height: 32px; border-radius: 7px; background: var(--bg-primary);",
                        span { style: "color: var(--text-muted); display: flex; align-items: center;",
                            LucideIcon { kind: IconKind::Search, size: 14 }
                        }
                        input {
                            r#type: "text",
                            value: "{local_q}",
                            placeholder: translate(locale, "action.search"),
                            style: "flex: 1; background: transparent; border: none; outline: none; color: var(--text-primary); font-size: 13px;",
                            oninput: move |e| local_q.set(e.value()),
                        }
                    }

                    select {
                        style: "height: 30px; padding: 0 10px; font-size: 12px; border-radius: 7px; background: var(--bg-primary); color: var(--text-primary); border: 1px solid var(--border-subtle); outline: none; cursor: pointer;",
                        value: if *sort_by_count.read() { "count" } else { "name" },
                        onchange: move |e| sort_by_count.set(e.value() == "count"),
                        option { value: "name", "Nom (A→Z)" }
                        option { value: "count", "Nombre de tomes" }
                    }
                }
            }

            if series_list.is_empty() {
                div { style: "padding: 40px; text-align: center; color: var(--text-muted);",
                    "{translate(locale, \"browse.series_empty\")}"
                }
            } else {
                div { style: "display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: 14px;",
                    for (series_name, list) in series_list {
                        {
                            let s_clone = series_name.clone();
                            let count = list.len();
                            let finished_count = list.iter().filter(|b| b.is_finished).count();
                            let pct = if count > 0 { (finished_count * 100) / count } else { 0 };
                            let covers: Vec<String> = list
                                .iter()
                                .filter_map(|b| resolve_cover_src(&b.id, b.cover_path.as_deref(), false))
                                .take(3)
                                .collect();
                            rsx! {
                                div {
                                    key: "ser-{series_name}",
                                    style: "display: flex; gap: 14px; padding: 14px; border-radius: 12px; background: var(--bg-card); border: 1px solid var(--border-subtle); cursor: pointer; align-items: center;",
                                    onclick: move |_| {
                                        selected_series.set(Some(s_clone.clone()));
                                        view_mode.set(ViewMode::Grid);
                                    },
                                    // BookOrbit-style 3-cover stacked fan
                                    div { style: "position: relative; width: 68px; height: 80px; flex-shrink: 0; display: flex; align-items: center;",
                                        if covers.is_empty() {
                                            div {
                                                style: "width: 54px; height: 78px; border-radius: 6px; background: var(--accent-glow); border: 1px solid var(--border-subtle); display: flex; align-items: center; justify-content: center; color: var(--accent-primary);",
                                                LucideIcon { kind: IconKind::Layers, size: 20 }
                                            }
                                        } else {
                                            for (idx, src) in covers.iter().enumerate() {
                                                img {
                                                    src: "{src}",
                                                    style: "position: absolute; left: {idx * 8}px; z-index: {10 - idx}; width: 52px; height: 76px; border-radius: 6px; object-fit: cover; border: 1px solid var(--border-subtle); box-shadow: 0 4px 10px rgba(0,0,0,0.3);",
                                                }
                                            }
                                        }
                                    }
                                    div { style: "flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 6px;",
                                        div { style: "font-size: 14px; font-weight: 700; color: var(--text-primary); white-space: nowrap; overflow: hidden; text-overflow: ellipsis;", "{series_name}" }
                                        div { style: "font-size: 11.5px; color: var(--text-muted);", "{count} vol. • {finished_count} terminé(s)" }
                                        div { style: "height: 5px; background: var(--border-subtle); border-radius: 4px; overflow: hidden;",
                                            div { style: "width: {pct}%; height: 100%; background: var(--accent-primary);" }
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
    let mut local_q = use_signal(String::new);
    let q_lower = local_q.read().trim().to_lowercase();

    let mut history: Vec<Book> = books
        .into_iter()
        .filter(|b| b.last_read_at.is_some() || b.progress_percent > 0.0 || b.is_finished)
        .filter(|b| {
            q_lower.is_empty()
                || b.title.to_lowercase().contains(&q_lower)
                || b.author.to_lowercase().contains(&q_lower)
        })
        .collect();
    history.sort_by(|a, b| b.last_read_at.cmp(&a.last_read_at));

    rsx! {
        div { style: "padding: 4px 4px 32px; display: flex; flex-direction: column; gap: 16px;",
            div { class: "library-header",
                div { class: "browse-title-row",
                    div { class: "library-title-group",
                        div { class: "browse-eyebrow", {translate(locale, "nav.reading_zone")} }
                        div { style: "display: flex; align-items: center; gap: 10px;",
                            span {
                                style: "width: 30px; height: 30px; border-radius: 8px; background: var(--accent-glow); color: var(--accent-primary); display: inline-flex; align-items: center; justify-content: center;",
                                LucideIcon { kind: IconKind::History, size: 16 }
                            }
                            h2 { "{translate(locale, \"browse.history_title\")}" }
                            span { class: "library-count", style: "margin-top: 0;", "{history.len()}" }
                        }
                    }
                }

                div { class: "browse-command-bar",
                    div {
                        style: "flex: 1; min-width: 180px; display: flex; align-items: center; gap: 8px; padding: 0 8px; height: 32px; border-radius: 7px; background: var(--bg-primary);",
                        span { style: "color: var(--text-muted); display: flex; align-items: center;",
                            LucideIcon { kind: IconKind::Search, size: 14 }
                        }
                        input {
                            r#type: "text",
                            value: "{local_q}",
                            placeholder: translate(locale, "action.search"),
                            style: "flex: 1; background: transparent; border: none; outline: none; color: var(--text-primary); font-size: 13px;",
                            oninput: move |e| local_q.set(e.value()),
                        }
                    }
                }
            }

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
                            let cover_src = resolve_cover_src(&b.id, b.cover_path.as_deref(), true);
                            rsx! {
                                div {
                                    key: "hist-{b.id}",
                                    style: "display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 12px 16px; border-radius: 12px; background: var(--bg-card); border: 1px solid var(--border-subtle);",
                                    div {
                                        style: "display: flex; align-items: center; gap: 12px; flex: 1; min-width: 0; cursor: pointer;",
                                        onclick: move |_| on_select.call(b_sel.clone()),
                                        div {
                                            style: "width: 36px; height: 52px; border-radius: 6px; overflow: hidden; background: var(--bg-surface); border: 1px solid var(--border-subtle); flex-shrink: 0; display: flex; align-items: center; justify-content: center;",
                                            if let Some(src) = cover_src {
                                                img { src: "{src}", style: "width: 100%; height: 100%; object-fit: cover;" }
                                            } else {
                                                LucideIcon { kind: IconKind::BookOpen, size: 15 }
                                            }
                                        }
                                        div { style: "min-width: 0; flex: 1;",
                                            div { style: "font-size: 13.5px; font-weight: 600; color: var(--text-primary); white-space: nowrap; overflow: hidden; text-overflow: ellipsis;", "{b.title}" }
                                            div { style: "font-size: 11.5px; color: var(--text-muted);", "{b.author} • {date_str}" }
                                        }
                                    }
                                    div { style: "display: flex; align-items: center; gap: 12px;",
                                        span { style: "font-size: 12px; font-weight: 700; color: var(--accent-primary);", "{pct}%" }
                                        button {
                                            class: "btn btn-primary",
                                            style: "font-size: 11.5px; padding: 5px 12px;",
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
        div { style: "padding: 4px 4px 32px; display: flex; flex-direction: column; gap: 16px;",
            div { class: "library-header",
                div { class: "browse-title-row",
                    div { class: "library-title-group",
                        div { class: "browse-eyebrow", "Lecture" }
                        div { style: "display: flex; align-items: center; gap: 10px;",
                            span {
                                style: "width: 30px; height: 30px; border-radius: 8px; background: rgba(250, 204, 21, 0.16); color: #facc15; display: inline-flex; align-items: center; justify-content: center;",
                                LucideIcon { kind: IconKind::Highlighter, size: 16 }
                            }
                            h2 { "Cahier d'annotations" }
                            span { class: "library-count", style: "margin-top: 0;", "{filtered.len()}" }
                        }
                    }
                }

                div { class: "browse-command-bar",
                    div {
                        style: "flex: 1; min-width: 180px; display: flex; align-items: center; gap: 8px; padding: 0 8px; height: 32px; border-radius: 7px; background: var(--bg-primary);",
                        span { style: "color: var(--text-muted); display: flex; align-items: center;",
                            LucideIcon { kind: IconKind::Search, size: 14 }
                        }
                        input {
                            r#type: "text",
                            value: "{query}",
                            placeholder: "Filtrer surlignages, notes ou livres...",
                            style: "flex: 1; background: transparent; border: none; outline: none; color: var(--text-primary); font-size: 13px;",
                            oninput: move |e| query.set(e.value()),
                        }
                    }
                    div { style: "display: flex; align-items: center; gap: 4px;",
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
                                            "padding: 4px 9px; border-radius: 6px; background: var(--accent-primary); color: #fff; border: none; font-size: 11.5px; font-weight: 600; cursor: pointer;"
                                        } else {
                                            "padding: 4px 9px; border-radius: 6px; background: var(--bg-primary); color: var(--text-muted); border: 1px solid var(--border-subtle); font-size: 11.5px; cursor: pointer;"
                                        },
                                        onclick: move |_| color_filter.set(val.map(str::to_string)),
                                        "{label}"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if filtered.is_empty() {
                div { style: "padding: 48px; text-align: center; color: var(--text-muted); background: var(--bg-card); border-radius: 12px; border: 1px dashed var(--border-subtle);",
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
                                    style: "padding: 14px 16px; border-radius: 12px; background: var(--bg-card); border: 1px solid var(--border-subtle); display: flex; flex-direction: column; gap: 8px;",
                                    div { style: "display: flex; justify-content: space-between; align-items: center; gap: 8px;",
                                        div {
                                            style: "font-size: 12.5px; font-weight: 600; color: var(--accent-primary); cursor: pointer;",
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
