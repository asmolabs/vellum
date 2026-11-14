//! Section des paramètres d'envoi sans-fil Kindle via SMTP.

use dioxus::prelude::*;
use vellum_domain::SmtpConfig;

#[component]
pub(super) fn SmtpSettingsSection(
    smtp_config: SmtpConfig,
    is_testing_smtp: bool,
    smtp_test_message: Option<(bool, String)>,
    on_save_smtp: EventHandler<SmtpConfig>,
    on_test_smtp: EventHandler<SmtpConfig>,
) -> Element {
    let mut smtp_enabled = use_signal(|| smtp_config.enabled);
    let mut smtp_host = use_signal(|| smtp_config.host.clone());
    let mut smtp_port = use_signal(|| smtp_config.port.to_string());
    let mut smtp_user = use_signal(|| smtp_config.username.clone());
    let mut smtp_pass = use_signal(|| smtp_config.password.clone());
    let mut smtp_from = use_signal(|| smtp_config.from_address.clone());
    let mut smtp_kindle = use_signal(|| smtp_config.kindle_email.clone());

    let (smtp_badge_text, smtp_badge_style) = if *smtp_enabled.read()
        && !smtp_kindle.read().is_empty()
    {
        (
            "CONFIGURÉ",
            "background: rgba(16, 185, 129, 0.2); color: #34d399; border-color: rgba(16, 185, 129, 0.4);",
        )
    } else if *smtp_enabled.read() {
        (
            "INCOMPLET",
            "background: rgba(245, 158, 11, 0.2); color: #fbbf24; border-color: rgba(245, 158, 11, 0.4);",
        )
    } else {
        (
            "DÉSACTIVÉ",
            "background: rgba(148, 163, 184, 0.2); color: #94a3b8; border-color: rgba(148, 163, 184, 0.3);",
        )
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
    }
}
