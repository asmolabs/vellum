mod epub_builder;
mod feed_parser;

use chrono::Utc;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};

use vellum_domain::{Book, NewsArticle, NewsFeedConfig, NewsRecipe, VellumError, VellumResult};
use vellum_storage::ParamRepository;

use crate::book_service::BookService;
use crate::mail_service::MailService;

pub const DEFAULT_NEWS_RECIPES_PARAM_KEY: &str = "news_recipes";

fn shared_async_http_client() -> VellumResult<&'static reqwest::Client> {
    static ASYNC_CLIENT: LazyLock<Result<reqwest::Client, String>> = LazyLock::new(|| {
        reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (compatible; VellumNewsEngine/1.0; +https://vellum.local)")
            .timeout(std::time::Duration::from_secs(12))
            .build()
            .map_err(|e| format!("Erreur client HTTP: {e}"))
    });
    match &*ASYNC_CLIENT {
        Ok(client) => Ok(client),
        Err(msg) => Err(VellumError::Plugin(msg.clone())),
    }
}

#[derive(Clone)]
pub struct NewsService {
    book_service: BookService,
    param_repo: ParamRepository,
    mail_service: Option<Arc<MailService>>,
}

impl PartialEq for NewsService {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl NewsService {
    pub fn new(
        book_service: BookService,
        param_repo: ParamRepository,
        mail_service: Option<Arc<MailService>>,
    ) -> Self {
        Self {
            book_service,
            param_repo,
            mail_service,
        }
    }

    /// Returns the list of configured news recipes, providing default ones if empty
    pub fn get_recipes(&self) -> Vec<NewsRecipe> {
        if let Ok(Some(json)) = self.param_repo.get_value(DEFAULT_NEWS_RECIPES_PARAM_KEY) {
            if let Ok(recipes) = serde_json::from_str::<Vec<NewsRecipe>>(&json) {
                if !recipes.is_empty() {
                    return recipes;
                }
            }
        }
        Self::default_recipes()
    }

    /// Saves the list of recipes to database storage
    pub fn save_recipes(&self, recipes: &[NewsRecipe]) -> VellumResult<()> {
        let json = serde_json::to_string(recipes)?;
        self.param_repo
            .set_value(DEFAULT_NEWS_RECIPES_PARAM_KEY, &json)?;
        Ok(())
    }

    /// Returns default pre-configured recipes
    pub fn default_recipes() -> Vec<NewsRecipe> {
        vec![
            NewsRecipe {
                id: "recipe-tech".to_string(),
                title: "Tech & Hackers Digest".to_string(),
                description: Some(
                    "Le meilleur de l'actualité tech, open-source et développement.".to_string(),
                ),
                feeds: vec![
                    NewsFeedConfig::new("hn", "Hacker News", "https://news.ycombinator.com/rss"),
                    NewsFeedConfig::new("gh", "GitHub Blog", "https://github.blog/feed/"),
                ],
                max_articles_per_feed: 8,
                auto_send_kindle: false,
            },
            NewsRecipe {
                id: "recipe-france".to_string(),
                title: "Presse & Actualités".to_string(),
                description: Some("Résumé de la presse francophone du jour.".to_string()),
                feeds: vec![
                    NewsFeedConfig::new(
                        "lemonde",
                        "Le Monde",
                        "https://www.lemonde.fr/rss/une.xml",
                    ),
                    NewsFeedConfig::new(
                        "franceinfo",
                        "France Info",
                        "https://www.francetvinfo.fr/titres.rss",
                    ),
                ],
                max_articles_per_feed: 8,
                auto_send_kindle: false,
            },
            NewsRecipe {
                id: "recipe-world".to_string(),
                title: "International World News".to_string(),
                description: Some("World news from international media.".to_string()),
                feeds: vec![NewsFeedConfig::new(
                    "bbc",
                    "BBC World",
                    "http://feeds.bbci.co.uk/news/world/rss.xml",
                )],
                max_articles_per_feed: 10,
                auto_send_kindle: false,
            },
        ]
    }

    /// Parses RSS 2.0 or Atom XML content into structured `NewsArticle` objects
    pub fn parse_feed_xml(
        xml: &str,
        feed_name: &str,
        max_articles: usize,
    ) -> VellumResult<Vec<NewsArticle>> {
        feed_parser::parse_feed_xml(xml, feed_name, max_articles)
    }

    /// Fetches articles from a live HTTP(S) RSS or Atom feed
    pub async fn fetch_feed_articles(
        &self,
        feed: &NewsFeedConfig,
        max_articles: usize,
    ) -> VellumResult<Vec<NewsArticle>> {
        let client = shared_async_http_client()?;

        let response = client.get(&feed.url).send().await.map_err(|e| {
            VellumError::Plugin(format!("Échec téléchargement flux '{}': {e}", feed.name))
        })?;

        let xml = response
            .text()
            .await
            .map_err(|e| VellumError::Plugin(format!("Échec lecture flux '{}': {e}", feed.name)))?;

        Self::parse_feed_xml(&xml, &feed.name, max_articles)
    }

    /// Fetches all feeds of a recipe and compiles them into a clean EPUB file
    pub async fn generate_recipe_epub(
        &self,
        recipe: &NewsRecipe,
        output_path: &Path,
    ) -> VellumResult<(PathBuf, usize)> {
        let mut all_articles = Vec::new();

        for feed in &recipe.feeds {
            match self
                .fetch_feed_articles(feed, recipe.max_articles_per_feed)
                .await
            {
                Ok(mut articles) => {
                    tracing::info!(
                        "Flux '{}' : {} articles récupérés",
                        feed.name,
                        articles.len()
                    );
                    all_articles.append(&mut articles);
                }
                Err(e) => {
                    tracing::warn!("Avertissement flux '{}': {e}", feed.name);
                }
            }
        }

        if all_articles.is_empty() {
            return Err(VellumError::NotFound(format!(
                "Aucun article n'a pu être récupéré pour la recette '{}'. Vérifiez les URLs des flux.",
                recipe.title
            )));
        }

        let total_count = all_articles.len();
        let epub_file = Self::build_news_epub(recipe, &all_articles, output_path)?;

        Ok((epub_file, total_count))
    }

    /// Builds a well-formed EPUB archive from a list of articles
    pub fn build_news_epub(
        recipe: &NewsRecipe,
        articles: &[NewsArticle],
        output_path: &Path,
    ) -> VellumResult<PathBuf> {
        epub_builder::build_news_epub(recipe, articles, output_path)
    }

    /// Fetches, compiles and imports a recipe into the Vellum Library as a new book
    pub async fn run_recipe_and_import(&self, recipe_id: &str) -> VellumResult<Book> {
        let recipes = self.get_recipes();
        let recipe = recipes
            .into_iter()
            .find(|r| r.id == recipe_id)
            .ok_or_else(|| VellumError::NotFound(format!("Recette introuvable: {recipe_id}")))?;

        let today = Utc::now();
        let date_slug = today.format("%Y-%m-%d").to_string();
        let filename = format!(
            "{}_{}.epub",
            epub_builder::sanitize_slug(&recipe.title),
            date_slug
        );
        let temp_dir = tempfile::tempdir()?;
        let target_epub = temp_dir.path().join(&filename);

        let (_path, total) = self.generate_recipe_epub(&recipe, &target_epub).await?;

        // Import into Vellum
        let mut book = self.book_service.import_book(&target_epub)?;

        // Tag and enrich
        let mut tags = book.tags.clone();
        if !tags.iter().any(|t| t.name == "Actualités") {
            tags.push(vellum_domain::Tag::new("Actualités"));
        }
        if !tags.iter().any(|t| t.name == "Presse") {
            tags.push(vellum_domain::Tag::new("Presse"));
        }
        book.tags = tags;
        book.description = Some(format!(
            "Digest d'actualités compilé automatiquement le {}.\nComprend {} articles issus de {} flux.",
            today.format("%d/%m/%Y"),
            total,
            recipe.feeds.len()
        ));
        let saved_book = self.book_service.book_repo().save(&book)?;

        // Auto-send to Kindle if requested
        if recipe.auto_send_kindle {
            if let Some(mail_svc) = &self.mail_service {
                if let Ok(Some(json)) = self.param_repo.get_value("smtp_config") {
                    if let Ok(smtp_cfg) = serde_json::from_str::<vellum_domain::SmtpConfig>(&json) {
                        if smtp_cfg.enabled && !smtp_cfg.kindle_email.is_empty() {
                            let _ = mail_svc
                                .send_to_kindle(&saved_book, &smtp_cfg, Some("epub"))
                                .await;
                        }
                    }
                }
            }
        }

        Ok(saved_book)
    }
}

#[cfg(test)]
mod tests {
    use super::feed_parser::sanitize_article_html;
    use super::*;
    use crate::epub_editor::EpubEditor;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_rss_2_xml() {
        let rss_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:dc="http://purl.org/dc/elements/1.1/">
  <channel>
    <title>Hacker News</title>
    <link>https://news.ycombinator.com/</link>
    <description>Links for the intellectually curious</description>
    <item>
      <title>Rust 1.95 Released</title>
      <link>https://blog.rust-lang.org/2026/rust-release</link>
      <dc:creator>Rust Team</dc:creator>
      <pubDate>Thu, 12 Nov 2026 10:00:00 GMT</pubDate>
      <description>&lt;p&gt;The Rust team is happy to announce a new version.&lt;/p&gt;</description>
    </item>
    <item>
      <title>Show HN: Vellum Local Ebook</title>
      <link>https://github.com/Asmo1973/vellum</link>
      <dc:creator>Asmo</dc:creator>
      <description>Check out our decentralized library.</description>
    </item>
  </channel>
</rss>"#;

        let articles = NewsService::parse_feed_xml(rss_xml, "Hacker News", 10).unwrap();
        assert_eq!(articles.len(), 2);
        assert_eq!(articles[0].title, "Rust 1.95 Released");
        assert_eq!(articles[0].author.as_deref(), Some("Rust Team"));
        assert!(articles[0].content_html.contains("The Rust team is happy"));
        assert_eq!(articles[1].title, "Show HN: Vellum Local Ebook");
    }

    #[test]
    fn test_parse_atom_xml() {
        let atom_xml = r#"<?xml version="1.0" encoding="utf-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>GitHub Blog</title>
  <link href="https://github.blog/"/>
  <entry>
    <title>New Security Features in GitHub</title>
    <link href="https://github.blog/sec-update"/>
    <author><name>Security Team</name></author>
    <published>2026-11-12T08:00:00Z</published>
    <content type="html">&lt;p&gt;We are upgrading code analysis.&lt;/p&gt;</content>
  </entry>
</feed>"#;

        let articles = NewsService::parse_feed_xml(atom_xml, "GitHub Blog", 10).unwrap();
        assert_eq!(articles.len(), 1);
        assert_eq!(articles[0].title, "New Security Features in GitHub");
        assert_eq!(articles[0].author.as_deref(), Some("Security Team"));
        assert_eq!(
            articles[0].link.as_deref(),
            Some("https://github.blog/sec-update")
        );
    }

    #[test]
    fn test_build_news_epub_and_verify_with_inspector() {
        let mut recipe = NewsRecipe::new("tech-digest", "Tech Daily");
        recipe.feeds.push(NewsFeedConfig::new(
            "hn",
            "HN",
            "https://news.ycombinator.com/rss",
        ));

        let articles = vec![
            NewsArticle {
                title: "Deep Dive into Tokio 1.5".to_string(),
                author: Some("Alice".to_string()),
                link: Some("https://tokio.rs/blog/dive".to_string()),
                pub_date: Some("12 Nov 2026".to_string()),
                content_html: "<p>Tokio handles asynchronous runtime seamlessly.</p>".to_string(),
                feed_name: "Tokio Blog".to_string(),
            },
            NewsArticle {
                title: "SQLite 3.50 Innovations".to_string(),
                author: Some("Bob".to_string()),
                link: Some("https://sqlite.org/news".to_string()),
                pub_date: Some("12 Nov 2026".to_string()),
                content_html: "<p>WAL performance has improved again.</p>".to_string(),
                feed_name: "SQLite News".to_string(),
            },
        ];

        let out_file = NamedTempFile::new().unwrap();
        let epub_path = NewsService::build_news_epub(&recipe, &articles, out_file.path()).unwrap();

        // Deep verify the generated EPUB with our EpubEditor
        let diag = EpubEditor::inspect(&epub_path).unwrap();
        assert!(diag.is_valid, "Le journal généré doit être un EPUB valide");
        assert_eq!(diag.spine_count, 3); // cover + 2 articles
        assert_eq!(diag.toc_entries_count, 3);
        assert!(!diag
            .issues
            .iter()
            .any(|i| i.severity == vellum_domain::EpubIssueSeverity::Error));

        let toc = EpubEditor::extract_toc(&epub_path).unwrap();
        assert_eq!(toc.len(), 3);
        assert!(toc[1].label.contains("Deep Dive into Tokio"));
        assert!(toc[2].label.contains("SQLite 3.50 Innovations"));
    }

    #[test]
    fn test_sanitize_article_html_blocks_xss() {
        let dirty = r#"<p>Hello</p><script>alert(1)</script><img src="x" onerror="alert(2)"><img src="https://example.com/pixel.gif" width="1" height="1"><a href="javascript:alert(3)">click</a><svg onload="alert(4)"></svg>"#;
        let clean = sanitize_article_html(dirty);
        assert!(clean.contains("<p>Hello</p>"));
        assert!(!clean.contains("script"));
        assert!(!clean.contains("onerror"));
        assert!(!clean.contains("javascript:"));
        assert!(!clean.contains("onload"));
    }
}
