use crate::state::ViewMode;
use dioxus::prelude::*;
use vellum_core::{translate, translate_fmt, ConnectedDevice, Locale};
use vellum_domain::DeviceKind;

#[component]
pub fn Header(
    locale: Locale,
    search_query: Signal<String>,
    mut view_mode: Signal<ViewMode>,
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
    on_open_palette: EventHandler<()>,
    on_open_settings: EventHandler<()>,
) -> Element {
    let mut tools_open = use_signal(|| false);
    let has_search = !search_query.read().is_empty();
    let cur_view = *view_mode.read();

    rsx! {
        header { class: "app-header",
            div { style: "display: flex; align-items: center; gap: 14px;",
                div {
                    class: "logo-area",
                    style: "cursor: pointer;",
                    onclick: move |_| view_mode.set(ViewMode::Home),
                    div { class: "logo-icon", "V" }
                    span { class: "logo-text", "Vellum" }
                }
                div { style: "display: flex; align-items: center; gap: 4px; background: rgba(255,255,255,0.05); padding: 3px; border-radius: 8px; border: 1px solid var(--border-subtle);",
                    button {
                        style: if cur_view == ViewMode::Home {
                            "padding: 4px 10px; font-size: 12px; border-radius: 6px; background: #6366f1; color: #fff; border: none; font-weight: 600; cursor: pointer;"
                        } else {
                            "padding: 4px 10px; font-size: 12px; border-radius: 6px; background: transparent; color: var(--text-secondary); border: none; cursor: pointer;"
                        },
                        onclick: move |_| view_mode.set(ViewMode::Home),
                        "{translate(locale, \"toolbar.view_home\")}"
                    }
                    button {
                        style: if cur_view != ViewMode::Home {
                            "padding: 4px 10px; font-size: 12px; border-radius: 6px; background: #6366f1; color: #fff; border: none; font-weight: 600; cursor: pointer;"
                        } else {
                            "padding: 4px 10px; font-size: 12px; border-radius: 6px; background: transparent; color: var(--text-secondary); border: none; cursor: pointer;"
                        },
                        onclick: move |_| view_mode.set(ViewMode::Grid),
                        "{translate(locale, \"nav.library\")}"
                    }
                }
            }

            div { class: "header-search",
                span { class: "search-icon-slot", "🔍" }
                input {
                    r#type: "text",
                    style: if has_search { "padding-right: 108px;" } else { "padding-right: 84px;" },
                    placeholder: "{translate(locale, \"action.search\")} (Dune, author:Herbert, tag:SF)...",
                    value: "{search_query}",
                    oninput: move |evt| {
                        search_query.set(evt.value());
                        if *view_mode.read() == ViewMode::Home {
                            view_mode.set(ViewMode::Grid);
                        }
                    },
                }
                if has_search {
                    button {
                        class: "btn btn-icon",
                        style: "position: absolute; right: 80px; top: 50%; transform: translateY(-50%); width: 20px; height: 20px; font-size: 11px; padding: 0; border-radius: 50%; background: rgba(255,255,255,0.1); color: var(--text-muted); border: none; cursor: pointer;",
                        title: translate(locale, "header.clear_search"),
                        onclick: move |_| search_query.set(String::new()),
                        "✕"
                    }
                }
                button {
                    class: "btn btn-icon",
                    style: "position: absolute; right: 44px; top: 50%; transform: translateY(-50%); width: auto; height: 24px; font-size: 11px; padding: 0 6px; border-radius: 5px; background: rgba(251, 191, 36, 0.12); color: #fbbf24; border: 1px solid rgba(251, 191, 36, 0.3); cursor: pointer;",
                    title: translate(locale, "header.content_search_tooltip"),
                    onclick: move |_| on_open_content_search.call(()),
                    "🔎"
                }
                button {
                    class: "btn btn-icon",
                    style: "position: absolute; right: 8px; top: 50%; transform: translateY(-50%); width: auto; height: 24px; font-size: 11px; padding: 0 7px; border-radius: 5px; background: rgba(255,255,255,0.08); color: var(--text-muted); border: 1px solid var(--border-subtle); cursor: pointer;",
                    title: translate(locale, "header.command_palette"),
                    onclick: move |_| on_open_palette.call(()),
                    "⌘K"
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
                        span { if is_syncing { "⏳" } else { "☁️" } }
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
                        span { "🧰" }
                        span { "{translate(locale, \"header.tools\")} ▾" }
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
                    span { "＋" }
                    span { "{translate(locale, \"action.import\")}" }
                }
                button {
                    class: "btn btn-icon",
                    title: translate(locale, "header.settings_tooltip"),
                    onclick: move |_| on_open_settings.call(()),
                    "⚙"
                }
            }
        }
    }
}
