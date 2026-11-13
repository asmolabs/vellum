use std::sync::Arc;
use dioxus::prelude::*;
use vellum_core::BookService;

#[derive(Clone, Copy, PartialEq, Eq)]
enum AuditTab {
    Duplicates,
    Incomplete,
    MissingFiles,
}

#[component]
pub fn LibraryAuditModal(
    book_service: Arc<BookService>,
    on_close: EventHandler<()>,
    on_updated: EventHandler<()>,
) -> Element {
    let audit_report = use_signal(|| book_service.audit_library().ok());
    let mut active_tab = use_signal(|| AuditTab::Duplicates);
    let is_processing = use_signal(|| false);
    let notification = use_signal(|| None::<(String, bool)>);
    let enriching_id = use_signal(|| None::<String>);
    let ignored_pairs = use_signal(std::collections::HashSet::<(String, String)>::new);

    let bs = book_service.clone();

    // Action : Fusionner deux livres
    let run_merge = {
        let bs = bs.clone();
        let mut is_proc = is_processing;
        let mut rep = audit_report;
        let mut notif = notification;
        let on_upd = on_updated;
        move |(target_id, source_id): (String, String)| {
            is_proc.set(true);
            notif.set(None);
            let bs_cloned = bs.clone();
            spawn(async move {
                let bs_task = bs_cloned.clone();
                let res = tokio::task::spawn_blocking(move || {
                    bs_task.merge_books(&target_id, &source_id)
                })
                .await;
                is_proc.set(false);
                match res {
                    Ok(Ok(merged)) => {
                        notif.set(Some((
                            format!("✨ Livres fusionnés avec succès ! '{}' rassemble désormais tous les formats.", merged.title),
                            true,
                        )));
                        if let Ok(new_rep) = bs_cloned.audit_library() {
                            rep.set(Some(new_rep));
                        }
                        on_upd.call(());
                    }
                    Ok(Err(e)) => {
                        notif.set(Some((format!("Erreur lors de la fusion : {e}"), false)));
                    }
                    Err(e) => {
                        notif.set(Some((format!("Erreur asynchrone : {e}"), false)));
                    }
                }
            });
        }
    };

    // Action : Enrichir via Google Books
    let run_enrich = {
        let bs = bs.clone();
        let mut rep = audit_report;
        let mut notif = notification;
        let mut enr_id = enriching_id;
        let on_upd = on_updated;
        move |book_id: String| {
            enr_id.set(Some(book_id.clone()));
            notif.set(None);
            let bs_cloned = bs.clone();
            spawn(async move {
                let bs_task = bs_cloned.clone();
                let res = tokio::task::spawn_blocking(move || {
                    bs_task.enrich_book(&book_id)
                })
                .await;
                enr_id.set(None);
                match res {
                    Ok(Ok(enriched)) => {
                        notif.set(Some((
                            format!("🪄 Fiche de '{}' enrichie avec succès via Google Books !", enriched.title),
                            true,
                        )));
                        if let Ok(new_rep) = bs_cloned.audit_library() {
                            rep.set(Some(new_rep));
                        }
                        on_upd.call(());
                    }
                    Ok(Err(e)) => {
                        notif.set(Some((format!("Erreur lors de l'enrichissement : {e}"), false)));
                    }
                    Err(e) => {
                        notif.set(Some((format!("Erreur asynchrone : {e}"), false)));
                    }
                }
            });
        }
    };

    // Action : Relancer un audit complet
    let run_refresh = {
        let bs = bs.clone();
        let mut rep = audit_report;
        let mut notif = notification;
        move |_| {
            match bs.audit_library() {
                Ok(new_rep) => {
                    rep.set(Some(new_rep));
                    notif.set(Some(("🩺 Audit de santé actualisé avec succès.".to_string(), true)));
                }
                Err(e) => {
                    notif.set(Some((format!("Erreur d'analyse : {e}"), false)));
                }
            }
        }
    };

    let report_opt = audit_report.read().clone();

    rsx! {
        div {
            class: "drawer-backdrop",
            onclick: move |_| on_close.call(()),

            div {
                class: "drawer-panel",
                style: "width: 860px; max-width: 95vw; max-height: 92vh; display: flex; flex-direction: column;",
                onclick: move |evt| evt.stop_propagation(),

                // Header
                div { class: "drawer-header", style: "border-bottom: 1px solid var(--border-color); padding-bottom: 12px; margin-bottom: 12px;",
                    div { style: "display: flex; align-items: center; gap: 10px;",
                        span { style: "font-size: 26px;", "🛡️" }
                        div {
                            h3 { style: "font-size: 17px; font-weight: 700; margin: 0;", "Santé & Audit de la Bibliothèque" }
                            p { style: "font-size: 12px; color: var(--text-secondary); margin: 0;", "Détectez les doublons, fusionnez les formats et optimisez vos métadonnées" }
                        }
                    }
                    div { style: "display: flex; align-items: center; gap: 8px;",
                        button {
                            class: "btn btn-secondary",
                            style: "font-size: 12px; padding: 4px 10px;",
                            onclick: run_refresh.clone(),
                            "🔄 Re-scanner"
                        }
                        button {
                            class: "btn btn-icon",
                            onclick: move |_| on_close.call(()),
                            "✕"
                        }
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

                if let Some(report) = report_opt {
                    // Global Score Banner
                    {
                        let score = report.overall_score;
                        let (score_color, score_bg, score_label) = if score >= 90 {
                            ("#34d399", "rgba(16, 185, 129, 0.15)", "🟢 Excellente santé")
                        } else if score >= 70 {
                            ("#fbbf24", "rgba(245, 158, 11, 0.15)", "🟡 Bonne condition générale")
                        } else {
                            ("#f87171", "rgba(239, 68, 68, 0.15)", "🔴 Optimisation requise")
                        };

                        rsx! {
                            div {
                                style: "display: flex; align-items: center; justify-content: space-between; padding: 14px 18px; border-radius: 10px; margin-bottom: 14px; background: {score_bg}; border: 1px solid {score_color}44;",
                                div { style: "display: flex; align-items: center; gap: 16px;",
                                    div {
                                        style: "font-size: 32px; font-weight: 800; color: {score_color}; line-height: 1;",
                                        "{score}%"
                                    }
                                    div {
                                        div { style: "font-size: 14px; font-weight: 700; color: {score_color};", "{score_label}" }
                                        div { style: "font-size: 12px; color: var(--text-secondary); margin-top: 2px;",
                                            "{report.total_books} livre(s) total • {report.total_formats} format(s) • {report.clean_books_count} livre(s) parfaitement complet(s)"
                                        }
                                    }
                                }
                                div { style: "display: flex; gap: 8px;",
                                    span { class: "tag", style: "background: rgba(99, 102, 241, 0.2); color: #a5b4fc; font-size: 11px;",
                                        "👯 {report.duplicate_pairs.len()} doublon(s)"
                                    }
                                    span { class: "tag", style: "background: rgba(245, 158, 11, 0.2); color: #fde68a; font-size: 11px;",
                                        "📝 {report.incomplete_issues.len()} incomplet(s)"
                                    }
                                    if !report.missing_file_issues.is_empty() {
                                        span { class: "tag", style: "background: rgba(239, 68, 68, 0.2); color: #fca5a5; font-size: 11px;",
                                            "⚠️ {report.missing_file_issues.len()} fichier(s) manquant(s)"
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Tabs
                    div { style: "display: flex; gap: 8px; border-bottom: 1px solid var(--border-color); padding-bottom: 8px; margin-bottom: 14px;",
                        button {
                            class: if *active_tab.read() == AuditTab::Duplicates { "btn btn-primary" } else { "btn btn-secondary" },
                            style: "font-size: 12px; padding: 6px 14px;",
                            onclick: move |_| active_tab.set(AuditTab::Duplicates),
                            "👯 Doublons détectés ({report.duplicate_pairs.len()})"
                        }
                        button {
                            class: if *active_tab.read() == AuditTab::Incomplete { "btn btn-primary" } else { "btn btn-secondary" },
                            style: "font-size: 12px; padding: 6px 14px;",
                            onclick: move |_| active_tab.set(AuditTab::Incomplete),
                            "📝 Fiches à enrichir ({report.incomplete_issues.len()})"
                        }
                        button {
                            class: if *active_tab.read() == AuditTab::MissingFiles { "btn btn-primary" } else { "btn btn-secondary" },
                            style: "font-size: 12px; padding: 6px 14px;",
                            onclick: move |_| active_tab.set(AuditTab::MissingFiles),
                            "📁 Fichiers & Intégrité ({report.missing_file_issues.len()})"
                        }
                    }

                    // Tab Body
                    div { style: "flex: 1; overflow-y: auto; display: flex; flex-direction: column; gap: 12px; padding-right: 4px;",
                        match *active_tab.read() {
                            AuditTab::Duplicates => rsx! {
                                if report.duplicate_pairs.is_empty() {
                                    div { style: "text-align: center; padding: 40px; color: var(--text-secondary);",
                                        "✨ Aucun doublon détecté dans votre bibliothèque !"
                                    }
                                } else {
                                    for pair in report.duplicate_pairs.iter() {
                                        {
                                            let pair_key = (pair.primary_book.id.clone(), pair.duplicate_book.id.clone());
                                            let is_ignored = ignored_pairs.read().contains(&pair_key);

                                            if !is_ignored {
                                                let p_id = pair.primary_book.id.clone();
                                                let d_id = pair.duplicate_book.id.clone();
                                                let p_title = pair.primary_book.title.clone();
                                                let d_title = pair.duplicate_book.title.clone();
                                                let p_author = pair.primary_book.author.clone();
                                                let d_author = pair.duplicate_book.author.clone();
                                                let p_fmts = pair.primary_book.formats.clone();
                                                let d_fmts = pair.duplicate_book.formats.clone();
                                                let reason = pair.reason.clone();
                                                let pct = (pair.similarity_score * 100.0).round() as u32;

                                                let mut merge_a = run_merge.clone();
                                                let mut merge_b = run_merge.clone();
                                                let mut ign = ignored_pairs;
                                                let key_c = pair_key.clone();

                                                rsx! {
                                                    div {
                                                        style: "background: var(--bg-card); border: 1px solid var(--border-color); border-radius: 10px; padding: 14px; display: flex; flex-direction: column; gap: 10px;",
                                                        // Badge similarity
                                                        div { style: "display: flex; justify-content: space-between; align-items: center;",
                                                            span {
                                                                style: "font-size: 12px; font-weight: 700; color: #a78bfa; background: rgba(167, 139, 250, 0.15); padding: 3px 10px; border-radius: 6px;",
                                                                "🔍 Similarité : {pct}% — {reason}"
                                                            }
                                                            button {
                                                                class: "btn btn-secondary",
                                                                style: "font-size: 11px; padding: 2px 8px;",
                                                                onclick: move |_| {
                                                                    let mut set = ign.read().clone();
                                                                    set.insert(key_c.clone());
                                                                    ign.set(set);
                                                                },
                                                                "Ignorer"
                                                            }
                                                        }

                                                        // Two books side by side
                                                        div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                                                            // Book A
                                                            div { style: "background: var(--bg-primary); border: 1px solid var(--border-color); border-radius: 8px; padding: 12px; display: flex; flex-direction: column; justify-content: space-between; gap: 8px;",
                                                                div {
                                                                    div { style: "font-size: 11px; font-weight: 700; color: var(--text-secondary);", "LIVRE A" }
                                                                    div { style: "font-size: 14px; font-weight: 700; color: var(--text-primary); margin-top: 2px;", "{p_title}" }
                                                                    div { style: "font-size: 12px; color: var(--text-secondary);", "par {p_author}" }
                                                                    div { style: "display: flex; gap: 4px; margin-top: 6px; flex-wrap: wrap;",
                                                                        for f in p_fmts.iter() {
                                                                            span { class: "format-badge", "{f.format.to_uppercase()}" }
                                                                        }
                                                                    }
                                                                }
                                                                button {
                                                                    class: "btn btn-primary",
                                                                    style: "font-size: 12px; padding: 6px 10px; width: 100%;",
                                                                    disabled: *is_processing.read(),
                                                                    onclick: {
                                                                        let p = p_id.clone();
                                                                        let d = d_id.clone();
                                                                        move |_| merge_a((p.clone(), d.clone()))
                                                                    },
                                                                    "⚡ Garder A (Absorber B)"
                                                                }
                                                            }

                                                            // Book B
                                                            div { style: "background: var(--bg-primary); border: 1px solid var(--border-color); border-radius: 8px; padding: 12px; display: flex; flex-direction: column; justify-content: space-between; gap: 8px;",
                                                                div {
                                                                    div { style: "font-size: 11px; font-weight: 700; color: var(--text-secondary);", "LIVRE B" }
                                                                    div { style: "font-size: 14px; font-weight: 700; color: var(--text-primary); margin-top: 2px;", "{d_title}" }
                                                                    div { style: "font-size: 12px; color: var(--text-secondary);", "par {d_author}" }
                                                                    div { style: "display: flex; gap: 4px; margin-top: 6px; flex-wrap: wrap;",
                                                                        for f in d_fmts.iter() {
                                                                            span { class: "format-badge", "{f.format.to_uppercase()}" }
                                                                        }
                                                                    }
                                                                }
                                                                button {
                                                                    class: "btn btn-secondary",
                                                                    style: "font-size: 12px; padding: 6px 10px; width: 100%;",
                                                                    disabled: *is_processing.read(),
                                                                    onclick: {
                                                                        let p = p_id.clone();
                                                                        let d = d_id.clone();
                                                                        move |_| merge_b((d.clone(), p.clone()))
                                                                    },
                                                                    "⚡ Garder B (Absorber A)"
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            } else {
                                                rsx! {}
                                            }
                                        }
                                    }
                                }
                            },

                            AuditTab::Incomplete => rsx! {
                                if report.incomplete_issues.is_empty() {
                                    div { style: "text-align: center; padding: 40px; color: var(--text-secondary);",
                                        "✨ Toutes vos fiches de livres sont parfaitement complètes !"
                                    }
                                } else {
                                    for issue in report.incomplete_issues.iter() {
                                        {
                                            let b_id = issue.book.id.clone();
                                            let title = issue.book.title.clone();
                                            let author = issue.book.author.clone();
                                            let is_enriching = enriching_id.read().as_ref() == Some(&b_id);
                                            let mut enr = run_enrich.clone();

                                            rsx! {
                                                div {
                                                    style: "background: var(--bg-card); border: 1px solid var(--border-color); border-radius: 8px; padding: 12px; display: flex; justify-content: space-between; align-items: center; gap: 12px;",
                                                    div { style: "flex: 1;",
                                                        div { style: "font-size: 14px; font-weight: 700; color: var(--text-primary);", "{title}" }
                                                        div { style: "font-size: 12px; color: var(--text-secondary); margin-bottom: 6px;", "par {author}" }
                                                        div { style: "display: flex; gap: 6px; flex-wrap: wrap;",
                                                            if issue.missing_cover {
                                                                span { style: "font-size: 11px; padding: 2px 6px; border-radius: 4px; background: rgba(239, 68, 68, 0.15); color: #f87171;", "🖼️ Sans couverture" }
                                                            }
                                                            if issue.missing_author {
                                                                span { style: "font-size: 11px; padding: 2px 6px; border-radius: 4px; background: rgba(245, 158, 11, 0.15); color: #fbbf24;", "👤 Auteur manquant" }
                                                            }
                                                            if issue.missing_description {
                                                                span { style: "font-size: 11px; padding: 2px 6px; border-radius: 4px; background: rgba(99, 102, 241, 0.15); color: #a5b4fc;", "📄 Sans résumé" }
                                                            }
                                                            if issue.missing_isbn {
                                                                span { style: "font-size: 11px; padding: 2px 6px; border-radius: 4px; background: rgba(148, 163, 184, 0.15); color: #94a3b8;", "🔢 Sans ISBN" }
                                                            }
                                                            if issue.missing_tags {
                                                                span { style: "font-size: 11px; padding: 2px 6px; border-radius: 4px; background: rgba(148, 163, 184, 0.15); color: #94a3b8;", "🏷️ Sans tags" }
                                                            }
                                                        }
                                                    }
                                                    button {
                                                        class: "btn btn-primary",
                                                        style: "font-size: 12px; padding: 6px 12px; white-space: nowrap;",
                                                        disabled: is_enriching,
                                                        onclick: {
                                                            let bid = b_id.clone();
                                                            move |_| enr(bid.clone())
                                                        },
                                                        if is_enriching {
                                                            "⏳ Recherche..."
                                                        } else {
                                                            "🪄 Enrichir (Google Books)"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            },

                            AuditTab::MissingFiles => rsx! {
                                if report.missing_file_issues.is_empty() {
                                    div { style: "text-align: center; padding: 40px; color: var(--text-secondary);",
                                        "✨ Tous les fichiers de vos formats sont présents et intègres sur le disque !"
                                    }
                                } else {
                                    div { style: "display: flex; flex-direction: column; gap: 8px;",
                                        for issue in report.missing_file_issues.iter() {
                                            div {
                                                style: "background: rgba(239, 68, 68, 0.1); border: 1px solid rgba(239, 68, 68, 0.3); border-radius: 8px; padding: 10px 14px; display: flex; justify-content: space-between; align-items: center;",
                                                div {
                                                    div { style: "font-size: 13px; font-weight: 700; color: #f87171;", "⚠️ {issue.book_title}" }
                                                    div { style: "font-size: 11px; color: var(--text-secondary); font-family: monospace; margin-top: 2px;",
                                                        "Format {issue.format.to_uppercase()} introuvable : {issue.expected_path}"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            },
                        }
                    }
                } else {
                    div { style: "text-align: center; padding: 50px; color: var(--text-secondary);",
                        "⏳ Analyse de la bibliothèque en cours..."
                    }
                }
            }
        }
    }
}
