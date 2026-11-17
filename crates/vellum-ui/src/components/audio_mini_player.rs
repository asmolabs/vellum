//! Mini-lecteur audio persistant en bas de fenêtre (inspiré de BookOrbit Audiobook Player).

use crate::components::{IconKind, LucideIcon};
use crate::cover_cache::resolve_cover_src;
use dioxus::prelude::*;
use vellum_domain::Book;

#[component]
pub fn AudioMiniPlayer(
    book: Book,
    on_close: EventHandler<()>,
    on_open_reader: EventHandler<Book>,
) -> Element {
    let mut is_playing = use_signal(|| true);
    let mut speed = use_signal(|| 1.0_f32);
    let mut sleep_minutes = use_signal(|| 0_u32);
    let cover_src = resolve_cover_src(&book.id, book.cover_path.as_deref(), true);

    let audio_src = book
        .formats
        .iter()
        .find(|f| {
            matches!(
                f.format.to_ascii_lowercase().as_str(),
                "m4b" | "m4a" | "mp3" | "opus" | "ogg" | "flac" | "aac" | "wav"
            )
        })
        .map(|f| format!("file://{}", f.filepath));

    rsx! {
        div {
            style: "position: fixed; bottom: 14px; left: 50%; transform: translateX(-50%); width: min(720px, 94vw); background: rgba(15, 23, 42, 0.96); backdrop-filter: blur(14px); border: 1px solid rgba(99, 102, 241, 0.45); border-radius: 14px; padding: 10px 14px; box-shadow: 0 18px 48px rgba(0, 0, 0, 0.6); z-index: 60; display: flex; align-items: center; gap: 12px;",

            // Cover
            div {
                style: "width: 42px; height: 42px; border-radius: 8px; overflow: hidden; background: #1e1b4b; flex-shrink: 0; cursor: pointer;",
                onclick: {
                    let b = book.clone();
                    move |_| on_open_reader.call(b.clone())
                },
                if let Some(src) = cover_src {
                    img { src: "{src}", style: "width: 100%; height: 100%; object-fit: cover;" }
                } else {
                    div { style: "width: 100%; height: 100%; display: flex; align-items: center; justify-content: center; color: #a5b4fc;",
                        LucideIcon { kind: IconKind::Headphones, size: 18 }
                    }
                }
            }

            // Title & Author
            div { style: "flex: 1; min-width: 0;",
                div { style: "font-size: 13px; font-weight: 700; color: #f8fafc; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;",
                    "{book.title}"
                }
                div { style: "font-size: 11px; color: #94a3b8; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;",
                    "{book.author}"
                }
            }

            // Controls: Play/Pause + Speed + Sleep Timer
            div { style: "display: flex; align-items: center; gap: 8px;",
                button {
                    style: "width: 34px; height: 34px; border-radius: 999px; background: #6366f1; color: #fff; border: none; display: inline-flex; align-items: center; justify-content: center; cursor: pointer;",
                    onclick: move |_| {
                        let next = !*is_playing.read();
                        is_playing.set(next);
                        let js = if next {
                            "document.getElementById('vellum-mini-audio')?.play().catch(()=>{});"
                        } else {
                            "document.getElementById('vellum-mini-audio')?.pause();"
                        };
                        let _ = document::eval(js);
                    },
                    if *is_playing.read() {
                        LucideIcon { kind: IconKind::Pause, size: 15 }
                    } else {
                        LucideIcon { kind: IconKind::Play, size: 15 }
                    }
                }

                // Speed cycle
                button {
                    title: "Vitesse de lecture",
                    style: "padding: 4px 8px; border-radius: 6px; background: rgba(255,255,255,0.07); color: #e2e8f0; border: 1px solid rgba(255,255,255,0.12); font-size: 11px; font-weight: 700; cursor: pointer;",
                    onclick: move |_| {
                        let next = match *speed.read() {
                            s if s < 1.1 => 1.25,
                            s if s < 1.3 => 1.5,
                            s if s < 1.6 => 2.0,
                            _ => 1.0,
                        };
                        speed.set(next);
                        let js = format!(
                            "const a = document.getElementById('vellum-mini-audio'); if(a) a.playbackRate = {next};"
                        );
                        let _ = document::eval(&js);
                    },
                    "{*speed.read():.2}x"
                }

                // Sleep timer cycle
                button {
                    title: "Minuterie de sommeil",
                    style: if *sleep_minutes.read() > 0 {
                        "padding: 4px 8px; border-radius: 6px; background: rgba(99, 102, 241, 0.25); color: #c7d2fe; border: 1px solid rgba(99, 102, 241, 0.45); font-size: 11px; font-weight: 600; cursor: pointer;"
                    } else {
                        "padding: 4px 8px; border-radius: 6px; background: rgba(255,255,255,0.07); color: #94a3b8; border: 1px solid rgba(255,255,255,0.12); font-size: 11px; cursor: pointer;"
                    },
                    onclick: move |_| {
                        let next = match *sleep_minutes.read() {
                            0 => 15,
                            15 => 30,
                            30 => 60,
                            _ => 0,
                        };
                        sleep_minutes.set(next);
                    },
                    if *sleep_minutes.read() > 0 {
                        "🌙 {*sleep_minutes.read()}m"
                    } else {
                        "🌙 Off"
                    }
                }

                if let Some(src) = audio_src {
                    audio {
                        id: "vellum-mini-audio",
                        src: "{src}",
                        autoplay: true,
                        style: "display: none;",
                    }
                }

                button {
                    title: "Fermer le lecteur",
                    style: "background: none; border: none; color: #94a3b8; cursor: pointer; font-size: 14px; padding: 4px;",
                    onclick: move |_| on_close.call(()),
                    "✕"
                }
            }
        }
    }
}
