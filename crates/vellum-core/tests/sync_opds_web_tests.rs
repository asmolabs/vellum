#![allow(clippy::unwrap_used)]

mod common;

use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

use common::create_test_services;
use vellum_core::litestream_service::{LitestreamService, LitestreamStatus};
use vellum_domain::LitestreamConfig;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

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
    let in_db = book_service
        .book_repo()
        .find_by_id(&imported[0].id)
        .unwrap();
    assert!(in_db.is_some());

    // 4. Test compare_device with Xteink / Generic Device
    let xteink_mount = temp.path().join("Xteink");
    let xteink_books = xteink_mount.join("books");
    std::fs::create_dir_all(&xteink_books).unwrap();

    // Book present on Xteink: "Solaris"
    let xteink_solaris = xteink_books.join("Stanislaw Lem - Solaris.epub");
    std::fs::write(&xteink_solaris, b"dummy epub").unwrap();

    // Book present only on Xteink: "Fondation"
    let xteink_fondation = xteink_books.join("Isaac Asimov - Fondation.epub");
    std::fs::write(&xteink_fondation, b"dummy epub").unwrap();

    let comparison = device_service
        .compare_device(vellum_domain::DeviceKind::Generic, Some(&xteink_mount))
        .unwrap();

    assert_eq!(comparison.device.kind, vellum_domain::DeviceKind::Generic);
    // Solaris matches
    assert_eq!(comparison.present_on_device.len(), 1);
    assert_eq!(comparison.present_on_device[0].title, "Solaris");
    // Neuromancer is in Vellum but not on Xteink
    assert!(comparison
        .in_vellum_not_on_device
        .iter()
        .any(|b| b.title == "Neuromancer"));
    // Fondation is only on Xteink
    assert_eq!(comparison.on_device_not_in_vellum.len(), 1);
    assert_eq!(comparison.on_device_not_in_vellum[0].title, "Fondation");

    // 5. Test export_books_batch
    let ids_to_export: Vec<String> = comparison
        .in_vellum_not_on_device
        .iter()
        .map(|b| b.id.clone())
        .collect();
    let exported_count = device_service
        .export_books_batch(
            &ids_to_export,
            vellum_domain::DeviceKind::Generic,
            Some(&xteink_mount),
        )
        .unwrap();
    assert!(exported_count >= 1);
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

    let cfg_a = vellum_domain::CloudSyncConfig {
        client_id: "client-macbook-a".to_string(),
        sync_folder_path: Some(gdrive_folder.clone()),
        enabled: true,
        auto_download_books: true,
        ..Default::default()
    };

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

    let cfg_b = vellum_domain::CloudSyncConfig {
        client_id: "client-imac-b".to_string(),
        sync_folder_path: Some(gdrive_folder.clone()),
        enabled: true,
        auto_download_books: true,
        ..Default::default()
    };

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
async fn test_mail_service_validation() {
    use vellum_core::MailService;
    use vellum_domain::{Book, BookFormat, SmtpConfig};

    let temp = TempDir::new().expect("tempdir");
    let mail_service = MailService::new(temp.path().to_path_buf());

    let mut cfg = SmtpConfig {
        enabled: false,
        ..Default::default()
    };

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

    let param_repo =
        vellum_storage::ParamRepository::new(vellum_storage::Database::open_in_memory().unwrap());
    let news_service = vellum_core::NewsService::new(book_service.clone(), param_repo, None);

    let mut recipe = vellum_domain::NewsRecipe::new("custom-tech", "Echo Tech");
    recipe.feeds.push(vellum_domain::NewsFeedConfig::new(
        "echofeed",
        "Echo Tech",
        format!("http://127.0.0.1:{port}/feed.xml"),
    ));

    news_service
        .save_recipes(&[recipe.clone()])
        .expect("save recipe");

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
    assert!(provider
        .chapter_html(1)
        .contains("Antigravity 2.0 Unveiled"));
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

    // 0. Healthcheck endpoint is accessible without token (for Docker HEALTHCHECK)
    let health = client
        .get(format!("http://127.0.0.1:{port}/api/v1/health"))
        .send()
        .await
        .unwrap();
    assert_eq!(health.status(), 200);
    assert_eq!(
        health
            .headers()
            .get("x-content-type-options")
            .and_then(|v| v.to_str().ok()),
        Some("nosniff")
    );

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

#[tokio::test]
async fn test_i18n_persistence_and_api_endpoint() {
    let temp = TempDir::new().expect("tempdir");
    let book_service = create_test_services(&temp);

    book_service
        .set_ui_locale(vellum_core::Locale::En)
        .expect("persist en locale");
    assert_eq!(book_service.get_ui_locale(), vellum_core::Locale::En);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let app = vellum_core::opds_server::OpdsServer::router(book_service);
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let res = reqwest::get(format!("http://127.0.0.1:{port}/api/i18n"))
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let json: serde_json::Value = res.json().await.unwrap();
    assert_eq!(json["locale"], "en");
    assert_eq!(json["messages"]["nav.library"], "Library");
}

#[tokio::test]
async fn test_api_export_catalog_csv_json_bibtex() {
    let temp = TempDir::new().expect("tempdir");
    let book_service = create_test_services(&temp);

    let book = vellum_domain::Book::new("Les Misérables", "Victor Hugo");
    book_service.book_repo().save(&book).unwrap();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let app = vellum_core::opds_server::OpdsServer::router(book_service);
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let csv_res = reqwest::get(format!("http://127.0.0.1:{port}/api/export?format=csv"))
        .await
        .unwrap();
    assert_eq!(csv_res.status(), 200);
    assert!(csv_res.text().await.unwrap().contains("Les Misérables"));

    let json_res = reqwest::get(format!("http://127.0.0.1:{port}/api/export?format=json"))
        .await
        .unwrap();
    assert_eq!(json_res.status(), 200);
    assert!(json_res.text().await.unwrap().contains("Victor Hugo"));

    let bib_res = reqwest::get(format!("http://127.0.0.1:{port}/api/export?format=bibtex"))
        .await
        .unwrap();
    assert_eq!(bib_res.status(), 200);
    assert!(bib_res.text().await.unwrap().contains("@book{"));

    let notes_res = reqwest::get(format!(
        "http://127.0.0.1:{port}/api/books/{}/notes",
        book.id
    ))
    .await
    .unwrap();
    assert_eq!(notes_res.status(), 200);
    assert!(notes_res.text().await.unwrap().contains("Les Misérables"));

    let sc_res = reqwest::get(format!("http://127.0.0.1:{port}/api/search-content?q=test"))
        .await
        .unwrap();
    assert_eq!(sc_res.status(), 200);
}

#[test]
fn test_web_companion_assets_security_and_dom_invariants() {
    let lib_html = vellum_core::web_companion_assets::library_page_html();
    let reader_html = vellum_core::web_companion_assets::reader_page_html();

    assert!(lib_html.contains("<!DOCTYPE html>"));
    assert!(lib_html.contains("function escapeHtml(str)"));
    assert!(lib_html.contains("escapeHtml(b.title)"));
    assert!(lib_html.contains("/api/books"));
    assert!(lib_html.contains("/api/i18n"));

    assert!(reader_html.contains("<!DOCTYPE html>"));
    assert!(reader_html.contains("function escapeHtml(str)"));
    assert!(reader_html.contains("/api/books/${bookId}/chapter/"));
    assert!(reader_html.contains("/api/i18n"));
}

#[tokio::test]
async fn test_google_drive_api_backend_and_token_refresh() {
    use axum::{
        body::Bytes,
        extract::{Path, Query, State},
        http::{HeaderMap, StatusCode},
        response::IntoResponse,
        routing::{get, patch, post, put},
        Json, Router,
    };
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use vellum_core::{CloudSyncBackend, GoogleDriveApiBackend, GoogleDriveTokens};

    #[derive(Clone, Default)]
    struct MockDriveItem {
        id: String,
        name: String,
        parent: String,
        mime_type: String,
        data: Vec<u8>,
    }

    type PendingUploadMap = HashMap<String, (String, String, Option<String>)>;

    #[derive(Clone, Default)]
    struct MockState {
        items: Arc<Mutex<Vec<MockDriveItem>>>,
        next_id: Arc<Mutex<usize>>,
        pending_uploads: Arc<Mutex<PendingUploadMap>>,
        token_refresh_calls: Arc<Mutex<usize>>,
    }

    impl MockState {
        fn alloc_id(&self) -> String {
            let mut g = self.next_id.lock().unwrap();
            *g += 1;
            format!("drv_{}", *g)
        }
    }

    async fn oauth_token(State(st): State<MockState>) -> impl IntoResponse {
        *st.token_refresh_calls.lock().unwrap() += 1;
        Json(serde_json::json!({
            "access_token": "fresh-access-token",
            "expires_in": 3600
        }))
    }

    async fn list_files(
        State(st): State<MockState>,
        Query(q): Query<HashMap<String, String>>,
    ) -> impl IntoResponse {
        let query_str = q.get("q").cloned().unwrap_or_default();
        let items = st.items.lock().unwrap();
        let matched: Vec<_> = items
            .iter()
            .filter(|it| {
                let parent_ok = query_str.contains(&format!("'{}' in parents", it.parent));
                let name_ok = if let Some(rest) = query_str.split("name = '").nth(1) {
                    let target_name = rest.split('\'').next().unwrap_or("");
                    it.name == target_name
                } else {
                    true
                };
                let folder_ok = if query_str
                    .contains("mimeType = 'application/vnd.google-apps.folder'")
                {
                    it.mime_type == "application/vnd.google-apps.folder"
                } else if query_str.contains("mimeType != 'application/vnd.google-apps.folder'") {
                    it.mime_type != "application/vnd.google-apps.folder"
                } else {
                    true
                };
                parent_ok && name_ok && folder_ok
            })
            .map(|it| serde_json::json!({ "id": it.id, "name": it.name }))
            .collect();

        Json(serde_json::json!({ "files": matched }))
    }

    async fn create_folder(
        State(st): State<MockState>,
        Json(body): Json<serde_json::Value>,
    ) -> impl IntoResponse {
        let id = st.alloc_id();
        let name = body["name"].as_str().unwrap_or("").to_string();
        let mime_type = body["mimeType"].as_str().unwrap_or("").to_string();
        let parent = body["parents"][0].as_str().unwrap_or("root").to_string();
        st.items.lock().unwrap().push(MockDriveItem {
            id: id.clone(),
            name,
            parent,
            mime_type,
            data: Vec::new(),
        });
        Json(serde_json::json!({ "id": id }))
    }

    async fn get_file(
        State(st): State<MockState>,
        Path(file_id): Path<String>,
    ) -> impl IntoResponse {
        let items = st.items.lock().unwrap();
        if let Some(it) = items.iter().find(|i| i.id == file_id) {
            (StatusCode::OK, it.data.clone()).into_response()
        } else {
            StatusCode::NOT_FOUND.into_response()
        }
    }

    async fn upload_files(
        State(st): State<MockState>,
        Query(q): Query<HashMap<String, String>>,
        headers: HeaderMap,
        body: Bytes,
    ) -> impl IntoResponse {
        let upload_type = q.get("uploadType").map(String::as_str).unwrap_or("");
        if upload_type == "multipart" {
            let ct = headers
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            let boundary = ct.split("boundary=").nth(1).unwrap_or("");
            let raw = String::from_utf8_lossy(&body);
            let parts: Vec<&str> = raw.split(&format!("--{boundary}")).collect();
            let meta_part = parts.get(1).unwrap_or(&"");
            let meta_json_str = meta_part.split("\r\n\r\n").nth(1).unwrap_or("{}").trim();
            let meta: serde_json::Value = serde_json::from_str(meta_json_str).unwrap();
            let data_part = parts.get(2).unwrap_or(&"");
            let data_str = data_part
                .split("\r\n\r\n")
                .nth(1)
                .unwrap_or("")
                .trim_end_matches("\r\n");

            let id = st.alloc_id();
            st.items.lock().unwrap().push(MockDriveItem {
                id: id.clone(),
                name: meta["name"].as_str().unwrap_or("").to_string(),
                parent: meta["parents"][0].as_str().unwrap_or("root").to_string(),
                mime_type: "application/json".to_string(),
                data: data_str.as_bytes().to_vec(),
            });
            return (StatusCode::OK, Json(serde_json::json!({ "id": id }))).into_response();
        }

        if upload_type == "resumable" {
            let meta: serde_json::Value = serde_json::from_slice(&body).unwrap();
            let session_id = st.alloc_id();
            let name = meta["name"].as_str().unwrap_or("").to_string();
            let parent = meta["parents"][0].as_str().unwrap_or("root").to_string();
            st.pending_uploads
                .lock()
                .unwrap()
                .insert(session_id.clone(), (name, parent, None));
            let host = headers
                .get("host")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("127.0.0.1");
            return (
                StatusCode::OK,
                [(
                    axum::http::header::LOCATION,
                    format!("http://{host}/upload/session/{session_id}"),
                )],
                "",
            )
                .into_response();
        }

        StatusCode::BAD_REQUEST.into_response()
    }

    async fn patch_resumable(
        State(st): State<MockState>,
        Path(file_id): Path<String>,
        headers: HeaderMap,
        body: Bytes,
    ) -> impl IntoResponse {
        let meta: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let session_id = st.alloc_id();
        let name = meta["name"].as_str().unwrap_or("").to_string();
        st.pending_uploads
            .lock()
            .unwrap()
            .insert(session_id.clone(), (name, String::new(), Some(file_id)));
        let host = headers
            .get("host")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("127.0.0.1");
        (
            StatusCode::OK,
            [(
                axum::http::header::LOCATION,
                format!("http://{host}/upload/session/{session_id}"),
            )],
            "",
        )
    }

    async fn put_session(
        State(st): State<MockState>,
        Path(session_id): Path<String>,
        body: Bytes,
    ) -> impl IntoResponse {
        let pending = st.pending_uploads.lock().unwrap().remove(&session_id);
        let Some((name, parent, existing_id)) = pending else {
            return StatusCode::NOT_FOUND.into_response();
        };

        let mut items = st.items.lock().unwrap();
        if let Some(eid) = existing_id {
            if let Some(it) = items.iter_mut().find(|i| i.id == eid) {
                it.data = body.to_vec();
                return (StatusCode::OK, Json(serde_json::json!({ "id": eid }))).into_response();
            }
        }
        let id = st.alloc_id();
        items.push(MockDriveItem {
            id: id.clone(),
            name,
            parent,
            mime_type: "application/octet-stream".to_string(),
            data: body.to_vec(),
        });
        (StatusCode::OK, Json(serde_json::json!({ "id": id }))).into_response()
    }

    let state = MockState::default();
    let app = Router::new()
        .route("/oauth/token", post(oauth_token))
        .route("/drive/v3/files", get(list_files).post(create_folder))
        .route("/drive/v3/files/{id}", get(get_file))
        .route("/upload/drive/v3/files", post(upload_files))
        .route("/upload/drive/v3/files/{id}", patch(patch_resumable))
        .route("/upload/session/{session_id}", put(put_session))
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let temp_dir = tempfile::tempdir().unwrap();
    let tokens_path = temp_dir.path().join("gdrive_tokens.json");

    let local_epub = temp_dir.path().join("dune.epub");
    std::fs::write(&local_epub, b"MOCK-EPUB-BYTES").unwrap();

    let downloaded_epub = temp_dir.path().join("downloaded.epub");

    let result = tokio::task::spawn_blocking(move || {
        // Expired access_token + valid refresh_token => forces TokenManager to refresh
        let initial_tokens = GoogleDriveTokens {
            access_token: "expired-token".to_string(),
            refresh_token: Some("valid-refresh-token".to_string()),
            expires_at_unix: 100,
            account_email: Some("reader@example.com".to_string()),
        };

        let backend = GoogleDriveApiBackend::new(
            "test-client-id".to_string(),
            None,
            "Vellum".to_string(),
            tokens_path.clone(),
        )
        .unwrap()
        .with_endpoints(
            format!("http://127.0.0.1:{port}/drive/v3"),
            format!("http://127.0.0.1:{port}/upload/drive/v3"),
            format!("http://127.0.0.1:{port}/oauth/token"),
        )
        .with_initial_tokens(initial_tokens);

        assert!(backend.is_available());
        assert!(!backend.file_exists("Frank Herbert/Dune/dune.epub").unwrap());

        backend
            .upload_file(&local_epub, "Frank Herbert/Dune/dune.epub")
            .unwrap();
        assert!(backend.file_exists("Frank Herbert/Dune/dune.epub").unwrap());

        let ok = backend
            .download_file("Frank Herbert/Dune/dune.epub", &downloaded_epub)
            .unwrap();
        assert!(ok);
        assert_eq!(std::fs::read(&downloaded_epub).unwrap(), b"MOCK-EPUB-BYTES");

        let delta = vellum_domain::SyncDelta {
            client_id: "client-linux".to_string(),
            created_at: "2026-11-17T20:00:00Z".to_string(),
            events: vec![vellum_domain::SyncEvent::new(
                "client-linux".to_string(),
                vellum_domain::SyncAction::BookDeleted {
                    book_id: "book-123".to_string(),
                },
            )],
        };
        let fname = backend.write_delta(&delta).unwrap();
        let deltas = backend.list_deltas().unwrap();
        assert_eq!(deltas, vec![fname.clone()]);
        let read_back = backend.read_delta(&fname).unwrap();
        assert_eq!(read_back.client_id, "client-linux");
        assert_eq!(read_back.events.len(), 1);
    })
    .await;

    result.unwrap();
    assert_eq!(*state.token_refresh_calls.lock().unwrap(), 1);
}

#[tokio::test]
async fn test_web_companion_upload_and_metadata_edit() {
    let temp = TempDir::new().expect("tempdir");
    let book_service = create_test_services(&temp);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let app = vellum_core::opds_server::OpdsServer::router(book_service);

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();

    // 1. Upload book via POST /api/books/upload (with traversal attempt in filename)
    let upload_res = client
        .post(format!(
            "http://127.0.0.1:{port}/api/books/upload?filename=../../novel.txt"
        ))
        .body("Chapter 1\n\nOnce upon a time in Vellum.")
        .send()
        .await
        .unwrap();
    assert_eq!(upload_res.status(), 201);
    let uploaded: vellum_core::opds_server::BookDto = upload_res.json().await.unwrap();
    assert!(!uploaded.id.is_empty());

    // 2. Edit metadata via POST /api/books/:id/metadata
    let edit_res = client
        .post(format!(
            "http://127.0.0.1:{port}/api/books/{}/metadata",
            uploaded.id
        ))
        .json(&serde_json::json!({
            "title": "Dune Messiah",
            "author": "Frank Herbert",
            "series": "Dune",
            "series_index": 2.0,
            "tags": ["Sci-Fi", "Classic"]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(edit_res.status(), 200);
    let updated: vellum_core::opds_server::BookDto = edit_res.json().await.unwrap();
    assert_eq!(updated.title, "Dune Messiah");
    assert_eq!(updated.author, "Frank Herbert");
    assert_eq!(updated.series.as_deref(), Some("Dune"));
    assert_eq!(updated.series_index, Some(2.0));
    assert!(updated.tags.contains(&"Sci-Fi".to_string()));

    // 3. Stats endpoint
    let stats_res = client
        .get(format!("http://127.0.0.1:{port}/api/stats"))
        .send()
        .await
        .unwrap();
    assert_eq!(stats_res.status(), 200);
    let stats_json: serde_json::Value = stats_res.json().await.unwrap();
    assert_eq!(stats_json["total_books"], 1);
    assert_eq!(stats_json["total_series"], 1);

    // 4. Sync status endpoint (no SyncService attached -> configured=false)
    let sync_st_res = client
        .get(format!("http://127.0.0.1:{port}/api/sync/status"))
        .send()
        .await
        .unwrap();
    assert_eq!(sync_st_res.status(), 200);
    let sync_st_json: serde_json::Value = sync_st_res.json().await.unwrap();
    assert_eq!(sync_st_json["configured"], false);

    // 5. KOReader Progress Sync endpoints
    let ko_auth = client
        .get(format!("http://127.0.0.1:{port}/users/auth"))
        .send()
        .await
        .unwrap();
    assert_eq!(ko_auth.status(), 200);

    let ko_put = client
        .put(format!("http://127.0.0.1:{port}/syncs/progress"))
        .json(&serde_json::json!({
            "document": uploaded.id,
            "progress": "/body/chapter/2",
            "percentage": 0.45,
            "device": "kobo-clara"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(ko_put.status(), 200);

    let ko_get = client
        .get(format!(
            "http://127.0.0.1:{port}/syncs/progress/{}",
            uploaded.id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(ko_get.status(), 200);
    let ko_json: serde_json::Value = ko_get.json().await.unwrap();
    assert_eq!(ko_json["progress"], "/body/chapter/2");
    assert!((ko_json["percentage"].as_f64().unwrap() - 0.45).abs() < 0.001);

    // 6. Audio streaming with HTTP Range (206 Partial Content)
    let audio_up = client
        .post(format!(
            "http://127.0.0.1:{port}/api/books/upload?filename=audiobook.mp3"
        ))
        .body(b"0123456789ABCDEF".to_vec())
        .send()
        .await
        .unwrap();
    assert_eq!(audio_up.status(), 201);
    let audio_book: vellum_core::opds_server::BookDto = audio_up.json().await.unwrap();

    let range_res = client
        .get(format!(
            "http://127.0.0.1:{port}/api/books/{}/audio",
            audio_book.id
        ))
        .header("Range", "bytes=4-9")
        .send()
        .await
        .unwrap();
    assert_eq!(range_res.status(), 206);
    assert_eq!(
        range_res
            .headers()
            .get("content-range")
            .and_then(|v| v.to_str().ok()),
        Some("bytes 4-9/16")
    );
    assert_eq!(range_res.bytes().await.unwrap().as_ref(), b"456789");

    // 7. Lightweight multi-user progress isolation via X-Vellum-User
    let alice_prog = client
        .post(format!(
            "http://127.0.0.1:{port}/api/books/{}/progress",
            uploaded.id
        ))
        .header("X-Vellum-User", "Alice")
        .json(&serde_json::json!({ "chapter_index": 3, "percentage": 0.72 }))
        .send()
        .await
        .unwrap();
    assert_eq!(alice_prog.status(), 200);

    let alice_detail: vellum_core::opds_server::BookDto = client
        .get(format!("http://127.0.0.1:{port}/api/books/{}", uploaded.id))
        .header("X-Vellum-User", "alice")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!((alice_detail.progress_percent - 72.0).abs() < 0.01);

    let bob_detail: vellum_core::opds_server::BookDto = client
        .get(format!("http://127.0.0.1:{port}/api/books/{}", uploaded.id))
        .header("X-Vellum-User", "bob")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(bob_detail.progress_percent, 0.0);
}
