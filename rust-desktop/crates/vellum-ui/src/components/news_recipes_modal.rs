use dioxus::prelude::*;
use std::sync::Arc;
use vellum_core::{translate, translate_fmt, Locale, NewsService};
use vellum_domain::{NewsFeedConfig, NewsRecipe};

#[component]
pub fn NewsRecipesModal(
    locale: Locale,
    news_service: Arc<NewsService>,
    on_close: EventHandler<()>,
    on_imported: EventHandler<()>,
) -> Element {
    let recipes = use_signal(|| news_service.get_recipes());
    let mut selected_recipe_id = use_signal(|| {
        news_service
            .get_recipes()
            .first()
            .map(|r| r.id.clone())
            .unwrap_or_default()
    });
    let is_generating = use_signal(|| false);
    let notification = use_signal(|| None::<(String, bool)>); // (message, is_success)

    // New feed form inputs
    let mut new_feed_name = use_signal(String::new);
    let mut new_feed_url = use_signal(String::new);

    let ns = news_service.clone();

    // Find currently active recipe
    let current_recipe = recipes
        .read()
        .iter()
        .find(|r| r.id == *selected_recipe_id.read())
        .cloned();

    // Action: Generate and import recipe
    let run_generate = {
        let ns = ns.clone();
        move |recipe_id: String| {
            let ns = ns.clone();
            let mut is_gen = is_generating;
            let mut notif = notification;
            is_gen.set(true);
            notif.set(None);

            spawn(async move {
                let res = ns.run_recipe_and_import(&recipe_id).await;
                is_gen.set(false);
                match res {
                    Ok(book) => {
                        notif.set(Some((
                            translate_fmt(locale, "news.generated", &[("title", &book.title)]),
                            true,
                        )));
                        on_imported.call(());
                    }
                    Err(e) => {
                        notif.set(Some((format!("Erreur : {e}"), false)));
                    }
                }
            });
        }
    };

    rsx! {
        div {
            class: "drawer-backdrop",
            onclick: move |_| on_close.call(()),

            div {
                class: "drawer-panel",
                style: "width: 780px; max-width: 95vw; max-height: 90vh; display: flex; flex-direction: column;",
                onclick: move |evt| evt.stop_propagation(),

                // Header
                div { class: "drawer-header", style: "border-bottom: 1px solid var(--border-color); padding-bottom: 12px; margin-bottom: 12px;",
                    div { style: "display: flex; align-items: center; gap: 10px;",
                        span { style: "font-size: 24px;", "📰" }
                        div {
                            h3 { style: "font-size: 16px; font-weight: 700; margin: 0;", {translate(locale, "news.title")} }
                            p { style: "font-size: 12px; color: var(--text-secondary); margin: 0;", {translate(locale, "news.subtitle")} }
                        }
                    }
                    button {
                        class: "btn btn-icon",
                        onclick: move |_| on_close.call(()),
                        "✕"
                    }
                }

                // Notification Banner
                if let Some((msg, is_success)) = notification.read().as_ref() {
                    div {
                        style: if *is_success {
                            "padding: 10px 14px; border-radius: 6px; font-size: 13px; margin-bottom: 12px; background: rgba(16, 185, 129, 0.15); color: #34d399; border: 1px solid rgba(16, 185, 129, 0.3);"
                        } else {
                            "padding: 10px 14px; border-radius: 6px; font-size: 13px; margin-bottom: 12px; background: rgba(239, 68, 68, 0.15); color: #f87171; border: 1px solid rgba(239, 68, 68, 0.3);"
                        },
                        "{msg}"
                    }
                }

                // Content Layout: Left Recipe List, Right Recipe Details
                div { style: "display: flex; gap: 16px; flex: 1; overflow: hidden; min-height: 380px;",
                    // Left Column: Recipe Selector & Add
                    div { style: "width: 240px; border-right: 1px solid var(--border-color); padding-right: 12px; display: flex; flex-direction: column; gap: 8px;",
                        div { style: "font-size: 11px; font-weight: 700; color: var(--text-secondary); text-transform: uppercase;", {translate(locale, "news.available")} }
                        div { style: "flex: 1; overflow-y: auto; display: flex; flex-direction: column; gap: 6px;",
                            for recipe in recipes.read().iter() {
                                {
                                    let r_id = recipe.id.clone();
                                    let is_active = *selected_recipe_id.read() == r_id;
                                    let feed_count = recipe.feeds.len().to_string();
                                    let title = recipe.title.clone();
                                    rsx! {
                                        div {
                                            style: if is_active {
                                                "padding: 10px 12px; border-radius: 8px; background: rgba(99, 102, 241, 0.15); border: 1px solid rgba(99, 102, 241, 0.3); color: #818cf8; font-weight: 600; cursor: pointer;"
                                            } else {
                                                "padding: 10px 12px; border-radius: 8px; background: var(--bg-card); border: 1px solid var(--border-color); color: var(--text-primary); cursor: pointer;"
                                            },
                                            onclick: move |_| selected_recipe_id.set(r_id.clone()),
                                            div { style: "font-size: 13px;", "{title}" }
                                            div { style: "font-size: 11px; color: var(--text-secondary); font-weight: 400; margin-top: 2px;",
                                                {translate_fmt(locale, "news.feeds_count", &[("count", &feed_count)])}
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Add new recipe button
                        button {
                            class: "btn btn-secondary",
                            style: "width: 100%; font-size: 12px; padding: 7px; margin-top: auto;",
                            onclick: {
                                let mut r_sig = recipes;
                                let ns = ns.clone();
                                move |_| {
                                    let mut list = r_sig.read().clone();
                                    let idx = (list.len() + 1).to_string();
                                    let r_title = translate_fmt(locale, "news.new_recipe_name", &[("idx", &idx)]);
                                    let new_r = NewsRecipe::new(format!("recipe-custom-{idx}"), r_title);
                                    list.push(new_r);
                                    let _ = ns.save_recipes(&list);
                                    r_sig.set(list);
                                }
                            },
                            {translate(locale, "news.new_recipe_btn")}
                        }
                    }

                    // Right Column: Recipe Settings and Feeds
                    div { style: "flex: 1; overflow-y: auto; display: flex; flex-direction: column; gap: 14px; padding-left: 4px;",
                        if let Some(recipe) = current_recipe {
                            // Recipe title & Action header
                            div { style: "display: flex; justify-content: space-between; align-items: flex-start; gap: 12px;",
                                div { style: "flex: 1;",
                                    input {
                                        class: "drawer-input",
                                        style: "font-size: 16px; font-weight: 700; width: 100%; padding: 6px 10px;",
                                        value: "{recipe.title}",
                                        oninput: {
                                            let r_id = recipe.id.clone();
                                            let mut r_sig = recipes;
                                            let ns = ns.clone();
                                            move |e: Event<FormData>| {
                                                let mut list = r_sig.read().clone();
                                                if let Some(r) = list.iter_mut().find(|r| r.id == r_id) {
                                                    r.title = e.value();
                                                    let _ = ns.save_recipes(&list);
                                                    r_sig.set(list);
                                                }
                                            }
                                        },
                                    }
                                    if let Some(desc) = &recipe.description {
                                        p { style: "font-size: 12px; color: var(--text-secondary); margin-top: 4px;", "{desc}" }
                                    }
                                }

                                // Compile button
                                button {
                                    class: "btn btn-primary",
                                    style: "font-size: 13px; padding: 8px 16px; font-weight: 600; white-space: nowrap;",
                                    disabled: *is_generating.read() || recipe.feeds.is_empty(),
                                    onclick: {
                                        let r_id = recipe.id.clone();
                                        let r_gen = run_generate.clone();
                                        move |_| r_gen(r_id.clone())
                                    },
                                    {if *is_generating.read() { translate(locale, "news.compiling") } else { translate(locale, "news.compile_btn") }}
                                }
                            }

                            // Feeds List
                            div { style: "display: flex; flex-direction: column; gap: 8px; margin-top: 6px;",
                                div { style: "font-size: 12px; font-weight: 700; color: var(--text-secondary); text-transform: uppercase;",
                                    {
                                        let f_len = recipe.feeds.len().to_string();
                                        translate_fmt(locale, "news.included_feeds", &[("count", &f_len)])
                                    }
                                }

                                for (f_idx, feed) in recipe.feeds.iter().enumerate() {
                                    {
                                        let feed_idx = f_idx;
                                        let feed_name = feed.name.clone();
                                        let feed_url = feed.url.clone();
                                        let r_id = recipe.id.clone();

                                        rsx! {
                                            div {
                                                key: "{feed_idx}",
                                                style: "display: flex; align-items: center; justify-content: space-between; gap: 8px; padding: 8px 12px; background: var(--bg-card); border: 1px solid var(--border-color); border-radius: 6px;",
                                                div { style: "flex: 1; overflow: hidden;",
                                                    div { style: "font-size: 13px; font-weight: 600;", "{feed_name}" }
                                                    div { style: "font-size: 11px; font-family: monospace; color: var(--text-secondary); text-overflow: ellipsis; overflow: hidden; white-space: nowrap;",
                                                        "{feed_url}"
                                                    }
                                                }
                                                button {
                                                    class: "btn btn-icon",
                                                    style: "width: 28px; height: 28px; color: #f87171;",
                                                    onclick: {
                                                        let r_id = r_id.clone();
                                                        let mut r_sig = recipes;
                                                        let ns = ns.clone();
                                                        move |_| {
                                                            let mut list = r_sig.read().clone();
                                                            if let Some(r) = list.iter_mut().find(|r| r.id == r_id) {
                                                                if feed_idx < r.feeds.len() {
                                                                    r.feeds.remove(feed_idx);
                                                                    let _ = ns.save_recipes(&list);
                                                                    r_sig.set(list);
                                                                }
                                                            }
                                                        }
                                                    },
                                                    "🗑️"
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            // Form to add a feed to this recipe
                            div { style: "padding: 12px; background: rgba(255, 255, 255, 0.03); border: 1px dashed var(--border-color); border-radius: 8px; display: flex; flex-direction: column; gap: 8px;",
                                div { style: "font-size: 12px; font-weight: 600;", {translate(locale, "news.add_feed_title")} }
                                div { style: "display: flex; gap: 8px;",
                                    input {
                                        class: "drawer-input",
                                        style: "width: 140px; font-size: 12px; padding: 6px 10px;",
                                        placeholder: translate(locale, "news.feed_name_ph"),
                                        value: "{new_feed_name}",
                                        oninput: move |e| new_feed_name.set(e.value()),
                                    }
                                    input {
                                        class: "drawer-input",
                                        style: "flex: 1; font-size: 12px; padding: 6px 10px;",
                                        placeholder: "https://example.com/rss...",
                                        value: "{new_feed_url}",
                                        oninput: move |e| new_feed_url.set(e.value()),
                                    }
                                    button {
                                        class: "btn btn-secondary",
                                        style: "font-size: 12px; padding: 6px 12px; white-space: nowrap;",
                                        disabled: new_feed_name.read().trim().is_empty() || new_feed_url.read().trim().is_empty(),
                                        onclick: {
                                            let r_id = recipe.id.clone();
                                            let mut r_sig = recipes;
                                            let ns = ns.clone();
                                            let mut name_sig = new_feed_name;
                                            let mut url_sig = new_feed_url;
                                            move |_| {
                                                let name = name_sig.read().trim().to_string();
                                                let url = url_sig.read().trim().to_string();
                                                if !name.is_empty() && !url.is_empty() {
                                                    let mut list = r_sig.read().clone();
                                                    if let Some(r) = list.iter_mut().find(|r| r.id == r_id) {
                                                        let f_id = format!("feed-{}", r.feeds.len() + 1);
                                                        r.feeds.push(NewsFeedConfig::new(f_id, name, url));
                                                        let _ = ns.save_recipes(&list);
                                                        r_sig.set(list);
                                                        name_sig.set(String::new());
                                                        url_sig.set(String::new());
                                                    }
                                                }
                                            }
                                        },
                                        {translate(locale, "news.add_btn")}
                                    }
                                }
                            }

                            // Options (max articles, auto-send to Kindle)
                            div { style: "display: flex; gap: 16px; align-items: center; padding-top: 10px; border-top: 1px solid var(--border-color);",
                                div { style: "display: flex; align-items: center; gap: 8px;",
                                    label { style: "font-size: 12px; color: var(--text-secondary);", {translate(locale, "news.max_articles")} }
                                    input {
                                        class: "drawer-input",
                                        r#type: "number",
                                        style: "width: 70px; padding: 4px 8px; font-size: 12px; text-align: center;",
                                        value: "{recipe.max_articles_per_feed}",
                                        oninput: {
                                            let r_id = recipe.id.clone();
                                            let mut r_sig = recipes;
                                            let ns = ns.clone();
                                            move |e: Event<FormData>| {
                                                if let Ok(val) = e.value().parse::<usize>() {
                                                    let mut list = r_sig.read().clone();
                                                    if let Some(r) = list.iter_mut().find(|r| r.id == r_id) {
                                                        r.max_articles_per_feed = val;
                                                        let _ = ns.save_recipes(&list);
                                                        r_sig.set(list);
                                                    }
                                                }
                                            }
                                        },
                                    }
                                }

                                label { style: "display: flex; align-items: center; gap: 6px; font-size: 12px; cursor: pointer;",
                                    input {
                                        r#type: "checkbox",
                                        checked: recipe.auto_send_kindle,
                                        onchange: {
                                            let r_id = recipe.id.clone();
                                            let mut r_sig = recipes;
                                            let ns = ns.clone();
                                            move |e: Event<FormData>| {
                                                let checked = e.value() == "true";
                                                let mut list = r_sig.read().clone();
                                                if let Some(r) = list.iter_mut().find(|r| r.id == r_id) {
                                                    r.auto_send_kindle = checked;
                                                    let _ = ns.save_recipes(&list);
                                                    r_sig.set(list);
                                                }
                                            }
                                        },
                                    }
                                    span { {translate(locale, "news.auto_kindle")} }
                                }
                            }
                        } else {
                            div { style: "text-align: center; padding: 40px; color: var(--text-secondary);",
                                {translate(locale, "news.select_hint")}
                            }
                        }
                    }
                }
            }
        }
    }
}
