use std::collections::HashSet;
use base64::Engine;
use dioxus::prelude::*;
use vellum_domain::Book;

fn resolve_cover_src(cover_path: Option<&str>) -> Option<String> {
    let p = cover_path?;
    if p.starts_with("data:") || p.starts_with("http") {
        return Some(p.to_string());
    }
    let full_path = if std::path::Path::new(p).is_absolute() {
        std::path::PathBuf::from(p)
    } else {
        dirs::home_dir().map(|h| h.join(".vellum").join("library").join(p))?
    };
    if !full_path.exists() {
        return None;
    }
    let bytes = std::fs::read(&full_path).ok()?;
    let ext = full_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("jpeg")
        .to_lowercase();
    let mime = match ext.as_str() {
        "png" => "image/png",
        "webp" => "image/webp",
        _ => "image/jpeg",
    };
    Some(format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(&bytes)
    ))
}

#[component]
pub fn BooksTable(
    books: Vec<Book>,
    show_carousel: bool,
    selected_ids: HashSet<String>,
    on_toggle_select: EventHandler<String>,
    on_select: EventHandler<Book>,
    on_read: EventHandler<Book>,
) -> Element {
    let mut focused_book_id = use_signal(|| books.first().map(|b| b.id.clone()));

    let active_id = focused_book_id
        .read()
        .clone()
        .filter(|id| books.iter().any(|b| &b.id == id))
        .or_else(|| books.first().map(|b| b.id.clone()));

    rsx! {
        div { style: "display: flex; flex-direction: column; flex: 1; min-height: 0;",
            if show_carousel {
                div { class: "cover-carousel",
                    for b in books.iter() {
                        {
                            let is_active = active_id.as_deref() == Some(b.id.as_str());
                            let cover_src = resolve_cover_src(b.cover_path.as_deref());
                            let b_click = b.clone();
                            let b_dbl = b.clone();
                            let id_click = b.id.clone();
                            rsx! {
                                div {
                                    key: "car-{b.id}",
                                    class: if is_active { "carousel-item active" } else { "carousel-item" },
                                    title: "{b.title} — {b.author}",
                                    onclick: move |_| {
                                        focused_book_id.set(Some(id_click.clone()));
                                        on_select.call(b_click.clone());
                                    },
                                    ondoubleclick: move |_| on_read.call(b_dbl.clone()),
                                    div { class: "carousel-cover",
                                        if let Some(src) = cover_src {
                                            img {
                                                src: "{src}",
                                                style: "width: 100%; height: 100%; object-fit: cover;",
                                            }
                                        } else {
                                            span { style: "font-size: 22px; color: var(--text-muted);", "📖" }
                                        }
                                    }
                                    div { class: "carousel-caption", "{b.title}" }
                                }
                            }
                        }
                    }
                }
            }

            div { class: "books-table-wrap",
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
                        for b in books.iter() {
                            {
                                let is_active = active_id.as_deref() == Some(b.id.as_str());
                                let is_checked = selected_ids.contains(&b.id);
                                let cover_src = resolve_cover_src(b.cover_path.as_deref());
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
                                let b_read = b.clone();
                                let b_detail = b.clone();
                                rsx! {
                                    tr {
                                        key: "row-{b.id}",
                                        class: if is_active { "active-row" } else { "" },
                                        onclick: move |_| {
                                            focused_book_id.set(Some(id_focus.clone()));
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
                    }
                }
            }
        }
    }
}
