#![allow(clippy::unwrap_used)]

mod common;

use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

use common::create_test_services;
use vellum_core::reader_provider::{
    create_content_provider, BookContentProvider, TextContentProvider,
};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

#[tokio::test]
async fn test_reader_provider_txt_and_cbz() {
    let temp = TempDir::new().expect("tempdir");

    // 1. TXT Reader test
    let txt_path = temp.path().join("story.txt");
    std::fs::write(
        &txt_path,
        "Chapter 1\nThe sun rose over the rolling hills.\n\nChapter 2\nThe traveller continued north.",
    )
    .unwrap();

    let txt_provider = TextContentProvider::new(&txt_path);
    assert_eq!(txt_provider.total_pages(), 2);
    let page1_html = txt_provider.get_page_html(0, "dark", 18, false);
    assert!(page1_html.contains("rolling hills"));

    // 2. CBZ Reader test
    let cbz_path = temp.path().join("comic.cbz");
    {
        let f = File::create(&cbz_path).expect("create cbz");
        let mut zip = ZipWriter::new(f);
        let opts = SimpleFileOptions::default();

        zip.start_file("01.jpg", opts).unwrap();
        zip.write_all(b"fake-image-1").unwrap();

        zip.start_file("02.png", opts).unwrap();
        zip.write_all(b"fake-image-2").unwrap();

        zip.finish().unwrap();
    }

    let cbz_provider = create_content_provider(&cbz_path);
    assert_eq!(cbz_provider.total_pages(), 2);
    assert!(cbz_provider.is_fixed_layout());
    let page1_cbz = cbz_provider.get_page_html(0, "dark", 18, false);
    assert!(page1_cbz.contains("data:image/jpeg;base64,"));

    // 2b. Manga RTL and Western LTR Double-Page Spread
    let double_ltr = cbz_provider.get_double_page_html(0, "dark", false);
    assert!(double_ltr.contains("cbz-double-page ltr"));
    assert!(double_ltr.contains("Page 1"));
    assert!(double_ltr.contains("Page 2"));

    let double_rtl = cbz_provider.get_double_page_html(0, "dark", true);
    assert!(double_rtl.contains("cbz-double-page rtl"));
    assert!(double_rtl.contains("Page 1"));
    assert!(double_rtl.contains("Page 2"));

    // 2c. Webtoon Continuous Vertical Strip
    let webtoon_html = cbz_provider.get_webtoon_html();
    assert!(webtoon_html.contains("webtoon-strip-container"));
    assert!(webtoon_html.contains("webtoon-page-1"));
    assert!(webtoon_html.contains("webtoon-page-2"));

    // 3. EPUB Reader with spine and embedded images
    let epub_path = temp.path().join("book_with_images.epub");
    {
        let f = File::create(&epub_path).expect("create epub");
        let mut zip = ZipWriter::new(f);
        let opts = SimpleFileOptions::default();

        zip.start_file("META-INF/container.xml", opts).unwrap();
        zip.write_all(
            br#"<?xml version="1.0"?>
            <container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
              <rootfiles>
                <rootfile full-path="OPS/content.opf" media-type="application/oebps-package+xml"/>
              </rootfiles>
            </container>"#,
        )
        .unwrap();

        zip.start_file("OPS/content.opf", opts).unwrap();
        zip.write_all(
            br#"<?xml version="1.0"?>
            <package xmlns="http://www.idpf.org/2007/opf" version="2.0">
              <manifest>
                <item id="c1" href="chap1.xhtml" media-type="application/xhtml+xml"/>
                <item id="c2" href="chap2.xhtml" media-type="application/xhtml+xml"/>
                <item id="img1" href="images/drawing.png" media-type="image/png"/>
              </manifest>
              <spine>
                <itemref idref="c1"/>
                <itemref idref="c2"/>
              </spine>
            </package>"#,
        )
        .unwrap();

        zip.start_file("OPS/images/drawing.png", opts).unwrap();
        zip.write_all(b"\x89PNG\r\n\x1a\nfake-png-data").unwrap();

        zip.start_file("OPS/chap1.xhtml", opts).unwrap();
        zip.write_all(
            br#"<html xmlns="http://www.w3.org/1999/xhtml">
            <head><title>Premier Pas</title></head>
            <body>
              <h1>Premier Pas</h1>
              <p>Voici une illustration :</p>
              <img src="images/drawing.png" alt="Dessin"/>
            </body>
            </html>"#,
        )
        .unwrap();

        zip.start_file("OPS/chap2.xhtml", opts).unwrap();
        zip.write_all(
            r#"<html xmlns="http://www.w3.org/1999/xhtml">
            <head><title>Deuxième Chapitre</title></head>
            <body>
              <h2>Deuxième Chapitre</h2>
              <p>Fin du récit.</p>
            </body>
            </html>"#
                .as_bytes(),
        )
        .unwrap();

        zip.finish().unwrap();
    }

    let epub_provider = create_content_provider(&epub_path);
    assert_eq!(epub_provider.total_pages(), 2);
    assert_eq!(
        epub_provider.get_section_title(0),
        Some("Premier Pas".to_string())
    );
    assert_eq!(
        epub_provider.get_section_title(1),
        Some("Deuxième Chapitre".to_string())
    );

    let page1_html = epub_provider.get_page_html(0, "dark", 18, false);
    assert!(page1_html.contains("data:image/png;base64,"));
    assert!(page1_html.contains("Premier Pas"));
    // Ensure clean sanitize without embedded full html document
    assert!(!page1_html.contains("<!DOCTYPE"));
    assert!(!page1_html.contains("<head"));
}

#[tokio::test]
async fn test_pivot_format_transitive_conversion() {
    let temp = TempDir::new().expect("tempdir");
    let book_service = create_test_services(&temp);
    let conversion_svc = book_service.conversion_service();

    // 1. Verify reachable formats through pivot
    let mobi_targets = conversion_svc.available_target_formats("mobi");
    println!("Mobi available targets: {:?}", mobi_targets);
    assert!(mobi_targets.contains(&"EPUB".to_string()));
    assert!(mobi_targets.contains(&"PDF".to_string()));
    assert!(mobi_targets.contains(&"TXT".to_string()));

    let text_targets = conversion_svc.available_target_formats("txt");
    println!("Text available targets: {:?}", text_targets);
    assert!(text_targets.contains(&"EPUB".to_string()));
    assert!(text_targets.contains(&"PDF".to_string()));

    // 2. Perform end-to-end 2-step conversion via pivot: TEXT -> (EPUB) -> PDF
    let txt_path = temp.path().join("input.txt");
    std::fs::write(
        &txt_path,
        "Chapitre 1\nCeci est un test de conversion transitive via le format pivot EPUB.",
    )
    .unwrap();

    let pdf_out = temp.path().join("output.pdf");
    let result = conversion_svc.convert_file("text", "pdf", &txt_path, &pdf_out, None);
    if let Err(ref e) = result {
        println!("Error during convert_file: {:?}", e);
    }
    assert!(
        result.is_ok(),
        "Conversion TEXT -> PDF via pivot EPUB should succeed"
    );
    assert!(pdf_out.exists(), "Output PDF must exist");

    let pdf_bytes = std::fs::read(&pdf_out).unwrap();
    assert!(
        pdf_bytes.starts_with(b"%PDF"),
        "Output file must be a valid PDF"
    );

    // 3. Test convert_book_to_format on BookService
    let incoming = temp.path().join("mon_livre.txt");
    std::fs::write(&incoming, "Titre du livre\nContenu captivant.").unwrap();
    let book = book_service.import_book(&incoming).unwrap();
    assert_eq!(book.formats.len(), 1);

    let updated_book = book_service
        .convert_book_to_format(&book.id, "pdf")
        .unwrap();
    assert_eq!(updated_book.formats.len(), 2);
    assert!(updated_book
        .formats
        .iter()
        .any(|f| f.format.eq_ignore_ascii_case("pdf")));
}

#[tokio::test]
async fn test_ereader_pagination_and_navigation() {
    let temp = TempDir::new().expect("tempdir");

    // Create an EPUB with a long chapter (e.g. 3500 chars) that should be split into ~3 pages
    let epub_path = temp.path().join("long_book.epub");
    {
        let f = File::create(&epub_path).expect("create epub");
        let mut zip = ZipWriter::new(f);
        let opts = SimpleFileOptions::default();

        zip.start_file("META-INF/container.xml", opts).unwrap();
        zip.write_all(
            br#"<?xml version="1.0"?>
            <container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
              <rootfiles>
                <rootfile full-path="content.opf" media-type="application/oebps-package+xml"/>
              </rootfiles>
            </container>"#,
        )
        .unwrap();

        zip.start_file("content.opf", opts).unwrap();
        zip.write_all(
            br#"<?xml version="1.0"?>
            <package xmlns="http://www.idpf.org/2007/opf" version="2.0">
              <manifest>
                <item id="c1" href="chapter1.xhtml" media-type="application/xhtml+xml"/>
                <item id="c2" href="chapter2.xhtml" media-type="application/xhtml+xml"/>
              </manifest>
              <spine>
                <itemref idref="c1"/>
                <itemref idref="c2"/>
              </spine>
            </package>"#,
        )
        .unwrap();

        // Chapter 1: 5 paragraphs of 700 chars each = ~3500 chars -> should split into 3 pages
        let p_text =
            "L'espace d'un instant, le ciel s'illumina d'un éclat bleuté et froid. ".repeat(10);
        let ch1_content = format!(
            r#"<html><head><title>L'Aube Sombre</title></head><body>
            <h1>L'Aube Sombre</h1>
            <p>{}</p>
            <p>{}</p>
            <p>{}</p>
            <p>{}</p>
            </body></html>"#,
            p_text, p_text, p_text, p_text
        );
        zip.start_file("chapter1.xhtml", opts).unwrap();
        zip.write_all(ch1_content.as_bytes()).unwrap();

        // Chapter 2: Short chapter (1 page)
        let ch2_content = r#"<html><head><title>Epilogue</title></head><body>
        <h2>Epilogue</h2>
        <p>Le calme revint sur la plaine silencieuse.</p>
        </body></html>"#;
        zip.start_file("chapter2.xhtml", opts).unwrap();
        zip.write_all(ch2_content.as_bytes()).unwrap();

        zip.finish().unwrap();
    }

    let provider = create_content_provider(&epub_path);
    // Chapter 1 has ~3 pages, Chapter 2 has 1 page -> total 4 pages
    assert!(provider.total_pages() >= 3);

    // Page 0 info
    let info0 = provider.get_page_info(0).expect("page 0 info");
    assert_eq!(info0.chapter_title, "L'Aube Sombre");
    assert_eq!(info0.page_in_chapter, 1);
    assert!(info0.total_in_chapter >= 2);
    assert_eq!(info0.global_page, 1);
    assert_eq!(info0.total_pages, provider.total_pages());

    // TOC check: TOC must contain the 2 chapters pointing to their start pages
    let toc = provider.get_table_of_contents();
    assert_eq!(toc.len(), 2);
    assert_eq!(toc[0].0, 0);
    assert_eq!(toc[0].1, "L'Aube Sombre");
    assert!(toc[1].0 > 0); // Chapter 2 starts at page index > 0
    assert_eq!(toc[1].1, "Epilogue");
}

#[tokio::test]
async fn test_epub_nested_div_pagination_and_text_integrity() {
    let temp = TempDir::new().expect("tempdir");
    let epub_path = temp.path().join("nested_div_book.epub");
    {
        let f = File::create(&epub_path).expect("create epub");
        let mut zip = ZipWriter::new(f);
        let opts = SimpleFileOptions::default();

        zip.start_file("META-INF/container.xml", opts).unwrap();
        zip.write_all(
            br#"<?xml version="1.0"?>
            <container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
              <rootfiles>
                <rootfile full-path="content.opf" media-type="application/oebps-package+xml"/>
              </rootfiles>
            </container>"#,
        )
        .unwrap();

        zip.start_file("content.opf", opts).unwrap();
        zip.write_all(
            br#"<?xml version="1.0"?>
            <package xmlns="http://www.idpf.org/2007/opf" version="2.0">
              <manifest>
                <item id="c1" href="chap.xhtml" media-type="application/xhtml+xml"/>
              </manifest>
              <spine>
                <itemref idref="c1"/>
              </spine>
            </package>"#,
        )
        .unwrap();

        // An epub chapter wrapped in nested divs like real world epubs
        let chap_html = r#"<?xml version="1.0" encoding="utf-8"?>
        <html xmlns="http://www.w3.org/1999/xhtml">
          <body>
            <div class="apple-interchange-newline">
              <div class="section-wrapper">
                <h1>Chapitre 1</h1>
                <p>Premier paragraphe du récit.</p>
                <p>Deuxième paragraphe avec des détails fascinants.</p>
                <p>Troisième paragraphe qui continue l'histoire de nos héros.</p>
                <p>Quatrième paragraphe vers la fin de cette section.</p>
                <p>Cinquième et dernier paragraphe du chapitre.</p>
              </div>
            </div>
          </body>
        </html>"#;

        zip.start_file("chap.xhtml", opts).unwrap();
        zip.write_all(chap_html.as_bytes()).unwrap();
        zip.finish().unwrap();
    }

    let provider = create_content_provider(&epub_path);
    assert!(provider.total_pages() >= 1);

    // Verify all paragraphs from first to last are preserved across the pages
    let mut all_pages_content = String::new();
    for p in 0..provider.total_pages() {
        all_pages_content.push_str(&provider.get_page_html(p, "dark", 18, false));
    }

    assert!(all_pages_content.contains("Chapitre 1"));
    assert!(all_pages_content.contains("Premier paragraphe du récit."));
    assert!(all_pages_content.contains("Deuxième paragraphe avec des détails fascinants."));
    assert!(all_pages_content.contains("Troisième paragraphe"));
    assert!(all_pages_content.contains("Quatrième paragraphe"));
    assert!(all_pages_content.contains("Cinquième et dernier paragraphe du chapitre."));
}

#[tokio::test]
async fn test_epub_inspector_and_toc_editor() {
    let temp = TempDir::new().expect("tempdir");
    let book_service = create_test_services(&temp);

    // Create an EPUB with a heading and a missing TOC
    let incoming = temp.path().join("incoming");
    std::fs::create_dir_all(&incoming).unwrap();
    let epub_path = incoming.join("Dune.epub");

    {
        let f = File::create(&epub_path).unwrap();
        let mut zip = ZipWriter::new(f);
        let stored =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        let deflated =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        zip.start_file("mimetype", stored).unwrap();
        zip.write_all(b"application/epub+zip").unwrap();

        zip.start_file("META-INF/container.xml", deflated).unwrap();
        zip.write_all(
            br#"<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#,
        )
        .unwrap();

        zip.start_file("content.opf", deflated).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="utf-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="2.0">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Dune</dc:title>
    <dc:creator>Frank Herbert</dc:creator>
  </metadata>
  <manifest>
    <item id="c1" href="ch1.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine>
    <itemref idref="c1"/>
  </spine>
</package>"#,
        )
        .unwrap();

        zip.start_file("ch1.xhtml", deflated).unwrap();
        zip.write_all(
            br#"<!DOCTYPE html><html><body><h1>Chapitre Premier : Arrakis</h1><p>Dune</p></body></html>"#,
        )
        .unwrap();

        // Add a junk file to test linting & cleaning
        zip.start_file(".DS_Store", deflated).unwrap();
        zip.write_all(b"junk").unwrap();

        zip.finish().unwrap();
    }

    let book = book_service.import_book(&epub_path).expect("import book");

    // 1. Inspect EPUB
    let diag = book_service
        .inspect_book_epub(&book.id)
        .expect("inspect epub");
    assert_eq!(diag.epub_version, "2.0");
    assert_eq!(diag.spine_count, 1);
    assert_eq!(diag.toc_entries_count, 0); // Missing TOC detected
    assert!(diag.issues.iter().any(|i| i.code == "MISSING_TOC"));
    assert!(diag.issues.iter().any(|i| i.code == "ORPHAN_ZIP_FILE"));

    // 2. Generate TOC from headings
    let generated_toc = book_service
        .generate_book_toc_from_headings(&book.id)
        .expect("generate toc");
    assert_eq!(generated_toc.len(), 1);
    assert_eq!(generated_toc[0].label, "Chapitre Premier : Arrakis");

    // 3. Save TOC
    book_service
        .update_book_toc(&book.id, &generated_toc)
        .expect("update toc");

    // 4. Retrieve TOC
    let retrieved_toc = book_service.get_book_toc(&book.id).expect("get toc");
    assert_eq!(retrieved_toc.len(), 1);
    assert_eq!(retrieved_toc[0].label, "Chapitre Premier : Arrakis");

    // 5. Clean & repair
    let diag_after = book_service
        .repair_book_epub(&book.id)
        .expect("repair epub");
    assert!(!diag_after
        .issues
        .iter()
        .any(|i| i.code == "ORPHAN_ZIP_FILE"));
    assert!(diag_after.toc_entries_count >= 1);
    assert!(diag_after.is_valid);
}

#[test]
fn test_polish_book_metadata_cover_and_toc() {
    use std::io::{Read, Write};

    let tmp = tempfile::tempdir().unwrap();
    let epub_path = tmp.path().join("raw.epub");
    let cover_path = tmp.path().join("cover.jpg");
    std::fs::write(&cover_path, b"\xFF\xD8\xFF\xE0FAKEJPEG").unwrap();

    {
        let f = std::fs::File::create(&epub_path).unwrap();
        let mut zip = zip::ZipWriter::new(f);
        let stored = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        let deflated = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        zip.start_file("mimetype", stored).unwrap();
        zip.write_all(b"application/epub+zip").unwrap();
        zip.start_file("META-INF/container.xml", deflated).unwrap();
        zip.write_all(
            br#"<?xml version="1.0"?><container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container"><rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles></container>"#,
        ).unwrap();
        zip.start_file("OEBPS/content.opf", deflated).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="utf-8"?><package xmlns="http://www.idpf.org/2007/opf" version="2.0"><metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>Old Title</dc:title><dc:creator>Old Author</dc:creator></metadata><manifest><item id="c1" href="ch1.xhtml" media-type="application/xhtml+xml"/></manifest><spine><itemref idref="c1"/></spine></package>"#,
        ).unwrap();
        zip.start_file("OEBPS/ch1.xhtml", deflated).unwrap();
        zip.write_all(
            br#"<html><body><h1>Chapitre Un</h1><p>Attendez... Vraiment -- oui !</p></body></html>"#,
        )
        .unwrap();
        zip.finish().unwrap();
    }

    let mut book = vellum_domain::Book::new("Titre Poli", "Auteur Poli");
    book.publisher = Some("Éditions Vellum".to_string());

    let report = vellum_core::EpubEditor::polish(
        &epub_path,
        &book,
        Some(&cover_path),
        vellum_core::PolishOptions::default(),
    )
    .unwrap();

    assert!(report.metadata_updated);
    assert!(report.cover_embedded);
    assert_eq!(report.chapters_smartened, 1);
    assert!(report.toc_rebuilt);
    assert!(report.repaired);

    let mut archive = zip::ZipArchive::new(std::fs::File::open(&epub_path).unwrap()).unwrap();
    let mut opf = String::new();
    archive
        .by_name("OEBPS/content.opf")
        .unwrap()
        .read_to_string(&mut opf)
        .unwrap();
    assert!(opf.contains("<dc:title>Titre Poli</dc:title>"));
    assert!(opf.contains("<dc:creator>Auteur Poli</dc:creator>"));
    assert!(opf.contains("<dc:publisher>Éditions Vellum</dc:publisher>"));
    assert!(opf.contains("vellum-cover-img"));

    let mut ch1 = String::new();
    archive
        .by_name("OEBPS/ch1.xhtml")
        .unwrap()
        .read_to_string(&mut ch1)
        .unwrap();
    assert!(ch1.contains("Attendez… Vraiment – oui !"));
    assert!(archive.by_name("OEBPS/images/vellum-cover.jpg").is_ok());

    let toc = vellum_core::EpubEditor::extract_toc(&epub_path).unwrap();
    assert_eq!(toc.len(), 1);
    assert_eq!(toc[0].label, "Chapitre Un");
}
