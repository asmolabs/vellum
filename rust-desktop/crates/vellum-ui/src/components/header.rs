use dioxus::prelude::*;
use vellum_core::ConnectedDevice;
use vellum_domain::DeviceKind;

#[component]
pub fn Header(
    search_query: Signal<String>,
    connected_devices: Vec<ConnectedDevice>,
    sync_available: bool,
    is_syncing: bool,
    on_import: EventHandler<()>,
    on_import_calibre: EventHandler<()>,
    on_open_news: EventHandler<()>,
    on_open_web_companion: EventHandler<()>,
    on_sync_device: EventHandler<DeviceKind>,
    on_sync_cloud: EventHandler<()>,
    on_open_palette: EventHandler<()>,
    on_open_settings: EventHandler<()>,
) -> Element {
    rsx! {
        header { class: "app-header",
            div { class: "logo-area",
                div { class: "logo-icon", "V" }
                span { class: "logo-text", "Vellum" }
                span { class: "logo-badge", "RUST" }
            }

            div { class: "header-search",
                span { class: "search-icon-slot", "🔍" }
                input {
                    r#type: "text",
                    placeholder: "Rechercher (ex: Dune, author:Herbert, series:Fondation, tag:SF)...",
                    value: "{search_query}",
                    oninput: move |evt| search_query.set(evt.value()),
                }
                button {
                    class: "btn btn-icon",
                    style: "font-size: 11px; padding: 2px 6px; border-radius: 4px; background: rgba(255,255,255,0.08); color: var(--text-muted); border: 1px solid var(--border-subtle); margin-right: 6px; cursor: pointer;",
                    title: "Ouvrir la palette de commandes (⌘K)",
                    onclick: move |_| on_open_palette.call(()),
                    "⌘K"
                }
            }

            div { class: "header-actions",
                // Synchro Google Drive Multi-Postes
                button {
                    class: "btn btn-secondary",
                    style: if sync_available {
                        "border-color: rgba(59, 130, 246, 0.4); color: #60a5fa; font-size: 12px; gap: 6px; padding: 6px 12px;"
                    } else {
                        "border-color: rgba(255, 255, 255, 0.1); color: var(--text-muted); font-size: 12px; gap: 6px; padding: 6px 12px;"
                    },
                    title: if sync_available {
                        "Synchroniser la bibliothèque avec Google Drive (livres & progression)"
                    } else {
                        "Google Drive non configuré — Cliquer pour configurer dans les paramètres"
                    },
                    disabled: is_syncing,
                    onclick: move |_| {
                        if sync_available {
                            on_sync_cloud.call(());
                        } else {
                            on_open_settings.call(());
                        }
                    },
                    span { if is_syncing { "⏳" } else { "☁️" } }
                    span {
                        if is_syncing {
                            "Synchro en cours..."
                        } else if sync_available {
                            "Synchro Drive"
                        } else {
                            "Drive (inactif)"
                        }
                    }
                }

                // Liseuses physiques connectées
                for dev in connected_devices.iter() {
                    {
                        let dev_kind = dev.kind;
                        let count = dev.book_count;
                        let label = match dev_kind {
                            DeviceKind::Kobo => format!("📱 Kobo ({count})"),
                            DeviceKind::Kindle => format!("📱 Kindle ({count})"),
                            DeviceKind::Generic => format!("📱 Liseuse ({count})"),
                        };
                        rsx! {
                            button {
                                class: "btn btn-secondary",
                                style: "border-color: rgba(16, 185, 129, 0.4); color: #34d399; font-size: 12px; gap: 6px; padding: 6px 12px;",
                                title: "Cliquez pour synchroniser et importer les livres depuis l'appareil",
                                onclick: move |_| on_sync_device.call(dev_kind),
                                span { "🔄" }
                                span { "{label}" }
                            }
                        }
                    }
                }

                button {
                    class: "btn btn-secondary",
                    style: "border-color: rgba(14, 165, 233, 0.4); color: #38bdf8; font-size: 12px; gap: 6px; padding: 6px 12px;",
                    title: "Recettes d'actualités : compiler des flux RSS en journal numérique quotidien (EPUB)",
                    onclick: move |_| on_open_news.call(()),
                    span { "📰" }
                    span { "Actualités" }
                }
                button {
                    class: "btn btn-secondary",
                    style: "border-color: rgba(99, 102, 241, 0.4); color: #818cf8; font-size: 12px; gap: 6px; padding: 6px 12px;",
                    title: "Ouvrir le Compagnon Web Mobile & Serveur OPDS dans le navigateur",
                    onclick: move |_| on_open_web_companion.call(()),
                    span { "🌐" }
                    span { "Web" }
                }
                button {
                    class: "btn btn-secondary",
                    style: "border-color: rgba(167, 139, 250, 0.4); color: #c4b5fd; font-size: 12px; gap: 6px; padding: 6px 12px;",
                    title: "Migrer ou importer une bibliothèque Calibre existante (metadata.db)",
                    onclick: move |_| on_import_calibre.call(()),
                    span { "📦" }
                    span { "Calibre" }
                }
                button {
                    class: "btn btn-primary",
                    onclick: move |_| on_import.call(()),
                    span { "＋" }
                    span { "Importer" }
                }
                button {
                    class: "btn btn-icon",
                    title: "Paramètres & Diagnostics",
                    onclick: move |_| on_open_settings.call(()),
                    "⚙"
                }
            }
        }
    }
}
