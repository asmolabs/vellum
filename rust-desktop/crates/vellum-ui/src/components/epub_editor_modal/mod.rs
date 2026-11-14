//! Modale d'inspection, de réparation et d'édition de TOC EPUB.

mod lint_tab;
mod toc_tab;

use std::sync::Arc;

use dioxus::prelude::*;
use vellum_core::{translate, translate_fmt, BookService, Locale};
use vellum_domain::{Book, EpubDiagnostics, EpubTocEntry};

use lint_tab::EpubLintTab;
use toc_tab::EpubTocTab;

#[component]
pub fn EpubEditorModal(
    locale: Locale,
    book: Book,
    book_service: Arc<BookService>,
    on_close: EventHandler<()>,
    on_updated: EventHandler<()>,
) -> Element {
    let mut active_tab = use_signal(|| "diagnostics".to_string());
    let diagnostics = use_signal(|| None::<EpubDiagnostics>);
    let toc_entries = use_signal(Vec::<EpubTocEntry>::new);
    let is_loading = use_signal(|| true);
    let is_operating = use_signal(|| false);
    let notification = use_signal(|| None::<(String, bool)>); // (message, is_success)

    let book_id = book.id.clone();
    let bs = book_service.clone();

    // Initial load of diagnostics and TOC
    use_effect({
        let b_id = book_id.clone();
        let bs_cloned = bs.clone();
        let mut load_sig = is_loading;
        let mut diag_sig = diagnostics;
        let mut toc_sig = toc_entries;
        move || {
            let b_id = b_id.clone();
            let bs = bs_cloned.clone();
            spawn(async move {
                load_sig.set(true);
                let (diag_res, toc_res) = tokio::task::spawn_blocking(move || {
                    let d = bs.inspect_book_epub(&b_id).ok();
                    let t = bs.get_book_toc(&b_id).unwrap_or_default();
                    (d, t)
                })
                .await
                .unwrap_or((None, Vec::new()));

                diag_sig.set(diag_res);
                toc_sig.set(toc_res);
                load_sig.set(false);
            });
        }
    });

    // Action: Auto-detect from headings
    let run_auto_generate_toc = {
        let b_id = book_id.clone();
        let bs = bs.clone();
        move || {
            let b_id = b_id.clone();
            let bs = bs.clone();
            let mut is_op = is_operating;
            let mut notif = notification;
            let mut te = toc_entries;
            is_op.set(true);
            notif.set(None);
            spawn(async move {
                let res =
                    tokio::task::spawn_blocking(move || bs.generate_book_toc_from_headings(&b_id))
                        .await;

                is_op.set(false);
                match res {
                    Ok(Ok(generated)) => {
                        let count = generated.len().to_string();
                        te.set(generated);
                        notif.set(Some((
                            translate_fmt(locale, "epub.toc_extracted", &[("count", &count)]),
                            true,
                        )));
                    }
                    Ok(Err(e)) => {
                        notif.set(Some((
                            format!("Erreur lors de la détection des titres: {e}"),
                            false,
                        )));
                    }
                    Err(e) => {
                        notif.set(Some((format!("Tâche interrompue: {e}"), false)));
                    }
                }
            });
        }
    };

    // Action: Save TOC
    let run_save_toc = {
        let b_id = book_id.clone();
        let bs = bs.clone();
        move || {
            let b_id = b_id.clone();
            let bs = bs.clone();
            let mut is_op = is_operating;
            let mut notif = notification;
            let mut diag_sig = diagnostics;
            let current_toc = toc_entries.read().clone();
            is_op.set(true);
            notif.set(None);
            spawn(async move {
                let res = tokio::task::spawn_blocking(move || {
                    bs.update_book_toc(&b_id, &current_toc)?;
                    bs.inspect_book_epub(&b_id)
                })
                .await;

                is_op.set(false);
                match res {
                    Ok(Ok(updated_diag)) => {
                        diag_sig.set(Some(updated_diag));
                        notif.set(Some((
                            translate(locale, "epub.toc_saved").to_string(),
                            true,
                        )));
                        on_updated.call(());
                    }
                    Ok(Err(e)) => {
                        notif.set(Some((
                            format!("Erreur d'enregistrement de la TOC: {e}"),
                            false,
                        )));
                    }
                    Err(e) => {
                        notif.set(Some((format!("Tâche interrompue: {e}"), false)));
                    }
                }
            });
        }
    };

    // Action: Repair and Clean EPUB
    let run_repair_epub = {
        let b_id = book_id.clone();
        let bs = bs.clone();
        move || {
            let b_id = b_id.clone();
            let bs = bs.clone();
            let mut is_op = is_operating;
            let mut notif = notification;
            let mut diag_sig = diagnostics;
            let mut te = toc_entries;
            is_op.set(true);
            notif.set(None);
            spawn(async move {
                let res = tokio::task::spawn_blocking(move || {
                    let d = bs.repair_book_epub(&b_id)?;
                    let t = bs.get_book_toc(&b_id).unwrap_or_default();
                    Ok::<(EpubDiagnostics, Vec<EpubTocEntry>), vellum_domain::VellumError>((d, t))
                })
                .await;

                is_op.set(false);
                match res {
                    Ok(Ok((new_diag, new_toc))) => {
                        diag_sig.set(Some(new_diag));
                        te.set(new_toc);
                        notif.set(Some((translate(locale, "epub.repaired").to_string(), true)));
                        on_updated.call(());
                    }
                    Ok(Err(e)) => {
                        notif.set(Some((format!("Erreur lors de la réparation: {e}"), false)));
                    }
                    Err(e) => {
                        notif.set(Some((format!("Tâche interrompue: {e}"), false)));
                    }
                }
            });
        }
    };

    rsx! {
        div {
            class: "drawer-backdrop",
            onclick: move |_| on_close.call(()),

            div {
                class: "drawer-panel",
                style: "width: 780px; max-width: 95vw; max-height: 90vh; display: flex; flex-direction: column;",
                onclick: move |evt| evt.stop_propagation(),

                // Header
                div { class: "drawer-header", style: "border-bottom: 1px solid var(--border-color); padding-bottom: 12px; margin-bottom: 12px;",
                    div { style: "display: flex; align-items: center; gap: 10px;",
                        span { style: "font-size: 22px;", "🛠️" }
                        div {
                            h3 { style: "font-size: 16px; font-weight: 700; margin: 0;", {translate(locale, "epub.title")} }
                            p { style: "font-size: 12px; color: var(--text-secondary); margin: 0;", "{book.title} — {book.author}" }
                        }
                    }
                    button {
                        class: "btn btn-icon",
                        onclick: move |_| on_close.call(()),
                        "✕"
                    }
                }

                // Notification Banner
                if let Some((msg, is_success)) = notification.read().as_ref() {
                    div {
                        style: if *is_success {
                            "padding: 10px 14px; border-radius: 6px; font-size: 13px; margin-bottom: 12px; background: rgba(16, 185, 129, 0.15); color: #34d399; border: 1px solid rgba(16, 185, 129, 0.3);"
                        } else {
                            "padding: 10px 14px; border-radius: 6px; font-size: 13px; margin-bottom: 12px; background: rgba(239, 68, 68, 0.15); color: #f87171; border: 1px solid rgba(239, 68, 68, 0.3);"
                        },
                        "{msg}"
                    }
                }

                // Tab Switcher
                div { style: "display: flex; gap: 8px; margin-bottom: 14px; border-bottom: 1px solid var(--border-color); padding-bottom: 8px;",
                    button {
                        class: if *active_tab.read() == "diagnostics" { "btn btn-primary" } else { "btn btn-secondary" },
                        style: "font-size: 13px; padding: 6px 14px;",
                        onclick: move |_| active_tab.set("diagnostics".to_string()),
                        {translate(locale, "epub.tab_diag")}
                    }
                    button {
                        class: if *active_tab.read() == "toc" { "btn btn-primary" } else { "btn btn-secondary" },
                        style: "font-size: 13px; padding: 6px 14px;",
                        onclick: move |_| active_tab.set("toc".to_string()),
                        {
                            let toc_len = toc_entries.read().len().to_string();
                            translate_fmt(locale, "epub.tab_toc", &[("count", &toc_len)])
                        }
                    }
                }

                // Body content based on tab
                div { style: "flex: 1; overflow-y: auto; padding-right: 4px; display: flex; flex-direction: column; gap: 14px;",
                    if *is_loading.read() {
                        div { style: "display: flex; align-items: center; justify-content: center; height: 180px; color: var(--text-secondary);",
                            {translate(locale, "epub.analyzing")}
                        }
                    } else if *active_tab.read() == "diagnostics" {
                        EpubLintTab {
                            locale,
                            diagnostics,
                            is_operating,
                            on_repair: move |_| run_repair_epub(),
                        }
                    } else if *active_tab.read() == "toc" {
                        EpubTocTab {
                            locale,
                            toc_entries,
                            is_operating,
                            on_auto_generate_toc: {
                                let r = run_auto_generate_toc.clone();
                                move |_| r()
                            },
                            on_save_toc: move |_| run_save_toc(),
                        }
                    }
                }
            }
        }
    }
}
