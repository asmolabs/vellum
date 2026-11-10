use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use tracing::info;
use vellum_domain::{VellumError, VellumResult};

#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
    path: PathBuf,
}

impl Database {
    pub fn open(path: impl AsRef<Path>) -> VellumResult<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }

        info!("Initializing SQLite database at {}", path.display());
        let conn = Connection::open(&path).map_err(|e| VellumError::Database(e.to_string()))?;

        // Configure WAL mode & pragmas for concurrent reads and Litestream replication
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;
             PRAGMA busy_timeout = 5000;",
        )
        .map_err(|e| VellumError::Database(e.to_string()))?;

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
            path,
        };
        db.init_schema()?;
        Ok(db)
    }

    pub fn open_in_memory() -> VellumResult<Self> {
        let conn =
            Connection::open_in_memory().map_err(|e| VellumError::Database(e.to_string()))?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")
            .map_err(|e| VellumError::Database(e.to_string()))?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
            path: PathBuf::from(":memory:"),
        };
        db.init_schema()?;
        Ok(db)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn lock(&self) -> VellumResult<MutexGuard<'_, Connection>> {
        self.conn
            .lock()
            .map_err(|e| VellumError::Database(format!("Mutex lock poisoned: {e}")))
    }

    fn init_schema(&self) -> VellumResult<()> {
        let conn = self.lock()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS books (
                id TEXT PRIMARY KEY NOT NULL,
                title TEXT NOT NULL,
                author TEXT NOT NULL,
                publisher TEXT,
                publish_date TEXT,
                isbn TEXT,
                series TEXT,
                series_index REAL,
                description TEXT,
                cover_path TEXT,
                added_at TEXT,
                current_chapter_index INTEGER DEFAULT 0,
                total_chapters INTEGER DEFAULT 0,
                progress_percent REAL DEFAULT 0.0,
                last_read_at TEXT,
                is_finished INTEGER DEFAULT 0,
                rating INTEGER
            );

            CREATE TABLE IF NOT EXISTS book_formats (
                id TEXT PRIMARY KEY NOT NULL,
                book_id TEXT NOT NULL,
                format TEXT NOT NULL,
                filename TEXT NOT NULL,
                filepath TEXT NOT NULL,
                filesize INTEGER NOT NULL,
                FOREIGN KEY (book_id) REFERENCES books(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS tags (
                name TEXT PRIMARY KEY NOT NULL
            );

            CREATE TABLE IF NOT EXISTS book_tags (
                book_id TEXT NOT NULL,
                tag_name TEXT NOT NULL,
                PRIMARY KEY (book_id, tag_name),
                FOREIGN KEY (book_id) REFERENCES books(id) ON DELETE CASCADE,
                FOREIGN KEY (tag_name) REFERENCES tags(name) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS params (
                id TEXT PRIMARY KEY NOT NULL,
                plugin_id TEXT NOT NULL,
                key TEXT NOT NULL,
                value TEXT NOT NULL,
                updated_at TEXT
            );

            CREATE TABLE IF NOT EXISTS sync_events (
                id TEXT PRIMARY KEY NOT NULL,
                client_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                action_type TEXT NOT NULL,
                payload TEXT NOT NULL,
                synced_at TEXT
            );

            CREATE TABLE IF NOT EXISTS applied_deltas (
                filename TEXT PRIMARY KEY NOT NULL,
                client_id TEXT NOT NULL,
                applied_at TEXT NOT NULL
            );",
        )
        .map_err(|e| VellumError::Database(e.to_string()))?;

        // Automatic migration for existing databases without 'isbn' column
        let has_isbn: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('books') WHERE name = 'isbn'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map(|c| c > 0)
            .unwrap_or(false);

        if !has_isbn {
            let _ = conn.execute("ALTER TABLE books ADD COLUMN isbn TEXT;", []);
        }

        // Automatic migration for existing databases without 'series' columns
        let has_series: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('books') WHERE name = 'series'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map(|c| c > 0)
            .unwrap_or(false);

        if !has_series {
            let _ = conn.execute("ALTER TABLE books ADD COLUMN series TEXT;", []);
            let _ = conn.execute("ALTER TABLE books ADD COLUMN series_index REAL;", []);
        }

        Ok(())
    }
}
