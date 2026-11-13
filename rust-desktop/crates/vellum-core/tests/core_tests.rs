use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

use vellum_core::book_service::BookService;
use vellum_core::conversion_service::ConversionService;
use vellum_core::litestream_service::{LitestreamService, LitestreamStatus};
use vellum_core::metadata_service::MetadataService;
use vellum_core::plugin_service::PluginService;
use vellum_core::reader_provider::{
    create_content_provider, BookContentProvider, TextContentProvider,
};
use vellum_domain::{LitestreamConfig, Tag};
use vellum_storage::{BookRepository, Database, ParamRepository, TagRepository};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

fn create_test_services(temp: &TempDir) -> BookService {
    let db = Database::open_in_memory().expect("in memory db");
    let book_repo = BookRepository::new(db.clone());
    let param_repo = ParamRepository::new(db.clone());
    let tag_repo = TagRepository::new(db.clone());

    let ext_dir = temp.path().join("plugins");
    let plugin_service = PluginService::new(param_repo.clone(), Some(ext_dir));
    let metadata_service = MetadataService::new(plugin_service.clone());
    let conversion_service = ConversionService::new(plugin_service.clone());
    let library_path = temp.path().join("library");

    BookService::new(
        book_repo,
        param_repo,
        tag_repo,
        metadata_service,
        plugin_service,
        conversion_service,
        library_path,
    )
}

#[tokio::test]
async fn test_book_service_import_and_lifecycle() {
    let temp = TempDir::new().expect("tempdir");
    let book_service = create_test_services(&temp);

    // Create a mock epub file
    let incoming_dir = temp.path().join("incoming");
    std::fs::create_dir_all(&incoming_dir).unwrap();
    let epub_path = incoming_dir.join("Isaac Asimov - Foundation.epub");

    {
        let f = File::create(&epub_path).expect("create file");
        let mut zip = ZipWriter::new(f);
        let opts = SimpleFileOptions::default();

        zip.start_file("META-INF/container.xml", opts).unwrap();
        zip.write_all(
            br#"<?xml version="1.0"?>
            <container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
              <rootfiles>
                <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
              </rootfiles>
            </container>"#,
        )
        .unwrap();

        zip.start_file("OEBPS/content.opf", opts).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="utf-8"?>
            <package xmlns="http://www.idpf.org/2007/opf" version="2.0">
              <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
                <dc:title>Foundation</dc:title>
                <dc:creator>Isaac Asimov</dc:creator>
                <dc:subject>Science Fiction</dc:subject>
              </metadata>
            </package>"#,
        )
        .unwrap();
        zip.finish().unwrap();
    }

    // 1. Import book
    let book = book_service
        .import_book(&epub_path)
        .expect("import book");
    assert_eq!(book.title, "Foundation");
    assert_eq!(book.author, "Isaac Asimov");
    assert_eq!(book.formats.len(), 1);
    assert_eq!(book.tags, vec![Tag::new("Science Fiction")]);

    // 2. Add an additional format (e.g. PDF)
    let pdf_path = incoming_dir.join("Foundation.pdf");
    std::fs::write(&pdf_path, b"%PDF-1.4\n1 0 obj\n<<>>\nendobj\ntrailer\n<<>>\n%%EOF").unwrap();
    let updated_book = book_service
        .add_format_to_book(&book.id, &pdf_path)
        .expect("attach format");
    assert_eq!(updated_book.formats.len(), 2);

    // 3. Update reading progress
    let progressed = book_service
        .update_reading_progress(&book.id, 5, 20, 25.0, false)
        .expect("update progress");
    assert_eq!(progressed.current_chapter_index, 5);
    assert_eq!(progressed.total_chapters, 20);
    assert!(!progressed.is_finished);

    // 4. Check library stats
    let stats = book_service.get_reading_stats().expect("stats");
    assert_eq!(stats.total_books, 1);
    assert_eq!(stats.in_progress_books, 1);
    assert_eq!(stats.finished_books, 0);

    // 5. Remove PDF format
    let book_after_remove = book_service
        .remove_format_from_book(&book.id, "pdf", true)
        .expect("remove format");
    assert_eq!(book_after_remove.formats.len(), 1);
    assert_eq!(book_after_remove.formats[0].format, "epub");

    // 6. Delete book
    book_service.delete_book(&book.id, true).expect("delete book");
    assert!(book_service.get_book_by_id(&book.id).expect("find").is_none());
}

#[test]
fn test_litestream_configuration_and_yaml() {
    let temp = TempDir::new().expect("tempdir");
    let db_path = temp.path().join("vellum.db");
    std::fs::write(&db_path, b"dummy").unwrap();

    let cfg = LitestreamConfig {
        enabled: true,
        replica_type: "file".to_string(),
        replica_path: temp.path().join("backups"),
        binary_path: "litestream".to_string(),
    };

    let service = LitestreamService::new(cfg, db_path.clone());
    let cfg_file = service.generate_config_file().expect("generate config");
    assert!(cfg_file.exists());
    let yaml = std::fs::read_to_string(&cfg_file).expect("read yaml");
    assert!(yaml.contains("dbs:"));

    let status = service.status();
    assert_ne!(status, LitestreamStatus::Running);
}

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
            </html>"#.as_bytes(),
        )
        .unwrap();

        zip.finish().unwrap();
    }

    let epub_provider = create_content_provider(&epub_path);
    assert_eq!(epub_provider.total_pages(), 2);
    assert_eq!(epub_provider.get_section_title(0), Some("Premier Pas".to_string()));
    assert_eq!(epub_provider.get_section_title(1), Some("Deuxième Chapitre".to_string()));

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
    std::fs::write(&txt_path, "Chapitre 1\nCeci est un test de conversion transitive via le format pivot EPUB.").unwrap();

    let pdf_out = temp.path().join("output.pdf");
    let result = conversion_svc.convert_file("text", "pdf", &txt_path, &pdf_out, None);
    if let Err(ref e) = result {
        println!("Error during convert_file: {:?}", e);
    }
    assert!(result.is_ok(), "Conversion TEXT -> PDF via pivot EPUB should succeed");
    assert!(pdf_out.exists(), "Output PDF must exist");

    let pdf_bytes = std::fs::read(&pdf_out).unwrap();
    assert!(pdf_bytes.starts_with(b"%PDF"), "Output file must be a valid PDF");

    // 3. Test convert_book_to_format on BookService
    let incoming = temp.path().join("mon_livre.txt");
    std::fs::write(&incoming, "Titre du livre\nContenu captivant.").unwrap();
    let book = book_service.import_book(&incoming).unwrap();
    assert_eq!(book.formats.len(), 1);

    let updated_book = book_service.convert_book_to_format(&book.id, "pdf").unwrap();
    assert_eq!(updated_book.formats.len(), 2);
    assert!(updated_book.formats.iter().any(|f| f.format.eq_ignore_ascii_case("pdf")));
}

#[tokio::test]
async fn test_update_book_metadata_and_cover() {
    let temp = TempDir::new().expect("tempdir");
    let book_service = create_test_services(&temp);

    let txt_path = temp.path().join("story.txt");
    std::fs::write(&txt_path, "Une histoire fantastique.").unwrap();
    let book = book_service.import_book(&txt_path).unwrap();

    // 1. Update metadata
    let updated = book_service
        .update_book_metadata(
            &book.id,
            "Le Nouveau Titre",
            "Auteur Célèbre",
            Some("Éditions Galaxie"),
            Some("Un synopsis incroyable."),
            vec!["Space Opera".to_string(), "Chef-d'œuvre".to_string()],
        )
        .unwrap();

    assert_eq!(updated.title, "Le Nouveau Titre");
    assert_eq!(updated.author, "Auteur Célèbre");
    assert_eq!(updated.publisher.as_deref(), Some("Éditions Galaxie"));
    assert_eq!(updated.description.as_deref(), Some("Un synopsis incroyable."));
    assert_eq!(updated.tags.len(), 2);
    assert!(updated.tags.iter().any(|t| t.name == "Space Opera"));
    assert!(updated.tags.iter().any(|t| t.name == "Chef-d'œuvre"));

    // Verify persistence
    let fetched = book_service.book_repo().find_by_id(&book.id).unwrap().unwrap();
    assert_eq!(fetched.title, "Le Nouveau Titre");
    assert_eq!(fetched.author, "Auteur Célèbre");

    // 2. Update cover
    let cover_img = temp.path().join("new_cover.jpg");
    std::fs::write(&cover_img, b"fake_jpeg_header_content").unwrap();

    let with_cover = book_service.update_book_cover(&book.id, &cover_img).unwrap();
    assert!(with_cover.cover_path.is_some());
    let cov_rel = with_cover.cover_path.unwrap();
    assert!(cov_rel.ends_with(".jpg"));

    let full_cov = book_service.library_path().join(&cov_rel);
    assert!(full_cov.exists());
    assert_eq!(std::fs::read(&full_cov).unwrap(), b"fake_jpeg_header_content");

    // 3. Update metadata with ISBN, Series & Publish date
    let with_isbn = book_service
        .update_book_metadata_full(vellum_core::UpdateMetadataRequest {
            book_id: &book.id,
            title: "Le Nouveau Titre",
            author: "Auteur Célèbre",
            publisher: Some("Éditions Galaxie"),
            description: Some("Un synopsis incroyable."),
            isbn: Some("9780441013593"),
            publish_date: Some("1965"),
            series: Some("Cycle de Dune"),
            series_index: Some(1.0),
            tag_names: vec!["Space Opera".to_string()],
        })
        .unwrap();
    assert_eq!(with_isbn.isbn.as_deref(), Some("9780441013593"));
    assert_eq!(with_isbn.publish_date.as_deref(), Some("1965"));
    assert_eq!(with_isbn.series.as_deref(), Some("Cycle de Dune"));
    assert_eq!(with_isbn.series_index, Some(1.0));

    // 4. Enrich book
    let enriched = book_service.enrich_book(&book.id).unwrap();
    assert_eq!(enriched.id, book.id);
    assert!(enriched.isbn.is_some());
}

#[tokio::test]
async fn test_device_service_kobo_and_kindle_integration() {
    let temp = TempDir::new().expect("tempdir");
    let book_service = std::sync::Arc::new(create_test_services(&temp));
    let device_service = vellum_core::device_service::DeviceService::new(
        book_service.clone(),
        book_service.plugin_service().clone(),
    );

    // 1. Create a book in Vellum
    let txt_path = temp.path().join("Solaris.txt");
    std::fs::write(&txt_path, "Chapitre 1\nSolaris et l'océan pensant.").unwrap();
    let book = book_service.import_book(&txt_path).unwrap();

    // 2. Mock Kobo Device
    let kobo_mount = temp.path().join("KOBOeReader");
    let dot_kobo = kobo_mount.join(".kobo");
    std::fs::create_dir_all(&dot_kobo).unwrap();
    std::fs::write(dot_kobo.join("version"), b"4.38.21908").unwrap();

    // Export Solaris to Kobo (via pivot EPUB/KEPUB)
    let exported_kobo = device_service
        .export_book_to_device(&book.id, vellum_domain::DeviceKind::Kobo, Some(&kobo_mount))
        .unwrap();
    assert!(exported_kobo.exists());
    assert!(exported_kobo.to_string_lossy().contains("KOBOeReader"));

    // 3. Mock Kindle Device
    let kindle_mount = temp.path().join("Kindle");
    let docs_dir = kindle_mount.join("documents");
    let sys_dir = kindle_mount.join("system");
    std::fs::create_dir_all(&docs_dir).unwrap();
    std::fs::create_dir_all(&sys_dir).unwrap();
    std::fs::write(sys_dir.join("version"), b"Kindle 5.16").unwrap();


    // Put a book on Kindle to test importing into Vellum
    let incoming_kindle_book = docs_dir.join("Neuromancer.txt");
    std::fs::write(&incoming_kindle_book, "Neuromancer par William Gibson.").unwrap();

    let imported = device_service
        .import_books_from_device(vellum_domain::DeviceKind::Kindle, Some(&kindle_mount))
        .unwrap();
    assert_eq!(imported.len(), 1);
    assert_eq!(imported[0].title, "Neuromancer");

    // Verify Neuromancer is now in Vellum library
    let in_db = book_service.book_repo().find_by_id(&imported[0].id).unwrap();
    assert!(in_db.is_some());
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
        let p_text = "L'espace d'un instant, le ciel s'illumina d'un éclat bleuté et froid. ".repeat(10);
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
async fn test_two_clients_sync_via_google_drive_folder() {
    let temp_root = TempDir::new().expect("tempdir");
    let gdrive_folder = temp_root.path().join("GoogleDrive").join("Vellum");

    // Client A setup
    let client_a_dir = temp_root.path().join("client_a");
    let client_a_lib = client_a_dir.join("library");
    let client_a_db = vellum_storage::Database::open(client_a_dir.join("vellum.db")).unwrap();
    let client_a_books = vellum_storage::BookRepository::new(client_a_db.clone());
    let client_a_tags = vellum_storage::TagRepository::new(client_a_db.clone());
    let client_a_sync_repo = vellum_storage::SyncRepository::new(client_a_db.clone());

    let mut cfg_a = vellum_domain::CloudSyncConfig::default();
    cfg_a.client_id = "client-macbook-a".to_string();
    cfg_a.sync_folder_path = Some(gdrive_folder.clone());
    cfg_a.enabled = true;
    cfg_a.auto_download_books = true;

    let sync_service_a = vellum_core::SyncService::new(
        cfg_a,
        client_a_sync_repo,
        client_a_books.clone(),
        client_a_tags.clone(),
        client_a_lib.clone(),
    );

    // Client B setup (completely separate machine / directory)
    let client_b_dir = temp_root.path().join("client_b");
    let client_b_lib = client_b_dir.join("library");
    let client_b_db = vellum_storage::Database::open(client_b_dir.join("vellum.db")).unwrap();
    let client_b_books = vellum_storage::BookRepository::new(client_b_db.clone());
    let client_b_tags = vellum_storage::TagRepository::new(client_b_db.clone());
    let client_b_sync_repo = vellum_storage::SyncRepository::new(client_b_db.clone());

    let mut cfg_b = vellum_domain::CloudSyncConfig::default();
    cfg_b.client_id = "client-imac-b".to_string();
    cfg_b.sync_folder_path = Some(gdrive_folder.clone());
    cfg_b.enabled = true;
    cfg_b.auto_download_books = true;

    let sync_service_b = vellum_core::SyncService::new(
        cfg_b,
        client_b_sync_repo,
        client_b_books.clone(),
        client_b_tags.clone(),
        client_b_lib.clone(),
    );

    // 1. Client A imports a book
    let book_id = "book-fondation-1";
    let mut book = vellum_domain::Book::new("Fondation", "Isaac Asimov");
    book.id = book_id.to_string();
    book.isbn = Some("9782207249117".to_string());
    let fmt_rel = "Isaac Asimov/Fondation (book-fondation-1)/fondation.epub";
    let fmt_abs = client_a_lib.join(fmt_rel);
    std::fs::create_dir_all(fmt_abs.parent().unwrap()).unwrap();
    std::fs::write(&fmt_abs, b"fake epub binary content on client a").unwrap();

    let fmt = vellum_domain::BookFormat::new("epub", "fondation.epub", fmt_rel, 36);
    book.add_format(fmt.clone());
    let tag = vellum_domain::Tag::new("Science-Fiction");
    book.add_tag(tag.clone());

    client_a_books.save(&book).unwrap();
    client_a_tags.save(&tag).unwrap();

    // Client A records BookCreated sync event
    sync_service_a
        .record_event(vellum_domain::SyncAction::BookCreated {
            book: book.clone(),
            formats: vec![fmt],
            tags: vec![tag],
        })
        .unwrap();

    // Client A syncs with Google Drive
    let report_a = sync_service_a.sync_now().unwrap();
    assert_eq!(report_a.uploaded_books, 1);
    assert_eq!(report_a.pushed_events, 1);

    // 2. Client B syncs with Google Drive (initial pull)
    assert_eq!(client_b_books.count().unwrap(), 0);
    let report_b = sync_service_b.sync_now().unwrap();
    assert_eq!(report_b.pulled_events, 1);
    assert_eq!(report_b.downloaded_books, 1);

    // Verify Client B has the book in its local database
    let book_on_b = client_b_books.find_by_id(book_id).unwrap().unwrap();
    assert_eq!(book_on_b.title, "Fondation");
    assert_eq!(book_on_b.author, "Isaac Asimov");
    assert_eq!(book_on_b.isbn.as_deref(), Some("9782207249117"));
    assert_eq!(book_on_b.tags.len(), 1);

    // Verify Client B downloaded the actual file
    let file_on_b = client_b_lib.join(fmt_rel);
    assert!(file_on_b.exists());
    assert_eq!(
        std::fs::read_to_string(&file_on_b).unwrap(),
        "fake epub binary content on client a"
    );

    // 3. Client B reads up to page 50 (progress 33%)
    sync_service_b
        .record_event(vellum_domain::SyncAction::ProgressUpdated {
            book_id: book_id.to_string(),
            progress_percent: 33.0,
            current_chapter_index: 2,
            total_chapters: 10,
            last_read_at: "2026-11-10T12:00:00Z".to_string(),
            is_finished: false,
        })
        .unwrap();

    let report_b_push = sync_service_b.sync_now().unwrap();
    assert_eq!(report_b_push.pushed_events, 1);

    // 4. Client A syncs and receives the updated reading progress from Client B!
    let report_a_pull = sync_service_a.sync_now().unwrap();
    assert_eq!(report_a_pull.pulled_events, 1);

    let book_on_a = client_a_books.find_by_id(book_id).unwrap().unwrap();
    assert_eq!(book_on_a.progress_percent, 33.0);
    assert_eq!(book_on_a.current_chapter_index, 2);
    assert_eq!(
        book_on_a.last_read_at.as_deref(),
        Some("2026-11-10T12:00:00Z")
    );
}

#[tokio::test]
async fn test_bulk_update_and_bulk_delete_books() {
    let temp = TempDir::new().expect("tempdir");
    let book_service = create_test_services(&temp);
    let incoming_dir = temp.path().join("incoming");
    std::fs::create_dir_all(&incoming_dir).unwrap();

    let mut book_ids = Vec::new();
    for i in 1..=3 {
        let path = incoming_dir.join(format!("Book_{i}.txt"));
        std::fs::write(&path, format!("Content of book {i}")).unwrap();
        let book = book_service.import_book(&path).expect("import book");
        book_ids.push(book.id);
    }

    assert_eq!(book_ids.len(), 3);

    // 1. Bulk update: set author, series with auto_index = true, and add tags
    let updated = book_service
        .bulk_update_books(
            &book_ids,
            Some("Frank Herbert"),
            Some("Dune Chronicles"),
            true,
            &["Space Opera".to_string(), "Classic".to_string()],
            &[],
        )
        .expect("bulk update");

    assert_eq!(updated.len(), 3);

    for (idx, b) in updated.iter().enumerate() {
        assert_eq!(b.author, "Frank Herbert");
        assert_eq!(b.series.as_deref(), Some("Dune Chronicles"));
        assert_eq!(b.series_index, Some((idx + 1) as f64));
        assert!(b.tags.iter().any(|t| t.name == "Space Opera"));
        assert!(b.tags.iter().any(|t| t.name == "Classic"));
    }

    // 2. Bulk remove tag
    let updated_no_tag = book_service
        .bulk_update_books(
            &book_ids,
            None,
            None,
            false,
            &[],
            &["Classic".to_string()],
        )
        .expect("bulk update remove tag");

    for b in updated_no_tag {
        assert_eq!(b.author, "Frank Herbert");
        assert!(!b.tags.iter().any(|t| t.name == "Classic"));
        assert!(b.tags.iter().any(|t| t.name == "Space Opera"));
    }

    // 3. Bulk delete
    book_service
        .bulk_delete_books(&book_ids, true)
        .expect("bulk delete");

    for id in &book_ids {
        let found = book_service.book_repo().find_by_id(id).unwrap();
        assert!(found.is_none());
    }
}

#[tokio::test]
async fn test_mail_service_validation() {
    use vellum_core::MailService;
    use vellum_domain::{Book, BookFormat, SmtpConfig};

    let temp = TempDir::new().expect("tempdir");
    let mail_service = MailService::new(temp.path().to_path_buf());

    let mut cfg = SmtpConfig::default();
    cfg.enabled = false;

    let book = Book::new("Dune", "Frank Herbert");
    let res = mail_service.send_to_kindle(&book, &cfg, None).await;
    assert!(res.is_err(), "Must fail when SMTP is disabled");

    cfg.enabled = true;
    cfg.from_address = "me@example.com".into();
    cfg.kindle_email = "mykindle@kindle.com".into();

    // Fails because book has no formats
    let res_no_format = mail_service.send_to_kindle(&book, &cfg, None).await;
    assert!(res_no_format.is_err());

    // Add format with existing file
    let epub_path = temp.path().join("dune.epub");
    std::fs::write(&epub_path, b"fake epub content").unwrap();

    let mut book_with_fmt = book.clone();
    book_with_fmt.formats.push(BookFormat::new(
        "epub",
        "dune.epub",
        epub_path.to_str().unwrap(),
        17,
    ));

    // Testing test_connection on localhost dummy port fails gracefully
    let mut bad_server_cfg = cfg.clone();
    bad_server_cfg.host = "127.0.0.1".into();
    bad_server_cfg.port = 65432;
    bad_server_cfg.username = "test".into();
    bad_server_cfg.password = "test".into();
    let conn_res = mail_service.test_connection(&bad_server_cfg).await;
    assert!(conn_res.is_err());
}

#[tokio::test]
async fn test_calibre_library_import() {
    use vellum_core::CalibreImporter;
    use rusqlite::Connection;

    let temp_calibre = TempDir::new().expect("temp calibre dir");
    let calibre_root = temp_calibre.path();
    let db_path = calibre_root.join("metadata.db");

    // 1. Create a Calibre SQLite database schema and sample data
    let conn = Connection::open(&db_path).unwrap();
    conn.execute_batch(
        "CREATE TABLE books (
            id INTEGER PRIMARY KEY,
            title TEXT NOT NULL,
            sort TEXT,
            timestamp TEXT,
            pubdate TEXT,
            series_index REAL DEFAULT 1.0,
            author_sort TEXT,
            isbn TEXT,
            path TEXT,
            has_cover INTEGER DEFAULT 0
        );
        CREATE TABLE authors (id INTEGER PRIMARY KEY, name TEXT NOT NULL, sort TEXT);
        CREATE TABLE books_authors_link (id INTEGER PRIMARY KEY, book INTEGER, author INTEGER);
        CREATE TABLE series (id INTEGER PRIMARY KEY, name TEXT NOT NULL, sort TEXT);
        CREATE TABLE books_series_link (id INTEGER PRIMARY KEY, book INTEGER, series INTEGER);
        CREATE TABLE tags (id INTEGER PRIMARY KEY, name TEXT NOT NULL);
        CREATE TABLE books_tags_link (id INTEGER PRIMARY KEY, book INTEGER, tag INTEGER);
        CREATE TABLE comments (id INTEGER PRIMARY KEY, book INTEGER, text TEXT);
        CREATE TABLE publishers (id INTEGER PRIMARY KEY, name TEXT NOT NULL);
        CREATE TABLE books_publishers_link (id INTEGER PRIMARY KEY, book INTEGER, publisher INTEGER);
        CREATE TABLE data (id INTEGER PRIMARY KEY, book INTEGER, format TEXT, uncompressed_size INTEGER, name TEXT);
        
        -- Insert author
        INSERT INTO authors (id, name) VALUES (1, 'Isaac Asimov');
        -- Insert series
        INSERT INTO series (id, name) VALUES (1, 'Foundation Saga');
        -- Insert tag
        INSERT INTO tags (id, name) VALUES (1, 'Space Opera');
        -- Insert publisher
        INSERT INTO publishers (id, name) VALUES (1, 'Gnome Press');
        
        -- Insert book 1
        INSERT INTO books (id, title, series_index, isbn, path, has_cover) 
        VALUES (1, 'Foundation', 1.0, '9780553293357', 'Isaac Asimov/Foundation (1)', 1);
        INSERT INTO books_authors_link (book, author) VALUES (1, 1);
        INSERT INTO books_series_link (book, series) VALUES (1, 1);
        INSERT INTO books_tags_link (book, tag) VALUES (1, 1);
        INSERT INTO books_publishers_link (book, publisher) VALUES (1, 1);
        INSERT INTO comments (book, text) VALUES (1, '<p>The first book in the Foundation series.</p>');
        INSERT INTO data (book, format, uncompressed_size, name) VALUES (1, 'EPUB', 256000, 'Foundation - Isaac Asimov');
        ",
    )
    .unwrap();

    // 2. Create the file on disk matching Calibre's layout
    let book_dir = calibre_root.join("Isaac Asimov").join("Foundation (1)");
    std::fs::create_dir_all(&book_dir).unwrap();
    let epub_file = book_dir.join("Foundation - Isaac Asimov.epub");
    std::fs::write(&epub_file, b"dummy epub content").unwrap();
    let cover_file = book_dir.join("cover.jpg");
    std::fs::write(&cover_file, b"dummy cover jpeg").unwrap();

    // 3. Test detection
    assert!(CalibreImporter::is_calibre_library(calibre_root));
    assert!(CalibreImporter::is_calibre_library(&db_path));

    // 4. Run import into Vellum
    let temp_vellum = TempDir::new().expect("temp vellum dir");
    let book_service = create_test_services(&temp_vellum);

    let report = CalibreImporter::import_calibre_library(calibre_root, true, &book_service)
        .expect("import calibre library");

    assert_eq!(report.books_imported, 1);
    assert_eq!(report.formats_imported, 1);
    assert_eq!(report.series_count, 1);
    assert_eq!(report.tags_count, 1);
    assert_eq!(report.errors.len(), 0);

    // 5. Verify book in Vellum repository
    let all_books = book_service.book_repo().find_all().unwrap();
    assert_eq!(all_books.len(), 1);
    let imported_book = &all_books[0];
    assert_eq!(imported_book.title, "Foundation");
    assert_eq!(imported_book.author, "Isaac Asimov");
    assert_eq!(imported_book.series.as_deref(), Some("Foundation Saga"));
    assert_eq!(imported_book.series_index, Some(1.0));
    assert_eq!(imported_book.publisher.as_deref(), Some("Gnome Press"));
    assert_eq!(
        imported_book.description.as_deref(),
        Some("The first book in the Foundation series.")
    );
    assert!(imported_book.cover_path.is_some());
    assert_eq!(imported_book.formats.len(), 1);
    assert_eq!(imported_book.formats[0].format, "epub");
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
        let stored = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        let deflated = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

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
    let diag = book_service.inspect_book_epub(&book.id).expect("inspect epub");
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
    let diag_after = book_service.repair_book_epub(&book.id).expect("repair epub");
    assert!(!diag_after.issues.iter().any(|i| i.code == "ORPHAN_ZIP_FILE"));
    assert!(diag_after.toc_entries_count >= 1);
    assert!(diag_after.is_valid);
}

#[tokio::test]
async fn test_web_companion_server_and_api() {
    let temp = TempDir::new().expect("tempdir");
    let book_service = create_test_services(&temp);

    // Create an EPUB book
    let incoming = temp.path().join("incoming");
    std::fs::create_dir_all(&incoming).unwrap();
    let epub_path = incoming.join("Hyperion.epub");

    {
        let f = File::create(&epub_path).unwrap();
        let mut zip = ZipWriter::new(f);
        let stored = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        let deflated = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

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
    <dc:title>Hyperion</dc:title>
    <dc:creator>Dan Simmons</dc:creator>
  </metadata>
  <manifest>
    <item id="c1" href="ch1.xhtml" media-type="application/xhtml+xml"/>
    <item id="c2" href="ch2.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine>
    <itemref idref="c1"/>
    <itemref idref="c2"/>
  </spine>
</package>"#,
        )
        .unwrap();

        zip.start_file("ch1.xhtml", deflated).unwrap();
        zip.write_all(
            br#"<!DOCTYPE html><html><body><h1>Chapitre 1 : Le Consul</h1><p>Sur le vaisseau...</p></body></html>"#,
        )
        .unwrap();

        zip.start_file("ch2.xhtml", deflated).unwrap();
        zip.write_all(
            r#"<!DOCTYPE html><html><body><h2>Chapitre 2 : Le Prêtre</h2><p>Le père Duré...</p></body></html>"#.as_bytes(),
        )
        .unwrap();

        zip.finish().unwrap();
    }

    let book = book_service.import_book(&epub_path).expect("import book");

    // Spawn server on ephemeral port (port 0 binds any free port)
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let app = vellum_core::opds_server::OpdsServer::router(book_service.clone());

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    // 1. Test Web Companion Library Page (HTML)
    let res_html = client.get(&base_url).send().await.unwrap();
    assert_eq!(res_html.status(), 200);
    let body_html = res_html.text().await.unwrap();
    assert!(body_html.contains("Vellum Companion"));

    // 2. Test Web Reader Page (HTML)
    let res_reader = client
        .get(format!("{base_url}/read/{}", book.id))
        .send()
        .await
        .unwrap();
    assert_eq!(res_reader.status(), 200);
    let body_reader = res_reader.text().await.unwrap();
    assert!(body_reader.contains("Lecteur Vellum"));

    // 3. Test API Status
    let res_status = client
        .get(format!("{base_url}/api/status"))
        .send()
        .await
        .unwrap();
    assert_eq!(res_status.status(), 200);
    let json_status: serde_json::Value = res_status.json().await.unwrap();
    assert_eq!(json_status["status"], "ok");
    assert_eq!(json_status["total_books"], 1);

    // 4. Test API Books List
    let res_books = client
        .get(format!("{base_url}/api/books"))
        .send()
        .await
        .unwrap();
    assert_eq!(res_books.status(), 200);
    let json_books: Vec<vellum_core::opds_server::BookDto> = res_books.json().await.unwrap();
    assert_eq!(json_books.len(), 1);
    assert_eq!(json_books[0].title, "Hyperion");
    assert_eq!(json_books[0].author, "Dan Simmons");

    // 5. Test API Book Details
    let res_detail = client
        .get(format!("{base_url}/api/books/{}", book.id))
        .send()
        .await
        .unwrap();
    assert_eq!(res_detail.status(), 200);
    let json_detail: vellum_core::opds_server::BookDto = res_detail.json().await.unwrap();
    assert_eq!(json_detail.id, book.id);
    assert_eq!(json_detail.title, "Hyperion");

    // 6. Test API Book TOC
    let res_toc = client
        .get(format!("{base_url}/api/books/{}/toc", book.id))
        .send()
        .await
        .unwrap();
    assert_eq!(res_toc.status(), 200);
    let json_toc: Vec<vellum_core::opds_server::TocItemDto> = res_toc.json().await.unwrap();
    assert_eq!(json_toc.len(), 2);
    assert_eq!(json_toc[0].title, "Chapitre 1 : Le Consul");
    assert_eq!(json_toc[1].title, "Chapitre 2 : Le Prêtre");

    // 7. Test API Book Chapter
    let res_ch = client
        .get(format!("{base_url}/api/books/{}/chapter/0", book.id))
        .send()
        .await
        .unwrap();
    assert_eq!(res_ch.status(), 200);
    let json_ch: vellum_core::opds_server::ChapterContentDto = res_ch.json().await.unwrap();
    assert_eq!(json_ch.chapter_index, 0);
    assert_eq!(json_ch.total_chapters, 2);
    assert!(json_ch.html.contains("Sur le vaisseau"));

    // 8. Test API Reading Progress Update
    let res_prog = client
        .post(format!("{base_url}/api/books/{}/progress", book.id))
        .json(&serde_json::json!({
            "chapter_index": 1,
            "percentage": 0.5
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res_prog.status(), 200);

    let updated_book = book_service.get_book_by_id(&book.id).unwrap().unwrap();
    assert_eq!(updated_book.current_chapter_index, 1);
    assert_eq!(updated_book.progress_percent, 50.0);

    // 9. Test OPDS Feed XML
    let res_opds = client.get(format!("{base_url}/opds")).send().await.unwrap();
    assert_eq!(res_opds.status(), 200);
    let opds_xml = res_opds.text().await.unwrap();
    assert!(opds_xml.contains("Vellum Desktop Ebook Library"));

    let res_opds_books = client
        .get(format!("{base_url}/opds/books"))
        .send()
        .await
        .unwrap();
    assert_eq!(res_opds_books.status(), 200);
    let opds_books_xml = res_opds_books.text().await.unwrap();
    assert!(opds_books_xml.contains("Hyperion"));
}

#[tokio::test]
async fn test_news_recipe_generation_and_library_import() {
    let temp = TempDir::new().expect("tempdir");
    let book_service = create_test_services(&temp);

    // Mock local RSS HTTP server
    let rss_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>Echo Tech Daily</title>
    <link>http://echotech.local</link>
    <description>Latest tech stories</description>
    <item>
      <title>Antigravity 2.0 Unveiled</title>
      <link>http://echotech.local/anti2</link>
      <author>Jean Dupont</author>
      <pubDate>Thu, 12 Nov 2026 06:00:00 GMT</pubDate>
      <description>&lt;p&gt;Google DeepMind launches next gen agent.&lt;/p&gt;</description>
    </item>
  </channel>
</rss>"#;

    let app = axum::Router::new().route(
        "/feed.xml",
        axum::routing::get(move || async move {
            let mut headers = axum::http::HeaderMap::new();
            headers.insert(
                axum::http::header::CONTENT_TYPE,
                axum::http::HeaderValue::from_static("application/xml"),
            );
            (axum::http::StatusCode::OK, headers, rss_content)
        }),
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let param_repo = vellum_storage::ParamRepository::new(vellum_storage::Database::open_in_memory().unwrap());
    let news_service = vellum_core::NewsService::new(book_service.clone(), param_repo, None);

    let mut recipe = vellum_domain::NewsRecipe::new("custom-tech", "Echo Tech");
    recipe.feeds.push(vellum_domain::NewsFeedConfig::new(
        "echofeed",
        "Echo Tech",
        format!("http://127.0.0.1:{port}/feed.xml"),
    ));

    news_service.save_recipes(&[recipe.clone()]).expect("save recipe");

    // Run recipe and import into library
    let imported_book = news_service
        .run_recipe_and_import("custom-tech")
        .await
        .expect("run recipe");

    assert!(imported_book.title.contains("Echo Tech"));
    assert_eq!(imported_book.author, "Vellum News Engine");
    assert!(imported_book.tags.iter().any(|t| t.name == "Actualités"));
    assert!(imported_book.tags.iter().any(|t| t.name == "Presse"));
    assert_eq!(imported_book.formats.len(), 1);
    assert_eq!(imported_book.formats[0].format, "epub");

    // Verify it is readable via reader provider
    let file_path = book_service.get_file_for_format(&imported_book.formats[0]);
    assert!(file_path.exists());
    let provider = vellum_core::reader_provider::create_content_provider(&file_path);
    assert!(provider.chapter_count() >= 2); // Cover + Article
    assert!(provider.chapter_html(1).contains("Antigravity 2.0 Unveiled"));
}

#[tokio::test]
async fn test_library_health_audit_and_merge_books() {
    let tmp = tempfile::tempdir().unwrap();
    let library_dir = tmp.path().join("library");
    std::fs::create_dir_all(&library_dir).unwrap();

    let db = vellum_storage::Database::open_in_memory().unwrap();
    let book_repo = vellum_storage::BookRepository::new(db.clone());
    let param_repo = vellum_storage::ParamRepository::new(db.clone());
    let tag_repo = vellum_storage::TagRepository::new(db.clone());

    let plugin_service = vellum_core::PluginService::new(param_repo.clone(), None);
    let metadata_service = vellum_core::MetadataService::new(plugin_service.clone());
    let conversion_service = vellum_core::ConversionService::new(plugin_service.clone());

    let book_service = vellum_core::BookService::new(
        book_repo,
        param_repo,
        tag_repo,
        metadata_service,
        plugin_service,
        conversion_service,
        library_dir.clone(),
    );

    // Create a dummy EPUB file on disk
    let dummy_epub = library_dir.join("test.epub");
    std::fs::write(&dummy_epub, b"PK00dummyepubdata").unwrap();

    // Create a dummy PDF file on disk
    let dummy_pdf = library_dir.join("test.pdf");
    std::fs::write(&dummy_pdf, b"%PDF-1.4 dummy pdf data").unwrap();

    // Create Book 1: EPUB with tag "Classique"
    let mut b1 = vellum_domain::Book::new("Les Misérables", "Victor Hugo");
    b1.description = Some("Un immense roman social et historique.".to_string());
    b1.publisher = Some("A. Lacroix, Verboeckhoven & Ce".to_string());
    b1.publish_date = Some("1862".to_string());
    b1.isbn = Some("9782070409228".to_string());
    b1.progress_percent = 25.0;
    b1.add_tag(vellum_domain::Tag::new("Classique"));
    let _ = book_service.tag_repo().save(&vellum_domain::Tag::new("Classique"));
    let b1 = book_service.book_repo().save(&b1).unwrap();
    let b1 = book_service.add_format_to_book(&b1.id, &dummy_epub).unwrap();

    // Create Book 2: Duplicate with slight title variation and author inversion, in PDF format
    let mut b2 = vellum_domain::Book::new("Les Misérables - Tome 1 (Édition Collector)", "Hugo Victor");
    b2.progress_percent = 60.0;
    b2.add_tag(vellum_domain::Tag::new("Roman"));
    let _ = book_service.tag_repo().save(&vellum_domain::Tag::new("Roman"));
    let b2 = book_service.book_repo().save(&b2).unwrap();
    let b2 = book_service.add_format_to_book(&b2.id, &dummy_pdf).unwrap();

    // 1. Audit: Should detect 1 duplicate pair with high similarity
    let audit1 = book_service.audit_library().expect("audit library");
    assert_eq!(audit1.total_books, 2);
    assert_eq!(audit1.duplicate_pairs.len(), 1);
    assert!(audit1.duplicate_pairs[0].similarity_score >= 0.85);
    assert_eq!(audit1.total_formats, 2);

    // 2. Merge Book 2 into Book 1
    let merged = book_service.merge_books(&b1.id, &b2.id).expect("merge books");

    // Verify merged result
    assert_eq!(merged.id, b1.id);
    assert_eq!(merged.title, "Les Misérables");
    assert_eq!(merged.author, "Victor Hugo");
    // Formats: now contains both EPUB and PDF
    assert_eq!(merged.formats.len(), 2);
    assert!(merged.formats.iter().any(|f| f.format == "epub"));
    assert!(merged.formats.iter().any(|f| f.format == "pdf"));
    // Tags: contains both "Classique" and "Roman"
    assert!(merged.tags.iter().any(|t| t.name == "Classique"));
    assert!(merged.tags.iter().any(|t| t.name == "Roman"));
    // Reading progress: took the highest (60%)
    assert_eq!(merged.progress_percent, 60.0);
    // Description preserved
    assert!(merged.description.as_ref().unwrap().contains("roman social"));

    // Verify Book 2 was deleted from repository
    let b2_check = book_service.book_repo().find_by_id(&b2.id).unwrap();
    assert!(b2_check.is_none());

    // 3. Re-audit: Now only 1 book and 0 duplicates!
    let audit2 = book_service.audit_library().expect("second audit");
    assert_eq!(audit2.total_books, 1);
    assert_eq!(audit2.duplicate_pairs.len(), 0);
    assert_eq!(audit2.total_formats, 2);
    assert!(audit2.overall_score >= audit1.overall_score);
}

#[tokio::test]
async fn test_opds_server_auth_and_security_headers() {
    let temp = TempDir::new().expect("tempdir");
    let book_service = create_test_services(&temp);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let app = vellum_core::opds_server::OpdsServer::router_with_token(
        book_service,
        Some("secret-token".to_string()),
    );

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{port}/api/status");

    // 1. Missing token -> 401 Unauthorized
    let unauth = client.get(&url).send().await.unwrap();
    assert_eq!(unauth.status(), 401);

    // 2. Wrong token -> 401 Unauthorized
    let wrong = client
        .get(&url)
        .bearer_auth("wrong-token")
        .send()
        .await
        .unwrap();
    assert_eq!(wrong.status(), 401);

    // 3. Valid Bearer token -> 200 OK + security headers
    let ok = client
        .get(&url)
        .bearer_auth("secret-token")
        .send()
        .await
        .unwrap();
    assert_eq!(ok.status(), 200);
    assert_eq!(
        ok.headers()
            .get("x-content-type-options")
            .and_then(|v| v.to_str().ok()),
        Some("nosniff")
    );
    assert_eq!(
        ok.headers()
            .get("x-frame-options")
            .and_then(|v| v.to_str().ok()),
        Some("SAMEORIGIN")
    );
}












