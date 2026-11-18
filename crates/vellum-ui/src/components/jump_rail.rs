//! Barre verticale alphabétique à droite (#, A–Z), inspirée de JumpRail dans BookOrbit.

use std::collections::HashSet;

use dioxus::prelude::*;

/// Extrait le bucket alphabétique ('A'..='Z' ou '#') d'un libellé.
pub fn letter_bucket(text: &str) -> char {
    let first = text
        .trim()
        .chars()
        .next()
        .unwrap_or('#')
        .to_ascii_uppercase();
    if first.is_ascii_alphabetic() {
        first
    } else {
        '#'
    }
}

#[component]
pub fn JumpRail(
    available_letters: HashSet<char>,
    mut active_letter: Signal<Option<char>>,
) -> Element {
    let cur = *active_letter.read();
    let mut slots: Vec<char> = Vec::with_capacity(27);
    if available_letters.contains(&'#') {
        slots.push('#');
    }
    for ch in 'A'..='Z' {
        slots.push(ch);
    }

    rsx! {
        nav {
            class: "jump-rail",
            aria_label: "Navigation par lettre",
            style: "position: sticky; top: 12px; align-self: flex-start; width: 30px; flex-shrink: 0; display: flex; flex-direction: column; align-items: center; gap: 1px; padding: 6px 3px; border-radius: 999px; background: var(--bg-surface); border: 1px solid var(--border-subtle); box-shadow: 0 6px 20px rgba(0,0,0,0.2); user-select: none;",
            if cur.is_some() {
                button {
                    title: "Tout afficher",
                    style: "width: 22px; height: 20px; border-radius: 999px; border: none; background: var(--accent-glow); color: var(--accent-primary); font-size: 10px; font-weight: 700; cursor: pointer; display: flex; align-items: center; justify-content: center; margin-bottom: 2px;",
                    onclick: move |_| active_letter.set(None),
                    "✕"
                }
            }
            for ch in slots {
                {
                    let has_items = available_letters.contains(&ch);
                    let is_active = cur == Some(ch);
                    let ch_str = ch.to_string();
                    let label = if has_items || is_active {
                        ch_str.clone()
                    } else {
                        "•".to_string()
                    };
                    let btn_style: &'static str = if is_active {
                        "width: 22px; height: 19px; border-radius: 999px; border: none; background: var(--accent-primary); color: #fff; font-size: 10.5px; font-weight: 700; cursor: pointer; display: flex; align-items: center; justify-content: center;"
                    } else if has_items {
                        "width: 22px; height: 19px; border-radius: 999px; border: none; background: transparent; color: var(--text-primary); font-size: 10.5px; font-weight: 600; cursor: pointer; display: flex; align-items: center; justify-content: center;"
                    } else {
                        "width: 22px; height: 15px; border-radius: 999px; border: none; background: transparent; color: var(--text-muted); opacity: 0.35; font-size: 9px; cursor: default; display: flex; align-items: center; justify-content: center;"
                    };
                    rsx! {
                        button {
                            key: "jr-{ch}",
                            disabled: !has_items && !is_active,
                            title: "{ch_str}",
                            style: btn_style,
                            onclick: move |_| {
                                if is_active {
                                    active_letter.set(None);
                                } else if has_items {
                                    active_letter.set(Some(ch));
                                }
                            },
                            "{label}"
                        }
                    }
                }
            }
        }
    }
}
