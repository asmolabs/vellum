use dioxus::prelude::*;
use std::path::PathBuf;
use vellum_domain::Book;

#[component]
pub fn DetailDrawer(
    book: Book,
    convertible_formats: Vec<String>,
    connected_devices: Vec<vellum_core::ConnectedDevice>,
    on_close: EventHandler<()>,
    on_read: EventHandler<Book>,
    on_delete: EventHandler<String>,
    on_convert: EventHandler<(String, String)>,
    on_change_cover: EventHandler<(String, PathBuf)>,
    on_save_metadata: EventHandler<(
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<f64>,
        Vec<String>,
    )>,
    on_enrich_metadata: EventHandler<String>,
    on_open_translate: EventHandler<Book>,
    on_export_to_device: EventHandler<(String, vellum_domain::DeviceKind)>,
    can_send_kindle: bool,
    is_sending_kindle: bool,
    on_send_to_kindle: EventHandler<Book>,
    on_export_file: EventHandler<(String, Option<String>)>,
    on_open_epub_editor: EventHandler<Book>,
) -> Element {
    let mut is_editing = use_signal(|| false);

    let mut edit_title = use_signal(|| book.title.clone());
    let mut edit_author = use_signal(|| book.author.clone());
    let mut edit_series = use_signal(|| book.series.clone().unwrap_or_default());
    let mut edit_series_index = use_signal(|| book.series_index.map(|i| i.to_string()).unwrap_or_default());
    let mut edit_publisher = use_signal(|| book.publisher.clone().unwrap_or_default());
    let mut edit_publish_date = use_signal(|| book.publish_date.clone().unwrap_or_default());
    let mut edit_isbn = use_signal(|| book.isbn.clone().unwrap_or_default());
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
        .unwrap_or_else(|| "Éditeur non renseigné".to_string());
    let description = book
        .description
        .clone()
        .unwrap_or_else(|| "Aucune description disponible.".to_string());
    let formats = book.formats.clone();
    let tags = book.tags.clone();
    let book_id = book.id.clone();

    use base64::Engine;
    let cover_src = book.cover_path.as_ref().and_then(|p| {
        if p.starts_with("data:") || p.starts_with("http") {
            Some(p.clone())
        } else {
            let full_path = if std::path::Path::new(p).is_absolute() {
                std::path::PathBuf::from(p)
            } else {
                dirs::home_dir().map(|h| h.join(".vellum").join("library").join(p))?
            };

            if full_path.exists() {
                if let Ok(bytes) = std::fs::read(&full_path) {
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
                    return Some(format!(
                        "data:{mime};base64,{}",
                        base64::engine::general_purpose::STANDARD.encode(&bytes)
                    ));
                }
            }
            None
        }
    });

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
                            "Édition des métadonnées"
                        } else {
                            "Détails de l'ouvrage"
                        }
                    }
                    div { style: "display: flex; align-items: center; gap: 8px;",
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
                                "✏️ Modifier"
                            }
                        }

                        button {
                            class: "btn btn-icon",
                            onclick: move |_| on_close.call(()),
                            "✕"
                        }
                    }
                }

                // If in Edit Mode
                if *is_editing.read() {
                    div { style: "display: flex; flex-direction: column; gap: 14px; padding: 4px 0;",
                        div { class: "drawer-form-group",
                            label { class: "drawer-form-label", "Titre :" }
                            input {
                                class: "drawer-input",
                                value: "{edit_title}",
                                oninput: move |e| edit_title.set(e.value()),
                            }
                        }

                        div { class: "drawer-form-group",
                            label { class: "drawer-form-label", "Auteur :" }
                            input {
                                class: "drawer-input",
                                value: "{edit_author}",
                                oninput: move |e| edit_author.set(e.value()),
                            }
                        }

                        div { style: "display: grid; grid-template-columns: 2fr 1fr; gap: 10px;",
                            div { class: "drawer-form-group",
                                label { class: "drawer-form-label", "Série / Saga :" }
                                input {
                                    class: "drawer-input",
                                    placeholder: "Ex: Le Cycle de Dune",
                                    value: "{edit_series}",
                                    oninput: move |e| edit_series.set(e.value()),
                                }
                            }
                            div { class: "drawer-form-group",
                                label { class: "drawer-form-label", "Tome / Numéro :" }
                                input {
                                    class: "drawer-input",
                                    placeholder: "Ex: 1",
                                    value: "{edit_series_index}",
                                    oninput: move |e| edit_series_index.set(e.value()),
                                }
                            }
                        }

                        div { class: "drawer-form-group",
                            label { class: "drawer-form-label", "Éditeur :" }
                            input {
                                class: "drawer-input",
                                placeholder: "Nom de l'éditeur...",
                                value: "{edit_publisher}",
                                oninput: move |e| edit_publisher.set(e.value()),
                            }
                        }

                        div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 10px;",
                            div { class: "drawer-form-group",
                                label { class: "drawer-form-label", "ISBN :" }
                                input {
                                    class: "drawer-input",
                                    placeholder: "Ex: 9782207249117",
                                    value: "{edit_isbn}",
                                    oninput: move |e| edit_isbn.set(e.value()),
                                }
                            }
                            div { class: "drawer-form-group",
                                label { class: "drawer-form-label", "Date de publication :" }
                                input {
                                    class: "drawer-input",
                                    placeholder: "Ex: 1965",
                                    value: "{edit_publish_date}",
                                    oninput: move |e| edit_publish_date.set(e.value()),
                                }
                            }
                        }

                        div { class: "drawer-form-group",
                            label { class: "drawer-form-label", "Tags / Catégories (séparés par virgule) :" }
                            input {
                                class: "drawer-input",
                                placeholder: "Roman, Science-Fiction, Hugo Award...",
                                value: "{edit_tags}",
                                oninput: move |e| edit_tags.set(e.value()),
                            }
                        }

                        div { class: "drawer-form-group",
                            label { class: "drawer-form-label", "Synopsis / Description :" }
                            textarea {
                                class: "drawer-textarea",
                                rows: "5",
                                value: "{edit_description}",
                                oninput: move |e| edit_description.set(e.value()),
                            }
                        }

                        div { style: "display: flex; justify-content: flex-end; gap: 10px; margin-top: 10px;",
                            button {
                                class: "btn btn-secondary",
                                onclick: move |_| is_editing.set(false),
                                "Annuler"
                            }
                            button {
                                class: "btn btn-primary",
                                onclick: {
                                    let b_id = book_id.clone();
                                    move |_| {
                                        let t_list: Vec<String> = edit_tags
                                            .read()
                                            .split(',')
                                            .map(|s| s.trim().to_string())
                                            .filter(|s| !s.is_empty())
                                            .collect();
                                        let pub_val = if edit_publisher.read().trim().is_empty() {
                                            None
                                        } else {
                                            Some(edit_publisher.read().trim().to_string())
                                        };
                                        let desc_val = if edit_description.read().trim().is_empty() {
                                            None
                                        } else {
                                            Some(edit_description.read().trim().to_string())
                                        };
                                        let isbn_val = if edit_isbn.read().trim().is_empty() {
                                            None
                                        } else {
                                            Some(edit_isbn.read().trim().to_string())
                                        };
                                        let date_val = if edit_publish_date.read().trim().is_empty() {
                                            None
                                        } else {
                                            Some(edit_publish_date.read().trim().to_string())
                                        };
                                        let series_val = if edit_series.read().trim().is_empty() {
                                            None
                                        } else {
                                            Some(edit_series.read().trim().to_string())
                                        };
                                        let series_idx_val = edit_series_index.read().trim().parse::<f64>().ok();

                                        on_save_metadata.call((
                                            b_id.clone(),
                                            edit_title.read().clone(),
                                            edit_author.read().clone(),
                                            pub_val,
                                            desc_val,
                                            isbn_val,
                                            date_val,
                                            series_val,
                                            series_idx_val,
                                            t_list,
                                        ));
                                        is_editing.set(false);
                                    }
                                },
                                "💾 Enregistrer"
                            }
                        }
                    }
                } else {
                    // Standard View Mode
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
                                    move |_| {
                                        let file = rfd::FileDialog::new()
                                            .set_title("Sélectionner une nouvelle image de couverture")
                                            .add_filter("Images", &["jpg", "jpeg", "png", "webp"])
                                            .pick_file();
                                        if let Some(path) = file {
                                            on_change_cover.call((b_id.clone(), path));
                                        }
                                    }
                                },
                                "🖼 Couverture"
                            }
                        }

                        div { class: "drawer-meta-col",
                            h3 { class: "drawer-title", "{title}" }
                            if let Some(ref s) = book.series {
                                div {
                                    style: "font-size: 13px; font-weight: 600; color: #a78bfa; margin-top: 2px;",
                                    if let Some(idx) = book.series_index {
                                        "📚 {s} • Tome {idx}"
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
                                div { style: "font-size: 11px; color: var(--text-muted); margin-top: 2px;", "Publié : {date_val}" }
                            }

                            if book.progress_percent > 0.0 {
                                div { style: "margin-top: 10px;",
                                    div { style: "display: flex; justify-content: space-between; font-size: 11px; margin-bottom: 4px;",
                                        span { "Progression" }
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
                                    style: "width: 100%;",
                                    onclick: {
                                        let book_clone = book.clone();
                                        move |_| on_read.call(book_clone.clone())
                                    },
                                    "📖 Lire l'ouvrage"
                                }

                                button {
                                    class: "drawer-action-btn",
                                    style: "width: 100%; justify-content: center; background: rgba(59, 130, 246, 0.12); color: #60a5fa; border-color: rgba(59, 130, 246, 0.3); font-weight: 600;",
                                    onclick: {
                                        let b_id = book_id.clone();
                                        move |_| on_enrich_metadata.call(b_id.clone())
                                    },
                                    "✨ Enrichir avec Google Books"
                                }

                                button {
                                    class: "drawer-action-btn",
                                    style: "width: 100%; justify-content: center; background: rgba(99, 102, 241, 0.12); color: #818cf8; border-color: rgba(99, 102, 241, 0.3); font-weight: 600;",
                                    onclick: {
                                        let book_clone = book.clone();
                                        move |_| on_open_translate.call(book_clone.clone())
                                    },
                                    "🌐 Traduire avec l'IA"
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
                                        if is_sending_kindle { " Envoi en cours vers Kindle..." } else { " Envoyer à ma Kindle (sans fil)" }
                                    }
                                }

                                button {
                                    class: "drawer-action-btn",
                                    style: "width: 100%; justify-content: center; background: rgba(167, 139, 250, 0.12); color: #c4b5fd; border-color: rgba(167, 139, 250, 0.3); font-weight: 600;",
                                    onclick: {
                                        let b_id = book_id.clone();
                                        move |_| on_export_file.call((b_id.clone(), None))
                                    },
                                    "💾 Exporter le fichier..."
                                }

                                if book.formats.iter().any(|f| f.format.eq_ignore_ascii_case("epub") || f.format.eq_ignore_ascii_case("kepub")) {
                                    button {
                                        class: "drawer-action-btn",
                                        style: "width: 100%; justify-content: center; background: rgba(14, 165, 233, 0.12); color: #38bdf8; border-color: rgba(14, 165, 233, 0.3); font-weight: 600;",
                                        onclick: {
                                            let book_clone = book.clone();
                                            move |_| on_open_epub_editor.call(book_clone.clone())
                                        },
                                        "🛠️ Inspecteur & TOC EPUB"
                                    }
                                }

                                // eReader Export Actions
                                if !connected_devices.is_empty() {
                                    for dev in connected_devices.iter() {
                                        {
                                            let b_id = book_id.clone();
                                            let dev_kind = dev.kind;
                                            let dev_name = dev.name.clone();
                                            let icon = match dev_kind {
                                                vellum_domain::DeviceKind::Kobo => "📱 Kobo (KEPUB)",
                                                vellum_domain::DeviceKind::Kindle => "📱 Kindle (AZW3/MOBI)",
                                                vellum_domain::DeviceKind::Generic => "📱 Liseuse",
                                            };
                                            rsx! {
                                                button {
                                                    class: "drawer-action-btn",
                                                    style: "width: 100%; justify-content: center; background: rgba(16, 185, 129, 0.12); color: #34d399; border-color: rgba(16, 185, 129, 0.3); font-weight: 600;",
                                                    onclick: move |_| on_export_to_device.call((b_id.clone(), dev_kind)),
                                                    title: "Exporter vers {dev_name}",
                                                    "📤 Envoyer vers {icon}"
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    div { style: "display: flex; gap: 6px;",
                                        button {
                                            class: "drawer-action-btn",
                                            style: "flex: 1; font-size: 11px; justify-content: center;",
                                            onclick: {
                                                let b_id = book_id.clone();
                                                move |_| on_export_to_device.call((b_id.clone(), vellum_domain::DeviceKind::Kobo))
                                            },
                                            "📱 Vers Kobo"
                                        }
                                        button {
                                            class: "drawer-action-btn",
                                            style: "flex: 1; font-size: 11px; justify-content: center;",
                                            onclick: {
                                                let b_id = book_id.clone();
                                                move |_| on_export_to_device.call((b_id.clone(), vellum_domain::DeviceKind::Kindle))
                                            },
                                            "📱 Vers Kindle"
                                        }
                                    }
                                }
                            }
                        }
                    }


                    // Tags
                    if !tags.is_empty() {
                        div {
                            div { class: "sidebar-section-title", "Catégories & Tags" }
                            div { class: "tags-cloud",
                                for t in tags.iter() {
                                    span { class: "tag-pill", "#{t.name}" }
                                }
                            }
                        }
                    }

                    // Formats Available & Conversions
                    div {
                        div { class: "sidebar-section-title", "Fichiers & Formats" }
                        div { style: "display: flex; flex-direction: column; gap: 6px;",
                            for fmt in formats.iter() {
                                div {
                                    style: "display: flex; align-items: center; justify-content: space-between; padding: 8px 12px; background: rgba(255,255,255,0.03); border: 1px solid var(--border-subtle); border-radius: 8px; font-size: 12px;",
                                    div { style: "display: flex; align-items: center; gap: 8px;",
                                        span { class: "format-pill", "{fmt.format.to_uppercase()}" }
                                        span { style: "color: var(--text-primary);", "{fmt.filename}" }
                                    }
                                    span { style: "color: var(--text-muted);", "{fmt.filesize / 1024} KB" }
                                }
                            }
                        }

                        if !convertible_formats.is_empty() {
                            div { style: "margin-top: 10px; padding: 10px 12px; background: rgba(99, 102, 241, 0.08); border: 1px dashed rgba(99, 102, 241, 0.3); border-radius: 8px;",
                                div { style: "font-size: 11px; font-weight: 600; color: var(--accent-primary); margin-bottom: 6px;", "Convertir vers (format pivot) :" }
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

                    // Description
                    div {
                        div { class: "sidebar-section-title", "Synopsis" }
                        div { class: "drawer-description", "{description}" }
                    }

                    // Danger zone (Delete)
                    div { style: "margin-top: auto; padding-top: 16px; border-top: 1px solid var(--border-subtle); display: flex; justify-content: flex-end;",
                        button {
                            class: "btn btn-secondary",
                            style: "color: var(--danger); border-color: rgba(239, 68, 68, 0.3); font-size: 12px;",
                            onclick: {
                                let b_id = book_id.clone();
                                move |_| on_delete.call(b_id.clone())
                            },
                            "🗑 Supprimer de la bibliothèque"
                        }
                    }
                }
            }
        }
    }
}
