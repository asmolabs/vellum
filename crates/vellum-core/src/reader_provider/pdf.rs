use std::path::Path;

use super::html::{escape_html, wrap_with_reader_theme};
use super::{BookContentProvider, PageInfo};

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
                for page_num in pages.keys() {
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

    fn get_page_html(
        &self,
        page_index: usize,
        theme: &str,
        font_size: u32,
        is_serif: bool,
    ) -> String {
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
