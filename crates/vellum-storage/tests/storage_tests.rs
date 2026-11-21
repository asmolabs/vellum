#![allow(clippy::unwrap_used)]

use serde_json::json;
use std::collections::HashMap;
use tempfile::tempdir;
use vellum_domain::{
    Book, BookFormat, CustomColumnDefinition, CustomColumnType, Param, Tag, VirtualLibrary,
};
use vellum_storage::{
    BookRepository, CustomColumnRepository, Database, ParamRepository, TagRepository,
    VirtualLibraryRepository,
};

#[test]
fn test_sqlite_wal_book_and_tag_persistence() {
    let dir = tempdir().unwrap();
    let db = Database::open(dir.path().join("test.db")).unwrap();
    let book_repo = BookRepository::new(db.clone());
    let tag_repo = TagRepository::new(db.clone());
    let param_repo = ParamRepository::new(db);

    let mut book = Book::new("Fondation", "Isaac Asimov");
    book.publisher = Some("Denoël".to_string());
    book.isbn = Some("9782207249117".to_string());
    book.description = Some("Cycle de Fondation".to_string());
    book.add_format(BookFormat::new(
        "epub",
        "fondation.epub",
        "Asimov/fondation.epub",
        250_000,
    ));
    book.add_tag(Tag::new("Science-Fiction"));
    book.add_tag(Tag::new("Classique"));

    book_repo.save(&book).unwrap();

    // Search
    let found = book_repo.search_books(Some("Asimov"), None).unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].title, "Fondation");
    assert_eq!(found[0].isbn.as_deref(), Some("9782207249117"));
    assert_eq!(found[0].formats.len(), 1);
    assert_eq!(found[0].tags.len(), 2);

    // Search by ISBN
    let found_by_isbn = book_repo.search_books(Some("9782207249117"), None).unwrap();
    assert_eq!(found_by_isbn.len(), 1);
    assert_eq!(found_by_isbn[0].title, "Fondation");

    let counts = tag_repo.find_tags_with_book_counts().unwrap();
    assert_eq!(counts.get("Science-Fiction"), Some(&1));
    assert_eq!(counts.get("Classique"), Some(&1));

    // Param persistence
    let mut param = Param::new("plugin:test:config", "test", "config", "{}");
    let mut map = HashMap::new();
    map.insert("lang".to_string(), json!("fr"));
    param.set_data(&map);
    param_repo.save(&param).unwrap();

    let loaded_param = param_repo
        .find_by_plugin_id_and_key("test", "config")
        .unwrap()
        .unwrap();
    assert_eq!(loaded_param.get_data().get("lang"), Some(&json!("fr")));

    // Delete
    book_repo.delete(&book.id).unwrap();
    assert_eq!(book_repo.count().unwrap(), 0);
}

#[test]
fn test_sync_repository_events_and_deltas() {
    let dir = tempdir().unwrap();
    let db = Database::open(dir.path().join("test_sync.db")).unwrap();
    let sync_repo = vellum_storage::SyncRepository::new(db);

    assert_eq!(sync_repo.count_unsynced_events().unwrap(), 0);

    let event = vellum_domain::SyncEvent::new(
        "macbook-1",
        vellum_domain::SyncAction::ProgressUpdated {
            book_id: "book-123".to_string(),
            progress_percent: 45.0,
            current_chapter_index: 3,
            total_chapters: 10,
            last_read_at: "2026-11-10T07:00:00Z".to_string(),
            is_finished: false,
        },
    );

    sync_repo.record_event(&event).unwrap();
    assert_eq!(sync_repo.count_unsynced_events().unwrap(), 1);

    let unsynced = sync_repo.get_unsynced_events().unwrap();
    assert_eq!(unsynced.len(), 1);
    assert_eq!(unsynced[0].client_id, "macbook-1");
    match &unsynced[0].action {
        vellum_domain::SyncAction::ProgressUpdated {
            progress_percent, ..
        } => {
            assert_eq!(*progress_percent, 45.0);
        }
        _ => panic!("Expected ProgressUpdated"),
    }

    sync_repo
        .mark_events_synced(&[unsynced[0].id.clone()], "2026-11-10T07:05:00Z")
        .unwrap();
    assert_eq!(sync_repo.count_unsynced_events().unwrap(), 0);

    // Delta applied tracking
    assert!(!sync_repo.is_delta_applied("client2_delta1.json").unwrap());
    sync_repo
        .mark_delta_applied("client2_delta1.json", "client-2")
        .unwrap();
    assert!(sync_repo.is_delta_applied("client2_delta1.json").unwrap());
}

#[test]
fn test_fts5_accent_insensitive_search_and_bm25_ranking() {
    let db = Database::open_in_memory().expect("open in-memory db");
    let book_repo = BookRepository::new(db);

    let mut b1 = Book::new("Les Misérables", "Victor Hugo");
    b1.description = Some("Roman social majeur du XIXe siècle".to_string());
    book_repo.save(&b1).unwrap();

    let mut b2 = Book::new("Guide de Paris", "Auteur Divers");
    b2.description = Some("Évoque les miserables quartiers de l'époque".to_string());
    book_repo.save(&b2).unwrap();

    // Accent-insensitive + prefix match + BM25 title weight > description weight
    let hits = book_repo.search_fts("miserab", None).unwrap();
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].title, "Les Misérables");

    // Special characters shouldn't trigger FTS5 syntax errors
    let safe_hits = book_repo
        .search_fts("hugo: \"paris\" + (test)!", None)
        .unwrap();
    assert!(safe_hits.is_empty());

    // Delete removes from FTS index via trigger
    book_repo.delete(&b1.id).unwrap();
    let after_del = book_repo.search_fts("miserables", None).unwrap();
    assert_eq!(after_del.len(), 1);
    assert_eq!(after_del[0].id, b2.id);
}

#[test]
fn test_custom_columns_crud_and_book_values() {
    let db = Database::open_in_memory().unwrap();
    let book_repo = BookRepository::new(db.clone());
    let col_repo = CustomColumnRepository::new(db);

    let book = Book::new("Hyperion", "Dan Simmons");
    book_repo.save(&book).unwrap();

    let col_read_status =
        CustomColumnDefinition::new("lecture_prio", "Priorité", CustomColumnType::Number);
    let col_source = CustomColumnDefinition::new("source", "Origine", CustomColumnType::Text);
    col_repo.save_definition(&col_read_status).unwrap();
    col_repo.save_definition(&col_source).unwrap();

    let defs = col_repo.list_definitions().unwrap();
    assert_eq!(defs.len(), 2);
    assert_eq!(defs[0].key, "source");
    assert_eq!(defs[1].key, "lecture_prio");

    let found = col_repo.find_by_key("lecture_prio").unwrap().unwrap();
    assert_eq!(found.column_type, CustomColumnType::Number);

    col_repo
        .set_value(&book.id, &col_read_status.id, "5")
        .unwrap();
    col_repo
        .set_value(&book.id, &col_source.id, "Librairie")
        .unwrap();
    // Upsert overwrites existing value cleanly
    col_repo
        .set_value(&book.id, &col_read_status.id, "1")
        .unwrap();

    let values = col_repo.get_values_for_book(&book.id).unwrap();
    assert_eq!(values.get("lecture_prio").map(String::as_str), Some("1"));
    assert_eq!(values.get("source").map(String::as_str), Some("Librairie"));

    assert!(col_repo.clear_value(&book.id, &col_source.id).unwrap());
    let after_clear = col_repo.get_values_for_book(&book.id).unwrap();
    assert!(!after_clear.contains_key("source"));

    assert!(col_repo.delete_definition(&col_read_status.id).unwrap());
    assert!(col_repo.get_values_for_book(&book.id).unwrap().is_empty());
}

#[test]
fn test_virtual_libraries_crud_and_upsert() {
    let db = Database::open_in_memory().unwrap();
    let vl_repo = VirtualLibraryRepository::new(db);

    let mut vl1 = VirtualLibrary::new(
        "SF Française",
        "tag:Sci-Fi language:fr",
        Some("#6366f1".to_string()),
    );
    let vl2 = VirtualLibrary::new("BD & Manga", "format:cbz", None);

    vl_repo.save(&vl1).unwrap();
    vl_repo.save(&vl2).unwrap();

    let all = vl_repo.find_all().unwrap();
    assert_eq!(all.len(), 2);
    assert_eq!(all[0].name, "BD & Manga");
    assert_eq!(all[1].name, "SF Française");

    vl1.query = "tag:Sci-Fi not format:pdf".to_string();
    vl_repo.save(&vl1).unwrap();
    let loaded = vl_repo.find_by_id(&vl1.id).unwrap().unwrap();
    assert_eq!(loaded.query, "tag:Sci-Fi not format:pdf");

    assert!(vl_repo.delete(&vl2.id).unwrap());
    assert!(!vl_repo.delete("non-existent-id").unwrap());
    assert_eq!(vl_repo.find_all().unwrap().len(), 1);
}

#[test]
fn test_param_kv_and_book_tag_replacement() {
    let db = Database::open_in_memory().unwrap();
    let book_repo = BookRepository::new(db.clone());
    let tag_repo = TagRepository::new(db.clone());
    let param_repo = ParamRepository::new(db);

    param_repo
        .set_value("watch_folder_path", "/tmp/vellum-inbox")
        .unwrap();
    assert_eq!(
        param_repo
            .get_value("watch_folder_path")
            .unwrap()
            .as_deref(),
        Some("/tmp/vellum-inbox")
    );

    let mut book = Book::new("La Horde du Contrevent", "Alain Damasio");
    book.add_tag(Tag::new("Fantasy"));
    book.add_tag(Tag::new("Temporaire"));
    book_repo.save(&book).unwrap();

    book.tags.retain(|t| t.name != "Temporaire");
    book.add_tag(Tag::new("Chef-d'oeuvre"));
    book_repo.save(&book).unwrap();

    let reloaded = book_repo.find_by_id(&book.id).unwrap().unwrap();
    let names: Vec<&str> = reloaded.tags.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"Fantasy"));
    assert!(names.contains(&"Chef-d'oeuvre"));

    let counts = tag_repo.find_tags_with_book_counts().unwrap();
    assert_eq!(counts.get("Fantasy"), Some(&1));
    assert_eq!(counts.get("Chef-d'oeuvre"), Some(&1));
    assert_eq!(counts.get("Temporaire").copied().unwrap_or(0), 0);
}

#[test]
fn test_save_batch_and_load_batch() {
    let dir = tempdir().unwrap();
    let db = Database::open(dir.path().join("batch.db")).unwrap();
    let book_repo = BookRepository::new(db);

    let mut batch_books = Vec::new();
    for i in 0..25 {
        let mut b = Book::new(format!("Book {i}"), "Batch Author");
        b.add_format(BookFormat::new("epub", format!("b{i}.epub"), format!("b{i}.epub"), 100));
        b.add_tag(Tag::new("BatchTag"));
        batch_books.push(b);
    }
    book_repo.save_batch(&batch_books).unwrap();

    let all_batch = book_repo.search_books(Some("Batch Author"), None).unwrap();
    assert_eq!(all_batch.len(), 25);
    for b in &all_batch {
        assert_eq!(b.formats.len(), 1);
        assert_eq!(b.tags.len(), 1);
        assert_eq!(b.tags[0].name, "BatchTag");
    }
    assert_eq!(book_repo.count().unwrap(), 25);
}
