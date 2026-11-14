use crate::state::ReaderSession;
use dioxus::prelude::*;
use vellum_core::reader_store::{ComicFitMode, ComicReadingMode, ReaderPreferences};

#[component]
pub(super) fn ReaderToolbar(
    mut session: Signal<Option<ReaderSession>>,
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
                    "← Bibliothèque"
                }

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

                if has_history {
                    button {
                        class: "btn btn-icon",
                        title: "Page précédente de l'historique",
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
                } else {
                    div {
                        style: "display: flex; align-items: center; gap: 6px; padding: 4px 10px; background: rgba(255,255,255,0.05); border-radius: 14px; font-size: 11px; color: var(--text-secondary); cursor: default;",
                        title: "Temps restant estimé : {time_ch} dans ce chapitre • {time_book} pour finir le livre (basé sur {prefs.words_per_minute} mpm)",
                        span { "⏱" }
                        span { "{time_ch}" }
                    }

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

                button {
                    class: "btn btn-icon",
                    title: "Mode immersif (M / F)",
                    onclick: move |_| show_chrome.set(false),
                    "⛶"
                }
            }
        }
    }
}
