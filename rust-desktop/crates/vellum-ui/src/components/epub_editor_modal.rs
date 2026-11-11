use std::sync::Arc;
use dioxus::prelude::*;
use vellum_core::BookService;
use vellum_domain::{Book, EpubDiagnostics, EpubIssueSeverity, EpubTocEntry};

#[component]
pub fn EpubEditorModal(
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
                let res = tokio::task::spawn_blocking(move || {
                    bs.generate_book_toc_from_headings(&b_id)
                })
                .await;

                is_op.set(false);
                match res {
                    Ok(Ok(generated)) => {
                        let count = generated.len();
                        te.set(generated);
                        notif.set(Some((
                            format!("⚡ {count} chapitres extraits avec succès des titres HTML !"),
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
                            "💾 Table des Matières enregistrée avec succès dans le fichier EPUB !".to_string(),
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
                        notif.set(Some((
                            "✨ Nettoyage et réparation terminés avec succès (archive re-packagée, DRM/junk purgés) !".to_string(),
                            true,
                        )));
                        on_updated.call(());
                    }
                    Ok(Err(e)) => {
                        notif.set(Some((
                            format!("Erreur lors de la réparation: {e}"),
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
                            h3 { style: "font-size: 16px; font-weight: 700; margin: 0;", "Inspecteur & TOC EPUB" }
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
                        "📋 Diagnostic & Santé EPUB"
                    }
                    button {
                        class: if *active_tab.read() == "toc" { "btn btn-primary" } else { "btn btn-secondary" },
                        style: "font-size: 13px; padding: 6px 14px;",
                        onclick: move |_| active_tab.set("toc".to_string()),
                        "📑 Table des Matières ({toc_entries.read().len()})"
                    }
                }

                // Body content based on tab
                div { style: "flex: 1; overflow-y: auto; padding-right: 4px; display: flex; flex-direction: column; gap: 14px;",
                    if *is_loading.read() {
                        div { style: "display: flex; align-items: center; justify-content: center; height: 180px; color: var(--text-secondary);",
                            "⏳ Analyse en cours de la structure EPUB..."
                        }
                    } else if *active_tab.read() == "diagnostics" {
                        if let Some(diag) = diagnostics.read().as_ref() {
                            // 1. Metric Cards
                            div { style: "display: grid; grid-template-columns: repeat(auto-fit, minmax(130px, 1fr)); gap: 10px;",
                                div { style: "background: var(--bg-card); padding: 10px 12px; border-radius: 8px; border: 1px solid var(--border-color);",
                                    div { style: "font-size: 11px; color: var(--text-secondary);", "Conformité" }
                                    div { style: "font-size: 14px; font-weight: 700; margin-top: 4px;",
                                        if diag.is_valid {
                                            span { style: "color: #34d399;", "🟢 Conforme" }
                                        } else {
                                            span { style: "color: #f87171;", "🔴 Erreurs" }
                                        }
                                    }
                                }

                                div { style: "background: var(--bg-card); padding: 10px 12px; border-radius: 8px; border: 1px solid var(--border-color);",
                                    div { style: "font-size: 11px; color: var(--text-secondary);", "Standard" }
                                    div { style: "font-size: 14px; font-weight: 700; margin-top: 4px; color: var(--text-primary);",
                                        "EPUB {diag.epub_version}"
                                    }
                                }

                                div { style: "background: var(--bg-card); padding: 10px 12px; border-radius: 8px; border: 1px solid var(--border-color);",
                                    div { style: "font-size: 11px; color: var(--text-secondary);", "Chapitres (Spine)" }
                                    div { style: "font-size: 14px; font-weight: 700; margin-top: 4px; color: var(--text-primary);",
                                        "{diag.spine_count}"
                                    }
                                }

                                div { style: "background: var(--bg-card); padding: 10px 12px; border-radius: 8px; border: 1px solid var(--border-color);",
                                    div { style: "font-size: 11px; color: var(--text-secondary);", "Entrées TOC" }
                                    div { style: "font-size: 14px; font-weight: 700; margin-top: 4px; color: var(--text-primary);",
                                        "{diag.toc_entries_count}"
                                    }
                                }

                                div { style: "background: var(--bg-card); padding: 10px 12px; border-radius: 8px; border: 1px solid var(--border-color);",
                                    div { style: "font-size: 11px; color: var(--text-secondary);", "Couverture" }
                                    div { style: "font-size: 14px; font-weight: 700; margin-top: 4px;",
                                        if diag.has_cover {
                                            span { style: "color: #34d399;", "✅ Déclarée" }
                                        } else {
                                            span { style: "color: #fbbf24;", "⚠️ Absente" }
                                        }
                                    }
                                }

                                div { style: "background: var(--bg-card); padding: 10px 12px; border-radius: 8px; border: 1px solid var(--border-color);",
                                    div { style: "font-size: 11px; color: var(--text-secondary);", "Poids" }
                                    div { style: "font-size: 14px; font-weight: 700; margin-top: 4px; color: var(--text-primary);",
                                        "{format_bytes(diag.total_size_bytes)}"
                                    }
                                }
                            }

                            // 2. Action to Clean and Repair
                            div { style: "display: flex; justify-content: space-between; align-items: center; padding: 12px 14px; background: rgba(59, 130, 246, 0.08); border: 1px solid rgba(59, 130, 246, 0.25); border-radius: 8px;",
                                div {
                                    div { style: "font-size: 13px; font-weight: 600; color: #60a5fa;", "✨ Réparation Automatique en 1 clic" }
                                    div { style: "font-size: 12px; color: var(--text-secondary); margin-top: 2px;",
                                        "Re-pack propre du mimetype sans compression, purge des fichiers résiduels (.DS_Store / DRM obsolètes), et régénération de la TOC."
                                    }
                                }
                                button {
                                    class: "btn btn-primary",
                                    style: "font-size: 13px; padding: 7px 16px; white-space: nowrap;",
                                    disabled: *is_operating.read(),
                                    onclick: move |_| run_repair_epub(),
                                    if *is_operating.read() { "⏳ Réparation..." } else { "🛠️ Nettoyer & Réparer" }
                                }
                            }

                            // 3. Issues List
                            div { style: "display: flex; flex-direction: column; gap: 8px; margin-top: 4px;",
                                h4 { style: "font-size: 13px; font-weight: 600; color: var(--text-secondary); margin: 0;",
                                    "Résultats de l'analyse ({diag.issues.len()} éléments relevés) :"
                                }

                                if diag.issues.is_empty() {
                                    div { style: "padding: 16px; text-align: center; color: #34d399; background: rgba(16, 185, 129, 0.08); border-radius: 8px; font-size: 13px;",
                                        "✅ Aucun problème structurel détecté. Cet EPUB est parfaitement valide."
                                    }
                                } else {
                                    for issue in &diag.issues {
                                        {
                                            let (badge_bg, badge_color, badge_text) = match issue.severity {
                                                EpubIssueSeverity::Error => ("rgba(239, 68, 68, 0.15)", "#f87171", "ERREUR"),
                                                EpubIssueSeverity::Warning => ("rgba(245, 158, 11, 0.15)", "#fbbf24", "AVERTISSEMENT"),
                                                EpubIssueSeverity::Info => ("rgba(59, 130, 246, 0.15)", "#60a5fa", "INFO"),
                                            };
                                            let file_desc = issue.file.as_deref().unwrap_or("");
                                            rsx! {
                                                div { style: "display: flex; gap: 10px; align-items: flex-start; padding: 10px 12px; background: var(--bg-card); border: 1px solid var(--border-color); border-radius: 6px;",
                                                    span {
                                                        style: "font-size: 10px; font-weight: 700; padding: 3px 6px; border-radius: 4px; background: {badge_bg}; color: {badge_color};",
                                                        "{badge_text}"
                                                    }
                                                    div { style: "flex: 1;",
                                                        div { style: "font-size: 13px; color: var(--text-primary); font-weight: 500;",
                                                            "{issue.message}"
                                                        }
                                                        if !file_desc.is_empty() {
                                                            div { style: "font-size: 11px; font-family: monospace; color: var(--text-secondary); margin-top: 2px;",
                                                                "Fichier: {file_desc}"
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
                    } else if *active_tab.read() == "toc" {
                        // Tab 2: TOC Editor
                        div { style: "display: flex; justify-content: space-between; align-items: center; padding-bottom: 8px; border-bottom: 1px solid var(--border-color);",
                            div { style: "display: flex; gap: 8px;",
                                button {
                                    class: "btn btn-secondary",
                                    style: "font-size: 12px; padding: 6px 12px; font-weight: 600;",
                                    disabled: *is_operating.read(),
                                    onclick: {
                                        let r = run_auto_generate_toc.clone();
                                        move |_| r()
                                    },
                                    "⚡ Auto-détecter depuis les <h1> / <h2>"
                                }
                                button {
                                    class: "btn btn-secondary",
                                    style: "font-size: 12px; padding: 6px 12px;",
                                    disabled: *is_operating.read(),
                                    onclick: move |_| {
                                        let mut te = toc_entries;
                                        let mut list = te.read().clone();
                                        let next_idx = list.len() + 1;
                                        list.push(EpubTocEntry {
                                            id: format!("navpoint-{next_idx}"),
                                            label: format!("Nouveau Chapitre {next_idx}"),
                                            href: "ch1.xhtml".to_string(),
                                            play_order: next_idx,
                                            children: Vec::new(),
                                        });
                                        te.set(list);
                                    },
                                    "➕ Ajouter"
                                }
                            }

                            button {
                                class: "btn btn-primary",
                                style: "font-size: 12px; padding: 6px 16px; font-weight: 600;",
                                disabled: *is_operating.read(),
                                onclick: move |_| run_save_toc(),
                                if *is_operating.read() { "⏳ Enregistrement..." } else { "💾 Enregistrer la TOC" }
                            }
                        }

                        // Chapters list
                        if toc_entries.read().is_empty() {
                            div { style: "text-align: center; padding: 32px 16px; color: var(--text-secondary);",
                                p { style: "font-size: 14px; margin-bottom: 12px;", "Aucune entrée dans la table des matières." }
                                button {
                                    class: "btn btn-primary",
                                    style: "font-size: 13px;",
                                    onclick: {
                                        let r = run_auto_generate_toc.clone();
                                        move |_| r()
                                    },
                                    "⚡ Générer automatiquement à partir des titres HTML"
                                }
                            }
                        } else {
                            div { style: "display: flex; flex-direction: column; gap: 8px;",
                                for (idx, entry) in toc_entries.read().iter().enumerate() {
                                    {
                                        let item_idx = idx;
                                        let item_label = entry.label.clone();
                                        let item_href = entry.href.clone();
                                        let can_up = item_idx > 0;
                                        let can_down = item_idx + 1 < toc_entries.read().len();

                                        rsx! {
                                            div {
                                                key: "{item_idx}",
                                                style: "display: flex; align-items: center; gap: 8px; padding: 8px 10px; background: var(--bg-card); border: 1px solid var(--border-color); border-radius: 6px;",

                                                // Index badge
                                                span { style: "font-size: 11px; font-weight: 700; color: var(--text-secondary); width: 26px; text-align: center;",
                                                    "#{item_idx + 1}"
                                                }

                                                // Title Input
                                                input {
                                                    class: "drawer-input",
                                                    style: "flex: 1; padding: 6px 10px; font-size: 13px;",
                                                    value: "{item_label}",
                                                    oninput: move |e| {
                                                        let mut te = toc_entries;
                                                        let mut list = te.read().clone();
                                                        if item_idx < list.len() {
                                                            list[item_idx].label = e.value();
                                                            te.set(list);
                                                        }
                                                    },
                                                }

                                                // Target Href chip
                                                span {
                                                    style: "font-size: 11px; font-family: monospace; color: var(--text-secondary); background: rgba(255, 255, 255, 0.05); padding: 4px 8px; border-radius: 4px; max-width: 180px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;",
                                                    title: "{item_href}",
                                                    "{item_href}"
                                                }

                                                // Reorder buttons
                                                button {
                                                    class: "btn btn-icon",
                                                    style: "width: 28px; height: 28px; font-size: 10px;",
                                                    disabled: !can_up,
                                                    onclick: move |_| {
                                                        let mut te = toc_entries;
                                                        let mut list = te.read().clone();
                                                        if item_idx > 0 && item_idx < list.len() {
                                                            list.swap(item_idx, item_idx - 1);
                                                            for (i, item) in list.iter_mut().enumerate() {
                                                                item.play_order = i + 1;
                                                            }
                                                            te.set(list);
                                                        }
                                                    },
                                                    "▲"
                                                }

                                                button {
                                                    class: "btn btn-icon",
                                                    style: "width: 28px; height: 28px; font-size: 10px;",
                                                    disabled: !can_down,
                                                    onclick: move |_| {
                                                        let mut te = toc_entries;
                                                        let mut list = te.read().clone();
                                                        if item_idx + 1 < list.len() {
                                                            list.swap(item_idx, item_idx + 1);
                                                            for (i, item) in list.iter_mut().enumerate() {
                                                                item.play_order = i + 1;
                                                            }
                                                            te.set(list);
                                                        }
                                                    },
                                                    "▼"
                                                }

                                                // Delete button
                                                button {
                                                    class: "btn btn-icon",
                                                    style: "width: 28px; height: 28px; font-size: 12px; color: #f87171;",
                                                    onclick: move |_| {
                                                        let mut te = toc_entries;
                                                        let mut list = te.read().clone();
                                                        if item_idx < list.len() {
                                                            list.remove(item_idx);
                                                            for (i, item) in list.iter_mut().enumerate() {
                                                                item.play_order = i + 1;
                                                            }
                                                            te.set(list);
                                                        }
                                                    },
                                                    "🗑️"
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
        }
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} Ko", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} Mo", bytes as f64 / (1024.0 * 1024.0))
    }
}
