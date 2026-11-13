use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Tag {
    pub name: String,
}

impl Tag {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

impl std::fmt::Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookFormat {
    pub id: String,
    pub book_id: Option<String>,
    pub format: String,
    pub filename: String,
    pub filepath: String,
    pub filesize: u64,
}

impl BookFormat {
    pub fn new(
        format: impl Into<String>,
        filename: impl Into<String>,
        filepath: impl Into<String>,
        filesize: u64,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            book_id: None,
            format: format.into(),
            filename: filename.into(),
            filepath: filepath.into(),
            filesize,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Book {
    pub id: String,
    pub title: String,
    pub author: String,
    pub publisher: Option<String>,
    pub publish_date: Option<String>,
    pub isbn: Option<String>,
    pub series: Option<String>,
    pub series_index: Option<f64>,
    pub description: Option<String>,
    pub cover_path: Option<String>,
    pub added_at: String,
    pub current_chapter_index: i32,
    pub total_chapters: i32,
    pub progress_percent: f64,
    pub last_read_at: Option<String>,
    pub is_finished: bool,
    pub rating: Option<i32>,
    pub formats: Vec<BookFormat>,
    pub tags: Vec<Tag>,
}

impl Book {
    pub fn new(title: impl Into<String>, author: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            title: title.into(),
            author: author.into(),
            publisher: None,
            publish_date: None,
            isbn: None,
            series: None,
            series_index: None,
            description: None,
            cover_path: None,
            added_at: Utc::now().to_rfc3339(),
            current_chapter_index: 0,
            total_chapters: 0,
            progress_percent: 0.0,
            last_read_at: None,
            is_finished: false,
            rating: None,
            formats: Vec::new(),
            tags: Vec::new(),
        }
    }

    pub fn add_format(&mut self, mut format: BookFormat) {
        format.book_id = Some(self.id.clone());
        self.formats.push(format);
    }

    pub fn add_tag(&mut self, tag: Tag) {
        if !self.tags.iter().any(|t| t.name == tag.name) {
            self.tags.push(tag);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Param {
    pub id: String,
    pub plugin_id: String,
    pub key: String,
    pub value: String,
    pub updated_at: String,
}

impl Param {
    pub fn new(
        id: impl Into<String>,
        plugin_id: impl Into<String>,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            plugin_id: plugin_id.into(),
            key: key.into(),
            value: value.into(),
            updated_at: Utc::now().to_rfc3339(),
        }
    }

    pub fn get_data(&self) -> HashMap<String, Value> {
        serde_json::from_str(&self.value).unwrap_or_default()
    }

    pub fn set_data(&mut self, data: &HashMap<String, Value>) {
        self.value = serde_json::to_string(data).unwrap_or_else(|_| "{}".to_string());
        self.updated_at = Utc::now().to_rfc3339();
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReadingStats {
    pub total_books: usize,
    pub finished_books: usize,
    pub in_progress_books: usize,
    pub favorite_books: usize,
    pub total_reading_hours: f64,
    pub tag_distribution: BTreeMap<String, usize>,
    pub recently_read: Vec<Book>,
    pub activity_by_day: HashMap<String, usize>,
}

/// Bibliothèque virtuelle (vue enregistrée basée sur une requête de recherche Calibre-like).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VirtualLibrary {
    pub id: String,
    pub name: String,
    pub query: String,
    pub color: Option<String>,
    pub created_at: String,
}

impl VirtualLibrary {
    pub fn new(name: impl Into<String>, query: impl Into<String>, color: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            query: query.into(),
            color,
            created_at: Utc::now().to_rfc3339(),
        }
    }
}

/// Type de données d'une colonne personnalisée.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CustomColumnType {
    Text,
    Number,
    Boolean,
    Date,
    Rating,
}

impl CustomColumnType {
    pub fn as_str(&self) -> &'static str {
        match self {
            CustomColumnType::Text => "text",
            CustomColumnType::Number => "number",
            CustomColumnType::Boolean => "boolean",
            CustomColumnType::Date => "date",
            CustomColumnType::Rating => "rating",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "text" => Some(Self::Text),
            "number" => Some(Self::Number),
            "boolean" | "bool" => Some(Self::Boolean),
            "date" => Some(Self::Date),
            "rating" => Some(Self::Rating),
            _ => None,
        }
    }
}

/// Définition d'une colonne personnalisée sur les livres de la bibliothèque.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomColumnDefinition {
    pub id: String,
    pub key: String,
    pub label: String,
    pub column_type: CustomColumnType,
    pub created_at: String,
}

impl CustomColumnDefinition {
    pub fn new(
        key: impl Into<String>,
        label: impl Into<String>,
        column_type: CustomColumnType,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            key: key.into(),
            label: label.into(),
            column_type,
            created_at: Utc::now().to_rfc3339(),
        }
    }
}
