use crate::db::Database;
use rusqlite::{params, OptionalExtension};
use std::collections::BTreeMap;
use vellum_domain::{Tag, VellumError, VellumResult};

#[derive(Clone)]
pub struct TagRepository {
    db: Database,
}

impl TagRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub fn save(&self, tag: &Tag) -> VellumResult<Tag> {
        let conn = self.db.lock()?;
        conn.execute(
            "INSERT OR IGNORE INTO tags (name) VALUES (?1)",
            params![tag.name],
        )
        .map_err(|e| VellumError::Database(e.to_string()))?;
        Ok(tag.clone())
    }

    pub fn find_by_id(&self, name: &str) -> VellumResult<Option<Tag>> {
        let conn = self.db.lock()?;
        conn.query_row(
            "SELECT name FROM tags WHERE name = ?1",
            params![name],
            |row| Ok(Tag::new(row.get::<_, String>(0)?)),
        )
        .optional()
        .map_err(|e| VellumError::Database(e.to_string()))
    }

    pub fn find_tags_with_book_counts(&self) -> VellumResult<BTreeMap<String, usize>> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT bt.tag_name, COUNT(bt.book_id) AS cnt
                 FROM book_tags bt
                 JOIN books b ON b.id = bt.book_id
                 GROUP BY bt.tag_name
                 ORDER BY bt.tag_name ASC",
            )
            .map_err(|e| VellumError::Database(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                let name: String = row.get(0)?;
                let cnt: i64 = row.get(1)?;
                Ok((name, cnt.max(0) as usize))
            })
            .map_err(|e| VellumError::Database(e.to_string()))?;

        let mut map = BTreeMap::new();
        for row in rows {
            let (name, count) = row.map_err(|e| VellumError::Database(e.to_string()))?;
            map.insert(name, count);
        }
        Ok(map)
    }
}
