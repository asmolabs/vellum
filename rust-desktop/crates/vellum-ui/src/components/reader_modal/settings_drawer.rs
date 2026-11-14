use crate::state::ReaderSession;
use dioxus::prelude::*;
use vellum_core::reader_store::{ComicFitMode, ComicReadingMode, ReaderPreferences};

#[component]
pub(super) fn ReaderSettingsDrawer(
    mut session: Signal<Option<ReaderSession>>,
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
                    span { style: "font-weight: 600; font-size: 14px; color: var(--text-primary);", "Préférences de lecture" }
                    button {
                        class: "btn btn-icon",
                        onclick: move |_| active_drawer.set(None),
                        "✕"
                    }
                }

                div { class: "settings-group",
                    span { class: "settings-label", "Thème d'affichage" }
                    div { style: "display: grid; grid-template-columns: repeat(4, 1fr); gap: 6px;",
                        for (theme_id, label, extra_style) in [
                            ("dark", "🌙 Sombre", ""),
                            ("sepia", "📜 Sépia", " background: #3d2b1f; color: #f8f0dc;"),
                            ("light", "☀️ Clair", " background: #e8edf2; color: #1e293b;"),
                            ("amoled", "⬛ Noir", " background: #000; color: #fff;"),
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
                            for (mode, label) in [
                                (ComicFitMode::FitHeight, "↕ Hauteur"),
                                (ComicFitMode::FitWidth, "↔ Largeur"),
                                (ComicFitMode::Original, "Taille 100%"),
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
                        span { class: "settings-label", "Police de caractères" }
                        div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 6px;",
                            for (font_id, label, serif_flag, style_str) in [
                                ("literata", "Literata", true, "font-family: Georgia, serif; font-size: 12px;"),
                                ("merriweather", "Merriweather", true, "font-family: Georgia, serif; font-size: 12px;"),
                                ("serif", "Georgia", true, "font-family: Georgia, serif; font-size: 12px;"),
                                ("sans", "Sans-serif", false, "font-family: sans-serif; font-size: 12px;"),
                                ("dyslexic", "OpenDyslexic (Accessibilité)", false, "font-size: 12px; grid-column: span 2;"),
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

                div { class: "settings-group",
                    span { class: "settings-label", "Sauvegarde & Export" }
                    button {
                        class: "btn btn-primary",
                        style: "width: 100%; font-size: 12px; padding: 10px;",
                        onclick: move |_| {
                            let sess_ref = session.read();
                            if let Some(s) = sess_ref.as_ref() {
                                let md = s.export_annotations_markdown();
                                let file_default = format!("notes_{}.md", book_title.replace(' ', "_"));
                                if let Some(dest) = rfd::FileDialog::new()
                                    .set_file_name(&file_default)
                                    .save_file()
                                {
                                    if std::fs::write(&dest, md).is_ok() {
                                        toast_msg.set(Some("Notes exportées avec succès !".to_string()));
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
}
