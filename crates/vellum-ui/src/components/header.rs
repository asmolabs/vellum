use crate::components::{IconKind, LucideIcon};
use crate::state::ViewMode;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use dioxus::prelude::*;
use std::sync::LazyLock;
use vellum_core::{translate, translate_fmt, ConnectedDevice, Locale};
use vellum_domain::DeviceKind;

pub static VELLUM_LOGO_DATA_URI: LazyLock<String> = LazyLock::new(|| {
    let bytes = include_bytes!("../vellum-icon.png");
    format!("data:image/png;base64,{}", BASE64.encode(bytes))
});

#[allow(dead_code)]
#[component]
pub fn Header(
    locale: Locale,
    mut view_mode: Signal<ViewMode>,
    mut sidebar_collapsed: Signal<bool>,
    connected_devices: Vec<ConnectedDevice>,
    sync_available: bool,
    is_syncing: bool,
    on_import: EventHandler<()>,
    on_import_calibre: EventHandler<()>,
    on_open_audit: EventHandler<()>,
    on_open_content_search: EventHandler<()>,
    on_open_import_rules: EventHandler<()>,
    on_open_bulk_regex: EventHandler<()>,
    on_open_news: EventHandler<()>,
    on_open_web_companion: EventHandler<()>,
    on_sync_device: EventHandler<DeviceKind>,
    on_sync_cloud: EventHandler<()>,
    on_open_settings: EventHandler<()>,
) -> Element {
    let mut tools_open = use_signal(|| false);
    let cur_view = *view_mode.read();

    let section_label = match cur_view {
        ViewMode::Home => translate(locale, "toolbar.view_home")
            .trim_start_matches("🏠 ")
            .to_string(),
        ViewMode::Authors => translate(locale, "nav.authors").to_string(),
        ViewMode::Series => translate(locale, "nav.series").to_string(),
        ViewMode::History => translate(locale, "nav.history").to_string(),
        ViewMode::Annotations => "Annotations".to_string(),
        ViewMode::Grid | ViewMode::Table => translate(locale, "nav.library").to_string(),
    };

    rsx! {
        header { class: "app-header", style: "justify-content: space-between;",
            div { style: "display: flex; align-items: center; gap: 10px;",
                if *sidebar_collapsed.read() {
                    button {
                        class: "btn btn-icon",
                        style: "width: 30px; height: 30px;",
                        title: "Déplier la barre latérale",
                        onclick: move |_| sidebar_collapsed.set(false),
                        LucideIcon { kind: IconKind::PanelLeft, size: 15 }
                    }
                }
                div { style: "display: flex; align-items: center; gap: 6px; font-size: 13px; color: var(--text-muted);",
                    span {
                        style: "cursor: pointer; font-weight: 600; color: var(--text-secondary);",
                        onclick: move |_| view_mode.set(ViewMode::Home),
                        "Vellum"
                    }
                    span { "›" }
                    span { style: "font-weight: 600; color: var(--text-primary);", "{section_label}" }
                }
            }

            div { class: "header-actions",
                // Synchro Google Drive Multi-Postes
                if sync_available {
                    button {
                        class: "btn btn-secondary",
                        style: "border-color: rgba(59, 130, 246, 0.4); color: #60a5fa;",
                        title: translate(locale, "header.sync_tooltip"),
                        disabled: is_syncing,
                        onclick: move |_| on_sync_cloud.call(()),
                        LucideIcon { kind: IconKind::Cloud, size: 15 }
                        span {
                            {if is_syncing {
                                translate(locale, "header.syncing")
                            } else {
                                translate(locale, "header.sync")
                            }}
                        }
                    }
                }

                // Liseuses connectées (Kobo / Kindle)
                for dev in connected_devices.iter() {
                    {
                        let kind = dev.kind;
                        let name = dev.name.clone();
                        let icon = match kind {
                            DeviceKind::Kobo => "📖",
                            DeviceKind::Kindle => "📱",
                            DeviceKind::Generic => "📲",
                        };
                        rsx! {
                            button {
                                class: "btn btn-secondary",
                                style: "border-color: rgba(16, 185, 129, 0.4); color: #34d399;",
                                title: translate_fmt(locale, "header.sync_device_tooltip", &[("name", &name)]),
                                onclick: move |_| on_sync_device.call(kind),
                                span { "{icon}" }
                                span { "{name}" }
                            }
                        }
                    }
                }

                // Menu Outils regroupé
                div { class: "tools-dropdown",
                    button {
                        class: "btn btn-secondary",
                        onclick: move |_| {
                            let cur = *tools_open.read();
                            tools_open.set(!cur);
                        },
                        LucideIcon { kind: IconKind::Wrench, size: 14 }
                        span { "{translate(locale, \"header.tools\")}" }
                        LucideIcon { kind: IconKind::ChevronDown, size: 13 }
                    }

                    if *tools_open.read() {
                        div {
                            style: "position: fixed; inset: 0; z-index: 1199;",
                            onclick: move |_| tools_open.set(false),
                        }
                        div { class: "tools-menu",
                            button {
                                class: "tools-menu-item",
                                onclick: move |_| {
                                    tools_open.set(false);
                                    on_open_audit.call(());
                                },
                                span { "🛡️" }
                                span { "{translate(locale, \"header.tools_audit\")}" }
                            }
                            button {
                                class: "tools-menu-item",
                                onclick: move |_| {
                                    tools_open.set(false);
                                    on_open_import_rules.call(());
                                },
                                span { "⚡" }
                                span { "{translate(locale, \"header.tools_import_rules\")}" }
                            }
                            button {
                                class: "tools-menu-item",
                                onclick: move |_| {
                                    tools_open.set(false);
                                    on_open_bulk_regex.call(());
                                },
                                span { "🔤" }
                                span { "{translate(locale, \"header.tools_bulk_regex\")}" }
                            }
                            button {
                                class: "tools-menu-item",
                                onclick: move |_| {
                                    tools_open.set(false);
                                    on_open_news.call(());
                                },
                                span { "📰" }
                                span { "{translate(locale, \"header.tools_news\")}" }
                            }
                            button {
                                class: "tools-menu-item",
                                onclick: move |_| {
                                    tools_open.set(false);
                                    on_open_web_companion.call(());
                                },
                                span { "🌐" }
                                span { "{translate(locale, \"header.tools_web_companion\")}" }
                            }
                            button {
                                class: "tools-menu-item",
                                onclick: move |_| {
                                    tools_open.set(false);
                                    on_import_calibre.call(());
                                },
                                span { "📦" }
                                span { "{translate(locale, \"header.tools_import_calibre\")}" }
                            }
                        }
                    }
                }

                button {
                    class: "btn btn-primary",
                    onclick: move |_| on_import.call(()),
                    LucideIcon { kind: IconKind::Plus, size: 15 }
                    span { "{translate(locale, \"action.import\")}" }
                }
                button {
                    class: "btn btn-icon",
                    title: translate(locale, "header.settings_tooltip"),
                    onclick: move |_| on_open_settings.call(()),
                    LucideIcon { kind: IconKind::Settings, size: 16 }
                }
            }
        }
    }
}
