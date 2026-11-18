mod edit_form;

use dioxus::prelude::*;
use std::collections::BTreeMap;
use std::path::PathBuf;
use vellum_core::{translate, translate_fmt, Locale};
use vellum_domain::{Book, CustomColumnDefinition};

use self::edit_form::MetadataEditForm;

/// Données soumises lors de l'enregistrement des métadonnées d'un livre.
#[derive(Debug, Clone, PartialEq)]
pub struct SaveMetadataRequest {
    pub book_id: String,
    pub title: String,
    pub author: String,
    pub publisher: Option<String>,
    pub description: Option<String>,
    pub isbn: Option<String>,
    pub publish_date: Option<String>,
    pub series: Option<String>,
    pub series_index: Option<f64>,
    pub tags: Vec<String>,
}

#[component]
pub fn DetailDrawer(
    locale: Locale,
    book: Book,
    convertible_formats: Vec<String>,
    connected_devices: Vec<vellum_core::ConnectedDevice>,
    on_close: EventHandler<()>,
    on_read: EventHandler<Book>,
    on_delete: EventHandler<String>,
    on_convert: EventHandler<(String, String)>,
    on_change_cover: EventHandler<(String, PathBuf)>,
    on_save_metadata: EventHandler<SaveMetadataRequest>,
    on_enrich_metadata: EventHandler<String>,
    on_open_translate: EventHandler<Book>,
    on_export_to_device: EventHandler<(String, vellum_domain::DeviceKind)>,
    can_send_kindle: bool,
    is_sending_kindle: bool,
    on_send_to_kindle: EventHandler<Book>,
    on_export_file: EventHandler<(String, Option<String>)>,
    on_export_notes_md: EventHandler<String>,
    on_open_epub_editor: EventHandler<Book>,
    on_polish_epub: EventHandler<String>,
    custom_columns: Vec<CustomColumnDefinition>,
    custom_values: BTreeMap<String, String>,
    on_save_custom_value: EventHandler<(String, String, String)>,
    #[props(default)] on_change_status: Option<
        EventHandler<(String, crate::components::KanbanTargetStatus)>,
    >,
    #[props(default)] on_attach_format: Option<EventHandler<String>>,
) -> Element {
    let mut is_editing = use_signal(|| false);
    let mut show_more_actions = use_signal(|| false);

    let mut edit_title = use_signal(|| book.title.clone());
    let mut edit_author = use_signal(|| book.author.clone());
    let edit_series = use_signal(|| book.series.clone().unwrap_or_default());
    let edit_series_index =
        use_signal(|| book.series_index.map(|i| i.to_string()).unwrap_or_default());
    let mut edit_publisher = use_signal(|| book.publisher.clone().unwrap_or_default());
    let edit_publish_date = use_signal(|| book.publish_date.clone().unwrap_or_default());
    let edit_isbn = use_signal(|| book.isbn.clone().unwrap_or_default());
    let mut edit_description = use_signal(|| book.description.clone().unwrap_or_default());
    let mut edit_tags = use_signal(|| {
        book.tags
            .iter()
            .map(|t| t.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    });

    let title = book.title.clone();
    let author = book.author.clone();
    let publisher = book
        .publisher
        .clone()
        .unwrap_or_else(|| translate(locale, "drawer.unknown_publisher").to_string());
    let description = book
        .description
        .clone()
        .unwrap_or_else(|| translate(locale, "drawer.no_description").to_string());
    let formats = book.formats.clone();
    let tags = book.tags.clone();
    let book_id = book.id.clone();

    let cover_src =
        crate::cover_cache::resolve_cover_src(&book_id, book.cover_path.as_deref(), false);

    let init_t = title.clone();
    let init_a = author.clone();
    let init_p = book.publisher.clone().unwrap_or_default();
    let init_d = book.description.clone().unwrap_or_default();
    let init_tags = tags
        .iter()
        .map(|t| t.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");

    rsx! {
        div {
            class: "drawer-backdrop",
            onclick: move |_| on_close.call(()),

            div {
                class: "drawer-panel",
                onclick: move |evt| evt.stop_propagation(),

                div { class: "drawer-header",
                    span { style: "font-weight: 600; color: var(--text-muted); font-size: 13px;",
                        if *is_editing.read() {
                            {translate(locale, "drawer.editing_title")}
                        } else {
                            {translate(locale, "drawer.details_title")}
                        }
                    }
                    div { style: "display: flex; align-items: center; gap: 8px;",
                        {
                            let (score, missing) = crate::library_logic::compute_metadata_score(&book);
                            let color = if score >= 80 { "#34d399" } else if score >= 50 { "#fbbf24" } else { "#f87171" };
                            let tip = if missing.is_empty() {
                                "Métadonnées complètes".to_string()
                            } else {
                                format!("Manquant : {}", missing.join(", "))
                            };
                            rsx! {
                                span {
                                    title: "{tip}",
                                    style: "padding: 3px 8px; border-radius: 6px; background: rgba(255,255,255,0.06); font-size: 11px; font-weight: 700; color: {color};",
                                    "Score {score}%"
                                }
                            }
                        }
                        if !*is_editing.read() {
                            button {
                                class: "drawer-action-btn",
                                style: "padding: 4px 10px; font-size: 11px;",
                                onclick: {
                                    let t = init_t.clone();
                                    let a = init_a.clone();
                                    let p = init_p.clone();
                                    let d = init_d.clone();
                                    let tg = init_tags.clone();
                                    move |_| {
                                        edit_title.set(t.clone());
                                        edit_author.set(a.clone());
                                        edit_publisher.set(p.clone());
                                        edit_description.set(d.clone());
                                        edit_tags.set(tg.clone());
                                        is_editing.set(true);
                                    }
                                },
                                {translate(locale, "drawer.edit_btn")}
                            }
                        }

                        button {
                            class: "btn btn-icon",
                            onclick: move |_| on_close.call(()),
                            "✕"
                        }
                    }
                }

                if *is_editing.read() {
                    MetadataEditForm {
                        locale: locale,
                        book_id: book_id.clone(),
                        edit_title,
                        edit_author,
                        edit_series,
                        edit_series_index,
                        edit_publisher,
                        edit_isbn,
                        edit_publish_date,
                        edit_tags,
                        edit_description,
                        on_cancel: move |_| is_editing.set(false),
                        on_save: move |req| {
                            on_save_metadata.call(req);
                            is_editing.set(false);
                        },
                    }
                } else {
                    div { class: "drawer-cover-row",
                        div { style: "display: flex; flex-direction: column; gap: 8px; align-items: center;",
                            if let Some(src) = cover_src {
                                img { class: "drawer-cover", src: "{src}" }
                            } else {
                                div { class: "drawer-cover", style: "background: #25283d; display: flex; align-items: center; justify-content: center;",
                                    "📖"
                                }
                            }

                            button {
                                class: "drawer-action-btn",
                                style: "font-size: 11px; padding: 4px 8px; width: 100%;",
                                onclick: {
                                    let b_id = book_id.clone();
                                    let pick_title = translate(locale, "drawer.pick_cover_title").to_string();
                                    move |_| {
                                        let file = crate::platform::pick_file(
                                            &pick_title,
                                            &[("Images", &["jpg", "jpeg", "png", "webp"])],
                                        );
                                        if let Some(path) = file {
                                            on_change_cover.call((b_id.clone(), path));
                                        }
                                    }
                                },
                                {translate(locale, "drawer.cover_btn")}
                            }
                        }

                        div { class: "drawer-meta-col",
                            h3 { class: "drawer-title", "{title}" }
                            if let Some(ref s) = book.series {
                                div {
                                    style: "font-size: 13px; font-weight: 600; color: #a78bfa; margin-top: 2px;",
                                    if let Some(idx) = book.series_index {
                                        {translate_fmt(locale, "drawer.series_volume", &[("series", s), ("index", &idx.to_string())])}
                                    } else {
                                        "📚 {s}"
                                    }
                                }
                            }
                            div { class: "drawer-author", "{author}" }
                            div { style: "font-size: 12px; color: var(--text-muted); margin-top: 4px;", "{publisher}" }
                            if let Some(ref isbn_val) = book.isbn {
                                div { style: "font-size: 11px; color: var(--text-muted); font-family: monospace; margin-top: 3px;", "ISBN: {isbn_val}" }
                            }
                            if let Some(ref date_val) = book.publish_date {
                                div { style: "font-size: 11px; color: var(--text-muted); margin-top: 2px;", {translate_fmt(locale, "drawer.published", &[("date", date_val)])} }
                            }

                            // Quick Status & Rating pills (BookOrbit BookDetailView style)
                            if let Some(status_cb) = on_change_status {
                                div { style: "display: flex; gap: 5px; margin-top: 8px; flex-wrap: wrap;",
                                    for (label, target, is_cur) in [
                                        (
                                            translate(locale, "nav.to_read"),
                                            crate::components::KanbanTargetStatus::ToRead,
                                            !book.is_finished && book.progress_percent == 0.0,
                                        ),
                                        (
                                            translate(locale, "nav.reading"),
                                            crate::components::KanbanTargetStatus::Reading,
                                            !book.is_finished && book.progress_percent > 0.0,
                                        ),
                                        (
                                            translate(locale, "nav.finished"),
                                            crate::components::KanbanTargetStatus::Finished,
                                            book.is_finished,
                                        ),
                                    ] {
                                        {
                                            let b_id = book_id.clone();
                                            rsx! {
                                                button {
                                                    style: if is_cur {
                                                        "padding: 3px 9px; font-size: 11px; border-radius: 6px; background: var(--accent-primary); color: #fff; border: 1px solid var(--accent-primary); font-weight: 600; cursor: pointer;"
                                                    } else {
                                                        "padding: 3px 9px; font-size: 11px; border-radius: 6px; background: var(--bg-primary); color: var(--text-secondary); border: 1px solid var(--border-subtle); cursor: pointer;"
                                                    },
                                                    onclick: move |_| status_cb.call((b_id.clone(), target)),
                                                    "{label}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            if book.progress_percent > 0.0 {
                                div { style: "margin-top: 10px;",
                                    div { style: "display: flex; justify-content: space-between; font-size: 11px; margin-bottom: 4px;",
                                        span { {translate(locale, "drawer.progress")} }
                                        span { "{book.progress_percent:.0}%" }
                                    }
                                    div { class: "progress-track",
                                        div { class: "progress-bar", style: "width: {book.progress_percent}%;" }
                                    }
                                }
                            }

                            div { style: "margin-top: auto; display: flex; flex-direction: column; gap: 8px;",
                                button {
                                    class: "btn btn-primary",
                                    style: "width: 100%; justify-content: center;",
                                    onclick: {
                                        let book_clone = book.clone();
                                        move |_| on_read.call(book_clone.clone())
                                    },
                                    {translate(locale, "drawer.read_book")}
                                }

                                div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 8px;",
                                    button {
                                        class: "drawer-action-btn",
                                        style: "justify-content: center; background: rgba(59, 130, 246, 0.12); color: #60a5fa; border-color: rgba(59, 130, 246, 0.3); font-weight: 600;",
                                        onclick: {
                                            let b_id = book_id.clone();
                                            move |_| on_enrich_metadata.call(b_id.clone())
                                        },
                                        {translate(locale, "drawer.enrich")}
                                    }
                                    button {
                                        class: "drawer-action-btn",
                                        style: "justify-content: center; background: rgba(167, 139, 250, 0.12); color: #c4b5fd; border-color: rgba(167, 139, 250, 0.3); font-weight: 600;",
                                        onclick: {
                                            let b_id = book_id.clone();
                                            move |_| on_export_file.call((b_id.clone(), None))
                                        },
                                        {translate(locale, "drawer.export")}
                                    }
                                }

                                button {
                                    class: "drawer-action-btn",
                                    style: "width: 100%; justify-content: space-between; font-size: 12px; color: var(--text-secondary);",
                                    onclick: move |_| {
                                        let cur = *show_more_actions.read();
                                        show_more_actions.set(!cur);
                                    },
                                    span { {translate(locale, "drawer.more_actions")} }
                                    span { if *show_more_actions.read() { "▴" } else { "▾" } }
                                }

                                if *show_more_actions.read() {
                                    div {
                                        style: "display: flex; flex-direction: column; gap: 6px; padding: 8px; border-radius: 10px; background: rgba(255,255,255,0.03); border: 1px solid var(--border-subtle);",
                                        button {
                                            class: "drawer-action-btn",
                                            style: "width: 100%; justify-content: center; background: rgba(99, 102, 241, 0.12); color: #818cf8; border-color: rgba(99, 102, 241, 0.3); font-weight: 600;",
                                            onclick: {
                                                let book_clone = book.clone();
                                                move |_| on_open_translate.call(book_clone.clone())
                                            },
                                            {translate(locale, "drawer.ai_translate")}
                                        }

                                        button {
                                            class: "drawer-action-btn",
                                            style: "width: 100%; justify-content: center; background: rgba(52, 211, 153, 0.12); color: #34d399; border-color: rgba(52, 211, 153, 0.3); font-weight: 600;",
                                            onclick: {
                                                let b_id = book_id.clone();
                                                move |_| on_export_notes_md.call(b_id.clone())
                                            },
                                            {translate(locale, "drawer.export_notes")}
                                        }

                                        if can_send_kindle {
                                            button {
                                                class: "drawer-action-btn",
                                                style: "width: 100%; justify-content: center; background: rgba(245, 158, 11, 0.12); color: #fbbf24; border-color: rgba(245, 158, 11, 0.3); font-weight: 600;",
                                                disabled: is_sending_kindle,
                                                onclick: {
                                                    let book_clone = book.clone();
                                                    move |_| on_send_to_kindle.call(book_clone.clone())
                                                },
                                                span { if is_sending_kindle { "⏳" } else { "✉️" } }
                                                {if is_sending_kindle { translate(locale, "drawer.sending_kindle") } else { translate(locale, "drawer.send_kindle") }}
                                            }
                                        }

                                        if book.formats.iter().any(|f| f.format.eq_ignore_ascii_case("epub") || f.format.eq_ignore_ascii_case("kepub")) {
                                            div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 6px;",
                                                button {
                                                    class: "drawer-action-btn",
                                                    style: "justify-content: center; font-size: 11.5px; background: rgba(14, 165, 233, 0.12); color: #38bdf8; border-color: rgba(14, 165, 233, 0.3); font-weight: 600;",
                                                    onclick: {
                                                        let book_clone = book.clone();
                                                        move |_| on_open_epub_editor.call(book_clone.clone())
                                                    },
                                                    "🛠️ TOC EPUB"
                                                }
                                                button {
                                                    class: "drawer-action-btn",
                                                    style: "justify-content: center; font-size: 11.5px; background: rgba(245, 158, 11, 0.12); color: #fbbf24; border-color: rgba(245, 158, 11, 0.3); font-weight: 600;",
                                                    onclick: {
                                                        let b_id = book_id.clone();
                                                        move |_| on_polish_epub.call(b_id.clone())
                                                    },
                                                    "✨ Polish EPUB"
                                                }
                                            }
                                        }
                                    }
                                }

                                if !custom_columns.is_empty() {
                                    div {
                                        style: "margin-top: 8px; padding: 10px; border-radius: 8px; background: rgba(255,255,255,0.03); border: 1px solid rgba(255,255,255,0.06); display: flex; flex-direction: column; gap: 6px;",
                                        div {
                                            style: "font-size: 11px; font-weight: 700; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.04em;",
                                            {translate(locale, "drawer.custom_columns")}
                                        }
                                        for col in custom_columns.iter() {
                                            {
                                                let b_id = book_id.clone();
                                                let col_key = col.key.clone();
                                                let col_label = col.label.clone();
                                                let val = custom_values.get(&col.key).cloned().unwrap_or_default();
                                                rsx! {
                                                    div {
                                                        style: "display: flex; align-items: center; justify-content: space-between; gap: 8px;",
                                                        label { style: "font-size: 12px; color: var(--text-secondary);", "{col_label}" }
                                                        input {
                                                            r#type: "text",
                                                            style: "width: 140px; font-size: 12px; padding: 3px 6px; border-radius: 4px; background: rgba(0,0,0,0.25); color: #e2e8f0; border: 1px solid rgba(255,255,255,0.1);",
                                                            value: "{val}",
                                                            onchange: move |e| on_save_custom_value.call((b_id.clone(), col_key.clone(), e.value())),
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                if !connected_devices.is_empty() {
                                    for dev in connected_devices.iter() {
                                        {
                                            let b_id = book_id.clone();
                                            let dev_kind = dev.kind;
                                            let dev_name = dev.name.clone();
                                            let icon = match dev_kind {
                                                vellum_domain::DeviceKind::Kobo => "📱 Kobo (KEPUB)",
                                                vellum_domain::DeviceKind::Kindle => "📱 Kindle (AZW3/MOBI)",
                                                vellum_domain::DeviceKind::Generic => translate(locale, "drawer.generic_device"),
                                            };
                                            rsx! {
                                                button {
                                                    class: "drawer-action-btn",
                                                    style: "width: 100%; justify-content: center; background: rgba(16, 185, 129, 0.12); color: #34d399; border-color: rgba(16, 185, 129, 0.3); font-weight: 600;",
                                                    onclick: move |_| on_export_to_device.call((b_id.clone(), dev_kind)),
                                                    title: translate_fmt(locale, "drawer.export_to_title", &[("device", &dev_name)]),
                                                    {translate_fmt(locale, "drawer.send_to_device", &[("device", icon)])}
                                                }
                                            }
                                        }
                                    }
                                } else if *show_more_actions.read() {
                                    div { style: "display: flex; gap: 6px;",
                                        button {
                                            class: "drawer-action-btn",
                                            style: "flex: 1; font-size: 11px; justify-content: center;",
                                            onclick: {
                                                let b_id = book_id.clone();
                                                move |_| on_export_to_device.call((b_id.clone(), vellum_domain::DeviceKind::Kobo))
                                            },
                                            {translate(locale, "drawer.to_kobo")}
                                        }
                                        button {
                                            class: "drawer-action-btn",
                                            style: "flex: 1; font-size: 11px; justify-content: center;",
                                            onclick: {
                                                let b_id = book_id.clone();
                                                move |_| on_export_to_device.call((b_id.clone(), vellum_domain::DeviceKind::Kindle))
                                            },
                                            {translate(locale, "drawer.to_kindle")}
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if !tags.is_empty() {
                        div {
                            div { class: "sidebar-section-title", {translate(locale, "drawer.categories_tags")} }
                            div { class: "tags-cloud",
                                for t in tags.iter() {
                                    span { class: "tag-pill", "#{t.name}" }
                                }
                            }
                        }
                    }

                    div {
                        div {
                            style: "display: flex; align-items: center; justify-content: space-between; margin-bottom: 6px;",
                            div { class: "sidebar-section-title", style: "margin-bottom: 0;", {translate(locale, "drawer.files_formats")} }
                            if let Some(attach_cb) = on_attach_format {
                                button {
                                    class: "btn btn-secondary",
                                    style: "padding: 3px 8px; font-size: 11px; display: inline-flex; align-items: center; gap: 4px;",
                                    onclick: {
                                        let b_id = book_id.clone();
                                        move |_| attach_cb.call(b_id.clone())
                                    },
                                    crate::components::LucideIcon { kind: crate::components::IconKind::Plus, size: 12 }
                                    span { "Ajouter format" }
                                }
                            }
                        }
                        div { style: "display: flex; flex-direction: column; gap: 6px;",
                            for fmt in formats.iter() {
                                {
                                    let b_id = book_id.clone();
                                    let fmt_ext = fmt.format.clone();
                                    rsx! {
                                        div {
                                            style: "display: flex; align-items: center; justify-content: space-between; padding: 8px 12px; background: var(--bg-primary); border: 1px solid var(--border-subtle); border-radius: 8px; font-size: 12px; gap: 8px;",
                                            div { style: "display: flex; align-items: center; gap: 8px; min-width: 0; flex: 1;",
                                                span { class: "format-pill", "{fmt.format.to_uppercase()}" }
                                                span { style: "color: var(--text-primary); white-space: nowrap; overflow: hidden; text-overflow: ellipsis;", "{fmt.filename}" }
                                            }
                                            div { style: "display: flex; align-items: center; gap: 8px; flex-shrink: 0;",
                                                span { style: "color: var(--text-muted);", "{fmt.filesize / 1024} KB" }
                                                button {
                                                    class: "btn btn-secondary",
                                                    style: "padding: 3px 7px; font-size: 11px; display: inline-flex; align-items: center;",
                                                    title: "Télécharger ce format",
                                                    onclick: move |_| on_export_file.call((b_id.clone(), Some(fmt_ext.clone()))),
                                                    crate::components::LucideIcon { kind: crate::components::IconKind::Download, size: 12 }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if !convertible_formats.is_empty() {
                            div { style: "margin-top: 10px; padding: 10px 12px; background: rgba(99, 102, 241, 0.08); border: 1px dashed rgba(99, 102, 241, 0.3); border-radius: 8px;",
                                div { style: "font-size: 11px; font-weight: 600; color: var(--accent-primary); margin-bottom: 6px;", {translate(locale, "drawer.convert_to")} }
                                div { style: "display: flex; flex-wrap: wrap; gap: 6px;",
                                    for target in convertible_formats.iter() {
                                        {
                                            let b_id = book_id.clone();
                                            let tgt = target.clone();
                                            rsx! {
                                                button {
                                                    class: "btn btn-secondary",
                                                    style: "padding: 3px 8px; font-size: 11px; border-radius: 6px;",
                                                    onclick: move |_| on_convert.call((b_id.clone(), tgt.clone())),
                                                    "⚡ {target}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    div {
                        div { class: "sidebar-section-title", {translate(locale, "drawer.synopsis")} }
                        div { class: "drawer-description", "{description}" }
                    }

                    div { style: "margin-top: auto; padding-top: 16px; border-top: 1px solid var(--border-subtle); display: flex; justify-content: flex-end;",
                        button {
                            class: "btn btn-secondary",
                            style: "color: var(--danger); border-color: rgba(239, 68, 68, 0.3); font-size: 12px;",
                            onclick: {
                                let b_id = book_id.clone();
                                move |_| on_delete.call(b_id.clone())
                            },
                            {translate(locale, "drawer.delete_book")}
                        }
                    }
                }
            }
        }
    }
}
