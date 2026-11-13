use rusqlite::{params, OptionalExtension};
use vellum_domain::{VellumError, VellumResult, VirtualLibrary};

use crate::Database;

/// Dépôt SQLite pour la persistance des bibliothèques virtuelles.
#[derive(Clone)]
pub struct VirtualLibraryRepository {
    db: Database,
}

impl VirtualLibraryRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub fn save(&self, vl: &VirtualLibrary) -> VellumResult<()> {
        let conn = self.db.lock()?;
        conn.execute(
            "INSERT INTO virtual_libraries (id, name, query, color, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                query = excluded.query,
                color = excluded.color",
            params![vl.id, vl.name, vl.query, vl.color, vl.created_at],
        )
        .map_err(|e| VellumError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn find_all(&self) -> VellumResult<Vec<VirtualLibrary>> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, query, color, created_at
                 FROM virtual_libraries
                 ORDER BY name COLLATE NOCASE ASC",
            )
            .map_err(|e| VellumError::Database(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(VirtualLibrary {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    query: row.get(2)?,
                    color: row.get(3)?,
                    created_at: row.get(4)?,
                })
            })
            .map_err(|e| VellumError::Database(e.to_string()))?;

        let mut list = Vec::new();
        for item in rows {
            list.push(item.map_err(|e| VellumError::Database(e.to_string()))?);
        }
        Ok(list)
    }

    pub fn find_by_id(&self, id: &str) -> VellumResult<Option<VirtualLibrary>> {
        let conn = self.db.lock()?;
        conn.query_row(
            "SELECT id, name, query, color, created_at
             FROM virtual_libraries
             WHERE id = ?1",
            params![id],
            |row| {
                Ok(VirtualLibrary {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    query: row.get(2)?,
                    color: row.get(3)?,
                    created_at: row.get(4)?,
                })
            },
        )
        .optional()
        .map_err(|e| VellumError::Database(e.to_string()))
    }

    pub fn delete(&self, id: &str) -> VellumResult<bool> {
        let conn = self.db.lock()?;
        let deleted = conn
            .execute("DELETE FROM virtual_libraries WHERE id = ?1", params![id])
            .map_err(|e| VellumError::Database(e.to_string()))?;
        Ok(deleted > 0)
    }
}

