use dioxus::prelude::*;
use vellum_domain::PluginDescriptor;

#[component]
pub fn SettingsModal(
    opds_port: u16,
    opds_running: bool,
    litestream_status_text: String,
    plugins: Vec<PluginDescriptor>,
    on_close: EventHandler<()>,
) -> Element {
    let opds_url = format!("http://localhost:{opds_port}/opds");

    rsx! {
        div {
            class: "drawer-backdrop",
            onclick: move |_| on_close.call(()),

            div {
                class: "drawer-panel",
                style: "width: 560px;",
                onclick: move |evt| evt.stop_propagation(),

                div { class: "drawer-header",
                    h3 { style: "font-size: 16px; font-weight: 700;", "Paramètres & Diagnostics Vellum" }
                    button {
                        class: "btn btn-icon",
                        onclick: move |_| on_close.call(()),
                        "✕"
                    }
                }

                // 1. Serveur OPDS
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

                // 2. Litestream
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

                // 3. Plugins
                div { style: "display: flex; flex-direction: column; gap: 8px;",
                    div { class: "sidebar-section-title", "Extensions & Formats Détectés ({plugins.len()})" }
                    div { style: "display: flex; flex-direction: column; gap: 6px; max-height: 240px; overflow-y: auto;",
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
