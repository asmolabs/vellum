//! Onglet d'édition de la table des matières (TOC) d'un EPUB.

use dioxus::prelude::*;
use vellum_core::{translate, translate_fmt, Locale};
use vellum_domain::EpubTocEntry;

#[component]
pub(super) fn EpubTocTab(
    locale: Locale,
    mut toc_entries: Signal<Vec<EpubTocEntry>>,
    is_operating: Signal<bool>,
    on_auto_generate_toc: EventHandler<()>,
    on_save_toc: EventHandler<()>,
) -> Element {
    rsx! {
        div { style: "display: flex; justify-content: space-between; align-items: center; padding-bottom: 8px; border-bottom: 1px solid var(--border-color);",
            div { style: "display: flex; gap: 8px;",
                button {
                    class: "btn btn-secondary",
                    style: "font-size: 12px; padding: 6px 12px; font-weight: 600;",
                    disabled: *is_operating.read(),
                    onclick: move |_| on_auto_generate_toc.call(()),
                    {translate(locale, "epub.toc_auto_detect")}
                }
                button {
                    class: "btn btn-secondary",
                    style: "font-size: 12px; padding: 6px 12px;",
                    disabled: *is_operating.read(),
                    onclick: move |_| {
                        let mut list = toc_entries.read().clone();
                        let next_idx = list.len() + 1;
                        let idx_str = next_idx.to_string();
                        list.push(EpubTocEntry {
                            id: format!("navpoint-{next_idx}"),
                            label: translate_fmt(locale, "epub.toc_new_chapter", &[("idx", &idx_str)]),
                            href: "ch1.xhtml".to_string(),
                            play_order: next_idx,
                            children: Vec::new(),
                        });
                        toc_entries.set(list);
                    },
                    {translate(locale, "epub.toc_add")}
                }
            }

            button {
                class: "btn btn-primary",
                style: "font-size: 12px; padding: 6px 16px; font-weight: 600;",
                disabled: *is_operating.read(),
                onclick: move |_| on_save_toc.call(()),
                {if *is_operating.read() { translate(locale, "epub.toc_saving") } else { translate(locale, "epub.toc_save") }}
            }
        }

        // Chapters list
        if toc_entries.read().is_empty() {
            div { style: "text-align: center; padding: 32px 16px; color: var(--text-secondary);",
                p { style: "font-size: 14px; margin-bottom: 12px;", {translate(locale, "epub.toc_empty")} }
                button {
                    class: "btn btn-primary",
                    style: "font-size: 13px;",
                    onclick: move |_| on_auto_generate_toc.call(()),
                    {translate(locale, "epub.toc_generate")}
                }
            }
        } else {
            div { style: "display: flex; flex-direction: column; gap: 8px;",
                for (idx, entry) in toc_entries.read().iter().enumerate() {
                    {
                        let item_idx = idx;
                        let item_label = entry.label.clone();
                        let item_href = entry.href.clone();
                        let can_up = item_idx > 0;
                        let can_down = item_idx + 1 < toc_entries.read().len();

                        rsx! {
                            div {
                                key: "{item_idx}",
                                style: "display: flex; align-items: center; gap: 8px; padding: 8px 10px; background: var(--bg-card); border: 1px solid var(--border-color); border-radius: 6px;",

                                // Index badge
                                span { style: "font-size: 11px; font-weight: 700; color: var(--text-secondary); width: 26px; text-align: center;",
                                    "#{item_idx + 1}"
                                }

                                // Title Input
                                input {
                                    class: "drawer-input",
                                    style: "flex: 1; padding: 6px 10px; font-size: 13px;",
                                    value: "{item_label}",
                                    oninput: move |e| {
                                        let mut list = toc_entries.read().clone();
                                        if item_idx < list.len() {
                                            list[item_idx].label = e.value();
                                            toc_entries.set(list);
                                        }
                                    },
                                }

                                // Target Href chip
                                span {
                                    style: "font-size: 11px; font-family: monospace; color: var(--text-secondary); background: rgba(255, 255, 255, 0.05); padding: 4px 8px; border-radius: 4px; max-width: 180px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;",
                                    title: "{item_href}",
                                    "{item_href}"
                                }

                                // Reorder buttons
                                button {
                                    class: "btn btn-icon",
                                    style: "width: 28px; height: 28px; font-size: 10px;",
                                    disabled: !can_up,
                                    onclick: move |_| {
                                        let mut list = toc_entries.read().clone();
                                        if item_idx > 0 && item_idx < list.len() {
                                            list.swap(item_idx, item_idx - 1);
                                            for (i, item) in list.iter_mut().enumerate() {
                                                item.play_order = i + 1;
                                            }
                                            toc_entries.set(list);
                                        }
                                    },
                                    "▲"
                                }

                                button {
                                    class: "btn btn-icon",
                                    style: "width: 28px; height: 28px; font-size: 10px;",
                                    disabled: !can_down,
                                    onclick: move |_| {
                                        let mut list = toc_entries.read().clone();
                                        if item_idx + 1 < list.len() {
                                            list.swap(item_idx, item_idx + 1);
                                            for (i, item) in list.iter_mut().enumerate() {
                                                item.play_order = i + 1;
                                            }
                                            toc_entries.set(list);
                                        }
                                    },
                                    "▼"
                                }

                                // Delete button
                                button {
                                    class: "btn btn-icon",
                                    style: "width: 28px; height: 28px; font-size: 12px; color: #f87171;",
                                    onclick: move |_| {
                                        let mut list = toc_entries.read().clone();
                                        if item_idx < list.len() {
                                            list.remove(item_idx);
                                            for (i, item) in list.iter_mut().enumerate() {
                                                item.play_order = i + 1;
                                            }
                                            toc_entries.set(list);
                                        }
                                    },
                                    "🗑️"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
