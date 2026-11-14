//! Sections complémentaires des paramètres : Watch Folder, colonnes personnalisées et export catalogue.

use dioxus::prelude::*;
use vellum_core::{translate, translate_fmt, CatalogFormat, Locale};
use vellum_domain::{CustomColumnDefinition, CustomColumnType};

#[component]
pub(super) fn LibraryToolsSection(
    locale: Locale,
    custom_columns: Vec<CustomColumnDefinition>,
    on_create_custom_column: EventHandler<(String, String, CustomColumnType)>,
    on_delete_custom_column: EventHandler<String>,
    watch_folder_path: String,
    on_scan_watch_folder: EventHandler<String>,
    on_export_catalog: EventHandler<CatalogFormat>,
) -> Element {
    let mut new_col_key = use_signal(String::new);
    let mut new_col_label = use_signal(String::new);
    let mut new_col_type = use_signal(|| CustomColumnType::Text);
    let mut watch_dir_input = use_signal(|| watch_folder_path.clone());

    rsx! {
        // Watch Folder (Auto-Add)
        div { style: "display: flex; flex-direction: column; gap: 8px; padding: 14px; background: rgba(255,255,255,0.03); border: 1px solid var(--border-subtle); border-radius: 10px;",
            div { style: "font-weight: 600; font-size: 13px;", {translate(locale, "settings.watch_title")} }
            p { style: "font-size: 12px; color: var(--text-secondary);",
                {translate(locale, "settings.watch_desc")}
            }
            div { style: "display: flex; gap: 8px;",
                input {
                    class: "search-input",
                    style: "flex: 1; font-size: 12px;",
                    r#type: "text",
                    placeholder: translate(locale, "settings.watch_placeholder"),
                    value: "{watch_dir_input}",
                    oninput: move |e| watch_dir_input.set(e.value()),
                }
                button {
                    class: "btn btn-primary",
                    style: "font-size: 12px;",
                    onclick: move |_| {
                        let dir = watch_dir_input.read().trim().to_string();
                        if !dir.is_empty() {
                            on_scan_watch_folder.call(dir);
                        }
                    },
                    {translate(locale, "settings.watch_scan")}
                }
            }
        }

        // Colonnes personnalisées
        div { style: "display: flex; flex-direction: column; gap: 8px; padding: 14px; background: rgba(255,255,255,0.03); border: 1px solid var(--border-subtle); border-radius: 10px;",
            div {
                style: "font-weight: 600; font-size: 13px;",
                {
                    let count_str = custom_columns.len().to_string();
                    translate_fmt(locale, "settings.custom_cols_title", &[("count", &count_str)])
                }
            }
            if !custom_columns.is_empty() {
                div { style: "display: flex; flex-direction: column; gap: 4px;",
                    for col in custom_columns.iter() {
                        {
                            let col_id = col.id.clone();
                            rsx! {
                                div {
                                    style: "display: flex; justify-content: space-between; align-items: center; padding: 6px 10px; background: rgba(0,0,0,0.2); border-radius: 6px; font-size: 12px;",
                                    span { "{col.label} ({col.key}) • {col.column_type.as_str()}" }
                                    button {
                                        style: "background: none; border: none; color: #f87171; cursor: pointer; font-weight: 700;",
                                        onclick: move |_| on_delete_custom_column.call(col_id.clone()),
                                        "×"
                                    }
                                }
                            }
                        }
                    }
                }
            }
            div { style: "display: flex; gap: 6px; flex-wrap: wrap;",
                input {
                    class: "search-input",
                    style: "flex: 1; min-width: 110px; font-size: 12px;",
                    r#type: "text",
                    placeholder: translate(locale, "settings.custom_col_key_ph"),
                    value: "{new_col_key}",
                    oninput: move |e| new_col_key.set(e.value()),
                }
                input {
                    class: "search-input",
                    style: "flex: 1; min-width: 130px; font-size: 12px;",
                    r#type: "text",
                    placeholder: translate(locale, "settings.custom_col_label_ph"),
                    value: "{new_col_label}",
                    oninput: move |e| new_col_label.set(e.value()),
                }
                select {
                    class: "sort-select",
                    style: "font-size: 12px;",
                    onchange: move |e| {
                        if let Some(t) = CustomColumnType::parse(&e.value()) {
                            new_col_type.set(t);
                        }
                    },
                    option { value: "text", {translate(locale, "settings.type_text")} }
                    option { value: "number", {translate(locale, "settings.type_number")} }
                    option { value: "boolean", {translate(locale, "settings.type_boolean")} }
                    option { value: "date", {translate(locale, "settings.type_date")} }
                    option { value: "rating", {translate(locale, "settings.type_rating")} }
                }
                button {
                    class: "btn btn-primary",
                    style: "font-size: 12px;",
                    onclick: move |_| {
                        let k = new_col_key.read().trim().to_string();
                        let l = new_col_label.read().trim().to_string();
                        let t = *new_col_type.read();
                        if !k.is_empty() && !l.is_empty() {
                            on_create_custom_column.call((k, l, t));
                            new_col_key.set(String::new());
                            new_col_label.set(String::new());
                        }
                    },
                    {translate(locale, "settings.add_btn")}
                }
            }
        }

        // Export du catalogue
        div { style: "display: flex; flex-direction: column; gap: 8px; padding: 14px; background: rgba(255,255,255,0.03); border: 1px solid var(--border-subtle); border-radius: 10px;",
            div { style: "font-weight: 600; font-size: 13px;", {translate(locale, "settings.export_title")} }
            p { style: "font-size: 12px; color: var(--text-secondary);",
                {translate(locale, "settings.export_desc")}
            }
            div { style: "display: flex; gap: 8px; flex-wrap: wrap;",
                button {
                    class: "btn btn-secondary",
                    style: "font-size: 12px;",
                    onclick: move |_| on_export_catalog.call(CatalogFormat::Csv),
                    {translate(locale, "settings.export_csv")}
                }
                button {
                    class: "btn btn-secondary",
                    style: "font-size: 12px;",
                    onclick: move |_| on_export_catalog.call(CatalogFormat::Json),
                    {translate(locale, "settings.export_json")}
                }
                button {
                    class: "btn btn-secondary",
                    style: "font-size: 12px;",
                    onclick: move |_| on_export_catalog.call(CatalogFormat::Bibtex),
                    {translate(locale, "settings.export_bibtex")}
                }
            }
        }
    }
}
