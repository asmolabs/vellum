use std::collections::BTreeMap;

use rusqlite::{params, OptionalExtension};
use vellum_domain::{CustomColumnDefinition, CustomColumnType, VellumError, VellumResult};

use crate::Database;

/// Dépôt SQLite pour les colonnes personnalisées et leurs valeurs par livre.
#[derive(Clone)]
pub struct CustomColumnRepository {
    db: Database,
}

impl CustomColumnRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub fn save_definition(&self, col: &CustomColumnDefinition) -> VellumResult<()> {
        let conn = self.db.lock()?;
        conn.execute(
            "INSERT INTO custom_columns (id, key, label, column_type, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET
                key = excluded.key,
                label = excluded.label,
                column_type = excluded.column_type",
            params![
                col.id,
                col.key,
                col.label,
                col.column_type.as_str(),
                col.created_at
            ],
        )
        .map_err(|e| VellumError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn list_definitions(&self) -> VellumResult<Vec<CustomColumnDefinition>> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, key, label, column_type, created_at
                 FROM custom_columns
                 ORDER BY label COLLATE NOCASE ASC",
            )
            .map_err(|e| VellumError::Database(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                let raw_type: String = row.get(3)?;
                Ok(CustomColumnDefinition {
                    id: row.get(0)?,
                    key: row.get(1)?,
                    label: row.get(2)?,
                    column_type: CustomColumnType::parse(&raw_type)
                        .unwrap_or(CustomColumnType::Text),
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

    pub fn find_by_key(&self, key: &str) -> VellumResult<Option<CustomColumnDefinition>> {
        let conn = self.db.lock()?;
        conn.query_row(
            "SELECT id, key, label, column_type, created_at
             FROM custom_columns
             WHERE key = ?1",
            params![key],
            |row| {
                let raw_type: String = row.get(3)?;
                Ok(CustomColumnDefinition {
                    id: row.get(0)?,
                    key: row.get(1)?,
                    label: row.get(2)?,
                    column_type: CustomColumnType::parse(&raw_type)
                        .unwrap_or(CustomColumnType::Text),
                    created_at: row.get(4)?,
                })
            },
        )
        .optional()
        .map_err(|e| VellumError::Database(e.to_string()))
    }

    pub fn delete_definition(&self, id: &str) -> VellumResult<bool> {
        let conn = self.db.lock()?;
        let deleted = conn
            .execute("DELETE FROM custom_columns WHERE id = ?1", params![id])
            .map_err(|e| VellumError::Database(e.to_string()))?;
        Ok(deleted > 0)
    }

    pub fn set_value(&self, book_id: &str, column_id: &str, value: &str) -> VellumResult<()> {
        let conn = self.db.lock()?;
        conn.execute(
            "INSERT INTO book_custom_values (book_id, column_id, value)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(book_id, column_id) DO UPDATE SET value = excluded.value",
            params![book_id, column_id, value],
        )
        .map_err(|e| VellumError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn clear_value(&self, book_id: &str, column_id: &str) -> VellumResult<bool> {
        let conn = self.db.lock()?;
        let deleted = conn
            .execute(
                "DELETE FROM book_custom_values WHERE book_id = ?1 AND column_id = ?2",
                params![book_id, column_id],
            )
            .map_err(|e| VellumError::Database(e.to_string()))?;
        Ok(deleted > 0)
    }

    /// Retourne les valeurs personnalisées d'un livre, indexées par clé de colonne (`column.key`).
    pub fn get_values_for_book(&self, book_id: &str) -> VellumResult<BTreeMap<String, String>> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT c.key, v.value
                 FROM book_custom_values v
                 INNER JOIN custom_columns c ON c.id = v.column_id
                 WHERE v.book_id = ?1
                 ORDER BY c.key ASC",
            )
            .map_err(|e| VellumError::Database(e.to_string()))?;

        let rows = stmt
            .query_map(params![book_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| VellumError::Database(e.to_string()))?;

        let mut map = BTreeMap::new();
        for item in rows {
            let (k, v) = item.map_err(|e| VellumError::Database(e.to_string()))?;
            map.insert(k, v);
        }
        Ok(map)
    }
}

