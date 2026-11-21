use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{info, warn};
use vellum_domain::{Book, BookFormat, Tag, VellumError, VellumResult};

use crate::book_service::BookService;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CalibreImportReport {
    pub books_imported: usize,
    pub formats_imported: usize,
    pub series_count: usize,
    pub tags_count: usize,
    pub skipped_count: usize,
    pub errors: Vec<String>,
    pub cancelled: bool,
}

struct CalibreRawBook {
    id: i64,
    title: String,
    pubdate: Option<String>,
    series_idx: Option<f64>,
    isbn: Option<String>,
    rel_path: Option<String>,
    has_cover: Option<i64>,
}

pub struct CalibreImporter;

impl CalibreImporter {
    /// Detects if the given path contains a valid Calibre library (i.e. metadata.db exists)
    pub fn is_calibre_library(path: &Path) -> bool {
        if path.is_file() {
            path.file_name()
                .map(|n| n.to_string_lossy().eq_ignore_ascii_case("metadata.db"))
                .unwrap_or(false)
        } else {
            path.join("metadata.db").exists()
        }
    }

    /// Resolves the Calibre library root directory and metadata.db path
    pub fn resolve_paths(path: &Path) -> VellumResult<(PathBuf, PathBuf)> {
        if path.is_file() {
            let db_path = path.to_path_buf();
            let root = path
                .parent()
                .ok_or_else(|| VellumError::InvalidArgument("Chemin invalide".into()))?
                .to_path_buf();
            Ok((root, db_path))
        } else {
            let db_path = path.join("metadata.db");
            if !db_path.exists() {
                return Err(VellumError::NotFound(format!(
                    "Aucun fichier 'metadata.db' trouvé dans {:?}",
                    path
                )));
            }
            Ok((path.to_path_buf(), db_path))
        }
    }

    /// Imports a complete Calibre library into Vellum
    pub fn import_calibre_library(
        calibre_path: &Path,
        copy_files: bool,
        book_service: &BookService,
    ) -> VellumResult<CalibreImportReport> {
        Self::import_calibre_library_with_progress(
            calibre_path,
            copy_files,
            book_service,
            None,
            |_, _, _| {},
        )
    }

    /// Imports a complete Calibre library into Vellum with real-time progress and cancellation support
    pub fn import_calibre_library_with_progress<F>(
        calibre_path: &Path,
        copy_files: bool,
        book_service: &BookService,
        cancel_token: Option<&std::sync::atomic::AtomicBool>,
        mut on_progress: F,
    ) -> VellumResult<CalibreImportReport>
    where
        F: FnMut(usize, usize, &str),
    {
        let (root_dir, db_path) = Self::resolve_paths(calibre_path)?;

        let conn = Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| {
                VellumError::Database(format!("Erreur ouverture Calibre metadata.db: {e}"))
            })?;

        let mut report = CalibreImportReport::default();
        let mut series_set = std::collections::HashSet::new();
        let mut tags_set = std::collections::HashSet::new();

        // Check which tables exist in metadata.db
        let existing_tables: std::collections::HashSet<String> = {
            let mut stmt = conn
                .prepare("SELECT lower(name) FROM sqlite_master WHERE type='table'")
                .map_err(|e| VellumError::Database(format!("Erreur lecture tables Calibre: {e}")))?;
            let rows = stmt
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(|e| VellumError::Database(format!("Erreur parcours tables Calibre: {e}")))?;
            rows.filter_map(Result::ok).collect()
        };

        // Check which columns exist in the books table
        let books_columns: std::collections::HashSet<String> = {
            let mut stmt = conn
                .prepare("PRAGMA table_info(books)")
                .map_err(|e| VellumError::Database(format!("Erreur lecture colonnes Calibre: {e}")))?;
            let rows = stmt
                .query_map([], |row| row.get::<_, String>(1))
                .map_err(|e| VellumError::Database(format!("Erreur parcours colonnes Calibre: {e}")))?;
            rows.filter_map(Result::ok)
                .map(|s| s.to_ascii_lowercase())
                .collect()
        };

        let pubdate_col = if books_columns.contains("pubdate") { "pubdate" } else { "NULL" };
        let series_idx_col = if books_columns.contains("series_index") { "series_index" } else { "NULL" };
        let isbn_col = if books_columns.contains("isbn") { "isbn" } else { "NULL" };
        let path_col = if books_columns.contains("path") { "path" } else { "NULL" };
        let has_cover_col = if books_columns.contains("has_cover") { "has_cover" } else { "0" };

        // 1. Query all books in Calibre
        let books_query = format!(
            "SELECT id, title, {pubdate_col}, {series_idx_col}, {isbn_col}, {path_col}, {has_cover_col} \
             FROM books \
             ORDER BY id ASC"
        );

        let mut stmt = conn
            .prepare(&books_query)
            .map_err(|e| VellumError::Database(format!("Erreur requête livres Calibre: {e}")))?;

        let book_rows: Vec<CalibreRawBook> = stmt
            .query_map([], |row| {
                Ok(CalibreRawBook {
                    id: row.get::<_, i64>(0)?,
                    title: row.get::<_, Option<String>>(1)?.unwrap_or_else(|| "Sans titre".to_string()),
                    pubdate: row.get::<_, Option<String>>(2)?,
                    series_idx: row.get::<_, Option<f64>>(3)?,
                    isbn: row.get::<_, Option<String>>(4)?,
                    rel_path: row.get::<_, Option<String>>(5)?,
                    has_cover: row.get::<_, Option<i64>>(6)?,
                })
            })
            .map_err(|e| VellumError::Database(format!("Erreur lecture livres Calibre: {e}")))?
            .filter_map(Result::ok)
            .collect();

        // Prepare child queries once for optimal performance and safety
        let mut ident_stmt = if existing_tables.contains("identifiers") {
            conn.prepare(
                "SELECT val FROM identifiers 
                 WHERE book = ?1 AND lower(type) IN ('isbn', 'isbn-13', 'isbn-10', 'isbn13', 'isbn10') 
                 ORDER BY CASE WHEN lower(type) = 'isbn' THEN 0 ELSE 1 END LIMIT 1",
            ).ok()
        } else {
            None
        };

        let mut authors_stmt = if existing_tables.contains("authors") && existing_tables.contains("books_authors_link") {
            conn.prepare(
                "SELECT a.name 
                 FROM authors a 
                 JOIN books_authors_link bal ON a.id = bal.author 
                 WHERE bal.book = ?1 
                 ORDER BY bal.id ASC",
            ).ok()
        } else {
            None
        };

        let mut series_stmt = if existing_tables.contains("series") && existing_tables.contains("books_series_link") {
            conn.prepare(
                "SELECT s.name 
                 FROM series s 
                 JOIN books_series_link bsl ON s.id = bsl.series 
                 WHERE bsl.book = ?1 
                 LIMIT 1",
            ).ok()
        } else {
            None
        };

        let mut tags_stmt = if existing_tables.contains("tags") && existing_tables.contains("books_tags_link") {
            conn.prepare(
                "SELECT t.name 
                 FROM tags t 
                 JOIN books_tags_link btl ON t.id = btl.tag 
                 WHERE btl.book = ?1",
            ).ok()
        } else {
            None
        };

        let mut pub_stmt = if existing_tables.contains("publishers") && existing_tables.contains("books_publishers_link") {
            conn.prepare(
                "SELECT p.name 
                 FROM publishers p 
                 JOIN books_publishers_link bpl ON p.id = bpl.publisher 
                 WHERE bpl.book = ?1 
                 LIMIT 1",
            ).ok()
        } else {
            None
        };

        let mut comm_stmt = if existing_tables.contains("comments") {
            conn.prepare("SELECT text FROM comments WHERE book = ?1 LIMIT 1").ok()
        } else {
            None
        };

        let mut data_stmt = if existing_tables.contains("data") {
            conn.prepare(
                "SELECT format, uncompressed_size, name 
                 FROM data 
                 WHERE book = ?1",
            ).ok()
        } else {
            None
        };

        let total = book_rows.len();
        let mut batch_books = Vec::with_capacity(250);

        for (idx, raw) in book_rows.into_iter().enumerate() {
            if let Some(token) = cancel_token {
                if token.load(std::sync::atomic::Ordering::Relaxed) {
                    report.cancelled = true;
                    break;
                }
            }
            // Throttled progress: notify every 15 books or on the final book
            // to avoid saturating the async channel and UI thread on large libraries (10k+ books)
            if idx % 15 == 0 || idx + 1 == total {
                on_progress(idx + 1, total, &raw.title);
            }

            let calibre_id = raw.id;
            let title = raw.title;
            let pubdate = raw.pubdate;
            let series_idx = raw.series_idx;
            let rel_path = raw.rel_path;
            let has_cover = raw.has_cover;

            // Fetch ISBN: prefer identifiers table if isbn not on books table, or fallback
            let final_isbn = match raw.isbn {
                Some(ref val) if !val.trim().is_empty() => Some(val.clone()),
                _ => ident_stmt
                    .as_mut()
                    .and_then(|stmt| stmt.query_row([calibre_id], |row| row.get::<_, String>(0)).ok()),
            };

            // 2. Fetch authors for this book
            let authors: Vec<String> = if let Some(ref mut stmt) = authors_stmt {
                stmt.query_map([calibre_id], |row| row.get(0))
                    .map(|rows| rows.filter_map(Result::ok).collect())
                    .unwrap_or_default()
            } else {
                Vec::new()
            };

            let author_str = if authors.is_empty() {
                "Inconnu".to_string()
            } else {
                authors.join(", ")
            };

            // 3. Fetch series for this book
            let series_name: Option<String> = if let Some(ref mut stmt) = series_stmt {
                stmt.query_row([calibre_id], |row| row.get(0)).ok()
            } else {
                None
            };

            if let Some(ref s) = series_name {
                series_set.insert(s.clone());
            }

            // 4. Fetch tags for this book
            let book_tags: Vec<String> = if let Some(ref mut stmt) = tags_stmt {
                stmt.query_map([calibre_id], |row| row.get(0))
                    .map(|rows| rows.filter_map(Result::ok).collect())
                    .unwrap_or_default()
            } else {
                Vec::new()
            };

            for t in &book_tags {
                tags_set.insert(t.clone());
            }

            // 5. Fetch publisher
            let publisher_name: Option<String> = if let Some(ref mut stmt) = pub_stmt {
                stmt.query_row([calibre_id], |row| row.get(0)).ok()
            } else {
                None
            };

            // 6. Fetch description (comments)
            let description: Option<String> = if let Some(ref mut stmt) = comm_stmt {
                stmt.query_row([calibre_id], |row| row.get(0))
                    .ok()
                    .map(|raw: String| strip_html_tags(&raw))
            } else {
                None
            };

            // 7. Fetch data formats
            let formats_info: Vec<(String, u64, String)> = if let Some(ref mut stmt) = data_stmt {
                stmt.query_map([calibre_id], |row| {
                    let fmt: String = row.get(0)?;
                    let size = u64::try_from(row.get::<_, i64>(1).unwrap_or(0)).unwrap_or(0);
                    let name: String = row.get(2)?;
                    Ok((fmt, size, name))
                })
                .map(|rows| rows.filter_map(Result::ok).collect())
                .unwrap_or_default()
            } else {
                Vec::new()
            };

            if formats_info.is_empty() {
                report.skipped_count += 1;
                continue;
            }

            // 8. Prepare target Vellum book directory
            let vellum_library_dir = book_service.library_path();
            let safe_author = sanitize_filename(&author_str);
            let safe_title = sanitize_filename(&title);

            let book_dest_dir = vellum_library_dir.join(&safe_author).join(&safe_title);
            if copy_files {
                let _ = std::fs::create_dir_all(&book_dest_dir);
            }

            // 9. Handle Cover
            let calibre_book_folder = rel_path
                .as_ref()
                .map(|p| root_dir.join(p))
                .unwrap_or_else(|| root_dir.clone());

            let mut cover_relative_path = None;
            if has_cover.unwrap_or(0) > 0 {
                let calibre_cover = calibre_book_folder.join("cover.jpg");
                if calibre_cover.exists() {
                    if copy_files {
                        let dest_cover = book_dest_dir.join("cover.jpg");
                        if std::fs::copy(&calibre_cover, &dest_cover).is_ok() {
                            cover_relative_path = dest_cover
                                .strip_prefix(vellum_library_dir)
                                .ok()
                                .map(|p| p.to_string_lossy().to_string());
                        }
                    } else {
                        cover_relative_path = Some(calibre_cover.to_string_lossy().to_string());
                    }
                }
            }

            // 10. Construct Book
            let mut book = Book::new(title.clone(), author_str.clone());
            book.series = series_name;
            book.series_index = series_idx;
            book.isbn = final_isbn;
            book.publisher = publisher_name;
            book.publish_date = pubdate;
            book.description = description;
            book.cover_path = cover_relative_path;

            for tag_name in book_tags {
                book.add_tag(Tag::new(tag_name));
            }

            // 11. Add format files
            for (fmt_str, size, file_stem) in formats_info {
                let ext = fmt_str.to_lowercase();
                let src_filename = format!("{file_stem}.{ext}");
                let calibre_file_path = calibre_book_folder.join(&src_filename);

                if !calibre_file_path.exists() {
                    continue;
                }

                let (final_filepath, final_filename) = if copy_files {
                    let dest_filename = format!("{safe_title}.{ext}");
                    let dest_path = book_dest_dir.join(&dest_filename);
                    if let Err(e) = std::fs::copy(&calibre_file_path, &dest_path) {
                        warn!(
                            "Impossible de copier le format {:?}: {e}",
                            calibre_file_path
                        );
                        continue;
                    }
                    let rel = dest_path
                        .strip_prefix(vellum_library_dir)
                        .unwrap_or(&dest_path)
                        .to_string_lossy()
                        .to_string();
                    (rel, dest_filename)
                } else {
                    (
                        calibre_file_path.to_string_lossy().to_string(),
                        src_filename,
                    )
                };

                let format_entity = BookFormat::new(ext, final_filename, final_filepath, size);
                book.add_format(format_entity);
                report.formats_imported += 1;
            }

            if book.formats.is_empty() {
                report.skipped_count += 1;
                continue;
            }

            batch_books.push(book);

            if batch_books.len() >= 250 {
                if let Err(e) = book_service.book_repo().save_batch(&batch_books) {
                    report
                        .errors
                        .push(format!("Erreur sauvegarde lot de livres: {e}"));
                } else {
                    report.books_imported += batch_books.len();
                }
                batch_books.clear();
            }
        }

        // Flush remaining books in the batch
        if !batch_books.is_empty() && !report.cancelled {
            if let Err(e) = book_service.book_repo().save_batch(&batch_books) {
                report
                    .errors
                    .push(format!("Erreur sauvegarde lot de livres: {e}"));
            } else {
                report.books_imported += batch_books.len();
            }
        }

        report.series_count = series_set.len();
        report.tags_count = tags_set.len();

        info!(
            "Migration Calibre terminée avec succès : {} livres importés, {} formats, {} séries, {} tags.",
            report.books_imported, report.formats_imported, report.series_count, report.tags_count
        );

        Ok(report)
    }
}

fn sanitize_filename(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect();
    let trimmed = cleaned.trim().trim_matches('.');
    if trimmed.is_empty() {
        "SansNom".to_string()
    } else {
        trimmed.to_string()
    }
}

fn strip_html_tags(input: &str) -> String {
    let mut output = String::new();
    let mut in_tag = false;

    for ch in input.chars() {
        if ch == '<' {
            in_tag = true;
        } else if ch == '>' {
            in_tag = false;
        } else if !in_tag {
            output.push(ch);
        }
    }

    // Clean up excessive whitespace
    output
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
