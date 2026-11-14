//! Service principal de gestion de la bibliothèque (import, métadonnées, conversion, audit).

mod bulk_regex;
mod conversion;
mod epub_and_audit;
mod formats_progress;
mod lifecycle;
mod metadata;
pub mod thumbnails;

pub use bulk_regex::{BulkRegexChange, BulkRegexReport, BulkRegexRequest, RegexTargetField};
pub use conversion::BulkConvertReport;
pub use thumbnails::{
    ensure_thumbnail_for_cover, generate_webp_thumbnail, thumbnail_path_for_book,
};

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

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

/// Requête DTO pour la mise à jour complète des métadonnées d'un livre.
#[derive(Debug, Clone, Default)]
pub struct UpdateMetadataRequest<'a> {
    pub book_id: &'a str,
    pub title: &'a str,
    pub author: &'a str,
    pub publisher: Option<&'a str>,
    pub description: Option<&'a str>,
    pub isbn: Option<&'a str>,
    pub publish_date: Option<&'a str>,
    pub series: Option<&'a str>,
    pub series_index: Option<f64>,
    pub tag_names: Vec<String>,
}

/// Requête DTO pour la traduction d'un livre vers une langue cible.
#[derive(Debug, Clone)]
pub struct TranslateBookRequest<'a> {
    pub book_id: &'a str,
    pub target_lang: &'a str,
    pub provider: &'a str,
    pub model: &'a str,
    pub base_url: Option<&'a str>,
    pub api_key: Option<&'a str>,
    pub bilingual_mode: bool,
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

    pub fn with_sync_repo(
        mut self,
        sync_repo: SyncRepository,
        client_id: impl Into<String>,
    ) -> Self {
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

    pub fn get_books(&self, search: Option<&str>, tag: Option<&str>) -> VellumResult<Vec<Book>> {
        self.book_repo.search_books(search, tag)
    }

    /// Recherche plein texte rapide via SQLite FTS5 (insensible aux accents, triée par pertinence BM25).
    pub fn search_books_fts(&self, query: &str, tag: Option<&str>) -> VellumResult<Vec<Book>> {
        self.book_repo.search_fts(query, tag)
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
                    } else if filter.eq_ignore_ascii_case("Terminés") || filter.contains("Terminé")
                    {
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

    /// Retourne la locale UI persistée dans `ParamRepository`, ou la locale système par défaut.
    pub fn get_ui_locale(&self) -> crate::i18n::Locale {
        match self.param_repo.get_value("ui_locale") {
            Ok(Some(code)) => crate::i18n::Locale::parse_code(&code),
            _ => crate::i18n::Locale::detect_system(),
        }
    }

    /// Persiste la locale UI choisie par l'utilisateur (`"fr"` ou `"en"`).
    pub fn set_ui_locale(&self, locale: crate::i18n::Locale) -> VellumResult<()> {
        self.param_repo.set_value("ui_locale", locale.as_code())
    }

    pub fn get_tags_with_counts(&self) -> VellumResult<BTreeMap<String, usize>> {
        self.tag_repo.find_tags_with_book_counts()
    }

    pub fn check_duplicate(
        &self,
        title: &str,
        author: &str,
    ) -> VellumResult<Option<DuplicateBookInfo>> {
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

    /// Charge les règles automatiques d'import persistées dans SQLite.
    pub fn get_import_rules(&self) -> Vec<crate::import_rules::ImportRule> {
        self.param_repo
            .get_value("import_rules")
            .ok()
            .flatten()
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default()
    }

    /// Enregistre les règles automatiques d'import dans SQLite.
    pub fn save_import_rules(&self, rules: &[crate::import_rules::ImportRule]) -> VellumResult<()> {
        let json = serde_json::to_string(rules)?;
        self.param_repo.set_value("import_rules", &json)
    }

    /// Exporte les notes, surlignages et signets d'un livre au format Markdown.
    pub fn export_book_notes_markdown(
        &self,
        book_id: &str,
        dest: Option<&Path>,
    ) -> VellumResult<String> {
        let book = self.book_repo.find_by_id(book_id)?.ok_or_else(|| {
            vellum_domain::VellumError::NotFound(format!("Livre introuvable: {book_id}"))
        })?;
        let vellum_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".vellum");
        let user_data = crate::BookUserData::load_from_dir(&vellum_dir, book_id);
        let md = crate::NotesExporter::to_markdown(&book, &user_data);
        if let Some(path) = dest {
            crate::NotesExporter::export_to_file(&book, &user_data, path)?;
        }
        Ok(md)
    }

    /// Indexe le texte brut de tous les chapitres d'un livre dans `book_content_fts`.
    pub fn index_book_content(&self, book_id: &str) -> VellumResult<usize> {
        let book = self.book_repo.find_by_id(book_id)?.ok_or_else(|| {
            vellum_domain::VellumError::NotFound(format!("Livre introuvable: {book_id}"))
        })?;
        let fmt = book
            .formats
            .iter()
            .find(|f| {
                matches!(
                    f.format.to_lowercase().as_str(),
                    "epub" | "kepub" | "txt" | "md"
                )
            })
            .or_else(|| book.formats.first())
            .ok_or_else(|| {
                vellum_domain::VellumError::NotFound("Aucun format lisible pour ce livre".into())
            })?;

        let abs_path = if Path::new(&fmt.filepath).is_absolute() {
            PathBuf::from(&fmt.filepath)
        } else {
            self.library_path.join(&fmt.filepath)
        };

        let provider = crate::reader_provider::create_content_provider(&abs_path);
        let total = provider.total_pages();
        let mut chapters = Vec::with_capacity(total);
        for idx in 0..total {
            let title = provider
                .get_section_title(idx)
                .unwrap_or_else(|| format!("Chapitre {}", idx + 1));
            let plain = provider.chapter_text(idx);
            if !plain.trim().is_empty() {
                chapters.push((idx, title, plain));
            }
        }
        let count = chapters.len();
        self.book_repo.replace_book_content(book_id, &chapters)?;
        Ok(count)
    }

    /// Recherche plein texte à l'intérieur du contenu indexé des livres.
    pub fn search_inside_books(
        &self,
        query: &str,
        limit: usize,
    ) -> VellumResult<Vec<vellum_domain::ContentSearchHit>> {
        self.book_repo.search_book_content(query, limit)
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
        assert_eq!(
            BookService::sanitize_filename("Dune\x00: Tome 1"),
            "Dune__ Tome 1"
        );
    }
}
