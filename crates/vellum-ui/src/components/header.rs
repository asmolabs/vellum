use crate::components::{IconKind, LucideIcon};
use crate::state::ViewMode;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use dioxus::prelude::*;
use std::sync::LazyLock;
use vellum_core::{translate, translate_fmt, ConnectedDevice, Locale};
use vellum_domain::DeviceKind;

static VELLUM_LOGO_DATA_URI: LazyLock<String> = LazyLock::new(|| {
    let bytes = include_bytes!("../vellum-icon.png");
    format!("data:image/png;base64,{}", BASE64.encode(bytes))
});

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

    rsx! {
        header { class: "app-header", style: "justify-content: space-between;",
            div { style: "display: flex; align-items: center; gap: 12px;",
                button {
                    class: "btn btn-icon",
                    style: "width: 32px; height: 32px; border-radius: 8px; background: rgba(255,255,255,0.05); border: 1px solid var(--border-subtle); cursor: pointer; display: flex; align-items: center; justify-content: center;",
                    title: "Toggle sidebar",
                    onclick: move |_| {
                        let cur = *sidebar_collapsed.read();
                        sidebar_collapsed.set(!cur);
                    },
                    LucideIcon { kind: IconKind::PanelLeft, size: 16 }
                }
                div {
                    class: "logo-area",
                    style: "cursor: pointer;",
                    onclick: move |_| view_mode.set(ViewMode::Home),
                    img {
                        src: "{*VELLUM_LOGO_DATA_URI}",
                        alt: "Vellum",
                        style: "width: 28px; height: 28px; border-radius: 6px; object-fit: contain;",
                    }
                    span { class: "logo-text", "Vellum" }
                }
                div { style: "display: flex; align-items: center; gap: 4px; background: rgba(255,255,255,0.05); padding: 3px; border-radius: 8px; border: 1px solid var(--border-subtle);",
                    button {
                        style: if cur_view == ViewMode::Home {
                            "display: flex; align-items: center; gap: 6px; padding: 4px 10px; font-size: 12px; border-radius: 6px; background: #6366f1; color: #fff; border: none; font-weight: 600; cursor: pointer;"
                        } else {
                            "display: flex; align-items: center; gap: 6px; padding: 4px 10px; font-size: 12px; border-radius: 6px; background: transparent; color: var(--text-secondary); border: none; cursor: pointer;"
                        },
                        onclick: move |_| view_mode.set(ViewMode::Home),
                        LucideIcon { kind: IconKind::Home, size: 14 }
                        span { "{translate(locale, \"toolbar.view_home\").trim_start_matches(\"🏠 \")}" }
                    }
                    button {
                        style: if cur_view != ViewMode::Home {
                            "display: flex; align-items: center; gap: 6px; padding: 4px 10px; font-size: 12px; border-radius: 6px; background: #6366f1; color: #fff; border: none; font-weight: 600; cursor: pointer;"
                        } else {
                            "display: flex; align-items: center; gap: 6px; padding: 4px 10px; font-size: 12px; border-radius: 6px; background: transparent; color: var(--text-secondary); border: none; cursor: pointer;"
                        },
                        onclick: move |_| view_mode.set(ViewMode::Grid),
                        LucideIcon { kind: IconKind::Library, size: 14 }
                        span { "{translate(locale, \"nav.library\")}" }
                    }
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
