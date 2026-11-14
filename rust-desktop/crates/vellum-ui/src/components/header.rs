use dioxus::prelude::*;
use vellum_core::{translate, ConnectedDevice, Locale};
use vellum_domain::DeviceKind;

#[component]
pub fn Header(
    locale: Locale,
    search_query: Signal<String>,
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
                    style: if has_search { "padding-right: 72px;" } else { "padding-right: 48px;" },
                    placeholder: "{translate(locale, \"action.search\")} (Dune, author:Herbert, tag:SF)...",
                    value: "{search_query}",
                    oninput: move |evt| search_query.set(evt.value()),
                }
                if has_search {
                    button {
                        class: "btn btn-icon",
                        style: "position: absolute; right: 44px; top: 50%; transform: translateY(-50%); width: 20px; height: 20px; font-size: 11px; padding: 0; border-radius: 50%; background: rgba(255,255,255,0.1); color: var(--text-muted); border: none; cursor: pointer;",
                        title: translate(locale, "header.clear_search"),
                        onclick: move |_| search_query.set(String::new()),
                        "✕"
                    }
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
                                title: "Synchroniser avec {name}",
                                onclick: move |_| on_sync_device.call(kind),
                                span { "{icon}" }
                                span { "{name}" }
                            }
                        }
                    }
                }

                button {
                    class: "btn btn-secondary",
                    style: "border-color: rgba(251, 191, 36, 0.35); color: #fbbf24;",
                    title: translate(locale, "header.content_search_tooltip"),
                    onclick: move |_| on_open_content_search.call(()),
                    span { "🔎" }
                    span { "{translate(locale, \"header.content_search\")}" }
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
