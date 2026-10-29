use dioxus::prelude::*;
use vellum_core::ConnectedDevice;
use vellum_domain::DeviceKind;

#[component]
pub fn Header(
    search_query: Signal<String>,
    connected_devices: Vec<ConnectedDevice>,
    on_import: EventHandler<()>,
    on_sync_device: EventHandler<DeviceKind>,
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
                    placeholder: "Rechercher par titre, auteur, tag...",
                    value: "{search_query}",
                    oninput: move |evt| search_query.set(evt.value()),
                }
            }

            div { class: "header-actions",
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

