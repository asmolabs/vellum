use chrono::{Datelike, Duration, Utc};
use dioxus::prelude::*;
use vellum_domain::{Book, ReadingStats};

#[component]
pub fn ReadingStatsModal(
    stats: ReadingStats,
    on_close: EventHandler<()>,
    on_read_book: EventHandler<Book>,
) -> Element {
    // Generate dates for the last 52 weeks (364 days) up to today
    let today = Utc::now().date_naive();
    // 52 weeks * 7 days = 364 days
    let start_date = today - Duration::days(363);

    // Group dates by week: Vec of 52 weeks, each containing 7 days
    let mut weeks: Vec<Vec<(String, usize, u32)>> = Vec::with_capacity(52);
    let mut current_week = Vec::with_capacity(7);

    let mut cur = start_date;
    while cur <= today {
        let date_str = cur.format("%Y-%m-%d").to_string();
        let count = stats.activity_by_day.get(&date_str).copied().unwrap_or(0);
        let weekday = cur.weekday().number_from_monday(); // 1 = Mon .. 7 = Sun
        current_week.push((date_str, count, weekday));

        if weekday == 7 || cur == today {
            weeks.push(current_week);
            current_week = Vec::with_capacity(7);
        }
        cur += Duration::days(1);
    }
    if !current_week.is_empty() {
        weeks.push(current_week);
    }

    let completion_pct = if stats.total_books > 0 {
        ((stats.finished_books as f64 / stats.total_books as f64) * 100.0).round() as u32
    } else {
        0
    };

    rsx! {
        div {
            class: "drawer-backdrop",
            onclick: move |_| on_close.call(()),

            div {
                class: "drawer-panel",
                style: "width: 860px; max-width: 95vw; max-height: 92vh; display: flex; flex-direction: column;",
                onclick: move |evt| evt.stop_propagation(),

                // Header
                div { class: "drawer-header", style: "border-bottom: 1px solid var(--border-color); padding-bottom: 12px; margin-bottom: 14px;",
                    div { style: "display: flex; align-items: center; gap: 10px;",
                        span { style: "font-size: 26px;", "📊" }
                        div {
                            h3 { style: "font-size: 17px; font-weight: 700; margin: 0;", "Statistiques & Habitudes de Lecture" }
                            p { style: "font-size: 12px; color: var(--text-secondary); margin: 0;", "Quantified Reading : suivez votre rythme de lecture et vos genres favoris" }
                        }
                    }
                    button {
                        class: "btn btn-icon",
                        onclick: move |_| on_close.call(()),
                        "✕"
                    }
                }

                // Scrollable Content
                div { style: "flex: 1; overflow-y: auto; display: flex; flex-direction: column; gap: 16px; padding-right: 4px;",
                    // Key Metrics Grid
                    div { style: "display: grid; grid-template-columns: repeat(5, 1fr); gap: 10px;",
                        div { style: "background: var(--bg-card); border: 1px solid var(--border-color); border-radius: 8px; padding: 12px; text-align: center;",
                            div { style: "font-size: 11px; font-weight: 700; color: var(--text-secondary); text-transform: uppercase;", "Livres total" }
                            div { style: "font-size: 24px; font-weight: 800; color: var(--text-primary); margin-top: 4px;", "{stats.total_books}" }
                        }
                        div { style: "background: var(--bg-card); border: 1px solid var(--border-color); border-radius: 8px; padding: 12px; text-align: center;",
                            div { style: "font-size: 11px; font-weight: 700; color: #38bdf8; text-transform: uppercase;", "Temps estimé" }
                            div { style: "font-size: 24px; font-weight: 800; color: #38bdf8; margin-top: 4px;", "{stats.total_reading_hours} h" }
                        }
                        div { style: "background: var(--bg-card); border: 1px solid var(--border-color); border-radius: 8px; padding: 12px; text-align: center;",
                            div { style: "font-size: 11px; font-weight: 700; color: #f59e0b; text-transform: uppercase;", "En cours" }
                            div { style: "font-size: 24px; font-weight: 800; color: #f59e0b; margin-top: 4px;", "{stats.in_progress_books}" }
                        }
                        div { style: "background: var(--bg-card); border: 1px solid var(--border-color); border-radius: 8px; padding: 12px; text-align: center;",
                            div { style: "font-size: 11px; font-weight: 700; color: #34d399; text-transform: uppercase;", "Terminés" }
                            div { style: "font-size: 24px; font-weight: 800; color: #34d399; margin-top: 4px;", "{stats.finished_books} ({completion_pct}%)" }
                        }
                        div { style: "background: var(--bg-card); border: 1px solid var(--border-color); border-radius: 8px; padding: 12px; text-align: center;",
                            div { style: "font-size: 11px; font-weight: 700; color: #c084fc; text-transform: uppercase;", "Coups de cœur" }
                            div { style: "font-size: 24px; font-weight: 800; color: #c084fc; margin-top: 4px;", "{stats.favorite_books}" }
                        }
                    }

                    // Annual Activity Heatmap (GitHub Style)
                    div { style: "background: var(--bg-card); border: 1px solid var(--border-color); border-radius: 10px; padding: 16px; display: flex; flex-direction: column; gap: 10px;",
                        div { style: "display: flex; justify-content: space-between; align-items: center;",
                            div { style: "font-size: 13px; font-weight: 700; color: var(--text-primary);", "📅 Activité de lecture sur l'année (52 semaines)" }
                            div { style: "font-size: 11px; color: var(--text-secondary);", "Chaque case représente un jour d'activité" }
                        }

                        // Grid of weeks
                        div {
                            style: "display: flex; gap: 3px; overflow-x: auto; padding-bottom: 6px; align-items: flex-start;",
                            for week in weeks.iter() {
                                div {
                                    style: "display: flex; flex-direction: column; gap: 3px;",
                                    for (day_str, count, _) in week.iter() {
                                        {
                                            let (bg_color, border_color) = if *count == 0 {
                                                ("rgba(255, 255, 255, 0.04)", "transparent")
                                            } else if *count == 1 {
                                                ("rgba(34, 197, 94, 0.35)", "rgba(34, 197, 94, 0.5)")
                                            } else if *count == 2 {
                                                ("rgba(34, 197, 94, 0.65)", "rgba(34, 197, 94, 0.8)")
                                            } else {
                                                ("#22c55e", "#4ade80")
                                            };
                                            let tip = if *count == 0 {
                                                format!("{day_str} : Aucune activité")
                                            } else {
                                                format!("{day_str} : {count} livre(s) lu(s) ou progressé(s)")
                                            };

                                            rsx! {
                                                div {
                                                    style: "width: 11px; height: 11px; border-radius: 2px; background: {bg_color}; border: 1px solid {border_color}; cursor: pointer;",
                                                    title: "{tip}",
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Heatmap Legend
                        div { style: "display: flex; justify-content: flex-end; align-items: center; gap: 6px; font-size: 11px; color: var(--text-secondary);",
                            span { "Moins" }
                            div { style: "width: 10px; height: 10px; border-radius: 2px; background: rgba(255, 255, 255, 0.04);" }
                            div { style: "width: 10px; height: 10px; border-radius: 2px; background: rgba(34, 197, 94, 0.35);" }
                            div { style: "width: 10px; height: 10px; border-radius: 2px; background: rgba(34, 197, 94, 0.65);" }
                            div { style: "width: 10px; height: 10px; border-radius: 2px; background: #22c55e;" }
                            span { "Plus" }
                        }
                    }

                    // Lower Section: Top Genres / Tags + Recently Read
                    div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 14px;",
                        // Top Genres / Tags
                        div { style: "background: var(--bg-card); border: 1px solid var(--border-color); border-radius: 10px; padding: 14px; display: flex; flex-direction: column; gap: 10px;",
                            div { style: "font-size: 13px; font-weight: 700; color: var(--text-primary);", "🏷️ Répartition par Genres & Étiquettes" }
                            if stats.tag_distribution.is_empty() {
                                div { style: "font-size: 12px; color: var(--text-secondary); font-style: italic; padding: 20px 0; text-align: center;", "Aucune étiquette renseignée" }
                            } else {
                                {
                                    let total_tagged: usize = stats.tag_distribution.values().sum();
                                    let mut sorted_tags: Vec<(&String, &usize)> = stats.tag_distribution.iter().collect();
                                    sorted_tags.sort_by(|a, b| b.1.cmp(a.1));
                                    rsx! {
                                        div { style: "display: flex; flex-direction: column; gap: 8px;",
                                            for (tag, count) in sorted_tags.iter().take(6) {
                                                {
                                                    let pct = if total_tagged > 0 {
                                                        ((**count as f64 / total_tagged as f64) * 100.0).round() as u32
                                                    } else {
                                                        0
                                                    };
                                                    rsx! {
                                                        div { style: "display: flex; flex-direction: column; gap: 3px;",
                                                            div { style: "display: flex; justify-content: space-between; font-size: 12px;",
                                                                span { style: "font-weight: 600; color: var(--text-primary);", "{tag}" }
                                                                span { style: "color: var(--text-secondary);", "{count} livre(s) ({pct}%)" }
                                                            }
                                                            div { style: "width: 100%; height: 6px; background: rgba(255, 255, 255, 0.08); border-radius: 3px; overflow: hidden;",
                                                                div { style: "height: 100%; width: {pct}%; background: #6366f1; border-radius: 3px;" }
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

                        // Recently Read Books
                        div { style: "background: var(--bg-card); border: 1px solid var(--border-color); border-radius: 10px; padding: 14px; display: flex; flex-direction: column; gap: 10px;",
                            div { style: "font-size: 13px; font-weight: 700; color: var(--text-primary);", "📖 Dernières Lectures Actives" }
                            if stats.recently_read.is_empty() {
                                div { style: "font-size: 12px; color: var(--text-secondary); font-style: italic; padding: 20px 0; text-align: center;", "Aucune session de lecture récente" }
                            } else {
                                div { style: "display: flex; flex-direction: column; gap: 8px;",
                                    for b in stats.recently_read.iter() {
                                        {
                                            let book_c = b.clone();
                                            let on_r = on_read_book;
                                            let prog = b.progress_percent as u32;
                                            rsx! {
                                                div {
                                                    style: "display: flex; justify-content: space-between; align-items: center; background: var(--bg-primary); padding: 8px 10px; border-radius: 6px; border: 1px solid var(--border-color);",
                                                    div { style: "flex: 1; min-width: 0; padding-right: 8px;",
                                                        div { style: "font-size: 12px; font-weight: 700; color: var(--text-primary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;", "{b.title}" }
                                                        div { style: "font-size: 11px; color: var(--text-secondary);", "{b.author} • {prog}% lu" }
                                                    }
                                                    button {
                                                        class: "btn btn-secondary",
                                                        style: "font-size: 11px; padding: 3px 8px; white-space: nowrap;",
                                                        onclick: move |_| on_r.call(book_c.clone()),
                                                        "Continuer"
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
}
