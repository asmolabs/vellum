use std::path::{Path, PathBuf};

use vellum_domain::{
    Book, EpubDiagnostics, EpubTocEntry, LibraryHealthReport, VellumError, VellumResult,
};

use crate::epub_editor::{EpubEditor, PolishOptions, PolishReport};
use super::BookService;

impl BookService {
    fn resolve_epub_path(&self, book_id: &str) -> VellumResult<(Book, PathBuf)> {
        let book = self
            .book_repo
            .find_by_id(book_id)?
            .ok_or_else(|| VellumError::NotFound(format!("Livre introuvable: {book_id}")))?;

        let format_entry = book
            .formats
            .iter()
            .find(|f| f.format.eq_ignore_ascii_case("epub") || f.format.eq_ignore_ascii_case("kepub"))
            .ok_or_else(|| {
                VellumError::NotFound(format!("Le livre '{}' ne possède aucun format EPUB", book.title))
            })?;

        let path = if Path::new(&format_entry.filepath).is_absolute() {
            PathBuf::from(&format_entry.filepath)
        } else {
            self.library_path.join(&format_entry.filepath)
        };

        if !path.exists() {
            return Err(VellumError::NotFound(format!(
                "Fichier EPUB introuvable sur le disque: {:?}",
                path
            )));
        }

        Ok((book, path))
    }

    /// Inspects the internal structure of a book's EPUB file (manifest, spine, toc, issues)
    pub fn inspect_book_epub(&self, book_id: &str) -> VellumResult<EpubDiagnostics> {
        let (_book, epub_path) = self.resolve_epub_path(book_id)?;
        EpubEditor::inspect(&epub_path)
    }

    /// Extracts the Table of Contents from a book's EPUB file
    pub fn get_book_toc(&self, book_id: &str) -> VellumResult<Vec<EpubTocEntry>> {
        let (_book, epub_path) = self.resolve_epub_path(book_id)?;
        EpubEditor::extract_toc(&epub_path)
    }

    /// Saves an updated Table of Contents back to the book's EPUB file
    pub fn update_book_toc(&self, book_id: &str, new_toc: &[EpubTocEntry]) -> VellumResult<()> {
        let (_book, epub_path) = self.resolve_epub_path(book_id)?;
        EpubEditor::save_toc(&epub_path, new_toc)
    }

    /// Automatically generates a fresh Table of Contents based on HTML heading tags
    pub fn generate_book_toc_from_headings(&self, book_id: &str) -> VellumResult<Vec<EpubTocEntry>> {
        let (_book, epub_path) = self.resolve_epub_path(book_id)?;
        EpubEditor::generate_toc_from_headings(&epub_path)
    }

    /// Cleans and repairs the book's EPUB file (mimetype placement, DRM stripping, TOC generation)
    pub fn repair_book_epub(&self, book_id: &str) -> VellumResult<EpubDiagnostics> {
        let (_book, epub_path) = self.resolve_epub_path(book_id)?;
        EpubEditor::clean_and_repair(&epub_path)
    }

    /// Exécute « Polish Book » sur l'EPUB du livre (métadonnées, couverture, typographie, TOC, réparation).
    pub fn polish_book(&self, book_id: &str, options: PolishOptions) -> VellumResult<PolishReport> {
        let (book, epub_path) = self.resolve_epub_path(book_id)?;
        let cover_abs = book.cover_path.as_ref().map(|rel| {
            let p = Path::new(rel);
            if p.is_absolute() {
                p.to_path_buf()
            } else {
                self.library_path.join(p)
            }
        });
        EpubEditor::polish(&epub_path, &book, cover_abs.as_deref(), options)
    }

    /// Réalise un audit complet de santé de la bibliothèque (doublons, fiches incomplètes, fichiers manquants).
    pub fn audit_library(&self) -> VellumResult<LibraryHealthReport> {
        let books = self.book_repo.find_all()?;
        Ok(crate::library_audit::audit_library(&books, &self.library_path))
    }

    /// Fusionne intelligemment un livre source dans un livre cible :
    /// - Transfère tous les formats du livre source vers le livre cible (en conservant les fichiers).
    /// - Fusionne les tags sans doublons.
    /// - Complète les métadonnées manquantes du livre cible (description, éditeur, date, ISBN, série, couverture).
    /// - Conserve la progression de lecture la plus avancée.
    /// - Supprime la fiche du livre source en nettoyant la base.
    pub fn merge_books(&self, target_book_id: &str, source_book_id: &str) -> VellumResult<Book> {
        if target_book_id == source_book_id {
            return Err(VellumError::InvalidArgument(
                "Impossible de fusionner un livre avec lui-même".to_string(),
            ));
        }

        let mut target = self
            .book_repo
            .find_by_id(target_book_id)?
            .ok_or_else(|| VellumError::NotFound(format!("Livre cible introuvable: {target_book_id}")))?;
        let source = self
            .book_repo
            .find_by_id(source_book_id)?
            .ok_or_else(|| VellumError::NotFound(format!("Livre source introuvable: {source_book_id}")))?;

        // 1. Déplacer / importer les formats du livre source dans la fiche cible
        for s_fmt in &source.formats {
            let src_file = self.get_file_for_format(s_fmt);
            if src_file.exists() {
                let target_has_fmt = target
                    .formats
                    .iter()
                    .any(|f| f.format.eq_ignore_ascii_case(&s_fmt.format));

                if !target_has_fmt {
                    target = self.add_format_to_book(&target.id, &src_file)?;
                } else if let Some(target_fmt) = target
                    .formats
                    .iter()
                    .find(|f| f.format.eq_ignore_ascii_case(&s_fmt.format))
                {
                    // Si le fichier source est plus volumineux ou plus complet, on le remplace
                    if s_fmt.filesize > target_fmt.filesize {
                        target = self.add_format_to_book(&target.id, &src_file)?;
                    }
                }
            }
        }

        // 2. Fusion des tags
        for tag in &source.tags {
            if !target.tags.iter().any(|t| t.name.eq_ignore_ascii_case(&tag.name)) {
                let _ = self.tag_repo.save(tag);
                target.add_tag(tag.clone());
            }
        }

        // 3. Compléter les métadonnées manquantes
        if target.description.as_deref().unwrap_or("").trim().is_empty() {
            if let Some(ref desc) = source.description {
                if !desc.trim().is_empty() {
                    target.description = Some(desc.clone());
                }
            }
        }

        if target.publisher.as_deref().unwrap_or("").trim().is_empty() {
            if let Some(ref publ) = source.publisher {
                if !publ.trim().is_empty() {
                    target.publisher = Some(publ.clone());
                }
            }
        }

        if target.publish_date.as_deref().unwrap_or("").trim().is_empty() {
            if let Some(ref pdate) = source.publish_date {
                if !pdate.trim().is_empty() {
                    target.publish_date = Some(pdate.clone());
                }
            }
        }

        if target.isbn.as_deref().unwrap_or("").trim().is_empty() {
            if let Some(ref isbn) = source.isbn {
                if !isbn.trim().is_empty() {
                    target.isbn = Some(isbn.clone());
                }
            }
        }

        if target.series.is_none() {
            target.series = source.series.clone();
            target.series_index = source.series_index;
        }

        if target.cover_path.is_none() {
            if let Some(ref scover) = source.cover_path {
                let src_cover_file = self.library_path.join(scover);
                if src_cover_file.exists() {
                    let _ = self.update_book_cover(&target.id, &src_cover_file);
                    if let Ok(Some(refreshed)) = self.book_repo.find_by_id(&target.id) {
                        target = refreshed;
                    }
                }
            }
        }

        // Progression de lecture : conserver la plus avancée
        if source.progress_percent > target.progress_percent {
            target.progress_percent = source.progress_percent;
            target.current_chapter_index = source.current_chapter_index;
            target.total_chapters = source.total_chapters;
            target.is_finished = source.is_finished || target.is_finished;
            target.last_read_at = source.last_read_at.clone().or(target.last_read_at);
        }

        let saved = self.update_book(&target)?;

        // 4. Supprimer l'ancienne fiche source avec ses fichiers restants
        let _ = self.delete_book(source_book_id, true);

        Ok(saved)
    }
}
