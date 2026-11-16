#![allow(clippy::unwrap_used)]

use std::fs::File;
use std::io::Write;
use tempfile::TempDir;
use vellum_domain::{Book, DeviceKind, DeviceSyncPlugin};
use vellum_plugins::ereader::{convert_epub_to_kepub, KindleSyncPlugin, KoboSyncPlugin};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

#[test]
fn test_kepubify_injection() {
    let temp = TempDir::new().unwrap();
    let epub_in = temp.path().join("source.epub");
    let kepub_out = temp.path().join("target.kepub.epub");

    // Create minimal valid epub
    {
        let file = File::create(&epub_in).unwrap();
        let mut zip = ZipWriter::new(file);
        let opts = SimpleFileOptions::default();

        zip.start_file("mimetype", opts).unwrap();
        zip.write_all(b"application/epub+zip").unwrap();

        zip.start_file("OEBPS/chapter1.xhtml", opts).unwrap();
        zip.write_all(
            r#"<?xml version="1.0" encoding="utf-8"?>
            <html xmlns="http://www.w3.org/1999/xhtml">
              <body>
                <p>Premier paragraphe du chapitre 1.</p>
                <p>Deuxième paragraphe avec du texte intéressant.</p>
              </body>
            </html>"#
                .as_bytes(),
        )
        .unwrap();

        zip.finish().unwrap();
    }

    // Convert to kepub
    convert_epub_to_kepub(&epub_in, &kepub_out).expect("conversion to kepub");
    assert!(kepub_out.exists());

    // Inspect kepub output
    let file = File::open(&kepub_out).unwrap();
    let mut zip = zip::ZipArchive::new(file).unwrap();
    let mut ch1 = zip.by_name("OEBPS/chapter1.xhtml").unwrap();
    let mut content = String::new();
    std::io::Read::read_to_string(&mut ch1, &mut content).unwrap();

    assert!(content.contains(r#"<span class="koboSpan" id="kobo.1.1">"#));
    assert!(content.contains(r#"<span class="koboSpan" id="kobo.1.2">"#));
    assert!(content.contains("Premier paragraphe du chapitre 1."));
}

#[test]
fn test_kobo_sync_plugin_lifecycle() {
    let temp = TempDir::new().unwrap();
    let kobo_mount = temp.path().join("KOBOeReader");
    let dot_kobo = kobo_mount.join(".kobo");
    std::fs::create_dir_all(&dot_kobo).unwrap();

    // 1. Setup mock Kobo SQLite database
    let db_path = dot_kobo.join("KoboReader.sqlite");
    {
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute(
            "CREATE TABLE content (
                ContentID TEXT PRIMARY KEY,
                ContentType INTEGER,
                Title TEXT,
                Attribution TEXT,
                ___PercentRead REAL,
                ReadStatus INTEGER,
                DateLastRead TEXT,
                MimeType TEXT
            )",
            [],
        )
        .unwrap();

        conn.execute(
            "CREATE TABLE Bookmark (
                VolumeID TEXT,
                Text TEXT,
                Annotation TEXT,
                ChapterProgress REAL,
                DateCreated TEXT
            )",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO content (ContentID, ContentType, Title, Attribution, ___PercentRead, ReadStatus, DateLastRead)
             VALUES ('Books/Frank Herbert/Dune.epub', 6, 'Dune', 'Frank Herbert', 45.5, 1, '2026-10-18T10:00:00Z')",
            [],
        ).unwrap();

        conn.execute(
            "INSERT INTO Bookmark (VolumeID, Text, Annotation, ChapterProgress, DateCreated)
             VALUES ('Books/Frank Herbert/Dune.epub', 'Je ne connaîtrai pas la peur.', 'Citation culte', 0.12, '2026-10-18T11:00:00Z')",
            [],
        ).unwrap();
    }

    let plugin = KoboSyncPlugin::new();
    assert_eq!(plugin.device_kind(), DeviceKind::Kobo);

    // 2. Detection
    let detected = plugin.detect_device(Some(&kobo_mount));
    assert_eq!(detected.as_deref(), Some(kobo_mount.as_path()));

    // 3. List books
    let books = plugin.list_books(&kobo_mount).unwrap();
    assert_eq!(books.len(), 1);
    assert_eq!(books[0].title, "Dune");
    assert_eq!(books[0].author, "Frank Herbert");
    assert_eq!(books[0].progress_percent, Some(45.5));

    // 4. Export a book
    let src_file = temp.path().join("Neuromancer.txt");
    std::fs::write(
        &src_file,
        "Le ciel au-dessus du port était couleur de télé calée sur un canal vide.",
    )
    .unwrap();
    let book = Book::new("Neuromancer", "William Gibson");
    let exported = plugin
        .export_book(
            &book,
            &src_file,
            &kobo_mount,
            &std::collections::HashMap::new(),
        )
        .unwrap();

    assert!(exported.exists());
    assert!(exported.to_string_lossy().contains("William Gibson"));

    // 5. Reading progress
    let progress = plugin.get_reading_progress(&kobo_mount).unwrap();
    assert_eq!(progress.len(), 1);
    assert_eq!(progress[0].book_title, "Dune");
    assert_eq!(progress[0].progress_percent, 45.5);

    // 6. Annotations
    let ann = plugin.get_annotations(&kobo_mount).unwrap();
    assert_eq!(ann.len(), 1);
    assert!(ann[0].text.contains("Je ne connaîtrai pas la peur"));
    assert_eq!(ann[0].note.as_deref(), Some("Citation culte"));
}

#[test]
fn test_kindle_sync_plugin_lifecycle() {
    let temp = TempDir::new().unwrap();
    let kindle_mount = temp.path().join("Kindle");
    let docs_dir = kindle_mount.join("documents");
    let sys_dir = kindle_mount.join("system");
    std::fs::create_dir_all(&docs_dir).unwrap();
    std::fs::create_dir_all(&sys_dir).unwrap();

    // 1. Put a book and clippings file
    let mobi_file = docs_dir.join("Hyperion.mobi");
    std::fs::write(&mobi_file, b"MOBI mock binary data").unwrap();

    let clippings = docs_dir.join("My Clippings.txt");
    std::fs::write(
        &clippings,
        r#"
Hyperion (Dan Simmons)
- Votre surlignement sur la page 15 | emplacement 210-212 | Ajouté le mardi 10 mars 2026 14:00:00

Le criquet du temps chantait dans les herbes mortes.
==========
"#,
    )
    .unwrap();

    let plugin = KindleSyncPlugin::new();
    assert_eq!(plugin.device_kind(), DeviceKind::Kindle);

    // 2. Detection
    let detected = plugin.detect_device(Some(&kindle_mount));
    assert_eq!(detected.as_deref(), Some(kindle_mount.as_path()));

    // 3. List books
    let books = plugin.list_books(&kindle_mount).unwrap();
    assert_eq!(books.len(), 1);
    assert_eq!(books[0].filename, "Hyperion.mobi");

    // 4. Export book
    let src_file = temp.path().join("Solaris.azw3");
    std::fs::write(&src_file, b"AZW3 mock data").unwrap();
    let book = Book::new("Solaris", "Stanislaw Lem");
    let exported = plugin
        .export_book(
            &book,
            &src_file,
            &kindle_mount,
            &std::collections::HashMap::new(),
        )
        .unwrap();
    assert!(exported.exists());
    assert!(exported.to_string_lossy().contains("Stanislaw Lem"));

    // 5. Annotations
    let ann = plugin.get_annotations(&kindle_mount).unwrap();
    assert_eq!(ann.len(), 1);
    assert_eq!(ann[0].book_title, "Hyperion");
    assert!(ann[0].text.contains("Le criquet du temps chantait"));
}
