use crate::db::Database;
use rusqlite::{params, Connection, OptionalExtension};
use vellum_domain::{Book, BookFormat, Tag, VellumError, VellumResult};

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
                id, title, author, publisher, publish_date, isbn, description, cover_path,
                added_at, current_chapter_index, total_chapters, progress_percent,
                last_read_at, is_finished, rating
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
            ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                author = excluded.author,
                publisher = excluded.publisher,
                publish_date = excluded.publish_date,
                isbn = excluded.isbn,
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
        tx.execute("DELETE FROM book_formats WHERE book_id = ?1", params![book.id])
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

        let mut stmt = conn
            .prepare(
                "SELECT DISTINCT b.id
                 FROM books b
                 LEFT JOIN book_tags bt ON bt.book_id = b.id
                 WHERE (?1 IS NULL OR LOWER(b.title) LIKE ?1 OR LOWER(b.author) LIKE ?1 OR LOWER(COALESCE(b.isbn, '')) LIKE ?1 OR LOWER(COALESCE(b.description, '')) LIKE ?1)
                   AND (?2 IS NULL OR bt.tag_name = ?2)
                 ORDER BY b.added_at DESC",
            )
            .map_err(|e| VellumError::Database(e.to_string()))?;

        let ids: Vec<String> = stmt
            .query_map(params![search_pattern, tag_filter], |row| row.get(0))
            .map_err(|e| VellumError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| VellumError::Database(e.to_string()))?;

        let mut books = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(book) = Self::load_book_by_id(&conn, &id)? {
                books.push(book);
            }
        }
        Ok(books)
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

    fn load_book_by_id(conn: &Connection, id: &str) -> VellumResult<Option<Book>> {
        let book_opt = conn
            .query_row(
                "SELECT id, title, author, publisher, publish_date, isbn, description, cover_path,
                        added_at, current_chapter_index, total_chapters, progress_percent,
                        last_read_at, is_finished, rating
                 FROM books WHERE id = ?1",
                params![id],
                |row| {
                    let is_fin: i32 = row.get::<_, Option<i32>>(13)?.unwrap_or(0);
                    Ok(Book {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        author: row.get(2)?,
                        publisher: row.get(3)?,
                        publish_date: row.get(4)?,
                        isbn: row.get(5)?,
                        description: row.get(6)?,
                        cover_path: row.get(7)?,
                        added_at: row.get::<_, Option<String>>(8)?.unwrap_or_default(),
                        current_chapter_index: row.get::<_, Option<i32>>(9)?.unwrap_or(0),
                        total_chapters: row.get::<_, Option<i32>>(10)?.unwrap_or(0),
                        progress_percent: row.get::<_, Option<f64>>(11)?.unwrap_or(0.0),
                        last_read_at: row.get(12)?,
                        is_finished: is_fin != 0,
                        rating: row.get(14)?,
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
            .prepare(
                "SELECT tag_name FROM book_tags WHERE book_id = ?1 ORDER BY tag_name ASC",
            )
            .map_err(|e| VellumError::Database(e.to_string()))?;
        let tags = tag_stmt
            .query_map(params![id], |row| Ok(Tag::new(row.get::<_, String>(0)?)))
            .map_err(|e| VellumError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| VellumError::Database(e.to_string()))?;
        book.tags = tags;

        Ok(Some(book))
    }
}
