use dioxus::prelude::*;
use vellum_core::{translate, translate_fmt, Locale};
use vellum_domain::Book;

#[derive(Clone, PartialEq)]
pub enum PaletteAction {
    SelectBook(Book),
    ReadBook(Book),
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

    let q = query.read().trim().to_lowercase();

    // Matching books
    let matched_books: Vec<Book> = if q.is_empty() {
        books.iter().take(5).cloned().collect()
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
            .take(8)
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

                // Results list
                div {
                    style: "max-height: 420px; overflow-y: auto; padding: 8px;",

                    // Actions section
                    if !matched_actions.is_empty() {
                        div {
                            style: "font-size: 11px; font-weight: 700; color: var(--text-muted); text-transform: uppercase; padding: 6px 12px; letter-spacing: 0.5px;",
                            {translate(locale, "palette.quick_actions")}
                        }
                        for (label, action, _) in matched_actions.iter() {
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

                    // Books section
                    if !matched_books.is_empty() {
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
                                        format!(" • 📚 {s} #{}", idx)
                                    } else {
                                        format!(" • 📚 {s}")
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
