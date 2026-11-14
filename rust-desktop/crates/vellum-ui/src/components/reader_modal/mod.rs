mod overlays;
mod search_drawer;
mod settings_drawer;
mod toc_drawer;
mod toolbar;

use crate::state::ReaderSession;
use dioxus::prelude::*;
use vellum_core::reader_store::{ComicFitMode, ComicReadingMode, SearchResult};

use self::overlays::ReaderSelectionOverlays;
use self::search_drawer::ReaderSearchDrawer;
use self::settings_drawer::ReaderSettingsDrawer;
use self::toc_drawer::ReaderTocDrawer;
use self::toolbar::ReaderToolbar;

pub(super) fn jump_and_notify(
    mut session: Signal<Option<ReaderSession>>,
    mut active_drawer: Signal<Option<String>>,
    on_progress_update: EventHandler<(usize, usize, f64)>,
    target_page: usize,
) {
    let mut mut_sess = session.write();
    if let Some(s) = mut_sess.as_mut() {
        s.jump_to_page(target_page);
        let (p, tot, pct) = (s.current_page, s.total_pages, s.progress_percent());
        drop(mut_sess);
        active_drawer.set(None);
        on_progress_update.call((p, tot, pct));
    }
}

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
    mut session: Signal<Option<ReaderSession>>,
    on_close: EventHandler<()>,
    on_progress_update: EventHandler<(usize, usize, f64)>,
) -> Element {
    let mut show_chrome = use_signal(|| true);
    let mut active_drawer = use_signal(|| None::<String>);
    let drawer_tab = use_signal(|| "toc".to_string());
    let toc_filter = use_signal(String::new);

    let search_query = use_signal(String::new);
    let search_results = use_signal(Vec::<SearchResult>::new);
    let is_searching = use_signal(|| false);

    let mut selected_text = use_signal(|| None::<String>);
    let mut note_mode = use_signal(|| false);
    let note_input = use_signal(String::new);

    let mut ai_modal_open = use_signal(|| false);
    let ai_query = use_signal(String::new);
    let ai_result = use_signal(|| None::<String>);
    let ai_loading = use_signal(|| false);

    let toast_msg = use_signal(|| None::<String>);

    use_effect(move || {
        let mut eval = document::eval(
            r#"
            document.addEventListener('mouseup', () => {
                setTimeout(() => {
                    const sel = window.getSelection().toString().trim();
                    if (sel.length >= 2) {
                        dioxus.send(sel);
                    }
                }, 10);
            });
        "#,
        );
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
    let cur_page = sess.current_page;
    let total_pages = sess.total_pages;
    let progress = sess.progress_percent();
    let toc_entries = sess.table_of_contents();
    let page_info = sess.page_info();
    let is_bookmarked = sess.is_current_bookmarked();
    let (time_ch, time_book) = sess.estimate_remaining_time();
    let has_history = !sess.history.is_empty();

    let prefs = sess.prefs.clone();

    let (chapter_title, page_in_ch, total_in_ch) = if let Some(ref info) = page_info {
        (
            info.chapter_title.clone(),
            info.page_in_chapter,
            info.total_in_chapter,
        )
    } else {
        (sess.current_title(), cur_page + 1, total_pages.max(1))
    };

    let can_prev = cur_page > 0;
    let can_next = total_pages > 0 && cur_page + 1 < total_pages;

    let (bg_color, text_color, card_bg, border_color) = prefs.theme_colors();
    let font_family = prefs.font_css_family();
    let font_size = prefs.font_size;
    let line_height = prefs.line_height;
    let margin_h = prefs.margin_horizontal;
    let justify_css = if prefs.justify_text {
        "justify"
    } else {
        "left"
    };
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
        format!(
            "padding: 22px {margin_h}px; text-align: {justify_css}; line-height: {line_height};"
        )
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
                if cur.as_deref() == Some("toc") {
                    active_drawer.set(None);
                } else {
                    active_drawer.set(Some("toc".to_string()));
                }
            }
            "s" | "S" => {
                let cur = active_drawer.read().clone();
                if cur.as_deref() == Some("search") {
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
        let dir_label = if prefs.comic_rtl {
            "Sens Manga (D➔G)"
        } else {
            "Sens BD (G➔D)"
        };
        match prefs.comic_reading_mode {
            ComicReadingMode::Webtoon => {
                format!("Défilement Webtoon continu • {} planches", total_pages)
            }
            ComicReadingMode::DoublePage => {
                let next_p = (cur_page + 2).min(total_pages);
                if next_p > cur_page + 1 {
                    format!(
                        "Planches {}-{} sur {} • {}",
                        cur_page + 1,
                        next_p,
                        total_pages,
                        dir_label
                    )
                } else {
                    format!(
                        "Planche {} sur {} • {}",
                        cur_page + 1,
                        total_pages,
                        dir_label
                    )
                }
            }
            ComicReadingMode::SinglePage => {
                format!(
                    "Planche {} sur {} • {}",
                    cur_page + 1,
                    total_pages,
                    dir_label
                )
            }
        }
    } else {
        format!("Page {page_in_ch} sur {total_in_ch} • {chapter_title}")
    };

    let footer_right = if is_comic && prefs.comic_reading_mode == ComicReadingMode::Webtoon {
        format!("{} planches", total_pages)
    } else {
        format!(
            "Page {} / {} ({:.0}%)",
            cur_page + 1,
            total_pages.max(1),
            progress
        )
    };

    rsx! {
        div {
            class: "reader-modal",
            style: "background-color: {bg_color};",
            tabindex: "0",
            autofocus: true,
            onkeydown: handle_keydown,

            if warmth_opacity > 0.01 {
                div {
                    class: "reader-warmth-overlay",
                    style: "opacity: {warmth_opacity};",
                }
            }

            if let Some(ref msg) = *toast_msg.read() {
                div {
                    style: "position: fixed; top: 68px; right: 24px; background: rgba(16, 185, 129, 0.95); color: #fff; padding: 10px 18px; border-radius: 8px; font-weight: 500; font-size: 13px; z-index: 200; box-shadow: 0 4px 16px rgba(0,0,0,0.3); animation: fadeInDown 0.2s ease;",
                    "{msg}"
                }
            }

            if is_chrome_visible {
                ReaderToolbar {
                    session,
                    show_chrome,
                    active_drawer,
                    current_drawer: current_drawer.clone(),
                    book_title: book_title.clone(),
                    chapter_title: chapter_title.clone(),
                    is_bookmarked,
                    has_history,
                    is_comic,
                    prefs: prefs.clone(),
                    time_ch,
                    time_book,
                    on_close: move |_| on_close.call(()),
                    on_history_back: go_history_back,
                }
            }

            ReaderSelectionOverlays {
                session,
                selected_text,
                note_mode,
                note_input,
                ai_modal_open,
                ai_query,
                ai_result,
                ai_loading,
                toast_msg,
            }

            if current_drawer.as_deref() == Some("toc") {
                ReaderTocDrawer {
                    session,
                    active_drawer,
                    drawer_tab,
                    toc_filter,
                    filtered_toc,
                    total_chapters,
                    cur_page,
                    on_progress_update,
                }
            }

            if current_drawer.as_deref() == Some("search") {
                ReaderSearchDrawer {
                    session,
                    active_drawer,
                    search_query,
                    search_results,
                    is_searching,
                    on_progress_update,
                }
            }

            if current_drawer.as_deref() == Some("settings") {
                ReaderSettingsDrawer {
                    session,
                    active_drawer,
                    toast_msg,
                    prefs: prefs.clone(),
                    is_comic,
                    book_title: book_title.clone(),
                }
            }

            div { class: "reader-viewport-area",
                if !is_webtoon {
                    div {
                        class: "reader-tap-zone left",
                        title: if is_comic && prefs.comic_rtl { "Page suivante (Manga D ➔ G)" } else { "Page précédente (Flèche gauche)" },
                        onclick: on_left_click,
                        if is_comic && prefs.comic_rtl {
                            if can_next {
                                span { class: "reader-tap-indicator", "›" }
                            }
                        } else if can_prev {
                            span { class: "reader-tap-indicator", "‹" }
                        }
                    }
                }

                div {
                    class: "{full_card_class}",
                    style: "{frame_style}",

                    div { class: "reader-screen-header",
                        span { class: "reader-screen-header-title", "{chapter_title}" }
                    }

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

                    div { class: "reader-screen-footer",
                        div { class: "reader-footer-left", "{footer_left}" }
                        div { class: "reader-footer-right", "{footer_right}" }
                    }
                }

                if !is_webtoon {
                    div {
                        class: "reader-tap-zone center",
                        onclick: move |_| {
                            let cur = *show_chrome.read();
                            show_chrome.set(!cur);
                        },
                    }

                    div {
                        class: "reader-tap-zone right",
                        title: if is_comic && prefs.comic_rtl { "Page précédente (Manga D ➔ G)" } else { "Page suivante (Flèche droite / Espace)" },
                        onclick: on_right_click,
                        if is_comic && prefs.comic_rtl {
                            if can_prev {
                                span { class: "reader-tap-indicator", "‹" }
                            }
                        } else if can_next {
                            span { class: "reader-tap-indicator", "›" }
                        }
                    }
                }
            }

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
