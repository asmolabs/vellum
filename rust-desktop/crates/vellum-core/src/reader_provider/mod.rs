//! Fournisseurs de contenu pour le lecteur intégré (EPUB, CBZ, MOBI, TXT, PDF).

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

mod cbz;
mod epub;
mod html;
mod mobi;
mod pdf;
mod txt;

pub use cbz::CbzContentProvider;
pub use epub::EpubContentProvider;
pub use html::{
    html_to_plain_text, paginate_html_into_pages, sanitize_reader_html, split_html_into_blocks,
    wrap_with_reader_theme,
};
pub use mobi::MobiContentProvider;
pub use pdf::PdfContentProvider;
pub use txt::TextContentProvider;

use serde::{Deserialize, Serialize};
use std::path::Path;

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
    fn get_page_html(
        &self,
        page_index: usize,
        theme: &str,
        font_size: u32,
        is_serif: bool,
    ) -> String;
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

// Converts HTML to whitespace-collapsed plain text with basic entity decoding.

pub struct EmptyContentProvider;

impl BookContentProvider for EmptyContentProvider {
    fn total_pages(&self) -> usize {
        0
    }

    fn get_page_html(
        &self,
        _page_index: usize,
        _theme: &str,
        _font_size: u32,
        _is_serif: bool,
    ) -> String {
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
