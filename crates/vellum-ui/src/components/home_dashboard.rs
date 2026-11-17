//! Tableau de bord d'accueil inspiré de BookOrbit (widgets stats, reprendre la lecture, sagas et ajouts récents).

use std::collections::BTreeMap;

use dioxus::prelude::*;
use vellum_core::{translate, Locale};
use vellum_domain::{Book, ReadingStats};

use crate::cover_cache::resolve_cover_src;
use crate::state::{FilterStatus, ViewMode};

#[component]
pub fn HomeDashboard(
    locale: Locale,
    books: Vec<Book>,
    stats: Option<ReadingStats>,
    mut current_filter: Signal<FilterStatus>,
    mut selected_series: Signal<Option<String>>,
    mut view_mode: Signal<ViewMode>,
    on_select: EventHandler<Book>,
    on_read: EventHandler<Book>,
) -> Element {
    let total_books = stats.as_ref().map(|s| s.total_books).unwrap_or(books.len());
    let in_progress = stats
        .as_ref()
        .map(|s| s.in_progress_books)
        .unwrap_or_else(|| {
            books
                .iter()
                .filter(|b| !b.is_finished && b.progress_percent > 0.0)
                .count()
        });
    let finished = stats
        .as_ref()
        .map(|s| s.finished_books)
        .unwrap_or_else(|| books.iter().filter(|b| b.is_finished).count());

    let mut series_map: BTreeMap<String, Vec<Book>> = BTreeMap::new();
    for b in &books {
        if let Some(ref s) = b.series {
            let trimmed = s.trim();
            if !trimmed.is_empty() {
                series_map
                    .entry(trimmed.to_string())
                    .or_default()
                    .push(b.clone());
            }
        }
    }
    let series_count = series_map.len();

    let continue_reading: Vec<Book> = {
        let mut list: Vec<Book> = books
            .iter()
            .filter(|b| !b.is_finished && b.progress_percent > 0.0)
            .cloned()
            .collect();
        list.sort_by(|a, b| {
            b.progress_percent
                .partial_cmp(&a.progress_percent)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        list.truncate(8);
        list
    };

    let recent_books: Vec<Book> = {
        let mut list = books.clone();
        list.sort_by(|a, b| b.added_at.cmp(&a.added_at));
        list.truncate(8);
        list
    };

    rsx! {
        div {
            style: "display: flex; flex-direction: column; gap: 22px; padding: 8px 24px 32px; overflow-y: auto;",

            // 1. Widgets KPI (cliquables pour filtrer directement)
            div {
                style: "display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 12px;",
                div {
                    style: "padding: 14px 16px; border-radius: 12px; background: rgba(99, 102, 241, 0.08); border: 1px solid rgba(99, 102, 241, 0.25); cursor: pointer;",
                    onclick: move |_| {
                        current_filter.set(FilterStatus::All);
                        selected_series.set(None);
                        view_mode.set(ViewMode::Grid);
                    },
                    div { style: "font-size: 11px; color: var(--text-muted); font-weight: 600;", "{translate(locale, \"home.kpi_total\")}" }
                    div { style: "font-size: 24px; font-weight: 800; color: #a5b4fc; margin-top: 4px;", "📚 {total_books}" }
                }
                div {
                    style: "padding: 14px 16px; border-radius: 12px; background: rgba(59, 130, 246, 0.08); border: 1px solid rgba(59, 130, 246, 0.25); cursor: pointer;",
                    onclick: move |_| {
                        current_filter.set(FilterStatus::Reading);
                        selected_series.set(None);
                        view_mode.set(ViewMode::Grid);
                    },
                    div { style: "font-size: 11px; color: var(--text-muted); font-weight: 600;", "{translate(locale, \"home.kpi_reading\")}" }
                    div { style: "font-size: 24px; font-weight: 800; color: #93c5fd; margin-top: 4px;", "📖 {in_progress}" }
                }
                div {
                    style: "padding: 14px 16px; border-radius: 12px; background: rgba(16, 185, 129, 0.08); border: 1px solid rgba(16, 185, 129, 0.25); cursor: pointer;",
                    onclick: move |_| {
                        current_filter.set(FilterStatus::Finished);
                        selected_series.set(None);
                        view_mode.set(ViewMode::Grid);
                    },
                    div { style: "font-size: 11px; color: var(--text-muted); font-weight: 600;", "{translate(locale, \"home.kpi_finished\")}" }
                    div { style: "font-size: 24px; font-weight: 800; color: #34d399; margin-top: 4px;", "✅ {finished}" }
                }
                div {
                    style: "padding: 14px 16px; border-radius: 12px; background: rgba(168, 85, 247, 0.08); border: 1px solid rgba(168, 85, 247, 0.25);",
                    div { style: "font-size: 11px; color: var(--text-muted); font-weight: 600;", "{translate(locale, \"home.kpi_series\")}" }
                    div { style: "font-size: 24px; font-weight: 800; color: #d8b4fe; margin-top: 4px;", "🗂️ {series_count}" }
                }
            }

            // 2. Reprendre la lecture (étagère horizontale avec couvertures et barres de progression)
            if !continue_reading.is_empty() {
                section { style: "display: flex; flex-direction: column; gap: 10px;",
                    h3 { style: "font-size: 14px; font-weight: 700; margin: 0; color: #c7d2fe;", "{translate(locale, \"library.continue_reading\")}" }
                    div { style: "display: flex; gap: 14px; overflow-x: auto; padding-bottom: 6px;",
                        for b in continue_reading {
                            {
                                let b_read = b.clone();
                                let b_sel = b.clone();
                                let cover_src = resolve_cover_src(&b.id, b.cover_path.as_deref(), false);
                                let pct = b.progress_percent.round() as i32;
                                rsx! {
                                    div {
                                        key: "home-cont-{b.id}",
                                        style: "min-width: 240px; max-width: 240px; display: flex; gap: 12px; padding: 10px; border-radius: 12px; background: rgba(255,255,255,0.03); border: 1px solid var(--border-subtle); align-items: center;",
                                        div {
                                            style: "width: 54px; height: 80px; border-radius: 6px; overflow: hidden; background: #1e293b; flex-shrink: 0; cursor: pointer; display: flex; align-items: center; justify-content: center;",
                                            onclick: move |_| on_select.call(b_sel.clone()),
                                            if let Some(src) = cover_src {
                                                img { src: "{src}", style: "width: 100%; height: 100%; object-fit: cover;" }
                                            } else {
                                                span { style: "font-size: 22px;", "📖" }
                                            }
                                        }
                                        div { style: "flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 6px;",
                                            div { style: "font-size: 13px; font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;", "{b.title}" }
                                            div { style: "font-size: 11px; color: var(--text-muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis;", "{b.author}" }
                                            div { style: "height: 4px; background: rgba(255,255,255,0.1); border-radius: 4px; overflow: hidden;",
                                                div { style: "width: {pct}%; height: 100%; background: #6366f1;" }
                                            }
                                            div { style: "display: flex; justify-content: space-between; align-items: center;",
                                                span { style: "font-size: 11px; color: #a5b4fc;", "{pct}%" }
                                                button {
                                                    class: "btn btn-primary",
                                                    style: "font-size: 11px; padding: 3px 10px;",
                                                    onclick: move |_| on_read.call(b_read.clone()),
                                                    "{translate(locale, \"home.resume_btn\")}"
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

            // 3. Étagères par Sagas / Séries
            if !series_map.is_empty() {
                section { style: "display: flex; flex-direction: column; gap: 10px;",
                    h3 { style: "font-size: 14px; font-weight: 700; margin: 0;", "{translate(locale, \"home.shelves_series\")}" }
                    div { style: "display: flex; gap: 10px; overflow-x: auto; padding-bottom: 4px;",
                        for (series_name, series_books) in series_map.into_iter().take(12) {
                            {
                                let s_name = series_name.clone();
                                let count = series_books.len();
                                rsx! {
                                    button {
                                        key: "home-ser-{series_name}",
                                        class: "btn btn-secondary",
                                        style: "padding: 10px 14px; border-radius: 10px; display: flex; flex-direction: column; align-items: flex-start; gap: 4px; min-width: 150px; flex-shrink: 0;",
                                        onclick: move |_| {
                                            selected_series.set(Some(s_name.clone()));
                                            view_mode.set(ViewMode::Grid);
                                        },
                                        span { style: "font-size: 12px; font-weight: 700; color: #c4b5fd;", "📚 {series_name}" }
                                        span { style: "font-size: 11px; color: var(--text-muted);", "{count} vol." }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // 4. Ajouts récents
            if !recent_books.is_empty() {
                section { style: "display: flex; flex-direction: column; gap: 10px;",
                    h3 { style: "font-size: 14px; font-weight: 700; margin: 0;", "{translate(locale, \"home.recent_adds\")}" }
                    div { style: "display: flex; gap: 14px; overflow-x: auto; padding-bottom: 6px;",
                        for b in recent_books {
                            {
                                let b_sel = b.clone();
                                let cover_src = resolve_cover_src(&b.id, b.cover_path.as_deref(), false);
                                rsx! {
                                    div {
                                        key: "home-rec-{b.id}",
                                        style: "min-width: 126px; max-width: 126px; display: flex; flex-direction: column; gap: 6px; cursor: pointer;",
                                        onclick: move |_| on_select.call(b_sel.clone()),
                                        div {
                                            style: "width: 126px; height: 184px; border-radius: 10px; overflow: hidden; background: #1e293b; border: 1px solid var(--border-subtle); display: flex; align-items: center; justify-content: center;",
                                            if let Some(src) = cover_src {
                                                img { src: "{src}", style: "width: 100%; height: 100%; object-fit: cover;" }
                                            } else {
                                                span { style: "font-size: 32px;", "📖" }
                                            }
                                        }
                                        div { style: "font-size: 12px; font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;", "{b.title}" }
                                        div { style: "font-size: 11px; color: var(--text-muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis;", "{b.author}" }
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
