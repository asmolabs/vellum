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
        let (root_dir, db_path) = Self::resolve_paths(calibre_path)?;

        let conn = Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| {
                VellumError::Database(format!("Erreur ouverture Calibre metadata.db: {e}"))
            })?;

        let mut report = CalibreImportReport::default();
        let mut series_set = std::collections::HashSet::new();
        let mut tags_set = std::collections::HashSet::new();

        // 1. Query all books in Calibre
        let mut stmt = conn
            .prepare(
                "SELECT id, title, pubdate, series_index, isbn, path, has_cover 
                 FROM books 
                 ORDER BY id ASC",
            )
            .map_err(|e| VellumError::Database(format!("Erreur requête livres Calibre: {e}")))?;

        let book_rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<f64>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<i64>>(6)?,
                ))
            })
            .map_err(|e| VellumError::Database(format!("Erreur lecture livres Calibre: {e}")))?;

        for row_res in book_rows {
            let (calibre_id, title, pubdate, series_idx, isbn, rel_path, has_cover) = match row_res
            {
                Ok(r) => r,
                Err(e) => {
                    report
                        .errors
                        .push(format!("Erreur lecture ligne livre: {e}"));
                    continue;
                }
            };

            // 2. Fetch authors for this book
            let mut authors_stmt = conn
                .prepare(
                    "SELECT a.name 
                     FROM authors a 
                     JOIN books_authors_link bal ON a.id = bal.author 
                     WHERE bal.book = ?1 
                     ORDER BY bal.id ASC",
                )
                .map_err(|e| VellumError::Database(e.to_string()))?;

            let authors: Vec<String> = authors_stmt
                .query_map([calibre_id], |row| row.get(0))
                .map_err(|e| VellumError::Database(e.to_string()))?
                .filter_map(Result::ok)
                .collect();

            let author_str = if authors.is_empty() {
                "Inconnu".to_string()
            } else {
                authors.join(", ")
            };

            // 3. Fetch series for this book
            let mut series_stmt = conn
                .prepare(
                    "SELECT s.name 
                     FROM series s 
                     JOIN books_series_link bsl ON s.id = bsl.series 
                     WHERE bsl.book = ?1 
                     LIMIT 1",
                )
                .map_err(|e| VellumError::Database(e.to_string()))?;

            let series_name: Option<String> =
                series_stmt.query_row([calibre_id], |row| row.get(0)).ok();

            if let Some(ref s) = series_name {
                series_set.insert(s.clone());
            }

            // 4. Fetch tags for this book
            let mut tags_stmt = conn
                .prepare(
                    "SELECT t.name 
                     FROM tags t 
                     JOIN books_tags_link btl ON t.id = btl.tag 
                     WHERE btl.book = ?1",
                )
                .map_err(|e| VellumError::Database(e.to_string()))?;

            let book_tags: Vec<String> = tags_stmt
                .query_map([calibre_id], |row| row.get(0))
                .map_err(|e| VellumError::Database(e.to_string()))?
                .filter_map(Result::ok)
                .collect();

            for t in &book_tags {
                tags_set.insert(t.clone());
            }

            // 5. Fetch publisher
            let mut pub_stmt = conn
                .prepare(
                    "SELECT p.name 
                     FROM publishers p 
                     JOIN books_publishers_link bpl ON p.id = bpl.publisher 
                     WHERE bpl.book = ?1 
                     LIMIT 1",
                )
                .map_err(|e| VellumError::Database(e.to_string()))?;

            let publisher_name: Option<String> =
                pub_stmt.query_row([calibre_id], |row| row.get(0)).ok();

            // 6. Fetch description (comments)
            let mut comm_stmt = conn
                .prepare("SELECT text FROM comments WHERE book = ?1 LIMIT 1")
                .map_err(|e| VellumError::Database(e.to_string()))?;

            let description: Option<String> = comm_stmt
                .query_row([calibre_id], |row| row.get(0))
                .ok()
                .map(|raw: String| strip_html_tags(&raw));

            // 7. Fetch data formats
            let mut data_stmt = conn
                .prepare(
                    "SELECT format, uncompressed_size, name 
                     FROM data 
                     WHERE book = ?1",
                )
                .map_err(|e| VellumError::Database(e.to_string()))?;

            let formats_info: Vec<(String, u64, String)> = data_stmt
                .query_map([calibre_id], |row| {
                    let fmt: String = row.get(0)?;
                    let size: u64 = row.get(1).unwrap_or(0);
                    let name: String = row.get(2)?;
                    Ok((fmt, size, name))
                })
                .map_err(|e| VellumError::Database(e.to_string()))?
                .filter_map(Result::ok)
                .collect();

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
            book.isbn = isbn;
            book.publisher = publisher_name;
            book.publish_date = pubdate;
            book.description = description;
            book.cover_path = cover_relative_path;

            for tag_name in book_tags {
                let tag = Tag::new(tag_name);
                let _ = book_service.tag_repo().save(&tag);
                book.add_tag(tag);
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

            if let Err(e) = book_service.book_repo().save(&book) {
                report
                    .errors
                    .push(format!("Erreur sauvegarde livre '{}': {e}", book.title));
            } else {
                report.books_imported += 1;
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
