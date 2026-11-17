use crate::components::{IconKind, LucideIcon};
use crate::state::{FilterStatus, MediaType, ViewMode};
use dioxus::prelude::*;
use vellum_core::{translate, Locale};
use vellum_domain::{ReadingStats, VirtualLibrary};

#[component]
pub fn Sidebar(
    locale: Locale,
    mut current_filter: Signal<FilterStatus>,
    mut media_type: Signal<MediaType>,
    mut view_mode: Signal<ViewMode>,
    mut sidebar_collapsed: Signal<bool>,
    mut search_query: Signal<String>,
    on_open_palette: EventHandler<()>,
    on_open_content_search: EventHandler<()>,
    mut selected_tag: Signal<Option<String>>,
    mut selected_series: Signal<Option<String>>,
    media_counts: (usize, usize, usize),
    tags_counts: Vec<(String, usize)>,
    series_counts: Vec<(String, usize)>,
    virtual_libraries: Vec<VirtualLibrary>,
    active_virtual_library_id: Signal<Option<String>>,
    on_create_virtual_library: EventHandler<(String, String)>,
    on_delete_virtual_library: EventHandler<String>,
    stats: Option<ReadingStats>,
    on_open_stats: EventHandler<()>,
) -> Element {
    let mut show_new_vl = use_signal(|| false);
    let mut new_vl_name = use_signal(String::new);
    let mut new_vl_query = use_signal(String::new);
    let mut show_series = use_signal(|| true);
    let mut show_tags = use_signal(|| true);
    let collapsed = *sidebar_collapsed.read();
    let cur_view = *view_mode.read();

    let filters = [
        (FilterStatus::All, IconKind::Library),
        (FilterStatus::Reading, IconKind::BookOpen),
        (FilterStatus::Finished, IconKind::CheckCircle),
        (FilterStatus::ToRead, IconKind::Bookmark),
    ];

    if collapsed {
        return rsx! {
            aside {
                class: "sidebar",
                style: "width: 56px; min-width: 56px; padding: 12px 6px; align-items: center; gap: 8px;",
                button {
                    class: "btn btn-icon",
                    style: "width: 40px; height: 40px; border-radius: 10px; display: flex; align-items: center; justify-content: center;",
                    title: translate(locale, "header.command_palette"),
                    onclick: move |_| on_open_palette.call(()),
                    LucideIcon { kind: IconKind::Search, size: 18 }
                }
                button {
                    class: if cur_view == ViewMode::Home { "btn btn-primary" } else { "btn btn-icon" },
                    style: "width: 40px; height: 40px; border-radius: 10px; display: flex; align-items: center; justify-content: center;",
                    title: translate(locale, "toolbar.view_home"),
                    onclick: move |_| view_mode.set(ViewMode::Home),
                    LucideIcon { kind: IconKind::Home, size: 18 }
                }
                div { style: "width: 28px; height: 1px; background: var(--border-subtle); margin: 4px 0;" }
                for (mt, icon_kind) in [
                    (MediaType::Books, IconKind::BookOpen),
                    (MediaType::Audiobooks, IconKind::Headphones),
                    (MediaType::Comics, IconKind::Image),
                ] {
                    {
                        let is_active = cur_view != ViewMode::Home && *media_type.read() == mt;
                        let label = translate(locale, mt.i18n_key());
                        rsx! {
                            button {
                                class: if is_active { "btn btn-primary" } else { "btn btn-icon" },
                                style: "width: 40px; height: 40px; border-radius: 10px; display: flex; align-items: center; justify-content: center;",
                                title: "{label}",
                                onclick: move |_| {
                                    media_type.set(mt);
                                    current_filter.set(FilterStatus::All);
                                    selected_tag.set(None);
                                    selected_series.set(None);
                                    if matches!(*view_mode.read(), ViewMode::Home | ViewMode::Authors | ViewMode::Series | ViewMode::History) {
                                        view_mode.set(ViewMode::Grid);
                                    }
                                },
                                LucideIcon { kind: icon_kind, size: 18 }
                            }
                        }
                    }
                }
                div { style: "width: 28px; height: 1px; background: var(--border-subtle); margin: 4px 0;" }
                for (f, icon_kind) in filters.iter().copied() {
                    {
                        let is_active = cur_view != ViewMode::Home && *current_filter.read() == f && selected_series.read().is_none() && selected_tag.read().is_none();
                        let label = translate(locale, f.i18n_key());
                        rsx! {
                            button {
                                class: if is_active { "btn btn-primary" } else { "btn btn-icon" },
                                style: "width: 40px; height: 40px; border-radius: 10px; display: flex; align-items: center; justify-content: center;",
                                title: "{label}",
                                onclick: move |_| {
                                    current_filter.set(f);
                                    selected_tag.set(None);
                                    selected_series.set(None);
                                    if *view_mode.read() == ViewMode::Home {
                                        view_mode.set(ViewMode::Grid);
                                    }
                                },
                                LucideIcon { kind: icon_kind, size: 18 }
                            }
                        }
                    }
                }
                div { style: "flex: 1;" }
                button {
                    class: "btn btn-icon",
                    style: "width: 40px; height: 40px; border-radius: 10px; display: flex; align-items: center; justify-content: center;",
                    title: translate(locale, "sidebar.stats_tooltip"),
                    onclick: move |_| on_open_stats.call(()),
                    LucideIcon { kind: IconKind::BarChart, size: 18 }
                }
            }
        };
    }

    rsx! {
        aside { class: "sidebar",
            // Recherche globale dans la sidebar façon BookOrbit (⌘K + recherche plein texte)
            div { style: "display: flex; align-items: center; gap: 6px; margin-bottom: 4px;",
                button {
                    class: "nav-item",
                    style: "flex: 1; background: rgba(255,255,255,0.04); border: 1px solid var(--border-subtle); border-radius: 8px; padding: 6px 10px; cursor: pointer; color: var(--text-secondary);",
                    onclick: move |_| on_open_palette.call(()),
                    span { style: "display: flex; align-items: center; gap: 8px; font-size: 13px;",
                        LucideIcon { kind: IconKind::Search, size: 15 }
                        span { "{translate(locale, \"action.search\")}" }
                    }
                    span {
                        style: "font-family: monospace; font-size: 10px; padding: 1px 5px; border-radius: 4px; background: rgba(255,255,255,0.07); border: 1px solid var(--border-subtle); color: var(--text-muted);",
                        "⌘K"
                    }
                }
                button {
                    class: "btn btn-icon",
                    style: "width: 32px; height: 32px; border-radius: 8px; background: rgba(255,255,255,0.04); border: 1px solid var(--border-subtle); display: flex; align-items: center; justify-content: center;",
                    title: translate(locale, "header.content_search_tooltip"),
                    onclick: move |_| on_open_content_search.call(()),
                    LucideIcon { kind: IconKind::BookOpen, size: 14 }
                }
            }
            if !search_query.read().is_empty() {
                div {
                    style: "display: flex; align-items: center; justify-content: space-between; padding: 4px 8px; border-radius: 6px; background: rgba(99, 102, 241, 0.15); border: 1px solid rgba(99, 102, 241, 0.3); font-size: 11px; color: #c7d2fe;",
                    span { style: "overflow: hidden; text-overflow: ellipsis; white-space: nowrap;", "\"{search_query}\"" }
                    button {
                        style: "background: none; border: none; color: inherit; cursor: pointer; padding: 0 4px;",
                        onclick: move |_| search_query.set(String::new()),
                        "✕"
                    }
                }
            }

            // Zone 1 : Primary (épinglé en haut comme dans BookOrbit)
            div { style: "display: flex; justify-content: space-between; align-items: center;",
                div {
                    class: if cur_view == ViewMode::Home { "nav-item active" } else { "nav-item" },
                    style: "flex: 1;",
                    onclick: move |_| view_mode.set(ViewMode::Home),
                    span { style: "display: flex; align-items: center; gap: 8px;",
                        LucideIcon { kind: IconKind::LayoutDashboard, size: 15 }
                        span { "{translate(locale, \"toolbar.view_home\").trim_start_matches(\"🏠 \")}" }
                    }
                }
                button {
                    style: "background: none; border: none; color: var(--text-muted); cursor: pointer; padding: 4px 6px; display: flex; align-items: center;",
                    title: "Collapse",
                    onclick: move |_| sidebar_collapsed.set(true),
                    LucideIcon { kind: IconKind::PanelLeft, size: 14 }
                }
            }

            // Zone 2 : Bibliothèques (séparation Livres, Audio, Comics comme dans BookOrbit)
            div { style: "border-top: 1px solid var(--border-subtle); padding-top: 10px;",
                div { class: "sidebar-section-title", "{translate(locale, \"nav.libraries\")}" }
                nav { class: "nav-list",
                    for (mt, icon_kind, count) in [
                        (MediaType::Books, IconKind::BookOpen, media_counts.0),
                        (MediaType::Audiobooks, IconKind::Headphones, media_counts.1),
                        (MediaType::Comics, IconKind::Image, media_counts.2),
                    ] {
                        {
                            let is_active = cur_view == ViewMode::Grid
                                && *media_type.read() == mt
                                && *current_filter.read() == FilterStatus::All
                                && selected_series.read().is_none()
                                && selected_tag.read().is_none();
                            rsx! {
                                div {
                                    class: if is_active { "nav-item active" } else { "nav-item" },
                                    onclick: move |_| {
                                        media_type.set(mt);
                                        current_filter.set(FilterStatus::All);
                                        selected_tag.set(None);
                                        selected_series.set(None);
                                        view_mode.set(ViewMode::Grid);
                                    },
                                    span { style: "display: flex; align-items: center; gap: 8px;",
                                        LucideIcon { kind: icon_kind, size: 15 }
                                        span { "{translate(locale, mt.i18n_key())}" }
                                    }
                                    span { class: "nav-badge", "{count}" }
                                }
                            }
                        }
                    }
                }
            }

            // Zone 3 : Parcourir (Tout, Séries, Auteurs)
            div { style: "border-top: 1px solid var(--border-subtle); padding-top: 10px;",
                div { class: "sidebar-section-title", "{translate(locale, \"nav.browse\")}" }
                nav { class: "nav-list",
                    div {
                        class: if cur_view == ViewMode::Grid && *media_type.read() == MediaType::All && *current_filter.read() == FilterStatus::All && selected_series.read().is_none() && selected_tag.read().is_none() { "nav-item active" } else { "nav-item" },
                        onclick: move |_| {
                            media_type.set(MediaType::All);
                            current_filter.set(FilterStatus::All);
                            selected_tag.set(None);
                            selected_series.set(None);
                            view_mode.set(ViewMode::Grid);
                        },
                        span { style: "display: flex; align-items: center; gap: 8px;",
                            LucideIcon { kind: IconKind::Library, size: 15 }
                            span { "{translate(locale, \"nav.all_books\")}" }
                        }
                        if let Some(ref s) = stats {
                            span { class: "nav-badge", "{s.total_books}" }
                        }
                    }
                    div {
                        class: if cur_view == ViewMode::Series { "nav-item active" } else { "nav-item" },
                        onclick: move |_| view_mode.set(ViewMode::Series),
                        span { style: "display: flex; align-items: center; gap: 8px;",
                            LucideIcon { kind: IconKind::Layers, size: 15 }
                            span { "{translate(locale, \"nav.series\")}" }
                        }
                        span { class: "nav-badge", "{series_counts.len()}" }
                    }
                    div {
                        class: if cur_view == ViewMode::Authors { "nav-item active" } else { "nav-item" },
                        onclick: move |_| view_mode.set(ViewMode::Authors),
                        span { style: "display: flex; align-items: center; gap: 8px;",
                            LucideIcon { kind: IconKind::Users, size: 15 }
                            span { "{translate(locale, \"nav.authors\")}" }
                        }
                    }
                }
            }

            // Zone 3 : Lecture (En cours, Terminés, À lire, Historique, Statistiques)
            div { style: "border-top: 1px solid var(--border-subtle); padding-top: 10px;",
                div { class: "sidebar-section-title", "{translate(locale, \"nav.reading_zone\")}" }
                nav { class: "nav-list",
                    for (f, icon_kind) in filters.iter().copied().skip(1) {
                        {
                            let is_active = cur_view == ViewMode::Grid && *current_filter.read() == f && selected_series.read().is_none() && selected_tag.read().is_none();
                            let filter_label = translate(locale, f.i18n_key());
                            let count_label = match (f, &stats) {
                                (FilterStatus::Reading, Some(s)) => format!("{}", s.in_progress_books),
                                (FilterStatus::Finished, Some(s)) => format!("{}", s.finished_books),
                                (FilterStatus::ToRead, Some(s)) => format!("{}", s.total_books.saturating_sub(s.in_progress_books + s.finished_books)),
                                _ => "-".to_string(),
                            };

                            rsx! {
                                div {
                                    class: if is_active { "nav-item active" } else { "nav-item" },
                                    onclick: move |_| {
                                        current_filter.set(f);
                                        selected_tag.set(None);
                                        selected_series.set(None);
                                        view_mode.set(ViewMode::Grid);
                                    },
                                    span { style: "display: flex; align-items: center; gap: 8px;",
                                        LucideIcon { kind: icon_kind, size: 15 }
                                        span { "{filter_label}" }
                                    }
                                    span { class: "nav-badge", "{count_label}" }
                                }
                            }
                        }
                    }
                    div {
                        class: if cur_view == ViewMode::History { "nav-item active" } else { "nav-item" },
                        onclick: move |_| view_mode.set(ViewMode::History),
                        span { style: "display: flex; align-items: center; gap: 8px;",
                            LucideIcon { kind: IconKind::History, size: 15 }
                            span { "{translate(locale, \"nav.history\")}" }
                        }
                    }
                    div {
                        class: if cur_view == ViewMode::Annotations { "nav-item active" } else { "nav-item" },
                        onclick: move |_| view_mode.set(ViewMode::Annotations),
                        span { style: "display: flex; align-items: center; gap: 8px;",
                            LucideIcon { kind: IconKind::Highlighter, size: 15 }
                            span { "Annotations" }
                        }
                    }
                    div {
                        class: "nav-item",
                        onclick: move |_| on_open_stats.call(()),
                        span { style: "display: flex; align-items: center; gap: 8px;",
                            LucideIcon { kind: IconKind::BarChart, size: 15 }
                            span { "{translate(locale, \"nav.statistics\")}" }
                        }
                    }
                }
            }

            // Bibliothèques virtuelles
            div { style: "border-top: 1px solid var(--border-subtle); padding-top: 10px;",
                div {
                    class: "sidebar-section-title",
                    style: "display: flex; justify-content: space-between; align-items: center;",
                    span { "{translate(locale, \"sidebar.virtual_libraries\")}" }
                    button {
                        style: "background: none; border: none; color: #818cf8; cursor: pointer; font-size: 14px; font-weight: 700; padding: 0 4px;",
                        title: translate(locale, "sidebar.create_vl"),
                        onclick: move |_| {
                            let next = !*show_new_vl.read();
                            show_new_vl.set(next);
                        },
                        "+"
                    }
                }
                if *show_new_vl.read() {
                    div {
                        style: "display: flex; flex-direction: column; gap: 6px; padding: 6px 8px; margin-bottom: 8px; background: rgba(255,255,255,0.04); border-radius: 6px;",
                        input {
                            r#type: "text",
                            placeholder: translate(locale, "sidebar.vl_name_placeholder"),
                            style: "font-size: 11px; padding: 4px 6px; border-radius: 4px; background: rgba(0,0,0,0.3); color: #e2e8f0; border: 1px solid rgba(255,255,255,0.1);",
                            value: "{new_vl_name}",
                            oninput: move |e| new_vl_name.set(e.value()),
                        }
                        input {
                            r#type: "text",
                            placeholder: translate(locale, "sidebar.vl_query_placeholder"),
                            style: "font-size: 11px; padding: 4px 6px; border-radius: 4px; background: rgba(0,0,0,0.3); color: #e2e8f0; border: 1px solid rgba(255,255,255,0.1);",
                            value: "{new_vl_query}",
                            oninput: move |e| new_vl_query.set(e.value()),
                        }
                        button {
                            style: "font-size: 11px; padding: 4px; border-radius: 4px; background: rgba(99, 102, 241, 0.25); color: #a5b4fc; border: 1px solid rgba(99, 102, 241, 0.4); cursor: pointer;",
                            onclick: move |_| {
                                let name = new_vl_name.read().trim().to_string();
                                let query = new_vl_query.read().trim().to_string();
                                if !name.is_empty() && !query.is_empty() {
                                    on_create_virtual_library.call((name, query));
                                    new_vl_name.set(String::new());
                                    new_vl_query.set(String::new());
                                    show_new_vl.set(false);
                                }
                            },
                            "{translate(locale, \"action.save\")}"
                        }
                    }
                }
                if !virtual_libraries.is_empty() {
                    div { class: "tags-cloud",
                        for vl in virtual_libraries.iter() {
                            {
                                let is_selected = active_virtual_library_id.read().as_ref() == Some(&vl.id);
                                let vl_id = vl.id.clone();
                                let vl_id_del = vl.id.clone();
                                let vl_query = vl.query.clone();
                                rsx! {
                                    span {
                                        class: if is_selected { "tag-pill active" } else { "tag-pill" },
                                        style: "display: inline-flex; align-items: center; gap: 4px; border-color: rgba(99, 102, 241, 0.45); color: #a5b4fc;",
                                        title: "{vl_query}",
                                        onclick: move |_| {
                                            if active_virtual_library_id.read().as_ref() == Some(&vl_id) {
                                                active_virtual_library_id.set(None);
                                            } else {
                                                active_virtual_library_id.set(Some(vl_id.clone()));
                                            }
                                        },
                                        "🗂️ {vl.name}"
                                        button {
                                            style: "background: none; border: none; color: inherit; cursor: pointer; font-size: 10px; opacity: 0.7; padding: 0 2px;",
                                            onclick: move |evt| {
                                                evt.stop_propagation();
                                                on_delete_virtual_library.call(vl_id_del.clone());
                                            },
                                            "×"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Séries & Sagas (repliable)
            if !series_counts.is_empty() {
                div { style: "border-top: 1px solid var(--border-subtle); padding-top: 10px;",
                    div {
                        style: "display: flex; justify-content: space-between; align-items: center; cursor: pointer;",
                        onclick: move |_| {
                            let cur = *show_series.read();
                            show_series.set(!cur);
                        },
                        div { class: "sidebar-section-title", style: "display: flex; align-items: center; gap: 6px;",
                            LucideIcon { kind: IconKind::Layers, size: 13 }
                            span { "{translate(locale, \"nav.series\")}" }
                        }
                        if *show_series.read() {
                            LucideIcon { kind: IconKind::ChevronDown, size: 13 }
                        } else {
                            LucideIcon { kind: IconKind::ChevronRight, size: 13 }
                        }
                    }
                    if *show_series.read() {
                    div { class: "tags-cloud",
                        for (s_name, count) in series_counts.iter() {
                            {
                                let is_selected = selected_series.read().as_ref() == Some(s_name);
                                let s_clone = s_name.clone();
                                rsx! {
                                    span {
                                        class: if is_selected { "tag-pill active" } else { "tag-pill" },
                                        style: "border-color: rgba(167, 139, 250, 0.4); color: #c4b5fd;",
                                        onclick: move |_| {
                                            if selected_series.read().as_ref() == Some(&s_clone) {
                                                selected_series.set(None);
                                            } else {
                                                selected_series.set(Some(s_clone.clone()));
                                                if *view_mode.read() == ViewMode::Home {
                                                    view_mode.set(ViewMode::Grid);
                                                }
                                            }
                                        },
                                        "📚 {s_name} ({count})"
                                    }
                                }
                            }
                        }
                    }
                    }
                }
            }

            // Étiquettes / Tags (repliable)
            if !tags_counts.is_empty() {
                div { style: "border-top: 1px solid var(--border-subtle); padding-top: 10px;",
                    div {
                        style: "display: flex; justify-content: space-between; align-items: center; cursor: pointer;",
                        onclick: move |_| {
                            let cur = *show_tags.read();
                            show_tags.set(!cur);
                        },
                        div { class: "sidebar-section-title", style: "display: flex; align-items: center; gap: 6px;",
                            LucideIcon { kind: IconKind::Tag, size: 13 }
                            span { "{translate(locale, \"nav.tags\")}" }
                        }
                        if *show_tags.read() {
                            LucideIcon { kind: IconKind::ChevronDown, size: 13 }
                        } else {
                            LucideIcon { kind: IconKind::ChevronRight, size: 13 }
                        }
                    }
                    if *show_tags.read() {
                    div { class: "tags-cloud",
                        for (tag_name, count) in tags_counts.iter() {
                            {
                                let is_selected = selected_tag.read().as_ref() == Some(tag_name);
                                let tag_clone = tag_name.clone();
                                rsx! {
                                    span {
                                        class: if is_selected { "tag-pill active" } else { "tag-pill" },
                                        onclick: move |_| {
                                            if selected_tag.read().as_ref() == Some(&tag_clone) {
                                                selected_tag.set(None);
                                            } else {
                                                selected_tag.set(Some(tag_clone.clone()));
                                                if *view_mode.read() == ViewMode::Home {
                                                    view_mode.set(ViewMode::Grid);
                                                }
                                            }
                                        },
                                        "{tag_name} ({count})"
                                    }
                                }
                            }
                        }
                    }
                    }
                }
            }

            if let Some(s) = stats {
                div {
                    class: "sidebar-stats-card",
                    style: "cursor: pointer; transition: transform 0.15s, border-color 0.15s;",
                    onclick: move |_| on_open_stats.call(()),
                    title: translate(locale, "sidebar.stats_tooltip"),
                    div { class: "stat-row",
                        span { "{translate(locale, \"sidebar.stats_total\")}" }
                        span { class: "stat-val", "{s.total_books}" }
                    }
                    div { class: "stat-row",
                        span { "{translate(locale, \"sidebar.stats_reading\")}" }
                        span { class: "stat-val", "{s.in_progress_books}" }
                    }
                    div { class: "stat-row",
                        span { "{translate(locale, \"sidebar.stats_finished\")}" }
                        span { class: "stat-val", "{s.finished_books}" }
                    }
                    div {
                        style: "margin-top: 6px; padding-top: 6px; border-top: 1px solid rgba(255,255,255,0.06); font-size: 11px; color: #818cf8; text-align: center; font-weight: 600;",
                        "{translate(locale, \"sidebar.stats_open\")}"
                    }
                }
            }
        }
    }
}
