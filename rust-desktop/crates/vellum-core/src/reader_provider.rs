use std::fs::File;
use std::io::Read;
use std::path::Path;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use regex::Regex;
use serde::{Deserialize, Serialize};
use vellum_plugins::converter::mobi_to_epub::decompress_palmdoc;
use zip::ZipArchive;

use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReaderSection {
    pub index: usize,
    pub title: Option<String>,
    pub html_content: String,
    pub is_image: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageInfo {
    pub chapter_index: usize,
    pub chapter_title: String,
    pub page_in_chapter: usize,
    pub total_in_chapter: usize,
    pub global_page: usize,
    pub total_pages: usize,
}

pub const DEFAULT_TARGET_CHARS_PER_PAGE: usize = 950;

#[derive(Debug, Clone)]
pub struct ReaderPage {
    pub html: String,
    pub chapter_index: usize,
    pub chapter_title: String,
    pub page_in_chapter: usize,
    pub total_in_chapter: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReaderContent {
    pub format: String,
    pub total_sections: usize,
    pub sections: Vec<ReaderSection>,
}

pub trait BookContentProvider: Send + Sync {
    fn total_pages(&self) -> usize;
    fn get_page_html(&self, page_index: usize, theme: &str, font_size: u32, is_serif: bool) -> String;
    fn get_section_title(&self, _page_index: usize) -> Option<String> {
        None
    }
    fn get_page_info(&self, _page_index: usize) -> Option<PageInfo> {
        None
    }
    fn get_table_of_contents(&self) -> Vec<(usize, String)> {
        (0..self.total_pages())
            .map(|idx| {
                let title = self
                    .get_section_title(idx)
                    .unwrap_or_else(|| format!("Section {}", idx + 1));
                (idx, title)
            })
            .collect()
    }

    // ---- Chapter-level API (used by the layout-based reader) ----

    /// Number of reflowable units (chapters). Fixed-layout formats return one unit per page.
    fn chapter_count(&self) -> usize {
        self.total_pages()
    }

    /// Full HTML of a chapter, sanitized and wrapped, ready to be paginated by the UI.
    fn chapter_html(&self, idx: usize) -> String {
        self.get_page_html(idx, "dark", 18, false)
    }

    fn chapter_title(&self, idx: usize) -> String {
        self.get_section_title(idx)
            .unwrap_or_else(|| format!("Section {}", idx + 1))
    }

    /// Plain text of a chapter (search, progress estimation, reading time).
    fn chapter_text(&self, idx: usize) -> String {
        html_to_plain_text(&self.chapter_html(idx))
    }

    /// True for image-based formats (CBZ) where text features make no sense.
    fn is_fixed_layout(&self) -> bool {
        false
    }

    /// Return double-page HTML for comics: shows page `page_index` and `page_index + 1`.
    /// `rtl`: if true (manga), right page is `page_index`, left page is `page_index + 1`.
    /// If false (western), left page is `page_index`, right page is `page_index + 1`.
    fn get_double_page_html(&self, page_index: usize, _theme: &str, _rtl: bool) -> String {
        self.get_page_html(page_index, _theme, 18, false)
    }

    /// Return continuous Webtoon HTML containing all comic images as a vertical strip.
    fn get_webtoon_html(&self) -> String {
        String::new()
    }
}

/// Converts HTML to whitespace-collapsed plain text with basic entity decoding.
pub fn html_to_plain_text(html: &str) -> String {
    let re_img = Regex::new(r"(?is)<img[^>]*>").unwrap();
    let re_tags = Regex::new(r"(?s)<[^>]+>").unwrap();
    let re_num = Regex::new(r"&#(x?[0-9a-fA-F]+);").unwrap();
    let re_ws = Regex::new(r"\s+").unwrap();

    let no_img = re_img.replace_all(html, " ");
    let no_tags = re_tags.replace_all(&no_img, " ");
    let decoded_num = re_num.replace_all(&no_tags, |c: &regex::Captures| {
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
    re_ws.replace_all(&decoded, " ").trim().to_string()
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
    let re_script = Regex::new(r"(?is)<script[^>]*>.*?</script>").unwrap();
    let re_style = Regex::new(r"(?is)<style[^>]*>.*?</style>").unwrap();
    let no_script = re_script.replace_all(html, "");
    let no_style = re_style.replace_all(&no_script, "");

    let re_body = Regex::new(r"(?is)<body[^>]*>(.*?)</body>").unwrap();
    let body_only = if let Some(caps) = re_body.captures(&no_style) {
        caps[1].to_string()
    } else {
        no_style.to_string()
    };

    let re_head = Regex::new(r"(?is)<head[^>]*>.*?</head>").unwrap();
    let no_head = re_head.replace_all(&body_only, "");

    no_head.trim().to_string()
}

pub fn split_html_into_blocks(html: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    // Match block-level elements: <p>, <h1>..<h6>, <blockquote>, <li>, <table>, <figure>, <img>, <hr>
    // Note: <div> is intentionally NOT matched as a single atomic block, because in ePubs <div> often
    // wraps the entire chapter or multiple paragraphs, which would cause the entire chapter to be treated as 1 page!
    let re_block = Regex::new(
        r#"(?is)(<p[^>]*>.*?</p>|<h[1-6][^>]*>.*?</h[1-6]>|<blockquote[^>]*>.*?</blockquote>|<li[^>]*>.*?</li>|<table[^>]*>.*?</table>|<figure[^>]*>.*?</figure>|<img[^>]*>|<hr[^>]*>)"#
    ).unwrap();

    let re_strip = Regex::new(r"<[^>]+>").unwrap();
    let mut last_end = 0;
    for mat in re_block.find_iter(html) {
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
    let re_strip = Regex::new(r"<[^>]+>").unwrap();

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

        if heading_break || size_break {
            if !current_page.is_empty() {
                pages.push(current_page.join("\n"));
                current_page.clear();
                current_chars = 0;
            }
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

fn normalize_zip_path(base_dir: &str, target_path: &str) -> String {
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

fn replace_image_sources(
    html: &str,
    chapter_path: &str,
    image_map: &HashMap<String, String>,
) -> String {
    let ch_dir = Path::new(chapter_path)
        .parent()
        .and_then(|p| p.to_str())
        .unwrap_or("");

    let re_attr = Regex::new(r#"(?i)(src|xlink:href|href)\s*=\s*(?:"([^"]*)"|'([^']*)')"#).unwrap();

    re_attr
        .replace_all(html, |caps: &regex::Captures| {
            let attr = &caps[1];
            let (quote, val) = if let Some(v) = caps.get(2) {
                ("\"", v.as_str())
            } else if let Some(v) = caps.get(3) {
                ("'", v.as_str())
            } else {
                ("\"", "")
            };

            if val.starts_with("data:")
                || val.starts_with("http://")
                || val.starts_with("https://")
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

fn natural_sort_key(s: &str) -> Vec<(bool, String, u64)> {
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

fn extract_chapter_title(html: &str, fallback_index: usize) -> String {
    let re_h1 = Regex::new(r#"(?is)<h1[^>]*>(.*?)</h1>"#).unwrap();
    let re_h2 = Regex::new(r#"(?is)<h2[^>]*>(.*?)</h2>"#).unwrap();
    let re_title = Regex::new(r#"(?is)<title[^>]*>(.*?)</title>"#).unwrap();
    let re_tags = Regex::new(r#"<[^>]+>"#).unwrap();

    let raw = if let Some(c) = re_h1.captures(html) {
        Some(c[1].to_string())
    } else if let Some(c) = re_h2.captures(html) {
        Some(c[1].to_string())
    } else if let Some(c) = re_title.captures(html) {
        Some(c[1].to_string())
    } else {
        None
    };

    if let Some(text) = raw {
        let clean = re_tags.replace_all(&text, " ").trim().to_string();
        if !clean.is_empty() && clean.len() < 80 {
            return clean;
        }
    }

    format!("Chapitre {}", fallback_index + 1)
}

// 1. EPUB Provider
pub struct EpubContentProvider {
    pages: Vec<ReaderPage>,
    toc: Vec<(usize, String)>,
    chapters: Vec<(String, String)>,
}

impl EpubContentProvider {
    pub fn new(path: &Path) -> Self {
        let mut raw_chapters: Vec<(String, String)> = Vec::new();

        if let Ok(file) = File::open(path) {
            if let Ok(mut archive) = ZipArchive::new(file) {
                // 1. Extract and index all images in the EPUB as Base64 data URIs
                let mut image_map: HashMap<String, String> = HashMap::new();
                for i in 0..archive.len() {
                    if let Ok(mut f) = archive.by_index(i) {
                        let name = f.name().map(|n| n.to_string()).unwrap_or_default();
                        let lower = name.to_lowercase();
                        let mime = if lower.ends_with(".png") {
                            Some("image/png")
                        } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
                            Some("image/jpeg")
                        } else if lower.ends_with(".webp") {
                            Some("image/webp")
                        } else if lower.ends_with(".gif") {
                            Some("image/gif")
                        } else if lower.ends_with(".svg") {
                            Some("image/svg+xml")
                        } else {
                            None
                        };

                        if let Some(mime_type) = mime {
                            let mut bytes = Vec::new();
                            if f.read_to_end(&mut bytes).is_ok() && !bytes.is_empty() {
                                let data_uri = format!("data:{};base64,{}", mime_type, BASE64.encode(&bytes));
                                image_map.insert(name.clone(), data_uri.clone());
                                image_map.insert(lower.clone(), data_uri.clone());

                                if let Some(file_name) = Path::new(&name).file_name().and_then(|fn_| fn_.to_str()) {
                                    image_map.insert(file_name.to_string(), data_uri.clone());
                                    image_map.insert(file_name.to_lowercase(), data_uri.clone());
                                }

                                let segments: Vec<&str> = name.split('/').collect();
                                for idx in 1..segments.len() {
                                    let subpath = segments[idx..].join("/");
                                    image_map.insert(subpath.clone(), data_uri.clone());
                                    image_map.insert(subpath.to_lowercase(), data_uri.clone());
                                }
                            }
                        }
                    }
                }

                // 2. Identify OPF package file
                let opf_path = {
                    let mut p = None;
                    if let Ok(mut container) = archive.by_name("META-INF/container.xml") {
                        let mut content = String::new();
                        if container.read_to_string(&mut content).is_ok() {
                            let re_root = Regex::new(r#"<rootfile[^>]+full-path=["']([^"']+)["']"#).unwrap();
                            if let Some(caps) = re_root.captures(&content) {
                                p = Some(caps[1].to_string());
                            }
                        }
                    }
                    if p.is_none() {
                        for i in 0..archive.len() {
                            if let Ok(f) = archive.by_index(i) {
                                let name = f.name().map(|n| n.to_string()).unwrap_or_default();
                                if name.to_lowercase().ends_with(".opf") {
                                    p = Some(name);
                                    break;
                                }
                            }
                        }
                    }
                    p
                };

                // 3. Resolve chapter ordering from OPF spine
                let mut ordered_chapter_names: Vec<String> = Vec::new();
                if let Some(opf_file_name) = &opf_path {
                    if let Ok(mut opf_file) = archive.by_name(opf_file_name) {
                        let mut opf_xml = String::new();
                        if opf_file.read_to_string(&mut opf_xml).is_ok() {
                            let opf_dir = Path::new(opf_file_name)
                                .parent()
                                .and_then(|p| p.to_str())
                                .unwrap_or("");

                            let mut manifest: HashMap<String, String> = HashMap::new();
                            let re_item = Regex::new(r#"<item\s+[^>]*>"#).unwrap();
                            let re_id = Regex::new(r#"id=["']([^"']+)["']"#).unwrap();
                            let re_href = Regex::new(r#"href=["']([^"']+)["']"#).unwrap();

                            for mat in re_item.find_iter(&opf_xml) {
                                let item_tag = mat.as_str();
                                if let (Some(id_caps), Some(href_caps)) = (re_id.captures(item_tag), re_href.captures(item_tag)) {
                                    let id = id_caps[1].to_string();
                                    let href = href_caps[1].to_string();
                                    let full_path = normalize_zip_path(opf_dir, &href);
                                    manifest.insert(id, full_path);
                                }
                            }

                            let re_spine = Regex::new(r#"(?s)<spine[^>]*>(.*?)</spine>"#).unwrap();
                            if let Some(spine_caps) = re_spine.captures(&opf_xml) {
                                let spine_content = &spine_caps[1];
                                let re_itemref = Regex::new(r#"<itemref\s+[^>]*idref=["']([^"']+)["']"#).unwrap();
                                for caps in re_itemref.captures_iter(spine_content) {
                                    let idref = &caps[1];
                                    if let Some(target_file) = manifest.get(idref) {
                                        let lower = target_file.to_lowercase();
                                        if lower.ends_with(".xhtml") || lower.ends_with(".html") || lower.ends_with(".htm") {
                                            ordered_chapter_names.push(target_file.clone());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Fallback to natural alphabetical sorting if no spine was retrieved
                if ordered_chapter_names.is_empty() {
                    let mut names: Vec<String> = Vec::new();
                    for i in 0..archive.len() {
                        if let Ok(file) = archive.by_index(i) {
                            let name = file.name().map(|n| n.to_string()).unwrap_or_default();
                            let lower = name.to_lowercase();
                            if (lower.ends_with(".xhtml") || lower.ends_with(".html") || lower.ends_with(".htm"))
                                && !lower.contains("toc")
                                && !lower.contains("cover")
                            {
                                names.push(name);
                            }
                        }
                    }
                    names.sort_by(|a, b| natural_sort_key(a).cmp(&natural_sort_key(b)));
                    ordered_chapter_names = names;
                }

                // 4. Load raw chapters
                for name in ordered_chapter_names {
                    if let Ok(mut ch_file) = archive.by_name(&name) {
                        let mut raw_content = String::new();
                        if ch_file.read_to_string(&mut raw_content).is_ok() && !raw_content.trim().is_empty() {
                            let processed_html = replace_image_sources(&raw_content, &name, &image_map);
                            let title = extract_chapter_title(&processed_html, raw_chapters.len());
                            raw_chapters.push((title, processed_html));
                        }
                    }
                }
            }
        }

        // 5. Paginate chapters into discrete eReader pages (~1300 chars target)
        let mut pages = Vec::new();
        let mut toc = Vec::new();
        let mut chapters = Vec::new();

        for (title, ch_html) in raw_chapters {
            let ch_pages = paginate_html_into_pages(&ch_html, DEFAULT_TARGET_CHARS_PER_PAGE);
            if ch_pages.is_empty() {
                continue;
            }
            chapters.push((title.clone(), sanitize_reader_html(&ch_html)));
            let ch_idx = chapters.len() - 1;
            let start_page_index = pages.len();
            toc.push((start_page_index, title.clone()));

            let total_in_ch = ch_pages.len();
            for (p_idx, page_html) in ch_pages.into_iter().enumerate() {
                pages.push(ReaderPage {
                    html: page_html,
                    chapter_index: ch_idx,
                    chapter_title: title.clone(),
                    page_in_chapter: p_idx + 1,
                    total_in_chapter: total_in_ch,
                });
            }
        }

        Self { pages, toc, chapters }
    }
}

/// Implements the chapter-level API for providers holding `chapters: Vec<(title, html)>`.
macro_rules! impl_chapter_api {
    () => {
        fn chapter_count(&self) -> usize {
            self.chapters.len()
        }
        fn chapter_html(&self, idx: usize) -> String {
            self.chapters
                .get(idx)
                .map(|(_, html)| wrap_with_reader_theme(html, "", 0, false))
                .unwrap_or_default()
        }
        fn chapter_title(&self, idx: usize) -> String {
            self.chapters
                .get(idx)
                .map(|(t, _)| t.clone())
                .unwrap_or_else(|| format!("Section {}", idx + 1))
        }
        fn chapter_text(&self, idx: usize) -> String {
            self.chapters
                .get(idx)
                .map(|(_, html)| html_to_plain_text(html))
                .unwrap_or_default()
        }
    };
}

impl BookContentProvider for EpubContentProvider {
    fn total_pages(&self) -> usize {
        self.pages.len()
    }

    fn get_page_html(&self, page_index: usize, theme: &str, font_size: u32, is_serif: bool) -> String {
        if page_index >= self.pages.len() {
            return String::new();
        }
        wrap_with_reader_theme(&self.pages[page_index].html, theme, font_size, is_serif)
    }

    fn get_section_title(&self, page_index: usize) -> Option<String> {
        self.pages.get(page_index).map(|p| {
            if p.total_in_chapter > 1 {
                format!("{} ({}/{})", p.chapter_title, p.page_in_chapter, p.total_in_chapter)
            } else {
                p.chapter_title.clone()
            }
        })
    }

    fn get_page_info(&self, page_index: usize) -> Option<PageInfo> {
        self.pages.get(page_index).map(|p| PageInfo {
            chapter_index: p.chapter_index,
            chapter_title: p.chapter_title.clone(),
            page_in_chapter: p.page_in_chapter,
            total_in_chapter: p.total_in_chapter,
            global_page: page_index + 1,
            total_pages: self.pages.len(),
        })
    }

    fn get_table_of_contents(&self) -> Vec<(usize, String)> {
        self.toc.clone()
    }

    impl_chapter_api!();
}
// 2. CBZ / CBR Provider
pub struct CbzContentProvider {
    images_b64: Vec<(String, String)>, // (mime, b64)
}

impl CbzContentProvider {
    pub fn new(path: &Path) -> Self {
        let mut images = Vec::new();
        if let Ok(file) = File::open(path) {
            if let Ok(mut archive) = ZipArchive::new(file) {
                let mut img_names: Vec<String> = Vec::new();
                for i in 0..archive.len() {
                    if let Ok(f) = archive.by_index(i) {
                        let name = f.name().map(|n| n.to_string()).unwrap_or_default();
                        let lower = name.to_lowercase();
                        if lower.ends_with(".jpg")
                            || lower.ends_with(".jpeg")
                            || lower.ends_with(".png")
                            || lower.ends_with(".webp")
                        {
                            img_names.push(name);
                        }
                    }
                }
                img_names.sort_by(|a, b| natural_sort_key(a).cmp(&natural_sort_key(b)));

                for name in img_names {
                    if let Ok(mut img_file) = archive.by_name(&name) {
                        let mut bytes = Vec::new();
                        if img_file.read_to_end(&mut bytes).is_ok() && !bytes.is_empty() {
                            let mime = if name.to_lowercase().ends_with(".png") {
                                "image/png"
                            } else if name.to_lowercase().ends_with(".webp") {
                                "image/webp"
                            } else {
                                "image/jpeg"
                            };
                            let b64 = BASE64.encode(&bytes);
                            images.push((mime.to_string(), b64));
                        }
                    }
                }
            }
        }
        Self { images_b64: images }
    }
}

impl BookContentProvider for CbzContentProvider {
    fn total_pages(&self) -> usize {
        self.images_b64.len()
    }

    fn get_page_html(&self, page_index: usize, _theme: &str, _font_size: u32, _is_serif: bool) -> String {
        if page_index >= self.images_b64.len() {
            return String::new();
        }
        let (mime, b64) = &self.images_b64[page_index];
        format!(
            r#"<div class="cbz-container"><img src="data:{mime};base64,{b64}" alt="Page {}"/></div>"#,
            page_index + 1
        )
    }

    fn get_section_title(&self, page_index: usize) -> Option<String> {
        Some(format!("Page {}", page_index + 1))
    }

    fn get_page_info(&self, page_index: usize) -> Option<PageInfo> {
        Some(PageInfo {
            chapter_index: page_index,
            chapter_title: format!("Page {}", page_index + 1),
            page_in_chapter: page_index + 1,
            total_in_chapter: self.images_b64.len(),
            global_page: page_index + 1,
            total_pages: self.images_b64.len(),
        })
    }

    fn is_fixed_layout(&self) -> bool {
        true
    }

    fn get_double_page_html(&self, page_index: usize, _theme: &str, rtl: bool) -> String {
        if page_index >= self.images_b64.len() {
            return String::new();
        }
        let p1 = &self.images_b64[page_index];
        let p2_opt = self.images_b64.get(page_index + 1);

        match (p2_opt, rtl) {
            (Some(p2), true) => {
                // Manga RTL: first page (p1) is on the right, next page (p2) is on the left
                format!(
                    r#"<div class="cbz-double-page rtl"><div class="cbz-page left"><img src="data:{};base64,{}" alt="Page {}"/></div><div class="cbz-page right"><img src="data:{};base64,{}" alt="Page {}"/></div></div>"#,
                    p2.0, p2.1, page_index + 2,
                    p1.0, p1.1, page_index + 1
                )
            }
            (Some(p2), false) => {
                // Western LTR: first page (p1) is on the left, next page (p2) is on the right
                format!(
                    r#"<div class="cbz-double-page ltr"><div class="cbz-page left"><img src="data:{};base64,{}" alt="Page {}"/></div><div class="cbz-page right"><img src="data:{};base64,{}" alt="Page {}"/></div></div>"#,
                    p1.0, p1.1, page_index + 1,
                    p2.0, p2.1, page_index + 2
                )
            }
            (None, _) => {
                format!(
                    r#"<div class="cbz-double-page single"><div class="cbz-page center"><img src="data:{};base64,{}" alt="Page {}"/></div></div>"#,
                    p1.0, p1.1, page_index + 1
                )
            }
        }
    }

    fn get_webtoon_html(&self) -> String {
        let mut html = String::from(r#"<div class="webtoon-strip-container">"#);
        for (i, (mime, b64)) in self.images_b64.iter().enumerate() {
            html.push_str(&format!(
                r#"<div class="webtoon-page-item" id="webtoon-page-{}"><img src="data:{};base64,{}" alt="Page {}" loading="lazy"/><div class="webtoon-page-badge">Page {}</div></div>"#,
                i + 1, mime, b64, i + 1, i + 1
            ));
        }
        html.push_str("</div>");
        html
    }
}

// 3. MOBI / AZW Provider
pub struct MobiContentProvider {
    pages: Vec<ReaderPage>,
    toc: Vec<(usize, String)>,
    chapters: Vec<(String, String)>,
}

impl MobiContentProvider {
    pub fn new(path: &Path) -> Self {
        let mut raw_text = String::new();
        if let Ok(bytes) = std::fs::read(path) {
            if bytes.len() > 78 {
                let num_records = u16::from_be_bytes([bytes[76], bytes[77]]) as usize;
                let mut record_offsets = Vec::with_capacity(num_records);
                let mut cursor = 78;
                for _ in 0..num_records {
                    if cursor + 4 <= bytes.len() {
                        let offset = u32::from_be_bytes([
                            bytes[cursor],
                            bytes[cursor + 1],
                            bytes[cursor + 2],
                            bytes[cursor + 3],
                        ]) as usize;
                        record_offsets.push(offset);
                    }
                    cursor += 8;
                }

                if !record_offsets.is_empty() {
                    let first_offset = record_offsets[0];
                    if first_offset + 10 <= bytes.len() {
                        let compression =
                            u16::from_be_bytes([bytes[first_offset], bytes[first_offset + 1]]);
                        let text_records = u16::from_be_bytes([
                            bytes[first_offset + 8],
                            bytes[first_offset + 9],
                        ]) as usize;

                        let mut text_bytes = Vec::new();
                        let limit = (text_records + 1).min(num_records);
                        for i in 1..limit {
                            let start = record_offsets[i];
                            let end = if i + 1 < num_records {
                                record_offsets[i + 1]
                            } else {
                                bytes.len()
                            };
                            if start < end && end <= bytes.len() {
                                let raw = &bytes[start..end];
                                if compression == 2 {
                                    text_bytes.extend_from_slice(&decompress_palmdoc(raw));
                                } else if compression == 1 {
                                    text_bytes.extend_from_slice(raw);
                                }
                            }
                        }

                        let text = String::from_utf8_lossy(&text_bytes);
                        if !text.trim().is_empty() {
                            raw_text = text.to_string();
                        }
                    }
                }
            }
        }

        let mut pages = Vec::new();
        let mut toc = Vec::new();
        let mut chapters: Vec<(String, String)> = Vec::new();

        if !raw_text.trim().is_empty() {
            // Kindle books mark chapter boundaries with <mbp:pagebreak/>
            let re_break = Regex::new(r"(?i)<mbp:pagebreak[^>]*>").unwrap();
            for part in re_break.split(&raw_text) {
                let clean = sanitize_reader_html(part);
                if html_to_plain_text(&clean).is_empty() && !clean.contains("<img") {
                    continue;
                }
                let title = extract_chapter_title(&clean, chapters.len());
                chapters.push((title, clean));
            }

            let p_list = paginate_html_into_pages(&raw_text, DEFAULT_TARGET_CHARS_PER_PAGE);
            let total = p_list.len();
            toc.push((0, "Début du livre".to_string()));
            for (idx, p_html) in p_list.into_iter().enumerate() {
                pages.push(ReaderPage {
                    html: p_html,
                    chapter_index: 0,
                    chapter_title: "Livre MOBI".to_string(),
                    page_in_chapter: idx + 1,
                    total_in_chapter: total,
                });
            }
        }

        if pages.is_empty() {
            pages.push(ReaderPage {
                html: "<p>Fichier MOBI vide ou non lisible.</p>".to_string(),
                chapter_index: 0,
                chapter_title: "Début".to_string(),
                page_in_chapter: 1,
                total_in_chapter: 1,
            });
            toc.push((0, "Début".to_string()));
        }
        if chapters.is_empty() {
            chapters.push(("Début".to_string(), pages[0].html.clone()));
        }

        Self { pages, toc, chapters }
    }
}

impl BookContentProvider for MobiContentProvider {
    fn total_pages(&self) -> usize {
        self.pages.len()
    }

    fn get_page_html(&self, page_index: usize, theme: &str, font_size: u32, is_serif: bool) -> String {
        if page_index >= self.pages.len() {
            return String::new();
        }
        wrap_with_reader_theme(&self.pages[page_index].html, theme, font_size, is_serif)
    }

    fn get_section_title(&self, page_index: usize) -> Option<String> {
        self.pages.get(page_index).map(|p| {
            if p.total_in_chapter > 1 {
                format!("{} ({}/{})", p.chapter_title, p.page_in_chapter, p.total_in_chapter)
            } else {
                p.chapter_title.clone()
            }
        })
    }

    fn get_page_info(&self, page_index: usize) -> Option<PageInfo> {
        self.pages.get(page_index).map(|p| PageInfo {
            chapter_index: p.chapter_index,
            chapter_title: p.chapter_title.clone(),
            page_in_chapter: p.page_in_chapter,
            total_in_chapter: p.total_in_chapter,
            global_page: page_index + 1,
            total_pages: self.pages.len(),
        })
    }

    fn get_table_of_contents(&self) -> Vec<(usize, String)> {
        self.toc.clone()
    }

    impl_chapter_api!();
}

// 4. Text / Markdown Provider
pub struct TextContentProvider {
    pages: Vec<ReaderPage>,
    toc: Vec<(usize, String)>,
    chapters: Vec<(String, String)>,
}

impl TextContentProvider {
    pub fn new(path: &Path) -> Self {
        let mut pages = Vec::new();
        let mut toc = Vec::new();
        let mut chapters: Vec<(String, String)> = Vec::new();

        if let Ok(content) = std::fs::read_to_string(path) {
            let re = Regex::new(r"(?m)(?:^#{1,3}\s+.*$|^(?:Chapitre|Chapter)\s+\d+.*$|\f|\n{3,})").unwrap();
            let mut split_points: Vec<usize> = re.find_iter(&content).map(|m| m.start()).collect();
            if split_points.is_empty() || split_points[0] != 0 {
                split_points.insert(0, 0);
            }
            split_points.push(content.len());

            let mut sections: Vec<(String, String)> = Vec::new();
            for (sec_idx, window) in split_points.windows(2).enumerate() {
                let start = window[0];
                let end = window[1];
                let section_text = content[start..end].trim();
                if !section_text.is_empty() {
                    let first_line = section_text.lines().next().unwrap_or("").trim();
                    let title = if first_line.starts_with('#') || first_line.to_lowercase().starts_with("chap") {
                        first_line.trim_start_matches('#').trim().to_string()
                    } else {
                        format!("Section {}", sec_idx + 1)
                    };

                    let formatted = section_text
                        .split("\n\n")
                        .map(|p| format!("<p>{}</p>", escape_html(p.trim()).replace('\n', "<br/>")))
                        .collect::<Vec<_>>()
                        .join("");

                    sections.push((title, formatted));
                }
            }

            for (title, sec_html) in sections {
                let p_list = paginate_html_into_pages(&sec_html, DEFAULT_TARGET_CHARS_PER_PAGE);
                if p_list.is_empty() {
                    continue;
                }
                chapters.push((title.clone(), sec_html.clone()));
                let ch_idx = chapters.len() - 1;
                let start_idx = pages.len();
                toc.push((start_idx, title.clone()));
                let total_in_sec = p_list.len();
                for (idx, p_html) in p_list.into_iter().enumerate() {
                    pages.push(ReaderPage {
                        html: p_html,
                        chapter_index: ch_idx,
                        chapter_title: title.clone(),
                        page_in_chapter: idx + 1,
                        total_in_chapter: total_in_sec,
                    });
                }
            }
        }

        if pages.is_empty() {
            pages.push(ReaderPage {
                html: "<p>Fichier texte vide.</p>".to_string(),
                chapter_index: 0,
                chapter_title: "Début".to_string(),
                page_in_chapter: 1,
                total_in_chapter: 1,
            });
            toc.push((0, "Début".to_string()));
        }

        if chapters.is_empty() {
            chapters.push(("Début".to_string(), pages[0].html.clone()));
        }

        Self { pages, toc, chapters }
    }
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

impl BookContentProvider for TextContentProvider {
    fn total_pages(&self) -> usize {
        self.pages.len()
    }

    fn get_page_html(&self, page_index: usize, theme: &str, font_size: u32, is_serif: bool) -> String {
        if page_index >= self.pages.len() {
            return String::new();
        }
        wrap_with_reader_theme(&self.pages[page_index].html, theme, font_size, is_serif)
    }

    fn get_section_title(&self, page_index: usize) -> Option<String> {
        self.pages.get(page_index).map(|p| {
            if p.total_in_chapter > 1 {
                format!("{} ({}/{})", p.chapter_title, p.page_in_chapter, p.total_in_chapter)
            } else {
                p.chapter_title.clone()
            }
        })
    }

    fn get_page_info(&self, page_index: usize) -> Option<PageInfo> {
        self.pages.get(page_index).map(|p| PageInfo {
            chapter_index: p.chapter_index,
            chapter_title: p.chapter_title.clone(),
            page_in_chapter: p.page_in_chapter,
            total_in_chapter: p.total_in_chapter,
            global_page: page_index + 1,
            total_pages: self.pages.len(),
        })
    }

    fn get_table_of_contents(&self) -> Vec<(usize, String)> {
        self.toc.clone()
    }

    impl_chapter_api!();
}

// 5. PDF Provider using lopdf
pub struct PdfContentProvider {
    total_pages: usize,
    pages_text: Vec<String>,
}

impl PdfContentProvider {
    pub fn new(path: &Path) -> Self {
        let mut pages_text = Vec::new();
        let total = match lopdf::Document::load(path) {
            Ok(doc) => {
                let pages = doc.get_pages();
                for (page_num, _) in &pages {
                    let text = doc.extract_text(&[*page_num]).unwrap_or_default();
                    let formatted = text
                        .lines()
                        .filter(|l| !l.trim().is_empty())
                        .map(|l| format!("<p>{}</p>", escape_html(l)))
                        .collect::<Vec<_>>()
                        .join("");
                    pages_text.push(if formatted.is_empty() {
                        format!("<p style='opacity:0.6; text-align:center;'>[Page {page_num} - Contenu vectoriel/image]</p>")
                    } else {
                        formatted
                    });
                }
                pages.len()
            }
            Err(_) => 0,
        };
        Self {
            total_pages: total,
            pages_text,
        }
    }
}

impl BookContentProvider for PdfContentProvider {
    fn total_pages(&self) -> usize {
        self.total_pages
    }

    fn get_page_html(&self, page_index: usize, theme: &str, font_size: u32, is_serif: bool) -> String {
        if page_index >= self.pages_text.len() {
            return String::new();
        }
        wrap_with_reader_theme(&self.pages_text[page_index], theme, font_size, is_serif)
    }

    fn get_section_title(&self, page_index: usize) -> Option<String> {
        Some(format!("Page {}", page_index + 1))
    }

    fn get_page_info(&self, page_index: usize) -> Option<PageInfo> {
        Some(PageInfo {
            chapter_index: page_index,
            chapter_title: format!("Page {}", page_index + 1),
            page_in_chapter: page_index + 1,
            total_in_chapter: self.total_pages,
            global_page: page_index + 1,
            total_pages: self.total_pages,
        })
    }

    fn is_fixed_layout(&self) -> bool {
        true
    }
}

// 6. Empty Fallback
pub struct EmptyContentProvider;

impl BookContentProvider for EmptyContentProvider {
    fn total_pages(&self) -> usize {
        0
    }

    fn get_page_html(&self, _page_index: usize, _theme: &str, _font_size: u32, _is_serif: bool) -> String {
        "<html><body><p>Aucun contenu disponible pour ce livre.</p></body></html>".to_string()
    }
}

pub fn create_content_provider(path: &Path) -> Box<dyn BookContentProvider> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "epub" => Box::new(EpubContentProvider::new(path)),
        "cbz" | "cbr" => Box::new(CbzContentProvider::new(path)),
        "mobi" | "azw" | "azw3" => Box::new(MobiContentProvider::new(path)),
        "txt" | "md" => Box::new(TextContentProvider::new(path)),
        "pdf" => Box::new(PdfContentProvider::new(path)),
        _ => Box::new(EmptyContentProvider),
    }
}
