use crate::state::ReaderSession;
use dioxus::prelude::*;
use vellum_core::i18n::{translate, translate_fmt, Locale};
use vellum_core::reader_store::{ComicFitMode, ComicReadingMode, ReaderPreferences};

#[component]
pub(super) fn ReaderSettingsDrawer(
    mut session: Signal<Option<ReaderSession>>,
    locale: Locale,
    mut active_drawer: Signal<Option<String>>,
    mut toast_msg: Signal<Option<String>>,
    prefs: ReaderPreferences,
    is_comic: bool,
    book_title: String,
) -> Element {
    rsx! {
        div {
            class: "reader-settings-backdrop",
            onclick: move |_| active_drawer.set(None),

            div {
                class: "reader-settings-drawer",
                onclick: move |e| e.stop_propagation(),

                div { style: "padding: 16px 20px; border-bottom: 1px solid var(--border-subtle); display: flex; align-items: center; justify-content: space-between;",
                    span { style: "font-weight: 600; font-size: 14px; color: var(--text-primary);", {translate(locale, "reader.settings_header")} }
                    button {
                        class: "btn btn-icon",
                        onclick: move |_| active_drawer.set(None),
                        "✕"
                    }
                }

                div { class: "settings-group",
                    span { class: "settings-label", {translate(locale, "reader.theme_label")} }
                    div { style: "display: grid; grid-template-columns: repeat(4, 1fr); gap: 6px;",
                        for (theme_id, label, extra_style) in [
                            ("dark", translate(locale, "reader.theme_dark"), ""),
                            ("sepia", translate(locale, "reader.theme_sepia"), " background: #3d2b1f; color: #f8f0dc;"),
                            ("light", translate(locale, "reader.theme_light"), " background: #e8edf2; color: #1e293b;"),
                            ("amoled", translate(locale, "reader.theme_amoled"), " background: #000; color: #fff;"),
                        ] {
                            button {
                                class: if prefs.theme == theme_id { "btn btn-secondary active" } else { "btn btn-secondary" },
                                style: "font-size: 11px; padding: 8px 4px; text-align: center;{extra_style}",
                                onclick: move |_| {
                                    let mut mut_sess = session.write();
                                    if let Some(s) = mut_sess.as_mut() {
                                        s.prefs.theme = theme_id.to_string();
                                        s.theme = theme_id.to_string();
                                        s.save_prefs();
                                    }
                                },
                                "{label}"
                            }
                        }
                    }
                }

                if is_comic {
                    div { class: "settings-group",
                        span { class: "settings-label", {translate(locale, "reader.comic_mode_label")} }
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
                                {translate(locale, "reader.comic_1p_btn")}
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
                                {translate(locale, "reader.comic_2p_btn")}
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
                                {translate(locale, "reader.comic_webtoon_btn")}
                            }
                        }
                        p { style: "font-size: 11px; color: var(--text-muted); margin: 0; line-height: 1.4;",
                            match prefs.comic_reading_mode {
                                ComicReadingMode::SinglePage => translate(locale, "reader.comic_single_desc"),
                                ComicReadingMode::DoublePage => translate(locale, "reader.comic_double_desc"),
                                ComicReadingMode::Webtoon => translate(locale, "reader.comic_webtoon_desc"),
                            }
                        }
                    }

                    div { class: "settings-group",
                        span { class: "settings-label", {translate(locale, "reader.direction_label")} }
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
                                {translate(locale, "reader.dir_ltr_btn")}
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
                                {translate(locale, "reader.dir_rtl_btn")}
                            }
                        }
                    }

                    div { class: "settings-group",
                        span { class: "settings-label", {translate(locale, "reader.fit_label")} }
                        div { style: "display: flex; gap: 6px;",
                            for (mode, label) in [
                                (ComicFitMode::FitHeight, translate(locale, "reader.fit_height_btn")),
                                (ComicFitMode::FitWidth, translate(locale, "reader.fit_width_btn")),
                                (ComicFitMode::Original, translate(locale, "reader.fit_original_short")),
                            ] {
                                button {
                                    class: if prefs.comic_fit_mode == mode { "btn btn-secondary active" } else { "btn btn-secondary" },
                                    style: "flex: 1; font-size: 11px;",
                                    onclick: move |_| {
                                        let mut mut_sess = session.write();
                                        if let Some(s) = mut_sess.as_mut() {
                                            s.prefs.comic_fit_mode = mode;
                                            s.save_prefs();
                                        }
                                    },
                                    "{label}"
                                }
                            }
                        }
                    }
                } else {
                    div { class: "settings-group",
                        span { class: "settings-label", {translate(locale, "reader.font_label")} }
                        div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 6px;",
                            for (font_id, label, serif_flag, style_str) in [
                                ("literata", "Literata", true, "font-family: Georgia, serif; font-size: 12px;"),
                                ("merriweather", "Merriweather", true, "font-family: Georgia, serif; font-size: 12px;"),
                                ("serif", "Georgia", true, "font-family: Georgia, serif; font-size: 12px;"),
                                ("sans", translate(locale, "reader.font_sans"), false, "font-family: sans-serif; font-size: 12px;"),
                                ("dyslexic", translate(locale, "reader.font_dyslexic"), false, "font-size: 12px; grid-column: span 2;"),
                            ] {
                                button {
                                    class: if prefs.font_family == font_id { "btn btn-secondary active" } else { "btn btn-secondary" },
                                    style: "{style_str}",
                                    onclick: move |_| {
                                        let mut mut_sess = session.write();
                                        if let Some(s) = mut_sess.as_mut() {
                                            s.prefs.font_family = font_id.to_string();
                                            s.is_serif = serif_flag;
                                            s.save_prefs();
                                        }
                                    },
                                    "{label}"
                                }
                            }
                        }
                    }

                    div { class: "settings-group",
                        div { style: "display: flex; justify-content: space-between; align-items: center;",
                            span { class: "settings-label", {translate(locale, "reader.font_size_label")} }
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

                    div { class: "settings-group",
                        div { style: "display: flex; justify-content: space-between; align-items: center;",
                            span { class: "settings-label", {translate(locale, "reader.line_height_label")} }
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

                    div { class: "settings-group",
                        div { style: "display: flex; justify-content: space-between; align-items: center;",
                            span { class: "settings-label", {translate(locale, "reader.margins_label")} }
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

                    div { class: "settings-group",
                        span { class: "settings-label", {translate(locale, "reader.layout_label")} }
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
                                if prefs.justify_text {
                                    {translate(locale, "reader.justify_on")}
                                } else {
                                    {translate(locale, "reader.justify_off")}
                                }
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
                                if prefs.two_columns {
                                    {translate(locale, "reader.spread_on")}
                                } else {
                                    {translate(locale, "reader.spread_off")}
                                }
                            }
                        }
                    }
                }

                div { class: "settings-group",
                    div { style: "display: flex; justify-content: space-between; align-items: center;",
                        span { class: "settings-label", {translate(locale, "reader.warmth_label")} }
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

                div { class: "settings-group",
                    div { style: "display: flex; justify-content: space-between; align-items: center;",
                        span { class: "settings-label", {translate(locale, "reader.brightness_label")} }
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

                div { class: "settings-group",
                    div { style: "display: flex; justify-content: space-between; align-items: center;",
                        span { class: "settings-label", {translate(locale, "reader.wpm_label")} }
                        span { style: "font-size: 12px; font-weight: 600; color: var(--accent-primary);",
                            {translate_fmt(locale, "reader.wpm_unit", &[("wpm", &prefs.words_per_minute.to_string())])}
                        }
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

                div { class: "settings-group",
                    span { class: "settings-label", {translate(locale, "reader.export_label")} }
                    button {
                        class: "btn btn-primary",
                        style: "width: 100%; font-size: 12px; padding: 10px;",
                        onclick: move |_| {
                            let sess_ref = session.read();
                            if let Some(s) = sess_ref.as_ref() {
                                let md = s.export_annotations_markdown();
                                let file_default = format!("notes_{}.md", book_title.replace(' ', "_"));
                                if let Some(dest) =
                                    crate::platform::save_file(&file_default, &file_default, &[])
                                {
                                    if std::fs::write(&dest, md).is_ok() {
                                        toast_msg.set(Some(translate(locale, "reader.export_notes_toast").to_string()));
                                    }
                                }
                            }
                        },
                        {translate(locale, "reader.export_notes_btn")}
                    }
                }
            }
        }
    }
}
