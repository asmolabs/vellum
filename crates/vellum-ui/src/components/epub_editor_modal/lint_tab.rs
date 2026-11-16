//! Onglet de diagnostic et réparation automatique EPUB.

use dioxus::prelude::*;
use vellum_core::{translate, translate_fmt, Locale};
use vellum_domain::{EpubDiagnostics, EpubIssueSeverity};

#[component]
pub(super) fn EpubLintTab(
    locale: Locale,
    diagnostics: Signal<Option<EpubDiagnostics>>,
    is_operating: Signal<bool>,
    on_repair: EventHandler<()>,
) -> Element {
    rsx! {
        if let Some(diag) = diagnostics.read().as_ref() {
            // 1. Metric Cards
            div { style: "display: grid; grid-template-columns: repeat(auto-fit, minmax(130px, 1fr)); gap: 10px;",
                div { style: "background: var(--bg-card); padding: 10px 12px; border-radius: 8px; border: 1px solid var(--border-color);",
                    div { style: "font-size: 11px; color: var(--text-secondary);", {translate(locale, "epub.compliance")} }
                    div { style: "font-size: 14px; font-weight: 700; margin-top: 4px;",
                        if diag.is_valid {
                            span { style: "color: #34d399;", {translate(locale, "epub.valid")} }
                        } else {
                            span { style: "color: #f87171;", {translate(locale, "epub.invalid")} }
                        }
                    }
                }

                div { style: "background: var(--bg-card); padding: 10px 12px; border-radius: 8px; border: 1px solid var(--border-color);",
                    div { style: "font-size: 11px; color: var(--text-secondary);", {translate(locale, "epub.standard")} }
                    div { style: "font-size: 14px; font-weight: 700; margin-top: 4px; color: var(--text-primary);",
                        "EPUB {diag.epub_version}"
                    }
                }

                div { style: "background: var(--bg-card); padding: 10px 12px; border-radius: 8px; border: 1px solid var(--border-color);",
                    div { style: "font-size: 11px; color: var(--text-secondary);", {translate(locale, "epub.spine")} }
                    div { style: "font-size: 14px; font-weight: 700; margin-top: 4px; color: var(--text-primary);",
                        "{diag.spine_count}"
                    }
                }

                div { style: "background: var(--bg-card); padding: 10px 12px; border-radius: 8px; border: 1px solid var(--border-color);",
                    div { style: "font-size: 11px; color: var(--text-secondary);", {translate(locale, "epub.toc_entries")} }
                    div { style: "font-size: 14px; font-weight: 700; margin-top: 4px; color: var(--text-primary);",
                        "{diag.toc_entries_count}"
                    }
                }

                div { style: "background: var(--bg-card); padding: 10px 12px; border-radius: 8px; border: 1px solid var(--border-color);",
                    div { style: "font-size: 11px; color: var(--text-secondary);", {translate(locale, "epub.cover")} }
                    div { style: "font-size: 14px; font-weight: 700; margin-top: 4px;",
                        if diag.has_cover {
                            span { style: "color: #34d399;", {translate(locale, "epub.cover_ok")} }
                        } else {
                            span { style: "color: #fbbf24;", {translate(locale, "epub.cover_missing")} }
                        }
                    }
                }

                div { style: "background: var(--bg-card); padding: 10px 12px; border-radius: 8px; border: 1px solid var(--border-color);",
                    div { style: "font-size: 11px; color: var(--text-secondary);", {translate(locale, "epub.size")} }
                    div { style: "font-size: 14px; font-weight: 700; margin-top: 4px; color: var(--text-primary);",
                        "{format_bytes(diag.total_size_bytes)}"
                    }
                }
            }

            // 2. Action to Clean and Repair
            div { style: "display: flex; justify-content: space-between; align-items: center; padding: 12px 14px; background: rgba(59, 130, 246, 0.08); border: 1px solid rgba(59, 130, 246, 0.25); border-radius: 8px;",
                div {
                    div { style: "font-size: 13px; font-weight: 600; color: #60a5fa;", {translate(locale, "epub.repair_title")} }
                    div { style: "font-size: 12px; color: var(--text-secondary); margin-top: 2px;",
                        {translate(locale, "epub.repair_desc")}
                    }
                }
                button {
                    class: "btn btn-primary",
                    style: "font-size: 13px; padding: 7px 16px; white-space: nowrap;",
                    disabled: *is_operating.read(),
                    onclick: move |_| on_repair.call(()),
                    {if *is_operating.read() { translate(locale, "epub.repairing") } else { translate(locale, "epub.repair_btn") }}
                }
            }

            // 3. Issues List
            div { style: "display: flex; flex-direction: column; gap: 8px; margin-top: 4px;",
                h4 { style: "font-size: 13px; font-weight: 600; color: var(--text-secondary); margin: 0;",
                    {
                        let iss_n = diag.issues.len().to_string();
                        translate_fmt(locale, "epub.issues_heading", &[("count", &iss_n)])
                    }
                }

                if diag.issues.is_empty() {
                    div { style: "padding: 16px; text-align: center; color: #34d399; background: rgba(16, 185, 129, 0.08); border-radius: 8px; font-size: 13px;",
                        {translate(locale, "epub.no_issues")}
                    }
                } else {
                    for issue in &diag.issues {
                        {
                            let (badge_bg, badge_color, badge_text) = match issue.severity {
                                EpubIssueSeverity::Error => ("rgba(239, 68, 68, 0.15)", "#f87171", translate(locale, "epub.sev_error")),
                                EpubIssueSeverity::Warning => ("rgba(245, 158, 11, 0.15)", "#fbbf24", translate(locale, "epub.sev_warn")),
                                EpubIssueSeverity::Info => ("rgba(59, 130, 246, 0.15)", "#60a5fa", translate(locale, "epub.sev_info")),
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
                                                {translate_fmt(locale, "epub.file_prefix", &[("file", file_desc)])}
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
