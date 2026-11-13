//! Inspecteur, réparateur et éditeur de table des matières EPUB.

mod inspector;
mod ncx_nav;
mod polish;
mod toc;

pub use polish::{PolishOptions, PolishReport};

use std::fs::File;
use std::io::Read;
use std::path::Path;
use regex::Regex;
use roxmltree::Document;
use vellum_domain::{VellumError, VellumResult};
use zip::ZipArchive;

pub(super) fn map_zip<T>(res: zip::result::ZipResult<T>) -> VellumResult<T> {
    res.map_err(|e| VellumError::Zip(e.to_string()))
}

/// EPUB Inspector, Linter & Table of Contents (TOC) Editor
pub struct EpubEditor;

impl EpubEditor {
    fn find_opf_path(zip: &mut ZipArchive<File>) -> VellumResult<String> {
        if let Ok(mut container) = zip.by_name("META-INF/container.xml") {
            let mut xml = String::new();
            if container.read_to_string(&mut xml).is_ok() {
                if let Ok(doc) = Document::parse(&xml) {
                    if let Some(rf) = doc.descendants().find(|n| n.has_tag_name("rootfile")) {
                        if let Some(fp) = rf.attribute("full-path") {
                            return Ok(fp.to_string());
                        }
                    }
                }
            }
        }

        for i in 0..zip.len() {
            if let Ok(entry) = zip.by_index(i) {
                let name = entry.name().map(|n| n.to_string()).unwrap_or_default();
                if name.to_lowercase().ends_with(".opf") {
                    return Ok(name);
                }
            }
        }

        Err(VellumError::NotFound("Fichier OPF introuvable dans l'EPUB".into()))
    }


}

pub(super) fn normalize_zip_path(base_dir: &str, rel_path: &str) -> String {
    let clean_rel = rel_path.trim_start_matches("./").replace('\\', "/");
    if base_dir.is_empty() {
        clean_rel
    } else {
        let joined = Path::new(base_dir).join(&clean_rel);
        joined.to_string_lossy().replace('\\', "/")
    }
}

pub(super) fn strip_html_tags(s: &str) -> String {
    let tag_re = Regex::new(r#"(?is)<[^>]*>"#).unwrap();
    let unescaped = tag_re.replace_all(s, " ");
    unescaped
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

pub(super) fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    use vellum_domain::EpubIssueSeverity;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    fn create_test_epub(with_toc: bool, has_issues: bool) -> NamedTempFile {
        let file = NamedTempFile::new().unwrap();
        let mut zip = ZipWriter::new(file.reopen().unwrap());

        let stored = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        let deflated = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        // 1. mimetype
        zip.start_file("mimetype", stored).unwrap();
        zip.write_all(b"application/epub+zip").unwrap();

        // 2. container.xml
        zip.start_file("META-INF/container.xml", deflated).unwrap();
        zip.write_all(
            br#"<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#,
        )
        .unwrap();

        // 3. content.opf
        zip.start_file("OEBPS/content.opf", deflated).unwrap();
        let opf_toc_attr = if with_toc { r#"toc="ncx""# } else { "" };
        let opf_ncx_item = if with_toc {
            r#"<item id="ncx" href="toc.ncx" media-type="application/x-dtbncx+xml"/>"#
        } else {
            ""
        };

        let opf = format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="2.0" unique-identifier="BookId">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Test Book</dc:title>
    <dc:creator>Test Author</dc:creator>
    <meta name="cover" content="cover-image"/>
  </metadata>
  <manifest>
    <item id="cover-image" href="cover.jpg" media-type="image/jpeg"/>
    <item id="ch1" href="ch1.xhtml" media-type="application/xhtml+xml"/>
    <item id="ch2" href="ch2.xhtml" media-type="application/xhtml+xml"/>
    {opf_ncx_item}
  </manifest>
  <spine {opf_toc_attr}>
    <itemref idref="ch1"/>
    <itemref idref="ch2"/>
  </spine>
</package>"#
        );
        zip.write_all(opf.as_bytes()).unwrap();

        // 4. Content files
        zip.start_file("OEBPS/cover.jpg", deflated).unwrap();
        zip.write_all(b"fake image bytes").unwrap();

        zip.start_file("OEBPS/ch1.xhtml", deflated).unwrap();
        zip.write_all(
            r#"<!DOCTYPE html><html><head><title>Chapitre 1</title></head><body><h1>Chapitre 1 : Le Réveil</h1><p>Texte</p></body></html>"#.as_bytes(),
        )
        .unwrap();

        zip.start_file("OEBPS/ch2.xhtml", deflated).unwrap();
        zip.write_all(
            r#"<!DOCTYPE html><html><head><title>Chapitre 2</title></head><body><h2 id="sec2">Chapitre 2 : L'Aventure</h2><p>Suite</p></body></html>"#.as_bytes(),
        )
        .unwrap();

        // Optional TOC
        if with_toc {
            zip.start_file("OEBPS/toc.ncx", deflated).unwrap();
            zip.write_all(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<ncx xmlns="http://www.daisy.org/z3986/2005/ncx/" version="2005-1">
  <docTitle><text>Test Book</text></docTitle>
  <navMap>
    <navPoint id="np-1" playOrder="1">
      <navLabel><text>Chapitre 1 : Le Réveil</text></navLabel>
      <content src="ch1.xhtml"/>
    </navPoint>
    <navPoint id="np-2" playOrder="2">
      <navLabel><text>Chapitre 2 : L'Aventure</text></navLabel>
      <content src="ch2.xhtml#sec2"/>
    </navPoint>
  </navMap>
</ncx>"#.as_bytes(),
            )
            .unwrap();
        }

        // Optional Issues (e.g. orphan junk file, DRM file)
        if has_issues {
            zip.start_file("OEBPS/.DS_Store", deflated).unwrap();
            zip.write_all(b"junk").unwrap();
            zip.start_file("META-INF/encryption.xml", deflated).unwrap();
            zip.write_all(b"<encryption></encryption>").unwrap();
        }

        zip.finish().unwrap();
        file
    }

    #[test]
    fn test_inspect_clean_epub() {
        let file = create_test_epub(true, false);
        let diag = EpubEditor::inspect(file.path()).expect("Inspect failed");

        assert!(diag.is_valid);
        assert_eq!(diag.epub_version, "2.0");
        assert_eq!(diag.manifest_count, 4);
        assert_eq!(diag.spine_count, 2);
        assert_eq!(diag.toc_entries_count, 2);
        assert!(diag.has_cover);
        assert!(!diag.issues.iter().any(|i| i.severity == EpubIssueSeverity::Error));
    }

    #[test]
    fn test_extract_and_save_toc() {
        let file = create_test_epub(true, false);
        let toc = EpubEditor::extract_toc(file.path()).expect("Extract TOC failed");

        assert_eq!(toc.len(), 2);
        assert_eq!(toc[0].label, "Chapitre 1 : Le Réveil");
        assert_eq!(toc[1].label, "Chapitre 2 : L'Aventure");
        assert_eq!(toc[1].href, "OEBPS/ch2.xhtml#sec2");

        // Modify label and save
        let mut modified_toc = toc;
        modified_toc[0].label = "Chapitre 1 : Modifié".to_string();

        EpubEditor::save_toc(file.path(), &modified_toc).expect("Save TOC failed");

        // Re-extract and verify
        let re_extracted = EpubEditor::extract_toc(file.path()).expect("Re-extract failed");
        assert_eq!(re_extracted[0].label, "Chapitre 1 : Modifié");
    }

    #[test]
    fn test_generate_toc_from_headings() {
        let file = create_test_epub(false, false);
        let generated = EpubEditor::generate_toc_from_headings(file.path()).unwrap();

        assert_eq!(generated.len(), 2);
        assert_eq!(generated[0].label, "Chapitre 1 : Le Réveil");
        assert_eq!(generated[1].label, "Chapitre 2 : L'Aventure");
    }

    #[test]
    fn test_clean_and_repair() {
        let file = create_test_epub(false, true);
        let diag_before = EpubEditor::inspect(file.path()).unwrap();
        assert!(diag_before.issues.iter().any(|i| i.code == "DRM_ENCRYPTION_DETECTED"));
        assert!(diag_before.issues.iter().any(|i| i.code == "ORPHAN_ZIP_FILE"));

        let diag_after = EpubEditor::clean_and_repair(file.path()).unwrap();
        assert!(!diag_after.issues.iter().any(|i| i.code == "DRM_ENCRYPTION_DETECTED"));
        assert!(!diag_after.issues.iter().any(|i| i.code == "ORPHAN_ZIP_FILE"));
        assert!(diag_after.toc_entries_count >= 2);
    }
}
