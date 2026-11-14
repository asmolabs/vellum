use dioxus::prelude::*;
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
                    || b.series.as_deref().unwrap_or_default().to_lowercase().contains(&q)
                    || b.tags.iter().any(|t| t.name.to_lowercase().contains(&q))
            })
            .take(8)
            .cloned()
            .collect()
    };

    // Quick built-in actions
    let all_actions = [
        ("⊞ Basculer en vue Grille classique", PaletteAction::SetViewGrid, "vue grille grid affichage"),
        ("☰ Basculer en vue Tableau (Calibre)", PaletteAction::SetViewTable, "vue tableau liste table calibre"),
        ("🎠 Basculer en vue Tableau + Carrousel (Calibre)", PaletteAction::SetViewTableCarousel, "vue tableau carrousel cover browser calibre"),
        ("📋 Basculer en vue Tableau Kanban (Pile à Lire)", PaletteAction::SetViewKanban, "vue kanban pal pile a lire tableau trello colonnes"),
        ("📥 Importer des livres (Fichiers EPUB, PDF, CBZ...)", PaletteAction::ImportBooks, "import ajouter"),
        ("📦 Importer une bibliothèque Calibre (metadata.db)", PaletteAction::ImportCalibre, "calibre import migration db metadata"),
        ("🛡️ Santé & Audit de la bibliothèque (Doublons, fusion, fiches incomplètes)", PaletteAction::OpenLibraryAudit, "sante audit doublons merge fusion proprete"),
        ("🔍 Recherche plein texte dans les chapitres (FTS5)", PaletteAction::OpenContentSearch, "fts5 recherche texte chapitre contenu"),
        ("🔤 Recherche & Remplacement Regex en masse", PaletteAction::OpenBulkRegex, "regex masse bulk nettoyer titre auteur"),
        ("⚡ Règles d'import automatiques (Tags, séries, polish)", PaletteAction::OpenImportRules, "regles import automatisation watch folder"),
        ("📊 Statistiques & Habitudes de lecture (Heatmap annuelle, rythme)", PaletteAction::OpenReadingStats, "stats statistiques heatmap calendrier lecture rythme wpm"),
        ("📰 Recettes d'actualités (compiler les flux RSS en journal EPUB)", PaletteAction::OpenNewsRecipes, "news rss actualites journal presse"),
        ("🌐 Ouvrir le Compagnon Web Mobile & OPDS (navigateur)", PaletteAction::OpenWebCompanion, "web companion mobile lecteur opds phone navigateur"),
        ("☁️ Synchroniser maintenant avec Google Drive", PaletteAction::SyncCloud, "synchro drive cloud gdrive"),
        ("⚙️ Ouvrir les Paramètres & Diagnostics", PaletteAction::OpenSettings, "paramètres settings config"),
        ("📖 Afficher les livres en cours de lecture", PaletteAction::FilterReading, "en cours lecture reading"),
        ("✓ Afficher les livres terminés", PaletteAction::FilterFinished, "terminés finished lu"),
        ("⏳ Afficher les livres à lire", PaletteAction::FilterToRead, "a lire to read"),
        ("📚 Afficher toute la bibliothèque", PaletteAction::FilterAll, "tous all bibliothèque"),
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
                        placeholder: "Rechercher un livre, une saga ou taper une action... (Échap pour fermer)",
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
                            "Actions Rapides"
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
                                        span { style: "font-size: 11px; color: var(--accent-primary);", "↵ Exécuter" }
                                    }
                                }
                            }
                        }
                    }

                    // Books section
                    if !matched_books.is_empty() {
                        div {
                            style: "font-size: 11px; font-weight: 700; color: var(--text-muted); text-transform: uppercase; padding: 8px 12px 6px; letter-spacing: 0.5px;",
                            "Livres & Sagas ({matched_books.len()})"
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
                                                "📖 Lire"
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
                    span { "Astuce : Appuyez sur ⌘K à tout moment pour ouvrir la palette" }
                    span { "Vellum v1.0 • Rust Native" }
                }
            }
        }
    }
}
