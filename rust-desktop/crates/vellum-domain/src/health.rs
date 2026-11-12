use crate::entity::Book;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DuplicatePair {
    pub primary_book: Book,
    pub duplicate_book: Book,
    pub similarity_score: f64, // 0.0 to 1.0 (ex: 0.95 pour 95%)
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IncompleteBookIssue {
    pub book: Book,
    pub missing_cover: bool,
    pub missing_author: bool,
    pub missing_description: bool,
    pub missing_isbn: bool,
    pub missing_tags: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MissingFileIssue {
    pub book_id: String,
    pub book_title: String,
    pub format: String,
    pub expected_path: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LibraryHealthReport {
    pub overall_score: u32, // 0 to 100%
    pub total_books: usize,
    pub total_formats: usize,
    pub duplicate_pairs: Vec<DuplicatePair>,
    pub incomplete_issues: Vec<IncompleteBookIssue>,
    pub missing_file_issues: Vec<MissingFileIssue>,
    pub clean_books_count: usize,
}

