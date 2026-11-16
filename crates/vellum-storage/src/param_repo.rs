use crate::db::Database;
use rusqlite::{params, OptionalExtension};
use vellum_domain::{Param, VellumError, VellumResult};

#[derive(Clone)]
pub struct ParamRepository {
    db: Database,
}

impl ParamRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub fn find_by_plugin_id_and_key(
        &self,
        plugin_id: &str,
        key: &str,
    ) -> VellumResult<Option<Param>> {
        let conn = self.db.lock()?;
        conn.query_row(
            "SELECT id, plugin_id, key, value, COALESCE(updated_at, '')
             FROM params WHERE plugin_id = ?1 AND key = ?2",
            params![plugin_id, key],
            |row| {
                Ok(Param {
                    id: row.get(0)?,
                    plugin_id: row.get(1)?,
                    key: row.get(2)?,
                    value: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            },
        )
        .optional()
        .map_err(|e| VellumError::Database(e.to_string()))
    }

    pub fn save(&self, param: &Param) -> VellumResult<Param> {
        let conn = self.db.lock()?;
        conn.execute(
            "INSERT INTO params (id, plugin_id, key, value, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET
                plugin_id = excluded.plugin_id,
                key = excluded.key,
                value = excluded.value,
                updated_at = excluded.updated_at",
            params![
                param.id,
                param.plugin_id,
                param.key,
                param.value,
                param.updated_at
            ],
        )
        .map_err(|e| VellumError::Database(e.to_string()))?;
        Ok(param.clone())
    }

    pub fn get_value(&self, key: &str) -> VellumResult<Option<String>> {
        let conn = self.db.lock()?;
        conn.query_row(
            "SELECT value FROM params WHERE key = ?1 ORDER BY rowid DESC LIMIT 1",
            params![key],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| VellumError::Database(e.to_string()))
    }

    pub fn set_value(&self, key: &str, value: &str) -> VellumResult<()> {
        let conn = self.db.lock()?;
        let existing: Option<String> = conn
            .query_row(
                "SELECT id FROM params WHERE key = ?1 ORDER BY rowid DESC LIMIT 1",
                params![key],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| VellumError::Database(e.to_string()))?;

        let now = chrono::Utc::now().to_rfc3339();
        if let Some(id) = existing {
            conn.execute(
                "UPDATE params SET value = ?1, updated_at = ?2 WHERE id = ?3",
                params![value, now, id],
            )
            .map_err(|e| VellumError::Database(e.to_string()))?;
        } else {
            let id = uuid::Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO params (id, plugin_id, key, value, updated_at)
                 VALUES (?1, '', ?2, ?3, ?4)",
                params![id, key, value, now],
            )
            .map_err(|e| VellumError::Database(e.to_string()))?;
        }
        Ok(())
    }
}
