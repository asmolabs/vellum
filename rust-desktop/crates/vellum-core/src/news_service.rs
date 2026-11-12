use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use chrono::Utc;
use regex::Regex;
use roxmltree::Document;
use tempfile::NamedTempFile;
use uuid::Uuid;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

use vellum_domain::{Book, NewsArticle, NewsFeedConfig, NewsRecipe, VellumError, VellumResult};
use vellum_storage::ParamRepository;

use crate::book_service::BookService;
use crate::mail_service::MailService;

pub const DEFAULT_NEWS_RECIPES_PARAM_KEY: &str = "news_recipes";

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
                description: Some("Le meilleur de l'actualité tech, open-source et développement.".to_string()),
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
                    NewsFeedConfig::new("lemonde", "Le Monde", "https://www.lemonde.fr/rss/une.xml"),
                    NewsFeedConfig::new("franceinfo", "France Info", "https://www.francetvinfo.fr/titres.rss"),
                ],
                max_articles_per_feed: 8,
                auto_send_kindle: false,
            },
            NewsRecipe {
                id: "recipe-world".to_string(),
                title: "International World News".to_string(),
                description: Some("World news from international media.".to_string()),
                feeds: vec![
                    NewsFeedConfig::new("bbc", "BBC World", "http://feeds.bbci.co.uk/news/world/rss.xml"),
                ],
                max_articles_per_feed: 10,
                auto_send_kindle: false,
            },
        ]
    }

    /// Parses RSS 2.0 or Atom XML content into structured NewsArticle objects
    pub fn parse_feed_xml(
        xml: &str,
        feed_name: &str,
        max_articles: usize,
    ) -> VellumResult<Vec<NewsArticle>> {
        let doc = Document::parse(xml)
            .map_err(|e| VellumError::InvalidArgument(format!("Erreur parsing XML flux: {e}")))?;

        let mut articles = Vec::new();

        // 1. Try RSS 2.0 (<rss><channel><item>)
        let rss_items: Vec<_> = doc
            .descendants()
            .filter(|n| n.has_tag_name("item"))
            .collect();

        if !rss_items.is_empty() {
            for item in rss_items.into_iter().take(max_articles) {
                let title = item
                    .children()
                    .find(|n| n.has_tag_name("title"))
                    .and_then(|n| n.text())
                    .unwrap_or("Sans titre")
                    .trim()
                    .to_string();

                let link = item
                    .children()
                    .find(|n| n.has_tag_name("link"))
                    .and_then(|n| n.text())
                    .map(|s| s.trim().to_string());

                let author = item
                    .children()
                    .find(|n| n.has_tag_name("creator") || n.has_tag_name("author"))
                    .and_then(|n| n.text())
                    .map(|s| s.trim().to_string());

                let pub_date = item
                    .children()
                    .find(|n| n.has_tag_name("pubDate") || n.has_tag_name("date"))
                    .and_then(|n| n.text())
                    .map(|s| s.trim().to_string());

                let content_html = item
                    .children()
                    .find(|n| n.has_tag_name("encoded"))
                    .and_then(|n| n.text())
                    .or_else(|| {
                        item.children()
                            .find(|n| n.has_tag_name("description"))
                            .and_then(|n| n.text())
                    })
                    .unwrap_or("")
                    .trim()
                    .to_string();

                articles.push(NewsArticle {
                    title,
                    author,
                    link,
                    pub_date,
                    content_html: sanitize_article_html(&content_html),
                    feed_name: feed_name.to_string(),
                });
            }
            return Ok(articles);
        }

        // 2. Try Atom (<feed><entry>)
        let atom_entries: Vec<_> = doc
            .descendants()
            .filter(|n| n.has_tag_name("entry"))
            .collect();

        for entry in atom_entries.into_iter().take(max_articles) {
            let title = entry
                .children()
                .find(|n| n.has_tag_name("title"))
                .and_then(|n| n.text())
                .unwrap_or("Sans titre")
                .trim()
                .to_string();

            let link = entry
                .children()
                .find(|n| n.has_tag_name("link"))
                .and_then(|n| n.attribute("href"))
                .map(|s| s.trim().to_string());

            let author = entry
                .descendants()
                .find(|n| n.has_tag_name("name"))
                .and_then(|n| n.text())
                .map(|s| s.trim().to_string());

            let pub_date = entry
                .children()
                .find(|n| n.has_tag_name("published") || n.has_tag_name("updated"))
                .and_then(|n| n.text())
                .map(|s| s.trim().to_string());

            let content_html = entry
                .children()
                .find(|n| n.has_tag_name("content"))
                .and_then(|n| n.text())
                .or_else(|| {
                    entry
                        .children()
                        .find(|n| n.has_tag_name("summary"))
                        .and_then(|n| n.text())
                })
                .unwrap_or("")
                .trim()
                .to_string();

            articles.push(NewsArticle {
                title,
                author,
                link,
                pub_date,
                content_html: sanitize_article_html(&content_html),
                feed_name: feed_name.to_string(),
            });
        }

        Ok(articles)
    }

    /// Fetches articles from a live HTTP(S) RSS or Atom feed
    pub async fn fetch_feed_articles(
        &self,
        feed: &NewsFeedConfig,
        max_articles: usize,
    ) -> VellumResult<Vec<NewsArticle>> {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (compatible; VellumNewsEngine/1.0; +https://vellum.local)")
            .timeout(std::time::Duration::from_secs(12))
            .build()
            .map_err(|e| VellumError::Plugin(format!("Erreur client HTTP: {e}")))?;

        let response = client
            .get(&feed.url)
            .send()
            .await
            .map_err(|e| VellumError::Plugin(format!("Échec téléchargement flux '{}': {e}", feed.name)))?;

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
            match self.fetch_feed_articles(feed, recipe.max_articles_per_feed).await {
                Ok(mut articles) => {
                    tracing::info!("Flux '{}' : {} articles récupérés", feed.name, articles.len());
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
        let today = Utc::now();
        let date_str = today.format("%d/%m/%Y").to_string();
        let date_iso = today.to_rfc3339();
        let book_title = format!("{} — {}", recipe.title, date_str);
        let book_uuid = format!("urn:uuid:{}", Uuid::new_v4());

        let temp_file = NamedTempFile::new()?;
        let mut zip = ZipWriter::new(temp_file.reopen()?);

        let stored = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        let deflated = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        // 1. mimetype (first uncompressed entry)
        zip.start_file("mimetype", stored)?;
        zip.write_all(b"application/epub+zip")?;

        // 2. META-INF/container.xml
        zip.start_file("META-INF/container.xml", deflated)?;
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#,
        )?;

        // 3. Cover page (OEBPS/cover.xhtml)
        let cover_html = format!(
            r#"<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" lang="fr">
<head>
  <title>Couverture</title>
  <style>
    body {{
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Georgia, serif;
      text-align: center;
      padding: 40px 20px;
      background: #0f172a;
      color: #f8fafc;
      min-height: 80vh;
      display: flex;
      flex-direction: column;
      justify-content: center;
      align-items: center;
    }}
    .icon {{ font-size: 72px; margin-bottom: 24px; }}
    h1 {{ font-size: 28px; font-weight: 800; color: #ffffff; margin-bottom: 12px; }}
    .subtitle {{ font-size: 18px; color: #818cf8; font-weight: 600; margin-bottom: 24px; }}
    .stats {{ font-size: 14px; color: #94a3b8; background: rgba(255,255,255,0.08); padding: 10px 20px; border-radius: 20px; }}
    .footer {{ margin-top: 40px; font-size: 12px; color: #64748b; }}
  </style>
</head>
<body>
  <div class="icon">📰</div>
  <h1>{}</h1>
  <div class="subtitle">Édition du {}</div>
  <div class="stats">{} articles compilés</div>
  <div class="footer">Généré automatiquement par Vellum News Engine</div>
</body>
</html>"#,
            escape_xml(&recipe.title),
            escape_xml(&date_str),
            articles.len()
        );

        zip.start_file("OEBPS/cover.xhtml", deflated)?;
        zip.write_all(cover_html.as_bytes())?;

        // 4. Articles XHTML files
        for (i, art) in articles.iter().enumerate() {
            let byline_author = art.author.as_deref().unwrap_or("Rédaction");
            let byline_date = art.pub_date.as_deref().unwrap_or("");
            let original_link = match &art.link {
                Some(l) => format!(
                    r#"<p class="source-link"><a href="{}">🔗 Lire l'article original</a></p>"#,
                    escape_xml(l)
                ),
                None => String::new(),
            };

            let body_content = if art.content_html.trim().is_empty() {
                format!("<p>Aucun contenu textuel extrait.</p>{original_link}")
            } else {
                format!("{}{original_link}", art.content_html)
            };

            let art_html = format!(
                r#"<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" lang="fr">
<head>
  <title>{}</title>
  <style>
    body {{
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Georgia, serif;
      line-height: 1.7;
      font-size: 18px;
      color: #1e293b;
      margin: 0;
      padding: 24px 16px;
    }}
    h1 {{
      font-size: 24px;
      line-height: 1.3;
      margin-bottom: 8px;
      color: #0f172a;
    }}
    .byline {{
      font-size: 13px;
      color: #64748b;
      margin-bottom: 24px;
      padding-bottom: 12px;
      border-bottom: 1px solid #e2e8f0;
    }}
    .feed-tag {{
      display: inline-block;
      font-weight: 700;
      color: #4f46e5;
      background: #e0e7ff;
      padding: 2px 8px;
      border-radius: 4px;
      margin-right: 6px;
    }}
    p {{
      margin-bottom: 1.2em;
      text-align: justify;
    }}
    img {{
      max-width: 100%;
      height: auto;
      border-radius: 4px;
      display: block;
      margin: 12px 0;
    }}
    .source-link {{
      margin-top: 32px;
      font-size: 14px;
      font-weight: 600;
    }}
    .source-link a {{
      color: #4f46e5;
      text-decoration: none;
    }}
  </style>
</head>
<body>
  <div class="byline">
    <span class="feed-tag">{}</span>
    <span>Par {}</span>
    {}
  </div>
  <h1>{}</h1>
  {}
</body>
</html>"#,
                escape_xml(&art.title),
                escape_xml(&art.feed_name),
                escape_xml(byline_author),
                if !byline_date.is_empty() {
                    format!(" • <span>{}</span>", escape_xml(byline_date))
                } else {
                    String::new()
                },
                escape_xml(&art.title),
                body_content
            );

            zip.start_file(&format!("OEBPS/art_{i}.xhtml"), deflated)?;
            zip.write_all(art_html.as_bytes())?;
        }

        // 5. NCX Table of Contents (OEBPS/toc.ncx)
        let mut ncx = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<ncx xmlns="http://www.daisy.org/z3986/2005/ncx/" version="2005-1">
  <head>
    <meta name="dtb:uid" content="{book_uuid}"/>
    <meta name="dtb:depth" content="2"/>
    <meta name="dtb:totalPageCount" content="0"/>
    <meta name="dtb:maxPageNumber" content="0"/>
  </head>
  <docTitle><text>{}</text></docTitle>
  <navMap>
    <navPoint id="np-cover" playOrder="1">
      <navLabel><text>Couverture</text></navLabel>
      <content src="cover.xhtml"/>
    </navPoint>
"#,
            escape_xml(&book_title)
        );

        for (i, art) in articles.iter().enumerate() {
            ncx.push_str(&format!(
                r#"    <navPoint id="np-{}" playOrder="{}">
      <navLabel><text>[{}] {}</text></navLabel>
      <content src="art_{}.xhtml"/>
    </navPoint>
"#,
                i + 1,
                i + 2,
                escape_xml(&art.feed_name),
                escape_xml(&art.title),
                i
            ));
        }
        ncx.push_str("  </navMap>\n</ncx>");

        zip.start_file("OEBPS/toc.ncx", deflated)?;
        zip.write_all(ncx.as_bytes())?;

        // 6. EPUB 3 Navigation Document (OEBPS/nav.xhtml)
        let mut nav = format!(
            r#"<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops" lang="fr">
<head>
  <title>Sommaire</title>
</head>
<body>
  <nav epub:type="toc" id="toc">
    <h1>Sommaire</h1>
    <ol>
      <li><a href="cover.xhtml">Couverture</a></li>
"#
        );

        for (i, art) in articles.iter().enumerate() {
            nav.push_str(&format!(
                r#"      <li><a href="art_{}.xhtml">[{}] {}</a></li>
"#,
                i,
                escape_xml(&art.feed_name),
                escape_xml(&art.title)
            ));
        }
        nav.push_str("    </ol>\n  </nav>\n</body>\n</html>");

        zip.start_file("OEBPS/nav.xhtml", deflated)?;
        zip.write_all(nav.as_bytes())?;

        // 7. Package OPF file (OEBPS/content.opf)
        let mut opf = format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="BookId">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:identifier id="BookId">{}</dc:identifier>
    <dc:title>{}</dc:title>
    <dc:creator>Vellum News Engine</dc:creator>
    <dc:language>fr</dc:language>
    <dc:date>{}</dc:date>
    <meta property="dcterms:modified">{}</meta>
  </metadata>
  <manifest>
    <item id="ncx" href="toc.ncx" media-type="application/x-dtbncx+xml"/>
    <item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>
    <item id="cover" href="cover.xhtml" media-type="application/xhtml+xml"/>
"#,
            book_uuid,
            escape_xml(&book_title),
            date_iso,
            date_iso
        );

        for i in 0..articles.len() {
            opf.push_str(&format!(
                r#"    <item id="art_{i}" href="art_{i}.xhtml" media-type="application/xhtml+xml"/>
"#
            ));
        }

        opf.push_str(
            r#"  </manifest>
  <spine toc="ncx">
    <itemref idref="cover"/>
"#,
        );

        for i in 0..articles.len() {
            opf.push_str(&format!(
                r#"    <itemref idref="art_{i}"/>
"#
            ));
        }

        opf.push_str("  </spine>\n</package>");

        zip.start_file("OEBPS/content.opf", deflated)?;
        zip.write_all(opf.as_bytes())?;

        zip.finish()?;

        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(temp_file.path(), output_path)?;

        Ok(output_path.to_path_buf())
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
        let filename = format!("{}_{}.epub", sanitize_slug(&recipe.title), date_slug);
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
                            let _ = mail_svc.send_to_kindle(&saved_book, &smtp_cfg, Some("epub")).await;
                        }
                    }
                }
            }
        }

        Ok(saved_book)
    }
}

fn sanitize_article_html(html: &str) -> String {
    let unescaped = html
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&amp;", "&");

    // Strip unsafe tags (script, style, iframe, form, object, embed)
    let mut cleaned = unescaped;
    for tag in &["script", "style", "iframe", "form", "object", "embed"] {
        let pattern = format!(r"(?is)<{tag}[^>]*>.*?</{tag}>");
        if let Ok(re) = Regex::new(&pattern) {
            cleaned = re.replace_all(&cleaned, "").to_string();
        }
    }

    // Strip tracking pixel images (width="1" or height="1")
    let re_pixel = Regex::new(r#"(?is)<img[^>]*(width=["']1["']|height=["']1["'])[^>]*>"#).unwrap();
    re_pixel.replace_all(&cleaned, "").to_string()
}

fn sanitize_slug(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '_' })
        .collect::<String>()
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
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
        assert_eq!(articles[0].link.as_deref(), Some("https://github.blog/sec-update"));
    }

    #[test]
    fn test_build_news_epub_and_verify_with_inspector() {
        let mut recipe = NewsRecipe::new("tech-digest", "Tech Daily");
        recipe.feeds.push(NewsFeedConfig::new("hn", "HN", "https://news.ycombinator.com/rss"));

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
        assert!(!diag.issues.iter().any(|i| i.severity == vellum_domain::EpubIssueSeverity::Error));

        let toc = EpubEditor::extract_toc(&epub_path).unwrap();
        assert_eq!(toc.len(), 3);
        assert!(toc[1].label.contains("Deep Dive into Tokio"));
        assert!(toc[2].label.contains("SQLite 3.50 Innovations"));
    }
}
