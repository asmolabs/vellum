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
    media_type: Signal<MediaType>,
    mut search_query: Signal<String>,
    mut selected_series: Signal<Option<String>>,
    mut selected_tag: Signal<Option<String>>,
    mut format_filter: Signal<Option<String>>,
    mut min_rating: Signal<Option<i32>>,
    mut collapse_series: Signal<bool>,
    on_shuffle: EventHandler<()>,
    filtered_count: usize,
    filtered_ids: Vec<String>,
    mut selected_ids: Signal<HashSet<String>>,
    mut view_mode: Signal<ViewMode>,
    mut current_sort: Signal<SortOption>,
) -> Element {
    let mut filter_open = use_signal(|| false);
    let cur_view = *view_mode.read();
    let cur_media = *media_type.read();
    let cur_filter = *current_filter.read();
    let cur_query = search_query.read().clone();
    let cur_fmt = format_filter.read().clone();
    let cur_rating = *min_rating.read();
    let is_collapsed = *collapse_series.read();
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

    let active_filter_count = usize::from(cur_filter != FilterStatus::All)
        + usize::from(selected_series.read().is_some())
        + usize::from(selected_tag.read().is_some())
        + usize::from(cur_fmt.is_some())
        + usize::from(cur_rating.is_some());

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

            // Row 2 : Command Bar (BookOrbit BrowseSearchField + BrowseSortControl + BrowseFilterControl + Shuffle + BrowseViewControls)
            div { class: "browse-command-bar", style: "position: relative;",
                // Search field (flex-1, BookOrbit BrowseSearchField style)
                div {
                    style: "flex: 1; min-width: 180px; display: flex; align-items: center; gap: 8px; padding: 0 8px; height: 32px; border-radius: 7px; background: rgba(255,255,255,0.03);",
                    span { style: "color: var(--text-muted); display: flex; align-items: center;",
                        LucideIcon { kind: IconKind::Search, size: 14 }
                    }
                    input {
                        r#type: "text",
                        value: "{cur_query}",
                        placeholder: translate(locale, "action.search"),
                        style: "flex: 1; background: transparent; border: none; outline: none; color: var(--text-primary); font-size: 13px; min-width: 0;",
                        oninput: move |evt| search_query.set(evt.value()),
                    }
                    if !cur_query.is_empty() {
                        button {
                            title: translate(locale, "header.clear_search"),
                            style: "background: none; border: none; color: var(--text-muted); cursor: pointer; font-size: 12px; padding: 2px 4px;",
                            onclick: move |_| search_query.set(String::new()),
                            "✕"
                        }
                    }
                }

                span { style: "width: 1px; height: 20px; background: var(--border-subtle);" }

                // Sort control
                select {
                    style: "height: 30px; padding: 0 10px; font-size: 12px; border-radius: 7px; background: rgba(255,255,255,0.04); color: var(--text-primary); border: 1px solid var(--border-subtle); outline: none; cursor: pointer;",
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

                // Filter control (BookOrbit BrowseFilterControl + BookFilterBuilder)
                div { style: "position: relative;",
                    button {
                        style: if active_filter_count > 0 || *filter_open.read() {
                            "height: 30px; padding: 0 10px; font-size: 12px; border-radius: 7px; background: rgba(99, 102, 241, 0.2); color: #c7d2fe; border: 1px solid rgba(99, 102, 241, 0.45); display: inline-flex; align-items: center; gap: 6px; cursor: pointer; font-weight: 600;"
                        } else {
                            "height: 30px; padding: 0 10px; font-size: 12px; border-radius: 7px; background: rgba(255,255,255,0.04); color: var(--text-secondary); border: 1px solid var(--border-subtle); display: inline-flex; align-items: center; gap: 6px; cursor: pointer;"
                        },
                        onclick: move |_| {
                            let next = !*filter_open.read();
                            filter_open.set(next);
                        },
                        LucideIcon { kind: IconKind::Filter, size: 13 }
                        span { "Filtres" }
                        if active_filter_count > 0 {
                            span {
                                style: "min-width: 17px; height: 17px; padding: 0 4px; border-radius: 999px; background: #6366f1; color: #fff; font-size: 10px; font-weight: 700; display: inline-flex; align-items: center; justify-content: center;",
                                "{active_filter_count}"
                            }
                        }
                    }

                    if *filter_open.read() {
                        div {
                            style: "position: absolute; top: 38px; right: 0; width: 270px; background: var(--bg-card); border: 1px solid var(--border-active); border-radius: 10px; padding: 12px; box-shadow: 0 12px 30px rgba(0,0,0,0.45); z-index: 40; display: flex; flex-direction: column; gap: 10px;",
                            // Statut
                            div {
                                div { style: "font-size: 11px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em; color: var(--text-muted); margin-bottom: 4px;",
                                    {translate(locale, "nav.reading_zone")}
                                }
                                div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 4px;",
                                    for f in [
                                        FilterStatus::All,
                                        FilterStatus::Reading,
                                        FilterStatus::Finished,
                                        FilterStatus::ToRead,
                                    ] {
                                        button {
                                            style: if cur_filter == f {
                                                "text-align: left; padding: 5px 8px; border-radius: 6px; background: rgba(99, 102, 241, 0.22); color: #c7d2fe; border: none; font-size: 12px; font-weight: 600; cursor: pointer;"
                                            } else {
                                                "text-align: left; padding: 5px 8px; border-radius: 6px; background: rgba(255,255,255,0.03); color: var(--text-secondary); border: none; font-size: 12px; cursor: pointer;"
                                            },
                                            onclick: move |_| current_filter.set(f),
                                            {translate(locale, f.i18n_key())}
                                        }
                                    }
                                }
                            }

                            // Format
                            div {
                                div { style: "font-size: 11px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em; color: var(--text-muted); margin-bottom: 4px;",
                                    "Format"
                                }
                                div { style: "display: flex; flex-wrap: wrap; gap: 4px;",
                                    for (label, val) in [
                                        ("Tous", None),
                                        ("EPUB", Some("epub")),
                                        ("PDF", Some("pdf")),
                                        ("CBZ/CBR", Some("cbz")),
                                        ("M4B/MP3", Some("m4b")),
                                    ] {
                                        {
                                            let is_on = cur_fmt.as_deref() == val;
                                            rsx! {
                                                button {
                                                    style: if is_on {
                                                        "padding: 4px 8px; border-radius: 6px; background: rgba(99, 102, 241, 0.22); color: #c7d2fe; border: none; font-size: 11.5px; font-weight: 600; cursor: pointer;"
                                                    } else {
                                                        "padding: 4px 8px; border-radius: 6px; background: rgba(255,255,255,0.03); color: var(--text-secondary); border: none; font-size: 11.5px; cursor: pointer;"
                                                    },
                                                    onclick: move |_| format_filter.set(val.map(str::to_string)),
                                                    "{label}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            // Note minimale
                            div {
                                div { style: "font-size: 11px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em; color: var(--text-muted); margin-bottom: 4px;",
                                    "Note minimale"
                                }
                                div { style: "display: flex; gap: 4px;",
                                    for (label, val) in [
                                        ("Toutes", None),
                                        ("3+ ★", Some(3_i32)),
                                        ("4+ ★", Some(4_i32)),
                                        ("5 ★", Some(5_i32)),
                                    ] {
                                        {
                                            let is_on = cur_rating == val;
                                            rsx! {
                                                button {
                                                    style: if is_on {
                                                        "padding: 4px 8px; border-radius: 6px; background: rgba(251, 191, 36, 0.2); color: #fde68a; border: none; font-size: 11.5px; font-weight: 600; cursor: pointer;"
                                                    } else {
                                                        "padding: 4px 8px; border-radius: 6px; background: rgba(255,255,255,0.03); color: var(--text-secondary); border: none; font-size: 11.5px; cursor: pointer;"
                                                    },
                                                    onclick: move |_| min_rating.set(val),
                                                    "{label}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            if active_filter_count > 0 {
                                div { style: "border-top: 1px solid var(--border-subtle); padding-top: 8px;",
                                    button {
                                        style: "width: 100%; padding: 5px 8px; border-radius: 6px; background: rgba(255,255,255,0.05); color: var(--text-muted); border: none; font-size: 12px; cursor: pointer;",
                                        onclick: move |_| {
                                            current_filter.set(FilterStatus::All);
                                            selected_series.set(None);
                                            selected_tag.set(None);
                                            format_filter.set(None);
                                            min_rating.set(None);
                                            filter_open.set(false);
                                        },
                                        {translate(locale, "header.clear_search")}
                                    }
                                }
                            }
                        }
                    }
                }

                // Shuffle (BookShuffleButton) & Series Collapse toggle
                button {
                    title: "Livre au hasard",
                    style: "width: 30px; height: 30px; display: inline-flex; align-items: center; justify-content: center; border-radius: 7px; background: rgba(255,255,255,0.04); color: var(--text-secondary); border: 1px solid var(--border-subtle); cursor: pointer;",
                    onclick: move |_| on_shuffle.call(()),
                    LucideIcon { kind: IconKind::Shuffle, size: 14 }
                }

                if cur_view == ViewMode::Grid {
                    button {
                        title: "Regrouper les séries",
                        style: if is_collapsed {
                            "width: 30px; height: 30px; display: inline-flex; align-items: center; justify-content: center; border-radius: 7px; background: rgba(99, 102, 241, 0.25); color: #c7d2fe; border: 1px solid rgba(99, 102, 241, 0.45); cursor: pointer;"
                        } else {
                            "width: 30px; height: 30px; display: inline-flex; align-items: center; justify-content: center; border-radius: 7px; background: rgba(255,255,255,0.04); color: var(--text-secondary); border: 1px solid var(--border-subtle); cursor: pointer;"
                        },
                        onclick: move |_| collapse_series.set(!is_collapsed),
                        LucideIcon { kind: IconKind::Layers, size: 14 }
                    }
                }

                span { style: "width: 1px; height: 20px; background: var(--border-subtle);" }

                // Right controls: Selection + View Mode icons (BookOrbit BrowseViewControls)
                div { style: "display: flex; align-items: center; gap: 8px;",
                    if !filtered_ids.is_empty() {
                        {
                            let all_selected = filtered_ids.iter().all(|id| selected_ids.read().contains(id));
                            let current_ids = filtered_ids.clone();
                            rsx! {
                                button {
                                    class: "btn btn-secondary",
                                    style: "height: 30px; padding: 0 10px; font-size: 12px; border-radius: 7px; display: flex; align-items: center; gap: 6px; cursor: pointer;",
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
