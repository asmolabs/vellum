use std::collections::BTreeSet;

use dioxus::prelude::*;
use vellum_core::{translate, translate_fmt, Locale};
use vellum_domain::Book;

#[derive(Clone, PartialEq)]
pub enum PaletteAction {
    SelectBook(Book),
    ReadBook(Book),
    FilterAuthor(String),
    FilterSeries(String),
    FilterAll,
    FilterReading,
    FilterFinished,
    FilterToRead,
    SetViewGrid,
    SetViewTable,
    SetViewTableCarousel,
    SetViewKanban,
    ImportBooks,
    ImportCalibre,
    OpenNewsRecipes,
    OpenLibraryAudit,
    OpenReadingStats,
    OpenContentSearch,
    OpenBulkRegex,
    OpenImportRules,
    OpenWebCompanion,
    SyncCloud,
    OpenSettings,
}

#[component]
pub fn CommandPalette(
    locale: Locale,
    books: Vec<Book>,
    series_list: Vec<String>,
    on_close: EventHandler<()>,
    on_action: EventHandler<PaletteAction>,
) -> Element {
    let mut query = use_signal(String::new);
    let mut active_tab = use_signal(|| "all");

    let q = query.read().trim().to_lowercase();
    let tab = *active_tab.read();
    let book_limit = if tab == "books" { 20 } else { 5 };
    let entity_limit = if tab == "all" { 3 } else { 20 };

    // Continue reading (when query is empty, BookOrbit-style)
    let continue_reading: Vec<Book> = books
        .iter()
        .filter(|b| !b.is_finished && b.progress_percent > 0.0)
        .take(4)
        .cloned()
        .collect();

    // Matching books
    let matched_books: Vec<Book> = if q.is_empty() {
        Vec::new()
    } else {
        books
            .iter()
            .filter(|b| {
                b.title.to_lowercase().contains(&q)
                    || b.author.to_lowercase().contains(&q)
                    || b.series
                        .as_deref()
                        .unwrap_or_default()
                        .to_lowercase()
                        .contains(&q)
                    || b.tags.iter().any(|t| t.name.to_lowercase().contains(&q))
            })
            .take(book_limit)
            .cloned()
            .collect()
    };

    // Matching authors
    let matched_authors: Vec<String> = if q.is_empty() && tab != "authors" {
        Vec::new()
    } else {
        let mut set = BTreeSet::new();
        for b in &books {
            let a = b.author.trim();
            if !a.is_empty() && (q.is_empty() || a.to_lowercase().contains(&q)) {
                set.insert(a.to_string());
            }
        }
        set.into_iter().take(entity_limit).collect()
    };

    // Matching series
    let matched_series: Vec<String> = if q.is_empty() && tab != "series" {
        Vec::new()
    } else {
        series_list
            .iter()
            .filter(|s| q.is_empty() || s.to_lowercase().contains(&q))
            .take(entity_limit)
            .cloned()
            .collect()
    };

    // Quick built-in actions
    let all_actions = [
        (
            translate(locale, "palette.act_grid"),
            PaletteAction::SetViewGrid,
            "vue grille grid affichage",
        ),
        (
            translate(locale, "palette.act_table"),
            PaletteAction::SetViewTable,
            "vue tableau liste table calibre",
        ),
        (
            translate(locale, "palette.act_carousel"),
            PaletteAction::SetViewTableCarousel,
            "vue tableau carrousel cover browser calibre",
        ),
        (
            translate(locale, "palette.act_kanban"),
            PaletteAction::SetViewKanban,
            "vue kanban pal pile a lire tableau trello colonnes",
        ),
        (
            translate(locale, "palette.act_import"),
            PaletteAction::ImportBooks,
            "import ajouter",
        ),
        (
            translate(locale, "palette.act_calibre"),
            PaletteAction::ImportCalibre,
            "calibre import migration db metadata",
        ),
        (
            translate(locale, "palette.act_audit"),
            PaletteAction::OpenLibraryAudit,
            "sante audit doublons merge fusion proprete health duplicates",
        ),
        (
            translate(locale, "palette.act_fts"),
            PaletteAction::OpenContentSearch,
            "fts5 recherche texte chapitre contenu full-text search",
        ),
        (
            translate(locale, "palette.act_regex"),
            PaletteAction::OpenBulkRegex,
            "regex masse bulk nettoyer titre auteur",
        ),
        (
            translate(locale, "palette.act_rules"),
            PaletteAction::OpenImportRules,
            "regles import automatisation watch folder rules",
        ),
        (
            translate(locale, "palette.act_stats"),
            PaletteAction::OpenReadingStats,
            "stats statistiques heatmap calendrier lecture rythme wpm",
        ),
        (
            translate(locale, "palette.act_news"),
            PaletteAction::OpenNewsRecipes,
            "news rss actualites journal presse",
        ),
        (
            translate(locale, "palette.act_web"),
            PaletteAction::OpenWebCompanion,
            "web companion mobile lecteur opds phone navigateur",
        ),
        (
            translate(locale, "palette.act_sync"),
            PaletteAction::SyncCloud,
            "synchro drive cloud gdrive sync",
        ),
        (
            translate(locale, "palette.act_settings"),
            PaletteAction::OpenSettings,
            "paramètres settings config",
        ),
        (
            translate(locale, "palette.act_reading"),
            PaletteAction::FilterReading,
            "en cours lecture reading",
        ),
        (
            translate(locale, "palette.act_finished"),
            PaletteAction::FilterFinished,
            "terminés finished lu",
        ),
        (
            translate(locale, "palette.act_to_read"),
            PaletteAction::FilterToRead,
            "a lire to read",
        ),
        (
            translate(locale, "palette.act_all"),
            PaletteAction::FilterAll,
            "tous all bibliothèque library",
        ),
    ];

    let matched_actions: Vec<_> = all_actions
        .iter()
        .filter(|(label, _, keywords)| {
            q.is_empty() || label.to_lowercase().contains(&q) || keywords.contains(&q)
        })
        .collect();

    rsx! {
        div {
            class: "drawer-backdrop",
            style: "align-items: flex-start; justify-content: center; padding-top: 80px;",
            onclick: move |_| on_close.call(()),

            div {
                class: "command-palette-panel",
                style: "width: 620px; max-width: 92vw; background: #16192e; border: 1px solid rgba(99, 102, 241, 0.4); border-radius: 14px; box-shadow: 0 20px 50px rgba(0,0,0,0.8); overflow: hidden; display: flex; flex-direction: column;",
                onclick: move |e| e.stop_propagation(),

                // Header search bar
                div {
                    style: "display: flex; align-items: center; gap: 12px; padding: 14px 18px; border-bottom: 1px solid var(--border-subtle); background: rgba(255,255,255,0.02);",
                    span { style: "font-size: 18px; color: var(--accent-primary);", "⌘" }
                    input {
                        style: "flex: 1; background: transparent; border: none; outline: none; font-size: 15px; color: var(--text-primary); font-family: inherit;",
                        placeholder: translate(locale, "palette.placeholder"),
                        autofocus: true,
                        value: "{query}",
                        oninput: move |e| query.set(e.value()),
                    }
                    span {
                        style: "font-size: 11px; padding: 2px 6px; border-radius: 4px; background: rgba(255,255,255,0.1); color: var(--text-muted);",
                        "ESC"
                    }
                }

                // Tabs (BookOrbit order: All | Books | Authors | Series | Actions)
                div {
                    style: "display: flex; gap: 6px; padding: 8px 16px; border-bottom: 1px solid var(--border-subtle); background: rgba(255,255,255,0.015);",
                    for (t_id, t_label) in [
                        ("all", "Tout"),
                        ("books", "Livres"),
                        ("authors", "Auteurs"),
                        ("series", "Séries"),
                        ("actions", "Actions"),
                    ] {
                        button {
                            style: if tab == t_id {
                                "padding: 3px 10px; font-size: 11px; border-radius: 6px; background: #6366f1; color: #fff; border: none; font-weight: 600; cursor: pointer;"
                            } else {
                                "padding: 3px 10px; font-size: 11px; border-radius: 6px; background: rgba(255,255,255,0.04); color: var(--text-secondary); border: 1px solid var(--border-subtle); cursor: pointer;"
                            },
                            onclick: move |_| active_tab.set(t_id),
                            "{t_label}"
                        }
                    }
                }

                // Results list (BookOrbit order: Continue Reading / Books -> Authors -> Series -> Actions)
                div {
                    style: "max-height: 420px; overflow-y: auto; padding: 8px;",

                    // 0. Continue reading when query is empty
                    if q.is_empty() && (tab == "all" || tab == "books") && !continue_reading.is_empty() {
                        div {
                            style: "font-size: 11px; font-weight: 700; color: var(--text-muted); text-transform: uppercase; padding: 6px 12px; letter-spacing: 0.5px;",
                            {translate(locale, "library.continue_reading")}
                        }
                        for b in continue_reading.iter() {
                            {
                                let book_for_read = b.clone();
                                let pct = b.progress_percent.round() as i32;
                                rsx! {
                                    div {
                                        class: "palette-item",
                                        style: "display: flex; align-items: center; justify-content: space-between; padding: 9px 14px; border-radius: 8px; cursor: pointer;",
                                        onclick: move |_| {
                                            on_action.call(PaletteAction::ReadBook(book_for_read.clone()));
                                            on_close.call(());
                                        },
                                        div { style: "display: flex; flex-direction: column; gap: 2px;",
                                            span { style: "font-size: 13px; font-weight: 600; color: var(--text-primary);", "{b.title}" }
                                            span { style: "font-size: 11px; color: var(--text-muted);", "{b.author}" }
                                        }
                                        span { style: "font-size: 11px; color: #a5b4fc; font-weight: 600;", "{pct}%" }
                                    }
                                }
                            }
                        }
                    }

                    // 1. Books section
                    if (tab == "all" || tab == "books") && !matched_books.is_empty() {
                        div {
                            style: "font-size: 11px; font-weight: 700; color: var(--text-muted); text-transform: uppercase; padding: 8px 12px 6px; letter-spacing: 0.5px;",
                            {
                                let b_count = matched_books.len().to_string();
                                translate_fmt(locale, "palette.books_section", &[("count", &b_count)])
                            }
                        }
                        for b in matched_books.iter() {
                            {
                                let book_for_select = b.clone();
                                let book_for_read = b.clone();
                                let series_info = if let Some(ref s) = b.series {
                                    if let Some(idx) = b.series_index {
                                        format!(" • {s} #{}", idx)
                                    } else {
                                        format!(" • {s}")
                                    }
                                } else {
                                    String::new()
                                };
                                rsx! {
                                    div {
                                        class: "palette-item",
                                        style: "display: flex; align-items: center; justify-content: space-between; padding: 10px 14px; border-radius: 8px; cursor: pointer; transition: background 0.15s ease;",
                                        onclick: move |_| {
                                            on_action.call(PaletteAction::SelectBook(book_for_select.clone()));
                                            on_close.call(());
                                        },
                                        div { style: "display: flex; flex-direction: column; gap: 2px;",
                                            span { style: "font-size: 13px; font-weight: 600; color: var(--text-primary);", "{b.title}" }
                                            span { style: "font-size: 11px; color: var(--text-muted);", "{b.author}{series_info}" }
                                        }
                                        div { style: "display: flex; align-items: center; gap: 8px;",
                                            button {
                                                class: "btn btn-secondary",
                                                style: "font-size: 11px; padding: 4px 8px;",
                                                onclick: {
                                                    let b_r = book_for_read.clone();
                                                    move |e: Event<MouseData>| {
                                                        e.stop_propagation();
                                                        on_action.call(PaletteAction::ReadBook(b_r.clone()));
                                                        on_close.call(());
                                                    }
                                                },
                                                {translate(locale, "palette.read_btn")}
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // 2. Authors section
                    if (tab == "all" || tab == "authors") && !matched_authors.is_empty() {
                        div {
                            style: "font-size: 11px; font-weight: 700; color: var(--text-muted); text-transform: uppercase; padding: 8px 12px 6px; letter-spacing: 0.5px;",
                            "{translate(locale, \"nav.authors\")}"
                        }
                        for author in matched_authors.iter() {
                            {
                                let a_clone = author.clone();
                                rsx! {
                                    div {
                                        class: "palette-item",
                                        style: "display: flex; align-items: center; justify-content: space-between; padding: 9px 14px; border-radius: 8px; cursor: pointer; font-size: 13px;",
                                        onclick: move |_| {
                                            on_action.call(PaletteAction::FilterAuthor(a_clone.clone()));
                                            on_close.call(());
                                        },
                                        span { "{author}" }
                                        span { style: "font-size: 11px; color: var(--text-muted);", "{translate(locale, \"nav.authors\")}" }
                                    }
                                }
                            }
                        }
                    }

                    // 3. Series section
                    if (tab == "all" || tab == "series") && !matched_series.is_empty() {
                        div {
                            style: "font-size: 11px; font-weight: 700; color: var(--text-muted); text-transform: uppercase; padding: 8px 12px 6px; letter-spacing: 0.5px;",
                            "{translate(locale, \"nav.series\")}"
                        }
                        for ser in matched_series.iter() {
                            {
                                let s_clone = ser.clone();
                                rsx! {
                                    div {
                                        class: "palette-item",
                                        style: "display: flex; align-items: center; justify-content: space-between; padding: 9px 14px; border-radius: 8px; cursor: pointer; font-size: 13px; color: #c4b5fd;",
                                        onclick: move |_| {
                                            on_action.call(PaletteAction::FilterSeries(s_clone.clone()));
                                            on_close.call(());
                                        },
                                        span { "{ser}" }
                                        span { style: "font-size: 11px; color: var(--text-muted);", "{translate(locale, \"nav.series\")}" }
                                    }
                                }
                            }
                        }
                    }

                    // 4. Actions section
                    if (tab == "all" || tab == "actions") && !matched_actions.is_empty() {
                        div {
                            style: "font-size: 11px; font-weight: 700; color: var(--text-muted); text-transform: uppercase; padding: 8px 12px 6px; letter-spacing: 0.5px;",
                            {translate(locale, "palette.quick_actions")}
                        }
                        for (label, action, _) in matched_actions.iter().take(if tab == "all" { 5 } else { 30 }) {
                            {
                                let act = (*action).clone();
                                rsx! {
                                    div {
                                        class: "palette-item",
                                        style: "display: flex; align-items: center; justify-content: space-between; padding: 10px 14px; border-radius: 8px; cursor: pointer; font-size: 13px; color: var(--text-primary); transition: background 0.15s ease;",
                                        onclick: move |_| {
                                            on_action.call(act.clone());
                                            on_close.call(());
                                        },
                                        span { "{label}" }
                                        span { style: "font-size: 11px; color: var(--accent-primary);", {translate(locale, "palette.run")} }
                                    }
                                }
                            }
                        }
                    }
                }

                // Footer tips
                div {
                    style: "display: flex; justify-content: space-between; align-items: center; padding: 8px 16px; background: rgba(0,0,0,0.3); border-top: 1px solid var(--border-subtle); font-size: 11px; color: var(--text-muted);",
                    span { {translate(locale, "palette.tip")} }
                    span { "Vellum v1.0 • Rust Native" }
                }
            }
        }
    }
}
