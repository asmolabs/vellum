use dioxus::prelude::*;

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
    selected_count: usize,
    on_save: EventHandler<BulkEditRequest>,
    on_close: EventHandler<()>,
) -> Element {
    let mut author_input = use_signal(String::new);
    let mut series_input = use_signal(String::new);
    let mut auto_index = use_signal(|| true);
    let mut add_tags_input = use_signal(String::new);
    let mut remove_tags_input = use_signal(String::new);

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
                        h3 { style: "font-size: 16px; font-weight: 700;", "Modification en masse ({selected_count} livres)" }
                    }
                    button {
                        class: "btn btn-icon",
                        onclick: move |_| on_close.call(()),
                        "✕"
                    }
                }

                p { style: "font-size: 12px; color: var(--text-secondary); line-height: 1.5;",
                    "Les champs laissés vides ne modifieront pas les valeurs existantes de vos livres."
                }

                div { style: "display: flex; flex-direction: column; gap: 14px; margin-top: 6px;",
                    div { class: "drawer-form-group",
                        label { class: "drawer-form-label", "Définir un auteur commun :" }
                        input {
                            class: "drawer-input",
                            placeholder: "Laisser vide pour ne pas modifier l'auteur...",
                            value: "{author_input}",
                            oninput: move |e| author_input.set(e.value()),
                        }
                    }

                    div { style: "display: flex; flex-direction: column; gap: 8px; padding: 12px; background: rgba(167, 139, 250, 0.05); border: 1px solid rgba(167, 139, 250, 0.2); border-radius: 8px;",
                        div { class: "drawer-form-group",
                            label { class: "drawer-form-label", "Assigner à une série / saga :" }
                            input {
                                class: "drawer-input",
                                placeholder: "Ex: Le Cycle de Fondation, Dune...",
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
                            span { style: "font-weight: 500; color: #c4b5fd;", "Numéroter automatiquement les tomes (T.1, T.2, T.3...)" }
                        }
                    }

                    div { class: "drawer-form-group",
                        label { class: "drawer-form-label", "Ajouter ces étiquettes (séparées par virgule) :" }
                        input {
                            class: "drawer-input",
                            placeholder: "Ex: Science-Fiction, Roman, Coup de cœur...",
                            value: "{add_tags_input}",
                            oninput: move |e| add_tags_input.set(e.value()),
                        }
                    }

                    div { class: "drawer-form-group",
                        label { class: "drawer-form-label", "Retirer ces étiquettes (séparées par virgule) :" }
                        input {
                            class: "drawer-input",
                            placeholder: "Ex: À trier, Brouillon...",
                            value: "{remove_tags_input}",
                            oninput: move |e| remove_tags_input.set(e.value()),
                        }
                    }

                    div { style: "display: flex; justify-content: flex-end; gap: 10px; margin-top: 14px;",
                        button {
                            class: "btn btn-secondary",
                            onclick: move |_| on_close.call(()),
                            "Annuler"
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
                            "💾 Appliquer aux {selected_count} livre(s)"
                        }
                    }
                }
            }
        }
    }
}
