use crate::cover_cache::resolve_cover_src;
use crate::library_logic::{compute_table_window, TABLE_ROW_HEIGHT_PX};
use dioxus::prelude::*;
use std::collections::HashSet;
use vellum_domain::Book;

#[component]
pub fn BooksTable(
    books: Vec<Book>,
    show_carousel: bool,
    selected_ids: HashSet<String>,
    mut focused_book_id: Signal<Option<String>>,
    on_toggle_select: EventHandler<String>,
    on_select: EventHandler<Book>,
    on_read: EventHandler<Book>,
) -> Element {
    let mut table_scroll = use_signal(|| (0.0_f64, 560.0_f64));

    use_effect(move || {
        let mut eval = document::eval(
            r#"
            const el = document.getElementById('vellum-books-table-wrap');
            if (!el) return;
            let rafId = 0;
            const emit = () => {
                rafId = 0;
                dioxus.send([el.scrollTop, el.clientHeight]);
            };
            const schedule = () => {
                if (!rafId) rafId = requestAnimationFrame(emit);
            };
            el.addEventListener('scroll', schedule, { passive: true });
            const ro = new ResizeObserver(schedule);
            ro.observe(el);
            emit();
        "#,
        );
        spawn(async move {
            while let Ok(vals) = eval.recv::<Vec<f64>>().await {
                if vals.len() == 2 {
                    table_scroll.set((vals[0], vals[1]));
                }
            }
        });
    });

    let active_idx =
        crate::library_logic::resolve_active_index(&books, focused_book_id.read().as_deref())
            .unwrap_or(0);
    let active_id = books.get(active_idx).map(|b| b.id.clone());
    let (scroll_top, viewport_h) = *table_scroll.read();
    let table_win = compute_table_window(books.len(), scroll_top, viewport_h, TABLE_ROW_HEIGHT_PX);

    rsx! {
        div { style: "display: flex; flex-direction: column; flex: 1; min-height: 0;",
            if show_carousel && !books.is_empty() {
                {
                    let total = books.len();
                    let books_prev = books.clone();
                    let books_next = books.clone();
                    let books_wheel = books.clone();
                    let active_book = books[active_idx].clone();
                    let active_read = active_book.clone();
                    let active_detail = active_book.clone();

                    rsx! {
                        div {
                            class: "coverflow-stage",
                            onwheel: move |evt: WheelEvent| {
                                let delta = evt.delta().strip_units();
                                let dy = if delta.x.abs() > delta.y.abs() { delta.x } else { delta.y };
                                if dy > 4.0 && active_idx + 1 < books_wheel.len() {
                                    focused_book_id.set(Some(books_wheel[active_idx + 1].id.clone()));
                                } else if dy < -4.0 && active_idx > 0 {
                                    focused_book_id.set(Some(books_wheel[active_idx - 1].id.clone()));
                                }
                            },

                            button {
                                class: "coverflow-nav prev",
                                title: "Précédent",
                                disabled: active_idx == 0,
                                onclick: move |_| {
                                    if active_idx > 0 {
                                        focused_book_id.set(Some(books_prev[active_idx - 1].id.clone()));
                                    }
                                },
                                "‹"
                            }

                            div { class: "coverflow-track",
                                for (idx, b) in books.iter().enumerate() {
                                    {
                                        let offset = idx as isize - active_idx as isize;
                                        if offset.abs() <= 5 {
                                            let is_center = offset == 0;
                                            let abs_off = offset.abs();
                                            let z_index = 20 - abs_off;
                                            let opacity = if is_center { 1.0 } else { (1.0 - (abs_off as f32) * 0.13).max(0.35) };
                                            let transform = if is_center {
                                                "translateX(-50%) translateZ(90px) scale(1.1) rotateY(0deg)".to_string()
                                            } else if offset < 0 {
                                                let tx = offset * 82 - 44;
                                                let tz = -(abs_off * 48);
                                                format!("translateX(calc(-50% + {tx}px)) translateZ({tz}px) rotateY(48deg) scale(0.92)")
                                            } else {
                                                let tx = offset * 82 + 44;
                                                let tz = -(abs_off * 48);
                                                format!("translateX(calc(-50% + {tx}px)) translateZ({tz}px) rotateY(-48deg) scale(0.92)")
                                            };
                                            let cover_src = resolve_cover_src(&b.id, b.cover_path.as_deref(), true);
                                            let id_click = b.id.clone();
                                            let b_click = b.clone();
                                            let b_dbl = b.clone();
                                            rsx! {
                                                div {
                                                    key: "cf-{b.id}",
                                                    class: if is_center { "coverflow-card is-center" } else { "coverflow-card" },
                                                    style: "transform: {transform}; z-index: {z_index}; opacity: {opacity};",
                                                    title: "{b.title} — {b.author}",
                                                    onclick: move |_| {
                                                        if is_center {
                                                            on_select.call(b_click.clone());
                                                        } else {
                                                            focused_book_id.set(Some(id_click.clone()));
                                                        }
                                                    },
                                                    ondoubleclick: move |_| on_read.call(b_dbl.clone()),
                                                    if let Some(src) = cover_src {
                                                        img {
                                                            src: "{src}",
                                                            style: "width: 100%; height: 100%; object-fit: cover; pointer-events: none;",
                                                        }
                                                    } else {
                                                        div { style: "padding: 10px; text-align: center; font-size: 11px; color: var(--text-secondary);",
                                                            div { style: "font-size: 26px; margin-bottom: 6px;", "📖" }
                                                            div { style: "font-weight: 600;", "{b.title}" }
                                                        }
                                                    }
                                                }
                                            }
                                        } else {
                                            rsx! {}
                                        }
                                    }
                                }
                            }

                            button {
                                class: "coverflow-nav next",
                                title: "Suivant",
                                disabled: active_idx + 1 >= total,
                                onclick: move |_| {
                                    if active_idx + 1 < books_next.len() {
                                        focused_book_id.set(Some(books_next[active_idx + 1].id.clone()));
                                    }
                                },
                                "›"
                            }

                            div { class: "coverflow-info",
                                span { style: "font-weight: 700;", "{active_book.title}" }
                                span { style: "color: var(--text-muted);", "— {active_book.author}" }
                                button {
                                    class: "btn btn-primary",
                                    style: "padding: 3px 10px; font-size: 11px; border-radius: 6px;",
                                    onclick: move |_| on_read.call(active_read.clone()),
                                    "📖 Lire"
                                }
                                button {
                                    class: "btn btn-secondary",
                                    style: "padding: 3px 10px; font-size: 11px; border-radius: 6px;",
                                    onclick: move |_| on_select.call(active_detail.clone()),
                                    "ℹ️ Détails"
                                }
                            }
                        }
                    }
                }
            }

            div {
                id: "vellum-books-table-wrap",
                class: "books-table-wrap",
                table { class: "books-table",
                    thead {
                        tr {
                            th { style: "width: 34px;", "" }
                            th { style: "width: 44px;", "" }
                            th { "Titre" }
                            th { "Auteur" }
                            th { "Série" }
                            th { "Formats" }
                            th { "Tags" }
                            th { style: "width: 90px;", "Progrès" }
                            th { style: "width: 92px; text-align: right;", "Actions" }
                        }
                    }
                    tbody {
                        if table_win.top_spacer_px > 0.0 {
                            tr {
                                key: "table-top-spacer",
                                style: "height: {table_win.top_spacer_px:.1}px; pointer-events: none;",
                                td { colspan: "9", style: "padding: 0; border: none;" }
                            }
                        }
                        for b in books[table_win.start_idx..table_win.end_idx].iter() {
                            {
                                let is_active = active_id.as_deref() == Some(b.id.as_str());
                                let is_checked = selected_ids.contains(&b.id);
                                let cover_src = resolve_cover_src(&b.id, b.cover_path.as_deref(), true);
                                let series_label = match (&b.series, b.series_index) {
                                    (Some(s), Some(idx)) if !s.is_empty() => format!("{s} #{idx}"),
                                    (Some(s), None) if !s.is_empty() => s.clone(),
                                    _ => "—".to_string(),
                                };
                                let fmts: Vec<String> = b.formats.iter().map(|f| f.format.to_uppercase()).collect();
                                let fmts_label = if fmts.is_empty() { "—".to_string() } else { fmts.join(", ") };
                                let tags_label = if b.tags.is_empty() {
                                    "—".to_string()
                                } else {
                                    b.tags.iter().map(|t| t.name.as_str()).collect::<Vec<_>>().join(", ")
                                };
                                let id_focus = b.id.clone();
                                let id_toggle = b.id.clone();
                                let b_row = b.clone();
                                let b_row_select = b.clone();
                                let b_read = b.clone();
                                let b_detail = b.clone();
                                rsx! {
                                    tr {
                                        key: "row-{b.id}",
                                        class: if is_active { "book-row active-row" } else { "book-row" },
                                        onclick: move |_| {
                                            focused_book_id.set(Some(id_focus.clone()));
                                            on_select.call(b_row_select.clone());
                                        },
                                        ondoubleclick: move |_| on_read.call(b_row.clone()),
                                        td {
                                            onclick: move |e| e.stop_propagation(),
                                            input {
                                                r#type: "checkbox",
                                                checked: is_checked,
                                                onchange: move |_| on_toggle_select.call(id_toggle.clone()),
                                            }
                                        }
                                        td {
                                            div { style: "width: 28px; height: 40px; border-radius: 4px; overflow: hidden; background: #181b2c; display: flex; align-items: center; justify-content: center;",
                                                if let Some(src) = cover_src {
                                                    img {
                                                        src: "{src}",
                                                        style: "width: 100%; height: 100%; object-fit: cover;",
                                                    }
                                                } else {
                                                    span { style: "font-size: 13px;", "📘" }
                                                }
                                            }
                                        }
                                        td { style: "font-weight: 600;", "{b.title}" }
                                        td { style: "color: var(--text-secondary);", "{b.author}" }
                                        td { style: "color: var(--text-muted);", "{series_label}" }
                                        td {
                                            span { class: "format-pill", "{fmts_label}" }
                                        }
                                        td { style: "color: var(--text-muted); max-width: 180px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;", "{tags_label}" }
                                        td {
                                            div { style: "display: flex; align-items: center; gap: 6px;",
                                                div { style: "flex: 1; height: 5px; background: rgba(255,255,255,0.08); border-radius: 99px; overflow: hidden;",
                                                    div { style: "width: {b.progress_percent}%; height: 100%; background: var(--accent-primary);" }
                                                }
                                                span { style: "font-size: 11px; color: var(--text-muted);", "{b.progress_percent}%" }
                                            }
                                        }
                                        td {
                                            style: "text-align: right;",
                                            onclick: move |e| e.stop_propagation(),
                                            div { style: "display: inline-flex; gap: 6px;",
                                                button {
                                                    class: "btn btn-secondary",
                                                    style: "padding: 4px 8px; font-size: 11px;",
                                                    title: "Lire",
                                                    onclick: move |_| on_read.call(b_read.clone()),
                                                    "📖"
                                                }
                                                button {
                                                    class: "btn btn-secondary",
                                                    style: "padding: 4px 8px; font-size: 11px;",
                                                    title: "Détails",
                                                    onclick: move |_| on_select.call(b_detail.clone()),
                                                    "ℹ️"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if table_win.bottom_spacer_px > 0.0 {
                            tr {
                                key: "table-bottom-spacer",
                                style: "height: {table_win.bottom_spacer_px:.1}px; pointer-events: none;",
                                td { colspan: "9", style: "padding: 0; border: none;" }
                            }
                        }
                    }
                }
            }
        }
    }
}
