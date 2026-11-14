//! Onglet de diagnostic et réparation automatique EPUB.

use dioxus::prelude::*;
use vellum_domain::{EpubDiagnostics, EpubIssueSeverity};

#[component]
pub(super) fn EpubLintTab(
    diagnostics: Signal<Option<EpubDiagnostics>>,
    is_operating: Signal<bool>,
    on_repair: EventHandler<()>,
) -> Element {
    rsx! {
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
                    onclick: move |_| on_repair.call(()),
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
