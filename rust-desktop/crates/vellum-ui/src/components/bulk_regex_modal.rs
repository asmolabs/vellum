use std::sync::Arc;
use dioxus::prelude::*;
use vellum_core::{BookService, BulkRegexReport, BulkRegexRequest, RegexTargetField};

#[component]
pub fn BulkRegexModal(
    book_service: Arc<BookService>,
    target_book_ids: Vec<String>,
    on_close: EventHandler<()>,
    on_updated: EventHandler<()>,
) -> Element {
    let mut field = use_signal(|| RegexTargetField::Title);
    let mut pattern = use_signal(String::new);
    let mut replacement = use_signal(String::new);
    let mut report = use_signal(|| None::<BulkRegexReport>);
    let mut is_running = use_signal(|| false);
    let mut error_msg = use_signal(|| None::<String>);

    let run_op = {
        let bs = book_service.clone();
        let ids = target_book_ids.clone();
        move |dry_run: bool| {
            let pat = pattern.read().clone();
            if pat.is_empty() {
                error_msg.set(Some("Veuillez saisir un motif Regex.".to_string()));
                return;
            }
            is_running.set(true);
            error_msg.set(None);
            let req = BulkRegexRequest {
                book_ids: ids.clone(),
                field: *field.read(),
                pattern: pat,
                replacement: replacement.read().clone(),
                dry_run,
            };
            let bs_task = bs.clone();
            spawn(async move {
                let res = tokio::task::spawn_blocking(move || bs_task.bulk_regex_replace(req)).await;
                is_running.set(false);
                match res {
                    Ok(Ok(rep)) => {
                        report.set(Some(rep));
                        if !dry_run {
                            on_updated.call(());
                        }
                    }
                    Ok(Err(e)) => error_msg.set(Some(format!("Erreur : {e}"))),
                    Err(e) => error_msg.set(Some(format!("Erreur asynchrone : {e}"))),
                }
            });
        }
    };

    let target_count = target_book_ids.len();

    rsx! {
        div {
            class: "drawer-backdrop",
            style: "align-items: center; justify-content: center;",
            onclick: move |_| on_close.call(()),
            div {
                class: "modal-card",
                style: "width: 680px; max-width: 94vw; max-height: 85vh; display: flex; flex-direction: column; padding: 24px; gap: 14px;",
                onclick: move |e| e.stop_propagation(),

                div {
                    style: "display: flex; align-items: center; justify-content: space-between;",
                    div {
                        h2 { style: "margin: 0; font-size: 18px;", "🔤 Recherche & Remplacement Regex en masse" }
                        p { style: "margin: 4px 0 0 0; font-size: 12px; color: var(--text-muted);", "Cible : {target_count} livre(s)" }
                    }
                    button {
                        class: "btn btn-secondary",
                        onclick: move |_| on_close.call(()),
                        "✕"
                    }
                }

                div {
                    style: "display: grid; grid-template-columns: 160px 1fr; gap: 10px; align-items: center;",
                    label { style: "font-size: 13px; font-weight: 600;", "Champ cible" }
                    select {
                        class: "form-input",
                        onchange: move |e| {
                            let next = match e.value().as_str() {
                                "author" => RegexTargetField::Author,
                                "series" => RegexTargetField::Series,
                                "publisher" => RegexTargetField::Publisher,
                                _ => RegexTargetField::Title,
                            };
                            field.set(next);
                        },
                        option { value: "title", selected: *field.read() == RegexTargetField::Title, "Titre" }
                        option { value: "author", selected: *field.read() == RegexTargetField::Author, "Auteur" }
                        option { value: "series", selected: *field.read() == RegexTargetField::Series, "Série" }
                        option { value: "publisher", selected: *field.read() == RegexTargetField::Publisher, "Éditeur" }
                    }

                    label { style: "font-size: 13px; font-weight: 600;", "Motif Regex" }
                    input {
                        class: "form-input",
                        placeholder: r"ex: \s*\[.*?\]$",
                        value: "{pattern}",
                        oninput: move |e| pattern.set(e.value()),
                    }

                    label { style: "font-size: 13px; font-weight: 600;", "Remplacement" }
                    input {
                        class: "form-input",
                        placeholder: "ex: $1 (laisser vide pour supprimer)",
                        value: "{replacement}",
                        oninput: move |e| replacement.set(e.value()),
                    }
                }

                if let Some(err) = error_msg.read().as_ref() {
                    div {
                        style: "padding: 8px 12px; border-radius: 6px; font-size: 13px; background: rgba(239,68,68,0.12); color: #fca5a5; border: 1px solid rgba(239,68,68,0.3);",
                        "{err}"
                    }
                }

                div {
                    style: "display: flex; gap: 10px; justify-content: flex-end;",
                    button {
                        class: "btn btn-secondary",
                        disabled: *is_running.read(),
                        onclick: {
                            let mut r = run_op.clone();
                            move |_| r(true)
                        },
                        "👁️ Prévisualiser (Dry-Run)"
                    }
                    button {
                        class: "btn btn-primary",
                        disabled: *is_running.read(),
                        onclick: {
                            let mut r = run_op.clone();
                            move |_| r(false)
                        },
                        "✅ Appliquer"
                    }
                }

                if let Some(rep) = report.read().as_ref() {
                    div {
                        style: "flex: 1; overflow-y: auto; border-top: 1px solid var(--border-color); padding-top: 12px; display: flex; flex-direction: column; gap: 8px;",
                        div {
                            style: "font-size: 13px; font-weight: 600;",
                            "{rep.matched_count} correspondance(s), {rep.updated_count} mise(s) à jour."
                        }
                        for ch in rep.changes.iter() {
                            div {
                                style: "padding: 8px 10px; border-radius: 6px; background: rgba(255,255,255,0.03); font-size: 12px;",
                                div { style: "color: #fca5a5;", "− {ch.before}" }
                                div { style: "color: #86efac;", "+ {ch.after}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

