//! Barre d'en-tête de la vue bibliothèque (inspirée de BrowseHeader dans BookOrbit).

use std::collections::HashSet;

use dioxus::prelude::*;
use vellum_core::{translate, translate_fmt, Locale};

use crate::components::{IconKind, LucideIcon};
use crate::state::{FilterStatus, MediaType, SortOption, ViewMode};

#[component]
pub fn LibraryToolbar(
    locale: Locale,
    mut current_filter: Signal<FilterStatus>,
    mut media_type: Signal<MediaType>,
    mut selected_series: Signal<Option<String>>,
    mut selected_tag: Signal<Option<String>>,
    filtered_count: usize,
    filtered_ids: Vec<String>,
    mut selected_ids: Signal<HashSet<String>>,
    mut view_mode: Signal<ViewMode>,
    mut current_sort: Signal<SortOption>,
) -> Element {
    let cur_view = *view_mode.read();
    let cur_media = *media_type.read();
    let cur_filter = *current_filter.read();
    let count_str = filtered_count.to_string();
    let heading = if cur_media != MediaType::All && cur_filter == FilterStatus::All {
        translate(locale, cur_media.i18n_key())
    } else {
        translate(locale, cur_filter.i18n_key())
    };
    let title_icon = match cur_media {
        MediaType::All => IconKind::Library,
        MediaType::Books => IconKind::BookOpen,
        MediaType::Audiobooks => IconKind::Headphones,
        MediaType::Comics => IconKind::Image,
    };

    rsx! {
        div { class: "library-header",
            // Row 1 : Title Row (BookOrbit BrowseHeader style)
            div { class: "browse-title-row",
                div { class: "library-title-group",
                    div { class: "browse-eyebrow", {translate(locale, "nav.libraries")} }
                    div { style: "display: flex; align-items: center; gap: 10px; flex-wrap: wrap;",
                        span {
                            style: "width: 30px; height: 30px; border-radius: 8px; background: rgba(99, 102, 241, 0.16); color: #818cf8; display: inline-flex; align-items: center; justify-content: center;",
                            LucideIcon { kind: title_icon, size: 16 }
                        }
                        h2 { "{heading}" }
                        span { class: "library-count", style: "margin-top: 0; margin-left: 2px;",
                            {translate_fmt(locale, "toolbar.books_displayed", &[("count", &count_str)])}
                        }
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
                }
            }

            // Row 2 : Command Bar (BookOrbit BrowseHeader bar)
            div { class: "browse-command-bar",
                div { style: "display: flex; align-items: center; gap: 8px; flex-wrap: wrap;",
                    // Media switcher pills
                    div { style: "display: flex; align-items: center; background: rgba(255,255,255,0.04); border-radius: 8px; padding: 2px; border: 1px solid var(--border-subtle);",
                        for mt in [
                            MediaType::All,
                            MediaType::Books,
                            MediaType::Audiobooks,
                            MediaType::Comics,
                        ] {
                            button {
                                style: if cur_media == mt {
                                    "padding: 4px 10px; font-size: 11.5px; border-radius: 6px; background: rgba(99, 102, 241, 0.22); color: #c7d2fe; border: none; font-weight: 600; cursor: pointer;"
                                } else {
                                    "padding: 4px 10px; font-size: 11.5px; border-radius: 6px; background: transparent; color: var(--text-muted); border: none; cursor: pointer;"
                                },
                                onclick: move |_| media_type.set(mt),
                                {translate(locale, mt.i18n_key())}
                            }
                        }
                    }

                    span { style: "width: 1px; height: 18px; background: var(--border-subtle);" }

                    // Reading status quick filter
                    select {
                        style: "padding: 4px 10px; font-size: 12px; border-radius: 7px; background: rgba(255,255,255,0.04); color: var(--text-primary); border: 1px solid var(--border-subtle); outline: none; cursor: pointer;",
                        value: match cur_filter {
                            FilterStatus::All => "all",
                            FilterStatus::Reading => "reading",
                            FilterStatus::Finished => "finished",
                            FilterStatus::ToRead => "to_read",
                        },
                        onchange: move |e| {
                            let f = match e.value().as_str() {
                                "reading" => FilterStatus::Reading,
                                "finished" => FilterStatus::Finished,
                                "to_read" => FilterStatus::ToRead,
                                _ => FilterStatus::All,
                            };
                            current_filter.set(f);
                        },
                        option { value: "all", {translate(locale, "nav.all_books")} }
                        option { value: "reading", {translate(locale, "nav.reading")} }
                        option { value: "finished", {translate(locale, "nav.finished")} }
                        option { value: "to_read", {translate(locale, "nav.to_read")} }
                    }

                    // Sort selector
                    select {
                        style: "padding: 4px 10px; font-size: 12px; border-radius: 7px; background: rgba(255,255,255,0.04); color: var(--text-primary); border: 1px solid var(--border-subtle); outline: none; cursor: pointer;",
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
                        option { value: "author_asc", {translate(locale, "toolbar.sort_author")} }
                        option { value: "progress", {translate(locale, "toolbar.sort_progress")} }
                    }
                }

                // Right controls: Selection + View Mode icons (BookOrbit BrowseViewControls)
                div { style: "display: flex; align-items: center; gap: 8px;",
                    if !filtered_ids.is_empty() {
                        {
                            let all_selected = filtered_ids.iter().all(|id| selected_ids.read().contains(id));
                            let current_ids = filtered_ids.clone();
                            rsx! {
                                button {
                                    class: "btn btn-secondary",
                                    style: "padding: 4px 10px; font-size: 12px; border-radius: 7px; display: flex; align-items: center; gap: 6px; cursor: pointer;",
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

                    div { style: "display: flex; align-items: center; background: rgba(255,255,255,0.04); border-radius: 8px; padding: 2px; border: 1px solid var(--border-subtle);",
                        for (mode, icon_kind, label_key) in [
                            (ViewMode::Grid, IconKind::Grid, "toolbar.view_grid"),
                            (ViewMode::Table, IconKind::List, "toolbar.view_table"),
                            (ViewMode::TableCarousel, IconKind::Columns, "toolbar.view_carousel"),
                            (ViewMode::Kanban, IconKind::LayoutDashboard, "toolbar.view_kanban"),
                        ] {
                            button {
                                title: translate(locale, label_key),
                                style: if cur_view == mode {
                                    "width: 28px; height: 26px; display: inline-flex; align-items: center; justify-content: center; border-radius: 6px; background: rgba(99, 102, 241, 0.25); color: #c7d2fe; border: none; cursor: pointer;"
                                } else {
                                    "width: 28px; height: 26px; display: inline-flex; align-items: center; justify-content: center; border-radius: 6px; background: transparent; color: var(--text-muted); border: none; cursor: pointer;"
                                },
                                onclick: move |_| view_mode.set(mode),
                                LucideIcon { kind: icon_kind, size: 14 }
                            }
                        }
                    }
                }
            }
        }
    }
}
