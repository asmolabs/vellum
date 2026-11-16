use crate::state::ReaderSession;
use dioxus::prelude::*;
use vellum_core::i18n::{translate, translate_fmt, Locale};
use vellum_core::reader_store::{ComicFitMode, ComicReadingMode, ReaderPreferences};

#[component]
pub(super) fn ReaderToolbar(
    mut session: Signal<Option<ReaderSession>>,
    locale: Locale,
    mut show_chrome: Signal<bool>,
    mut active_drawer: Signal<Option<String>>,
    current_drawer: Option<String>,
    book_title: String,
    chapter_title: String,
    is_bookmarked: bool,
    has_history: bool,
    is_comic: bool,
    prefs: ReaderPreferences,
    time_ch: String,
    time_book: String,
    on_close: EventHandler<()>,
    on_history_back: EventHandler<()>,
) -> Element {
    let font_size = prefs.font_size;

    rsx! {
        div { class: "reader-toolbar",
            div { style: "display: flex; align-items: center; gap: 8px;",
                button {
                    class: "btn btn-secondary",
                    onclick: move |_| on_close.call(()),
                    {translate(locale, "reader.back_library")}
                }

                button {
                    class: if is_bookmarked { "btn btn-secondary active" } else { "btn btn-secondary" },
                    title: translate(locale, "reader.bookmark_title"),
                    onclick: move |_| {
                        let mut mut_sess = session.write();
                        if let Some(s) = mut_sess.as_mut() {
                            s.toggle_current_bookmark();
                        }
                    },
                    if is_bookmarked {
                        {translate(locale, "reader.bookmarked")}
                    } else {
                        {translate(locale, "reader.bookmark_btn")}
                    }
                }

                button {
                    class: if current_drawer.as_deref() == Some("toc") { "btn btn-secondary active" } else { "btn btn-secondary" },
                    title: translate(locale, "reader.toc_title"),
                    onclick: move |_| {
                        let cur = active_drawer.read().clone();
                        if cur.as_deref() == Some("toc") {
                            active_drawer.set(None);
                        } else {
                            active_drawer.set(Some("toc".to_string()));
                        }
                    },
                    {translate(locale, "reader.toc_btn")}
                }

                button {
                    class: if current_drawer.as_deref() == Some("search") { "btn btn-secondary active" } else { "btn btn-secondary" },
                    title: translate(locale, "reader.search_title"),
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

                if has_history {
                    button {
                        class: "btn btn-icon",
                        title: translate(locale, "reader.history_back_title"),
                        onclick: move |_| on_history_back.call(()),
                        "↩"
                    }
                }

                div { style: "display: flex; flex-direction: column; margin-left: 6px; max-width: 280px;",
                    span { style: "font-weight: 600; font-size: 13px; color: var(--text-primary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;", "{book_title}" }
                    span { style: "font-size: 11px; color: var(--text-muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;", "{chapter_title}" }
                }
            }

            div { style: "display: flex; align-items: center; gap: 10px;",
                if is_comic {
                    div { class: "comic-mode-pill",
                        button {
                            class: if prefs.comic_reading_mode == ComicReadingMode::SinglePage { "comic-pill-btn active" } else { "comic-pill-btn" },
                            title: translate(locale, "reader.comic_1p_title"),
                            onclick: move |_| {
                                let mut mut_sess = session.write();
                                if let Some(s) = mut_sess.as_mut() {
                                    s.prefs.comic_reading_mode = ComicReadingMode::SinglePage;
                                    s.save_prefs();
                                }
                            },
                            {translate(locale, "reader.comic_1p_btn")}
                        }
                        button {
                            class: if prefs.comic_reading_mode == ComicReadingMode::DoublePage { "comic-pill-btn active" } else { "comic-pill-btn" },
                            title: translate(locale, "reader.comic_2p_title"),
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
                            {translate(locale, "reader.comic_2p_btn")}
                        }
                        button {
                            class: if prefs.comic_reading_mode == ComicReadingMode::Webtoon { "comic-pill-btn active" } else { "comic-pill-btn" },
                            title: translate(locale, "reader.comic_webtoon_title"),
                            onclick: move |_| {
                                let mut mut_sess = session.write();
                                if let Some(s) = mut_sess.as_mut() {
                                    s.prefs.comic_reading_mode = ComicReadingMode::Webtoon;
                                    s.save_prefs();
                                }
                            },
                            {translate(locale, "reader.comic_webtoon_btn")}
                        }
                    }

                    button {
                        class: if prefs.comic_rtl { "btn btn-secondary active" } else { "btn btn-secondary" },
                        style: "font-size: 11px; padding: 4px 10px;",
                        title: if prefs.comic_rtl {
                            translate(locale, "reader.dir_rtl_title")
                        } else {
                            translate(locale, "reader.dir_ltr_title")
                        },
                        onclick: move |_| {
                            let mut mut_sess = session.write();
                            if let Some(s) = mut_sess.as_mut() {
                                s.prefs.comic_rtl = !s.prefs.comic_rtl;
                                s.save_prefs();
                            }
                        },
                        if prefs.comic_rtl {
                            {translate(locale, "reader.dir_rtl_btn")}
                        } else {
                            {translate(locale, "reader.dir_ltr_btn")}
                        }
                    }

                    button {
                        class: "btn btn-secondary",
                        style: "font-size: 11px; padding: 4px 10px;",
                        title: match prefs.comic_fit_mode {
                            ComicFitMode::FitHeight => translate(locale, "reader.fit_height_title"),
                            ComicFitMode::FitWidth => translate(locale, "reader.fit_width_title"),
                            ComicFitMode::Original => translate(locale, "reader.fit_original_title"),
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
                            ComicFitMode::FitHeight => translate(locale, "reader.fit_height_btn"),
                            ComicFitMode::FitWidth => translate(locale, "reader.fit_width_btn"),
                            ComicFitMode::Original => translate(locale, "reader.fit_original_btn"),
                        }
                    }
                } else {
                    div {
                        style: "display: flex; align-items: center; gap: 6px; padding: 4px 10px; background: rgba(255,255,255,0.05); border-radius: 14px; font-size: 11px; color: var(--text-secondary); cursor: default;",
                        title: "{translate_fmt(locale, \"reader.eta_title\", &[(\"ch\", &time_ch), (\"book\", &time_book), (\"wpm\", &prefs.words_per_minute.to_string())])}",
                        span { "⏱" }
                        span { "{time_ch}" }
                    }

                    div { style: "display: flex; align-items: center; gap: 2px;",
                        button {
                            class: "btn btn-icon",
                            title: translate(locale, "reader.font_smaller"),
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
                            title: translate(locale, "reader.font_larger"),
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

                button {
                    class: if current_drawer.as_deref() == Some("settings") { "btn btn-secondary active" } else { "btn btn-secondary" },
                    title: translate(locale, "reader.settings_title"),
                    onclick: move |_| {
                        let cur = active_drawer.read().clone();
                        if cur.as_deref() == Some("settings") {
                            active_drawer.set(None);
                        } else {
                            active_drawer.set(Some("settings".to_string()));
                        }
                    },
                    {translate(locale, "reader.settings_btn")}
                }

                button {
                    class: "btn btn-icon",
                    title: translate(locale, "reader.immersive_title"),
                    onclick: move |_| show_chrome.set(false),
                    "⛶"
                }
            }
        }
    }
}
