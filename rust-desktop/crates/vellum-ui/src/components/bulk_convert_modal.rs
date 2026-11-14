use dioxus::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use vellum_core::{translate, translate_fmt, BookService, BulkConvertReport, Locale};

#[component]
pub fn BulkConvertModal(
    locale: Locale,
    book_service: Arc<BookService>,
    target_book_ids: Vec<String>,
    on_close: EventHandler<()>,
    on_converted: EventHandler<BulkConvertReport>,
) -> Element {
    let mut target_format = use_signal(|| "epub".to_string());
    let mut is_running = use_signal(|| false);
    let mut progress_done = use_signal(|| 0usize);
    let mut progress_total = use_signal(|| 0usize);
    let mut current_title = use_signal(String::new);
    let mut cancel_token = use_signal(|| None::<Arc<AtomicBool>>);
    let mut report = use_signal(|| None::<BulkConvertReport>);

    let count = target_book_ids.len();

    let run_convert = {
        let bs = book_service.clone();
        let ids = target_book_ids.clone();
        move |_| {
            is_running.set(true);
            report.set(None);
            progress_done.set(0);
            progress_total.set(ids.len());
            current_title.set(String::new());

            let flag = Arc::new(AtomicBool::new(false));
            cancel_token.set(Some(flag.clone()));

            let fmt = target_format.read().clone();
            let bs_task = bs.clone();
            let ids_task = ids.clone();

            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<(usize, usize, String)>();

            let handle = tokio::task::spawn_blocking(move || {
                bs_task.bulk_convert_books_with_progress(
                    &ids_task,
                    &fmt,
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

                let rep = handle.await.unwrap_or_else(|e| BulkConvertReport {
                    target_format: String::new(),
                    converted_count: 0,
                    skipped_count: 0,
                    errors: vec![format!("Erreur asynchrone : {e}")],
                    cancelled: false,
                });

                is_running.set(false);
                cancel_token.set(None);
                report.set(Some(rep.clone()));
                on_converted.call(rep);
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
                style: "width: 480px; max-width: 94vw; padding: 24px; display: flex; flex-direction: column; gap: 16px;",
                onclick: move |e| e.stop_propagation(),

                div {
                    style: "display: flex; align-items: center; justify-content: space-between;",
                    div {
                        h2 { style: "margin: 0; font-size: 18px;", {translate(locale, "bulk.convert_title")} }
                        p {
                            style: "margin: 4px 0 0 0; font-size: 12px; color: var(--text-muted);",
                            {
                                let count_str = count.to_string();
                                translate_fmt(locale, "bulk.selected_count", &[("count", &count_str)])
                            }
                        }
                    }
                    button {
                        class: "btn btn-secondary",
                        disabled: *is_running.read(),
                        onclick: move |_| on_close.call(()),
                        "✕"
                    }
                }

                div {
                    style: "display: flex; flex-direction: column; gap: 8px;",
                    label { style: "font-size: 13px; font-weight: 600;", {translate(locale, "bulk.target_format")} }
                    select {
                        class: "form-input",
                        disabled: *is_running.read(),
                        value: "{target_format}",
                        onchange: move |e| target_format.set(e.value()),
                        option { value: "epub", {translate(locale, "bulk.fmt_epub")} }
                        option { value: "kepub", {translate(locale, "bulk.fmt_kepub")} }
                        option { value: "pdf", {translate(locale, "bulk.fmt_pdf")} }
                        option { value: "cbz", {translate(locale, "bulk.fmt_cbz")} }
                        option { value: "txt", {translate(locale, "bulk.fmt_txt")} }
                    }
                }

                if *is_running.read() {
                    div {
                        style: "padding: 10px 12px; border-radius: 8px; background: rgba(99, 102, 241, 0.08); border: 1px solid rgba(99, 102, 241, 0.25); display: flex; flex-direction: column; gap: 6px;",
                        div { style: "display: flex; justify-content: space-between; font-size: 12px; font-weight: 600;",
                            span {
                                {
                                    let d = done_val.to_string();
                                    let t = total_val.to_string();
                                    translate_fmt(locale, "bulk.progress", &[("done", &d), ("total", &t)])
                                }
                            }
                            span { "{pct:.0}%" }
                        }
                        div { style: "width: 100%; height: 6px; background: rgba(255,255,255,0.08); border-radius: 99px; overflow: hidden;",
                            div { style: "width: {pct}%; height: 100%; background: var(--accent-primary); transition: width 0.15s ease;" }
                        }
                        if !current_title.read().is_empty() {
                            div { style: "font-size: 11px; color: var(--text-muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis;",
                                {translate_fmt(locale, "bulk.running", &[("title", &current_title.read())])}
                            }
                        }
                    }
                }

                if let Some(rep) = report.read().as_ref() {
                    {
                        let err_count = rep.errors.len().to_string();
                        let converted = rep.converted_count.to_string();
                        let skipped = rep.skipped_count.to_string();
                        let status_prefix = if rep.cancelled {
                            translate(locale, "bulk.status_cancelled")
                        } else {
                            translate(locale, "bulk.status_result")
                        };
                        rsx! {
                            div {
                                style: "padding: 10px 12px; border-radius: 8px; background: rgba(255,255,255,0.04); border: 1px solid var(--border-subtle); font-size: 13px; display: flex; flex-direction: column; gap: 6px;",
                                div { style: "font-weight: 600;",
                                    {translate_fmt(
                                        locale,
                                        "bulk.convert_summary",
                                        &[
                                            ("prefix", status_prefix),
                                            ("fmt", &rep.target_format),
                                            ("converted", &converted),
                                            ("skipped", &skipped),
                                            ("errors", &err_count),
                                        ],
                                    )}
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
                    if *is_running.read() {
                        button {
                            class: "btn btn-danger",
                            onclick: move |_| {
                                if let Some(ref flag) = *cancel_token.read() {
                                    flag.store(true, Ordering::Relaxed);
                                }
                            },
                            {translate(locale, "bulk.cancel_batch")}
                        }
                    } else {
                        button {
                            class: "btn btn-secondary",
                            onclick: move |_| on_close.call(()),
                            {translate(locale, "action.close")}
                        }
                        button {
                            class: "btn btn-primary",
                            onclick: run_convert,
                            {translate(locale, "bulk.start_convert")}
                        }
                    }
                }
            }
        }
    }
}
