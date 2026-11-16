#![allow(dead_code, clippy::unwrap_used)]

use tempfile::TempDir;
use vellum_core::book_service::BookService;
use vellum_core::conversion_service::ConversionService;
use vellum_core::metadata_service::MetadataService;
use vellum_core::plugin_service::PluginService;
use vellum_storage::{BookRepository, Database, ParamRepository, TagRepository};

pub fn create_test_services(temp: &TempDir) -> BookService {
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
