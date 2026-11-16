use dioxus::prelude::*;
use vellum_core::{translate, Locale};
use vellum_domain::Book;

#[derive(Clone, Debug)]
pub struct TranslationRequest {
    pub book_id: String,
    pub target_lang: String,
    pub provider: String,
    pub model: String,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub bilingual_mode: bool,
}

#[component]
pub fn TranslateModal(
    locale: Locale,
    book: Book,
    on_close: EventHandler<()>,
    on_start_translate: EventHandler<TranslationRequest>,
    is_translating: bool,
) -> Element {
    let book_title = book.title.clone();
    let mut target_lang = use_signal(|| "Français".to_string());
    let mut provider = use_signal(|| "Ollama (Local - Gratuit)".to_string());
    let mut model = use_signal(|| "qwen2.5:14b".to_string());
    let mut custom_url = use_signal(|| "http://localhost:11434".to_string());
    let mut api_key = use_signal(String::new);
    let mut bilingual_mode = use_signal(|| false);

    let is_ollama = provider.read().contains("Ollama");
    let is_openai = provider.read().contains("OpenAI");

    rsx! {
        div {
            class: "drawer-backdrop",
            onclick: move |_| {
                if !is_translating {
                    on_close.call(());
                }
            },

            div {
                class: "drawer-panel",
                style: "width: 580px; max-height: 90vh; overflow-y: auto;",
                onclick: move |e| e.stop_propagation(),

                div { class: "drawer-header",
                    div { style: "display: flex; flex-direction: column;",
                        span { style: "font-weight: 700; font-size: 16px;", {translate(locale, "translate.title")} }
                        span { style: "font-size: 11px; color: var(--text-muted);", "{book_title}" }
                    }
                    if !is_translating {
                        button {
                            class: "btn btn-icon",
                            onclick: move |_| on_close.call(()),
                            "✕"
                        }
                    }
                }

                div { style: "display: flex; flex-direction: column; gap: 16px; padding: 4px 0;",
                    // Info Banner
                    div { style: "background: rgba(99, 102, 241, 0.08); border: 1px solid rgba(99, 102, 241, 0.25); border-radius: 8px; padding: 12px 14px; font-size: 12px; line-height: 1.5; color: var(--text-secondary);",
                        {translate(locale, "translate.banner")}
                    }

                    // 1. Langue cible
                    div { style: "display: flex; flex-direction: column; gap: 6px;",
                        label { style: "font-size: 12px; font-weight: 600; color: var(--text-muted);", {translate(locale, "translate.target_lang")} }
                        select {
                            style: "padding: 8px 12px; background: rgba(255,255,255,0.05); border: 1px solid var(--border-subtle); border-radius: 6px; color: var(--text-primary); font-size: 13px; outline: none;",
                            value: "{target_lang}",
                            disabled: is_translating,
                            onchange: move |e| target_lang.set(e.value()),
                            option { value: "Français", "🇫🇷 Français" }
                            option { value: "Anglais", "🇬🇧 English" }
                            option { value: "Espagnol", "🇪🇸 Español" }
                            option { value: "Allemand", "🇩🇪 Deutsch" }
                            option { value: "Italien", "🇮🇹 Italiano" }
                            option { value: "Japonais", "🇯🇵 日本語" }
                            option { value: "Portugais", "🇵🇹 Português" }
                        }
                    }

                    // 2. Fournisseur d'IA
                    div { style: "display: flex; flex-direction: column; gap: 6px;",
                        label { style: "font-size: 12px; font-weight: 600; color: var(--text-muted);", {translate(locale, "translate.provider")} }
                        select {
                            style: "padding: 8px 12px; background: rgba(255,255,255,0.05); border: 1px solid var(--border-subtle); border-radius: 6px; color: var(--text-primary); font-size: 13px; outline: none;",
                            value: "{provider}",
                            disabled: is_translating,
                            onchange: move |e| {
                                let val = e.value();
                                provider.set(val.clone());
                                if val.contains("Ollama") {
                                    custom_url.set("http://localhost:11434".to_string());
                                    model.set("qwen2.5:14b".to_string());
                                } else if val.contains("OpenAI") {
                                    custom_url.set("https://api.openai.com/v1".to_string());
                                    model.set("gpt-4o-mini".to_string());
                                }
                            },
                            option { value: "Ollama (Local - Gratuit)", {translate(locale, "translate.provider_ollama")} }
                            option { value: "OpenAI (API)", {translate(locale, "translate.provider_openai")} }
                            option { value: "Autre serveur compatible OpenAI", {translate(locale, "translate.provider_other")} }
                        }
                    }

                    // 3. Modèle et Endpoint
                    div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                        div { style: "display: flex; flex-direction: column; gap: 6px;",
                            label { style: "font-size: 12px; font-weight: 600; color: var(--text-muted);", {translate(locale, "translate.model")} }
                            input {
                                style: "padding: 8px 12px; background: rgba(255,255,255,0.05); border: 1px solid var(--border-subtle); border-radius: 6px; color: var(--text-primary); font-size: 13px; outline: none;",
                                value: "{model}",
                                disabled: is_translating,
                                oninput: move |e| model.set(e.value()),
                            }
                        }
                        div { style: "display: flex; flex-direction: column; gap: 6px;",
                            label { style: "font-size: 12px; font-weight: 600; color: var(--text-muted);", {translate(locale, "translate.api_url")} }
                            input {
                                style: "padding: 8px 12px; background: rgba(255,255,255,0.05); border: 1px solid var(--border-subtle); border-radius: 6px; color: var(--text-primary); font-size: 13px; outline: none;",
                                value: "{custom_url}",
                                disabled: is_translating,
                                oninput: move |e| custom_url.set(e.value()),
                            }
                        }
                    }

                    // Clé API si OpenAI
                    if is_openai || !is_ollama {
                        div { style: "display: flex; flex-direction: column; gap: 6px;",
                            label { style: "font-size: 12px; font-weight: 600; color: var(--text-muted);", {translate(locale, "translate.api_key")} }
                            input {
                                style: "padding: 8px 12px; background: rgba(255,255,255,0.05); border: 1px solid var(--border-subtle); border-radius: 6px; color: var(--text-primary); font-size: 13px; outline: none;",
                                type: "password",
                                placeholder: "sk-...",
                                value: "{api_key}",
                                disabled: is_translating,
                                oninput: move |e| api_key.set(e.value()),
                            }
                        }
                    }

                    // 4. Mode bilingue
                    div { style: "display: flex; align-items: center; gap: 10px; padding: 10px 12px; background: rgba(255,255,255,0.03); border: 1px solid var(--border-subtle); border-radius: 8px; cursor: pointer;",
                        onclick: move |_| {
                            if !is_translating {
                                let cur = *bilingual_mode.read();
                                bilingual_mode.set(!cur);
                            }
                        },
                        input {
                            type: "checkbox",
                            checked: *bilingual_mode.read(),
                            disabled: is_translating,
                            style: "cursor: pointer;",
                        }
                        div { style: "display: flex; flex-direction: column;",
                            span { style: "font-size: 13px; font-weight: 500;", {translate(locale, "translate.bilingual_title")} }
                            span { style: "font-size: 11px; color: var(--text-muted);", {translate(locale, "translate.bilingual_desc")} }
                        }
                    }

                    // Statut de traduction
                    if is_translating {
                        div { style: "display: flex; flex-direction: column; gap: 8px; padding: 14px; background: rgba(99, 102, 241, 0.1); border: 1px solid var(--accent-primary); border-radius: 8px; align-items: center;",
                            div { style: "font-size: 13px; font-weight: 600; color: var(--accent-primary);", {translate(locale, "translate.in_progress")} }
                            div { style: "font-size: 11px; color: var(--text-secondary); text-align: center;",
                                {translate(locale, "translate.in_progress_desc")}
                            }
                        }
                    }
                }

                // Actions Footer
                div { style: "margin-top: 24px; padding-top: 16px; border-top: 1px solid var(--border-subtle); display: flex; justify-content: flex-end; gap: 10px;",
                    if !is_translating {
                        button {
                            class: "btn btn-secondary",
                            onclick: move |_| on_close.call(()),
                            {translate(locale, "action.cancel")}
                        }
                        button {
                            class: "btn btn-primary",
                            onclick: {
                                let b_id = book.id.clone();
                                move |_| {
                                    let req = TranslationRequest {
                                        book_id: b_id.clone(),
                                        target_lang: target_lang.read().clone(),
                                        provider: provider.read().clone(),
                                        model: model.read().clone(),
                                        base_url: Some(custom_url.read().clone()),
                                        api_key: if api_key.read().trim().is_empty() { None } else { Some(api_key.read().trim().to_string()) },
                                        bilingual_mode: *bilingual_mode.read(),
                                    };
                                    on_start_translate.call(req);
                                }
                            },
                            {translate(locale, "translate.start")}
                        }
                    } else {
                        span { style: "font-size: 12px; color: var(--text-muted); align-self: center;", {translate(locale, "translate.bg_running")} }
                    }
                }
            }
        }
    }
}
