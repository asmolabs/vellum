//! Centre d'Upload & Téléchargement externe (Livres, Livres audio & Comics).

use std::sync::Arc;

use dioxus::prelude::*;
use vellum_core::BookService;

use crate::components::{IconKind, LucideIcon};

#[component]
pub fn UploadModal(
    book_service: Arc<BookService>,
    on_close: EventHandler<()>,
    on_pick_files: EventHandler<()>,
    on_pick_folder: EventHandler<()>,
    on_imported: EventHandler<String>,
) -> Element {
    let mut tab = use_signal(|| "files");
    let mut custom_url = use_signal(String::new);
    let mut custom_name = use_signal(|| "book.epub".to_string());
    let mut downloading_key = use_signal(|| None::<String>);
    let mut status_msg = use_signal(|| None::<(bool, String)>);

    let catalog = [
        (
            "frankenstein",
            "EPUB",
            "Frankenstein — Mary Shelley",
            "Gutenberg • Livre classique",
            "https://www.gutenberg.org/ebooks/84.epub3.images",
            "Frankenstein.epub",
        ),
        (
            "dracula",
            "EPUB",
            "Dracula — Bram Stoker",
            "Gutenberg • Livre classique",
            "https://www.gutenberg.org/ebooks/345.epub3.images",
            "Dracula.epub",
        ),
        (
            "sherlock",
            "EPUB",
            "The Adventures of Sherlock Holmes",
            "Gutenberg • Livre classique",
            "https://www.gutenberg.org/ebooks/1661.epub3.images",
            "Sherlock_Holmes.epub",
        ),
        (
            "alice_audio",
            "MP3",
            "Alice's Adventures in Wonderland (Ch. 1)",
            "LibriVox • Livre audio libre",
            "https://archive.org/download/alice_wonderland_0711_librivox/wonderland_ch_01_64kb.mp3",
            "Alice_Wonderland_Ch01.mp3",
        ),
    ];

    rsx! {
        div {
            class: "modal-overlay",
            onclick: move |_| on_close.call(()),
            div {
                style: "width: min(620px, 94vw); background: var(--bg-surface); border: 1px solid var(--border-active); border-radius: 14px; padding: 20px; box-shadow: 0 24px 64px rgba(0,0,0,0.6); display: flex; flex-direction: column; gap: 16px;",
                onclick: move |e| e.stop_propagation(),

                // Header
                div { style: "display: flex; align-items: center; justify-content: space-between;",
                    div { style: "display: flex; align-items: center; gap: 10px;",
                        span {
                            style: "width: 32px; height: 32px; border-radius: 8px; background: var(--accent-glow); color: var(--accent-primary); display: inline-flex; align-items: center; justify-content: center;",
                            LucideIcon { kind: IconKind::Upload, size: 17 }
                        }
                        div {
                            h3 { style: "margin: 0; font-size: 16px; font-weight: 700; color: var(--text-primary);", "Importer / Télécharger des médias" }
                            div { style: "font-size: 12px; color: var(--text-muted);", "Livres (EPUB, PDF), Livres audio (M4B, MP3) et Comics (CBZ, CBR)" }
                        }
                    }
                    button {
                        class: "btn btn-icon",
                        onclick: move |_| on_close.call(()),
                        LucideIcon { kind: IconKind::X, size: 15 }
                    }
                }

                // Tabs
                div { style: "display: flex; gap: 6px; padding: 4px; border-radius: 10px; background: var(--bg-primary); border: 1px solid var(--border-subtle);",
                    for (id, label, icon) in [
                        ("files", "Fichiers locaux", IconKind::Upload),
                        ("folder", "Dossier Audio / Comics", IconKind::FolderPlus),
                        ("url", "Télécharger (URL / Libre)", IconKind::Download),
                    ] {
                        {
                            let active = *tab.read() == id;
                            rsx! {
                                button {
                                    style: if active {
                                        "flex: 1; padding: 7px 10px; border-radius: 7px; border: none; background: var(--accent-primary); color: #fff; font-size: 12px; font-weight: 600; display: flex; align-items: center; justify-content: center; gap: 6px; cursor: pointer;"
                                    } else {
                                        "flex: 1; padding: 7px 10px; border-radius: 7px; border: none; background: transparent; color: var(--text-secondary); font-size: 12px; display: flex; align-items: center; justify-content: center; gap: 6px; cursor: pointer;"
                                    },
                                    onclick: move |_| tab.set(id),
                                    LucideIcon { kind: icon, size: 14 }
                                    span { "{label}" }
                                }
                            }
                        }
                    }
                }

                if *tab.read() == "files" {
                    div {
                        style: "padding: 28px 20px; border-radius: 12px; border: 2px dashed var(--border-active); background: var(--bg-primary); display: flex; flex-direction: column; align-items: center; text-align: center; gap: 10px;",
                        LucideIcon { kind: IconKind::Upload, size: 28 }
                        div { style: "font-size: 14px; font-weight: 600; color: var(--text-primary);", "Sélectionner un ou plusieurs fichiers" }
                        div { style: "font-size: 12px; color: var(--text-muted); max-width: 420px;",
                            "Livres : EPUB, PDF, MOBI, AZW3, TXT  •  Audio : M4B, MP3, M4A, FLAC, OPUS  •  Comics : CBZ, CBR, CB7"
                        }
                        button {
                            class: "btn btn-primary",
                            style: "margin-top: 6px; padding: 8px 18px;",
                            onclick: move |_| {
                                on_pick_files.call(());
                                on_close.call(());
                            },
                            LucideIcon { kind: IconKind::Plus, size: 15 }
                            span { "Choisir des fichiers..." }
                        }
                    }
                } else if *tab.read() == "folder" {
                    div {
                        style: "padding: 24px 20px; border-radius: 12px; border: 1px solid var(--border-subtle); background: var(--bg-primary); display: flex; flex-direction: column; gap: 12px;",
                        div { style: "display: flex; align-items: flex-start; gap: 12px;",
                            span { style: "padding: 8px; border-radius: 8px; background: var(--accent-glow); color: var(--accent-primary);",
                                LucideIcon { kind: IconKind::Headphones, size: 18 }
                            }
                            div {
                                div { style: "font-size: 13.5px; font-weight: 700; color: var(--text-primary);", "Dossier de Livre audio multi-pistes" }
                                div { style: "font-size: 12px; color: var(--text-muted);", "Regroupe automatiquement toutes les pistes MP3/M4A + cover.jpg en un seul livre audio." }
                            }
                        }
                        div { style: "display: flex; align-items: flex-start; gap: 12px;",
                            span { style: "padding: 8px; border-radius: 8px; background: var(--accent-glow); color: var(--accent-primary);",
                                LucideIcon { kind: IconKind::Image, size: 18 }
                            }
                            div {
                                div { style: "font-size: 13.5px; font-weight: 700; color: var(--text-primary);", "Dossier de Série Comics / Manga" }
                                div { style: "font-size: 12px; color: var(--text-muted);", "Importe tous les CBZ/CBR du dossier et associe automatiquement le nom du dossier comme série." }
                            }
                        }
                        button {
                            class: "btn btn-primary",
                            style: "justify-content: center; padding: 9px 16px; margin-top: 4px;",
                            onclick: move |_| {
                                on_pick_folder.call(());
                                on_close.call(());
                            },
                            LucideIcon { kind: IconKind::FolderPlus, size: 15 }
                            span { "Choisir un dossier à importer..." }
                        }
                    }
                } else {
                    div { style: "display: flex; flex-direction: column; gap: 12px;",
                        // URL directe
                        div { style: "display: flex; gap: 8px;",
                            input {
                                class: "form-input",
                                r#type: "text",
                                placeholder: "URL directe (.epub, .pdf, .cbz, .m4b, .mp3)...",
                                value: "{custom_url}",
                                oninput: move |e| custom_url.set(e.value()),
                            }
                            input {
                                class: "form-input",
                                style: "width: 150px;",
                                r#type: "text",
                                placeholder: "Nom.ext",
                                value: "{custom_name}",
                                oninput: move |e| custom_name.set(e.value()),
                            }
                            button {
                                class: "btn btn-primary",
                                disabled: downloading_key.read().is_some() || custom_url.read().trim().is_empty(),
                                onclick: {
                                    let bs = book_service.clone();
                                    move |_| {
                                        let u = custom_url.read().trim().to_string();
                                        let n = custom_name.read().trim().to_string();
                                        if u.is_empty() { return; }
                                        downloading_key.set(Some("custom".to_string()));
                                        let bs_c = bs.clone();
                                        spawn(async move {
                                            let res = tokio::task::spawn_blocking(move || bs_c.download_and_import_url(&u, &n)).await;
                                            downloading_key.set(None);
                                            match res {
                                                Ok(Ok(b)) => {
                                                    status_msg.set(Some((true, format!("Téléchargé : {}", b.title))));
                                                    on_imported.call(b.title);
                                                }
                                                Ok(Err(e)) => status_msg.set(Some((false, e.to_string()))),
                                                Err(e) => status_msg.set(Some((false, e.to_string()))),
                                            }
                                        });
                                    }
                                },
                                LucideIcon { kind: IconKind::Download, size: 14 }
                                span { "Importer" }
                            }
                        }

                        div { style: "font-size: 11px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em; color: var(--text-muted);",
                            "Catalogue Domaine Public (1 clic)"
                        }
                        div { style: "display: flex; flex-direction: column; gap: 6px; max-height: 220px; overflow-y: auto;",
                            for (key, fmt, title, sub, url, fname) in catalog {
                                {
                                    let is_busy = downloading_key.read().as_deref() == Some(key);
                                    let bs = book_service.clone();
                                    rsx! {
                                        div {
                                            key: "{key}",
                                            style: "display: flex; align-items: center; justify-content: space-between; gap: 10px; padding: 8px 12px; border-radius: 8px; background: var(--bg-primary); border: 1px solid var(--border-subtle);",
                                            div { style: "min-width: 0; flex: 1;",
                                                div { style: "font-size: 12.5px; font-weight: 600; color: var(--text-primary); display: flex; align-items: center; gap: 6px;",
                                                    span { class: "format-pill", style: "position: static;", "{fmt}" }
                                                    span { style: "white-space: nowrap; overflow: hidden; text-overflow: ellipsis;", "{title}" }
                                                }
                                                div { style: "font-size: 11px; color: var(--text-muted); margin-top: 2px;", "{sub}" }
                                            }
                                            button {
                                                class: "btn btn-secondary",
                                                style: "padding: 5px 10px; font-size: 11.5px;",
                                                disabled: downloading_key.read().is_some(),
                                                onclick: move |_| {
                                                    downloading_key.set(Some(key.to_string()));
                                                    let bs_c = bs.clone();
                                                    let u = url.to_string();
                                                    let n = fname.to_string();
                                                    spawn(async move {
                                                        let res = tokio::task::spawn_blocking(move || bs_c.download_and_import_url(&u, &n)).await;
                                                        downloading_key.set(None);
                                                        match res {
                                                            Ok(Ok(b)) => {
                                                                status_msg.set(Some((true, format!("Téléchargé : {}", b.title))));
                                                                on_imported.call(b.title);
                                                            }
                                                            Ok(Err(e)) => status_msg.set(Some((false, e.to_string()))),
                                                            Err(e) => status_msg.set(Some((false, e.to_string()))),
                                                        }
                                                    });
                                                },
                                                if is_busy { "⏳" } else { "Télécharger" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if let Some((ok, ref msg)) = *status_msg.read() {
                    div {
                        style: if ok {
                            "padding: 8px 12px; border-radius: 8px; background: rgba(16, 185, 129, 0.15); border: 1px solid rgba(16, 185, 129, 0.4); color: var(--success); font-size: 12px;"
                        } else {
                            "padding: 8px 12px; border-radius: 8px; background: rgba(239, 68, 68, 0.15); border: 1px solid rgba(239, 68, 68, 0.4); color: var(--danger); font-size: 12px;"
                        },
                        "{msg}"
                    }
                }
            }
        }
    }
}
