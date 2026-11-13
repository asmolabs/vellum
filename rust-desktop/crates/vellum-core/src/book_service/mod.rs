//! Service principal de gestion de la bibliothèque (import, métadonnées, conversion, audit).

mod conversion;
mod epub_and_audit;
mod lifecycle;
mod metadata;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

use vellum_domain::{Book, VellumResult};
use vellum_storage::{BookRepository, ParamRepository, SyncRepository, TagRepository};

use crate::conversion_service::ConversionService;
use crate::metadata_service::MetadataService;
use crate::plugin_service::PluginService;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DuplicateBookInfo {
    pub existing_book: Book,
    pub matching_fields: Vec<String>,
}

#[derive(Clone)]
pub struct BookService {
    pub(super) book_repo: BookRepository,
    pub(super) param_repo: ParamRepository,
    pub(super) tag_repo: TagRepository,
    pub(super) sync_repo: Option<SyncRepository>,
    pub(super) client_id: String,
    pub(super) metadata_service: MetadataService,
    pub(super) plugin_service: PluginService,
    pub(super) conversion_service: ConversionService,
    pub(super) library_path: PathBuf,
}

impl PartialEq for BookService {
    fn eq(&self, other: &Self) -> bool {
        self.client_id == other.client_id && self.library_path == other.library_path
    }
}

impl BookService {
    pub fn new(
        book_repo: BookRepository,
        param_repo: ParamRepository,
        tag_repo: TagRepository,
        metadata_service: MetadataService,
        plugin_service: PluginService,
        conversion_service: ConversionService,
        library_path: PathBuf,
    ) -> Self {
        let service = Self {
            book_repo,
            param_repo,
            tag_repo,
            sync_repo: None,
            client_id: "default-client".to_string(),
            metadata_service,
            plugin_service,
            conversion_service,
            library_path,
        };
        let _ = service.ensure_library_directory();
        service
    }

    pub fn with_sync_repo(mut self, sync_repo: SyncRepository, client_id: impl Into<String>) -> Self {
        self.sync_repo = Some(sync_repo);
        self.client_id = client_id.into();
        self
    }

    pub fn library_path(&self) -> &Path {
        &self.library_path
    }

    pub fn book_repo(&self) -> &BookRepository {
        &self.book_repo
    }

    pub fn param_repo(&self) -> &ParamRepository {
        &self.param_repo
    }

    pub fn tag_repo(&self) -> &TagRepository {
        &self.tag_repo
    }

    pub fn conversion_service(&self) -> &ConversionService {
        &self.conversion_service
    }

    pub fn plugin_service(&self) -> &PluginService {
        &self.plugin_service
    }

    pub fn metadata_service(&self) -> &MetadataService {
        &self.metadata_service
    }

    pub fn ensure_library_directory(&self) -> VellumResult<()> {
        if !self.library_path.exists() {
            fs::create_dir_all(&self.library_path)?;
        }
        Ok(())
    }

    pub fn sanitize_filename(name: &str) -> String {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return "Sans nom".to_string();
        }
        let sanitized: String = trimmed
            .chars()
            .map(|c| {
                if c.is_control() {
                    '_'
                } else {
                    match c {
                        '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
                        _ => c,
                    }
                }
            })
            .collect();
        let cleaned = sanitized.trim().trim_matches('.').trim();
        if cleaned.is_empty() || cleaned == "." || cleaned == ".." {
            "Sans nom".to_string()
        } else {
            cleaned.to_string()
        }
    }

    pub fn get_books(
        &self,
        search: Option<&str>,
        tag: Option<&str>,
    ) -> VellumResult<Vec<Book>> {
        self.book_repo.search_books(search, tag)
    }

    pub fn get_books_filtered(
        &self,
        search: Option<&str>,
        tag: Option<&str>,
        status: Option<&str>,
    ) -> VellumResult<Vec<Book>> {
        let mut books = self.book_repo.search_books(search, tag)?;

        if let Some(st) = status {
            let filter = st.trim();
            if !filter.is_empty() && !filter.eq_ignore_ascii_case("Tous") {
                books.retain(|b| {
                    if filter.eq_ignore_ascii_case("En cours") || filter.contains("cours") {
                        b.progress_percent > 0.0 && !b.is_finished
                    } else if filter.eq_ignore_ascii_case("Terminés") || filter.contains("Terminé") {
                        b.is_finished
                    } else if filter.eq_ignore_ascii_case("À lire") || filter.contains("lire") {
                        b.progress_percent <= 0.0 && !b.is_finished
                    } else {
                        true
                    }
                });
            }
        }

        Ok(books)
    }

    pub fn get_book_by_id(&self, id: &str) -> VellumResult<Option<Book>> {
        self.book_repo.find_by_id(id)
    }

    pub fn get_total_books_count(&self) -> VellumResult<usize> {
        self.book_repo.count()
    }

    pub fn get_tags_with_counts(&self) -> VellumResult<BTreeMap<String, usize>> {
        self.tag_repo.find_tags_with_book_counts()
    }

    pub fn check_duplicate(&self, title: &str, author: &str) -> VellumResult<Option<DuplicateBookInfo>> {
        let books = self.book_repo.find_all()?;
        let clean_title = title.trim().to_lowercase();
        let clean_author = author.trim().to_lowercase();

        for b in books {
            let mut matches = Vec::new();
            let b_title = b.title.trim().to_lowercase();
            let b_author = b.author.trim().to_lowercase();

            if !clean_title.is_empty() && b_title == clean_title {
                matches.push("title".to_string());
            }
            if !clean_author.is_empty() && b_author == clean_author {
                matches.push("author".to_string());
            }

            if !matches.is_empty() && matches.len() == 2 {
                return Ok(Some(DuplicateBookInfo {
                    existing_book: b,
                    matching_fields: matches,
                }));
            }
        }
        Ok(None)
    }


}

#[cfg(test)]
mod tests {
    use super::BookService;

    #[test]
    fn test_sanitize_filename_blocks_traversal_and_control_chars() {
        assert_eq!(BookService::sanitize_filename(".."), "Sans nom");
        assert_eq!(BookService::sanitize_filename("."), "Sans nom");
        assert_eq!(BookService::sanitize_filename("..."), "Sans nom");
        assert_eq!(BookService::sanitize_filename("  .hidden.  "), "hidden");
        assert_eq!(
            BookService::sanitize_filename("../../etc/passwd"),
            "_.._etc_passwd"
        );
        assert_eq!(BookService::sanitize_filename("Dune\x00: Tome 1"), "Dune__ Tome 1");
    }
}
