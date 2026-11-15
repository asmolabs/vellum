use dioxus::prelude::*;
use std::sync::Arc;
use vellum_core::{translate, translate_fmt, BookService, ImportRule, ImportRuleTarget, Locale};

#[component]
pub fn ImportRulesModal(
    locale: Locale,
    book_service: Arc<BookService>,
    on_close: EventHandler<()>,
) -> Element {
    let mut rules = use_signal(|| book_service.get_import_rules());
    let mut name = use_signal(String::new);
    let mut target = use_signal(|| ImportRuleTarget::Title);
    let mut match_regex = use_signal(String::new);
    let mut add_tags = use_signal(String::new);
    let mut set_series = use_signal(String::new);
    let mut auto_polish = use_signal(|| false);
    let mut status_msg = use_signal(|| None::<(String, bool)>);

    let persist = {
        let bs = book_service.clone();
        move |next: Vec<ImportRule>| match bs.save_import_rules(&next) {
            Ok(()) => {
                rules.set(next);
                status_msg.set(Some((translate(locale, "rules.saved").to_string(), true)));
            }
            Err(e) => status_msg.set(Some((
                translate_fmt(locale, "common.error", &[("error", &e.to_string())]),
                false,
            ))),
        }
    };

    let handle_add = {
        let mut p = persist.clone();
        move |_| {
            let n = name.read().trim().to_string();
            let pat = match_regex.read().trim().to_string();
            if n.is_empty() || pat.is_empty() {
                status_msg.set(Some((
                    translate(locale, "rules.missing_fields").to_string(),
                    false,
                )));
                return;
            }
            let mut rule = ImportRule::new(n, *target.read(), pat);
            rule.add_tags = add_tags
                .read()
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            let ser = set_series.read().trim().to_string();
            if !ser.is_empty() {
                rule.set_series = Some(ser);
            }
            rule.auto_polish = *auto_polish.read();

            let mut next = rules.read().clone();
            next.push(rule);
            p(next);

            name.set(String::new());
            match_regex.set(String::new());
            add_tags.set(String::new());
            set_series.set(String::new());
            auto_polish.set(false);
        }
    };

    rsx! {
        div {
            class: "drawer-backdrop",
            style: "align-items: center; justify-content: center;",
            onclick: move |_| on_close.call(()),
            div {
                class: "modal-card",
                style: "width: 740px; max-width: 94vw; max-height: 85vh; display: flex; flex-direction: column; padding: 24px; gap: 14px;",
                onclick: move |e| e.stop_propagation(),

                div {
                    style: "display: flex; align-items: center; justify-content: space-between;",
                    div {
                        h2 { style: "margin: 0; font-size: 18px;", {translate(locale, "rules.title")} }
                        p { style: "margin: 4px 0 0 0; font-size: 12px; color: var(--text-muted);", {translate(locale, "rules.subtitle")} }
                    }
                    button {
                        class: "btn btn-secondary",
                        onclick: move |_| on_close.call(()),
                        "✕"
                    }
                }

                div {
                    style: "padding: 12px; border-radius: 8px; background: rgba(255,255,255,0.03); border: 1px solid var(--border-color); display: flex; flex-direction: column; gap: 8px;",
                    div {
                        style: "display: grid; grid-template-columns: 1fr 140px 1fr; gap: 8px;",
                        input {
                            class: "form-input",
                            placeholder: translate(locale, "rules.name_ph"),
                            value: "{name}",
                            oninput: move |e| name.set(e.value()),
                        }
                        select {
                            class: "form-input",
                            onchange: move |e| {
                                let t = match e.value().as_str() {
                                    "author" => ImportRuleTarget::Author,
                                    "filename" => ImportRuleTarget::Filename,
                                    _ => ImportRuleTarget::Title,
                                };
                                target.set(t);
                            },
                            option { value: "title", selected: *target.read() == ImportRuleTarget::Title, {translate(locale, "table.col_title")} }
                            option { value: "author", selected: *target.read() == ImportRuleTarget::Author, {translate(locale, "table.col_author")} }
                            option { value: "filename", selected: *target.read() == ImportRuleTarget::Filename, {translate(locale, "rules.filename")} }
                        }
                        input {
                            class: "form-input",
                            placeholder: "Regex (ex: (?i)dune)",
                            value: "{match_regex}",
                            oninput: move |e| match_regex.set(e.value()),
                        }
                    }
                    div {
                        style: "display: grid; grid-template-columns: 1fr 1fr auto auto; gap: 8px; align-items: center;",
                        input {
                            class: "form-input",
                            placeholder: translate(locale, "rules.tags_ph"),
                            value: "{add_tags}",
                            oninput: move |e| add_tags.set(e.value()),
                        }
                        input {
                            class: "form-input",
                            placeholder: translate(locale, "rules.series_ph"),
                            value: "{set_series}",
                            oninput: move |e| set_series.set(e.value()),
                        }
                        label {
                            style: "display: flex; align-items: center; gap: 6px; font-size: 12px; cursor: pointer;",
                            input {
                                r#type: "checkbox",
                                checked: *auto_polish.read(),
                                onchange: move |e| auto_polish.set(e.checked()),
                            }
                            "Auto-Polish"
                        }
                        button {
                            class: "btn btn-primary",
                            onclick: handle_add,
                            {translate(locale, "settings.add_btn")}
                        }
                    }
                }

                if let Some((msg, ok)) = status_msg.read().as_ref() {
                    div {
                        style: if *ok {
                            "padding: 8px 12px; border-radius: 6px; font-size: 13px; background: rgba(34,197,94,0.12); color: #86efac; border: 1px solid rgba(34,197,94,0.3);"
                        } else {
                            "padding: 8px 12px; border-radius: 6px; font-size: 13px; background: rgba(239,68,68,0.12); color: #fca5a5; border: 1px solid rgba(239,68,68,0.3);"
                        },
                        "{msg}"
                    }
                }

                div {
                    style: "flex: 1; overflow-y: auto; display: flex; flex-direction: column; gap: 8px;",
                    if rules.read().is_empty() {
                        p { style: "color: var(--text-muted); font-size: 13px; text-align: center; padding: 20px;", {translate(locale, "rules.empty")} }
                    }
                    for rule in rules.read().iter() {
                        {
                            let id_toggle = rule.id.clone();
                            let id_delete = rule.id.clone();
                            let tags_str = if rule.add_tags.is_empty() { "—".to_string() } else { rule.add_tags.join(", ") };
                            let ser_str = rule.set_series.clone().unwrap_or_else(|| "—".to_string());
                            let mut p_toggle = persist.clone();
                            let mut p_delete = persist.clone();
                            rsx! {
                                div {
                                    style: "padding: 10px 12px; border-radius: 8px; background: rgba(255,255,255,0.03); border: 1px solid var(--border-color); display: flex; align-items: center; justify-content: space-between; gap: 12px;",
                                    div {
                                        style: "display: flex; flex-direction: column; gap: 2px;",
                                        div {
                                            style: "font-weight: 600; font-size: 13px;",
                                            "{rule.name} ",
                                            span { style: "font-weight: 400; color: var(--text-muted);", "({rule.match_regex})" }
                                        }
                                        div {
                                            style: "font-size: 12px; color: var(--text-secondary);",
                                            "Tags: {tags_str} • {translate(locale, \"table.col_series\")}: {ser_str} • Polish: ",
                                            {if rule.auto_polish { translate(locale, "rules.yes") } else { translate(locale, "rules.no") }}
                                        }
                                    }
                                    div {
                                        style: "display: flex; gap: 8px; align-items: center;",
                                        button {
                                            class: "btn btn-secondary",
                                            onclick: move |_| {
                                                let mut next = rules.read().clone();
                                                if let Some(r) = next.iter_mut().find(|x| x.id == id_toggle) {
                                                    r.enabled = !r.enabled;
                                                }
                                                p_toggle(next);
                                            },
                                            {if rule.enabled { translate(locale, "rules.enabled") } else { translate(locale, "rules.disabled") }}
                                        }
                                        button {
                                            class: "btn btn-danger",
                                            onclick: move |_| {
                                                let next: Vec<_> = rules.read().iter().filter(|x| x.id != id_delete).cloned().collect();
                                                p_delete(next);
                                            },
                                            "🗑️"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
