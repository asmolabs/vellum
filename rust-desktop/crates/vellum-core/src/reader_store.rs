use std::path::Path;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::reader_provider::BookContentProvider;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Default)]
pub enum ComicReadingMode {
    #[default]
    SinglePage,
    DoublePage,
    Webtoon,
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Default)]
pub enum ComicFitMode {
    #[default]
    FitHeight,
    FitWidth,
    Original,
}


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReaderPreferences {
    pub theme: String,              // "dark", "sepia", "light", "amoled"
    pub font_family: String,        // "sans", "serif", "literata", "merriweather", "dyslexic"
    pub font_size: u32,             // 12 to 36 px
    pub line_height: f32,           // 1.3 to 2.2
    pub margin_horizontal: u32,     // 16 to 72 px
    pub justify_text: bool,
    pub two_columns: bool,          // Two-page spread on wide viewport
    pub warmth: u8,                 // 0 to 80% amber tint for night reading
    pub brightness: u8,             // 30 to 100%
    pub animation_style: String,    // "slide", "fade", "none"
    pub words_per_minute: usize,    // Reading speed for time estimation (default: 200)

    // Comic & Manga modes (CBZ / CBR)
    #[serde(default)]
    pub comic_reading_mode: ComicReadingMode, // SinglePage, DoublePage, Webtoon
    #[serde(default)]
    pub comic_rtl: bool,                      // Japanese Manga direction: Right-to-Left
    #[serde(default)]
    pub comic_fit_mode: ComicFitMode,         // FitHeight, FitWidth, Original
}

impl Default for ReaderPreferences {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            font_family: "serif".to_string(),
            font_size: 18,
            line_height: 1.65,
            margin_horizontal: 32,
            justify_text: true,
            two_columns: false,
            warmth: 0,
            brightness: 100,
            animation_style: "slide".to_string(),
            words_per_minute: 200,
            comic_reading_mode: ComicReadingMode::SinglePage,
            comic_rtl: false,
            comic_fit_mode: ComicFitMode::FitHeight,
        }
    }
}

impl ReaderPreferences {
    pub fn load_from_dir(dir: &Path) -> Self {
        let path = dir.join("reader_preferences.json");
        if let Ok(bytes) = std::fs::read(&path) {
            if let Ok(prefs) = serde_json::from_slice::<Self>(&bytes) {
                return prefs;
            }
        }
        Self::default()
    }

    pub fn save_to_dir(&self, dir: &Path) -> std::io::Result<()> {
        let _ = std::fs::create_dir_all(dir);
        let path = dir.join("reader_preferences.json");
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)
    }

    pub fn font_css_family(&self) -> &'static str {
        match self.font_family.as_str() {
            "sans" => "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif",
            "literata" => "'Literata', Georgia, serif",
            "merriweather" => "'Merriweather', Cambria, Georgia, serif",
            "dyslexic" => "'OpenDyslexic', 'Comic Sans MS', sans-serif",
            _ => "Georgia, Cambria, 'Times New Roman', serif",
        }
    }

    pub fn theme_colors(&self) -> (&'static str, &'static str, &'static str, &'static str) {
        // (background, text, card_bg, border)
        match self.theme.as_str() {
            "light" => ("#e8edf2", "#1e293b", "#ffffff", "#cbd5e1"),
            "sepia" => ("#e4d9c0", "#3d2b1f", "#f8f0dc", "#d6c5a3"),
            "amoled" => ("#000000", "#e5e7eb", "#050507", "#18181b"),
            _ => ("#0b0d14", "#e2e8f0", "#131622", "#24293d"), // dark
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Bookmark {
    pub id: String,
    pub book_id: String,
    pub chapter_index: usize,
    pub chapter_title: String,
    pub page_in_chapter: usize,
    pub progress_percent: f64,
    pub preview: String,
    pub created_at: String,
}

impl Bookmark {
    pub fn new(
        book_id: &str,
        chapter_index: usize,
        chapter_title: &str,
        page_in_chapter: usize,
        progress_percent: f64,
        preview: &str,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            book_id: book_id.to_string(),
            chapter_index,
            chapter_title: chapter_title.to_string(),
            page_in_chapter,
            progress_percent,
            preview: preview.chars().take(120).collect(),
            created_at: Utc::now().to_rfc3339(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Highlight {
    pub id: String,
    pub book_id: String,
    pub chapter_index: usize,
    pub chapter_title: String,
    pub color: String, // "yellow", "green", "blue", "pink"
    pub selected_text: String,
    pub note: Option<String>,
    pub created_at: String,
}

impl Highlight {
    pub fn new(
        book_id: &str,
        chapter_index: usize,
        chapter_title: &str,
        color: &str,
        selected_text: &str,
        note: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            book_id: book_id.to_string(),
            chapter_index,
            chapter_title: chapter_title.to_string(),
            color: color.to_string(),
            selected_text: selected_text.to_string(),
            note,
            created_at: Utc::now().to_rfc3339(),
        }
    }

    pub fn color_rgba(&self) -> &'static str {
        match self.color.as_str() {
            "green" => "rgba(74, 222, 128, 0.35)",
            "blue" => "rgba(96, 165, 250, 0.35)",
            "pink" => "rgba(244, 114, 182, 0.35)",
            _ => "rgba(250, 204, 21, 0.35)", // yellow
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BookUserData {
    pub bookmarks: Vec<Bookmark>,
    pub highlights: Vec<Highlight>,
    pub total_seconds_read: u64,
    pub pages_turned: usize,
    pub last_read_timestamp: i64,
}

impl BookUserData {
    pub fn load_from_dir(dir: &Path, book_id: &str) -> Self {
        let path = dir.join(format!("book_userdata_{}.json", book_id));
        if let Ok(bytes) = std::fs::read(&path) {
            if let Ok(data) = serde_json::from_slice::<Self>(&bytes) {
                return data;
            }
        }
        Self::default()
    }

    pub fn save_to_dir(&self, dir: &Path, book_id: &str) -> std::io::Result<()> {
        let _ = std::fs::create_dir_all(dir);
        let path = dir.join(format!("book_userdata_{}.json", book_id));
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)
    }

    pub fn add_bookmark(&mut self, bookmark: Bookmark) {
        // Avoid duplicate bookmarks on the exact same chapter + page
        self.bookmarks.retain(|b| {
            !(b.chapter_index == bookmark.chapter_index
                && b.page_in_chapter == bookmark.page_in_chapter)
        });
        self.bookmarks.push(bookmark);
        self.bookmarks
            .sort_by(|a, b| a.chapter_index.cmp(&b.chapter_index).then(a.page_in_chapter.cmp(&b.page_in_chapter)));
    }

    pub fn remove_bookmark(&mut self, id: &str) {
        self.bookmarks.retain(|b| b.id != id);
    }

    pub fn is_bookmarked(&self, chapter_index: usize, page_in_chapter: usize) -> bool {
        self.bookmarks
            .iter()
            .any(|b| b.chapter_index == chapter_index && b.page_in_chapter == page_in_chapter)
    }

    pub fn add_highlight(&mut self, highlight: Highlight) {
        self.highlights.push(highlight);
        self.highlights.sort_by_key(|a| a.chapter_index);
    }

    pub fn update_highlight_note(&mut self, id: &str, note: Option<String>) {
        if let Some(h) = self.highlights.iter_mut().find(|h| h.id == id) {
            h.note = note;
        }
    }

    pub fn remove_highlight(&mut self, id: &str) {
        self.highlights.retain(|h| h.id != id);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub chapter_index: usize,
    pub chapter_title: String,
    pub snippet_before: String,
    pub matched_text: String,
    pub snippet_after: String,
    pub char_offset: usize,
}

pub fn search_in_book(
    provider: &dyn BookContentProvider,
    query: &str,
    max_results: usize,
) -> Vec<SearchResult> {
    let clean_query = query.trim().to_lowercase();
    if clean_query.is_empty() || clean_query.len() < 2 {
        return Vec::new();
    }

    let mut results = Vec::new();
    let total_chapters = provider.chapter_count();

    for ch_idx in 0..total_chapters {
        let text = provider.chapter_text(ch_idx);
        let lower = text.to_lowercase();
        let ch_title = provider.chapter_title(ch_idx);

        let mut start_search = 0;
        while let Some(pos) = lower[start_search..].find(&clean_query) {
            let match_idx = start_search + pos;
            let end_idx = match_idx + clean_query.len();

            let snippet_start = match_idx.saturating_sub(45);
            let snippet_end = (end_idx + 45).min(text.len());

            let before = &text[snippet_start..match_idx];
            let matched = &text[match_idx..end_idx];
            let after = &text[end_idx..snippet_end];

            results.push(SearchResult {
                chapter_index: ch_idx,
                chapter_title: ch_title.clone(),
                snippet_before: format!("{}{}", if snippet_start > 0 { "…" } else { "" }, before),
                matched_text: matched.to_string(),
                snippet_after: format!("{}{}", after, if snippet_end < text.len() { "…" } else { "" }),
                char_offset: match_idx,
            });

            if results.len() >= max_results {
                return results;
            }

            start_search = end_idx;
        }
    }

    results
}

pub fn estimate_reading_time(words: usize, wpm: usize) -> String {
    let speed = if wpm == 0 { 200 } else { wpm };
    let minutes = ((words as f64) / (speed as f64)).ceil() as usize;

    if minutes <= 1 {
        "< 1 min".to_string()
    } else if minutes < 60 {
        format!("~{} min", minutes)
    } else {
        let hours = minutes / 60;
        let rem_min = minutes % 60;
        if rem_min == 0 {
            format!("~{} h", hours)
        } else {
            format!("~{} h {}", hours, rem_min)
        }
    }
}

pub fn export_highlights_markdown(
    book_title: &str,
    author: &str,
    highlights: &[Highlight],
    bookmarks: &[Bookmark],
) -> String {
    let mut out = String::new();
    out.push_str(&format!("# Notes & Citations : {}\n", book_title));
    out.push_str(&format!("**Auteur** : {}\n", author));
    out.push_str(&format!("**Date d'export** : {}\n\n", Utc::now().format("%Y-%m-%d %H:%M")));

    if !highlights.is_empty() {
        out.push_str("## Citations et Surlignages\n\n");
        for h in highlights {
            out.push_str(&format!("### {}\n", h.chapter_title));
            out.push_str(&format!("> {}\n\n", h.selected_text.replace('\n', "\n> ")));
            if let Some(ref note) = h.note {
                if !note.trim().is_empty() {
                    out.push_str(&format!("*Note* : {}\n\n", note.trim()));
                }
            }
            out.push_str("---\n\n");
        }
    }

    if !bookmarks.is_empty() {
        out.push_str("## Signets\n\n");
        for b in bookmarks {
            out.push_str(&format!(
                "- **{}** (page {}) : *« {} »* ({:.1}%)\n",
                b.chapter_title, b.page_in_chapter, b.preview, b.progress_percent
            ));
        }
        out.push('\n');
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_preferences_save_and_load() {
        let temp = TempDir::new().unwrap();
        let mut prefs = ReaderPreferences::default();
        prefs.font_size = 22;
        prefs.theme = "sepia".to_string();
        prefs.two_columns = true;
        prefs.comic_reading_mode = ComicReadingMode::Webtoon;
        prefs.comic_rtl = true;
        prefs.comic_fit_mode = ComicFitMode::FitWidth;

        prefs.save_to_dir(temp.path()).unwrap();
        let loaded = ReaderPreferences::load_from_dir(temp.path());
        assert_eq!(loaded.font_size, 22);
        assert_eq!(loaded.theme, "sepia");
        assert!(loaded.two_columns);
        assert_eq!(loaded.comic_reading_mode, ComicReadingMode::Webtoon);
        assert!(loaded.comic_rtl);
        assert_eq!(loaded.comic_fit_mode, ComicFitMode::FitWidth);
    }

    #[test]
    fn test_book_user_data_bookmarks_and_highlights() {
        let temp = TempDir::new().unwrap();
        let book_id = "test-book-123";
        let mut data = BookUserData::default();

        let b1 = Bookmark::new(book_id, 0, "Chapitre 1", 1, 5.0, "Il était une fois");
        data.add_bookmark(b1.clone());
        assert!(data.is_bookmarked(0, 1));
        assert!(!data.is_bookmarked(0, 2));

        let h1 = Highlight::new(
            book_id,
            0,
            "Chapitre 1",
            "yellow",
            "La vérité est ailleurs.",
            Some("Penser à vérifier la source".to_string()),
        );
        data.add_highlight(h1.clone());
        assert_eq!(data.highlights.len(), 1);

        data.save_to_dir(temp.path(), book_id).unwrap();
        let loaded = BookUserData::load_from_dir(temp.path(), book_id);
        assert_eq!(loaded.bookmarks.len(), 1);
        assert_eq!(loaded.highlights.len(), 1);
        assert_eq!(loaded.highlights[0].note.as_deref(), Some("Penser à vérifier la source"));

        let md = export_highlights_markdown("Mon Livre", "Auteur", &loaded.highlights, &loaded.bookmarks);
        assert!(md.contains("La vérité est ailleurs."));
        assert!(md.contains("Penser à vérifier la source"));
    }

    #[test]
    fn test_estimate_reading_time() {
        assert_eq!(estimate_reading_time(100, 200), "< 1 min");
        assert_eq!(estimate_reading_time(600, 200), "~3 min");
        assert_eq!(estimate_reading_time(12000, 200), "~1 h");
        assert_eq!(estimate_reading_time(13000, 200), "~1 h 5");
    }
}
