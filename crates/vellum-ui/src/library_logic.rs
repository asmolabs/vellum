//! Pure library filtering, sorting, aggregation, and keyboard selection helpers.

use std::collections::HashMap;

use vellum_core::SearchQuery;
use vellum_domain::Book;

use crate::state::{FilterStatus, MediaType, SortOption};

/// Computes `(books_count, audiobooks_count, comics_count)` across `books`.
pub fn compute_media_counts(books: &[Book]) -> (usize, usize, usize) {
    let mut books_count = 0;
    let mut audio_count = 0;
    let mut comics_count = 0;
    for b in books {
        if MediaType::Audiobooks.matches_book(b) {
            audio_count += 1;
        }
        if MediaType::Comics.matches_book(b) {
            comics_count += 1;
        }
        if MediaType::Books.matches_book(b) {
            books_count += 1;
        }
    }
    (books_count, audio_count, comics_count)
}

/// Computes metadata completeness `(percentage 0..=100, missing_field_labels)` for a book (BookOrbit MetadataScore).
pub fn compute_metadata_score(book: &Book) -> (u8, Vec<&'static str>) {
    let checks: [(&'static str, bool); 8] = [
        ("Titre", !book.title.trim().is_empty()),
        (
            "Auteur",
            !book.author.trim().is_empty() && book.author.trim() != "Inconnu",
        ),
        (
            "Couverture",
            book.cover_path
                .as_ref()
                .is_some_and(|p| !p.trim().is_empty()),
        ),
        (
            "Description",
            book.description
                .as_ref()
                .is_some_and(|d| !d.trim().is_empty()),
        ),
        (
            "ISBN",
            book.isbn.as_ref().is_some_and(|i| !i.trim().is_empty()),
        ),
        (
            "Éditeur",
            book.publisher
                .as_ref()
                .is_some_and(|p| !p.trim().is_empty()),
        ),
        (
            "Série",
            book.series.as_ref().is_some_and(|s| !s.trim().is_empty()),
        ),
        ("Tags", !book.tags.is_empty()),
    ];
    let mut present = 0usize;
    let mut missing = Vec::new();
    for (label, ok) in checks {
        if ok {
            present += 1;
        } else {
            missing.push(label);
        }
    }
    let pct = ((present * 100) / checks.len()) as u8;
    (pct, missing)
}

/// Collapses books belonging to the same series into a single representative entry `(book, Some((series_name, count)))`.
pub fn collapse_books_by_series(books: &[Book]) -> Vec<(Book, Option<(String, usize)>)> {
    let mut series_counts: HashMap<String, usize> = HashMap::new();
    for b in books {
        if let Some(ref s) = b.series {
            let trimmed = s.trim();
            if !trimmed.is_empty() {
                *series_counts.entry(trimmed.to_string()).or_insert(0) += 1;
            }
        }
    }
    let mut seen_series = std::collections::HashSet::new();
    let mut out = Vec::with_capacity(books.len());
    for b in books {
        if let Some(ref s) = b.series {
            let trimmed = s.trim();
            if !trimmed.is_empty() {
                let total = series_counts.get(trimmed).copied().unwrap_or(1);
                if total > 1 {
                    if seen_series.insert(trimmed.to_string()) {
                        out.push((b.clone(), Some((trimmed.to_string(), total))));
                    }
                    continue;
                }
            }
        }
        out.push((b.clone(), None));
    }
    out
}

/// Aggregates tag counts across `books`, sorted by descending count then alphabetically.
pub fn compute_tags_counts(books: &[Book]) -> Vec<(String, usize)> {
    let mut map = HashMap::new();
    for b in books {
        for t in &b.tags {
            *map.entry(t.name.clone()).or_insert(0) += 1;
        }
    }
    let mut list: Vec<_> = map.into_iter().collect();
    list.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    list
}

/// Aggregates non-empty series counts across `books`, sorted by descending count then name.
pub fn compute_series_counts(books: &[Book]) -> Vec<(String, usize)> {
    let mut map = HashMap::new();
    for b in books {
        if let Some(ref s) = b.series {
            let trimmed = s.trim();
            if !trimmed.is_empty() {
                *map.entry(trimmed.to_string()).or_insert(0) += 1;
            }
        }
    }
    let mut list: Vec<_> = map.into_iter().collect();
    list.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    list
}

/// Filters `books` according to status, tag, series, search query and optional virtual library query,
/// then sorts them according to `sort`.
#[allow(dead_code)]
pub fn filter_and_sort_books(
    books: &[Book],
    filter: FilterStatus,
    sort: SortOption,
    tag_filter: Option<&str>,
    series_filter: Option<&str>,
    raw_query: &str,
    active_vl_query: Option<&str>,
) -> Vec<Book> {
    filter_and_sort_books_by_media(
        books,
        filter,
        MediaType::All,
        sort,
        tag_filter,
        series_filter,
        raw_query,
        active_vl_query,
    )
}

/// Filters `books` according to status, media type, tag, series, search query and optional virtual library query,
/// then sorts them according to `sort`.
#[allow(clippy::too_many_arguments)]
pub fn filter_and_sort_books_by_media(
    books: &[Book],
    filter: FilterStatus,
    media_type: MediaType,
    sort: SortOption,
    tag_filter: Option<&str>,
    series_filter: Option<&str>,
    raw_query: &str,
    active_vl_query: Option<&str>,
) -> Vec<Book> {
    let parsed_query = SearchQuery::parse(raw_query);
    let parsed_vl_query = active_vl_query.map(SearchQuery::parse);

    let mut list: Vec<Book> = books
        .iter()
        .filter(|b| {
            if !media_type.matches_book(b) {
                return false;
            }
            if let Some(ref vl_q) = parsed_vl_query {
                if !vl_q.is_empty() && !vl_q.matches(b) {
                    return false;
                }
            }

            let matches_status = match filter {
                FilterStatus::All => true,
                FilterStatus::Reading => b.progress_percent > 0.0 && !b.is_finished,
                FilterStatus::Finished => b.is_finished,
                FilterStatus::ToRead => b.progress_percent == 0.0 && !b.is_finished,
            };
            if !matches_status {
                return false;
            }

            if let Some(t) = tag_filter {
                if !b.tags.iter().any(|bt| bt.name == t) {
                    return false;
                }
            }

            if let Some(s) = series_filter {
                if b.series.as_deref().map(str::trim) != Some(s) {
                    return false;
                }
            }

            if !parsed_query.is_empty() && !parsed_query.matches(b) {
                return false;
            }

            true
        })
        .cloned()
        .collect();

    match sort {
        SortOption::TitleAsc => list.sort_by_key(|a| a.title.to_lowercase()),
        SortOption::TitleDesc => list.sort_by_key(|a| std::cmp::Reverse(a.title.to_lowercase())),
        SortOption::AuthorAsc => list.sort_by(|a, b| {
            a.author
                .to_lowercase()
                .cmp(&b.author.to_lowercase())
                .then_with(|| a.title.cmp(&b.title))
        }),
        SortOption::RecentAdded => list.sort_by(|a, b| b.added_at.cmp(&a.added_at)),
        SortOption::ProgressDesc => list.sort_by(|a, b| {
            b.progress_percent
                .partial_cmp(&a.progress_percent)
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
    }

    list
}

/// Resolves the active index in `books` matching `focused_id`, defaulting to `0` if non-empty.
pub fn resolve_active_index(books: &[Book], focused_id: Option<&str>) -> Option<usize> {
    if books.is_empty() {
        return None;
    }
    let idx = focused_id
        .and_then(|id| books.iter().position(|b| b.id == id))
        .unwrap_or(0);
    Some(idx)
}

/// Steps `current_idx` by `delta`, clamping within `0..len`.
pub fn step_selection_index(current_idx: usize, delta: isize, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    let next = current_idx as isize + delta;
    next.clamp(0, (len - 1) as isize) as usize
}

/// Returns the next focused book ID after applying `delta` relative to `current_focused`.
pub fn next_focused_book_id(
    books: &[Book],
    current_focused: Option<&str>,
    delta: isize,
) -> Option<String> {
    let cur_idx = resolve_active_index(books, current_focused)?;
    let next_idx = step_selection_index(cur_idx, delta, books.len());
    books.get(next_idx).map(|b| b.id.clone())
}

/// Fenêtre de virtualisation calculée pour la grille de couvertures (`.books-grid`).
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridWindow {
    pub start_idx: usize,
    pub end_idx: usize,
    pub top_spacer_px: f64,
    pub bottom_spacer_px: f64,
}

/// Calcule la plage visible `[start_idx..end_idx]` et les espaces haut/bas pour la grille virtualisée.
#[allow(dead_code)]
pub fn compute_grid_window(
    total_items: usize,
    container_width_px: f64,
    scroll_top_px: f64,
    viewport_height_px: f64,
) -> GridWindow {
    const CARD_MIN_W: f64 = 136.0;
    const GRID_GAP: f64 = 14.0;
    const CARD_ASPECT: f64 = 1.45;
    const OVERSCAN_ROWS: usize = 2;

    if total_items == 0 {
        return GridWindow {
            start_idx: 0,
            end_idx: 0,
            top_spacer_px: 0.0,
            bottom_spacer_px: 0.0,
        };
    }

    let width = container_width_px.max(CARD_MIN_W);
    let view_h = viewport_height_px.max(200.0);
    let cols = ((width + GRID_GAP) / (CARD_MIN_W + GRID_GAP))
        .floor()
        .max(1.0) as usize;
    let card_w =
        ((width - (cols.saturating_sub(1)) as f64 * GRID_GAP) / cols as f64).max(CARD_MIN_W);
    let row_stride = card_w * CARD_ASPECT + GRID_GAP;
    let total_rows = total_items.div_ceil(cols);

    let first_visible_row =
        ((scroll_top_px.max(0.0) / row_stride).floor() as usize).min(total_rows.saturating_sub(1));
    let start_row = first_visible_row.saturating_sub(OVERSCAN_ROWS);
    let visible_rows = (view_h / row_stride).ceil().max(1.0) as usize;
    let end_row = (first_visible_row + visible_rows + OVERSCAN_ROWS + 1).min(total_rows);

    let start_idx = (start_row * cols).min(total_items);
    let end_idx = (end_row * cols).min(total_items);
    let top_spacer_px = start_row as f64 * row_stride;
    let bottom_spacer_px = total_rows.saturating_sub(end_row) as f64 * row_stride;

    GridWindow {
        start_idx,
        end_idx,
        top_spacer_px,
        bottom_spacer_px,
    }
}

/// Fenêtre de virtualisation calculée pour le tableau des livres (`.books-table`).
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TableWindow {
    pub start_idx: usize,
    pub end_idx: usize,
    pub top_spacer_px: f64,
    pub bottom_spacer_px: f64,
}

/// Hauteur nominale (px) d'une ligne de tableau dans `.books-table`.
#[allow(dead_code)]
pub const TABLE_ROW_HEIGHT_PX: f64 = 57.0;

/// Calcule la plage visible `[start_idx..end_idx]` et les espaces haut/bas pour le tableau virtualisé.
#[allow(dead_code)]
pub fn compute_table_window(
    total_rows: usize,
    scroll_top_px: f64,
    viewport_height_px: f64,
    row_height_px: f64,
) -> TableWindow {
    const OVERSCAN_ROWS: usize = 8;

    if total_rows == 0 {
        return TableWindow {
            start_idx: 0,
            end_idx: 0,
            top_spacer_px: 0.0,
            bottom_spacer_px: 0.0,
        };
    }

    let row_h = row_height_px.max(24.0);
    let view_h = viewport_height_px.max(row_h);
    let first_visible =
        ((scroll_top_px.max(0.0) / row_h).floor() as usize).min(total_rows.saturating_sub(1));
    let start_idx = first_visible.saturating_sub(OVERSCAN_ROWS);
    let visible_rows = (view_h / row_h).ceil().max(1.0) as usize + 1;
    let end_idx = (first_visible + visible_rows + OVERSCAN_ROWS).min(total_rows);

    TableWindow {
        start_idx,
        end_idx,
        top_spacer_px: start_idx as f64 * row_h,
        bottom_spacer_px: total_rows.saturating_sub(end_idx) as f64 * row_h,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::bulk_edit_modal::BulkEditRequest;
    use crate::components::detail_drawer::SaveMetadataRequest;
    use chrono::TimeZone;
    use vellum_domain::Tag;

    fn sample_book(id: &str, title: &str, author: &str, added_day: u32) -> Book {
        let mut b = Book::new(title, author);
        b.id = id.to_string();
        b.added_at = chrono::Utc
            .with_ymd_and_hms(2026, 3, added_day, 12, 0, 0)
            .unwrap()
            .to_rfc3339();
        b
    }

    #[test]
    fn test_compute_tags_counts_sorts_by_count_desc_then_name() {
        let mut b1 = sample_book("1", "Dune", "Frank Herbert", 1);
        b1.tags = vec![Tag::new("Sci-Fi"), Tag::new("Classic")];
        let mut b2 = sample_book("2", "Foundation", "Isaac Asimov", 2);
        b2.tags = vec![Tag::new("Sci-Fi"), Tag::new("Space")];

        let counts = compute_tags_counts(&[b1, b2]);
        assert_eq!(
            counts,
            vec![
                ("Sci-Fi".to_string(), 2),
                ("Classic".to_string(), 1),
                ("Space".to_string(), 1),
            ]
        );
    }

    #[test]
    fn test_compute_series_counts_ignores_blank_and_trims() {
        let mut b1 = sample_book("1", "Dune", "Frank Herbert", 1);
        b1.series = Some(" Dune Chronicles ".to_string());
        let mut b2 = sample_book("2", "Dune Messiah", "Frank Herbert", 2);
        b2.series = Some("Dune Chronicles".to_string());
        let mut b3 = sample_book("3", "Standalone", "Author", 3);
        b3.series = Some("   ".to_string());

        let counts = compute_series_counts(&[b1, b2, b3]);
        assert_eq!(counts, vec![("Dune Chronicles".to_string(), 2)]);
    }

    #[test]
    fn test_filter_by_status_tag_series_and_virtual_library() {
        let mut b1 = sample_book("1", "Book A", "Alice", 1);
        b1.progress_percent = 40.0;
        b1.tags = vec![Tag::new("Fantasy")];
        b1.series = Some("Realm".to_string());

        let mut b2 = sample_book("2", "Book B", "Bob", 2);
        b2.is_finished = true;
        b2.progress_percent = 100.0;
        b2.tags = vec![Tag::new("Fantasy")];
        b2.series = Some("Realm".to_string());

        let b3 = sample_book("3", "Book C", "Charlie", 3);
        let books = [b1, b2, b3];

        let reading = filter_and_sort_books(
            &books,
            FilterStatus::Reading,
            SortOption::TitleAsc,
            Some("Fantasy"),
            Some("Realm"),
            "",
            None,
        );
        assert_eq!(reading.len(), 1);
        assert_eq!(reading[0].id, "1");

        let vl = filter_and_sort_books(
            &books,
            FilterStatus::All,
            SortOption::RecentAdded,
            None,
            None,
            "",
            Some("author:Bob"),
        );
        assert_eq!(vl.len(), 1);
        assert_eq!(vl[0].id, "2");
    }

    #[test]
    fn test_filter_and_sort_options() {
        let mut b1 = sample_book("1", "Zulu", "B Author", 1);
        b1.progress_percent = 10.0;
        let mut b2 = sample_book("2", "Alpha", "A Author", 5);
        b2.progress_percent = 90.0;
        let books = [b1, b2];

        let title_asc = filter_and_sort_books(
            &books,
            FilterStatus::All,
            SortOption::TitleAsc,
            None,
            None,
            "",
            None,
        );
        assert_eq!(title_asc[0].title, "Alpha");

        let title_desc = filter_and_sort_books(
            &books,
            FilterStatus::All,
            SortOption::TitleDesc,
            None,
            None,
            "",
            None,
        );
        assert_eq!(title_desc[0].title, "Zulu");

        let author_asc = filter_and_sort_books(
            &books,
            FilterStatus::All,
            SortOption::AuthorAsc,
            None,
            None,
            "",
            None,
        );
        assert_eq!(author_asc[0].author, "A Author");

        let progress_desc = filter_and_sort_books(
            &books,
            FilterStatus::All,
            SortOption::ProgressDesc,
            None,
            None,
            "",
            None,
        );
        assert_eq!(progress_desc[0].id, "2");
    }

    #[test]
    fn test_selection_stepping_and_next_focused_book_id() {
        let books = vec![
            sample_book("a", "A", "Author", 1),
            sample_book("b", "B", "Author", 2),
            sample_book("c", "C", "Author", 3),
        ];

        assert_eq!(resolve_active_index(&books, Some("b")), Some(1));
        assert_eq!(resolve_active_index(&books, Some("missing")), Some(0));
        assert_eq!(resolve_active_index(&[], None), None);

        assert_eq!(step_selection_index(0, -1, 3), 0);
        assert_eq!(step_selection_index(1, 1, 3), 2);
        assert_eq!(step_selection_index(2, 5, 3), 2);

        assert_eq!(
            next_focused_book_id(&books, Some("a"), 1),
            Some("b".to_string())
        );
        assert_eq!(
            next_focused_book_id(&books, Some("c"), -1),
            Some("b".to_string())
        );
        assert_eq!(next_focused_book_id(&[], None, 1), None);
    }

    #[test]
    fn test_typed_ui_request_structs() {
        let save_req = SaveMetadataRequest {
            book_id: "id-1".to_string(),
            title: "Titre".to_string(),
            author: "Auteur".to_string(),
            publisher: Some("Editeur".to_string()),
            description: Some("Desc".to_string()),
            isbn: Some("9780000000002".to_string()),
            publish_date: Some("2026-01-01".to_string()),
            series: Some("Serie".to_string()),
            series_index: Some(2.0),
            tags: vec!["tag1".to_string(), "tag2".to_string()],
        };
        assert_eq!(save_req.series_index, Some(2.0));
        assert_eq!(save_req.tags, vec!["tag1", "tag2"]);

        let bulk_req = BulkEditRequest {
            author: Some("Nouvel Auteur".to_string()),
            series: Some("Cycle".to_string()),
            auto_index: true,
            add_tags: vec!["SF".to_string()],
            remove_tags: vec!["Old".to_string()],
        };
        assert!(bulk_req.auto_index);
        assert_eq!(bulk_req.add_tags, vec!["SF"]);
        assert_eq!(bulk_req.remove_tags, vec!["Old"]);
    }

    #[test]
    fn test_compute_grid_window_and_spacers() {
        let empty = compute_grid_window(0, 900.0, 0.0, 700.0);
        assert_eq!(empty.start_idx, 0);
        assert_eq!(empty.end_idx, 0);

        // 1000 items across ~6 columns (900px width -> 6 cols)
        let top_win = compute_grid_window(1000, 900.0, 0.0, 700.0);
        assert_eq!(top_win.start_idx, 0);
        assert_eq!(top_win.top_spacer_px, 0.0);
        assert!(top_win.end_idx > 0 && top_win.end_idx < 100);
        assert!(top_win.bottom_spacer_px > 0.0);

        let mid_win = compute_grid_window(1000, 900.0, 2000.0, 700.0);
        assert!(mid_win.start_idx > 0);
        assert!(mid_win.top_spacer_px > 0.0);
        assert!(mid_win.end_idx < 1000);
        assert!(mid_win.bottom_spacer_px > 0.0);
    }

    #[test]
    fn test_compute_table_window_bounds() {
        let empty = compute_table_window(0, 0.0, 500.0, TABLE_ROW_HEIGHT_PX);
        assert_eq!(empty.start_idx, 0);
        assert_eq!(empty.end_idx, 0);

        let top = compute_table_window(2000, 0.0, 570.0, TABLE_ROW_HEIGHT_PX);
        assert_eq!(top.start_idx, 0);
        assert!(top.end_idx <= 25);
        assert_eq!(top.top_spacer_px, 0.0);
        assert!(top.bottom_spacer_px > 0.0);

        let scrolled = compute_table_window(2000, 5700.0, 570.0, TABLE_ROW_HEIGHT_PX);
        assert_eq!(scrolled.start_idx, 92);
        assert!(scrolled.end_idx > 100 && scrolled.end_idx < 130);
        assert!(scrolled.top_spacer_px > 0.0);
        assert!(scrolled.bottom_spacer_px > 0.0);
    }
}
