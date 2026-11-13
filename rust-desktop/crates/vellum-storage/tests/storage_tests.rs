use serde_json::json;
use std::collections::HashMap;
use tempfile::tempdir;
use vellum_domain::{Book, BookFormat, Param, Tag};
use vellum_storage::{BookRepository, Database, ParamRepository, TagRepository};

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

    // Tag counts
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
        vellum_domain::SyncAction::ProgressUpdated { progress_percent, .. } => {
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
    let safe_hits = book_repo.search_fts("hugo: \"paris\" + (test)!", None).unwrap();
    assert!(safe_hits.is_empty());

    // Delete removes from FTS index via trigger
    book_repo.delete(&b1.id).unwrap();
    let after_del = book_repo.search_fts("miserables", None).unwrap();
    assert_eq!(after_del.len(), 1);
    assert_eq!(after_del[0].id, b2.id);
}
