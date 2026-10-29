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
    assert_eq!(found[0].formats.len(), 1);
    assert_eq!(found[0].tags.len(), 2);

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
