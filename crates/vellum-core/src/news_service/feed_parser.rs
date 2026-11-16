use regex::Regex;
use roxmltree::Document;
use std::sync::LazyLock;

use vellum_domain::{NewsArticle, VellumError, VellumResult};

/// Parses RSS 2.0 or Atom XML content into structured `NewsArticle` objects.
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

pub(super) fn sanitize_article_html(html: &str) -> String {
    let unescaped = html
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&amp;", "&");

    // Strip tracking pixel images (width="1" or height="1") before whitelist sanitization
    static RE_PIXEL: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?is)<img[^>]*(width=["']1["']|height=["']1["'])[^>]*>"#)
            .expect("valid tracking pixel regex")
    });
    let without_pixels = RE_PIXEL.replace_all(&unescaped, "");

    // Whitelist HTML sanitization via ammonia (blocks script, iframe, svg, on*, javascript:, etc.)
    ammonia::Builder::default()
        .add_url_schemes(["data"])
        .add_generic_attributes(["class", "id", "alt", "title"])
        .add_tag_attributes("img", ["src", "alt", "width", "height"])
        .clean(&without_pixels)
        .to_string()
}
