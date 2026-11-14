use dioxus::prelude::*;
use vellum_core::{translate, Locale};

use super::SaveMetadataRequest;

fn optional_trimmed(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[component]
pub fn MetadataEditForm(
    locale: Locale,
    book_id: String,
    mut edit_title: Signal<String>,
    mut edit_author: Signal<String>,
    mut edit_series: Signal<String>,
    mut edit_series_index: Signal<String>,
    mut edit_publisher: Signal<String>,
    mut edit_isbn: Signal<String>,
    mut edit_publish_date: Signal<String>,
    mut edit_tags: Signal<String>,
    mut edit_description: Signal<String>,
    on_cancel: EventHandler<()>,
    on_save: EventHandler<SaveMetadataRequest>,
) -> Element {
    rsx! {
        div { style: "display: flex; flex-direction: column; gap: 14px; padding: 4px 0;",
            div { class: "drawer-form-group",
                label { class: "drawer-form-label", {translate(locale, "drawer.form_title")} }
                input {
                    class: "drawer-input",
                    value: "{edit_title}",
                    oninput: move |e| edit_title.set(e.value()),
                }
            }

            div { class: "drawer-form-group",
                label { class: "drawer-form-label", {translate(locale, "drawer.form_author")} }
                input {
                    class: "drawer-input",
                    value: "{edit_author}",
                    oninput: move |e| edit_author.set(e.value()),
                }
            }

            div { style: "display: grid; grid-template-columns: 2fr 1fr; gap: 10px;",
                div { class: "drawer-form-group",
                    label { class: "drawer-form-label", {translate(locale, "drawer.form_series")} }
                    input {
                        class: "drawer-input",
                        placeholder: translate(locale, "drawer.form_series_placeholder"),
                        value: "{edit_series}",
                        oninput: move |e| edit_series.set(e.value()),
                    }
                }
                div { class: "drawer-form-group",
                    label { class: "drawer-form-label", {translate(locale, "drawer.form_series_index")} }
                    input {
                        class: "drawer-input",
                        placeholder: translate(locale, "drawer.form_series_index_placeholder"),
                        value: "{edit_series_index}",
                        oninput: move |e| edit_series_index.set(e.value()),
                    }
                }
            }

            div { class: "drawer-form-group",
                label { class: "drawer-form-label", {translate(locale, "drawer.form_publisher")} }
                input {
                    class: "drawer-input",
                    placeholder: translate(locale, "drawer.form_publisher_placeholder"),
                    value: "{edit_publisher}",
                    oninput: move |e| edit_publisher.set(e.value()),
                }
            }

            div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 10px;",
                div { class: "drawer-form-group",
                    label { class: "drawer-form-label", {translate(locale, "drawer.form_isbn")} }
                    input {
                        class: "drawer-input",
                        placeholder: "Ex: 9782207249117",
                        value: "{edit_isbn}",
                        oninput: move |e| edit_isbn.set(e.value()),
                    }
                }
                div { class: "drawer-form-group",
                    label { class: "drawer-form-label", {translate(locale, "drawer.form_publish_date")} }
                    input {
                        class: "drawer-input",
                        placeholder: "Ex: 1965",
                        value: "{edit_publish_date}",
                        oninput: move |e| edit_publish_date.set(e.value()),
                    }
                }
            }

            div { class: "drawer-form-group",
                label { class: "drawer-form-label", {translate(locale, "drawer.form_tags")} }
                input {
                    class: "drawer-input",
                    placeholder: translate(locale, "drawer.form_tags_placeholder"),
                    value: "{edit_tags}",
                    oninput: move |e| edit_tags.set(e.value()),
                }
            }

            div { class: "drawer-form-group",
                label { class: "drawer-form-label", {translate(locale, "drawer.form_description")} }
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
                    onclick: move |_| on_cancel.call(()),
                    {translate(locale, "action.cancel")}
                }
                button {
                    class: "btn btn-primary",
                    onclick: move |_| {
                        let tags: Vec<String> = edit_tags
                            .read()
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                        let series_index = edit_series_index.read().trim().parse::<f64>().ok();

                        on_save.call(SaveMetadataRequest {
                            book_id: book_id.clone(),
                            title: edit_title.read().clone(),
                            author: edit_author.read().clone(),
                            publisher: optional_trimmed(&edit_publisher.read()),
                            description: optional_trimmed(&edit_description.read()),
                            isbn: optional_trimmed(&edit_isbn.read()),
                            publish_date: optional_trimmed(&edit_publish_date.read()),
                            series: optional_trimmed(&edit_series.read()),
                            series_index,
                            tags,
                        });
                    },
                    {translate(locale, "drawer.form_save")}
                }
            }
        }
    }
}
