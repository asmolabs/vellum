use dioxus::prelude::*;
use vellum_domain::{CloudSyncConfig, PluginDescriptor};

#[component]
pub fn SettingsModal(
    opds_port: u16,
    opds_running: bool,
    litestream_status_text: String,
    plugins: Vec<PluginDescriptor>,
    sync_config: CloudSyncConfig,
    sync_available: bool,
    is_syncing: bool,
    sync_message: Option<String>,
    on_sync_now: EventHandler<()>,
    on_select_folder: EventHandler<()>,
    on_auto_detect_folder: EventHandler<()>,
    on_toggle_auto_download: EventHandler<bool>,
    on_toggle_enabled: EventHandler<bool>,
    on_close: EventHandler<()>,
) -> Element {
    let opds_url = format!("http://localhost:{opds_port}/opds");
    let folder_display = sync_config
        .sync_folder_path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "Aucun dossier sélectionné".to_string());

    let (sync_badge_text, sync_badge_style) = if sync_available && sync_config.enabled {
        ("ACTIF", "background: rgba(16, 185, 129, 0.2); color: #34d399; border-color: rgba(16, 185, 129, 0.4);")
    } else if sync_available && !sync_config.enabled {
        ("DÉSACTIVÉ", "background: rgba(148, 163, 184, 0.2); color: #94a3b8; border-color: rgba(148, 163, 184, 0.3);")
    } else if sync_config.sync_folder_path.is_some() && !sync_available {
        ("DOSSIER INTROUVABLE", "background: rgba(245, 158, 11, 0.2); color: #fbbf24; border-color: rgba(245, 158, 11, 0.4);")
    } else {
        ("NON CONFIGURÉ", "background: rgba(239, 68, 68, 0.2); color: #f87171; border-color: rgba(239, 68, 68, 0.3);")
    };

    let last_sync_text = sync_config
        .last_synced_at
        .as_deref()
        .unwrap_or("Jamais synchronisé");

    rsx! {
        div {
            class: "drawer-backdrop",
            onclick: move |_| on_close.call(()),

            div {
                class: "drawer-panel",
                style: "width: 620px;",
                onclick: move |evt| evt.stop_propagation(),

                div { class: "drawer-header",
                    h3 { style: "font-size: 16px; font-weight: 700;", "Paramètres & Diagnostics Vellum" }
                    button {
                        class: "btn btn-icon",
                        onclick: move |_| on_close.call(()),
                        "✕"
                    }
                }

                // 1. Synchronisation Google Drive Multi-Postes
                div { style: "display: flex; flex-direction: column; gap: 12px; padding: 16px; background: rgba(59, 130, 246, 0.04); border: 1px solid rgba(59, 130, 246, 0.2); border-radius: 12px;",
                    div { style: "display: flex; justify-content: space-between; align-items: center;",
                        div { style: "display: flex; align-items: center; gap: 8px;",
                            span { style: "font-size: 16px;", "☁️" }
                            span { style: "font-weight: 700; font-size: 14px; color: #93c5fd;", "Synchronisation Google Drive Multi-Postes" }
                        }
                        span {
                            class: "format-pill",
                            style: "{sync_badge_style}",
                            "{sync_badge_text}"
                        }
                    }

                    p { style: "font-size: 12px; color: var(--text-secondary); line-height: 1.5;",
                        "Synchronise de manière transparente vos livres (EPUB, PDF, CBZ...) et votre progression de lecture entre plusieurs machines. "
                        "Architecture décentralisée sécurisée sans risque de corruption de base de données."
                    }

                    // Emplacement du dossier Drive
                    div { style: "display: flex; flex-direction: column; gap: 6px;",
                        span { style: "font-size: 12px; font-weight: 600; color: var(--text-secondary);", "Dossier de synchronisation Google Drive :" }
                        div { style: "display: flex; align-items: center; gap: 8px;",
                            div {
                                style: "flex: 1; font-size: 11px; background: rgba(0,0,0,0.4); padding: 8px 12px; border-radius: 6px; font-family: monospace; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; border: 1px solid var(--border-subtle);",
                                title: "{folder_display}",
                                "{folder_display}"
                            }
                            button {
                                class: "btn btn-secondary",
                                style: "font-size: 11px; padding: 6px 10px; white-space: nowrap;",
                                onclick: move |_| on_select_folder.call(()),
                                "📁 Choisir..."
                            }
                            button {
                                class: "btn btn-secondary",
                                style: "font-size: 11px; padding: 6px 10px; white-space: nowrap;",
                                title: "Détecter automatiquement Google Drive sur macOS / Windows",
                                onclick: move |_| on_auto_detect_folder.call(()),
                                "🔍 Auto-détecter"
                            }
                        }
                    }

                    // Options de synchronisation
                    div { style: "display: flex; flex-direction: column; gap: 8px; margin-top: 4px; padding-top: 8px; border-top: 1px solid rgba(255,255,255,0.06);",
                        label { style: "display: flex; align-items: center; gap: 10px; cursor: pointer; font-size: 12px;",
                            input {
                                r#type: "checkbox",
                                checked: sync_config.enabled,
                                onchange: move |evt| on_toggle_enabled.call(evt.value() == "true"),
                            }
                            span { style: "font-weight: 500;", "Activer la synchronisation avec ce dossier" }
                        }

                        label { style: "display: flex; align-items: center; gap: 10px; cursor: pointer; font-size: 12px;",
                            input {
                                r#type: "checkbox",
                                checked: sync_config.auto_download_books,
                                onchange: move |evt| on_toggle_auto_download.call(evt.value() == "true"),
                            }
                            div { style: "display: flex; flex-direction: column;",
                                span { style: "font-weight: 500;", "Télécharger automatiquement les nouveaux livres (Miroir complet)" }
                                span { style: "font-size: 11px; color: var(--text-muted);", "Récupère les fichiers EPUB ajoutés depuis vos autres ordinateurs." }
                            }
                        }
                    }

                    // Message d'état de synchronisation si présent
                    if let Some(msg) = sync_message.as_ref() {
                        div {
                            style: "font-size: 12px; padding: 8px 12px; border-radius: 6px; background: rgba(99, 102, 241, 0.15); border: 1px solid rgba(99, 102, 241, 0.3); color: #c7d2fe;",
                            "{msg}"
                        }
                    }

                    // Actions & Pied du bloc Sync
                    div { style: "display: flex; justify-content: space-between; align-items: center; margin-top: 4px; font-size: 11px; color: var(--text-muted);",
                        div { style: "display: flex; flex-direction: column; gap: 2px;",
                            span { "Poste : {sync_config.client_name} ({sync_config.client_id})" }
                            span { "Dernière synchro : {last_sync_text}" }
                        }
                        button {
                            class: "btn btn-primary",
                            style: "font-size: 12px; padding: 6px 14px; gap: 6px;",
                            disabled: is_syncing || !sync_available,
                            onclick: move |_| on_sync_now.call(()),
                            span { if is_syncing { "⏳" } else { "🔄" } }
                            span { if is_syncing { "Synchro en cours..." } else { "Synchroniser maintenant" } }
                        }
                    }
                }

                // 2. Serveur OPDS
                div { style: "display: flex; flex-direction: column; gap: 8px; padding: 14px; background: rgba(255,255,255,0.03); border: 1px solid var(--border-subtle); border-radius: 10px;",
                    div { style: "display: flex; justify-content: space-between; align-items: center;",
                        span { style: "font-weight: 600; font-size: 13px;", "📡 Serveur OPDS v1.2" }
                        span {
                            class: "format-pill",
                            style: if opds_running { "background: rgba(16, 185, 129, 0.2); color: #34d399; border-color: rgba(16, 185, 129, 0.4);" } else { "background: rgba(239, 68, 68, 0.2); color: #f87171;" },
                            if opds_running { "ACTIF" } else { "ARRÊTÉ" }
                        }
                    }
                    p { style: "font-size: 12px; color: var(--text-secondary);",
                        "Partagez votre catalogue local avec vos liseuses physiques et applications mobiles (Kobo, Moon+ Reader, Thorium, Aldiko, Koreader)."
                    }
                    div { style: "display: flex; align-items: center; justify-content: space-between; font-size: 12px; background: rgba(0,0,0,0.3); padding: 8px 12px; border-radius: 6px; font-family: monospace;",
                        span { "{opds_url}" }
                        span { style: "color: var(--accent-primary);", "Port {opds_port}" }
                    }
                }

                // 3. Litestream
                div { style: "display: flex; flex-direction: column; gap: 8px; padding: 14px; background: rgba(255,255,255,0.03); border: 1px solid var(--border-subtle); border-radius: 10px;",
                    div { style: "display: flex; justify-content: space-between; align-items: center;",
                        span { style: "font-weight: 600; font-size: 13px;", "☁️ Réplication SQLite (Litestream)" }
                        span { class: "format-pill", "WAL STREAM" }
                    }
                    p { style: "font-size: 12px; color: var(--text-secondary);",
                        "Sauvegarde continue et réplication temps réel vers stockage distant ou bucket S3 compatible."
                    }
                    div { style: "font-size: 12px; color: var(--text-muted);",
                        "État : {litestream_status_text}"
                    }
                }

                // 4. Plugins
                div { style: "display: flex; flex-direction: column; gap: 8px;",
                    div { class: "sidebar-section-title", "Extensions & Formats Détectés ({plugins.len()})" }
                    div { style: "display: flex; flex-direction: column; gap: 6px; max-height: 200px; overflow-y: auto;",
                        for p in plugins.iter() {
                            div {
                                style: "display: flex; justify-content: space-between; align-items: center; padding: 8px 12px; background: rgba(255,255,255,0.02); border: 1px solid var(--border-subtle); border-radius: 8px; font-size: 12px;",
                                div { style: "display: flex; flex-direction: column;",
                                    span { style: "font-weight: 600;", "{p.name}" }
                                    span { style: "font-size: 10px; color: var(--text-muted);", "{p.id} • v{p.version}" }
                                }
                                span { class: "format-pill", "{p.type_string()}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
