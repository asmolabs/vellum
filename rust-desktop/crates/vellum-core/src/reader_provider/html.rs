use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;

static RE_TAGS_STRIP: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<[^>]+>").expect("valid strip tag regex"));

pub fn html_to_plain_text(html: &str) -> String {
    static RE_IMG: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?is)<img[^>]*>").expect("valid img regex"));
    static RE_TAGS_DOTALL: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?s)<[^>]+>").expect("valid dotall tags regex"));
    static RE_NUM: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"&#(x?[0-9a-fA-F]+);").expect("valid entity num regex"));
    static RE_WS: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\s+").expect("valid whitespace regex"));

    let no_img = RE_IMG.replace_all(html, " ");
    let no_tags = RE_TAGS_DOTALL.replace_all(&no_img, " ");
    let decoded_num = RE_NUM.replace_all(&no_tags, |c: &regex::Captures| {
        let raw = &c[1];
        let code = if let Some(hex) = raw.strip_prefix('x').or_else(|| raw.strip_prefix('X')) {
            u32::from_str_radix(hex, 16).ok()
        } else {
            raw.parse::<u32>().ok()
        };
        code.and_then(char::from_u32)
            .map(|ch| ch.to_string())
            .unwrap_or_default()
    });
    let decoded = decoded_num
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&rsquo;", "’")
        .replace("&lsquo;", "‘")
        .replace("&hellip;", "…")
        .replace("&mdash;", "—")
        .replace("&ndash;", "–")
        .replace("&laquo;", "«")
        .replace("&raquo;", "»")
        .replace("&amp;", "&");
    RE_WS.replace_all(&decoded, " ").trim().to_string()
}

pub fn wrap_with_reader_theme(
    body_content: &str,
    _theme: &str,
    _font_size: u32,
    _is_serif: bool,
) -> String {
    let clean_body = sanitize_reader_html(body_content);
    format!(r#"<div class="reader-container">{}</div>"#, clean_body)
}

pub fn sanitize_reader_html(html: &str) -> String {
    static RE_BODY: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?is)<body[^>]*>(.*?)</body>").expect("valid body regex"));
    let body_only = RE_BODY
        .captures(html)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str())
        .unwrap_or(html);

    let mut cleaner = ammonia::Builder::default();
    cleaner
        .add_url_schemes(["data"])
        .add_generic_attributes(["class", "id", "alt", "title"])
        .add_tag_attributes("img", ["src", "alt", "width", "height", "loading"])
        .clean(body_only)
        .to_string()
        .trim()
        .to_string()
}

pub fn split_html_into_blocks(html: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    // Match block-level elements: <p>, <h1>..<h6>, <blockquote>, <li>, <table>, <figure>, <img>, <hr>
    // Note: <div> is intentionally NOT matched as a single atomic block, because in ePubs <div> often
    // wraps the entire chapter or multiple paragraphs, which would cause the entire chapter to be treated as 1 page!
    static RE_BLOCK: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r#"(?is)(<p[^>]*>.*?</p>|<h[1-6][^>]*>.*?</h[1-6]>|<blockquote[^>]*>.*?</blockquote>|<li[^>]*>.*?</li>|<table[^>]*>.*?</table>|<figure[^>]*>.*?</figure>|<img[^>]*>|<hr[^>]*>)"#,
        )
        .expect("valid block regex")
    });

    let re_strip = &*RE_TAGS_STRIP;
    let mut last_end = 0;
    for mat in RE_BLOCK.find_iter(html) {
        let inter = html[last_end..mat.start()].trim();
        if !inter.is_empty() {
            let stripped = re_strip.replace_all(inter, " ").trim().to_string();
            // If there's real printable text between recognized block tags, wrap it in a <p>
            if !stripped.is_empty() {
                blocks.push(format!("<p>{}</p>", inter));
            }
        }
        blocks.push(mat.as_str().trim().to_string());
        last_end = mat.end();
    }

    if last_end < html.len() {
        let tail = html[last_end..].trim();
        if !tail.is_empty() {
            let stripped = re_strip.replace_all(tail, " ").trim().to_string();
            if !stripped.is_empty() {
                blocks.push(format!("<p>{}</p>", tail));
            }
        }
    }

    if blocks.is_empty() && !html.trim().is_empty() {
        for p in html.split("\n\n") {
            let p_trim = p.trim();
            if !p_trim.is_empty() {
                blocks.push(format!("<p>{}</p>", p_trim));
            }
        }
    }

    blocks
}

pub fn paginate_html_into_pages(raw_html: &str, target_chars: usize) -> Vec<String> {
    let clean = sanitize_reader_html(raw_html);
    if clean.trim().is_empty() {
        return Vec::new();
    }

    let raw_blocks = split_html_into_blocks(&clean);
    let re_strip = &*RE_TAGS_STRIP;

    let mut pages = Vec::new();
    let mut current_page = Vec::new();
    let mut current_chars: usize = 0;

    for block in raw_blocks {
        let plain = re_strip.replace_all(&block, "");
        let is_image = block.contains("<img") || block.contains("<image");
        let is_heading = block.starts_with("<h1") || block.starts_with("<h2");
        let weight = if is_image { 600 } else { plain.len() };

        let heading_break = is_heading && current_chars >= 350;
        let size_break = (current_chars + weight > target_chars) && current_chars > 0;

        if (heading_break || size_break) && !current_page.is_empty() {
            pages.push(current_page.join("\n"));
            current_page.clear();
            current_chars = 0;
        }

        current_page.push(block);
        current_chars += weight;
    }

    if !current_page.is_empty() {
        pages.push(current_page.join("\n"));
    }

    if pages.is_empty() && !clean.trim().is_empty() {
        pages.push(clean);
    }

    pages
}

pub(super) fn normalize_zip_path(base_dir: &str, target_path: &str) -> String {
    let clean_target = target_path
        .split('?')
        .next()
        .unwrap_or(target_path)
        .split('#')
        .next()
        .unwrap_or(target_path);

    let mut parts: Vec<&str> = Vec::new();
    if !base_dir.is_empty() {
        for part in base_dir.split('/') {
            if !part.is_empty() && part != "." {
                parts.push(part);
            }
        }
    }

    for part in clean_target.split('/') {
        if part.is_empty() || part == "." {
            continue;
        } else if part == ".." {
            parts.pop();
        } else {
            parts.push(part);
        }
    }

    parts.join("/")
}

pub(super) fn replace_image_sources(
    html: &str,
    chapter_path: &str,
    image_map: &HashMap<String, String>,
) -> String {
    let ch_dir = Path::new(chapter_path)
        .parent()
        .and_then(|p| p.to_str())
        .unwrap_or("");

    static RE_ATTR: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?i)(src|xlink:href|href)\s*=\s*(?:"([^"]*)"|'([^']*)')"#)
            .expect("valid attr regex")
    });

    RE_ATTR
        .replace_all(html, |caps: &regex::Captures| {
            let attr = &caps[1];
            let (quote, val) = if let Some(v) = caps.get(2) {
                ("\"", v.as_str())
            } else if let Some(v) = caps.get(3) {
                ("'", v.as_str())
            } else {
                ("\"", "")
            };

            if val.starts_with("data:") || val.starts_with("http://") || val.starts_with("https://")
            {
                return format!("{}={}{}{}", attr, quote, val, quote);
            }

            let lower = val.to_lowercase();
            let is_img = lower.ends_with(".jpg")
                || lower.ends_with(".jpeg")
                || lower.ends_with(".png")
                || lower.ends_with(".webp")
                || lower.ends_with(".gif")
                || lower.ends_with(".svg")
                || attr.eq_ignore_ascii_case("src");

            if is_img {
                let normalized = normalize_zip_path(ch_dir, val);
                let file_name = Path::new(val)
                    .file_name()
                    .and_then(|f| f.to_str())
                    .unwrap_or("");

                if let Some(data_uri) = image_map
                    .get(&normalized)
                    .or_else(|| image_map.get(&normalized.to_lowercase()))
                    .or_else(|| image_map.get(val))
                    .or_else(|| image_map.get(&lower))
                    .or_else(|| image_map.get(file_name))
                    .or_else(|| image_map.get(&file_name.to_lowercase()))
                {
                    return format!("{}={}{}{}", attr, quote, data_uri, quote);
                }
            }

            format!("{}={}{}{}", attr, quote, val, quote)
        })
        .to_string()
}

pub(super) fn natural_sort_key(s: &str) -> Vec<(bool, String, u64)> {
    let mut chunks = Vec::new();
    let mut current_digits = String::new();
    let mut current_alpha = String::new();

    for c in s.chars() {
        if c.is_ascii_digit() {
            if !current_alpha.is_empty() {
                chunks.push((false, current_alpha.clone(), 0));
                current_alpha.clear();
            }
            current_digits.push(c);
        } else {
            if !current_digits.is_empty() {
                let num = current_digits.parse::<u64>().unwrap_or(0);
                chunks.push((true, current_digits.clone(), num));
                current_digits.clear();
            }
            current_alpha.push(c.to_ascii_lowercase());
        }
    }
    if !current_alpha.is_empty() {
        chunks.push((false, current_alpha, 0));
    }
    if !current_digits.is_empty() {
        let num = current_digits.parse::<u64>().unwrap_or(0);
        chunks.push((true, current_digits, num));
    }
    chunks
}

pub(super) fn extract_chapter_title(html: &str, fallback_index: usize) -> String {
    static RE_H1: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"(?is)<h1[^>]*>(.*?)</h1>"#).expect("valid h1 regex"));
    static RE_H2: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"(?is)<h2[^>]*>(.*?)</h2>"#).expect("valid h2 regex"));
    static RE_TITLE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?is)<title[^>]*>(.*?)</title>"#).expect("valid title regex")
    });

    let raw = if let Some(c) = RE_H1.captures(html) {
        Some(c[1].to_string())
    } else if let Some(c) = RE_H2.captures(html) {
        Some(c[1].to_string())
    } else {
        RE_TITLE.captures(html).map(|c| c[1].to_string())
    };

    if let Some(text) = raw {
        let clean = RE_TAGS_STRIP.replace_all(&text, " ").trim().to_string();
        if !clean.is_empty() && clean.len() < 80 {
            return clean;
        }
    }

    format!("Chapitre {}", fallback_index + 1)
}

// 1. EPUB Provider

pub(super) fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_reader_html_blocks_xss_and_preserves_data_images() {
        let malicious = r#"
            <html><head><title>Evil</title><style>body{display:none}</style></head>
            <body>
                <h1>Chapitre 1</h1>
                <p onclick="alert(1)">Texte normal <a href="javascript:alert(2)">lien</a></p>
                <img src="x" onerror="fetch('https://evil.test')" />
                <img src="data:image/png;base64,iVBORw0KGgo=" alt="Cover" />
                <svg onload="alert(3)"></svg>
                <iframe src="https://evil.test"></iframe>
                <script>window.pwn = true;</script>
            </body></html>
        "#;

        let clean = sanitize_reader_html(malicious);
        assert!(clean.contains("<h1>Chapitre 1</h1>"));
        assert!(clean.contains("Texte normal"));
        assert!(clean.contains("data:image/png;base64,iVBORw0KGgo="));
        assert!(!clean.contains("onerror"));
        assert!(!clean.contains("onclick"));
        assert!(!clean.contains("onload"));
        assert!(!clean.contains("javascript:"));
        assert!(!clean.contains("<script"));
        assert!(!clean.contains("<iframe"));
        assert!(!clean.contains("<svg"));
    }
}
