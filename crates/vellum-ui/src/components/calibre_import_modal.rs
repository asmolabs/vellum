use dioxus::prelude::*;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use vellum_core::{translate, translate_fmt, BookService, CalibreImportReport, Locale};

#[component]
pub fn CalibreImportModal(
    locale: Locale,
    book_service: Arc<BookService>,
    calibre_path: PathBuf,
    on_close: EventHandler<()>,
) -> Element {
    let mut copy_files = use_signal(|| true);
    let mut is_running = use_signal(|| false);
    let mut progress_done = use_signal(|| 0usize);
    let mut progress_total = use_signal(|| 0usize);
    let mut current_title = use_signal(String::new);
    let mut cancel_token = use_signal(|| None::<Arc<AtomicBool>>);
    let mut report = use_signal(|| None::<CalibreImportReport>);

    let start_import = {
        let bs = book_service.clone();
        let path = calibre_path.clone();
        move |_| {
            is_running.set(true);
            report.set(None);
            progress_done.set(0);
            progress_total.set(0);
            current_title.set(String::new());

            let flag = Arc::new(AtomicBool::new(false));
            cancel_token.set(Some(flag.clone()));

            let bs_task = bs.clone();
            let path_task = path.clone();
            let copy_task = *copy_files.read();

            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<(usize, usize, String)>();

            let handle = tokio::task::spawn_blocking(move || {
                bs_task.import_calibre_library_with_progress(
                    &path_task,
                    copy_task,
                    Some(flag.as_ref()),
                    |done, total, title| {
                        let _ = tx.send((done, total, title.to_string()));
                    },
                )
            });

            spawn(async move {
                while let Some((done, total, title)) = rx.recv().await {
                    progress_done.set(done);
                    progress_total.set(total);
                    current_title.set(title);
                }

                let rep = match handle.await {
                    Ok(Ok(r)) => r,
                    Ok(Err(e)) => CalibreImportReport {
                        errors: vec![e.to_string()],
                        cancelled: false,
                        ..Default::default()
                    },
                    Err(e) => CalibreImportReport {
                        errors: vec![translate_fmt(
                            locale,
                            "common.async_error",
                            &[("error", &e.to_string())],
                        )],
                        cancelled: false,
                        ..Default::default()
                    },
                };

                is_running.set(false);
                cancel_token.set(None);
                report.set(Some(rep));
            });
        }
    };

    let done_val = *progress_done.read();
    let total_val = *progress_total.read();
    let pct = if total_val > 0 {
        ((done_val as f64 / total_val as f64) * 100.0).min(100.0)
    } else {
        0.0
    };

    rsx! {
        div {
            class: "drawer-backdrop",
            style: "align-items: center; justify-content: center;",
            onclick: move |_| {
                if !*is_running.read() {
                    on_close.call(());
                }
            },
            div {
                class: "modal-card",
                style: "width: 520px; max-width: 94vw; padding: 24px; display: flex; flex-direction: column; gap: 16px;",
                onclick: move |e| e.stop_propagation(),

                // Header
                div {
                    style: "display: flex; align-items: flex-start; justify-content: space-between;",
                    div {
                        h2 { style: "margin: 0; font-size: 18px;", {translate(locale, "calibre.modal_title")} }
                        p {
                            style: "margin: 4px 0 0 0; font-size: 12px; color: var(--text-muted); word-break: break-all;",
                            "{calibre_path.display()}"
                        }
                    }
                    button {
                        class: "btn btn-secondary",
                        disabled: *is_running.read(),
                        onclick: move |_| on_close.call(()),
                        "✕"
                    }
                }

                // Options before starting
                if !*is_running.read() && report.read().is_none() {
                    div {
                        style: "display: flex; flex-direction: column; gap: 12px; padding: 12px; border-radius: 8px; background: rgba(255,255,255,0.03); border: 1px solid var(--border-subtle);",
                        p {
                            style: "margin: 0; font-size: 13px; color: var(--text-secondary); line-height: 1.4;",
                            {translate(locale, "calibre.modal_desc")}
                        }
                        label {
                            style: "display: flex; align-items: flex-start; gap: 10px; cursor: pointer; margin-top: 4px;",
                            input {
                                r#type: "checkbox",
                                checked: *copy_files.read(),
                                onchange: move |e| copy_files.set(e.value() == "true" || e.value() == "on"),
                            }
                            div {
                                div { style: "font-size: 13px; font-weight: 500;", {translate(locale, "calibre.copy_files")} }
                                div { style: "font-size: 11px; color: var(--text-muted); margin-top: 2px;", {translate(locale, "calibre.copy_files_hint")} }
                            }
                        }
                    }
                }

                // Live Progress
                if *is_running.read() {
                    div {
                        style: "padding: 14px 16px; border-radius: 8px; background: rgba(99, 102, 241, 0.08); border: 1px solid rgba(99, 102, 241, 0.25); display: flex; flex-direction: column; gap: 10px;",
                        div {
                            style: "display: flex; justify-content: space-between; font-size: 13px; font-weight: 600;",
                            span {
                                {
                                    let d = done_val.to_string();
                                    let t = total_val.to_string();
                                    let p = format!("{pct:.0}");
                                    translate_fmt(locale, "calibre.progress", &[("done", &d), ("total", &t), ("pct", &p)])
                                }
                            }
                            span { "{pct:.0}%" }
                        }
                        div {
                            style: "width: 100%; height: 8px; background: rgba(255,255,255,0.08); border-radius: 99px; overflow: hidden;",
                            div {
                                style: "width: {pct}%; height: 100%; background: var(--accent-primary); transition: width 0.15s ease;"
                            }
                        }
                        if !current_title.read().is_empty() {
                            div {
                                style: "font-size: 12px; color: var(--text-muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis;",
                                {translate_fmt(locale, "calibre.current_book", &[("title", &current_title.read())])}
                            }
                        }
                    }
                }

                // Summary Report
                if let Some(rep) = report.read().as_ref() {
                    div {
                        style: "padding: 14px 16px; border-radius: 8px; background: rgba(255,255,255,0.04); border: 1px solid var(--border-subtle); font-size: 13px; display: flex; flex-direction: column; gap: 8px;",
                        div {
                            style: "font-weight: 600; display: flex; align-items: center; gap: 8px;",
                            if rep.cancelled {
                                span { style: "color: #f59e0b;", "⚠️ {translate(locale, \"calibre.status_cancelled\")}" }
                            } else {
                                span { style: "color: #10b981;", "✅ {translate(locale, \"calibre.status_done\")}" }
                            }
                        }
                        div {
                            style: "color: var(--text-secondary); line-height: 1.4;",
                            {
                                let b = rep.books_imported.to_string();
                                let f = rep.formats_imported.to_string();
                                let s = rep.series_count.to_string();
                                let t = rep.tags_count.to_string();
                                translate_fmt(
                                    locale,
                                    "calibre.summary_details",
                                    &[("books", &b), ("formats", &f), ("series", &s), ("tags", &t)],
                                )
                            }
                        }
                        if !rep.errors.is_empty() {
                            div {
                                style: "margin-top: 4px; padding-top: 6px; border-top: 1px solid rgba(255,255,255,0.08);",
                                div { style: "font-weight: 600; color: #fca5a5; margin-bottom: 4px;", "Erreurs ({rep.errors.len()}) :" }
                                for err in rep.errors.iter().take(5) {
                                    div { style: "font-size: 12px; color: #fca5a5;", "• {err}" }
                                }
                            }
                        }
                    }
                }

                // Actions Footer
                div {
                    style: "display: flex; justify-content: flex-end; gap: 10px; margin-top: 4px;",
                    if *is_running.read() {
                        button {
                            class: "btn btn-danger",
                            onclick: move |_| {
                                if let Some(ref flag) = *cancel_token.read() {
                                    flag.store(true, Ordering::Relaxed);
                                }
                            },
                            {translate(locale, "calibre.cancel_btn")}
                        }
                    } else if report.read().is_some() {
                        button {
                            class: "btn btn-primary",
                            onclick: move |_| on_close.call(()),
                            {translate(locale, "action.close")}
                        }
                    } else {
                        button {
                            class: "btn btn-secondary",
                            onclick: move |_| on_close.call(()),
                            {translate(locale, "action.close")}
                        }
                        button {
                            class: "btn btn-primary",
                            onclick: start_import,
                            {translate(locale, "calibre.start_import")}
                        }
                    }
                }
            }
        }
    }
}
