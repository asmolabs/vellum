use std::path::Path;

use regex::Regex;
use vellum_plugins::converter::mobi_to_epub::decompress_palmdoc;

use super::html::{
    extract_chapter_title, html_to_plain_text, paginate_html_into_pages, sanitize_reader_html,
    wrap_with_reader_theme,
};
use super::{BookContentProvider, PageInfo, ReaderPage, DEFAULT_TARGET_CHARS_PER_PAGE};

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
                        let text_records =
                            u16::from_be_bytes([bytes[first_offset + 8], bytes[first_offset + 9]])
                                as usize;

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
            static RE_BREAK: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
                Regex::new(r"(?i)<mbp:pagebreak[^>]*>").expect("valid mobi break regex")
            });
            for part in RE_BREAK.split(&raw_text) {
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

        Self {
            pages,
            toc,
            chapters,
        }
    }
}

impl BookContentProvider for MobiContentProvider {
    fn total_pages(&self) -> usize {
        self.pages.len()
    }

    fn get_page_html(
        &self,
        page_index: usize,
        theme: &str,
        font_size: u32,
        is_serif: bool,
    ) -> String {
        if page_index >= self.pages.len() {
            return String::new();
        }
        wrap_with_reader_theme(&self.pages[page_index].html, theme, font_size, is_serif)
    }

    fn get_section_title(&self, page_index: usize) -> Option<String> {
        self.pages.get(page_index).map(|p| {
            if p.total_in_chapter > 1 {
                format!(
                    "{} ({}/{})",
                    p.chapter_title, p.page_in_chapter, p.total_in_chapter
                )
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
