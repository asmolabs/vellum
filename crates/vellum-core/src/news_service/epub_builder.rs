use chrono::Utc;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;
use uuid::Uuid;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

use vellum_domain::{NewsArticle, NewsRecipe, VellumError, VellumResult};

fn map_zip<T>(res: zip::result::ZipResult<T>) -> VellumResult<T> {
    res.map_err(|e| VellumError::Zip(e.to_string()))
}

pub(super) fn sanitize_slug(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
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

/// Builds a well-formed EPUB archive from a list of articles.
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
    let deflated =
        SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // 1. mimetype (first uncompressed entry)
    map_zip(zip.start_file("mimetype", stored))?;
    zip.write_all(b"application/epub+zip")?;

    // 2. META-INF/container.xml
    map_zip(zip.start_file("META-INF/container.xml", deflated))?;
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

    map_zip(zip.start_file("OEBPS/cover.xhtml", deflated))?;
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

        map_zip(zip.start_file(format!("OEBPS/art_{i}.xhtml"), deflated))?;
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

    map_zip(zip.start_file("OEBPS/toc.ncx", deflated))?;
    zip.write_all(ncx.as_bytes())?;

    // 6. EPUB 3 Navigation Document (OEBPS/nav.xhtml)
    let mut nav = r#"<!DOCTYPE html>
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
    .to_string();

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

    map_zip(zip.start_file("OEBPS/nav.xhtml", deflated))?;
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

    map_zip(zip.start_file("OEBPS/content.opf", deflated))?;
    zip.write_all(opf.as_bytes())?;

    map_zip(zip.finish())?;

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(temp_file.path(), output_path)?;

    Ok(output_path.to_path_buf())
}
