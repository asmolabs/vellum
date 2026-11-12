use serde::{Deserialize, Serialize};

/// Represents an RSS or Atom source feed within a news recipe
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewsFeedConfig {
    pub id: String,
    pub name: String,
    pub url: String,
}

impl NewsFeedConfig {
    pub fn new(id: impl Into<String>, name: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            url: url.into(),
        }
    }
}

/// A Calibre-like news compilation recipe (grouping multiple feeds into a single daily EPUB)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewsRecipe {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub feeds: Vec<NewsFeedConfig>,
    pub max_articles_per_feed: usize,
    pub auto_send_kindle: bool,
}

impl NewsRecipe {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: None,
            feeds: Vec::new(),
            max_articles_per_feed: 10,
            auto_send_kindle: false,
        }
    }
}

/// An article extracted from an RSS 2.0 or Atom feed
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewsArticle {
    pub title: String,
    pub author: Option<String>,
    pub link: Option<String>,
    pub pub_date: Option<String>,
    pub content_html: String,
    pub feed_name: String,
}

