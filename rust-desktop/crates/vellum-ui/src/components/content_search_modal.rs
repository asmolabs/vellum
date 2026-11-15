use dioxus::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use vellum_core::{translate, translate_fmt, BookService, Locale};
use vellum_domain::{Book, ContentSearchHit};

#[component]
pub fn ContentSearchModal(
    locale: Locale,
    book_service: Arc<BookService>,
    books: Vec<Book>,
    on_close: EventHandler<()>,
    on_select_book: EventHandler<Book>,
) -> Element {
    let mut query = use_signal(String::new);
    let mut results = use_signal(Vec::<ContentSearchHit>::new);
    let mut is_searching = use_signal(|| false);
    let mut is_indexing = use_signal(|| false);
    let mut status_msg = use_signal(|| None::<(String, bool)>);

    let book_map: HashMap<String, Book> = books.iter().map(|b| (b.id.clone(), b.clone())).collect();

    let run_search = {
        let bs = book_service.clone();
        move |_| {
            let q = query.read().trim().to_string();
            if q.is_empty() {
                results.set(Vec::new());
                return;
            }
            is_searching.set(true);
            status_msg.set(None);
            let bs_task = bs.clone();
            spawn(async move {
                let res =
                    tokio::task::spawn_blocking(move || bs_task.search_inside_books(&q, 40)).await;
                is_searching.set(false);
                match res {
                    Ok(Ok(hits)) => {
                        let count = hits.len().to_string();
                        results.set(hits);
                        status_msg.set(Some((
                            translate_fmt(locale, "search.hits_found", &[("count", &count)]),
                            true,
                        )));
                    }
                    Ok(Err(e)) => status_msg.set(Some((
                        translate_fmt(locale, "common.error", &[("error", &e.to_string())]),
                        false,
                    ))),
                    Err(e) => status_msg.set(Some((
                        translate_fmt(locale, "common.async_error", &[("error", &e.to_string())]),
                        false,
                    ))),
                }
            });
        }
    };

    let run_index_all = {
        let bs = book_service.clone();
        let ids: Vec<String> = books.iter().map(|b| b.id.clone()).collect();
        move |_| {
            is_indexing.set(true);
            status_msg.set(Some((
                translate(locale, "search.indexing").to_string(),
                true,
            )));
            let bs_task = bs.clone();
            let ids_task = ids.clone();
            spawn(async move {
                let res = tokio::task::spawn_blocking(move || {
                    let mut total_chapters = 0usize;
                    let mut indexed_books = 0usize;
                    for id in ids_task {
                        if let Ok(ch) = bs_task.index_book_content(&id) {
                            total_chapters += ch;
                            if ch > 0 {
                                indexed_books += 1;
                            }
                        }
                    }
                    (indexed_books, total_chapters)
                })
                .await;
                is_indexing.set(false);
                match res {
                    Ok((b_count, ch_count)) => {
                        let b_str = b_count.to_string();
                        let c_str = ch_count.to_string();
                        status_msg.set(Some((
                            translate_fmt(
                                locale,
                                "search.indexed",
                                &[("books", &b_str), ("chapters", &c_str)],
                            ),
                            true,
                        )));
                    }
                    Err(e) => status_msg.set(Some((
                        translate_fmt(locale, "common.error", &[("error", &e.to_string())]),
                        false,
                    ))),
                }
            });
        }
    };

    rsx! {
        div {
            class: "drawer-backdrop",
            style: "align-items: center; justify-content: center;",
            onclick: move |_| on_close.call(()),
            div {
                class: "modal-card",
                style: "width: 760px; max-width: 94vw; max-height: 85vh; display: flex; flex-direction: column; padding: 24px; gap: 16px;",
                onclick: move |e| e.stop_propagation(),

                div {
                    style: "display: flex; align-items: center; justify-content: space-between;",
                    div {
                        h2 { style: "margin: 0; font-size: 18px;", {translate(locale, "search.title")} }
                        p { style: "margin: 4px 0 0 0; font-size: 12px; color: var(--text-muted);", {translate(locale, "search.subtitle")} }
                    }
                    button {
                        class: "btn btn-secondary",
                        onclick: move |_| on_close.call(()),
                        "✕"
                    }
                }

                div {
                    style: "display: flex; gap: 8px; align-items: center;",
                    input {
                        class: "form-input",
                        style: "flex: 1;",
                        placeholder: translate(locale, "search.placeholder"),
                        value: "{query}",
                        oninput: move |e| query.set(e.value()),
                        onkeydown: {
                            let mut rs = run_search.clone();
                            move |e: KeyboardEvent| {
                                if e.key() == Key::Enter {
                                    rs(());
                                }
                            }
                        },
                    }
                    button {
                        class: "btn btn-primary",
                        disabled: *is_searching.read(),
                        onclick: {
                            let mut rs = run_search.clone();
                            move |_| rs(())
                        },
                        {if *is_searching.read() { translate(locale, "search.searching") } else { translate(locale, "action.search") }}
                    }
                    button {
                        class: "btn btn-secondary",
                        disabled: *is_indexing.read(),
                        onclick: run_index_all,
                        {if *is_indexing.read() { translate(locale, "search.indexing_btn") } else { translate(locale, "search.index_all") }}
                    }
                }

                if let Some((msg, ok)) = status_msg.read().as_ref() {
                    div {
                        style: if *ok {
                            "padding: 8px 12px; border-radius: 6px; font-size: 13px; background: rgba(34,197,94,0.12); color: #86efac; border: 1px solid rgba(34,197,94,0.3);"
                        } else {
                            "padding: 8px 12px; border-radius: 6px; font-size: 13px; background: rgba(239,68,68,0.12); color: #fca5a5; border: 1px solid rgba(239,68,68,0.3);"
                        },
                        "{msg}"
                    }
                }

                div {
                    style: "flex: 1; overflow-y: auto; display: flex; flex-direction: column; gap: 10px;",
                    for hit in results.read().iter() {
                        {
                            let book_opt = book_map.get(&hit.book_id).cloned();
                            let title = book_opt
                                .as_ref()
                                .map(|b| format!("{} — {}", b.title, b.author))
                                .unwrap_or_else(|| hit.book_id.clone());
                            let ch_idx = (hit.chapter_index + 1).to_string();
                            let ch_title = hit.chapter_title.clone();
                            let snippet = hit.snippet.clone();
                            let chap_label = translate_fmt(
                                locale,
                                "search.chap_prefix",
                                &[("idx", &ch_idx), ("title", &ch_title)],
                            );
                            rsx! {
                                div {
                                    style: "padding: 12px; border-radius: 8px; background: rgba(255,255,255,0.03); border: 1px solid var(--border-color); cursor: pointer;",
                                    onclick: move |_| {
                                        if let Some(b) = book_opt.clone() {
                                            on_select_book.call(b);
                                            on_close.call(());
                                        }
                                    },
                                    div {
                                        style: "display: flex; justify-content: space-between; font-weight: 600; font-size: 14px; margin-bottom: 4px;",
                                        span { "{title}" }
                                        span { style: "font-size: 12px; color: var(--text-muted);", "{chap_label}" }
                                    }
                                    p {
                                        style: "margin: 0; font-size: 13px; color: var(--text-secondary); line-height: 1.4;",
                                        "{snippet}"
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
