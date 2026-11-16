use crate::state::ReaderSession;
use dioxus::prelude::*;
use vellum_core::i18n::{translate, translate_fmt, Locale};

use super::jump_and_notify;

#[component]
pub(super) fn ReaderTocDrawer(
    mut session: Signal<Option<ReaderSession>>,
    locale: Locale,
    mut active_drawer: Signal<Option<String>>,
    mut drawer_tab: Signal<String>,
    mut toc_filter: Signal<String>,
    filtered_toc: Vec<(usize, String)>,
    total_chapters: usize,
    cur_page: usize,
    on_progress_update: EventHandler<(usize, usize, f64)>,
) -> Element {
    let user_data = session
        .read()
        .as_ref()
        .map(|s| s.user_data.clone())
        .unwrap_or_default();

    rsx! {
        div {
            class: "reader-toc-backdrop",
            onclick: move |_| active_drawer.set(None),

            div {
                class: "reader-toc-drawer",
                onclick: move |e| e.stop_propagation(),

                div { class: "reader-toc-header",
                    div { style: "display: flex; flex-direction: column;",
                        span { style: "font-weight: 600; font-size: 14px; color: var(--text-primary);", {translate(locale, "reader.toc_header")} }
                        span { style: "font-size: 11px; color: var(--text-muted);",
                            {translate_fmt(
                                locale,
                                "reader.toc_summary",
                                &[
                                    ("chapters", &total_chapters.to_string()),
                                    ("bookmarks", &user_data.bookmarks.len().to_string()),
                                    ("notes", &user_data.highlights.len().to_string()),
                                ],
                            )}
                        }
                    }
                    button {
                        class: "btn btn-icon",
                        onclick: move |_| active_drawer.set(None),
                        "✕"
                    }
                }

                div { class: "reader-drawer-tabs",
                    button {
                        class: if *drawer_tab.read() == "toc" { "reader-drawer-tab active" } else { "reader-drawer-tab" },
                        onclick: move |_| drawer_tab.set("toc".to_string()),
                        {translate(locale, "reader.tab_toc")}
                    }
                    button {
                        class: if *drawer_tab.read() == "bookmarks" { "reader-drawer-tab active" } else { "reader-drawer-tab" },
                        onclick: move |_| drawer_tab.set("bookmarks".to_string()),
                        {translate_fmt(
                            locale,
                            "reader.tab_bookmarks",
                            &[("count", &user_data.bookmarks.len().to_string())],
                        )}
                    }
                    button {
                        class: if *drawer_tab.read() == "highlights" { "reader-drawer-tab active" } else { "reader-drawer-tab" },
                        onclick: move |_| drawer_tab.set("highlights".to_string()),
                        {translate_fmt(
                            locale,
                            "reader.tab_highlights",
                            &[("count", &user_data.highlights.len().to_string())],
                        )}
                    }
                }

                if *drawer_tab.read() == "toc" {
                    div { style: "padding: 10px 14px; border-bottom: 1px solid var(--border-subtle);",
                        input {
                            style: "width: 100%; padding: 8px 12px; background: rgba(255,255,255,0.05); border: 1px solid var(--border-subtle); border-radius: 6px; color: var(--text-primary); font-size: 12px; outline: none;",
                            placeholder: translate(locale, "reader.toc_filter_placeholder"),
                            value: "{toc_filter}",
                            oninput: move |e| toc_filter.set(e.value()),
                        }
                    }

                    div { class: "reader-toc-list",
                        if filtered_toc.is_empty() {
                            div { style: "padding: 30px 10px; text-align: center; color: var(--text-muted); font-size: 12px;",
                                {translate(locale, "reader.toc_empty")}
                            }
                        } else {
                            for (idx, title) in filtered_toc.iter() {
                                {
                                    let is_current = *idx == cur_page;
                                    let target_idx = *idx;
                                    let title_display = title.clone();
                                    rsx! {
                                        button {
                                            key: "{target_idx}",
                                            class: if is_current { "reader-toc-item active" } else { "reader-toc-item" },
                                            onclick: move |_| jump_and_notify(session, active_drawer, on_progress_update, target_idx),
                                            span { style: "overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 250px;",
                                                "{title_display}"
                                            }
                                            span { style: "font-size: 11px; opacity: 0.6; margin-left: 8px;",
                                                "p. {target_idx + 1}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if *drawer_tab.read() == "bookmarks" {
                    div { class: "reader-toc-list",
                        if user_data.bookmarks.is_empty() {
                            div { style: "padding: 40px 20px; text-align: center; color: var(--text-muted); font-size: 12px;",
                                p { style: "font-size: 24px; margin-bottom: 8px;", "🔖" }
                                p { {translate(locale, "reader.bookmarks_empty")} }
                                p { style: "margin-top: 6px; font-size: 11px; opacity: 0.7;", {translate(locale, "reader.bookmarks_hint")} }
                            }
                        } else {
                            for bm in user_data.bookmarks.iter() {
                                {
                                    let bm_id = bm.id.clone();
                                    let bm_ch = bm.chapter_index;
                                    let bm_title = bm.chapter_title.clone();
                                    let bm_preview = bm.preview.clone();
                                    let bm_pct = bm.progress_percent;
                                    rsx! {
                                        div {
                                            key: "{bm_id}",
                                            class: "reader-annotation-item",
                                            div { style: "display: flex; justify-content: space-between; align-items: center;",
                                                span {
                                                    style: "font-weight: 600; font-size: 12px; color: var(--accent-primary); cursor: pointer;",
                                                    onclick: move |_| jump_and_notify(session, active_drawer, on_progress_update, bm_ch),
                                                    "🔖 {bm_title} ({bm_pct:.0}%)"
                                                }
                                                button {
                                                    class: "btn btn-icon",
                                                    style: "padding: 2px 6px; font-size: 11px;",
                                                    title: translate(locale, "reader.bookmark_delete"),
                                                    onclick: {
                                                        let id = bm_id.clone();
                                                        move |_| {
                                                            let mut mut_sess = session.write();
                                                            if let Some(s) = mut_sess.as_mut() {
                                                                s.remove_bookmark(&id);
                                                            }
                                                        }
                                                    },
                                                    "🗑"
                                                }
                                            }
                                            span {
                                                style: "font-size: 11px; color: var(--text-secondary); line-height: 1.4; cursor: pointer;",
                                                onclick: move |_| jump_and_notify(session, active_drawer, on_progress_update, bm_ch),
                                                "{bm_preview}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if *drawer_tab.read() == "highlights" {
                    div { class: "reader-toc-list",
                        if user_data.highlights.is_empty() {
                            div { style: "padding: 40px 20px; text-align: center; color: var(--text-muted); font-size: 12px;",
                                p { style: "font-size: 24px; margin-bottom: 8px;", "🖍" }
                                p { {translate(locale, "reader.highlights_empty")} }
                                p { style: "margin-top: 6px; font-size: 11px; opacity: 0.7;", {translate(locale, "reader.highlights_hint")} }
                            }
                        } else {
                            for h in user_data.highlights.iter() {
                                {
                                    let h_id = h.id.clone();
                                    let h_ch = h.chapter_index;
                                    let h_title = h.chapter_title.clone();
                                    let h_text = h.selected_text.clone();
                                    let h_note = h.note.clone();
                                    let dot_color = match h.color.as_str() {
                                        "green" => "#4ade80",
                                        "blue" => "#60a5fa",
                                        "pink" => "#f472b6",
                                        _ => "#facc15",
                                    };
                                    rsx! {
                                        div {
                                            key: "{h_id}",
                                            class: "reader-annotation-item",
                                            div { style: "display: flex; justify-content: space-between; align-items: center;",
                                                div { style: "display: flex; align-items: center; gap: 6px;",
                                                    div { style: "width: 8px; height: 8px; border-radius: 50%; background: {dot_color};" }
                                                    span { style: "font-size: 11px; font-weight: 600; color: var(--text-muted);", "{h_title}" }
                                                }
                                                button {
                                                    class: "btn btn-icon",
                                                    style: "padding: 2px 6px; font-size: 11px;",
                                                    title: translate(locale, "reader.highlight_delete"),
                                                    onclick: {
                                                        let id = h_id.clone();
                                                        move |_| {
                                                            let mut mut_sess = session.write();
                                                            if let Some(s) = mut_sess.as_mut() {
                                                                s.remove_highlight(&id);
                                                            }
                                                        }
                                                    },
                                                    "🗑"
                                                }
                                            }
                                            span {
                                                style: "font-size: 12px; color: var(--text-primary); font-style: italic; border-left: 2px solid {dot_color}; padding-left: 8px; line-height: 1.4; cursor: pointer;",
                                                onclick: move |_| jump_and_notify(session, active_drawer, on_progress_update, h_ch),
                                                "« {h_text} »"
                                            }
                                            if let Some(note) = h_note {
                                                span { style: "font-size: 11px; color: var(--accent-primary); margin-top: 4px; padding-left: 10px;",
                                                    "💬 {note}"
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
}
