use std::path::PathBuf;
use vellum_core::reader_provider::BookContentProvider;
use vellum_domain::Book;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Home,
    Grid,
    Table,
    TableCarousel,
    Kanban,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterStatus {
    All,
    Reading,
    Finished,
    ToRead,
}

impl FilterStatus {
    pub fn i18n_key(&self) -> &'static str {
        match self {
            FilterStatus::All => "nav.all_books",
            FilterStatus::Reading => "nav.reading",
            FilterStatus::Finished => "nav.finished",
            FilterStatus::ToRead => "nav.to_read",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOption {
    TitleAsc,
    TitleDesc,
    AuthorAsc,
    RecentAdded,
    ProgressDesc,
}

use vellum_core::reader_store::{
    BookUserData, Bookmark, ComicReadingMode, Highlight, ReaderPreferences, SearchResult,
};

pub struct ReaderSession {
    pub book: Book,
    pub provider: Box<dyn BookContentProvider>,
    pub current_page: usize,
    pub total_pages: usize,
    pub theme: String,
    pub font_size: u32,
    pub is_serif: bool,
    pub prefs: ReaderPreferences,
    pub user_data: BookUserData,
    pub history: Vec<usize>,
}

impl ReaderSession {
    pub fn new(book: Book, book_path: PathBuf) -> Self {
        let provider = vellum_core::reader_provider::create_content_provider(&book_path);
        let total_pages = provider.total_pages();
        let current_page = (book.current_chapter_index as usize).min(if total_pages > 0 {
            total_pages - 1
        } else {
            0
        });

        let vellum_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".vellum");
        let prefs = ReaderPreferences::load_from_dir(&vellum_dir);
        let book_id_str = book.id.clone();
        let user_data = BookUserData::load_from_dir(&vellum_dir, &book_id_str);

        let theme = prefs.theme.clone();
        let font_size = prefs.font_size;
        let is_serif = prefs.font_family != "sans";

        Self {
            book,
            provider,
            current_page,
            total_pages,
            theme,
            font_size,
            is_serif,
            prefs,
            user_data,
            history: Vec::new(),
        }
    }

    pub fn save_prefs(&self) {
        let vellum_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".vellum");
        let _ = self.prefs.save_to_dir(&vellum_dir);
    }

    pub fn save_user_data(&self) {
        let vellum_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".vellum");
        let _ = self.user_data.save_to_dir(&vellum_dir, &self.book.id);
    }

    pub fn jump_to_page(&mut self, target: usize) {
        if target < self.total_pages && target != self.current_page {
            self.history.push(self.current_page);
            self.current_page = target;
        }
    }

    pub fn history_back(&mut self) -> Option<usize> {
        if let Some(prev) = self.history.pop() {
            if prev < self.total_pages {
                self.current_page = prev;
                return Some(prev);
            }
        }
        None
    }

    pub fn is_comic(&self) -> bool {
        self.provider.is_fixed_layout()
            || self.book.formats.iter().any(|f| {
                f.format.eq_ignore_ascii_case("cbz") || f.format.eq_ignore_ascii_case("cbr")
            })
    }

    pub fn current_html(&self) -> String {
        if self.is_comic() {
            match self.prefs.comic_reading_mode {
                ComicReadingMode::Webtoon => self.provider.get_webtoon_html(),
                ComicReadingMode::DoublePage => self.provider.get_double_page_html(
                    self.current_page,
                    &self.prefs.theme,
                    self.prefs.comic_rtl,
                ),
                ComicReadingMode::SinglePage => self.provider.get_page_html(
                    self.current_page,
                    &self.prefs.theme,
                    self.prefs.font_size,
                    self.prefs.font_family != "sans",
                ),
            }
        } else {
            self.provider.get_page_html(
                self.current_page,
                &self.prefs.theme,
                self.prefs.font_size,
                self.prefs.font_family != "sans",
            )
        }
    }

    pub fn current_title(&self) -> String {
        self.provider
            .get_section_title(self.current_page)
            .unwrap_or_else(|| {
                format!(
                    "Section {} / {}",
                    self.current_page + 1,
                    self.total_pages.max(1)
                )
            })
    }

    pub fn progress_percent(&self) -> f64 {
        if self.total_pages <= 1 {
            100.0
        } else {
            ((self.current_page + 1) as f64 / self.total_pages as f64) * 100.0
        }
    }

    pub fn table_of_contents(&self) -> Vec<(usize, String)> {
        self.provider.get_table_of_contents()
    }

    pub fn page_info(&self) -> Option<vellum_core::reader_provider::PageInfo> {
        self.provider.get_page_info(self.current_page)
    }

    pub fn search(&self, query: &str) -> Vec<SearchResult> {
        vellum_core::search_in_book(&*self.provider, query, 60)
    }

    pub fn is_current_bookmarked(&self) -> bool {
        if let Some(info) = self.page_info() {
            self.user_data
                .is_bookmarked(info.chapter_index, info.page_in_chapter)
        } else {
            self.user_data.is_bookmarked(self.current_page, 0)
        }
    }

    pub fn toggle_current_bookmark(&mut self) {
        let (ch_idx, p_in_ch, ch_title) = if let Some(info) = self.page_info() {
            (info.chapter_index, info.page_in_chapter, info.chapter_title)
        } else {
            (self.current_page, 0, self.current_title())
        };

        if self.user_data.is_bookmarked(ch_idx, p_in_ch) {
            let id = self
                .user_data
                .bookmarks
                .iter()
                .find(|b| b.chapter_index == ch_idx && b.page_in_chapter == p_in_ch)
                .map(|b| b.id.clone());
            if let Some(id) = id {
                self.user_data.remove_bookmark(&id);
                self.save_user_data();
            }
        } else {
            let preview = self.provider.chapter_text(ch_idx);
            let snippet = preview.chars().take(100).collect::<String>();
            let bm = Bookmark::new(
                &self.book.id,
                ch_idx,
                &ch_title,
                p_in_ch,
                self.progress_percent(),
                &snippet,
            );
            self.user_data.add_bookmark(bm);
            self.save_user_data();
        }
    }

    pub fn add_highlight(&mut self, color: &str, text: &str, note: Option<String>) {
        let (ch_idx, ch_title) = if let Some(info) = self.page_info() {
            (info.chapter_index, info.chapter_title)
        } else {
            (self.current_page, self.current_title())
        };
        let h = Highlight::new(&self.book.id, ch_idx, &ch_title, color, text, note);
        self.user_data.add_highlight(h);
        self.save_user_data();
    }

    pub fn remove_highlight(&mut self, id: &str) {
        self.user_data.remove_highlight(id);
        self.save_user_data();
    }

    pub fn remove_bookmark(&mut self, id: &str) {
        self.user_data.remove_bookmark(id);
        self.save_user_data();
    }

    pub fn estimate_remaining_time(&self) -> (String, String) {
        let wpm = self.prefs.words_per_minute;
        let mut chapter_words = 0;
        let mut book_words_remaining = 0;

        let total_chapters = self.provider.chapter_count();
        let cur_ch = if let Some(info) = self.page_info() {
            info.chapter_index
        } else {
            self.current_page
        };

        for ch in cur_ch..total_chapters {
            let words = self.provider.chapter_text(ch).split_whitespace().count();
            if ch == cur_ch {
                if let Some(info) = self.page_info() {
                    if info.total_in_chapter > 0 {
                        let remaining_ratio = 1.0
                            - ((info.page_in_chapter.saturating_sub(1)) as f64
                                / info.total_in_chapter as f64);
                        chapter_words = ((words as f64) * remaining_ratio).round() as usize;
                    } else {
                        chapter_words = words;
                    }
                } else {
                    chapter_words = words;
                }
                book_words_remaining += chapter_words;
            } else {
                book_words_remaining += words;
            }
        }

        (
            vellum_core::estimate_reading_time(chapter_words, wpm),
            vellum_core::estimate_reading_time(book_words_remaining, wpm),
        )
    }

    pub fn export_annotations_markdown(&self) -> String {
        vellum_core::export_highlights_markdown(
            &self.book.title,
            &self.book.author,
            &self.user_data.highlights,
            &self.user_data.bookmarks,
        )
    }
}
