use dioxus::prelude::*;
use vellum_domain::Book;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum KanbanTargetStatus {
    ToRead,
    Reading,
    Finished,
    Favorite,
}

#[component]
pub fn KanbanBoard(
    books: Vec<Book>,
    on_select: EventHandler<Book>,
    on_read: EventHandler<Book>,
    on_change_status: EventHandler<(String, KanbanTargetStatus)>,
) -> Element {
    let to_read_books: Vec<Book> = books
        .iter()
        .filter(|b| b.progress_percent == 0.0 && !b.is_finished)
        .cloned()
        .collect();

    let reading_books: Vec<Book> = books
        .iter()
        .filter(|b| b.progress_percent > 0.0 && !b.is_finished)
        .cloned()
        .collect();

    let finished_books: Vec<Book> = books
        .iter()
        .filter(|b| b.is_finished || b.progress_percent >= 100.0)
        .cloned()
        .collect();

    let favorite_books: Vec<Book> = books
        .iter()
        .filter(|b| {
            b.rating.unwrap_or(0) >= 4
                || b.tags.iter().any(|t| {
                    let n = t.name.to_lowercase();
                    n.contains("coup de c") || n.contains("favori")
                })
        })
        .cloned()
        .collect();

    rsx! {
        div {
            class: "kanban-board-container",
            style: "flex: 1; display: grid; grid-template-columns: repeat(4, 1fr); gap: 16px; overflow-x: auto; padding: 4px 0 16px 0; min-height: 400px; height: 100%;",

            // 1. Colonne : À lire (PAL)
            KanbanColumn {
                icon: "⏳",
                title: "À lire (PAL)",
                accent_color: "#38bdf8",
                books: to_read_books,
                current_col: KanbanTargetStatus::ToRead,
                on_select: on_select,
                on_read: on_read,
                on_change_status: on_change_status,
            }

            // 2. Colonne : En cours
            KanbanColumn {
                icon: "📖",
                title: "En cours",
                accent_color: "#f59e0b",
                books: reading_books,
                current_col: KanbanTargetStatus::Reading,
                on_select: on_select,
                on_read: on_read,
                on_change_status: on_change_status,
            }

            // 3. Colonne : Terminés
            KanbanColumn {
                icon: "✅",
                title: "Terminés",
                accent_color: "#10b981",
                books: finished_books,
                current_col: KanbanTargetStatus::Finished,
                on_select: on_select,
                on_read: on_read,
                on_change_status: on_change_status,
            }

            // 4. Colonne : Coups de cœur
            KanbanColumn {
                icon: "⭐",
                title: "Coups de cœur",
                accent_color: "#c084fc",
                books: favorite_books,
                current_col: KanbanTargetStatus::Favorite,
                on_select: on_select,
                on_read: on_read,
                on_change_status: on_change_status,
            }
        }
    }
}

#[component]
fn KanbanColumn(
    icon: &'static str,
    title: &'static str,
    accent_color: &'static str,
    books: Vec<Book>,
    current_col: KanbanTargetStatus,
    on_select: EventHandler<Book>,
    on_read: EventHandler<Book>,
    on_change_status: EventHandler<(String, KanbanTargetStatus)>,
) -> Element {
    let count = books.len();

    rsx! {
        div {
            class: "kanban-column",
            style: "background: var(--bg-card); border: 1px solid var(--border-color); border-radius: 12px; display: flex; flex-direction: column; min-width: 250px; overflow: hidden;",

            // Column Header
            div {
                style: "padding: 12px 14px; border-bottom: 2px solid {accent_color}; display: flex; align-items: center; justify-content: space-between; background: rgba(255,255,255,0.02);",
                div { style: "display: flex; align-items: center; gap: 8px;",
                    span { style: "font-size: 16px;", "{icon}" }
                    span { style: "font-size: 14px; font-weight: 700; color: var(--text-primary);", "{title}" }
                }
                span {
                    style: "font-size: 11px; font-weight: 700; color: {accent_color}; background: rgba(255,255,255,0.06); padding: 2px 8px; border-radius: 10px;",
                    "{count}"
                }
            }

            // Books List inside Column
            div {
                style: "flex: 1; overflow-y: auto; padding: 10px; display: flex; flex-direction: column; gap: 10px;",
                if books.is_empty() {
                    div {
                        style: "text-align: center; padding: 30px 10px; color: var(--text-secondary); font-size: 12px; font-style: italic;",
                        "Aucun livre dans cette colonne"
                    }
                } else {
                    for book in books.into_iter() {
                        KanbanCard {
                            key: "{book.id}",
                            book: book,
                            current_col: current_col,
                            on_select: on_select,
                            on_read: on_read,
                            on_change_status: on_change_status,
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn KanbanCard(
    book: Book,
    current_col: KanbanTargetStatus,
    on_select: EventHandler<Book>,
    on_read: EventHandler<Book>,
    on_change_status: EventHandler<(String, KanbanTargetStatus)>,
) -> Element {
    let b_id = book.id.clone();
    let title = book.title.clone();
    let author = book.author.clone();
    let cover_path = book.cover_path.clone();
    let progress = book.progress_percent;
    let formats = book.formats.clone();

    let cover_src = crate::cover_cache::resolve_cover_src(&b_id, cover_path.as_deref(), true);

    rsx! {
        div {
            class: "kanban-card",
            style: "background: var(--bg-primary); border: 1px solid var(--border-color); border-radius: 8px; padding: 10px; display: flex; flex-direction: column; gap: 8px; cursor: pointer; transition: transform 0.15s, border-color 0.15s, box-shadow 0.15s;",
            onclick: {
                let book = book.clone();
                move |_| on_select.call(book.clone())
            },

            // Card Top: Thumbnail + Details
            div { style: "display: flex; gap: 10px; align-items: flex-start;",
                // Cover Thumbnail
                div {
                    style: "width: 44px; height: 62px; flex-shrink: 0; border-radius: 4px; overflow: hidden; background: rgba(255,255,255,0.05); display: flex; align-items: center; justify-content: center; box-shadow: 0 2px 4px rgba(0,0,0,0.3);",
                    if let Some(src) = cover_src {
                        img {
                            src: "{src}",
                            alt: "{title}",
                            style: "width: 100%; height: 100%; object-fit: cover;",
                            loading: "lazy",
                        }
                    } else {
                        span { style: "font-size: 20px;", "📖" }
                    }
                }

                // Text Details
                div { style: "flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 2px;",
                    div {
                        style: "font-size: 13px; font-weight: 700; color: var(--text-primary); line-height: 1.3; overflow: hidden; text-overflow: ellipsis; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical;",
                        title: "{title}",
                        "{title}"
                    }
                    div {
                        style: "font-size: 11px; color: var(--text-secondary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;",
                        "{author}"
                    }

                    if let Some(ref s) = book.series {
                        span {
                            style: "font-size: 10px; color: #c4b5fd; margin-top: 2px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;",
                            "📚 {s}"
                        }
                    }

                    // Formats Chips
                    div { style: "display: flex; gap: 4px; margin-top: 4px; flex-wrap: wrap;",
                        for fmt in formats.iter() {
                            span {
                                style: "font-size: 9px; font-weight: 700; padding: 1px 4px; border-radius: 3px; background: rgba(255,255,255,0.08); color: var(--text-secondary);",
                                "{fmt.format.to_uppercase()}"
                            }
                        }
                    }
                }
            }

            // Progress bar if in reading column
            if current_col == KanbanTargetStatus::Reading {
                div { style: "margin-top: 2px;",
                    div { style: "display: flex; justify-content: space-between; font-size: 10px; color: #f59e0b; font-weight: 600; margin-bottom: 2px;",
                        span { "Progression" }
                        span { "{progress as i32}%" }
                    }
                    div { style: "width: 100%; height: 4px; background: rgba(255,255,255,0.1); border-radius: 2px; overflow: hidden;",
                        div {
                            style: "height: 100%; width: {progress}%; background: #f59e0b; border-radius: 2px; transition: width 0.3s;"
                        }
                    }
                }
            }

            // Quick Actions Bar
            div {
                style: "display: flex; align-items: center; justify-content: space-between; margin-top: 4px; padding-top: 6px; border-top: 1px solid rgba(255,255,255,0.05);",
                onclick: move |e| e.stop_propagation(),

                // Action Read
                button {
                    class: "btn btn-secondary",
                    style: "font-size: 11px; padding: 3px 8px; display: flex; align-items: center; gap: 4px;",
                    title: "Ouvrir dans le lecteur",
                    onclick: {
                        let book = book.clone();
                        move |_| on_read.call(book.clone())
                    },
                    "📖 Lire"
                }

                // Transition button based on current status
                match current_col {
                    KanbanTargetStatus::ToRead => {
                        let id = b_id.clone();
                        let ch = on_change_status;
                        rsx! {
                            button {
                                class: "btn btn-primary",
                                style: "font-size: 10px; padding: 3px 8px; background: rgba(245, 158, 11, 0.2); color: #f59e0b; border: 1px solid rgba(245, 158, 11, 0.4);",
                                onclick: move |_| ch.call((id.clone(), KanbanTargetStatus::Reading)),
                                "▶ Commencer"
                            }
                        }
                    },
                    KanbanTargetStatus::Reading => {
                        let id = b_id.clone();
                        let ch = on_change_status;
                        rsx! {
                            button {
                                class: "btn btn-primary",
                                style: "font-size: 10px; padding: 3px 8px; background: rgba(16, 185, 129, 0.2); color: #34d399; border: 1px solid rgba(16, 185, 129, 0.4);",
                                onclick: move |_| ch.call((id.clone(), KanbanTargetStatus::Finished)),
                                "✔ Terminé"
                            }
                        }
                    },
                    KanbanTargetStatus::Finished => {
                        let id = b_id.clone();
                        let ch = on_change_status;
                        rsx! {
                            button {
                                class: "btn btn-secondary",
                                style: "font-size: 10px; padding: 3px 8px; color: #c084fc;",
                                onclick: move |_| ch.call((id.clone(), KanbanTargetStatus::Favorite)),
                                "⭐ Coup de cœur"
                            }
                        }
                    },
                    KanbanTargetStatus::Favorite => {
                        let id = b_id.clone();
                        let ch = on_change_status;
                        rsx! {
                            button {
                                class: "btn btn-secondary",
                                style: "font-size: 10px; padding: 3px 8px;",
                                onclick: move |_| ch.call((id.clone(), KanbanTargetStatus::ToRead)),
                                "↩ À lire"
                            }
                        }
                    },
                }
            }
        }
    }
}
