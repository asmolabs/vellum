use dioxus::prelude::*;
use crate::state::FilterStatus;
use vellum_core::{translate, Locale};
use vellum_domain::{ReadingStats, VirtualLibrary};

#[component]
pub fn Sidebar(
    locale: Locale,
    current_filter: Signal<FilterStatus>,
    selected_tag: Signal<Option<String>>,
    selected_series: Signal<Option<String>>,
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
    let filters = [
        FilterStatus::All,
        FilterStatus::Reading,
        FilterStatus::Finished,
        FilterStatus::ToRead,
    ];

    rsx! {
        aside { class: "sidebar",
            div { class: "sidebar-section-title", "{translate(locale, \"nav.library\")}" }
            nav { class: "nav-list",
                for f in filters.iter() {
                    {
                        let is_active = *current_filter.read() == *f && selected_series.read().is_none() && selected_tag.read().is_none();
                        let filter_label = match f {
                            FilterStatus::All => translate(locale, "nav.all_books"),
                            FilterStatus::Reading => translate(locale, "nav.reading"),
                            FilterStatus::Finished => translate(locale, "nav.finished"),
                            FilterStatus::ToRead => translate(locale, "nav.to_read"),
                        };
                        let count_label = match (f, &stats) {
                            (FilterStatus::All, Some(s)) => format!("{}", s.total_books),
                            (FilterStatus::Reading, Some(s)) => format!("{}", s.in_progress_books),
                            (FilterStatus::Finished, Some(s)) => format!("{}", s.finished_books),
                            (FilterStatus::ToRead, Some(s)) => format!("{}", s.total_books.saturating_sub(s.in_progress_books + s.finished_books)),
                            _ => "-".to_string(),
                        };
                        let f_val = *f;

                        rsx! {
                            div {
                                class: if is_active { "nav-item active" } else { "nav-item" },
                                onclick: move |_| {
                                    current_filter.set(f_val);
                                    selected_tag.set(None);
                                    selected_series.set(None);
                                },
                                span { "{filter_label}" }
                                span { class: "nav-badge", "{count_label}" }
                            }
                        }
                    }
                }
            }

            // Bibliothèques virtuelles
            div {
                div {
                    class: "sidebar-section-title",
                    style: "display: flex; justify-content: space-between; align-items: center;",
                    span { "Bibliothèques virtuelles" }
                    button {
                        style: "background: none; border: none; color: #818cf8; cursor: pointer; font-size: 14px; font-weight: 700; padding: 0 4px;",
                        title: "Créer une bibliothèque virtuelle",
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
                            placeholder: "Nom (ex: SF à lire)",
                            style: "font-size: 11px; padding: 4px 6px; border-radius: 4px; background: rgba(0,0,0,0.3); color: #e2e8f0; border: 1px solid rgba(255,255,255,0.1);",
                            value: "{new_vl_name}",
                            oninput: move |e| new_vl_name.set(e.value()),
                        }
                        input {
                            r#type: "text",
                            placeholder: "Requête (ex: tag:sf -status:finished)",
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
                            "Enregistrer"
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

            // Séries & Sagas
            if !series_counts.is_empty() {
                div {
                    div { class: "sidebar-section-title", "{translate(locale, \"nav.series\")}" }
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

            // Étiquettes / Tags
            if !tags_counts.is_empty() {
                div {
                    div { class: "sidebar-section-title", "Étiquettes" }
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

            if let Some(s) = stats {
                div {
                    class: "sidebar-stats-card",
                    style: "cursor: pointer; transition: transform 0.15s, border-color 0.15s;",
                    onclick: move |_| on_open_stats.call(()),
                    title: "Cliquer pour ouvrir les statistiques détaillées et le calendrier d'activité",
                    div { class: "stat-row",
                        span { "Total :" }
                        span { class: "stat-val", "{s.total_books}" }
                    }
                    div { class: "stat-row",
                        span { "En lecture :" }
                        span { class: "stat-val", "{s.in_progress_books}" }
                    }
                    div { class: "stat-row",
                        span { "Terminés :" }
                        span { class: "stat-val", "{s.finished_books}" }
                    }
                    div {
                        style: "margin-top: 6px; padding-top: 6px; border-top: 1px solid rgba(255,255,255,0.06); font-size: 11px; color: #818cf8; text-align: center; font-weight: 600;",
                        "📊 Voir les statistiques ➔"
                    }
                }
            }
        }
    }
}
