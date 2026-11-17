//! Modale de paramètres et diagnostics de Vellum.

mod extras_section;
mod smtp_section;

use dioxus::prelude::*;
use vellum_core::{translate, translate_fmt, CatalogFormat, Locale};
use vellum_domain::{
    CloudSyncConfig, CloudSyncMode, CustomColumnDefinition, CustomColumnType, PluginDescriptor,
    SmtpConfig,
};

use extras_section::LibraryToolsSection;
use smtp_section::SmtpSettingsSection;

#[component]
pub fn SettingsModal(
    locale: Locale,
    on_change_locale: EventHandler<Locale>,
    opds_port: u16,
    opds_running: bool,
    opds_enabled: bool,
    on_toggle_opds: EventHandler<bool>,
    litestream_status_text: String,
    plugins: Vec<PluginDescriptor>,
    sync_config: CloudSyncConfig,
    sync_available: bool,
    is_syncing: bool,
    sync_message: Option<String>,
    smtp_config: SmtpConfig,
    is_testing_smtp: bool,
    smtp_test_message: Option<(bool, String)>,
    custom_columns: Vec<CustomColumnDefinition>,
    on_create_custom_column: EventHandler<(String, String, CustomColumnType)>,
    on_delete_custom_column: EventHandler<String>,
    watch_folder_path: String,
    on_scan_watch_folder: EventHandler<String>,
    on_export_catalog: EventHandler<CatalogFormat>,
    on_sync_now: EventHandler<()>,
    on_select_folder: EventHandler<()>,
    on_auto_detect_folder: EventHandler<()>,
    on_toggle_auto_download: EventHandler<bool>,
    on_toggle_enabled: EventHandler<bool>,
    on_set_sync_mode: EventHandler<CloudSyncMode>,
    on_save_gdrive_creds: EventHandler<(String, String, String)>,
    on_connect_gdrive: EventHandler<()>,
    on_disconnect_gdrive: EventHandler<()>,
    on_save_smtp: EventHandler<SmtpConfig>,
    on_test_smtp: EventHandler<SmtpConfig>,
    on_close: EventHandler<()>,
) -> Element {
    let opds_url = format!("http://localhost:{opds_port}/opds");
    let folder_display = sync_config
        .sync_folder_path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| translate(locale, "settings.no_folder").to_string());

    let (sync_badge_text, sync_badge_style) = if sync_available && sync_config.enabled {
        (
            translate(locale, "settings.status_active"),
            "background: rgba(16, 185, 129, 0.2); color: #34d399; border-color: rgba(16, 185, 129, 0.4);",
        )
    } else if sync_available && !sync_config.enabled {
        (
            translate(locale, "settings.status_disabled"),
            "background: rgba(148, 163, 184, 0.2); color: #94a3b8; border-color: rgba(148, 163, 184, 0.3);",
        )
    } else if sync_config.sync_folder_path.is_some() && !sync_available {
        (
            translate(locale, "settings.status_folder_missing"),
            "background: rgba(245, 158, 11, 0.2); color: #fbbf24; border-color: rgba(245, 158, 11, 0.4);",
        )
    } else {
        (
            translate(locale, "settings.status_unconfigured"),
            "background: rgba(239, 68, 68, 0.2); color: #f87171; border-color: rgba(239, 68, 68, 0.3);",
        )
    };

    let last_sync_text = sync_config
        .last_synced_at
        .as_deref()
        .unwrap_or_else(|| translate(locale, "settings.never_synced"));

    let mut gdrive_client_id =
        use_signal(|| sync_config.google_client_id.clone().unwrap_or_default());
    let mut gdrive_client_secret =
        use_signal(|| sync_config.google_client_secret.clone().unwrap_or_default());
    let mut gdrive_folder_name = use_signal(|| sync_config.google_drive_folder_name.clone());
    let is_api_mode = sync_config.mode == CloudSyncMode::GoogleDriveApi;
    let connected_label = match sync_config.google_account_email.as_deref() {
        Some(email) => translate_fmt(locale, "settings.gdrive_connected_as", &[("email", email)]),
        None => translate(locale, "settings.gdrive_connected_anon").to_string(),
    };

    rsx! {
        div {
            class: "drawer-backdrop",
            onclick: move |_| on_close.call(()),

            div {
                class: "drawer-panel",
                style: "width: 660px; max-height: 90vh; overflow-y: auto;",
                onclick: move |evt| evt.stop_propagation(),

                div { class: "drawer-header",
                    h3 { style: "font-size: 16px; font-weight: 700;", "{translate(locale, \"settings.title\")}" }
                    button {
                        class: "btn btn-icon",
                        onclick: move |_| on_close.call(()),
                        "✕"
                    }
                }

                // 0. Langue de l'interface (i18n)
                div { style: "display: flex; justify-content: space-between; align-items: center; padding: 12px 16px; background: rgba(255, 255, 255, 0.03); border: 1px solid var(--border-subtle); border-radius: 10px;",
                    span { style: "font-size: 13px; font-weight: 600;", "{translate(locale, \"settings.language\")}" }
                    div { style: "display: flex; gap: 8px;",
                        for loc in Locale::all().iter().copied() {
                            button {
                                class: if locale == loc { "btn btn-primary" } else { "btn btn-secondary" },
                                style: "font-size: 12px; padding: 4px 12px;",
                                onclick: move |_| on_change_locale.call(loc),
                                "{loc.display_name()}"
                            }
                        }
                    }
                }

                // 1. Synchronisation Google Drive Multi-Postes
                div { style: "display: flex; flex-direction: column; gap: 12px; padding: 16px; background: rgba(59, 130, 246, 0.04); border: 1px solid rgba(59, 130, 246, 0.2); border-radius: 12px;",
                    div { style: "display: flex; justify-content: space-between; align-items: center;",
                        div { style: "display: flex; align-items: center; gap: 8px;",
                            span { style: "font-size: 16px;", "☁️" }
                            span { style: "font-weight: 700; font-size: 14px; color: #93c5fd;", {translate(locale, "settings.sync_title")} }
                        }
                        span {
                            class: "format-pill",
                            style: "{sync_badge_style}",
                            "{sync_badge_text}"
                        }
                    }

                    p { style: "font-size: 12px; color: var(--text-secondary); line-height: 1.5;",
                        {translate(locale, "settings.sync_desc")}
                    }

                    // Mode de connexion Google Drive (API directe vs Dossier local)
                    div { style: "display: flex; flex-direction: column; gap: 6px;",
                        span { style: "font-size: 12px; font-weight: 600; color: var(--text-secondary);", {translate(locale, "settings.sync_mode_label")} }
                        div { style: "display: flex; gap: 8px;",
                            button {
                                class: if is_api_mode { "btn btn-primary" } else { "btn btn-secondary" },
                                style: "flex: 1; font-size: 11px; padding: 6px 10px;",
                                onclick: move |_| on_set_sync_mode.call(CloudSyncMode::GoogleDriveApi),
                                {translate(locale, "settings.sync_mode_api")}
                            }
                            button {
                                class: if !is_api_mode { "btn btn-primary" } else { "btn btn-secondary" },
                                style: "flex: 1; font-size: 11px; padding: 6px 10px;",
                                onclick: move |_| on_set_sync_mode.call(CloudSyncMode::GoogleDriveFolder),
                                {translate(locale, "settings.sync_mode_folder")}
                            }
                        }
                    }

                    if is_api_mode {
                        div { style: "display: flex; flex-direction: column; gap: 8px; padding: 10px 12px; background: rgba(0,0,0,0.25); border-radius: 8px; border: 1px solid var(--border-subtle);",
                            div { style: "display: flex; flex-direction: column; gap: 4px;",
                                span { style: "font-size: 11px; font-weight: 600; color: var(--text-secondary);", {translate(locale, "settings.gdrive_client_id")} }
                                input {
                                    class: "input",
                                    style: "font-size: 11px; font-family: monospace;",
                                    placeholder: "123456789-xxxx.apps.googleusercontent.com",
                                    value: "{gdrive_client_id}",
                                    oninput: move |evt| gdrive_client_id.set(evt.value()),
                                }
                            }
                            div { style: "display: flex; gap: 8px;",
                                div { style: "flex: 1; display: flex; flex-direction: column; gap: 4px;",
                                    span { style: "font-size: 11px; font-weight: 600; color: var(--text-secondary);", {translate(locale, "settings.gdrive_client_secret")} }
                                    input {
                                        class: "input",
                                        r#type: "password",
                                        style: "font-size: 11px; font-family: monospace;",
                                        placeholder: "GOCSPX-...",
                                        value: "{gdrive_client_secret}",
                                        oninput: move |evt| gdrive_client_secret.set(evt.value()),
                                    }
                                }
                                div { style: "width: 160px; display: flex; flex-direction: column; gap: 4px;",
                                    span { style: "font-size: 11px; font-weight: 600; color: var(--text-secondary);", {translate(locale, "settings.gdrive_folder_name")} }
                                    input {
                                        class: "input",
                                        style: "font-size: 11px;",
                                        placeholder: "Vellum",
                                        value: "{gdrive_folder_name}",
                                        oninput: move |evt| gdrive_folder_name.set(evt.value()),
                                    }
                                }
                            }
                            div { style: "display: flex; align-items: center; justify-content: space-between; gap: 8px; margin-top: 2px;",
                                div { style: "display: flex; align-items: center; gap: 8px;",
                                    button {
                                        class: "btn btn-secondary",
                                        style: "font-size: 11px; padding: 6px 10px;",
                                        onclick: move |_| {
                                            on_save_gdrive_creds.call((
                                                gdrive_client_id.read().clone(),
                                                gdrive_client_secret.read().clone(),
                                                gdrive_folder_name.read().clone(),
                                            ));
                                        },
                                        {translate(locale, "settings.gdrive_save_creds")}
                                    }
                                    button {
                                        class: "btn btn-primary",
                                        style: "font-size: 11px; padding: 6px 12px;",
                                        disabled: gdrive_client_id.read().trim().is_empty(),
                                        onclick: move |_| {
                                            on_save_gdrive_creds.call((
                                                gdrive_client_id.read().clone(),
                                                gdrive_client_secret.read().clone(),
                                                gdrive_folder_name.read().clone(),
                                            ));
                                            on_connect_gdrive.call(());
                                        },
                                        {translate(locale, "settings.gdrive_connect")}
                                    }
                                    if sync_available {
                                        button {
                                            class: "btn btn-secondary",
                                            style: "font-size: 11px; padding: 6px 10px; color: #f87171;",
                                            onclick: move |_| on_disconnect_gdrive.call(()),
                                            {translate(locale, "settings.gdrive_disconnect")}
                                        }
                                    }
                                }
                                if sync_available {
                                    span { style: "font-size: 11px; color: #34d399;", "{connected_label}" }
                                }
                            }
                            p { style: "font-size: 11px; color: var(--text-muted); line-height: 1.4; margin: 0;",
                                {translate(locale, "settings.gdrive_help")}
                            }
                        }
                    } else {
                        // Emplacement du dossier Drive local
                        div { style: "display: flex; flex-direction: column; gap: 6px;",
                            span { style: "font-size: 12px; font-weight: 600; color: var(--text-secondary);", {translate(locale, "settings.sync_folder_label")} }
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
                                    {translate(locale, "settings.choose_folder")}
                                }
                                button {
                                    class: "btn btn-secondary",
                                    style: "font-size: 11px; padding: 6px 10px; white-space: nowrap;",
                                    title: translate(locale, "settings.auto_detect_tooltip"),
                                    onclick: move |_| on_auto_detect_folder.call(()),
                                    {translate(locale, "settings.auto_detect_btn")}
                                }
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
                            span { style: "font-weight: 500;", {translate(locale, "settings.sync_enable")} }
                        }

                        label { style: "display: flex; align-items: center; gap: 10px; cursor: pointer; font-size: 12px;",
                            input {
                                r#type: "checkbox",
                                checked: sync_config.auto_download_books,
                                onchange: move |evt| on_toggle_auto_download.call(evt.value() == "true"),
                            }
                            div { style: "display: flex; flex-direction: column;",
                                span { style: "font-weight: 500;", {translate(locale, "settings.sync_auto_download")} }
                                span { style: "font-size: 11px; color: var(--text-muted);", {translate(locale, "settings.sync_auto_download_desc")} }
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
                            span { {translate_fmt(locale, "settings.sync_client", &[("name", &sync_config.client_name), ("id", &sync_config.client_id)])} }
                            span { {translate_fmt(locale, "settings.sync_last", &[("date", last_sync_text)])} }
                        }
                        button {
                            class: "btn btn-primary",
                            style: "font-size: 12px; padding: 6px 14px; gap: 6px;",
                            disabled: is_syncing || !sync_available,
                            onclick: move |_| on_sync_now.call(()),
                            span { if is_syncing { "⏳" } else { "🔄" } }
                            span { {if is_syncing { translate(locale, "settings.sync_in_progress") } else { translate(locale, "settings.sync_now") }} }
                        }
                    }
                }

                // 2. Envoi sans-fil Kindle (Send-to-Kindle via SMTP)
                SmtpSettingsSection {
                    locale,
                    smtp_config,
                    is_testing_smtp,
                    smtp_test_message,
                    on_save_smtp,
                    on_test_smtp,
                }

                // 3. Serveur OPDS
                div { style: "display: flex; flex-direction: column; gap: 8px; padding: 14px; background: rgba(255,255,255,0.03); border: 1px solid var(--border-subtle); border-radius: 10px;",
                    div { style: "display: flex; justify-content: space-between; align-items: center;",
                        span { style: "font-weight: 600; font-size: 13px;", {translate(locale, "settings.opds_title")} }
                        span {
                            class: "format-pill",
                            style: if opds_running { "background: rgba(16, 185, 129, 0.2); color: #34d399; border-color: rgba(16, 185, 129, 0.4);" } else { "background: rgba(239, 68, 68, 0.2); color: #f87171;" },
                            {if opds_running { translate(locale, "settings.status_active") } else { translate(locale, "settings.status_stopped") }}
                        }
                    }
                    p { style: "font-size: 12px; color: var(--text-secondary);",
                        {translate(locale, "settings.opds_desc")}
                    }
                    div { style: "display: flex; align-items: center; justify-content: space-between; font-size: 12px; background: rgba(0,0,0,0.3); padding: 8px 12px; border-radius: 6px; font-family: monospace;",
                        span { "{opds_url}" }
                        span { style: "color: var(--accent-primary);", "Port {opds_port}" }
                    }
                    label { style: "display: flex; align-items: center; gap: 10px; cursor: pointer; font-size: 12px; margin-top: 2px;",
                        input {
                            r#type: "checkbox",
                            checked: opds_enabled,
                            onchange: move |evt| on_toggle_opds.call(evt.value() == "true"),
                        }
                        span { style: "font-weight: 500;", {translate(locale, "settings.opds_enable")} }
                        span { style: "font-size: 11px; color: var(--text-muted);", {translate(locale, "settings.opds_restart_hint")} }
                    }
                }

                // 4. Litestream
                div { style: "display: flex; flex-direction: column; gap: 8px; padding: 14px; background: rgba(255,255,255,0.03); border: 1px solid var(--border-subtle); border-radius: 10px;",
                    div { style: "display: flex; justify-content: space-between; align-items: center;",
                        span { style: "font-weight: 600; font-size: 13px;", {translate(locale, "settings.litestream_title")} }
                        span { class: "format-pill", "WAL STREAM" }
                    }
                    p { style: "font-size: 12px; color: var(--text-secondary);",
                        {translate(locale, "settings.litestream_desc")}
                    }
                    div { style: "font-size: 12px; color: var(--text-muted);",
                        {translate_fmt(locale, "settings.litestream_status", &[("status", &litestream_status_text)])}
                    }
                }

                LibraryToolsSection {
                    locale,
                    custom_columns,
                    on_create_custom_column,
                    on_delete_custom_column,
                    watch_folder_path,
                    on_scan_watch_folder,
                    on_export_catalog,
                }

                // 5. Plugins
                div { style: "display: flex; flex-direction: column; gap: 8px;",
                    div {
                        class: "sidebar-section-title",
                        {
                            let p_count = plugins.len().to_string();
                            translate_fmt(locale, "settings.plugins_title", &[("count", &p_count)])
                        }
                    }
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
