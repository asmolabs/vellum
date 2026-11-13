use dioxus::prelude::*;
use vellum_core::{translate, CatalogFormat, Locale};
use vellum_domain::{
    CloudSyncConfig, CustomColumnDefinition, CustomColumnType, PluginDescriptor, SmtpConfig,
};

#[component]
pub fn SettingsModal(
    locale: Locale,
    on_change_locale: EventHandler<Locale>,
    opds_port: u16,
    opds_running: bool,
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
    on_save_smtp: EventHandler<SmtpConfig>,
    on_test_smtp: EventHandler<SmtpConfig>,
    on_close: EventHandler<()>,
) -> Element {
    let mut new_col_key = use_signal(String::new);
    let mut new_col_label = use_signal(String::new);
    let mut new_col_type = use_signal(|| CustomColumnType::Text);
    let mut watch_dir_input = use_signal(|| watch_folder_path.clone());
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

    // Local form state for SMTP
    let mut smtp_enabled = use_signal(|| smtp_config.enabled);
    let mut smtp_host = use_signal(|| smtp_config.host.clone());
    let mut smtp_port = use_signal(|| smtp_config.port.to_string());
    let mut smtp_user = use_signal(|| smtp_config.username.clone());
    let mut smtp_pass = use_signal(|| smtp_config.password.clone());
    let mut smtp_from = use_signal(|| smtp_config.from_address.clone());
    let mut smtp_kindle = use_signal(|| smtp_config.kindle_email.clone());

    let (smtp_badge_text, smtp_badge_style) = if *smtp_enabled.read() && !smtp_kindle.read().is_empty() {
        ("CONFIGURÉ", "background: rgba(16, 185, 129, 0.2); color: #34d399; border-color: rgba(16, 185, 129, 0.4);")
    } else if *smtp_enabled.read() {
        ("INCOMPLET", "background: rgba(245, 158, 11, 0.2); color: #fbbf24; border-color: rgba(245, 158, 11, 0.4);")
    } else {
        ("DÉSACTIVÉ", "background: rgba(148, 163, 184, 0.2); color: #94a3b8; border-color: rgba(148, 163, 184, 0.3);")
    };

    let build_current_smtp = move || {
        let p: u16 = smtp_port.read().trim().parse().unwrap_or(587);
        SmtpConfig {
            enabled: *smtp_enabled.read(),
            host: smtp_host.read().trim().to_string(),
            port: p,
            username: smtp_user.read().trim().to_string(),
            password: smtp_pass.read().clone(),
            from_address: smtp_from.read().trim().to_string(),
            kindle_email: smtp_kindle.read().trim().to_string(),
        }
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
                    h3 { style: "font-size: 16px; font-weight: 700;", "Paramètres & Diagnostics Vellum" }
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

                // 2. Envoi sans-fil Kindle (Send-to-Kindle via SMTP)
                div { style: "display: flex; flex-direction: column; gap: 12px; padding: 16px; background: rgba(245, 158, 11, 0.04); border: 1px solid rgba(245, 158, 11, 0.2); border-radius: 12px;",
                    div { style: "display: flex; justify-content: space-between; align-items: center;",
                        div { style: "display: flex; align-items: center; gap: 8px;",
                            span { style: "font-size: 16px;", "✉️" }
                            span { style: "font-weight: 700; font-size: 14px; color: #fcd34d;", "Envoi sans-fil Kindle (Send-to-Kindle SMTP)" }
                        }
                        span {
                            class: "format-pill",
                            style: "{smtp_badge_style}",
                            "{smtp_badge_text}"
                        }
                    }

                    p { style: "font-size: 12px; color: var(--text-secondary); line-height: 1.5;",
                        "Permet d'envoyer vos livres EPUB directement vers votre liseuse Amazon Kindle sans câble USB en passant par votre compte e-mail."
                    }

                    label { style: "display: flex; align-items: center; gap: 10px; cursor: pointer; font-size: 12px;",
                        input {
                            r#type: "checkbox",
                            checked: *smtp_enabled.read(),
                            onchange: move |e| smtp_enabled.set(e.value() == "true"),
                        }
                        span { style: "font-weight: 600;", "Activer l'envoi sans-fil vers Kindle" }
                    }

                    if *smtp_enabled.read() {
                        div { style: "display: flex; flex-direction: column; gap: 10px; margin-top: 4px;",
                            div { style: "display: grid; grid-template-columns: 2fr 1fr; gap: 10px;",
                                div { class: "drawer-form-group",
                                    label { class: "drawer-form-label", "Serveur SMTP :" }
                                    input {
                                        class: "drawer-input",
                                        placeholder: "smtp.gmail.com",
                                        value: "{smtp_host}",
                                        oninput: move |e| smtp_host.set(e.value()),
                                    }
                                }
                                div { class: "drawer-form-group",
                                    label { class: "drawer-form-label", "Port (587/465) :" }
                                    input {
                                        class: "drawer-input",
                                        placeholder: "587",
                                        value: "{smtp_port}",
                                        oninput: move |e| smtp_port.set(e.value()),
                                    }
                                }
                            }

                            div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 10px;",
                                div { class: "drawer-form-group",
                                    label { class: "drawer-form-label", "Nom d'utilisateur / Email :" }
                                    input {
                                        class: "drawer-input",
                                        placeholder: "mon.email@gmail.com",
                                        value: "{smtp_user}",
                                        oninput: move |e| smtp_user.set(e.value()),
                                    }
                                }
                                div { class: "drawer-form-group",
                                    label { class: "drawer-form-label", "Mot de passe d'application :" }
                                    input {
                                        class: "drawer-input",
                                        r#type: "password",
                                        placeholder: "Mot de passe SMTP...",
                                        value: "{smtp_pass}",
                                        oninput: move |e| smtp_pass.set(e.value()),
                                    }
                                }
                            }

                            div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 10px;",
                                div { class: "drawer-form-group",
                                    label { class: "drawer-form-label", "Expéditeur approuvé (From) :" }
                                    input {
                                        class: "drawer-input",
                                        placeholder: "mon.email@gmail.com",
                                        value: "{smtp_from}",
                                        oninput: move |e| smtp_from.set(e.value()),
                                    }
                                }
                                div { class: "drawer-form-group",
                                    label { class: "drawer-form-label", "Adresse Kindle destinataire :" }
                                    input {
                                        class: "drawer-input",
                                        placeholder: "ma_liseuse@kindle.com",
                                        value: "{smtp_kindle}",
                                        oninput: move |e| smtp_kindle.set(e.value()),
                                    }
                                }
                            }

                            if let Some((ok, msg)) = smtp_test_message.as_ref() {
                                div {
                                    style: if *ok {
                                        "font-size: 12px; padding: 8px 12px; border-radius: 6px; background: rgba(16, 185, 129, 0.15); border: 1px solid rgba(16, 185, 129, 0.3); color: #6ee7b7;"
                                    } else {
                                        "font-size: 12px; padding: 8px 12px; border-radius: 6px; background: rgba(239, 68, 68, 0.15); border: 1px solid rgba(239, 68, 68, 0.3); color: #fca5a5;"
                                    },
                                    "{msg}"
                                }
                            }

                            div { style: "display: flex; justify-content: flex-end; gap: 8px; margin-top: 6px;",
                                button {
                                    class: "btn btn-secondary",
                                    style: "font-size: 12px; padding: 6px 12px;",
                                    disabled: is_testing_smtp,
                                    onclick: {
                                        let build = build_current_smtp;
                                        move |_| on_test_smtp.call(build())
                                    },
                                    if is_testing_smtp { "⏳ Test en cours..." } else { "🔍 Tester la connexion" }
                                }
                                button {
                                    class: "btn btn-primary",
                                    style: "font-size: 12px; padding: 6px 14px;",
                                    onclick: {
                                        let build = build_current_smtp;
                                        move |_| on_save_smtp.call(build())
                                    },
                                    "💾 Enregistrer SMTP"
                                }
                            }
                        }
                    }
                }

                // 3. Serveur OPDS
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

                // 4. Litestream
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

                // 4b. Watch Folder (Auto-Add)
                div { style: "display: flex; flex-direction: column; gap: 8px; padding: 14px; background: rgba(255,255,255,0.03); border: 1px solid var(--border-subtle); border-radius: 10px;",
                    div { style: "font-weight: 600; font-size: 13px;", "📥 Watch Folder (Auto-Add)" }
                    p { style: "font-size: 12px; color: var(--text-secondary);",
                        "Importe automatiquement les nouveaux ebooks déposés dans un dossier et évite les doublons."
                    }
                    div { style: "display: flex; gap: 8px;",
                        input {
                            class: "search-input",
                            style: "flex: 1; font-size: 12px;",
                            r#type: "text",
                            placeholder: "/chemin/vers/dossier-a-importer",
                            value: "{watch_dir_input}",
                            oninput: move |e| watch_dir_input.set(e.value()),
                        }
                        button {
                            class: "btn btn-primary",
                            style: "font-size: 12px;",
                            onclick: move |_| {
                                let dir = watch_dir_input.read().trim().to_string();
                                if !dir.is_empty() {
                                    on_scan_watch_folder.call(dir);
                                }
                            },
                            "Scanner maintenant"
                        }
                    }
                }

                // 4c. Colonnes personnalisées
                div { style: "display: flex; flex-direction: column; gap: 8px; padding: 14px; background: rgba(255,255,255,0.03); border: 1px solid var(--border-subtle); border-radius: 10px;",
                    div { style: "font-weight: 600; font-size: 13px;", "🏷️ Colonnes personnalisées ({custom_columns.len()})" }
                    if !custom_columns.is_empty() {
                        div { style: "display: flex; flex-direction: column; gap: 4px;",
                            for col in custom_columns.iter() {
                                {
                                    let col_id = col.id.clone();
                                    rsx! {
                                        div {
                                            style: "display: flex; justify-content: space-between; align-items: center; padding: 6px 10px; background: rgba(0,0,0,0.2); border-radius: 6px; font-size: 12px;",
                                            span { "{col.label} ({col.key}) • {col.column_type.as_str()}" }
                                            button {
                                                style: "background: none; border: none; color: #f87171; cursor: pointer; font-weight: 700;",
                                                onclick: move |_| on_delete_custom_column.call(col_id.clone()),
                                                "×"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    div { style: "display: flex; gap: 6px; flex-wrap: wrap;",
                        input {
                            class: "search-input",
                            style: "flex: 1; min-width: 110px; font-size: 12px;",
                            r#type: "text",
                            placeholder: "Clé (ex: pages)",
                            value: "{new_col_key}",
                            oninput: move |e| new_col_key.set(e.value()),
                        }
                        input {
                            class: "search-input",
                            style: "flex: 1; min-width: 130px; font-size: 12px;",
                            r#type: "text",
                            placeholder: "Libellé (ex: Pages)",
                            value: "{new_col_label}",
                            oninput: move |e| new_col_label.set(e.value()),
                        }
                        select {
                            class: "sort-select",
                            style: "font-size: 12px;",
                            onchange: move |e| {
                                if let Some(t) = CustomColumnType::parse(&e.value()) {
                                    new_col_type.set(t);
                                }
                            },
                            option { value: "text", "Texte" }
                            option { value: "number", "Nombre" }
                            option { value: "boolean", "Booléen" }
                            option { value: "date", "Date" }
                            option { value: "rating", "Note (0-5)" }
                        }
                        button {
                            class: "btn btn-primary",
                            style: "font-size: 12px;",
                            onclick: move |_| {
                                let k = new_col_key.read().trim().to_string();
                                let l = new_col_label.read().trim().to_string();
                                let t = *new_col_type.read();
                                if !k.is_empty() && !l.is_empty() {
                                    on_create_custom_column.call((k, l, t));
                                    new_col_key.set(String::new());
                                    new_col_label.set(String::new());
                                }
                            },
                            "+ Ajouter"
                        }
                    }
                }

                // 4d. Export du catalogue
                div { style: "display: flex; flex-direction: column; gap: 8px; padding: 14px; background: rgba(255,255,255,0.03); border: 1px solid var(--border-subtle); border-radius: 10px;",
                    div { style: "font-weight: 600; font-size: 13px;", "📚 Export du catalogue" }
                    p { style: "font-size: 12px; color: var(--text-secondary);",
                        "Exportez l'ensemble de votre bibliothèque au format CSV, JSON ou BibTeX."
                    }
                    div { style: "display: flex; gap: 8px; flex-wrap: wrap;",
                        button {
                            class: "btn btn-secondary",
                            style: "font-size: 12px;",
                            onclick: move |_| on_export_catalog.call(CatalogFormat::Csv),
                            "📄 Exporter CSV"
                        }
                        button {
                            class: "btn btn-secondary",
                            style: "font-size: 12px;",
                            onclick: move |_| on_export_catalog.call(CatalogFormat::Json),
                            "🧾 Exporter JSON"
                        }
                        button {
                            class: "btn btn-secondary",
                            style: "font-size: 12px;",
                            onclick: move |_| on_export_catalog.call(CatalogFormat::Bibtex),
                            "🎓 Exporter BibTeX"
                        }
                    }
                }

                // 5. Plugins
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
