//! Aides de navigation, raccourcis clavier et barre inférieure du lecteur.

use dioxus::prelude::*;
use vellum_core::i18n::{translate, translate_fmt, Locale};
use vellum_core::reader_store::{ComicReadingMode, ReaderPreferences};

use crate::state::ReaderSession;

pub(super) fn step_page(
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

pub(super) struct ReaderKeyContext {
    pub session: Signal<Option<ReaderSession>>,
    pub active_drawer: Signal<Option<String>>,
    pub show_chrome: Signal<bool>,
    pub selected_text: Signal<Option<String>>,
    pub note_mode: Signal<bool>,
    pub ai_modal_open: Signal<bool>,
    pub is_comic: bool,
    pub comic_rtl: bool,
    pub on_close: EventHandler<()>,
    pub on_progress_update: EventHandler<(usize, usize, f64)>,
}

pub(super) fn handle_reader_keydown(key: &str, mut ctx: ReaderKeyContext) {
    if ctx.active_drawer.read().is_some() || *ctx.note_mode.read() || *ctx.ai_modal_open.read() {
        if key == "Escape" {
            if *ctx.ai_modal_open.read() {
                ctx.ai_modal_open.set(false);
            } else if *ctx.note_mode.read() {
                ctx.note_mode.set(false);
            } else {
                ctx.active_drawer.set(None);
            }
        }
        return;
    }

    match key {
        "ArrowLeft" | "PageUp" => {
            let fwd = ctx.is_comic && ctx.comic_rtl;
            step_page(ctx.session, ctx.on_progress_update, fwd);
        }
        "ArrowRight" | "PageDown" | " " | "Enter" => {
            let fwd = !(ctx.is_comic && ctx.comic_rtl && key == "ArrowRight");
            step_page(ctx.session, ctx.on_progress_update, fwd);
        }
        "Backspace" => {
            step_page(ctx.session, ctx.on_progress_update, false);
        }
        "Escape" => {
            if ctx.selected_text.read().is_some() {
                ctx.selected_text.set(None);
            } else {
                ctx.on_close.call(());
            }
        }
        "b" | "B" => {
            let mut mut_sess = ctx.session.write();
            if let Some(s) = mut_sess.as_mut() {
                s.toggle_current_bookmark();
            }
        }
        "t" | "T" => {
            let cur = ctx.active_drawer.read().clone();
            ctx.active_drawer.set(if cur.as_deref() == Some("toc") {
                None
            } else {
                Some("toc".to_string())
            });
        }
        "s" | "S" => {
            let cur = ctx.active_drawer.read().clone();
            ctx.active_drawer.set(if cur.as_deref() == Some("search") {
                None
            } else {
                Some("search".to_string())
            });
        }
        "m" | "M" | "f" | "F" => {
            let cur = *ctx.show_chrome.read();
            ctx.show_chrome.set(!cur);
        }
        "+" | "=" => adjust_font_size(ctx.session, 2),
        "-" => adjust_font_size(ctx.session, -2),
        _ => {}
    }
}

fn adjust_font_size(mut session: Signal<Option<ReaderSession>>, delta: i32) {
    let mut mut_sess = session.write();
    if let Some(s) = mut_sess.as_mut() {
        let next = (s.prefs.font_size as i32 + delta).clamp(12, 36) as u32;
        if next != s.prefs.font_size {
            s.prefs.font_size = next;
            s.font_size = next;
            s.save_prefs();
        }
    }
}

pub(super) struct FooterInfo<'a> {
    pub locale: Locale,
    pub is_comic: bool,
    pub prefs: &'a ReaderPreferences,
    pub cur_page: usize,
    pub total_pages: usize,
    pub progress: f64,
    pub page_in_ch: usize,
    pub total_in_ch: usize,
    pub chapter_title: &'a str,
}

pub(super) fn format_reader_footers(info: FooterInfo<'_>) -> (String, String) {
    let FooterInfo {
        locale,
        is_comic,
        prefs,
        cur_page,
        total_pages,
        progress,
        page_in_ch,
        total_in_ch,
        chapter_title,
    } = info;
    let total_str = total_pages.to_string();
    let total_max_str = total_pages.max(1).to_string();
    let cur_str = (cur_page + 1).to_string();
    let footer_left = if is_comic {
        let dir_label = if prefs.comic_rtl {
            translate(locale, "reader.dir_rtl")
        } else {
            translate(locale, "reader.dir_ltr")
        };
        match prefs.comic_reading_mode {
            ComicReadingMode::Webtoon => {
                translate_fmt(locale, "reader.footer_webtoon", &[("total", &total_str)])
            }
            ComicReadingMode::DoublePage => {
                let next_p = (cur_page + 2).min(total_pages);
                if next_p > cur_page + 1 {
                    let end_str = next_p.to_string();
                    translate_fmt(
                        locale,
                        "reader.footer_double",
                        &[
                            ("start", &cur_str),
                            ("end", &end_str),
                            ("total", &total_str),
                            ("dir", dir_label),
                        ],
                    )
                } else {
                    translate_fmt(
                        locale,
                        "reader.footer_single",
                        &[("cur", &cur_str), ("total", &total_str), ("dir", dir_label)],
                    )
                }
            }
            ComicReadingMode::SinglePage => translate_fmt(
                locale,
                "reader.footer_single",
                &[("cur", &cur_str), ("total", &total_str), ("dir", dir_label)],
            ),
        }
    } else {
        let page_str = page_in_ch.to_string();
        let total_ch_str = total_in_ch.to_string();
        translate_fmt(
            locale,
            "reader.footer_epub",
            &[
                ("page", &page_str),
                ("total", &total_ch_str),
                ("chapter", chapter_title),
            ],
        )
    };

    let footer_right = if is_comic && prefs.comic_reading_mode == ComicReadingMode::Webtoon {
        translate_fmt(
            locale,
            "reader.footer_right_webtoon",
            &[("total", &total_str)],
        )
    } else {
        let pct_str = format!("{progress:.0}");
        translate_fmt(
            locale,
            "reader.footer_right_page",
            &[
                ("cur", &cur_str),
                ("total", &total_max_str),
                ("pct", &pct_str),
            ],
        )
    };

    (footer_left, footer_right)
}
