use std::sync::Arc;
use dioxus::prelude::*;
use vellum_core::{BookService, BulkConvertReport};

#[component]
pub fn BulkConvertModal(
    book_service: Arc<BookService>,
    target_book_ids: Vec<String>,
    on_close: EventHandler<()>,
    on_converted: EventHandler<BulkConvertReport>,
) -> Element {
    let mut target_format = use_signal(|| "epub".to_string());
    let mut is_running = use_signal(|| false);
    let mut report = use_signal(|| None::<BulkConvertReport>);

    let count = target_book_ids.len();

    let run_convert = {
        let bs = book_service.clone();
        let ids = target_book_ids.clone();
        move |_| {
            is_running.set(true);
            let fmt = target_format.read().clone();
            let bs_task = bs.clone();
            let ids_task = ids.clone();
            spawn(async move {
                let rep = tokio::task::spawn_blocking(move || {
                    bs_task.bulk_convert_books(&ids_task, &fmt)
                })
                .await
                .unwrap_or_else(|e| BulkConvertReport {
                    target_format: String::new(),
                    converted_count: 0,
                    skipped_count: 0,
                    errors: vec![format!("Erreur asynchrone : {e}")],
                });
                is_running.set(false);
                report.set(Some(rep.clone()));
                on_converted.call(rep);
            });
        }
    };

    rsx! {
        div {
            class: "drawer-backdrop",
            style: "align-items: center; justify-content: center;",
            onclick: move |_| on_close.call(()),
            div {
                class: "modal-card",
                style: "width: 480px; max-width: 94vw; padding: 24px; display: flex; flex-direction: column; gap: 16px;",
                onclick: move |e| e.stop_propagation(),

                div {
                    style: "display: flex; align-items: center; justify-content: space-between;",
                    div {
                        h2 { style: "margin: 0; font-size: 18px;", "🔄 Conversion en batch" }
                        p { style: "margin: 4px 0 0 0; font-size: 12px; color: var(--text-muted);", "{count} livre(s) sélectionné(s)" }
                    }
                    button {
                        class: "btn btn-secondary",
                        onclick: move |_| on_close.call(()),
                        "✕"
                    }
                }

                div {
                    style: "display: flex; flex-direction: column; gap: 8px;",
                    label { style: "font-size: 13px; font-weight: 600;", "Format cible" }
                    select {
                        class: "form-input",
                        value: "{target_format}",
                        onchange: move |e| target_format.set(e.value()),
                        option { value: "epub", "EPUB (Pivot universel / liseuses)" }
                        option { value: "pdf", "PDF (Document imprimable)" }
                        option { value: "cbz", "CBZ (Archive BD / Manga)" }
                        option { value: "txt", "TXT (Texte brut)" }
                    }
                }

                if let Some(rep) = report.read().as_ref() {
                    {
                        let err_count = rep.errors.len();
                        rsx! {
                            div {
                                style: "padding: 10px 12px; border-radius: 8px; background: rgba(255,255,255,0.04); border: 1px solid var(--border-subtle); font-size: 13px; display: flex; flex-direction: column; gap: 6px;",
                                div { style: "font-weight: 600;",
                                    "Résultat ({rep.target_format}) : {rep.converted_count} converti(s), {rep.skipped_count} ignoré(s), {err_count} erreur(s)"
                                }
                                for err in rep.errors.iter().take(5) {
                                    div { style: "font-size: 12px; color: #fca5a5;", "• {err}" }
                                }
                            }
                        }
                    }
                }

                div {
                    style: "display: flex; justify-content: flex-end; gap: 10px;",
                    button {
                        class: "btn btn-secondary",
                        onclick: move |_| on_close.call(()),
                        "Fermer"
                    }
                    button {
                        class: "btn btn-primary",
                        disabled: *is_running.read(),
                        onclick: run_convert,
                        if *is_running.read() { "⏳ Conversion en cours..." } else { "🚀 Lancer la conversion" }
                    }
                }
            }
        }
    }
}
