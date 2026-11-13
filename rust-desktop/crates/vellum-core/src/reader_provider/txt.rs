use std::path::Path;

use regex::Regex;

use super::html::{
    escape_html, html_to_plain_text, paginate_html_into_pages, wrap_with_reader_theme,
};
use super::{BookContentProvider, PageInfo, ReaderPage, DEFAULT_TARGET_CHARS_PER_PAGE};

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
