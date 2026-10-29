use dioxus::prelude::*;
use crate::state::FilterStatus;
use vellum_core::book_service::ReadingStats;

#[component]
pub fn Sidebar(
    current_filter: Signal<FilterStatus>,
    selected_tag: Signal<Option<String>>,
    tags_counts: Vec<(String, usize)>,
    stats: Option<ReadingStats>,
) -> Element {
    let filters = [
        FilterStatus::All,
        FilterStatus::Reading,
        FilterStatus::Finished,
        FilterStatus::ToRead,
    ];

    rsx! {
        aside { class: "sidebar",
            div { class: "sidebar-section-title", "Bibliothèque" }
            nav { class: "nav-list",
                for f in filters.iter() {
                    {
                        let is_active = *current_filter.read() == *f;
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
                                },
                                span { "{f.label()}" }
                                span { class: "nav-badge", "{count_label}" }
                            }
                        }
                    }
                }
            }

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
                div { class: "sidebar-stats-card",
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
                }
            }
        }
    }
}
