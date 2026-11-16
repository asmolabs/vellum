//! Barre d'en-tête de la vue bibliothèque (titre de filtre actif, puces série/tag, changement de vue, sélection et tri).

use std::collections::HashSet;

use dioxus::prelude::*;
use vellum_core::{translate, translate_fmt, Locale};

use crate::state::{FilterStatus, SortOption, ViewMode};

#[component]
pub fn LibraryToolbar(
    locale: Locale,
    current_filter: Signal<FilterStatus>,
    mut selected_series: Signal<Option<String>>,
    mut selected_tag: Signal<Option<String>>,
    filtered_count: usize,
    filtered_ids: Vec<String>,
    mut selected_ids: Signal<HashSet<String>>,
    mut view_mode: Signal<ViewMode>,
    mut current_sort: Signal<SortOption>,
) -> Element {
    let cur_view = *view_mode.read();
    let count_str = filtered_count.to_string();

    rsx! {
        div { class: "library-header",
            div { class: "library-title-group",
                div { style: "display: flex; align-items: center; gap: 10px;",
                    h2 { {translate(locale, current_filter.read().i18n_key())} }
                    if let Some(ref s) = *selected_series.read() {
                        div {
                            style: "display: flex; align-items: center; gap: 6px; padding: 2px 8px; border-radius: 6px; background: rgba(167, 139, 250, 0.2); border: 1px solid rgba(167, 139, 250, 0.4); font-size: 12px; color: #c4b5fd;",
                            span { "📚 {s}" }
                            button {
                                style: "background: none; border: none; color: inherit; cursor: pointer; padding: 0 2px;",
                                onclick: move |_| selected_series.set(None),
                                "✕"
                            }
                        }
                    }
                    if let Some(ref t) = *selected_tag.read() {
                        div {
                            style: "display: flex; align-items: center; gap: 6px; padding: 2px 8px; border-radius: 6px; background: rgba(99, 102, 241, 0.2); border: 1px solid rgba(99, 102, 241, 0.4); font-size: 12px; color: #a5b4fc;",
                            span { "🏷️ {t}" }
                            button {
                                style: "background: none; border: none; color: inherit; cursor: pointer; padding: 0 2px;",
                                onclick: move |_| selected_tag.set(None),
                                "✕"
                            }
                        }
                    }
                }
                div { class: "library-count",
                    {translate_fmt(locale, "toolbar.books_displayed", &[("count", &count_str)])}
                }
            }

            div { style: "display: flex; align-items: center; gap: 8px;",
                div { style: "display: flex; align-items: center; background: rgba(255,255,255,0.06); border-radius: 8px; padding: 2px; border: 1px solid var(--border-subtle); margin-right: 4px;",
                    for (mode, label_key) in [
                        (ViewMode::Grid, "toolbar.view_grid"),
                        (ViewMode::Table, "toolbar.view_table"),
                        (ViewMode::TableCarousel, "toolbar.view_carousel"),
                        (ViewMode::Kanban, "toolbar.view_kanban"),
                    ] {
                        button {
                            style: if cur_view == mode {
                                "padding: 4px 10px; font-size: 12px; border-radius: 6px; background: #6366f1; color: #fff; border: none; font-weight: 600; cursor: pointer;"
                            } else {
                                "padding: 4px 10px; font-size: 12px; border-radius: 6px; background: transparent; color: var(--text-secondary); border: none; cursor: pointer;"
                            },
                            onclick: move |_| view_mode.set(mode),
                            {translate(locale, label_key)}
                        }
                    }
                }

                if !filtered_ids.is_empty() {
                    {
                        let all_selected = filtered_ids.iter().all(|id| selected_ids.read().contains(id));
                        let current_ids = filtered_ids.clone();
                        rsx! {
                            button {
                                class: "btn btn-secondary",
                                style: "padding: 6px 12px; font-size: 12px; border-radius: 8px; display: flex; align-items: center; gap: 6px; cursor: pointer;",
                                onclick: move |_| {
                                    let mut set = selected_ids.read().clone();
                                    if all_selected {
                                        for id in &current_ids {
                                            set.remove(id);
                                        }
                                    } else {
                                        for id in &current_ids {
                                            set.insert(id.clone());
                                        }
                                    }
                                    selected_ids.set(set);
                                },
                                {if all_selected { translate(locale, "toolbar.deselect_all") } else { translate(locale, "toolbar.select_all") }}
                            }
                        }
                    }
                }

                span { style: "font-size: 12px; color: var(--text-muted); font-weight: 500;", {translate(locale, "toolbar.sort_by")} }
                select {
                    style: "padding: 6px 12px; font-size: 12px; border-radius: 8px; background: rgba(255,255,255,0.05); color: var(--text-primary); border: 1px solid var(--border-subtle); outline: none; cursor: pointer;",
                    value: match *current_sort.read() {
                        SortOption::RecentAdded => "recent",
                        SortOption::TitleAsc => "title_asc",
                        SortOption::TitleDesc => "title_desc",
                        SortOption::AuthorAsc => "author_asc",
                        SortOption::ProgressDesc => "progress",
                    },
                    onchange: move |evt| {
                        let val = evt.value();
                        let sort = match val.as_str() {
                            "title_asc" => SortOption::TitleAsc,
                            "title_desc" => SortOption::TitleDesc,
                            "author_asc" => SortOption::AuthorAsc,
                            "progress" => SortOption::ProgressDesc,
                            _ => SortOption::RecentAdded,
                        };
                        current_sort.set(sort);
                    },
                    option { value: "recent", {translate(locale, "toolbar.sort_recent")} }
                    option { value: "title_asc", {translate(locale, "toolbar.sort_title_asc")} }
                    option { value: "title_desc", {translate(locale, "toolbar.sort_title_desc")} }
                    option { value: "author_asc", {translate(locale, "toolbar.sort_author_asc")} }
                    option { value: "progress", {translate(locale, "toolbar.sort_progress")} }
                }
            }
        }
    }
}
