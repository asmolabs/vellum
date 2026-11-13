use std::fs::File;
use std::io::Read;
use std::path::Path;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use zip::ZipArchive;

use super::html::natural_sort_key;
use super::{BookContentProvider, PageInfo};

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
                img_names.sort_by_key(|a| natural_sort_key(a));

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
