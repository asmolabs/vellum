use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use vellum_domain::{Book, BookFormat, VellumError, VellumResult};

use super::{BookService, TranslateBookRequest};

/// Rapport d'une opération de conversion en lot (`bulk_convert_books`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BulkConvertReport {
    pub target_format: String,
    pub converted_count: usize,
    pub skipped_count: usize,
    pub errors: Vec<String>,
}

impl BookService {
    pub fn get_convertible_formats_for_book(&self, book: &Book) -> Vec<String> {
        let mut available = std::collections::HashSet::new();
        let existing: std::collections::HashSet<String> = book
            .formats
            .iter()
            .map(|f| f.format.to_uppercase())
            .collect();

        for fmt in &book.formats {
            for target in self.conversion_service.available_target_formats(&fmt.format) {
                if !existing.contains(&target) {
                    available.insert(target);
                }
            }
        }

        let mut list: Vec<String> = available.into_iter().collect();
        list.sort();
        list
    }

    /// Convertit un lot de livres vers `target_format`, en ignorant ceux qui possèdent déjà ce format.
    pub fn bulk_convert_books(
        &self,
        book_ids: &[String],
        target_format: &str,
    ) -> BulkConvertReport {
        let target_ext = target_format.trim().to_lowercase();
        let mut report = BulkConvertReport {
            target_format: target_ext.to_uppercase(),
            ..Default::default()
        };

        for book_id in book_ids {
            let Ok(Some(book)) = self.book_repo.find_by_id(book_id) else {
                report
                    .errors
                    .push(format!("{book_id} : livre introuvable"));
                continue;
            };
            if book
                .formats
                .iter()
                .any(|f| f.format.eq_ignore_ascii_case(&target_ext))
            {
                report.skipped_count += 1;
                continue;
            }
            match self.convert_book_to_format(book_id, &target_ext) {
                Ok(_) => report.converted_count += 1,
                Err(e) => report.errors.push(format!("{} : {e}", book.title)),
            }
        }

        report
    }

    pub fn convert_book_to_format(
        &self,
        book_id: &str,
        target_format: &str,
    ) -> VellumResult<Book> {
        let mut book = self
            .book_repo
            .find_by_id(book_id)?
            .ok_or_else(|| VellumError::NotFound(format!("Livre introuvable: {book_id}")))?;

        let target_ext = target_format.trim().to_lowercase();
        if book
            .formats
            .iter()
            .any(|f| f.format.eq_ignore_ascii_case(&target_ext))
        {
            return Ok(book);
        }

        // Priorité au format pivot (EPUB) s'il est déjà présent
        let mut source_candidate = None;
        if let Some(epub_fmt) = book
            .formats
            .iter()
            .find(|f| f.format.eq_ignore_ascii_case("epub"))
        {
            if self
                .conversion_service
                .find_conversion_path("epub", &target_ext)
                .is_some()
            {
                source_candidate = Some(epub_fmt);
            }
        }
        if source_candidate.is_none() {
            source_candidate = book.formats.iter().find(|f| {
                self.conversion_service
                    .find_conversion_path(&f.format, &target_ext)
                    .is_some()
            });
        }

        let source = source_candidate.ok_or_else(|| {
            VellumError::Unsupported(format!(
                "Aucun format source convertible en {} pour ce livre",
                target_ext.to_uppercase()
            ))
        })?;

        let source_abs = self.library_path.join(&source.filepath);
        if !source_abs.exists() {
            return Err(VellumError::NotFound(format!(
                "Fichier source introuvable: {}",
                source_abs.display()
            )));
        }

        let book_dir_abs = source_abs
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| self.library_path.clone());

        let target_filename = format!("{}.{target_ext}", Self::sanitize_filename(&book.title));
        let dest_abs = book_dir_abs.join(&target_filename);

        self.conversion_service.convert_file(
            &source.format,
            &target_ext,
            &source_abs,
            &dest_abs,
            None,
        )?;

        let dest_file_rel = dest_abs
            .strip_prefix(&self.library_path)
            .unwrap_or(&dest_abs)
            .to_string_lossy()
            .to_string();

        let file_size = fs::metadata(&dest_abs).map(|m| m.len()).unwrap_or(0);
        let fmt = BookFormat::new(target_ext, target_filename, dest_file_rel, file_size);
        book.add_format(fmt);

        let saved = self.book_repo.save(&book)?;

        if let Some(ref sr) = self.sync_repo {
            let event = vellum_domain::SyncEvent::new(
                &self.client_id,
                vellum_domain::SyncAction::BookUpdated {
                    book: saved.clone(),
                    tags: saved.tags.clone(),
                },
            );
            let _ = sr.record_event(&event);
        }

        Ok(saved)
    }


    pub fn translate_book(&self, req: TranslateBookRequest<'_>) -> VellumResult<Book> {
        let book = self
            .book_repo
            .find_by_id(req.book_id)?
            .ok_or_else(|| VellumError::NotFound(format!("Livre introuvable: {}", req.book_id)))?;

        let mut _temp_guard = None;
        let epub_path = if let Some(fmt) = book
            .formats
            .iter()
            .find(|f| f.format.eq_ignore_ascii_case("epub"))
        {
            self.library_path.join(&fmt.filepath)
        } else if let Some(first_fmt) = book.formats.first() {
            let src_file = self.library_path.join(&first_fmt.filepath);
            let temp_epub = tempfile::Builder::new()
                .prefix("vellum_translate_pivot_")
                .suffix(".epub")
                .tempfile()?;
            let temp_path = temp_epub.path().to_path_buf();
            _temp_guard = Some(temp_epub);
            self.conversion_service
                .convert_file(&first_fmt.format, "epub", &src_file, &temp_path, None)?;
            temp_path
        } else {
            return Err(VellumError::NotFound(
                "Aucun format de fichier disponible pour ce livre".to_string(),
            ));
        };

        let clean_author = Self::sanitize_filename(&book.author);
        let clean_title = format!(
            "{} (Traduit {})",
            Self::sanitize_filename(&book.title),
            req.target_lang
        );
        let new_book_dir = self
            .library_path
            .join(&clean_author)
            .join(format!("{clean_title} ({})", uuid::Uuid::new_v4()));
        fs::create_dir_all(&new_book_dir)?;

        let target_filename = format!("{clean_title}.epub");
        let dest_abs = new_book_dir.join(&target_filename);

        let mut config: HashMap<String, String> = HashMap::new();
        config.insert("targetLanguage".to_string(), req.target_lang.to_string());
        config.insert("provider".to_string(), req.provider.to_string());
        config.insert("model".to_string(), req.model.to_string());
        if let Some(u) = req.base_url {
            config.insert("baseUrl".to_string(), u.to_string());
        }
        if let Some(k) = req.api_key {
            config.insert("apiKey".to_string(), k.to_string());
        }
        config.insert("bilingualMode".to_string(), req.bilingual_mode.to_string());

        vellum_plugins::translator::pipeline::translate_epub(&epub_path, &dest_abs, &config)?;

        let cover_path = if let Some(ref orig_cov) = book.cover_path {
            let orig_cov_abs = self.library_path.join(orig_cov);
            if orig_cov_abs.exists() {
                let dest_cov = new_book_dir.join("cover.jpg");
                let _ = fs::copy(&orig_cov_abs, &dest_cov);
                dest_cov
                    .strip_prefix(&self.library_path)
                    .ok()
                    .map(|p| p.to_string_lossy().to_string())
            } else {
                None
            }
        } else {
            None
        };

        let mut new_book = Book::new(clean_title, book.author.clone());
        new_book.publisher = book.publisher.clone();
        new_book.description = book.description.clone();
        new_book.cover_path = cover_path;
        for t in &book.tags {
            new_book.add_tag(t.clone());
        }
        let tag_tr = vellum_domain::Tag::new(format!("Traduction {}", req.target_lang));
        let _ = self.tag_repo.save(&tag_tr);
        new_book.add_tag(tag_tr);

        let file_size = fs::metadata(&dest_abs).map(|m| m.len()).unwrap_or(0);
        let dest_file_rel = dest_abs
            .strip_prefix(&self.library_path)
            .unwrap_or(&dest_abs)
            .to_string_lossy()
            .to_string();
        let fmt = BookFormat::new("epub".to_string(), target_filename, dest_file_rel, file_size);
        new_book.add_format(fmt);

        self.book_repo.save(&new_book)?;
        Ok(new_book)
    }

    /// Imports an entire Calibre library (metadata.db) into Vellum
    pub fn import_calibre_library(
        &self,
        calibre_path: &Path,
        copy_files: bool,
    ) -> VellumResult<crate::calibre_importer::CalibreImportReport> {
        crate::calibre_importer::CalibreImporter::import_calibre_library(
            calibre_path,
            copy_files,
            self,
        )
    }

    /// Exports a book to an external directory using a naming template
    pub fn export_book_to_directory(
        &self,
        book_id: &str,
        target_dir: &Path,
        preferred_format: Option<&str>,
        template: Option<&str>,
    ) -> VellumResult<PathBuf> {
        let book = self
            .book_repo
            .find_by_id(book_id)?
            .ok_or_else(|| VellumError::NotFound(format!("Livre introuvable: {book_id}")))?;

        let format_entry = if let Some(pref) = preferred_format {
            book.formats.iter().find(|f| f.format.eq_ignore_ascii_case(pref))
        } else {
            book.formats
                .iter()
                .find(|f| f.format.eq_ignore_ascii_case("epub"))
                .or_else(|| book.formats.first())
        }
        .ok_or_else(|| VellumError::NotFound(format!("Aucun format disponible pour {}", book.title)))?;

        let source_path = if Path::new(&format_entry.filepath).is_absolute() {
            PathBuf::from(&format_entry.filepath)
        } else {
            self.library_path.join(&format_entry.filepath)
        };

        if !source_path.exists() {
            return Err(VellumError::NotFound(format!("Fichier source introuvable: {:?}", source_path)));
        }

        let tmpl = template.unwrap_or(vellum_domain::DEFAULT_NAMING_TEMPLATE);
        let relative_path = vellum_domain::NamingTemplate::format_filename(&book, tmpl, &format_entry.format);
        let dest_path = target_dir.join(relative_path);

        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::copy(&source_path, &dest_path)?;
        tracing::info!("Livre '{}' exporté avec succès vers {:?}", book.title, dest_path);

        Ok(dest_path)
    }

    /// Exporte l'ensemble du catalogue de la bibliothèque vers un fichier (CSV, JSON ou BibTeX).
    pub fn export_catalog(
        &self,
        format: crate::catalog_exporter::CatalogFormat,
        dest: &Path,
    ) -> VellumResult<usize> {
        let books = self.book_repo.find_all()?;
        let count = books.len();
        crate::catalog_exporter::CatalogExporter::export_to_file(&books, format, dest)?;
        Ok(count)
    }
}
