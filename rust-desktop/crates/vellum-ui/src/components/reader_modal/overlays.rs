use crate::state::ReaderSession;
use dioxus::prelude::*;

#[component]
pub(super) fn ReaderSelectionOverlays(
    mut session: Signal<Option<ReaderSession>>,
    mut selected_text: Signal<Option<String>>,
    mut note_mode: Signal<bool>,
    mut note_input: Signal<String>,
    mut ai_modal_open: Signal<bool>,
    mut ai_query: Signal<String>,
    mut ai_result: Signal<Option<String>>,
    mut ai_loading: Signal<bool>,
    mut toast_msg: Signal<Option<String>>,
) -> Element {
    rsx! {
        if let Some(ref text) = *selected_text.read() {
            {
                let txt = text.clone();
                let color_choices = [
                    ("yellow", "#facc15", "Jaune"),
                    ("green", "#4ade80", "Vert"),
                    ("blue", "#60a5fa", "Bleu"),
                    ("pink", "#f472b6", "Rose"),
                ];
                rsx! {
                    div { class: "reader-selection-bar",
                        span { style: "font-size: 11px; color: var(--text-muted); margin-right: 4px;", "Surligner :" }
                        for (color_name, hex, label) in color_choices {
                            {
                                let t = txt.clone();
                                rsx! {
                                    div {
                                        class: "color-dot",
                                        style: "background-color: {hex};",
                                        title: "{label}",
                                        onclick: move |_| {
                                            let mut mut_sess = session.write();
                                            if let Some(s) = mut_sess.as_mut() {
                                                s.add_highlight(color_name, &t, None);
                                            }
                                            selected_text.set(None);
                                        }
                                    }
                                }
                            }
                        }

                        div { style: "width: 1px; height: 18px; background: rgba(255,255,255,0.15); margin: 0 4px;" }

                        button {
                            class: "btn btn-icon",
                            style: "font-size: 11px; padding: 4px 8px; border-radius: 12px;",
                            title: "Ajouter une note",
                            onclick: move |_| note_mode.set(true),
                            "📝 Note"
                        }

                        button {
                            class: "btn btn-icon",
                            style: "font-size: 11px; padding: 4px 8px; border-radius: 12px; color: #a855f7;",
                            title: "Définition ou traduction IA",
                            onclick: {
                                let t = txt.clone();
                                move |_| {
                                    ai_query.set(t.clone());
                                    ai_result.set(None);
                                    ai_loading.set(true);
                                    ai_modal_open.set(true);
                                    selected_text.set(None);

                                    let prompt_text = t.clone();
                                    spawn(async move {
                                        let res = tokio::task::spawn_blocking(move || {
                                            vellum_core::define_passage_with_ai(&prompt_text)
                                        }).await;

                                        ai_loading.set(false);
                                        match res {
                                            Ok(Ok(content)) => ai_result.set(Some(content)),
                                            Ok(Err(e)) => ai_result.set(Some(format!(
                                                "Erreur lors de la requête IA : {e}\n\nVérifiez qu'Ollama est lancé sur votre machine (`ollama run qwen2.5:14b`)."
                                            ))),
                                            Err(e) => ai_result.set(Some(format!("Tâche interrompue : {e}"))),
                                        }
                                    });
                                }
                            },
                            "✨ Définir / IA"
                        }

                        button {
                            class: "btn btn-icon",
                            style: "padding: 2px 6px; font-size: 11px; margin-left: 4px;",
                            onclick: move |_| selected_text.set(None),
                            "✕"
                        }
                    }
                }
            }
        }

        if *note_mode.read() {
            if let Some(ref txt) = *selected_text.read() {
                {
                    let text_val = txt.clone();
                    rsx! {
                        div {
                            class: "reader-ai-modal",
                            onclick: move |_| note_mode.set(false),
                            div {
                                class: "reader-ai-card",
                                onclick: move |e| e.stop_propagation(),
                                h3 { style: "font-size: 16px; font-weight: 600;", "Ajouter une note de lecture" }
                                blockquote { style: "font-size: 12px; color: var(--text-secondary); border-left: 2px solid var(--accent-primary); padding-left: 10px; margin: 4px 0;",
                                    "{text_val}"
                                }
                                textarea {
                                    class: "drawer-textarea",
                                    rows: 4,
                                    placeholder: "Écrivez votre commentaire ou réflexion ici...",
                                    value: "{note_input}",
                                    oninput: move |e| note_input.set(e.value()),
                                }
                                div { style: "display: flex; justify-content: flex-end; gap: 8px; margin-top: 10px;",
                                    button {
                                        class: "btn btn-secondary",
                                        onclick: move |_| note_mode.set(false),
                                        "Annuler"
                                    }
                                    button {
                                        class: "btn btn-primary",
                                        onclick: {
                                            let t = text_val.clone();
                                            move |_| {
                                                let note_str = note_input.read().trim().to_string();
                                                let opt_note = if note_str.is_empty() { None } else { Some(note_str) };
                                                let mut mut_sess = session.write();
                                                if let Some(s) = mut_sess.as_mut() {
                                                    s.add_highlight("yellow", &t, opt_note);
                                                }
                                                note_input.set(String::new());
                                                note_mode.set(false);
                                                selected_text.set(None);
                                            }
                                        },
                                        "Enregistrer la note"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if *ai_modal_open.read() {
            div {
                class: "reader-ai-modal",
                onclick: move |_| ai_modal_open.set(false),
                div {
                    class: "reader-ai-card",
                    onclick: move |e| e.stop_propagation(),
                    div { style: "display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid var(--border-subtle); padding-bottom: 12px;",
                        div { style: "display: flex; align-items: center; gap: 8px;",
                            span { style: "font-size: 20px;", "✨" }
                            span { style: "font-weight: 600; font-size: 15px;", "Explication & Définition IA" }
                        }
                        button {
                            class: "btn btn-icon",
                            onclick: move |_| ai_modal_open.set(false),
                            "✕"
                        }
                    }

                    div { style: "font-size: 13px; font-weight: 600; color: var(--accent-primary); background: rgba(99,102,241,0.08); padding: 8px 12px; border-radius: 6px;",
                        "« {ai_query} »"
                    }

                    div { style: "min-height: 90px; max-height: 300px; overflow-y: auto; font-size: 13px; line-height: 1.6; color: var(--text-primary);",
                        if *ai_loading.read() {
                            div { style: "display: flex; align-items: center; gap: 10px; color: var(--text-muted); padding: 20px 0;",
                                span { style: "animation: pulse 1s infinite;", "Recherche en cours avec Ollama..." }
                            }
                        } else if let Some(ref res) = *ai_result.read() {
                            div { style: "white-space: pre-wrap;", "{res}" }
                        }
                    }

                    div { style: "display: flex; justify-content: flex-end; gap: 8px; border-top: 1px solid var(--border-subtle); padding-top: 12px;",
                        button {
                            class: "btn btn-secondary",
                            onclick: move |_| ai_modal_open.set(false),
                            "Fermer"
                        }
                        if let Some(ref res) = *ai_result.read() {
                            {
                                let note_text = res.clone();
                                let query_text = ai_query.read().clone();
                                rsx! {
                                    button {
                                        class: "btn btn-primary",
                                        onclick: move |_| {
                                            let mut mut_sess = session.write();
                                            if let Some(s) = mut_sess.as_mut() {
                                                s.add_highlight("blue", &query_text, Some(note_text.clone()));
                                            }
                                            ai_modal_open.set(false);
                                            toast_msg.set(Some("Enregistré comme note de lecture !".to_string()));
                                        },
                                        "📌 Garder comme note"
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
