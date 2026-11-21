use dioxus::prelude::*;
use std::collections::HashSet;
use std::sync::Arc;
use vellum_core::{
    translate, translate_fmt, BookService, BracketSeriesCandidate, BracketSeriesScanReport, Locale,
};

#[component]
pub fn BracketSeriesModal(
    locale: Locale,
    book_service: Arc<BookService>,
    on_close: EventHandler<()>,
    on_updated: EventHandler<()>,
) -> Element {
    let mut is_scanning = use_signal(|| false);
    let mut is_applying = use_signal(|| false);
    let mut scan_report = use_signal(|| None::<BracketSeriesScanReport>);
    let mut selected_ids = use_signal(HashSet::<String>::new);
    let mut success_count = use_signal(|| None::<usize>);
    let mut error_msg = use_signal(|| None::<String>);

    // Scan automatique au montage
    use_effect({
        let bs = book_service.clone();
        move || {
            is_scanning.set(true);
            error_msg.set(None);
            let bs_task = bs.clone();
            spawn(async move {
                let res = tokio::task::spawn_blocking(move || bs_task.scan_bracket_series()).await;
                is_scanning.set(false);
                match res {
                    Ok(Ok(rep)) => {
                        let mut all_ids = HashSet::new();
                        for c in &rep.candidates {
                            all_ids.insert(c.book_id.clone());
                        }
                        selected_ids.set(all_ids);
                        scan_report.set(Some(rep));
                    }
                    Ok(Err(e)) => {
                        error_msg.set(Some(translate_fmt(
                            locale,
                            "common.error",
                            &[("error", &e.to_string())],
                        )));
                    }
                    Err(e) => {
                        error_msg.set(Some(translate_fmt(
                            locale,
                            "common.async_error",
                            &[("error", &e.to_string())],
                        )));
                    }
                }
            });
        }
    });

    let apply_action = {
        let bs = book_service.clone();
        move |_| {
            let Some(ref rep) = *scan_report.read() else {
                return;
            };
            let current_sel = selected_ids.read().clone();
            let to_apply: Vec<BracketSeriesCandidate> = rep
                .candidates
                .iter()
                .filter(|c| current_sel.contains(&c.book_id))
                .cloned()
                .collect();

            if to_apply.is_empty() {
                return;
            }

            is_applying.set(true);
            error_msg.set(None);
            let bs_task = bs.clone();
            spawn(async move {
                let count_to_apply = to_apply.len();
                let res = tokio::task::spawn_blocking(move || {
                    bs_task.apply_bracket_series(&to_apply)
                })
                .await;
                is_applying.set(false);
                match res {
                    Ok(Ok(_updated)) => {
                        success_count.set(Some(count_to_apply));
                        on_updated.call(());
                    }
                    Ok(Err(e)) => {
                        error_msg.set(Some(translate_fmt(
                            locale,
                            "common.error",
                            &[("error", &e.to_string())],
                        )));
                    }
                    Err(e) => {
                        error_msg.set(Some(translate_fmt(
                            locale,
                            "common.async_error",
                            &[("error", &e.to_string())],
                        )));
                    }
                }
            });
        }
    };

    let report_opt = scan_report.read();
    let sel_len = selected_ids.read().len();

    rsx! {
        div { class: "modal-backdrop",
            div {
                class: "modal-content",
                style: "max-width: 860px; max-height: 85vh; display: flex; flex-direction: column; gap: 16px; overflow: hidden; padding: 24px;",

                // Header
                div { style: "display: flex; justify-content: space-between; align-items: flex-start;",
                    div {
                        h2 { style: "margin: 0; font-size: 19px; font-weight: 700; color: var(--text-primary);",
                            "📚 {translate(locale, \"tools.bracket_series\")}"
                        }
                        p { style: "margin: 4px 0 0; font-size: 13px; color: var(--text-muted); line-height: 1.4;",
                            "{translate(locale, \"tools.bracket_series_desc\")}"
                        }
                    }
                    button {
                        class: "btn btn-icon",
                        onclick: move |_| on_close.call(()),
                        "✕"
                    }
                }

                // Error message
                if let Some(ref err) = *error_msg.read() {
                    div { class: "alert alert-error", style: "font-size: 13px;", "{err}" }
                }

                // Success message
                if let Some(count) = *success_count.read() {
                    div { class: "alert alert-success", style: "font-size: 13px; display: flex; justify-content: space-between; align-items: center;",
                        span {
                            "✅ {translate_fmt(locale, \"tools.bracket_series_success\", &[(\"count\", &count.to_string())])}"
                        }
                        button {
                            class: "btn btn-primary",
                            style: "font-size: 12px; padding: 4px 12px;",
                            onclick: move |_| on_close.call(()),
                            "{translate(locale, \"action.close\")}"
                        }
                    }
                }

                // Loading scan state
                if *is_scanning.read() {
                    div { style: "display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 12px; padding: 40px 0; color: var(--text-muted);",
                        span { style: "font-size: 28px; animation: spin 1s linear infinite;", "⏳" }
                        span { style: "font-size: 14px; font-weight: 500;", "{translate(locale, \"tools.bracket_series_scanning\")}" }
                    }
                } else if let Some(ref rep) = *report_opt {
                    if rep.candidates.is_empty() {
                        div { style: "padding: 30px; text-align: center; color: var(--text-muted); font-size: 14px;",
                            "{translate(locale, \"tools.bracket_series_no_match\")}"
                        }
                    } else if success_count.read().is_none() {
                        // Toolbar / Selection controls
                        div { style: "display: flex; justify-content: space-between; align-items: center; background: var(--bg-card); padding: 10px 14px; border-radius: 8px; border: 1px solid var(--border-subtle);",
                            div { style: "display: flex; align-items: center; gap: 12px; font-size: 13px;",
                                span { style: "font-weight: 600; color: var(--text-primary);",
                                    "{rep.matched_count} livres détectés"
                                }
                                span { style: "color: var(--text-muted);",
                                    "({sel_len} sélectionnés)"
                                }
                            }
                            div { style: "display: flex; gap: 8px;",
                                button {
                                    class: "btn btn-secondary",
                                    style: "font-size: 11px; padding: 4px 8px;",
                                    onclick: {
                                        let all_ids = rep.candidates.iter().map(|c| c.book_id.clone()).collect::<HashSet<_>>();
                                        move |_| selected_ids.set(all_ids.clone())
                                    },
                                    "{translate(locale, \"tools.bracket_series_select_all\")}"
                                }
                                button {
                                    class: "btn btn-secondary",
                                    style: "font-size: 11px; padding: 4px 8px;",
                                    onclick: move |_| selected_ids.set(HashSet::new()),
                                    "{translate(locale, \"tools.bracket_series_deselect_all\")}"
                                }
                            }
                        }

                        // Preview Table
                        div {
                            style: "flex: 1; overflow-y: auto; border: 1px solid var(--border-subtle); border-radius: 8px; max-height: 48vh;",
                            table { class: "books-table", style: "width: 100%; border-collapse: collapse; font-size: 12px;",
                                thead {
                                    tr { style: "position: sticky; top: 0; background: var(--bg-surface); z-index: 2;",
                                        th { style: "width: 38px; text-align: center;", "" }
                                        th { "{translate(locale, \"tools.bracket_series_col_title\")}" }
                                        th { "{translate(locale, \"tools.bracket_series_col_series\")}" }
                                        th { style: "width: 70px; text-align: center;", "{translate(locale, \"tools.bracket_series_col_index\")}" }
                                        th { style: "width: 80px; text-align: center;", "{translate(locale, \"tools.bracket_series_col_lang\")}" }
                                    }
                                }
                                tbody {
                                    for c in rep.candidates.iter().take(400) {
                                        {
                                            let b_id = c.book_id.clone();
                                            let is_checked = selected_ids.read().contains(&b_id);
                                            let idx_str = c.series_index.map(|i| if i.fract() == 0.0 { format!("{}", i as i64) } else { format!("{:.1}", i) }).unwrap_or_else(|| "—".to_string());
                                            let lang_str = c.language.as_deref().unwrap_or("—");
                                            rsx! {
                                                tr {
                                                    key: "{c.book_id}",
                                                    style: "cursor: pointer;",
                                                    onclick: {
                                                        let b_id_c = b_id.clone();
                                                        move |_| {
                                                            let mut set = selected_ids.read().clone();
                                                            if set.contains(&b_id_c) {
                                                                set.remove(&b_id_c);
                                                            } else {
                                                                set.insert(b_id_c.clone());
                                                            }
                                                            selected_ids.set(set);
                                                        }
                                                    },
                                                    td { style: "text-align: center;",
                                                        input {
                                                            r#type: "checkbox",
                                                            checked: is_checked,
                                                            onclick: move |e| e.stop_propagation(),
                                                            onchange: {
                                                                let b_id_c = b_id.clone();
                                                                move |e: Event<FormData>| {
                                                                    let mut set = selected_ids.read().clone();
                                                                    if e.checked() {
                                                                        set.insert(b_id_c.clone());
                                                                    } else {
                                                                        set.remove(&b_id_c);
                                                                    }
                                                                    selected_ids.set(set);
                                                                }
                                                            }
                                                        }
                                                    }
                                                    td { style: "font-weight: 500; color: var(--text-primary);", "{c.title}" }
                                                    td { style: "font-weight: 600; color: #a78bfa;", "📚 {c.series}" }
                                                    td { style: "text-align: center; color: var(--text-muted); font-variant-numeric: tabular-nums;", "#{idx_str}" }
                                                    td { style: "text-align: center; font-size: 11px; color: var(--text-muted);", "{lang_str}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if rep.candidates.len() > 400 {
                            div { style: "font-size: 11px; color: var(--text-muted); text-align: center;",
                                "Affichage limité aux 400 premiers livres pour la prévisualisation. L'application mettra à jour l'ensemble des {sel_len} livres sélectionnés."
                            }
                        }

                        // Footer actions
                        div { style: "display: flex; justify-content: flex-end; gap: 10px; padding-top: 8px; border-top: 1px solid var(--border-subtle);",
                            button {
                                class: "btn btn-secondary",
                                disabled: *is_applying.read(),
                                onclick: move |_| on_close.call(()),
                                "{translate(locale, \"action.cancel\")}"
                            }
                            button {
                                class: "btn btn-primary",
                                disabled: sel_len == 0 || *is_applying.read(),
                                onclick: apply_action,
                                if *is_applying.read() {
                                    "{translate(locale, \"tools.bracket_series_applying\")}"
                                } else {
                                    "{translate_fmt(locale, \"tools.bracket_series_apply\", &[(\"count\", &sel_len.to_string())])}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
