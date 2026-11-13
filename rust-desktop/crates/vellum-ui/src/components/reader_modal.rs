use dioxus::prelude::*;
use crate::state::ReaderSession;
use vellum_core::reader_store::{ComicFitMode, ComicReadingMode, SearchResult};

fn step_page(
    mut session: Signal<Option<ReaderSession>>,
    on_progress_update: EventHandler<(usize, usize, f64)>,
    forward: bool,
) {
    let mut mut_sess = session.write();
    if let Some(s) = mut_sess.as_mut() {
        let step = if s.is_comic() && s.prefs.comic_reading_mode == ComicReadingMode::DoublePage {
            2
        } else {
            1
        };
        if forward {
            if s.current_page + 1 < s.total_pages {
                s.current_page = (s.current_page + step).min(s.total_pages.saturating_sub(1));
            }
        } else {
            s.current_page = s.current_page.saturating_sub(step);
        }
        let (p, tot, pct) = (s.current_page, s.total_pages, s.progress_percent());
        drop(mut_sess);
        on_progress_update.call((p, tot, pct));
    }
}

#[component]
pub fn ReaderModal(
    session: Signal<Option<ReaderSession>>,
    on_close: EventHandler<()>,
    on_progress_update: EventHandler<(usize, usize, f64)>,
) -> Element {
    let mut show_chrome = use_signal(|| true);
    // Active drawer: None, "toc", "settings", "search"
    let mut active_drawer = use_signal(|| None::<String>);
    // Drawer tabs: "toc", "bookmarks", "highlights"
    let mut drawer_tab = use_signal(|| "toc".to_string());
    let mut toc_filter = use_signal(String::new);

    // Search state
    let mut search_query = use_signal(String::new);
    let mut search_results = use_signal(Vec::<SearchResult>::new);
    let mut is_searching = use_signal(|| false);

    // Selection & Annotation state
    let mut selected_text = use_signal(|| None::<String>);
    let mut note_mode = use_signal(|| false);
    let mut note_input = use_signal(String::new);

    // AI definition modal state
    let mut ai_modal_open = use_signal(|| false);
    let mut ai_query = use_signal(String::new);
    let mut ai_result = use_signal(|| None::<String>);
    let mut ai_loading = use_signal(|| false);

    // Status toast message (e.g. "Exporté avec succès !")
    let mut toast_msg = use_signal(|| None::<String>);

    // Listen to selection changes via JS eval
    use_effect(move || {
        let mut eval = document::eval(r#"
            document.addEventListener('mouseup', () => {
                setTimeout(() => {
                    const sel = window.getSelection().toString().trim();
                    if (sel.length >= 2) {
                        dioxus.send(sel);
                    }
                }, 10);
            });
        "#);
        spawn(async move {
            while let Ok(msg) = eval.recv::<String>().await {
                if !msg.trim().is_empty() {
                    selected_text.set(Some(msg));
                }
            }
        });
    });

    let sess_read = session.read();
    let Some(sess) = sess_read.as_ref() else {
        return rsx! {};
    };

    let current_html = sess.current_html();
    let book_title = sess.book.title.clone();
    let _book_author = sess.book.author.clone();
    let cur_page = sess.current_page;
    let total_pages = sess.total_pages;
    let progress = sess.progress_percent();
    let toc_entries = sess.table_of_contents();
    let page_info = sess.page_info();
    let is_bookmarked = sess.is_current_bookmarked();
    let (time_ch, time_book) = sess.estimate_remaining_time();
    let has_history = !sess.history.is_empty();

    let prefs = sess.prefs.clone();
    let user_data = sess.user_data.clone();

    let (chapter_title, page_in_ch, total_in_ch) = if let Some(ref info) = page_info {
        (
            info.chapter_title.clone(),
            info.page_in_chapter,
            info.total_in_chapter,
        )
    } else {
        (
            sess.current_title(),
            cur_page + 1,
            total_pages.max(1),
        )
    };

    let can_prev = cur_page > 0;
    let can_next = total_pages > 0 && cur_page + 1 < total_pages;

    let (bg_color, text_color, card_bg, border_color) = prefs.theme_colors();
    let font_family = prefs.font_css_family();
    let font_size = prefs.font_size;
    let line_height = prefs.line_height;
    let margin_h = prefs.margin_horizontal;
    let justify_css = if prefs.justify_text { "justify" } else { "left" };
    let brightness_pct = prefs.brightness;
    let warmth_opacity = (prefs.warmth as f32 / 100.0) * 0.75;
    let is_spread = prefs.two_columns;
    let is_comic = sess.is_comic();
    let is_webtoon = is_comic && prefs.comic_reading_mode == ComicReadingMode::Webtoon;
    let is_double = is_comic && prefs.comic_reading_mode == ComicReadingMode::DoublePage;

    let frame_card_class = if is_webtoon {
        "reader-screen-card webtoon-mode"
    } else if is_double {
        "reader-screen-card comic-double-mode"
    } else if is_spread {
        "reader-screen-card spread-mode"
    } else {
        "reader-screen-card"
    };

    let fit_class = match prefs.comic_fit_mode {
        ComicFitMode::FitHeight => "fit-height",
        ComicFitMode::FitWidth => "fit-width",
        ComicFitMode::Original => "fit-original",
    };

    let full_card_class = if is_comic {
        format!("{frame_card_class} {fit_class}")
    } else {
        frame_card_class.to_string()
    };

    let frame_style = format!(
        "background-color: {card_bg}; color: {text_color}; font-family: {font_family}; font-size: {font_size}px; border: 1px solid {border_color}; filter: brightness({brightness_pct}%);"
    );

    let body_style = if is_webtoon {
        "padding: 0; overflow-y: auto; height: 100%; text-align: center;".to_string()
    } else {
        format!("padding: 22px {margin_h}px; text-align: {justify_css}; line-height: {line_height};")
    };

    let body_class = if is_webtoon {
        "reader-screen-body webtoon-mode"
    } else {
        "reader-screen-body"
    };

    let on_left_click = move |_| {
        let fwd = is_comic && prefs.comic_rtl;
        step_page(session, on_progress_update, fwd);
    };

    let on_right_click = move |_| {
        let fwd = !(is_comic && prefs.comic_rtl);
        step_page(session, on_progress_update, fwd);
    };

    let go_history_back = move |_| {
        let mut mut_sess = session.write();
        if let Some(s) = mut_sess.as_mut() {
            if let Some(prev) = s.history_back() {
                let (p, tot, pct) = (prev, s.total_pages, s.progress_percent());
                drop(mut_sess);
                on_progress_update.call((p, tot, pct));
            }
        }
    };

    let handle_keydown = move |evt: KeyboardEvent| {
        let key_str = evt.key().to_string();
        let key = key_str.as_str();

        // If typing in an input field or note textarea, avoid capturing navigation keys
        if active_drawer.read().is_some() || *note_mode.read() || *ai_modal_open.read() {
            if key == "Escape" {
                if *ai_modal_open.read() {
                    ai_modal_open.set(false);
                } else if *note_mode.read() {
                    note_mode.set(false);
                } else {
                    active_drawer.set(None);
                }
            }
            return;
        }

        match key {
            "ArrowLeft" | "PageUp" => {
                let fwd = is_comic && prefs.comic_rtl;
                step_page(session, on_progress_update, fwd);
            }
            "ArrowRight" | "PageDown" | " " | "Enter" => {
                let fwd = !(is_comic && prefs.comic_rtl && key == "ArrowRight");
                step_page(session, on_progress_update, fwd);
            }
            "Backspace" => {
                step_page(session, on_progress_update, false);
            }
            "Escape" => {
                if selected_text.read().is_some() {
                    selected_text.set(None);
                } else {
                    on_close.call(());
                }
            }
            "b" | "B" => {
                let mut mut_sess = session.write();
                if let Some(s) = mut_sess.as_mut() {
                    s.toggle_current_bookmark();
                }
            }
            "t" | "T" => {
                let cur = active_drawer.read().clone();
                if cur == Some("toc".to_string()) {
                    active_drawer.set(None);
                } else {
                    active_drawer.set(Some("toc".to_string()));
                }
            }
            "s" | "S" => {
                let cur = active_drawer.read().clone();
                if cur == Some("search".to_string()) {
                    active_drawer.set(None);
                } else {
                    active_drawer.set(Some("search".to_string()));
                }
            }
            "m" | "M" | "f" | "F" => {
                let cur = *show_chrome.read();
                show_chrome.set(!cur);
            }
            "+" | "=" => {
                let mut mut_sess = session.write();
                if let Some(s) = mut_sess.as_mut() {
                    if s.prefs.font_size < 36 {
                        s.prefs.font_size += 2;
                        s.font_size = s.prefs.font_size;
                        s.save_prefs();
                    }
                }
            }
            "-" => {
                let mut mut_sess = session.write();
                if let Some(s) = mut_sess.as_mut() {
                    if s.prefs.font_size > 12 {
                        s.prefs.font_size -= 2;
                        s.font_size = s.prefs.font_size;
                        s.save_prefs();
                    }
                }
            }
            _ => {}
        }
    };

    let query_toc = toc_filter.read().trim().to_lowercase();
    let total_chapters = toc_entries.len();
    let filtered_toc: Vec<(usize, String)> = toc_entries
        .into_iter()
        .filter(|(_, title)| query_toc.is_empty() || title.to_lowercase().contains(&query_toc))
        .collect();

    let is_chrome_visible = *show_chrome.read();
    let current_drawer = active_drawer.read().clone();

    let footer_left = if is_comic {
        match prefs.comic_reading_mode {
            ComicReadingMode::Webtoon => format!("Défilement Webtoon continu • {} planches", total_pages),
            ComicReadingMode::DoublePage => {
                let next_p = (cur_page + 2).min(total_pages);
                if next_p > cur_page + 1 {
                    format!("Planches {}-{} sur {} • {}", cur_page + 1, next_p, total_pages, if prefs.comic_rtl { "Sens Manga (D➔G)" } else { "Sens BD (G➔D)" })
                } else {
                    format!("Planche {} sur {} • {}", cur_page + 1, total_pages, if prefs.comic_rtl { "Sens Manga (D➔G)" } else { "Sens BD (G➔D)" })
                }
            }
            ComicReadingMode::SinglePage => format!("Planche {} sur {} • {}", cur_page + 1, total_pages, if prefs.comic_rtl { "Sens Manga (D➔G)" } else { "Sens BD (G➔D)" }),
        }
    } else {
        format!("Page {page_in_ch} sur {total_in_ch} • {chapter_title}")
    };

    let footer_right = if is_comic && prefs.comic_reading_mode == ComicReadingMode::Webtoon {
        format!("{} planches", total_pages)
    } else {
        format!("Page {} / {} ({:.0}%)", cur_page + 1, total_pages.max(1), progress)
    };

    rsx! {
        div {
            class: "reader-modal",
            style: "background-color: {bg_color};",
            tabindex: "0",
            autofocus: true,
            onkeydown: handle_keydown,

            // Amber Warmth Filter Overlay (Anti blue-light night mode)
            if warmth_opacity > 0.01 {
                div {
                    class: "reader-warmth-overlay",
                    style: "opacity: {warmth_opacity};",
                }
            }

            // Notification toast
            if let Some(ref msg) = *toast_msg.read() {
                div {
                    style: "position: fixed; top: 68px; right: 24px; background: rgba(16, 185, 129, 0.95); color: #fff; padding: 10px 18px; border-radius: 8px; font-weight: 500; font-size: 13px; z-index: 200; box-shadow: 0 4px 16px rgba(0,0,0,0.3); animation: fadeInDown 0.2s ease;",
                    "{msg}"
                }
            }

            // Reader Top Toolbar
            if is_chrome_visible {
                div { class: "reader-toolbar",
                    div { style: "display: flex; align-items: center; gap: 8px;",
                        button {
                            class: "btn btn-secondary",
                            onclick: move |_| on_close.call(()),
                            "← Bibliothèque"
                        }

                        // Bookmark button
                        button {
                            class: if is_bookmarked { "btn btn-secondary active" } else { "btn btn-secondary" },
                            title: "Marquer cette page (B)",
                            onclick: move |_| {
                                let mut mut_sess = session.write();
                                if let Some(s) = mut_sess.as_mut() {
                                    s.toggle_current_bookmark();
                                }
                            },
                            if is_bookmarked { "🔖 Marqué" } else { "📑 Marquer" }
                        }

                        // TOC & Notes Drawer button
                        button {
                            class: if current_drawer.as_deref() == Some("toc") { "btn btn-secondary active" } else { "btn btn-secondary" },
                            title: "Sommaire, signets & citations (T)",
                            onclick: move |_| {
                                let cur = active_drawer.read().clone();
                                if cur.as_deref() == Some("toc") {
                                    active_drawer.set(None);
                                } else {
                                    active_drawer.set(Some("toc".to_string()));
                                }
                            },
                            "📑 Sommaire"
                        }

                        // Search drawer button
                        button {
                            class: if current_drawer.as_deref() == Some("search") { "btn btn-secondary active" } else { "btn btn-secondary" },
                            title: "Recherche dans le livre (S)",
                            onclick: move |_| {
                                let cur = active_drawer.read().clone();
                                if cur.as_deref() == Some("search") {
                                    active_drawer.set(None);
                                } else {
                                    active_drawer.set(Some("search".to_string()));
                                }
                            },
                            "🔍"
                        }

                        // History back button
                        if has_history {
                            button {
                                class: "btn btn-icon",
                                title: "Page précédente de l'historique",
                                onclick: go_history_back,
                                "↩"
                            }
                        }

                        // Book Title & Chapter badge
                        div { style: "display: flex; flex-direction: column; margin-left: 6px; max-width: 280px;",
                            span { style: "font-weight: 600; font-size: 13px; color: var(--text-primary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;", "{book_title}" }
                            span { style: "font-size: 11px; color: var(--text-muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;", "{chapter_title}" }
                        }
                    }

                    div { style: "display: flex; align-items: center; gap: 10px;",
                        // Comic Toolbar Controls
                        if is_comic {
                            div { class: "comic-mode-pill",
                                button {
                                    class: if prefs.comic_reading_mode == ComicReadingMode::SinglePage { "comic-pill-btn active" } else { "comic-pill-btn" },
                                    title: "Mode 1 planche",
                                    onclick: move |_| {
                                        let mut mut_sess = session.write();
                                        if let Some(s) = mut_sess.as_mut() {
                                            s.prefs.comic_reading_mode = ComicReadingMode::SinglePage;
                                            s.save_prefs();
                                        }
                                    },
                                    "📄 1 Page"
                                }
                                button {
                                    class: if prefs.comic_reading_mode == ComicReadingMode::DoublePage { "comic-pill-btn active" } else { "comic-pill-btn" },
                                    title: "Mode double-page (planche manga/BD)",
                                    onclick: move |_| {
                                        let mut mut_sess = session.write();
                                        if let Some(s) = mut_sess.as_mut() {
                                            s.prefs.comic_reading_mode = ComicReadingMode::DoublePage;
                                            if s.current_page % 2 != 0 && s.current_page > 0 {
                                                s.current_page -= 1;
                                            }
                                            s.save_prefs();
                                        }
                                    },
                                    "📖 2 Pages"
                                }
                                button {
                                    class: if prefs.comic_reading_mode == ComicReadingMode::Webtoon { "comic-pill-btn active" } else { "comic-pill-btn" },
                                    title: "Mode Webtoon (défilement vertical infini)",
                                    onclick: move |_| {
                                        let mut mut_sess = session.write();
                                        if let Some(s) = mut_sess.as_mut() {
                                            s.prefs.comic_reading_mode = ComicReadingMode::Webtoon;
                                            s.save_prefs();
                                        }
                                    },
                                    "📜 Webtoon"
                                }
                            }

                            // RTL Manga direction toggle button
                            button {
                                class: if prefs.comic_rtl { "btn btn-secondary active" } else { "btn btn-secondary" },
                                style: "font-size: 11px; padding: 4px 10px;",
                                title: if prefs.comic_rtl { "Sens japonais : Droite à Gauche (Actif)" } else { "Sens occidental : Gauche à Droite (Actif)" },
                                onclick: move |_| {
                                    let mut mut_sess = session.write();
                                    if let Some(s) = mut_sess.as_mut() {
                                        s.prefs.comic_rtl = !s.prefs.comic_rtl;
                                        s.save_prefs();
                                    }
                                },
                                if prefs.comic_rtl { "🇯🇵 Manga (D ➔ G)" } else { "🇫🇷 BD (G ➔ D)" }
                            }

                            // Fit mode quick toggle
                            button {
                                class: "btn btn-secondary",
                                style: "font-size: 11px; padding: 4px 10px;",
                                title: match prefs.comic_fit_mode {
                                    ComicFitMode::FitHeight => "Ajusté à la hauteur de l'écran",
                                    ComicFitMode::FitWidth => "Ajusté à la largeur",
                                    ComicFitMode::Original => "Taille originale de l'image (100%)",
                                },
                                onclick: move |_| {
                                    let mut mut_sess = session.write();
                                    if let Some(s) = mut_sess.as_mut() {
                                        s.prefs.comic_fit_mode = match s.prefs.comic_fit_mode {
                                            ComicFitMode::FitHeight => ComicFitMode::FitWidth,
                                            ComicFitMode::FitWidth => ComicFitMode::Original,
                                            ComicFitMode::Original => ComicFitMode::FitHeight,
                                        };
                                        s.save_prefs();
                                    }
                                },
                                match prefs.comic_fit_mode {
                                    ComicFitMode::FitHeight => "↕ Hauteur",
                                    ComicFitMode::FitWidth => "↔ Largeur",
                                    ComicFitMode::Original => "🔍 100%",
                                }
                            }
                        }

                        // Text Book Controls (Reading time & Font size)
                        if !is_comic {
                            // Remaining reading time estimate badge
                            div {
                                style: "display: flex; align-items: center; gap: 6px; padding: 4px 10px; background: rgba(255,255,255,0.05); border-radius: 14px; font-size: 11px; color: var(--text-secondary); cursor: default;",
                                title: "Temps restant estimé : {time_ch} dans ce chapitre • {time_book} pour finir le livre (basé sur {prefs.words_per_minute} mpm)",
                                span { "⏱" }
                                span { "{time_ch}" }
                            }

                            // Quick font size
                            div { style: "display: flex; align-items: center; gap: 2px;",
                                button {
                                    class: "btn btn-icon",
                                    title: "Réduire police (-)",
                                    onclick: move |_| {
                                        let mut mut_sess = session.write();
                                        if let Some(s) = mut_sess.as_mut() {
                                            if s.prefs.font_size > 12 {
                                                s.prefs.font_size -= 2;
                                                s.font_size = s.prefs.font_size;
                                                s.save_prefs();
                                            }
                                        }
                                    },
                                    "A-"
                                }
                                span { style: "font-size: 11px; width: 22px; text-align: center; color: var(--text-primary);", "{font_size}" }
                                button {
                                    class: "btn btn-icon",
                                    title: "Agrandir police (+)",
                                    onclick: move |_| {
                                        let mut mut_sess = session.write();
                                        if let Some(s) = mut_sess.as_mut() {
                                            if s.prefs.font_size < 36 {
                                                s.prefs.font_size += 2;
                                                s.font_size = s.prefs.font_size;
                                                s.save_prefs();
                                            }
                                        }
                                    },
                                    "A+"
                                }
                            }
                        }

                        // Settings Drawer toggle
                        button {
                            class: if current_drawer.as_deref() == Some("settings") { "btn btn-secondary active" } else { "btn btn-secondary" },
                            title: "Préférences d'affichage et lecture",
                            onclick: move |_| {
                                let cur = active_drawer.read().clone();
                                if cur.as_deref() == Some("settings") {
                                    active_drawer.set(None);
                                } else {
                                    active_drawer.set(Some("settings".to_string()));
                                }
                            },
                            "⚙ Réglages"
                        }

                        // Toggle Immersion Button
                        button {
                            class: "btn btn-icon",
                            title: "Mode immersif (M / F)",
                            onclick: move |_| show_chrome.set(false),
                            "⛶"
                        }
                    }
                }
            }

            // Floating Text Selection Action Bar
            if let Some(ref text) = *selected_text.read() {
                {
                    let txt = text.clone();
                    rsx! {
                        div { class: "reader-selection-bar",
                            span { style: "font-size: 11px; color: var(--text-muted); margin-right: 4px;", "Surligner :" }
                            // Yellow
                            div {
                                class: "color-dot",
                                style: "background-color: #facc15;",
                                title: "Jaune",
                                onclick: {
                                    let t = txt.clone();
                                    move |_| {
                                        let mut mut_sess = session.write();
                                        if let Some(s) = mut_sess.as_mut() {
                                            s.add_highlight("yellow", &t, None);
                                        }
                                        selected_text.set(None);
                                    }
                                }
                            }
                            // Green
                            div {
                                class: "color-dot",
                                style: "background-color: #4ade80;",
                                title: "Vert",
                                onclick: {
                                    let t = txt.clone();
                                    move |_| {
                                        let mut mut_sess = session.write();
                                        if let Some(s) = mut_sess.as_mut() {
                                            s.add_highlight("green", &t, None);
                                        }
                                        selected_text.set(None);
                                    }
                                }
                            }
                            // Blue
                            div {
                                class: "color-dot",
                                style: "background-color: #60a5fa;",
                                title: "Bleu",
                                onclick: {
                                    let t = txt.clone();
                                    move |_| {
                                        let mut mut_sess = session.write();
                                        if let Some(s) = mut_sess.as_mut() {
                                            s.add_highlight("blue", &t, None);
                                        }
                                        selected_text.set(None);
                                    }
                                }
                            }
                            // Pink
                            div {
                                class: "color-dot",
                                style: "background-color: #f472b6;",
                                title: "Rose",
                                onclick: {
                                    let t = txt.clone();
                                    move |_| {
                                        let mut mut_sess = session.write();
                                        if let Some(s) = mut_sess.as_mut() {
                                            s.add_highlight("pink", &t, None);
                                        }
                                        selected_text.set(None);
                                    }
                                }
                            }

                            div { style: "width: 1px; height: 18px; background: rgba(255,255,255,0.15); margin: 0 4px;" }

                            // Add Note button
                            button {
                                class: "btn btn-icon",
                                style: "font-size: 11px; padding: 4px 8px; border-radius: 12px;",
                                title: "Ajouter une note",
                                onclick: move |_| {
                                    note_mode.set(true);
                                },
                                "📝 Note"
                            }

                            // AI Definition / Translation button
                            button {
                                class: "btn btn-icon",
                                style: "font-size: 11px; padding: 4px 8px; border-radius: 12px; color: #a855f7;",
                                title: "Définition ou traduction IA",
                                onclick: {
                                    let t = txt.clone();
                                    move |_| {
                                        ai_query.set(t.clone());
                                        ai_result.set(None);
                                        ai_loading.set(true);
                                        ai_modal_open.set(true);
                                        selected_text.set(None);

                                        let prompt_text = t.clone();
                                        spawn(async move {
                                            let res = tokio::task::spawn_blocking(move || {
                                                let client = vellum_plugins::translator::llm_client::LlmClient::new(
                                                    "Ollama (Local - Gratuit)",
                                                    "http://localhost:11434/v1",
                                                    "qwen2.5:14b",
                                                    None,
                                                    0.3,
                                                );
                                                client.chat(
                                                    "Tu es un dictionnaire et assistant littéraire concis en français. Fournis la définition claire, le sens littéraire ou la traduction élégante du terme ou passage sélectionné, avec un exemple si opportun. Reste concis.",
                                                    &prompt_text,
                                                )
                                            }).await;

                                            ai_loading.set(false);
                                            match res {
                                                Ok(Ok(content)) => ai_result.set(Some(content)),
                                                Ok(Err(e)) => ai_result.set(Some(format!("Erreur lors de la requête IA : {}\n\nVérifiez qu'Ollama est lancé sur votre machine (`ollama run qwen2.5:14b`).", e))),
                                                Err(e) => ai_result.set(Some(format!("Tâche interrompue : {}", e))),
                                            }
                                        });
                                    }
                                },
                                "✨ Définir / IA"
                            }

                            // Dismiss selection
                            button {
                                class: "btn btn-icon",
                                style: "padding: 2px 6px; font-size: 11px; margin-left: 4px;",
                                onclick: move |_| selected_text.set(None),
                                "✕"
                            }
                        }
                    }
                }
            }

            // Note Input Popup
            if *note_mode.read() {
                if let Some(ref txt) = *selected_text.read() {
                    {
                        let text_val = txt.clone();
                        rsx! {
                            div {
                                class: "reader-ai-modal",
                                onclick: move |_| note_mode.set(false),
                                div {
                                    class: "reader-ai-card",
                                    onclick: move |e| e.stop_propagation(),
                                    h3 { style: "font-size: 16px; font-weight: 600;", "Ajouter une note de lecture" }
                                    blockquote { style: "font-size: 12px; color: var(--text-secondary); border-left: 2px solid var(--accent-primary); padding-left: 10px; margin: 4px 0;",
                                        "{text_val}"
                                    }
                                    textarea {
                                        class: "drawer-textarea",
                                        rows: 4,
                                        placeholder: "Écrivez votre commentaire ou réflexion ici...",
                                        value: "{note_input}",
                                        oninput: move |e| note_input.set(e.value()),
                                    }
                                    div { style: "display: flex; justify-content: flex-end; gap: 8px; margin-top: 10px;",
                                        button {
                                            class: "btn btn-secondary",
                                            onclick: move |_| note_mode.set(false),
                                            "Annuler"
                                        }
                                        button {
                                            class: "btn btn-primary",
                                            onclick: {
                                                let t = text_val.clone();
                                                move |_| {
                                                    let note_str = note_input.read().trim().to_string();
                                                    let opt_note = if note_str.is_empty() { None } else { Some(note_str) };
                                                    let mut mut_sess = session.write();
                                                    if let Some(s) = mut_sess.as_mut() {
                                                        s.add_highlight("yellow", &t, opt_note);
                                                    }
                                                    note_input.set(String::new());
                                                    note_mode.set(false);
                                                    selected_text.set(None);
                                                }
                                            },
                                            "Enregistrer la note"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // AI Definition Modal
            if *ai_modal_open.read() {
                div {
                    class: "reader-ai-modal",
                    onclick: move |_| ai_modal_open.set(false),
                    div {
                        class: "reader-ai-card",
                        onclick: move |e| e.stop_propagation(),
                        div { style: "display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid var(--border-subtle); padding-bottom: 12px;",
                            div { style: "display: flex; align-items: center; gap: 8px;",
                                span { style: "font-size: 20px;", "✨" }
                                span { style: "font-weight: 600; font-size: 15px;", "Explication & Définition IA" }
                            }
                            button {
                                class: "btn btn-icon",
                                onclick: move |_| ai_modal_open.set(false),
                                "✕"
                            }
                        }

                        div { style: "font-size: 13px; font-weight: 600; color: var(--accent-primary); background: rgba(99,102,241,0.08); padding: 8px 12px; border-radius: 6px;",
                            "« {ai_query} »"
                        }

                        div { style: "min-height: 90px; max-height: 300px; overflow-y: auto; font-size: 13px; line-height: 1.6; color: var(--text-primary);",
                            if *ai_loading.read() {
                                div { style: "display: flex; align-items: center; gap: 10px; color: var(--text-muted); padding: 20px 0;",
                                    span { style: "animation: pulse 1s infinite;", "Recherche en cours avec Ollama..." }
                                }
                            } else if let Some(ref res) = *ai_result.read() {
                                div { style: "white-space: pre-wrap;", "{res}" }
                            }
                        }

                        div { style: "display: flex; justify-content: flex-end; gap: 8px; border-top: 1px solid var(--border-subtle); padding-top: 12px;",
                            button {
                                class: "btn btn-secondary",
                                onclick: move |_| ai_modal_open.set(false),
                                "Fermer"
                            }
                            if let Some(ref res) = *ai_result.read() {
                                {
                                    let note_text = res.clone();
                                    let query_text = ai_query.read().clone();
                                    rsx! {
                                        button {
                                            class: "btn btn-primary",
                                            onclick: move |_| {
                                                let mut mut_sess = session.write();
                                                if let Some(s) = mut_sess.as_mut() {
                                                    s.add_highlight("blue", &query_text, Some(note_text.clone()));
                                                }
                                                ai_modal_open.set(false);
                                                toast_msg.set(Some("Enregistré comme note de lecture !".to_string()));
                                            },
                                            "📌 Garder comme note"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Table of Contents, Bookmarks & Highlights Drawer
            if current_drawer.as_deref() == Some("toc") {
                div {
                    class: "reader-toc-backdrop",
                    onclick: move |_| active_drawer.set(None),

                    div {
                        class: "reader-toc-drawer",
                        onclick: move |e| e.stop_propagation(),

                        // Header with close button
                        div { class: "reader-toc-header",
                            div { style: "display: flex; flex-direction: column;",
                                span { style: "font-weight: 600; font-size: 14px; color: var(--text-primary);", "Sommaire & Annotations" }
                                span { style: "font-size: 11px; color: var(--text-muted);", "{total_chapters} chapitres • {user_data.bookmarks.len()} signets • {user_data.highlights.len()} notes" }
                            }
                            button {
                                class: "btn btn-icon",
                                onclick: move |_| active_drawer.set(None),
                                "✕"
                            }
                        }

                        // Navigation tabs inside drawer
                        div { class: "reader-drawer-tabs",
                            button {
                                class: if *drawer_tab.read() == "toc" { "reader-drawer-tab active" } else { "reader-drawer-tab" },
                                onclick: move |_| drawer_tab.set("toc".to_string()),
                                "Sommaire"
                            }
                            button {
                                class: if *drawer_tab.read() == "bookmarks" { "reader-drawer-tab active" } else { "reader-drawer-tab" },
                                onclick: move |_| drawer_tab.set("bookmarks".to_string()),
                                "Signets ({user_data.bookmarks.len()})"
                            }
                            button {
                                class: if *drawer_tab.read() == "highlights" { "reader-drawer-tab active" } else { "reader-drawer-tab" },
                                onclick: move |_| drawer_tab.set("highlights".to_string()),
                                "Notes ({user_data.highlights.len()})"
                            }
                        }

                        // Tab 1: TOC Content
                        if *drawer_tab.read() == "toc" {
                            div { style: "padding: 10px 14px; border-bottom: 1px solid var(--border-subtle);",
                                input {
                                    style: "width: 100%; padding: 8px 12px; background: rgba(255,255,255,0.05); border: 1px solid var(--border-subtle); border-radius: 6px; color: var(--text-primary); font-size: 12px; outline: none;",
                                    placeholder: "Filtrer les chapitres...",
                                    value: "{toc_filter}",
                                    oninput: move |e| toc_filter.set(e.value()),
                                }
                            }

                            div { class: "reader-toc-list",
                                if filtered_toc.is_empty() {
                                    div { style: "padding: 30px 10px; text-align: center; color: var(--text-muted); font-size: 12px;",
                                        "Aucun chapitre trouvé."
                                    }
                                } else {
                                    for (idx, title) in filtered_toc.iter() {
                                        {
                                            let is_current = *idx == cur_page;
                                            let target_idx = *idx;
                                            let title_display = title.clone();
                                            rsx! {
                                                button {
                                                    key: "{target_idx}",
                                                    class: if is_current { "reader-toc-item active" } else { "reader-toc-item" },
                                                    onclick: move |_| {
                                                        let mut mut_sess = session.write();
                                                        if let Some(s) = mut_sess.as_mut() {
                                                            s.jump_to_page(target_idx);
                                                            let (p, tot, pct) = (s.current_page, s.total_pages, s.progress_percent());
                                                            drop(mut_sess);
                                                            active_drawer.set(None);
                                                            on_progress_update.call((p, tot, pct));
                                                        }
                                                    },
                                                    span { style: "overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 250px;",
                                                        "{title_display}"
                                                    }
                                                    span { style: "font-size: 11px; opacity: 0.6; margin-left: 8px;",
                                                        "p. {target_idx + 1}"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Tab 2: Bookmarks list
                        if *drawer_tab.read() == "bookmarks" {
                            div { class: "reader-toc-list",
                                if user_data.bookmarks.is_empty() {
                                    div { style: "padding: 40px 20px; text-align: center; color: var(--text-muted); font-size: 12px;",
                                        p { style: "font-size: 24px; margin-bottom: 8px;", "🔖" }
                                        p { "Aucun signet enregistré." }
                                        p { style: "margin-top: 6px; font-size: 11px; opacity: 0.7;", "Appuyez sur B ou sur « 📑 Marquer » pour ajouter un signet." }
                                    }
                                } else {
                                    for bm in user_data.bookmarks.iter() {
                                        {
                                            let bm_id = bm.id.clone();
                                            let bm_ch = bm.chapter_index;
                                            let bm_title = bm.chapter_title.clone();
                                            let bm_preview = bm.preview.clone();
                                            let bm_pct = bm.progress_percent;
                                            rsx! {
                                                div {
                                                    key: "{bm_id}",
                                                    class: "reader-annotation-item",
                                                    div { style: "display: flex; justify-content: space-between; align-items: center;",
                                                        span {
                                                            style: "font-weight: 600; font-size: 12px; color: var(--accent-primary); cursor: pointer;",
                                                            onclick: move |_| {
                                                                let mut mut_sess = session.write();
                                                                if let Some(s) = mut_sess.as_mut() {
                                                                    s.jump_to_page(bm_ch);
                                                                    let (p, tot, pct) = (s.current_page, s.total_pages, s.progress_percent());
                                                                    drop(mut_sess);
                                                                    active_drawer.set(None);
                                                                    on_progress_update.call((p, tot, pct));
                                                                }
                                                            },
                                                            "🔖 {bm_title} ({bm_pct:.0}%)"
                                                        }
                                                        button {
                                                            class: "btn btn-icon",
                                                            style: "padding: 2px 6px; font-size: 11px;",
                                                            title: "Supprimer ce signet",
                                                            onclick: {
                                                                let id = bm_id.clone();
                                                                move |_| {
                                                                    let mut mut_sess = session.write();
                                                                    if let Some(s) = mut_sess.as_mut() {
                                                                        s.remove_bookmark(&id);
                                                                    }
                                                                }
                                                            },
                                                            "🗑"
                                                        }
                                                    }
                                                    span {
                                                        style: "font-size: 11px; color: var(--text-secondary); line-height: 1.4; cursor: pointer;",
                                                        onclick: move |_| {
                                                            let mut mut_sess = session.write();
                                                            if let Some(s) = mut_sess.as_mut() {
                                                                s.jump_to_page(bm_ch);
                                                                let (p, tot, pct) = (s.current_page, s.total_pages, s.progress_percent());
                                                                drop(mut_sess);
                                                                active_drawer.set(None);
                                                                on_progress_update.call((p, tot, pct));
                                                            }
                                                        },
                                                        "{bm_preview}"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Tab 3: Highlights & Notes
                        if *drawer_tab.read() == "highlights" {
                            div { class: "reader-toc-list",
                                if user_data.highlights.is_empty() {
                                    div { style: "padding: 40px 20px; text-align: center; color: var(--text-muted); font-size: 12px;",
                                        p { style: "font-size: 24px; margin-bottom: 8px;", "🖍" }
                                        p { "Aucun surlignage ni note." }
                                        p { style: "margin-top: 6px; font-size: 11px; opacity: 0.7;", "Sélectionnez du texte dans le livre pour surligner ou ajouter une note." }
                                    }
                                } else {
                                    for h in user_data.highlights.iter() {
                                        {
                                            let h_id = h.id.clone();
                                            let h_ch = h.chapter_index;
                                            let h_title = h.chapter_title.clone();
                                            let h_text = h.selected_text.clone();
                                            let h_note = h.note.clone();
                                            let dot_color = match h.color.as_str() {
                                                "green" => "#4ade80",
                                                "blue" => "#60a5fa",
                                                "pink" => "#f472b6",
                                                _ => "#facc15",
                                            };
                                            rsx! {
                                                div {
                                                    key: "{h_id}",
                                                    class: "reader-annotation-item",
                                                    div { style: "display: flex; justify-content: space-between; align-items: center;",
                                                        div { style: "display: flex; align-items: center; gap: 6px;",
                                                            div { style: "width: 8px; height: 8px; border-radius: 50%; background: {dot_color};" }
                                                            span { style: "font-size: 11px; font-weight: 600; color: var(--text-muted);", "{h_title}" }
                                                        }
                                                        button {
                                                            class: "btn btn-icon",
                                                            style: "padding: 2px 6px; font-size: 11px;",
                                                            title: "Supprimer cette note",
                                                            onclick: {
                                                                let id = h_id.clone();
                                                                move |_| {
                                                                    let mut mut_sess = session.write();
                                                                    if let Some(s) = mut_sess.as_mut() {
                                                                        s.remove_highlight(&id);
                                                                    }
                                                                }
                                                            },
                                                            "🗑"
                                                        }
                                                    }
                                                    span {
                                                        style: "font-size: 12px; color: var(--text-primary); font-style: italic; border-left: 2px solid {dot_color}; padding-left: 8px; line-height: 1.4; cursor: pointer;",
                                                        onclick: move |_| {
                                                            let mut mut_sess = session.write();
                                                            if let Some(s) = mut_sess.as_mut() {
                                                                s.jump_to_page(h_ch);
                                                                let (p, tot, pct) = (s.current_page, s.total_pages, s.progress_percent());
                                                                drop(mut_sess);
                                                                active_drawer.set(None);
                                                                on_progress_update.call((p, tot, pct));
                                                            }
                                                        },
                                                        "« {h_text} »"
                                                    }
                                                    if let Some(note) = h_note {
                                                        span { style: "font-size: 11px; color: var(--accent-primary); margin-top: 4px; padding-left: 10px;",
                                                            "💬 {note}"
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
            }

            // Search Drawer
            if current_drawer.as_deref() == Some("search") {
                div {
                    class: "reader-search-backdrop",
                    onclick: move |_| active_drawer.set(None),

                    div {
                        class: "reader-search-drawer",
                        onclick: move |e| e.stop_propagation(),

                        div { style: "padding: 16px 20px; border-bottom: 1px solid var(--border-subtle); display: flex; align-items: center; justify-content: space-between;",
                            span { style: "font-weight: 600; font-size: 14px; color: var(--text-primary);", "Recherche dans le livre" }
                            button {
                                class: "btn btn-icon",
                                onclick: move |_| active_drawer.set(None),
                                "✕"
                            }
                        }

                        div { style: "padding: 12px 16px; border-bottom: 1px solid var(--border-subtle); display: flex; gap: 8px;",
                            input {
                                style: "flex: 1; padding: 8px 12px; background: rgba(255,255,255,0.06); border: 1px solid var(--border-subtle); border-radius: 6px; color: var(--text-primary); font-size: 13px; outline: none;",
                                placeholder: "Rechercher un mot, une phrase...",
                                autofocus: true,
                                value: "{search_query}",
                                oninput: move |e| search_query.set(e.value()),
                                onkeydown: move |e: KeyboardEvent| {
                                    if e.key().to_string() == "Enter" {
                                        let q = search_query.read().clone();
                                        if !q.trim().is_empty() {
                                            is_searching.set(true);
                                            let res = session.read().as_ref().map(|s| s.search(&q)).unwrap_or_default();
                                            search_results.set(res);
                                            is_searching.set(false);
                                        }
                                    }
                                },
                            }
                            button {
                                class: "btn btn-primary",
                                onclick: move |_| {
                                    let q = search_query.read().clone();
                                    if !q.trim().is_empty() {
                                        is_searching.set(true);
                                        let res = session.read().as_ref().map(|s| s.search(&q)).unwrap_or_default();
                                        search_results.set(res);
                                        is_searching.set(false);
                                    }
                                },
                                "Chercher"
                            }
                        }

                        div { style: "flex: 1; overflow-y: auto; padding: 12px 16px;",
                            if *is_searching.read() {
                                div { style: "padding: 30px; text-align: center; color: var(--text-muted); font-size: 12px;",
                                    "Recherche en cours dans l'ensemble des chapitres..."
                                }
                            } else if search_results.read().is_empty() {
                                div { style: "padding: 40px 10px; text-align: center; color: var(--text-muted); font-size: 12px;",
                                    if search_query.read().trim().is_empty() {
                                        "Entrez un mot ou une phrase à rechercher dans le livre."
                                    } else {
                                        "Aucun résultat trouvé pour cette recherche."
                                    }
                                }
                            } else {
                                div { style: "font-size: 11px; color: var(--text-muted); margin-bottom: 10px;",
                                    "{search_results.read().len()} occurrences trouvées :"
                                }
                                for item in search_results.read().iter() {
                                    {
                                        let ch_idx = item.chapter_index;
                                        let ch_title = item.chapter_title.clone();
                                        let before = item.snippet_before.clone();
                                        let matched = item.matched_text.clone();
                                        let after = item.snippet_after.clone();
                                        rsx! {
                                            div {
                                                class: "reader-search-card",
                                                onclick: move |_| {
                                                    let mut mut_sess = session.write();
                                                    if let Some(s) = mut_sess.as_mut() {
                                                        s.jump_to_page(ch_idx);
                                                        let (p, tot, pct) = (s.current_page, s.total_pages, s.progress_percent());
                                                        drop(mut_sess);
                                                        active_drawer.set(None);
                                                        on_progress_update.call((p, tot, pct));
                                                    }
                                                },
                                                div { style: "font-size: 11px; font-weight: 600; color: var(--accent-primary);",
                                                    "{ch_title}"
                                                }
                                                div { class: "reader-search-snippet",
                                                    span { "{before}" }
                                                    mark { "{matched}" }
                                                    span { "{after}" }
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

            // Settings & Preferences Drawer
            if current_drawer.as_deref() == Some("settings") {
                div {
                    class: "reader-settings-backdrop",
                    onclick: move |_| active_drawer.set(None),

                    div {
                        class: "reader-settings-drawer",
                        onclick: move |e| e.stop_propagation(),

                        div { style: "padding: 16px 20px; border-bottom: 1px solid var(--border-subtle); display: flex; align-items: center; justify-content: space-between;",
                            span { style: "font-weight: 600; font-size: 14px; color: var(--text-primary);", "Préférences de lecture" }
                            button {
                                class: "btn btn-icon",
                                onclick: move |_| active_drawer.set(None),
                                "✕"
                            }
                        }

                        // Theme selector
                        div { class: "settings-group",
                            span { class: "settings-label", "Thème d'affichage" }
                            div { style: "display: grid; grid-template-columns: repeat(4, 1fr); gap: 6px;",
                                button {
                                    class: if prefs.theme == "dark" { "btn btn-secondary active" } else { "btn btn-secondary" },
                                    style: "font-size: 11px; padding: 8px 4px; text-align: center;",
                                    onclick: move |_| {
                                        let mut mut_sess = session.write();
                                        if let Some(s) = mut_sess.as_mut() {
                                            s.prefs.theme = "dark".to_string();
                                            s.theme = "dark".to_string();
                                            s.save_prefs();
                                        }
                                    },
                                    "🌙 Sombre"
                                }
                                button {
                                    class: if prefs.theme == "sepia" { "btn btn-secondary active" } else { "btn btn-secondary" },
                                    style: "font-size: 11px; padding: 8px 4px; text-align: center; background: #3d2b1f; color: #f8f0dc;",
                                    onclick: move |_| {
                                        let mut mut_sess = session.write();
                                        if let Some(s) = mut_sess.as_mut() {
                                            s.prefs.theme = "sepia".to_string();
                                            s.theme = "sepia".to_string();
                                            s.save_prefs();
                                        }
                                    },
                                    "📜 Sépia"
                                }
                                button {
                                    class: if prefs.theme == "light" { "btn btn-secondary active" } else { "btn btn-secondary" },
                                    style: "font-size: 11px; padding: 8px 4px; text-align: center; background: #e8edf2; color: #1e293b;",
                                    onclick: move |_| {
                                        let mut mut_sess = session.write();
                                        if let Some(s) = mut_sess.as_mut() {
                                            s.prefs.theme = "light".to_string();
                                            s.theme = "light".to_string();
                                            s.save_prefs();
                                        }
                                    },
                                    "☀️ Clair"
                                }
                                button {
                                    class: if prefs.theme == "amoled" { "btn btn-secondary active" } else { "btn btn-secondary" },
                                    style: "font-size: 11px; padding: 8px 4px; text-align: center; background: #000; color: #fff;",
                                    onclick: move |_| {
                                        let mut mut_sess = session.write();
                                        if let Some(s) = mut_sess.as_mut() {
                                            s.prefs.theme = "amoled".to_string();
                                            s.theme = "amoled".to_string();
                                            s.save_prefs();
                                        }
                                    },
                                    "⬛ Noir"
                                }
                            }
                        }

                        if is_comic {
                            // Comic / Manga Settings Section
                            div { class: "settings-group",
                                span { class: "settings-label", "Mode de lecture BD & Manga" }
                                div { style: "display: flex; gap: 6px; margin-bottom: 8px;",
                                    button {
                                        class: if prefs.comic_reading_mode == ComicReadingMode::SinglePage { "btn btn-secondary active" } else { "btn btn-secondary" },
                                        style: "flex: 1; font-size: 11px;",
                                        onclick: move |_| {
                                            let mut mut_sess = session.write();
                                            if let Some(s) = mut_sess.as_mut() {
                                                s.prefs.comic_reading_mode = ComicReadingMode::SinglePage;
                                                s.save_prefs();
                                            }
                                        },
                                        "📄 1 Page"
                                    }
                                    button {
                                        class: if prefs.comic_reading_mode == ComicReadingMode::DoublePage { "btn btn-secondary active" } else { "btn btn-secondary" },
                                        style: "flex: 1; font-size: 11px;",
                                        onclick: move |_| {
                                            let mut mut_sess = session.write();
                                            if let Some(s) = mut_sess.as_mut() {
                                                s.prefs.comic_reading_mode = ComicReadingMode::DoublePage;
                                                if s.current_page % 2 != 0 && s.current_page > 0 {
                                                    s.current_page -= 1;
                                                }
                                                s.save_prefs();
                                            }
                                        },
                                        "📖 2 Pages"
                                    }
                                    button {
                                        class: if prefs.comic_reading_mode == ComicReadingMode::Webtoon { "btn btn-secondary active" } else { "btn btn-secondary" },
                                        style: "flex: 1; font-size: 11px;",
                                        onclick: move |_| {
                                            let mut mut_sess = session.write();
                                            if let Some(s) = mut_sess.as_mut() {
                                                s.prefs.comic_reading_mode = ComicReadingMode::Webtoon;
                                                s.save_prefs();
                                            }
                                        },
                                        "📜 Webtoon"
                                    }
                                }
                                p { style: "font-size: 11px; color: var(--text-muted); margin: 0; line-height: 1.4;",
                                    match prefs.comic_reading_mode {
                                        ComicReadingMode::SinglePage => "Affichage d'une planche avec ajustement dynamique.",
                                        ComicReadingMode::DoublePage => "Affichage de 2 planches côte à côte comme un livre ouvert.",
                                        ComicReadingMode::Webtoon => "Défilement vertical continu et infini, sans coupure.",
                                    }
                                }
                            }

                            div { class: "settings-group",
                                span { class: "settings-label", "Sens de lecture" }
                                div { style: "display: flex; gap: 8px;",
                                    button {
                                        class: if !prefs.comic_rtl { "btn btn-secondary active" } else { "btn btn-secondary" },
                                        style: "flex: 1; font-size: 11px;",
                                        onclick: move |_| {
                                            let mut mut_sess = session.write();
                                            if let Some(s) = mut_sess.as_mut() {
                                                s.prefs.comic_rtl = false;
                                                s.save_prefs();
                                            }
                                        },
                                        "🇫🇷 BD (G ➔ D)"
                                    }
                                    button {
                                        class: if prefs.comic_rtl { "btn btn-secondary active" } else { "btn btn-secondary" },
                                        style: "flex: 1; font-size: 11px;",
                                        onclick: move |_| {
                                            let mut mut_sess = session.write();
                                            if let Some(s) = mut_sess.as_mut() {
                                                s.prefs.comic_rtl = true;
                                                s.save_prefs();
                                            }
                                        },
                                        "🇯🇵 Manga (D ➔ G)"
                                    }
                                }
                            }

                            div { class: "settings-group",
                                span { class: "settings-label", "Ajustement de l'image" }
                                div { style: "display: flex; gap: 6px;",
                                    button {
                                        class: if prefs.comic_fit_mode == ComicFitMode::FitHeight { "btn btn-secondary active" } else { "btn btn-secondary" },
                                        style: "flex: 1; font-size: 11px;",
                                        onclick: move |_| {
                                            let mut mut_sess = session.write();
                                            if let Some(s) = mut_sess.as_mut() {
                                                s.prefs.comic_fit_mode = ComicFitMode::FitHeight;
                                                s.save_prefs();
                                            }
                                        },
                                        "↕ Hauteur"
                                    }
                                    button {
                                        class: if prefs.comic_fit_mode == ComicFitMode::FitWidth { "btn btn-secondary active" } else { "btn btn-secondary" },
                                        style: "flex: 1; font-size: 11px;",
                                        onclick: move |_| {
                                            let mut mut_sess = session.write();
                                            if let Some(s) = mut_sess.as_mut() {
                                                s.prefs.comic_fit_mode = ComicFitMode::FitWidth;
                                                s.save_prefs();
                                            }
                                        },
                                        "↔ Largeur"
                                    }
                                    button {
                                        class: if prefs.comic_fit_mode == ComicFitMode::Original { "btn btn-secondary active" } else { "btn btn-secondary" },
                                        style: "flex: 1; font-size: 11px;",
                                        onclick: move |_| {
                                            let mut mut_sess = session.write();
                                            if let Some(s) = mut_sess.as_mut() {
                                                s.prefs.comic_fit_mode = ComicFitMode::Original;
                                                s.save_prefs();
                                            }
                                        },
                                        "Taille 100%"
                                    }
                                }
                            }
                        } else {
                            // Text Typography & Layout Settings (EPUB / MOBI / TXT)
                            div { class: "settings-group",
                                span { class: "settings-label", "Police de caractères" }
                                div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 6px;",
                                    button {
                                        class: if prefs.font_family == "literata" { "btn btn-secondary active" } else { "btn btn-secondary" },
                                        style: "font-family: Georgia, serif; font-size: 12px;",
                                        onclick: move |_| {
                                            let mut mut_sess = session.write();
                                            if let Some(s) = mut_sess.as_mut() {
                                                s.prefs.font_family = "literata".to_string();
                                                s.is_serif = true;
                                                s.save_prefs();
                                            }
                                        },
                                        "Literata"
                                    }
                                    button {
                                        class: if prefs.font_family == "merriweather" { "btn btn-secondary active" } else { "btn btn-secondary" },
                                        style: "font-family: Georgia, serif; font-size: 12px;",
                                        onclick: move |_| {
                                            let mut mut_sess = session.write();
                                            if let Some(s) = mut_sess.as_mut() {
                                                s.prefs.font_family = "merriweather".to_string();
                                                s.is_serif = true;
                                                s.save_prefs();
                                            }
                                        },
                                        "Merriweather"
                                    }
                                    button {
                                        class: if prefs.font_family == "serif" { "btn btn-secondary active" } else { "btn btn-secondary" },
                                        style: "font-family: Georgia, serif; font-size: 12px;",
                                        onclick: move |_| {
                                            let mut mut_sess = session.write();
                                            if let Some(s) = mut_sess.as_mut() {
                                                s.prefs.font_family = "serif".to_string();
                                                s.is_serif = true;
                                                s.save_prefs();
                                            }
                                        },
                                        "Georgia"
                                    }
                                    button {
                                        class: if prefs.font_family == "sans" { "btn btn-secondary active" } else { "btn btn-secondary" },
                                        style: "font-family: sans-serif; font-size: 12px;",
                                        onclick: move |_| {
                                            let mut mut_sess = session.write();
                                            if let Some(s) = mut_sess.as_mut() {
                                                s.prefs.font_family = "sans".to_string();
                                                s.is_serif = false;
                                                s.save_prefs();
                                            }
                                        },
                                        "Sans-serif"
                                    }
                                    button {
                                        class: if prefs.font_family == "dyslexic" { "btn btn-secondary active" } else { "btn btn-secondary" },
                                        style: "font-size: 12px; grid-column: span 2;",
                                        onclick: move |_| {
                                            let mut mut_sess = session.write();
                                            if let Some(s) = mut_sess.as_mut() {
                                                s.prefs.font_family = "dyslexic".to_string();
                                                s.is_serif = false;
                                                s.save_prefs();
                                            }
                                        },
                                        "OpenDyslexic (Accessibilité)"
                                    }
                                }
                            }

                            // Font size slider
                            div { class: "settings-group",
                                div { style: "display: flex; justify-content: space-between; align-items: center;",
                                    span { class: "settings-label", "Taille de police" }
                                    span { style: "font-size: 12px; font-weight: 600; color: var(--accent-primary);", "{prefs.font_size} px" }
                                }
                                input {
                                    r#type: "range",
                                    min: "13",
                                    max: "32",
                                    value: "{prefs.font_size}",
                                    style: "accent-color: var(--accent-primary); cursor: pointer;",
                                    oninput: move |e| {
                                        if let Ok(v) = e.value().parse::<u32>() {
                                            let mut mut_sess = session.write();
                                            if let Some(s) = mut_sess.as_mut() {
                                                s.prefs.font_size = v;
                                                s.font_size = v;
                                                s.save_prefs();
                                            }
                                        }
                                    }
                                }
                            }

                            // Line height slider
                            div { class: "settings-group",
                                div { style: "display: flex; justify-content: space-between; align-items: center;",
                                    span { class: "settings-label", "Interligne" }
                                    span { style: "font-size: 12px; font-weight: 600; color: var(--accent-primary);", "{prefs.line_height:.2}" }
                                }
                                input {
                                    r#type: "range",
                                    min: "130",
                                    max: "220",
                                    value: "{(prefs.line_height * 100.0) as u32}",
                                    style: "accent-color: var(--accent-primary); cursor: pointer;",
                                    oninput: move |e| {
                                        if let Ok(v) = e.value().parse::<f32>() {
                                            let mut mut_sess = session.write();
                                            if let Some(s) = mut_sess.as_mut() {
                                                s.prefs.line_height = v / 100.0;
                                                s.save_prefs();
                                            }
                                        }
                                    }
                                }
                            }

                            // Horizontal Margins
                            div { class: "settings-group",
                                div { style: "display: flex; justify-content: space-between; align-items: center;",
                                    span { class: "settings-label", "Marges latérales" }
                                    span { style: "font-size: 12px; font-weight: 600; color: var(--accent-primary);", "{prefs.margin_horizontal} px" }
                                }
                                input {
                                    r#type: "range",
                                    min: "16",
                                    max: "64",
                                    value: "{prefs.margin_horizontal}",
                                    style: "accent-color: var(--accent-primary); cursor: pointer;",
                                    oninput: move |e| {
                                        if let Ok(v) = e.value().parse::<u32>() {
                                            let mut mut_sess = session.write();
                                            if let Some(s) = mut_sess.as_mut() {
                                                s.prefs.margin_horizontal = v;
                                                s.save_prefs();
                                            }
                                        }
                                    }
                                }
                            }

                            // Spread mode & Justification
                            div { class: "settings-group",
                                span { class: "settings-label", "Disposition & Alignement" }
                                div { style: "display: flex; gap: 8px;",
                                    button {
                                        class: if prefs.justify_text { "btn btn-secondary active" } else { "btn btn-secondary" },
                                        style: "flex: 1; font-size: 11px;",
                                        onclick: move |_| {
                                            let mut mut_sess = session.write();
                                            if let Some(s) = mut_sess.as_mut() {
                                                s.prefs.justify_text = !s.prefs.justify_text;
                                                s.save_prefs();
                                            }
                                        },
                                        if prefs.justify_text { "Justifié" } else { "Aligné gauche" }
                                    }
                                    button {
                                        class: if prefs.two_columns { "btn btn-secondary active" } else { "btn btn-secondary" },
                                        style: "flex: 1; font-size: 11px;",
                                        onclick: move |_| {
                                            let mut mut_sess = session.write();
                                            if let Some(s) = mut_sess.as_mut() {
                                                s.prefs.two_columns = !s.prefs.two_columns;
                                                s.save_prefs();
                                            }
                                        },
                                        if prefs.two_columns { "📖 2 Pages (Spread)" } else { "📄 1 Page" }
                                    }
                                }
                            }
                        }

                        // Warmth / Anti blue-light filter
                        div { class: "settings-group",
                            div { style: "display: flex; justify-content: space-between; align-items: center;",
                                span { class: "settings-label", "Filtre nuit (Lumière bleue / Ambre)" }
                                span { style: "font-size: 12px; font-weight: 600; color: #f59e0b;", "{prefs.warmth}%" }
                            }
                            input {
                                r#type: "range",
                                min: "0",
                                max: "70",
                                value: "{prefs.warmth}",
                                style: "accent-color: #f59e0b; cursor: pointer;",
                                oninput: move |e| {
                                    if let Ok(v) = e.value().parse::<u8>() {
                                        let mut mut_sess = session.write();
                                        if let Some(s) = mut_sess.as_mut() {
                                            s.prefs.warmth = v;
                                            s.save_prefs();
                                        }
                                    }
                                }
                            }
                        }

                        // Software Brightness
                        div { class: "settings-group",
                            div { style: "display: flex; justify-content: space-between; align-items: center;",
                                span { class: "settings-label", "Luminosité logicielle" }
                                span { style: "font-size: 12px; font-weight: 600; color: var(--accent-primary);", "{prefs.brightness}%" }
                            }
                            input {
                                r#type: "range",
                                min: "35",
                                max: "100",
                                value: "{prefs.brightness}",
                                style: "accent-color: var(--accent-primary); cursor: pointer;",
                                oninput: move |e| {
                                    if let Ok(v) = e.value().parse::<u8>() {
                                        let mut mut_sess = session.write();
                                        if let Some(s) = mut_sess.as_mut() {
                                            s.prefs.brightness = v;
                                            s.save_prefs();
                                        }
                                    }
                                }
                            }
                        }

                        // Reading speed (WPM)
                        div { class: "settings-group",
                            div { style: "display: flex; justify-content: space-between; align-items: center;",
                                span { class: "settings-label", "Vitesse de lecture (WPM)" }
                                span { style: "font-size: 12px; font-weight: 600; color: var(--accent-primary);", "{prefs.words_per_minute} mots/min" }
                            }
                            input {
                                r#type: "range",
                                min: "120",
                                max: "450",
                                value: "{prefs.words_per_minute}",
                                style: "accent-color: var(--accent-primary); cursor: pointer;",
                                oninput: move |e| {
                                    if let Ok(v) = e.value().parse::<usize>() {
                                        let mut mut_sess = session.write();
                                        if let Some(s) = mut_sess.as_mut() {
                                            s.prefs.words_per_minute = v;
                                            s.save_prefs();
                                        }
                                    }
                                }
                            }
                        }

                        // Export annotations to Markdown button
                        div { class: "settings-group",
                            span { class: "settings-label", "Sauvegarde & Export" }
                            button {
                                class: "btn btn-primary",
                                style: "width: 100%; font-size: 12px; padding: 10px;",
                                onclick: {
                                    let b_title = book_title.clone();
                                    move |_| {
                                        let sess_ref = session.read();
                                        if let Some(s) = sess_ref.as_ref() {
                                            let md = s.export_annotations_markdown();
                                            let file_default = format!("notes_{}.md", b_title.replace(' ', "_"));
                                            if let Some(dest) = rfd::FileDialog::new()
                                                .set_file_name(&file_default)
                                                .save_file()
                                            {
                                                if std::fs::write(&dest, md).is_ok() {
                                                    toast_msg.set(Some("Notes exportées avec succès !".to_string()));
                                                }
                                            }
                                        }
                                    }
                                },
                                "📥 Exporter notes & signets (.md)"
                            }
                        }
                    }
                }
            }

            // eReader Main Reading Viewport with Tap Zones
            div { class: "reader-viewport-area",
                // Left Tap Zone (Previous Page or Next Page in Manga RTL)
                if !is_webtoon {
                    div {
                        class: "reader-tap-zone left",
                        title: if is_comic && prefs.comic_rtl { "Page suivante (Manga D ➔ G)" } else { "Page précédente (Flèche gauche)" },
                        onclick: on_left_click,
                        if is_comic && prefs.comic_rtl {
                            if can_next {
                                span { class: "reader-tap-indicator", "›" }
                            }
                        } else {
                            if can_prev {
                                span { class: "reader-tap-indicator", "‹" }
                            }
                        }
                    }
                }

                // Center eReader Screen Card
                div {
                    class: "{full_card_class}",
                    style: "{frame_style}",

                    // Screen Header (Discreet eReader Chapter Header)
                    div { class: "reader-screen-header",
                        span { class: "reader-screen-header-title", "{chapter_title}" }
                    }

                    // Screen Body (Strictly Paginated, No Cutoff Lines, or Continuous Webtoon)
                    div {
                        class: "{body_class}",
                        style: "{body_style}",
                        if current_html.trim().is_empty() {
                            div {
                                style: "margin: auto; text-align: center; color: var(--text-muted); padding: 60px 20px;",
                                p { style: "font-size: 15px; font-weight: 500;", "Aucun contenu textuel ou graphique disponible pour cette page." }
                            }
                        } else {
                            div {
                                class: "reader-inner",
                                dangerous_inner_html: "{current_html}",
                            }
                        }
                    }

                    // Screen Footer (eReader Page Count & Book Progress)
                    div { class: "reader-screen-footer",
                        div { class: "reader-footer-left",
                            "{footer_left}"
                        }
                        div { class: "reader-footer-right",
                            "{footer_right}"
                        }
                    }
                }

                // Center Tap Zone (Toggle Chrome)
                if !is_webtoon {
                    div {
                        class: "reader-tap-zone center",
                        onclick: move |_| {
                            let cur = *show_chrome.read();
                            show_chrome.set(!cur);
                        },
                    }
                }

                // Right Tap Zone (Next Page or Previous Page in Manga RTL)
                if !is_webtoon {
                    div {
                        class: "reader-tap-zone right",
                        title: if is_comic && prefs.comic_rtl { "Page précédente (Manga D ➔ G)" } else { "Page suivante (Flèche droite / Espace)" },
                        onclick: on_right_click,
                        if is_comic && prefs.comic_rtl {
                            if can_prev {
                                span { class: "reader-tap-indicator", "‹" }
                            }
                        } else {
                            if can_next {
                                span { class: "reader-tap-indicator", "›" }
                            }
                        }
                    }
                }
            }

            // Reader Bottom Navigation (collapsible with chrome)
            if is_chrome_visible {
                div { class: "reader-bottom-nav",
                    button {
                        class: "btn btn-secondary",
                        disabled: if is_comic && prefs.comic_rtl { !can_next } else { !can_prev },
                        onclick: on_left_click,
                        if is_comic && prefs.comic_rtl { "Suivante ◀ (Manga)" } else { "◀ Page précédente" }
                    }

                    div { style: "display: flex; align-items: center; gap: 16px; flex: 1; max-width: 540px; margin: 0 20px;",
                        input {
                            r#type: "range",
                            min: "0",
                            max: "{total_pages.saturating_sub(1)}",
                            value: "{cur_page}",
                            style: "flex: 1; accent-color: var(--accent-primary); cursor: pointer;",
                            oninput: move |e| {
                                if let Ok(target) = e.value().parse::<usize>() {
                                    let mut mut_sess = session.write();
                                    if let Some(s) = mut_sess.as_mut() {
                                        let aligned = if s.is_comic() && s.prefs.comic_reading_mode == ComicReadingMode::DoublePage && target > 0 {
                                            target - (target % 2)
                                        } else {
                                            target
                                        };
                                        s.jump_to_page(aligned.min(s.total_pages.saturating_sub(1)));
                                        let (p, tot, pct) = (s.current_page, s.total_pages, s.progress_percent());
                                        drop(mut_sess);
                                        on_progress_update.call((p, tot, pct));

                                        if is_webtoon {
                                            let _ = document::eval(&format!(
                                                "const el = document.getElementById('webtoon-page-{}'); if (el) el.scrollIntoView({{behavior: 'smooth', block: 'start'}});",
                                                target + 1
                                            ));
                                        }
                                    }
                                }
                            },
                        }
                        span { style: "font-size: 12px; color: var(--text-muted); white-space: nowrap;",
                            if is_webtoon {
                                "Planche {cur_page + 1} / {total_pages.max(1)}"
                            } else {
                                "Page {cur_page + 1} / {total_pages.max(1)} ({progress:.1}%)"
                            }
                        }
                    }

                    button {
                        class: "btn btn-primary",
                        disabled: if is_comic && prefs.comic_rtl { !can_prev } else { !can_next },
                        onclick: on_right_click,
                        if is_comic && prefs.comic_rtl { "Précédente ▶ (Manga)" } else { "Page suivante ▶" }
                    }
                }
            }
        }
    }
}
