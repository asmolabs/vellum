use dioxus::prelude::*;
use vellum_core::{translate, translate_fmt, Locale};

/// Données soumises par la modale d'édition en masse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkEditRequest {
    pub author: Option<String>,
    pub series: Option<String>,
    pub auto_index: bool,
    pub add_tags: Vec<String>,
    pub remove_tags: Vec<String>,
}

#[component]
pub fn BulkEditModal(
    locale: Locale,
    selected_count: usize,
    on_save: EventHandler<BulkEditRequest>,
    on_close: EventHandler<()>,
) -> Element {
    let mut author_input = use_signal(String::new);
    let mut series_input = use_signal(String::new);
    let mut auto_index = use_signal(|| true);
    let mut add_tags_input = use_signal(String::new);
    let mut remove_tags_input = use_signal(String::new);
    let count_str = selected_count.to_string();

    rsx! {
        div {
            class: "drawer-backdrop",
            onclick: move |_| on_close.call(()),

            div {
                class: "drawer-panel",
                style: "width: 540px;",
                onclick: move |evt| evt.stop_propagation(),

                div { class: "drawer-header",
                    div { style: "display: flex; align-items: center; gap: 8px;",
                        span { style: "font-size: 18px;", "✏️" }
                        h3 { style: "font-size: 16px; font-weight: 700;", {translate_fmt(locale, "bulk.edit_title", &[("count", &count_str)])} }
                    }
                    button {
                        class: "btn btn-icon",
                        onclick: move |_| on_close.call(()),
                        "✕"
                    }
                }

                p { style: "font-size: 12px; color: var(--text-secondary); line-height: 1.5;",
                    {translate(locale, "bulk.edit_hint")}
                }

                div { style: "display: flex; flex-direction: column; gap: 14px; margin-top: 6px;",
                    div { class: "drawer-form-group",
                        label { class: "drawer-form-label", {translate(locale, "bulk.common_author")} }
                        input {
                            class: "drawer-input",
                            placeholder: translate(locale, "bulk.author_ph"),
                            value: "{author_input}",
                            oninput: move |e| author_input.set(e.value()),
                        }
                    }

                    div { style: "display: flex; flex-direction: column; gap: 8px; padding: 12px; background: rgba(167, 139, 250, 0.05); border: 1px solid rgba(167, 139, 250, 0.2); border-radius: 8px;",
                        div { class: "drawer-form-group",
                            label { class: "drawer-form-label", {translate(locale, "bulk.assign_series")} }
                            input {
                                class: "drawer-input",
                                placeholder: translate(locale, "bulk.series_ph"),
                                value: "{series_input}",
                                oninput: move |e| series_input.set(e.value()),
                            }
                        }

                        label { style: "display: flex; align-items: center; gap: 8px; cursor: pointer; font-size: 12px; margin-top: 4px;",
                            input {
                                r#type: "checkbox",
                                checked: *auto_index.read(),
                                onchange: move |e| auto_index.set(e.value() == "true"),
                            }
                            span { style: "font-weight: 500; color: #c4b5fd;", {translate(locale, "bulk.auto_index")} }
                        }
                    }

                    div { class: "drawer-form-group",
                        label { class: "drawer-form-label", {translate(locale, "bulk.add_tags")} }
                        input {
                            class: "drawer-input",
                            placeholder: translate(locale, "bulk.add_tags_ph"),
                            value: "{add_tags_input}",
                            oninput: move |e| add_tags_input.set(e.value()),
                        }
                    }

                    div { class: "drawer-form-group",
                        label { class: "drawer-form-label", {translate(locale, "bulk.remove_tags")} }
                        input {
                            class: "drawer-input",
                            placeholder: translate(locale, "bulk.remove_tags_ph"),
                            value: "{remove_tags_input}",
                            oninput: move |e| remove_tags_input.set(e.value()),
                        }
                    }

                    div { style: "display: flex; justify-content: flex-end; gap: 10px; margin-top: 14px;",
                        button {
                            class: "btn btn-secondary",
                            onclick: move |_| on_close.call(()),
                            {translate(locale, "action.cancel")}
                        }
                        button {
                            class: "btn btn-primary",
                            onclick: move |_| {
                                let author = if author_input.read().trim().is_empty() {
                                    None
                                } else {
                                    Some(author_input.read().trim().to_string())
                                };
                                let series = if series_input.read().trim().is_empty() {
                                    None
                                } else {
                                    Some(series_input.read().trim().to_string())
                                };
                                let add_tags: Vec<String> = add_tags_input
                                    .read()
                                    .split(',')
                                    .map(|s| s.trim().to_string())
                                    .filter(|s| !s.is_empty())
                                    .collect();
                                let remove_tags: Vec<String> = remove_tags_input
                                    .read()
                                    .split(',')
                                    .map(|s| s.trim().to_string())
                                    .filter(|s| !s.is_empty())
                                    .collect();

                                on_save.call(BulkEditRequest {
                                    author,
                                    series,
                                    auto_index: *auto_index.read(),
                                    add_tags,
                                    remove_tags,
                                });
                                on_close.call(());
                            },
                            {translate_fmt(locale, "bulk.apply_count", &[("count", &count_str)])}
                        }
                    }
                }
            }
        }
    }
}
