use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use vellum_domain::{
    BookConverterPlugin, ConfigField, PluginDescriptor, PluginType, TestResult, VellumPlugin,
    VellumResult,
};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

pub struct TextToEpubConverterPlugin {
    descriptor: PluginDescriptor,
}

impl Default for TextToEpubConverterPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl TextToEpubConverterPlugin {
    pub fn new() -> Self {
        Self {
            descriptor: PluginDescriptor {
                id: "converter-txt-to-epub".to_string(),
                name: "Générateur EPUB (TXT/MD ➔ EPUB)".to_string(),
                description: "Structure un document texte brut ou Markdown en véritable livre numérique EPUB avec table des matières et chapitres.".to_string(),
                version: "1.0.0".to_string(),
                plugin_type: PluginType::Conversion,
                external: false,
                command: None,
                supported_extensions: Vec::new(),
                source_format: Some("txt".to_string()),
                target_format: Some("epub".to_string()),
                config_schema: vec![ConfigField::new(
                    "font_family",
                    "Police de lecture",
                    "select",
                    json!("Georgia, serif"),
                    "Famille de police injectée dans la feuille de style.",
                )
                .with_options(vec!["Georgia, serif", "sans-serif", "monospace"])],
            },
        }
    }
}

impl VellumPlugin for TextToEpubConverterPlugin {
    fn descriptor(&self) -> &PluginDescriptor {
        &self.descriptor
    }

    fn test_configuration(&self, _config: &HashMap<String, Value>) -> TestResult {
        TestResult::ok("Générateur EPUB prêt (packaging OCF 2.0 / XHTML pur Rust).")
    }

    fn as_converter_plugin(&self) -> Option<&dyn BookConverterPlugin> {
        Some(self)
    }
}

impl BookConverterPlugin for TextToEpubConverterPlugin {
    fn source_format(&self) -> &str {
        "txt"
    }

    fn target_format(&self) -> &str {
        "epub"
    }

    fn can_convert(&self, from_ext: &str, to_ext: &str) -> bool {
        (from_ext.eq_ignore_ascii_case("txt") || from_ext.eq_ignore_ascii_case("md"))
            && to_ext.eq_ignore_ascii_case("epub")
    }

    fn convert(
        &self,
        input_file: &Path,
        output_file: &Path,
        config: &HashMap<String, Value>,
    ) -> VellumResult<PathBuf> {
        let content = std::fs::read_to_string(input_file)?;
        let default_title = input_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Document");
        let book_title = config
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or(default_title);
        let book_author = config
            .get("author")
            .and_then(|v| v.as_str())
            .unwrap_or("Auteur inconnu");
        let font_family = config
            .get("font_family")
            .and_then(|v| v.as_str())
            .unwrap_or("Georgia, serif");

        let chapters = split_into_chapters(&content, book_title);

        let file = File::create(output_file)?;
        let mut zip = ZipWriter::new(file);

        // 1. mimetype uncompressed
        let stored_opts =
            SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        zip.start_file("mimetype", stored_opts)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        zip.write_all(b"application/epub+zip")?;

        let deflate_opts = SimpleFileOptions::default();

        // 2. META-INF/container.xml
        let container_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#;
        zip.start_file("META-INF/container.xml", deflate_opts)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        zip.write_all(container_xml.as_bytes())?;

        // 3. OEBPS/stylesheet.css
        let css = format!(
            "body {{ font-family: {font_family}; line-height: 1.6; margin: 5%; color: #222; }}\nh1, h2, h3 {{ font-family: sans-serif; color: #111; }}\np {{ margin: 0 0 1em 0; text-indent: 1.2em; }}\n"
        );
        zip.start_file("OEBPS/stylesheet.css", deflate_opts)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        zip.write_all(css.as_bytes())?;

        // 4. Chapters
        for (i, chap) in chapters.iter().enumerate() {
            let xhtml = build_chapter_xhtml(chap);
            zip.start_file(format!("OEBPS/chapter_{:03}.xhtml", i + 1), deflate_opts)
                .map_err(|e| std::io::Error::other(e.to_string()))?;
            zip.write_all(xhtml.as_bytes())?;
        }

        // 5. content.opf
        let opf = build_opf(book_title, book_author, &chapters);
        zip.start_file("OEBPS/content.opf", deflate_opts)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        zip.write_all(opf.as_bytes())?;

        // 6. toc.ncx
        let ncx = build_ncx(book_title, &chapters);
        zip.start_file("OEBPS/toc.ncx", deflate_opts)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        zip.write_all(ncx.as_bytes())?;

        zip.finish()
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        Ok(output_file.to_path_buf())
    }
}

struct Chapter {
    title: String,
    paragraphs: Vec<String>,
}

fn split_into_chapters(content: &str, default_title: &str) -> Vec<Chapter> {
    let mut chapters = Vec::new();
    let mut current_title = default_title.to_string();
    let mut current_paragraphs = Vec::new();
    let mut current_p = String::new();

    for line in content.lines() {
        let trimmed = line.trim();
        let upper = trimmed.to_uppercase();
        if trimmed.starts_with("# ")
            || trimmed.starts_with("## ")
            || upper.starts_with("CHAPITRE ")
            || upper.starts_with("CHAPTER ")
        {
            if !current_p.is_empty() {
                current_paragraphs.push(std::mem::take(&mut current_p));
            }
            if !current_paragraphs.is_empty() {
                chapters.push(Chapter {
                    title: current_title,
                    paragraphs: std::mem::take(&mut current_paragraphs),
                });
            }
            current_title = trimmed.trim_start_matches('#').trim().to_string();
        } else if trimmed.is_empty() {
            if !current_p.is_empty() {
                current_paragraphs.push(std::mem::take(&mut current_p));
            }
        } else {
            if !current_p.is_empty() {
                current_p.push(' ');
            }
            current_p.push_str(trimmed);
        }
    }

    if !current_p.is_empty() {
        current_paragraphs.push(current_p);
    }
    if !current_paragraphs.is_empty() || chapters.is_empty() {
        chapters.push(Chapter {
            title: current_title,
            paragraphs: current_paragraphs,
        });
    }
    chapters
}

fn build_chapter_xhtml(c: &Chapter) -> String {
    let mut sb = String::new();
    sb.push_str("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<!DOCTYPE html>\n");
    sb.push_str("<html xmlns=\"http://www.w3.org/1999/xhtml\">\n<head>\n");
    sb.push_str(&format!("<title>{}</title>\n", escape_xml(&c.title)));
    sb.push_str("<link rel=\"stylesheet\" type=\"text/css\" href=\"stylesheet.css\"/>\n</head>\n<body>\n");
    sb.push_str(&format!("<h1>{}</h1>\n", escape_xml(&c.title)));
    for p in &c.paragraphs {
        sb.push_str(&format!("<p>{}</p>\n", escape_xml(p)));
    }
    sb.push_str("</body>\n</html>");
    sb
}

fn build_opf(title: &str, author: &str, chapters: &[Chapter]) -> String {
    let uuid = Uuid::new_v4();
    let mut sb = String::new();
    sb.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    sb.push_str(
        "<package xmlns=\"http://www.idpf.org/2007/opf\" unique-identifier=\"BookId\" version=\"2.0\">\n",
    );
    sb.push_str("  <metadata xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\n");
    sb.push_str(&format!("    <dc:title>{}</dc:title>\n", escape_xml(title)));
    sb.push_str(&format!(
        "    <dc:creator>{}</dc:creator>\n",
        escape_xml(author)
    ));
    sb.push_str(&format!(
        "    <dc:identifier id=\"BookId\">urn:uuid:{uuid}</dc:identifier>\n"
    ));
    sb.push_str("    <dc:language>fr</dc:language>\n  </metadata>\n  <manifest>\n");
    sb.push_str("    <item id=\"ncx\" href=\"toc.ncx\" media-type=\"application/x-dtbncx+xml\"/>\n");
    sb.push_str("    <item id=\"css\" href=\"stylesheet.css\" media-type=\"text/css\"/>\n");
    for i in 0..chapters.len() {
        sb.push_str(&format!(
            "    <item id=\"chap{:03}\" href=\"chapter_{:03}.xhtml\" media-type=\"application/xhtml+xml\"/>\n",
            i + 1,
            i + 1
        ));
    }
    sb.push_str("  </manifest>\n  <spine toc=\"ncx\">\n");
    for i in 0..chapters.len() {
        sb.push_str(&format!("    <itemref idref=\"chap{:03}\"/>\n", i + 1));
    }
    sb.push_str("  </spine>\n</package>");
    sb
}

fn build_ncx(title: &str, chapters: &[Chapter]) -> String {
    let mut sb = String::new();
    sb.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    sb.push_str("<ncx xmlns=\"http://www.daisy.org/z3986/2005/ncx/\" version=\"2005-1\">\n");
    sb.push_str(&format!(
        "  <docTitle><text>{}</text></docTitle>\n  <navMap>\n",
        escape_xml(title)
    ));
    for (i, c) in chapters.iter().enumerate() {
        sb.push_str(&format!(
            "    <navPoint id=\"nav{:03}\" playOrder=\"{}\">\n      <navLabel><text>{}</text></navLabel>\n      <content src=\"chapter_{:03}.xhtml\"/>\n    </navPoint>\n",
            i + 1,
            i + 1,
            escape_xml(&c.title),
            i + 1
        ));
    }
    sb.push_str("  </navMap>\n</ncx>");
    sb
}

pub fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
