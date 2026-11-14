//! Pure library filtering, sorting, aggregation, and keyboard selection helpers.

use std::collections::HashMap;

use vellum_core::SearchQuery;
use vellum_domain::Book;

use crate::state::{FilterStatus, SortOption};

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
pub fn filter_and_sort_books(
    books: &[Book],
    filter: FilterStatus,
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
}
