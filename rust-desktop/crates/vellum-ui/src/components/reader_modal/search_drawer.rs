use crate::state::ReaderSession;
use dioxus::prelude::*;
use vellum_core::reader_store::SearchResult;

use super::jump_and_notify;

fn execute_reader_search(
    session: Signal<Option<ReaderSession>>,
    search_query: Signal<String>,
    mut search_results: Signal<Vec<SearchResult>>,
    mut is_searching: Signal<bool>,
) {
    let q = search_query.read().clone();
    if !q.trim().is_empty() {
        is_searching.set(true);
        let res = session
            .read()
            .as_ref()
            .map(|s| s.search(&q))
            .unwrap_or_default();
        search_results.set(res);
        is_searching.set(false);
    }
}

#[component]
pub(super) fn ReaderSearchDrawer(
    session: Signal<Option<ReaderSession>>,
    mut active_drawer: Signal<Option<String>>,
    mut search_query: Signal<String>,
    search_results: Signal<Vec<SearchResult>>,
    is_searching: Signal<bool>,
    on_progress_update: EventHandler<(usize, usize, f64)>,
) -> Element {
    rsx! {
        div {
            class: "reader-search-backdrop",
            onclick: move |_| active_drawer.set(None),

            div {
                class: "reader-search-drawer",
                onclick: move |e| e.stop_propagation(),

                div { style: "padding: 16px 20px; border-bottom: 1px solid var(--border-subtle); display: flex; align-items: center; justify-content: space-between;",
                    span { style: "font-weight: 600; font-size: 14px; color: var(--text-primary);", "Recherche dans le livre" }
                    button {
                        class: "btn btn-icon",
                        onclick: move |_| active_drawer.set(None),
                        "✕"
                    }
                }

                div { style: "padding: 12px 16px; border-bottom: 1px solid var(--border-subtle); display: flex; gap: 8px;",
                    input {
                        style: "flex: 1; padding: 8px 12px; background: rgba(255,255,255,0.06); border: 1px solid var(--border-subtle); border-radius: 6px; color: var(--text-primary); font-size: 13px; outline: none;",
                        placeholder: "Rechercher un mot, une phrase...",
                        autofocus: true,
                        value: "{search_query}",
                        oninput: move |e| search_query.set(e.value()),
                        onkeydown: move |e: KeyboardEvent| {
                            if e.key().to_string() == "Enter" {
                                execute_reader_search(session, search_query, search_results, is_searching);
                            }
                        },
                    }
                    button {
                        class: "btn btn-primary",
                        onclick: move |_| execute_reader_search(session, search_query, search_results, is_searching),
                        "Chercher"
                    }
                }

                div { style: "flex: 1; overflow-y: auto; padding: 12px 16px;",
                    if *is_searching.read() {
                        div { style: "padding: 30px; text-align: center; color: var(--text-muted); font-size: 12px;",
                            "Recherche en cours dans l'ensemble des chapitres..."
                        }
                    } else if search_results.read().is_empty() {
                        div { style: "padding: 40px 10px; text-align: center; color: var(--text-muted); font-size: 12px;",
                            if search_query.read().trim().is_empty() {
                                "Entrez un mot ou une phrase à rechercher dans le livre."
                            } else {
                                "Aucun résultat trouvé pour cette recherche."
                            }
                        }
                    } else {
                        div { style: "font-size: 11px; color: var(--text-muted); margin-bottom: 10px;",
                            "{search_results.read().len()} occurrences trouvées :"
                        }
                        for item in search_results.read().iter() {
                            {
                                let ch_idx = item.chapter_index;
                                let ch_title = item.chapter_title.clone();
                                let before = item.snippet_before.clone();
                                let matched = item.matched_text.clone();
                                let after = item.snippet_after.clone();
                                rsx! {
                                    div {
                                        class: "reader-search-card",
                                        onclick: move |_| jump_and_notify(session, active_drawer, on_progress_update, ch_idx),
                                        div { style: "font-size: 11px; font-weight: 600; color: var(--accent-primary);",
                                            "{ch_title}"
                                        }
                                        div { class: "reader-search-snippet",
                                            span { "{before}" }
                                            mark { "{matched}" }
                                            span { "{after}" }
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
