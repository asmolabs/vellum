use crate::entity::{Book, BookFormat, Tag};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum SyncAction {
    BookCreated {
        book: Book,
        formats: Vec<BookFormat>,
        tags: Vec<Tag>,
    },
    BookUpdated {
        book: Book,
        tags: Vec<Tag>,
    },
    BookDeleted {
        book_id: String,
    },
    ProgressUpdated {
        book_id: String,
        progress_percent: f64,
        current_chapter_index: i32,
        total_chapters: i32,
        last_read_at: String,
        is_finished: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyncEvent {
    pub id: String,
    pub client_id: String,
    pub timestamp: String,
    pub action: SyncAction,
}

impl SyncEvent {
    pub fn new(client_id: impl Into<String>, action: SyncAction) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            client_id: client_id.into(),
            timestamp: Utc::now().to_rfc3339(),
            action,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyncDelta {
    pub client_id: String,
    pub created_at: String,
    pub events: Vec<SyncEvent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Default)]
pub enum CloudSyncMode {
    #[default]
    GoogleDriveFolder,
    GoogleDriveApi,
}


#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloudSyncConfig {
    pub enabled: bool,
    pub client_id: String,
    pub client_name: String,
    pub mode: CloudSyncMode,
    pub sync_folder_path: Option<PathBuf>,
    pub auto_download_books: bool,
    pub last_synced_at: Option<String>,
}

impl Default for CloudSyncConfig {
    fn default() -> Self {
        let client_id = Uuid::new_v4().to_string()[..8].to_string();
        let client_name = match std::env::var("USER").or_else(|_| std::env::var("USERNAME")) {
            Ok(u) => format!("{u}-device"),
            Err(_) => "vellum-device".to_string(),
        };

        Self {
            enabled: false,
            client_id,
            client_name,
            mode: CloudSyncMode::GoogleDriveFolder,
            sync_folder_path: None,
            auto_download_books: true,
            last_synced_at: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SyncReport {
    pub uploaded_books: usize,
    pub downloaded_books: usize,
    pub pushed_events: usize,
    pub pulled_events: usize,
    pub timestamp: String,
}
