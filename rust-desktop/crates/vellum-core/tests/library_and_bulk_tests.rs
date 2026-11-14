#![allow(clippy::unwrap_used, unused_imports)]

mod common;

use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

use common::create_test_services;
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
    let book = book_service.import_book(&epub_path).expect("import book");
    assert_eq!(book.title, "Foundation");
    assert_eq!(book.author, "Isaac Asimov");
    assert_eq!(book.formats.len(), 1);
    assert_eq!(book.tags, vec![Tag::new("Science Fiction")]);

    // 2. Add an additional format (e.g. PDF)
    let pdf_path = incoming_dir.join("Foundation.pdf");
    std::fs::write(
        &pdf_path,
        b"%PDF-1.4\n1 0 obj\n<<>>\nendobj\ntrailer\n<<>>\n%%EOF",
    )
    .unwrap();
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
    book_service
        .delete_book(&book.id, true)
        .expect("delete book");
    assert!(book_service
        .get_book_by_id(&book.id)
        .expect("find")
        .is_none());
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
    assert_eq!(
        updated.description.as_deref(),
        Some("Un synopsis incroyable.")
    );
    assert_eq!(updated.tags.len(), 2);
    assert!(updated.tags.iter().any(|t| t.name == "Space Opera"));
    assert!(updated.tags.iter().any(|t| t.name == "Chef-d'œuvre"));

    // Verify persistence
    let fetched = book_service
        .book_repo()
        .find_by_id(&book.id)
        .unwrap()
        .unwrap();
    assert_eq!(fetched.title, "Le Nouveau Titre");
    assert_eq!(fetched.author, "Auteur Célèbre");

    // 2. Update cover
    let cover_img = temp.path().join("new_cover.jpg");
    std::fs::write(&cover_img, b"fake_jpeg_header_content").unwrap();

    let with_cover = book_service
        .update_book_cover(&book.id, &cover_img)
        .unwrap();
    assert!(with_cover.cover_path.is_some());
    let cov_rel = with_cover.cover_path.unwrap();
    assert!(cov_rel.ends_with(".jpg"));

    let full_cov = book_service.library_path().join(&cov_rel);
    assert!(full_cov.exists());
    assert_eq!(
        std::fs::read(&full_cov).unwrap(),
        b"fake_jpeg_header_content"
    );

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
        .bulk_update_books(&book_ids, None, None, false, &[], &["Classic".to_string()])
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
async fn test_calibre_library_import() {
    use rusqlite::Connection;
    use vellum_core::CalibreImporter;

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
    let _ = book_service
        .tag_repo()
        .save(&vellum_domain::Tag::new("Classique"));
    let b1 = book_service.book_repo().save(&b1).unwrap();
    let b1 = book_service
        .add_format_to_book(&b1.id, &dummy_epub)
        .unwrap();

    // Create Book 2: Duplicate with slight title variation and author inversion, in PDF format
    let mut b2 =
        vellum_domain::Book::new("Les Misérables - Tome 1 (Édition Collector)", "Hugo Victor");
    b2.progress_percent = 60.0;
    b2.add_tag(vellum_domain::Tag::new("Roman"));
    let _ = book_service
        .tag_repo()
        .save(&vellum_domain::Tag::new("Roman"));
    let b2 = book_service.book_repo().save(&b2).unwrap();
    let b2 = book_service.add_format_to_book(&b2.id, &dummy_pdf).unwrap();

    // 1. Audit: Should detect 1 duplicate pair with high similarity
    let audit1 = book_service.audit_library().expect("audit library");
    assert_eq!(audit1.total_books, 2);
    assert_eq!(audit1.duplicate_pairs.len(), 1);
    assert!(audit1.duplicate_pairs[0].similarity_score >= 0.85);
    assert_eq!(audit1.total_formats, 2);

    // 2. Merge Book 2 into Book 1
    let merged = book_service
        .merge_books(&b1.id, &b2.id)
        .expect("merge books");

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
    assert!(merged
        .description
        .as_ref()
        .unwrap()
        .contains("roman social"));

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

#[test]
fn test_watch_folder_auto_add_and_deduplication() {
    let temp = TempDir::new().expect("tempdir");
    let book_service = create_test_services(&temp);

    let watch_dir = temp.path().join("watch");
    std::fs::create_dir_all(&watch_dir).unwrap();
    let sample_epub = watch_dir.join("WatchTest.epub");

    let write_sample_epub = |dest: &std::path::Path| {
        let f = File::create(dest).unwrap();
        let mut zip = ZipWriter::new(f);
        let stored =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        let deflated =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        zip.start_file("mimetype", stored).unwrap();
        zip.write_all(b"application/epub+zip").unwrap();
        zip.start_file("META-INF/container.xml", deflated).unwrap();
        zip.write_all(
            br#"<?xml version="1.0"?><container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container"><rootfiles><rootfile full-path="content.opf" media-type="application/oebps-package+xml"/></rootfiles></container>"#,
        ).unwrap();
        zip.start_file("content.opf", deflated).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="utf-8"?><package xmlns="http://www.idpf.org/2007/opf" version="2.0"><metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>Zzxq Watch Book</dc:title><dc:creator>Zzxq Author</dc:creator></metadata><manifest><item id="c1" href="ch1.xhtml" media-type="application/xhtml+xml"/></manifest><spine><itemref idref="c1"/></spine></package>"#,
        ).unwrap();
        zip.start_file("ch1.xhtml", deflated).unwrap();
        zip.write_all(b"<html><body><p>Hello</p></body></html>")
            .unwrap();
        zip.finish().unwrap();
    };

    write_sample_epub(&sample_epub);
    // Ignored non-ebook file
    std::fs::write(watch_dir.join("readme.png"), b"not-an-ebook").unwrap();

    // 1. First scan imports WatchTest.epub and moves it to processed/
    let r1 = vellum_core::WatchFolderService::scan_once(&book_service, &watch_dir, true).unwrap();
    assert_eq!(r1.scanned_files, 1);
    assert_eq!(r1.imported_book_ids.len(), 1);
    assert_eq!(r1.skipped_duplicates, 0);
    assert!(!sample_epub.exists());
    assert!(watch_dir.join("processed").join("WatchTest.epub").exists());

    // 2. Re-drop the same book -> skipped as duplicate
    write_sample_epub(&sample_epub);
    let r2 = vellum_core::WatchFolderService::scan_once(&book_service, &watch_dir, true).unwrap();
    assert_eq!(r2.scanned_files, 1);
    assert_eq!(r2.imported_book_ids.len(), 0);
    assert_eq!(r2.skipped_duplicates, 1);
}

#[test]
fn test_virtual_library_crud_and_query_filtering() {
    let db = vellum_storage::Database::open_in_memory().unwrap();
    let repo = vellum_storage::VirtualLibraryRepository::new(db);
    let service = vellum_core::VirtualLibraryService::new(repo);

    // Validation rejects empty name or query
    assert!(service.create("  ", "tag:sf", None).is_err());
    assert!(service.create("SF", "   ", None).is_err());

    let vl = service
        .create(
            "SF à lire",
            "tag:sf -status:finished",
            Some("#6366f1".into()),
        )
        .unwrap();
    assert_eq!(vl.name, "SF à lire");
    assert_eq!(service.list().unwrap().len(), 1);

    let mut b1 = vellum_domain::Book::new("Dune", "Frank Herbert");
    b1.add_tag(vellum_domain::Tag::new("SF"));
    b1.is_finished = false;

    let mut b2 = vellum_domain::Book::new("Fondation", "Isaac Asimov");
    b2.add_tag(vellum_domain::Tag::new("SF"));
    b2.is_finished = true;

    let mut b3 = vellum_domain::Book::new("Le Petit Prince", "Antoine de Saint-Exupéry");
    b3.add_tag(vellum_domain::Tag::new("Classique"));

    let filtered = service.filter_books(&[b1, b2, b3], &vl.id).unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].title, "Dune");

    let updated = service
        .update(&vl.id, "SF complète", "tag:sf", Some("#10b981".into()))
        .unwrap();
    assert_eq!(updated.name, "SF complète");
    assert_eq!(service.get(&vl.id).unwrap().unwrap().query, "tag:sf");

    assert!(service.delete(&vl.id).unwrap());
    assert!(service.list().unwrap().is_empty());
}

#[test]
fn test_custom_columns_definition_validation_and_values() {
    let db = vellum_storage::Database::open_in_memory().unwrap();
    let book_repo = vellum_storage::BookRepository::new(db.clone());
    let col_repo = vellum_storage::CustomColumnRepository::new(db);
    let service = vellum_core::CustomColumnService::new(col_repo);

    let book = vellum_domain::Book::new("Hypérion", "Dan Simmons");
    book_repo.save(&book).unwrap();

    // Define columns of different types
    let mood = service
        .define_column(
            "Reading Mood",
            "Humeur",
            vellum_domain::CustomColumnType::Text,
        )
        .unwrap();
    assert_eq!(mood.key, "reading_mood");

    let pages = service
        .define_column(
            "pages",
            "Nombre de pages",
            vellum_domain::CustomColumnType::Number,
        )
        .unwrap();
    let owned = service
        .define_column(
            "owned",
            "Possédé papier",
            vellum_domain::CustomColumnType::Boolean,
        )
        .unwrap();
    let note = service
        .define_column(
            "personal_rating",
            "Note perso",
            vellum_domain::CustomColumnType::Rating,
        )
        .unwrap();

    // Duplicate key rejected
    assert!(service
        .define_column("pages", "Doublon", vellum_domain::CustomColumnType::Number)
        .is_err());

    // Type validation errors
    assert!(service
        .set_book_value(&book.id, "pages", "not-a-number")
        .is_err());
    assert!(service.set_book_value(&book.id, "owned", "maybe").is_err());
    assert!(service
        .set_book_value(&book.id, "personal_rating", "9")
        .is_err());

    // Valid values
    service
        .set_book_value(&book.id, "reading_mood", "Épique")
        .unwrap();
    service.set_book_value(&book.id, "pages", "512").unwrap();
    service.set_book_value(&book.id, "owned", "oui").unwrap();
    service
        .set_book_value(&book.id, "personal_rating", "5")
        .unwrap();

    let values = service.get_book_values(&book.id).unwrap();
    assert_eq!(
        values.get("reading_mood").map(String::as_str),
        Some("Épique")
    );
    assert_eq!(values.get("pages").map(String::as_str), Some("512"));
    assert_eq!(values.get("owned").map(String::as_str), Some("true"));
    assert_eq!(values.get("personal_rating").map(String::as_str), Some("5"));

    // Clear a value with empty string
    service
        .set_book_value(&book.id, "reading_mood", "   ")
        .unwrap();
    assert!(!service
        .get_book_values(&book.id)
        .unwrap()
        .contains_key("reading_mood"));

    // Delete column definition cascades to values
    assert!(service.delete_column(&pages.id).unwrap());
    assert!(service.delete_column(&owned.id).unwrap());
    assert!(service.delete_column(&note.id).unwrap());
    assert_eq!(service.list_columns().unwrap().len(), 1);
}

#[test]
fn test_notes_export_markdown_and_bulk_regex_and_import_rules() {
    let temp = TempDir::new().expect("tempdir");
    let book_service = create_test_services(&temp);

    // 1. NotesExporter Markdown
    let mut book = vellum_domain::Book::new("Dune", "Herbert, Frank");
    book.add_tag(vellum_domain::Tag::new("SF"));
    let saved = book_service.book_repo().save(&book).unwrap();

    let mut user_data = vellum_core::BookUserData::default();
    user_data.highlights.push(vellum_core::Highlight::new(
        &saved.id,
        0,
        "Chapitre 1",
        "yellow",
        "La peur tue l'esprit.",
        Some("Citation clé".to_string()),
    ));
    user_data.bookmarks.push(vellum_core::Bookmark::new(
        &saved.id,
        0,
        "Chapitre 1",
        2,
        12.5,
        "Arrakis...",
    ));

    let md_path = temp.path().join("notes.md");
    vellum_core::NotesExporter::export_to_file(&saved, &user_data, &md_path).unwrap();
    let md = std::fs::read_to_string(&md_path).unwrap();
    assert!(md.contains("title: \"Dune\""));
    assert!(md.contains("La peur tue l'esprit."));
    assert!(md.contains("Citation clé"));
    assert!(md.contains("## Signets"));

    // 2. Bulk Regex Replace (dry_run then apply: "Herbert, Frank" -> "Frank Herbert")
    let preview = book_service
        .bulk_regex_replace(vellum_core::BulkRegexRequest {
            book_ids: vec![saved.id.clone()],
            field: vellum_core::RegexTargetField::Author,
            pattern: r"^([^,]+),\s*(.+)$".to_string(),
            replacement: "$2 $1".to_string(),
            dry_run: true,
        })
        .unwrap();
    assert_eq!(preview.matched_count, 1);
    assert_eq!(preview.updated_count, 0);
    assert_eq!(preview.changes[0].after, "Frank Herbert");

    let applied = book_service
        .bulk_regex_replace(vellum_core::BulkRegexRequest {
            book_ids: vec![saved.id.clone()],
            field: vellum_core::RegexTargetField::Author,
            pattern: r"^([^,]+),\s*(.+)$".to_string(),
            replacement: "$2 $1".to_string(),
            dry_run: false,
        })
        .unwrap();
    assert_eq!(applied.updated_count, 1);
    assert_eq!(
        book_service
            .get_book_by_id(&saved.id)
            .unwrap()
            .unwrap()
            .author,
        "Frank Herbert"
    );

    // 3. Import Rules persisted & evaluated
    let mut rule = vellum_core::ImportRule::new(
        "Auto SF",
        vellum_core::ImportRuleTarget::Filename,
        r"(?i)foundation",
    );
    rule.add_tags = vec!["Classique SF".to_string()];
    rule.set_series = Some("Cycle de Fondation".to_string());
    book_service.save_import_rules(&[rule]).unwrap();
    assert_eq!(book_service.get_import_rules().len(), 1);

    let mut b2 = vellum_domain::Book::new("Fondation", "Isaac Asimov");
    let outcome = vellum_core::apply_import_rules(
        &mut b2,
        "Asimov - Foundation (1951).epub",
        &book_service.get_import_rules(),
    );
    assert_eq!(outcome.matched_rule_names, vec!["Auto SF".to_string()]);
    assert!(b2.tags.iter().any(|t| t.name == "Classique SF"));
    assert_eq!(b2.series.as_deref(), Some("Cycle de Fondation"));
}

#[test]
fn test_full_text_search_inside_books() {
    let temp = TempDir::new().expect("tempdir");
    let book_service = create_test_services(&temp);

    let txt_path = temp.path().join("Chronique.txt");
    std::fs::write(
        &txt_path,
        "Chapitre 1\n\nL'épice gérontologique prolonge la vie sur Arrakis.\n\nChapitre 2\n\nLes vers des sables gardent le désert.",
    )
    .unwrap();

    let imported = book_service.import_book(&txt_path).unwrap();

    let indexed_chapters = book_service.index_book_content(&imported.id).unwrap();
    assert!(indexed_chapters >= 1);

    // Accent-insensitive FTS5 search inside book text ("epice" matches "épice")
    let hits = book_service.search_inside_books("epice", 10).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].book_id, imported.id);
    assert!(hits[0].snippet.contains("épice"));
}
