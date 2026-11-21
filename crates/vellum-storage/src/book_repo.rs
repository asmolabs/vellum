use crate::db::Database;
use rusqlite::{params, Connection, OptionalExtension};
use vellum_domain::{Book, BookFormat, ContentSearchHit, Tag, VellumError, VellumResult};

fn to_fts_match_expr(query: &str) -> Option<String> {
    let fts_terms: Vec<String> = query
        .split_whitespace()
        .filter_map(|raw| {
            let clean: String = raw
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
                .collect();
            if clean.chars().any(|c| c.is_alphanumeric()) {
                Some(format!("\"{clean}\"*"))
            } else {
                None
            }
        })
        .collect();

    if fts_terms.is_empty() {
        None
    } else {
        Some(fts_terms.join(" "))
    }
}

#[derive(Clone)]
pub struct BookRepository {
    db: Database,
}

impl BookRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub fn save(&self, book: &Book) -> VellumResult<Book> {
        let mut conn = self.db.lock()?;
        let tx = conn
            .transaction()
            .map_err(|e| VellumError::Database(e.to_string()))?;

        tx.execute(
            "INSERT INTO books (
                id, title, author, publisher, publish_date, isbn, series, series_index, description, cover_path,
                added_at, current_chapter_index, total_chapters, progress_percent,
                last_read_at, is_finished, rating
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
            ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                author = excluded.author,
                publisher = excluded.publisher,
                publish_date = excluded.publish_date,
                isbn = excluded.isbn,
                series = excluded.series,
                series_index = excluded.series_index,
                description = excluded.description,
                cover_path = excluded.cover_path,
                added_at = excluded.added_at,
                current_chapter_index = excluded.current_chapter_index,
                total_chapters = excluded.total_chapters,
                progress_percent = excluded.progress_percent,
                last_read_at = excluded.last_read_at,
                is_finished = excluded.is_finished,
                rating = excluded.rating",
            params![
                book.id,
                book.title,
                book.author,
                book.publisher,
                book.publish_date,
                book.isbn,
                book.series,
                book.series_index,
                book.description,
                book.cover_path,
                book.added_at,
                book.current_chapter_index,
                book.total_chapters,
                book.progress_percent,
                book.last_read_at,
                if book.is_finished { 1 } else { 0 },
                book.rating,
            ],
        )
        .map_err(|e| VellumError::Database(e.to_string()))?;

        // Replace formats
        tx.execute(
            "DELETE FROM book_formats WHERE book_id = ?1",
            params![book.id],
        )
        .map_err(|e| VellumError::Database(e.to_string()))?;
        for fmt in &book.formats {
            tx.execute(
                "INSERT INTO book_formats (id, book_id, format, filename, filepath, filesize)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    fmt.id,
                    book.id,
                    fmt.format,
                    fmt.filename,
                    fmt.filepath,
                    fmt.filesize as i64
                ],
            )
            .map_err(|e| VellumError::Database(e.to_string()))?;
        }

        // Replace tags
        tx.execute("DELETE FROM book_tags WHERE book_id = ?1", params![book.id])
            .map_err(|e| VellumError::Database(e.to_string()))?;
        for tag in &book.tags {
            tx.execute(
                "INSERT OR IGNORE INTO tags (name) VALUES (?1)",
                params![tag.name],
            )
            .map_err(|e| VellumError::Database(e.to_string()))?;
            tx.execute(
                "INSERT OR IGNORE INTO book_tags (book_id, tag_name) VALUES (?1, ?2)",
                params![book.id, tag.name],
            )
            .map_err(|e| VellumError::Database(e.to_string()))?;
        }

        tx.commit()
            .map_err(|e| VellumError::Database(e.to_string()))?;

        Ok(book.clone())
    }

    /// Saves a batch of books in a single SQLite transaction with cached prepared statements.
    pub fn save_batch(&self, books: &[Book]) -> VellumResult<()> {
        if books.is_empty() {
            return Ok(());
        }
        let mut conn = self.db.lock()?;
        let tx = conn
            .transaction()
            .map_err(|e| VellumError::Database(e.to_string()))?;
        {
            let mut book_stmt = tx.prepare_cached(
                "INSERT INTO books (
                    id, title, author, publisher, publish_date, isbn, series, series_index, description, cover_path,
                    added_at, current_chapter_index, total_chapters, progress_percent,
                    last_read_at, is_finished, rating
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
                ON CONFLICT(id) DO UPDATE SET
                    title = excluded.title,
                    author = excluded.author,
                    publisher = excluded.publisher,
                    publish_date = excluded.publish_date,
                    isbn = excluded.isbn,
                    series = excluded.series,
                    series_index = excluded.series_index,
                    description = excluded.description,
                    cover_path = excluded.cover_path,
                    added_at = excluded.added_at,
                    current_chapter_index = excluded.current_chapter_index,
                    total_chapters = excluded.total_chapters,
                    progress_percent = excluded.progress_percent,
                    last_read_at = excluded.last_read_at,
                    is_finished = excluded.is_finished,
                    rating = excluded.rating",
            ).map_err(|e| VellumError::Database(e.to_string()))?;

            let mut del_fmt = tx.prepare_cached("DELETE FROM book_formats WHERE book_id = ?1")
                .map_err(|e| VellumError::Database(e.to_string()))?;
            let mut ins_fmt = tx.prepare_cached(
                "INSERT INTO book_formats (id, book_id, format, filename, filepath, filesize)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            ).map_err(|e| VellumError::Database(e.to_string()))?;

            let mut del_tags = tx.prepare_cached("DELETE FROM book_tags WHERE book_id = ?1")
                .map_err(|e| VellumError::Database(e.to_string()))?;
            let mut ins_tag = tx.prepare_cached("INSERT OR IGNORE INTO tags (name) VALUES (?1)")
                .map_err(|e| VellumError::Database(e.to_string()))?;
            let mut ins_btag = tx.prepare_cached("INSERT OR IGNORE INTO book_tags (book_id, tag_name) VALUES (?1, ?2)")
                .map_err(|e| VellumError::Database(e.to_string()))?;

            for book in books {
                book_stmt.execute(params![
                    book.id,
                    book.title,
                    book.author,
                    book.publisher,
                    book.publish_date,
                    book.isbn,
                    book.series,
                    book.series_index,
                    book.description,
                    book.cover_path,
                    book.added_at,
                    book.current_chapter_index,
                    book.total_chapters,
                    book.progress_percent,
                    book.last_read_at,
                    if book.is_finished { 1 } else { 0 },
                    book.rating,
                ]).map_err(|e| VellumError::Database(e.to_string()))?;

                del_fmt.execute(params![book.id])
                    .map_err(|e| VellumError::Database(e.to_string()))?;
                for fmt in &book.formats {
                    ins_fmt.execute(params![
                        fmt.id,
                        book.id,
                        fmt.format,
                        fmt.filename,
                        fmt.filepath,
                        fmt.filesize as i64
                    ]).map_err(|e| VellumError::Database(e.to_string()))?;
                }

                del_tags.execute(params![book.id])
                    .map_err(|e| VellumError::Database(e.to_string()))?;
                for tag in &book.tags {
                    ins_tag.execute(params![tag.name])
                        .map_err(|e| VellumError::Database(e.to_string()))?;
                    ins_btag.execute(params![book.id, tag.name])
                        .map_err(|e| VellumError::Database(e.to_string()))?;
                }
            }
        }
        tx.commit()
            .map_err(|e| VellumError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn find_by_id(&self, id: &str) -> VellumResult<Option<Book>> {
        let conn = self.db.lock()?;
        Self::load_book_by_id(&conn, id)
    }

    pub fn find_all(&self) -> VellumResult<Vec<Book>> {
        self.search_books(None, None)
    }

    pub fn search_books(
        &self,
        search: Option<&str>,
        tag_name: Option<&str>,
    ) -> VellumResult<Vec<Book>> {
        let conn = self.db.lock()?;
        let search_pattern = search
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| format!("%{}%", s.to_lowercase()));
        let tag_filter = tag_name
            .map(str::trim)
            .filter(|t| !t.is_empty() && !t.eq_ignore_ascii_case("Tous"))
            .map(String::from);

        let ids: Vec<String> = if search_pattern.is_none() && tag_filter.is_none() {
            let mut stmt = conn
                .prepare("SELECT id FROM books ORDER BY added_at DESC")
                .map_err(|e| VellumError::Database(e.to_string()))?;
            let rows = stmt.query_map([], |row| row.get(0))
                .map_err(|e| VellumError::Database(e.to_string()))?;
            let mut out = Vec::new();
            for r in rows {
                out.push(r.map_err(|e| VellumError::Database(e.to_string()))?);
            }
            out
        } else {
            let mut stmt = conn
                .prepare(
                    "SELECT DISTINCT b.id
                     FROM books b
                     LEFT JOIN book_tags bt ON bt.book_id = b.id
                     WHERE (?1 IS NULL OR LOWER(b.title) LIKE ?1 OR LOWER(b.author) LIKE ?1 OR LOWER(COALESCE(b.series, '')) LIKE ?1 OR LOWER(COALESCE(b.isbn, '')) LIKE ?1 OR LOWER(COALESCE(b.description, '')) LIKE ?1)
                       AND (?2 IS NULL OR bt.tag_name = ?2)
                     ORDER BY b.added_at DESC",
                )
                .map_err(|e| VellumError::Database(e.to_string()))?;
            let rows = stmt.query_map(params![search_pattern, tag_filter], |row| row.get(0))
                .map_err(|e| VellumError::Database(e.to_string()))?;
            let mut out = Vec::new();
            for r in rows {
                out.push(r.map_err(|e| VellumError::Database(e.to_string()))?);
            }
            out
        };

        Self::load_books_by_ids_batch(&conn, &ids)
    }

    /// Recherche plein texte via SQLite FTS5 (insensible aux accents, triée par pertinence BM25).
    pub fn search_fts(&self, query: &str, tag_name: Option<&str>) -> VellumResult<Vec<Book>> {
        let Some(fts_expr) = to_fts_match_expr(query) else {
            return self.search_books(None, tag_name);
        };
        let tag_filter = tag_name
            .map(str::trim)
            .filter(|t| !t.is_empty() && !t.eq_ignore_ascii_case("Tous"))
            .map(String::from);

        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT DISTINCT f.book_id
                 FROM books_fts f
                 LEFT JOIN book_tags bt ON bt.book_id = f.book_id
                 WHERE books_fts MATCH ?1
                   AND (?2 IS NULL OR bt.tag_name = ?2)
                 ORDER BY bm25(books_fts, 0.0, 10.0, 6.0, 3.0, 4.0, 8.0, 1.0)",
            )
            .map_err(|e| VellumError::Database(e.to_string()))?;

        let ids: Vec<String> = stmt
            .query_map(params![fts_expr, tag_filter], |row| row.get(0))
            .map_err(|e| VellumError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| VellumError::Database(e.to_string()))?;

        Self::load_books_by_ids_batch(&conn, &ids)
    }

    pub fn count(&self) -> VellumResult<usize> {
        let conn = self.db.lock()?;
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM books", [], |row| row.get(0))
            .map_err(|e| VellumError::Database(e.to_string()))?;
        Ok(count as usize)
    }

    pub fn delete(&self, id: &str) -> VellumResult<()> {
        let conn = self.db.lock()?;
        conn.execute("DELETE FROM book_formats WHERE book_id = ?1", params![id])
            .map_err(|e| VellumError::Database(e.to_string()))?;
        conn.execute("DELETE FROM book_tags WHERE book_id = ?1", params![id])
            .map_err(|e| VellumError::Database(e.to_string()))?;
        conn.execute("DELETE FROM books WHERE id = ?1", params![id])
            .map_err(|e| VellumError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn delete_all(&self) -> VellumResult<()> {
        let conn = self.db.lock()?;
        conn.execute_batch(
            "DELETE FROM book_formats;
             DELETE FROM book_tags;
             DELETE FROM books;
             DELETE FROM tags;",
        )
        .map_err(|e| VellumError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn list_all_series(&self) -> VellumResult<Vec<(String, usize)>> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT series, COUNT(*) as cnt
                 FROM books
                 WHERE series IS NOT NULL AND TRIM(series) != ''
                 GROUP BY series
                 ORDER BY series COLLATE NOCASE ASC",
            )
            .map_err(|e| VellumError::Database(e.to_string()))?;

        let list = stmt
            .query_map([], |row| {
                let name: String = row.get(0)?;
                let count: i64 = row.get(1)?;
                Ok((name, count as usize))
            })
            .map_err(|e| VellumError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| VellumError::Database(e.to_string()))?;
        Ok(list)
    }

    /// Met à jour en masse la série et le numéro de tome de livres existants, sans toucher aux titres.
    pub fn update_series_batch(&self, updates: &[(String, String, Option<f64>)]) -> VellumResult<usize> {
        if updates.is_empty() {
            return Ok(0);
        }
        let mut conn = self.db.lock()?;
        let tx = conn
            .transaction()
            .map_err(|e| VellumError::Database(e.to_string()))?;
        let count = {
            let mut stmt = tx
                .prepare_cached("UPDATE books SET series = ?1, series_index = ?2 WHERE id = ?3")
                .map_err(|e| VellumError::Database(e.to_string()))?;
            let mut updated = 0;
            for (id, series, index) in updates {
                updated += stmt
                    .execute(params![series, index, id])
                    .map_err(|e| VellumError::Database(e.to_string()))?;
            }
            updated
        };
        tx.commit().map_err(|e| VellumError::Database(e.to_string()))?;
        Ok(count)
    }

    fn load_books_by_ids_batch(conn: &Connection, ids: &[String]) -> VellumResult<Vec<Book>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let mut id_order = std::collections::HashMap::with_capacity(ids.len());
        for (i, id) in ids.iter().enumerate() {
            id_order.insert(id.as_str(), i);
        }

        // Process in chunks of 500 to stay safely below SQLite parameter limit (999)
        let mut books: Vec<Option<Book>> = vec![None; ids.len()];

        for chunk in ids.chunks(500) {
            let placeholders = chunk.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            let query = format!(
                "SELECT id, title, author, publisher, publish_date, isbn, series, series_index, description, cover_path,
                        added_at, current_chapter_index, total_chapters, progress_percent,
                        last_read_at, is_finished, rating
                 FROM books WHERE id IN ({placeholders})"
            );
            let mut stmt = conn.prepare(&query).map_err(|e| VellumError::Database(e.to_string()))?;
            let rows = stmt.query_map(rusqlite::params_from_iter(chunk), |row| {
                let is_fin: i32 = row.get::<_, Option<i32>>(15)?.unwrap_or(0);
                Ok(Book {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    author: row.get(2)?,
                    publisher: row.get(3)?,
                    publish_date: row.get(4)?,
                    isbn: row.get(5)?,
                    series: row.get(6)?,
                    series_index: row.get(7)?,
                    description: row.get(8)?,
                    cover_path: row.get(9)?,
                    added_at: row.get::<_, Option<String>>(10)?.unwrap_or_default(),
                    current_chapter_index: row.get::<_, Option<i32>>(11)?.unwrap_or(0),
                    total_chapters: row.get::<_, Option<i32>>(12)?.unwrap_or(0),
                    progress_percent: row.get::<_, Option<f64>>(13)?.unwrap_or(0.0),
                    last_read_at: row.get(14)?,
                    is_finished: is_fin != 0,
                    rating: row.get(16)?,
                    formats: Vec::new(),
                    tags: Vec::new(),
                })
            }).map_err(|e| VellumError::Database(e.to_string()))?;

            for r in rows {
                let b = r.map_err(|e| VellumError::Database(e.to_string()))?;
                if let Some(&idx) = id_order.get(b.id.as_str()) {
                    books[idx] = Some(b);
                }
            }

            // Load formats in chunk
            let fmt_query = format!(
                "SELECT id, book_id, format, filename, filepath, filesize
                 FROM book_formats WHERE book_id IN ({placeholders})"
            );
            let mut fmt_stmt = conn.prepare(&fmt_query).map_err(|e| VellumError::Database(e.to_string()))?;
            let fmt_rows = fmt_stmt.query_map(rusqlite::params_from_iter(chunk), |row| {
                let sz: i64 = row.get(5)?;
                Ok((
                    row.get::<_, String>(1)?,
                    BookFormat {
                        id: row.get(0)?,
                        book_id: Some(row.get(1)?),
                        format: row.get(2)?,
                        filename: row.get(3)?,
                        filepath: row.get(4)?,
                        filesize: sz.max(0) as u64,
                    },
                ))
            }).map_err(|e| VellumError::Database(e.to_string()))?;

            for r in fmt_rows {
                let (bid, fmt) = r.map_err(|e| VellumError::Database(e.to_string()))?;
                if let Some(&idx) = id_order.get(bid.as_str()) {
                    if let Some(ref mut b) = books[idx] {
                        b.formats.push(fmt);
                    }
                }
            }

            // Load tags in chunk
            let tag_query = format!(
                "SELECT book_id, tag_name FROM book_tags WHERE book_id IN ({placeholders})"
            );
            let mut tag_stmt = conn.prepare(&tag_query).map_err(|e| VellumError::Database(e.to_string()))?;
            let tag_rows = tag_stmt.query_map(rusqlite::params_from_iter(chunk), |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            }).map_err(|e| VellumError::Database(e.to_string()))?;

            for r in tag_rows {
                let (bid, tname) = r.map_err(|e| VellumError::Database(e.to_string()))?;
                if let Some(&idx) = id_order.get(bid.as_str()) {
                    if let Some(ref mut b) = books[idx] {
                        b.tags.push(Tag::new(tname));
                    }
                }
            }
        }

        Ok(books.into_iter().flatten().collect())
    }

    fn load_book_by_id(conn: &Connection, id: &str) -> VellumResult<Option<Book>> {
        let book_opt = conn
            .query_row(
                "SELECT id, title, author, publisher, publish_date, isbn, series, series_index, description, cover_path,
                        added_at, current_chapter_index, total_chapters, progress_percent,
                        last_read_at, is_finished, rating
                 FROM books WHERE id = ?1",
                params![id],
                |row| {
                    let is_fin: i32 = row.get::<_, Option<i32>>(15)?.unwrap_or(0);
                    Ok(Book {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        author: row.get(2)?,
                        publisher: row.get(3)?,
                        publish_date: row.get(4)?,
                        isbn: row.get(5)?,
                        series: row.get(6)?,
                        series_index: row.get(7)?,
                        description: row.get(8)?,
                        cover_path: row.get(9)?,
                        added_at: row.get::<_, Option<String>>(10)?.unwrap_or_default(),
                        current_chapter_index: row.get::<_, Option<i32>>(11)?.unwrap_or(0),
                        total_chapters: row.get::<_, Option<i32>>(12)?.unwrap_or(0),
                        progress_percent: row.get::<_, Option<f64>>(13)?.unwrap_or(0.0),
                        last_read_at: row.get(14)?,
                        is_finished: is_fin != 0,
                        rating: row.get(16)?,
                        formats: Vec::new(),
                        tags: Vec::new(),
                    })
                },
            )
            .optional()
            .map_err(|e| VellumError::Database(e.to_string()))?;

        let Some(mut book) = book_opt else {
            return Ok(None);
        };

        // Load formats
        let mut fmt_stmt = conn
            .prepare(
                "SELECT id, book_id, format, filename, filepath, filesize
                 FROM book_formats WHERE book_id = ?1",
            )
            .map_err(|e| VellumError::Database(e.to_string()))?;
        let formats = fmt_stmt
            .query_map(params![id], |row| {
                let sz: i64 = row.get(5)?;
                Ok(BookFormat {
                    id: row.get(0)?,
                    book_id: Some(row.get(1)?),
                    format: row.get(2)?,
                    filename: row.get(3)?,
                    filepath: row.get(4)?,
                    filesize: sz.max(0) as u64,
                })
            })
            .map_err(|e| VellumError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| VellumError::Database(e.to_string()))?;
        book.formats = formats;

        // Load tags
        let mut tag_stmt = conn
            .prepare("SELECT tag_name FROM book_tags WHERE book_id = ?1 ORDER BY tag_name ASC")
            .map_err(|e| VellumError::Database(e.to_string()))?;
        let tags = tag_stmt
            .query_map(params![id], |row| Ok(Tag::new(row.get::<_, String>(0)?)))
            .map_err(|e| VellumError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| VellumError::Database(e.to_string()))?;
        book.tags = tags;

        Ok(Some(book))
    }

    /// Indexe (ou réindexe) le texte brut des chapitres d'un livre dans `book_content_fts`.
    pub fn replace_book_content(
        &self,
        book_id: &str,
        chapters: &[(usize, String, String)],
    ) -> VellumResult<()> {
        let mut conn = self.db.lock()?;
        let tx = conn
            .transaction()
            .map_err(|e| VellumError::Database(e.to_string()))?;

        tx.execute(
            "DELETE FROM book_content_fts WHERE book_id = ?1",
            params![book_id],
        )
        .map_err(|e| VellumError::Database(e.to_string()))?;

        {
            let mut stmt = tx
                .prepare(
                    "INSERT INTO book_content_fts (book_id, chapter_index, chapter_title, content)
                     VALUES (?1, ?2, ?3, ?4)",
                )
                .map_err(|e| VellumError::Database(e.to_string()))?;

            for (idx, title, text) in chapters {
                stmt.execute(params![book_id, *idx as i64, title, text])
                    .map_err(|e| VellumError::Database(e.to_string()))?;
            }
        }

        tx.commit()
            .map_err(|e| VellumError::Database(e.to_string()))?;
        Ok(())
    }

    /// Recherche plein texte à l'intérieur du contenu indexé des livres (FTS5 + BM25).
    pub fn search_book_content(
        &self,
        query: &str,
        limit: usize,
    ) -> VellumResult<Vec<ContentSearchHit>> {
        let Some(fts_expr) = to_fts_match_expr(query) else {
            return Ok(Vec::new());
        };
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT book_id, chapter_index, chapter_title,
                        snippet(book_content_fts, 3, '[', ']', '…', 18)
                 FROM book_content_fts
                 WHERE book_content_fts MATCH ?1
                 ORDER BY bm25(book_content_fts, 0.0, 0.0, 2.0, 1.0)
                 LIMIT ?2",
            )
            .map_err(|e| VellumError::Database(e.to_string()))?;

        let rows = stmt
            .query_map(params![fts_expr, limit.max(1) as i64], |row| {
                let idx: i64 = row.get(1)?;
                Ok(ContentSearchHit {
                    book_id: row.get(0)?,
                    chapter_index: idx.max(0) as usize,
                    chapter_title: row.get(2)?,
                    snippet: row.get(3)?,
                })
            })
            .map_err(|e| VellumError::Database(e.to_string()))?;

        let mut hits = Vec::new();
        for item in rows {
            hits.push(item.map_err(|e| VellumError::Database(e.to_string()))?);
        }
        Ok(hits)
    }
}
