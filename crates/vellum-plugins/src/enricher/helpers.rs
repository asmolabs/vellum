use regex::Regex;
use serde_json::Value;
use std::sync::LazyLock;

pub(super) fn download_image(client: &reqwest::blocking::Client, url: &str) -> Option<Vec<u8>> {
    let resp = client.get(url).send().ok()?;
    if !resp.status().is_success() {
        return None;
    }
    resp.bytes().ok().map(|b| b.to_vec())
}

pub(super) fn detect_image_extension(bytes: &[u8], url: &str) -> &'static str {
    if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        ".png"
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        ".jpg"
    } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        ".webp"
    } else if url.contains(".png") {
        ".png"
    } else if url.contains(".webp") {
        ".webp"
    } else {
        ".jpg"
    }
}

pub(super) fn clean_string(val: Option<&Value>) -> String {
    val.and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string()
}

pub(super) fn clean_term(s: &str) -> String {
    s.replace(['"', '\'', '(', ')', '[', ']', '{', '}'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn clean_tag(s: &str) -> String {
    let t = s.trim().trim_matches(&['"', '\'', '#'][..]);
    if t.is_empty() {
        return String::new();
    }
    let mut chars = t.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => {
            let mut capitalized = first.to_uppercase().to_string();
            capitalized.push_str(chars.as_str());
            capitalized
        }
    }
}

pub(super) fn is_generic_value(s: &str) -> bool {
    let lower = s.trim().to_lowercase();
    matches!(
        lower.as_str(),
        "unknown"
            | "unknown author"
            | "sans titre"
            | "inconnu"
            | "auteur inconnu"
            | "n/a"
            | "none"
            | "livre"
            | "book"
    )
}

pub(super) fn strip_html(raw: &str) -> String {
    static TAG_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"<[^>]*>").expect("valid html strip regex"));
    let stripped = TAG_RE.replace_all(raw, " ");
    stripped
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn url_encode(s: &str) -> String {
    let mut encoded = String::new();
    for b in s.as_bytes() {
        match *b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(*b as char);
            }
            b' ' => encoded.push('+'),
            _ => {
                encoded.push_str(&format!("%{:02X}", b));
            }
        }
    }
    encoded
}
