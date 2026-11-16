use crate::db::Database;
use rusqlite::params;
use serde_json;
use vellum_domain::{SyncAction, SyncEvent, VellumError, VellumResult};

#[derive(Clone)]
pub struct SyncRepository {
    db: Database,
}

impl SyncRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub fn record_event(&self, event: &SyncEvent) -> VellumResult<()> {
        let conn = self.db.lock()?;
        let action_type = match &event.action {
            SyncAction::BookCreated { .. } => "BOOK_CREATED",
            SyncAction::BookUpdated { .. } => "BOOK_UPDATED",
            SyncAction::BookDeleted { .. } => "BOOK_DELETED",
            SyncAction::ProgressUpdated { .. } => "PROGRESS_UPDATED",
        };
        let payload = serde_json::to_string(&event.action).map_err(VellumError::Json)?;

        conn.execute(
            "INSERT INTO sync_events (id, client_id, timestamp, action_type, payload, synced_at)
             VALUES (?1, ?2, ?3, ?4, ?5, NULL)",
            params![
                event.id,
                event.client_id,
                event.timestamp,
                action_type,
                payload
            ],
        )
        .map_err(|e| VellumError::Database(e.to_string()))?;

        Ok(())
    }

    pub fn get_unsynced_events(&self) -> VellumResult<Vec<SyncEvent>> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, client_id, timestamp, payload
                 FROM sync_events
                 WHERE synced_at IS NULL
                 ORDER BY timestamp ASC",
            )
            .map_err(|e| VellumError::Database(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let client_id: String = row.get(1)?;
                let timestamp: String = row.get(2)?;
                let payload_str: String = row.get(3)?;
                Ok((id, client_id, timestamp, payload_str))
            })
            .map_err(|e| VellumError::Database(e.to_string()))?;

        let mut events = Vec::new();
        for r in rows {
            let (id, client_id, timestamp, payload_str) =
                r.map_err(|e| VellumError::Database(e.to_string()))?;
            let action: SyncAction =
                serde_json::from_str(&payload_str).map_err(VellumError::Json)?;
            events.push(SyncEvent {
                id,
                client_id,
                timestamp,
                action,
            });
        }

        Ok(events)
    }

    pub fn count_unsynced_events(&self) -> VellumResult<usize> {
        let conn = self.db.lock()?;
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sync_events WHERE synced_at IS NULL",
                [],
                |row| row.get(0),
            )
            .map_err(|e| VellumError::Database(e.to_string()))?;
        Ok(count as usize)
    }

    pub fn mark_events_synced(&self, ids: &[String], synced_at: &str) -> VellumResult<()> {
        if ids.is_empty() {
            return Ok(());
        }
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare("UPDATE sync_events SET synced_at = ?1 WHERE id = ?2")
            .map_err(|e| VellumError::Database(e.to_string()))?;

        for id in ids {
            stmt.execute(params![synced_at, id])
                .map_err(|e| VellumError::Database(e.to_string()))?;
        }

        Ok(())
    }

    pub fn is_delta_applied(&self, filename: &str) -> VellumResult<bool> {
        let conn = self.db.lock()?;
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM applied_deltas WHERE filename = ?1",
                params![filename],
                |row| row.get(0),
            )
            .map_err(|e| VellumError::Database(e.to_string()))?;
        Ok(count > 0)
    }

    pub fn mark_delta_applied(&self, filename: &str, client_id: &str) -> VellumResult<()> {
        let conn = self.db.lock()?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT OR IGNORE INTO applied_deltas (filename, client_id, applied_at)
             VALUES (?1, ?2, ?3)",
            params![filename, client_id, now],
        )
        .map_err(|e| VellumError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn get_applied_deltas(&self) -> VellumResult<Vec<String>> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare("SELECT filename FROM applied_deltas")
            .map_err(|e| VellumError::Database(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| VellumError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| VellumError::Database(e.to_string()))?;

        Ok(rows)
    }
}
