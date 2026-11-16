use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Context {
    pub before: String,
    pub after: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextSpan {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

pub struct ParsedHtmlDocument {
    raw: String,
    pub spans: Vec<TextSpan>,
}

impl ParsedHtmlDocument {
    pub fn parse(html: &str) -> Self {
        let mut spans = Vec::new();
        let bytes = html.as_bytes();
        let mut i = 0;
        let mut in_script_or_style = false;

        while i < bytes.len() {
            if bytes[i] == b'<' {
                let tag_start = i;
                while i < bytes.len() && bytes[i] != b'>' {
                    i += 1;
                }
                if i < bytes.len() {
                    i += 1; // include '>'
                }
                let tag_str = html[tag_start..i].to_lowercase();
                if tag_str.starts_with("<script") || tag_str.starts_with("<style") {
                    in_script_or_style = true;
                } else if tag_str.starts_with("</script") || tag_str.starts_with("</style") {
                    in_script_or_style = false;
                }
            } else {
                let text_start = i;
                while i < bytes.len() && bytes[i] != b'<' {
                    i += 1;
                }
                if !in_script_or_style && i > text_start {
                    let segment = &html[text_start..i];
                    if !segment.is_empty() {
                        spans.push(TextSpan {
                            start: text_start,
                            end: i,
                            text: segment.to_string(),
                        });
                    }
                }
            }
        }

        Self {
            raw: html.to_string(),
            spans,
        }
    }

    pub fn set_span_text(&mut self, index: usize, new_text: impl Into<String>) {
        if let Some(span) = self.spans.get_mut(index) {
            span.text = new_text.into();
        }
    }

    pub fn outer_html(&self) -> String {
        let mut result = String::with_capacity(self.raw.len() + 128);
        let mut last_idx = 0;
        for span in &self.spans {
            if span.start >= last_idx && span.end <= self.raw.len() {
                result.push_str(&self.raw[last_idx..span.start]);
                result.push_str(&span.text);
                last_idx = span.end;
            }
        }
        if last_idx < self.raw.len() {
            result.push_str(&self.raw[last_idx..]);
        }
        result
    }
}

pub fn should_translate(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return false;
    }
    trimmed.chars().any(|c| c.is_alphabetic())
}

pub fn get_context(spans: &[TextSpan], index: usize, max_len: usize) -> Context {
    if index >= spans.len() {
        return Context {
            before: String::new(),
            after: String::new(),
        };
    }

    let mut before_parts = Vec::new();
    let mut b_len = 0;
    let mut idx = index;
    while idx > 0 && b_len < max_len {
        idx -= 1;
        let t = spans[idx].text.trim();
        if !t.is_empty() {
            b_len += t.len() + 1;
            before_parts.push(t);
        }
    }
    before_parts.reverse();
    let before = before_parts.join(" ");

    let mut after_parts = Vec::new();
    let mut a_len = 0;
    let mut idx_a = index + 1;
    while idx_a < spans.len() && a_len < max_len {
        let t = spans[idx_a].text.trim();
        if !t.is_empty() {
            a_len += t.len() + 1;
            after_parts.push(t);
        }
        idx_a += 1;
    }
    let after = after_parts.join(" ");

    Context { before, after }
}

pub fn split_text_into_chunks(text: &str, max_len: usize) -> Vec<String> {
    static SENTENCE_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"[^.!?]+[.!?]+(?:\s+|$)|[^.!?]+(?:\s+|$)").expect("valid sentence regex")
    });
    let mut sentences: Vec<String> = SENTENCE_RE
        .find_iter(text)
        .map(|m| m.as_str().trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if sentences.is_empty() && !text.trim().is_empty() {
        sentences.push(text.trim().to_string());
    }

    let mut chunks = Vec::new();
    let mut current = String::new();

    for sentence in sentences {
        if current.is_empty() {
            current.push_str(&sentence);
        } else if current.len() + 1 + sentence.len() <= max_len {
            current.push(' ');
            current.push_str(&sentence);
        } else {
            chunks.push(std::mem::take(&mut current));
            current.push_str(&sentence);
        }
    }

    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

pub fn preserve_whitespace(original: &str, translated: &str) -> String {
    let lead: String = original.chars().take_while(|c| c.is_whitespace()).collect();
    let trail: String = original
        .chars()
        .rev()
        .take_while(|c| c.is_whitespace())
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{lead}{}{trail}", translated.trim())
}

pub fn extract_visible_text(html: &str) -> String {
    let doc = ParsedHtmlDocument::parse(html);
    let joined: Vec<&str> = doc
        .spans
        .iter()
        .map(|s| s.text.trim())
        .filter(|s| !s.is_empty())
        .collect();
    joined.join(" ")
}

#[derive(Debug, Clone)]
pub struct Paragraph {
    pub start: usize,
    pub end: usize,
    pub tag: String,
    pub raw_html: String,
    pub inner_html: String,
    pub marked_text: String,
    pub inline_tags: Vec<(String, String)>,
}

pub fn extract_paragraphs(html: &str) -> Vec<Paragraph> {
    static P_REGEX: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?is)<(p|h1|h2|h3|h4|h5|h6|div)\b([^>]*)>(.*?)</(p|h1|h2|h3|h4|h5|h6|div)>")
            .expect("valid paragraph regex")
    });
    static INLINE_TAG_REGEX: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?is)<([a-zA-Z0-9]+)\b([^>]*)>(.*?)</([a-zA-Z0-9]+)>")
            .expect("valid inline tag regex")
    });
    let p_regex = &*P_REGEX;
    let inline_tag_regex = &*INLINE_TAG_REGEX;

    let mut paragraphs = Vec::new();
    for cap in p_regex.captures_iter(html) {
        let open_tag = cap[1].to_lowercase();
        let close_tag = cap[4].to_lowercase();
        if open_tag != close_tag {
            continue;
        }

        let Some(mat) = cap.get(0) else {
            continue;
        };
        let raw = mat.as_str();
        let start = mat.start();
        let end = mat.end();
        let inner = &cap[3];

        let trimmed_inner = inner.trim();
        if trimmed_inner.is_empty() {
            continue;
        }

        let tag = open_tag;
        let mut marked = trimmed_inner.to_string();
        let mut inline_tags = Vec::new();

        let mut inline_idx = 0;
        for icap in inline_tag_regex.captures_iter(trimmed_inner) {
            let i_open = icap[1].to_lowercase();
            let i_close = icap[4].to_lowercase();
            if i_open != i_close {
                continue;
            }

            let full_tag = &icap[0];
            let tag_name = &icap[1];
            let tag_attrs = &icap[2];
            let tag_content = &icap[3];

            let open = format!("<{tag_name}{tag_attrs}>");
            let close = format!("</{tag_name}>");
            inline_tags.push((open, close));

            let placeholder = format!("{{{inline_idx}}}{tag_content}{{/{inline_idx}}}");
            marked = marked.replace(full_tag, &placeholder);
            inline_idx += 1;
        }

        paragraphs.push(Paragraph {
            start,
            end,
            tag,
            raw_html: raw.to_string(),
            inner_html: inner.to_string(),
            marked_text: marked,
            inline_tags,
        });
    }
    paragraphs
}

pub fn restore_inline_tags(translated_marked: &str, inline_tags: &[(String, String)]) -> String {
    let mut restored = translated_marked.to_string();
    for (i, (open, close)) in inline_tags.iter().enumerate() {
        let open_marker = format!("{{{i}}}");
        let close_marker = format!("{{/{i}}}");
        restored = restored.replace(&open_marker, open);
        restored = restored.replace(&close_marker, close);
    }
    restored
}

pub fn rebuild_chapter_html(
    html: &str,
    paragraphs: &[Paragraph],
    translated_inners: &[String],
) -> String {
    let mut out = String::with_capacity(html.len() + 512);
    let mut last_idx = 0;

    for (p, translated) in paragraphs.iter().zip(translated_inners.iter()) {
        if p.start >= last_idx && p.end <= html.len() {
            out.push_str(&html[last_idx..p.start]);
            let open_tag = &p.raw_html[..p.raw_html.find('>').unwrap_or(0) + 1];
            let close_tag = &p.raw_html[p.raw_html.rfind('<').unwrap_or(p.raw_html.len())..];
            out.push_str(open_tag);
            out.push_str(translated);
            out.push_str(close_tag);
            last_idx = p.end;
        }
    }

    if last_idx < html.len() {
        out.push_str(&html[last_idx..]);
    }

    out
}

pub fn build_bilingual_chapter(
    html: &str,
    paragraphs: &[Paragraph],
    translated_inners: &[String],
) -> String {
    let mut out = String::with_capacity(html.len() * 2);
    let mut last_idx = 0;

    for (p, translated) in paragraphs.iter().zip(translated_inners.iter()) {
        if p.start >= last_idx && p.end <= html.len() {
            out.push_str(&html[last_idx..p.start]);

            // Source with class
            let tag_name = &p.tag;
            out.push_str(&format!(
                "<{tag_name} class=\"vellum-bilingual-source\">{}</{tag_name}>\n",
                p.inner_html
            ));
            // Target with class
            out.push_str(&format!(
                "<{tag_name} class=\"vellum-bilingual-target\">{translated}</{tag_name}>"
            ));
            last_idx = p.end;
        }
    }

    if last_idx < html.len() {
        out.push_str(&html[last_idx..]);
    }

    out
}
