use std::collections::{BTreeMap, HashMap};

use std::fs;
use std::path::{Path, PathBuf};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use vellum_domain::{Book, BookFormat, Tag, VellumError, VellumResult};
use vellum_storage::{BookRepository, ParamRepository, SyncRepository, TagRepository};

use crate::conversion_service::ConversionService;
use crate::metadata_service::MetadataService;
use crate::plugin_service::PluginService;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DuplicateBookInfo {
    pub existing_book: Book,
    pub matching_fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReadingStats {
    pub total_books: usize,
    pub finished_books: usize,
    pub in_progress_books: usize,
    pub tag_distribution: BTreeMap<String, usize>,
    pub recently_read: Vec<Book>,
}

#[derive(Clone)]
pub struct BookService {
    book_repo: BookRepository,
    param_repo: ParamRepository,
    tag_repo: TagRepository,
    sync_repo: Option<SyncRepository>,
    client_id: String,
    metadata_service: MetadataService,
    plugin_service: PluginService,
    conversion_service: ConversionService,
    library_path: PathBuf,
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
            .map(|c| match c {
                '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
                _ => c,
            })
            .collect();
        let cleaned = sanitized.trim();
        if cleaned.is_empty() {
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

    pub fn import_book(&self, source_file: &Path) -> VellumResult<Book> {
        if !source_file.exists() {
            return Err(VellumError::NotFound(format!(
                "Fichier source introuvable: {}",
                source_file.display()
            )));
        }

        let ext = source_file
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if ext.is_empty() {
            return Err(VellumError::Unsupported(
                "Le fichier n'a pas d'extension valide".to_string(),
            ));
        }

        let meta = self.metadata_service.extract_metadata(source_file);

        let mut input_meta: HashMap<String, serde_json::Value> = HashMap::new();
        if let Some(ref t) = meta.title {
            input_meta.insert("title".to_string(), serde_json::json!(t));
        } else if let Some(stem) = source_file.file_stem().and_then(|s| s.to_str()) {
            input_meta.insert("title".to_string(), serde_json::json!(stem));
        }
        if let Some(ref a) = meta.author {
            input_meta.insert("author".to_string(), serde_json::json!(a));
        }
        if let Some(ref p) = meta.publisher {
            input_meta.insert("publisher".to_string(), serde_json::json!(p));
        }
        if let Some(ref d) = meta.publish_date {
            input_meta.insert("publishDate".to_string(), serde_json::json!(d));
        }
        if let Some(ref desc) = meta.description {
            input_meta.insert("description".to_string(), serde_json::json!(desc));
        }
        if let Some(ref isbn) = meta.isbn {
            input_meta.insert("isbn".to_string(), serde_json::json!(isbn));
        }
        input_meta.insert("tags".to_string(), serde_json::json!(meta.tags));

        let enriched = self
            .plugin_service
            .run_import_enrichers(Some(source_file), input_meta);

        let title = enriched
            .get("title")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .or(meta.title)
            .unwrap_or_else(|| {
                source_file
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Sans titre")
                    .to_string()
            });

        let author = enriched
            .get("author")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .or(meta.author)
            .unwrap_or_else(|| "Auteur inconnu".to_string());

        let publisher = enriched
            .get("publisher")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .or(meta.publisher);

        let publish_date = enriched
            .get("publishDate")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .or(meta.publish_date);

        let description = enriched
            .get("description")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .or(meta.description);

        let isbn = enriched
            .get("isbn")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .or(meta.isbn);

        let mut tags_list = meta.tags;
        if let Some(tags_val) = enriched.get("tags").and_then(|v| v.as_array()) {
            for tv in tags_val {
                if let Some(t_str) = tv.as_str().map(str::trim).filter(|s| !s.is_empty()) {
                    if !tags_list.iter().any(|t| t.eq_ignore_ascii_case(t_str)) {
                        tags_list.push(t_str.to_string());
                    }
                }
            }
        }

        let book_id = Uuid::new_v4().to_string();
        let clean_author = Self::sanitize_filename(&author);
        let clean_title = Self::sanitize_filename(&title);

        let book_dir_rel = PathBuf::from(&clean_author).join(format!("{clean_title} ({book_id})"));
        let book_dir_abs = self.library_path.join(&book_dir_rel);
        fs::create_dir_all(&book_dir_abs)?;

        let target_filename = format!("{clean_title} ({book_id}).{ext}");
        let target_filepath_abs = book_dir_abs.join(&target_filename);
        let target_filepath_rel = book_dir_rel.join(&target_filename);

        fs::copy(source_file, &target_filepath_abs)?;
        let file_size = fs::metadata(&target_filepath_abs)
            .map(|m| m.len())
            .unwrap_or(0);

        let mut cover_path_rel = None;
        if let Ok(Some(cover_data)) = self.metadata_service.extract_cover(source_file) {
            let cover_filename = "cover.jpg";
            let cover_file_abs = book_dir_abs.join(cover_filename);
            if fs::write(&cover_file_abs, cover_data).is_ok() {
                cover_path_rel = Some(
                    book_dir_rel
                        .join(cover_filename)
                        .to_string_lossy()
                        .to_string(),
                );
            }
        } else if let Some(b64) = enriched.get("coverData").and_then(|v| v.as_str()) {
            use base64::Engine;
            if let Ok(cover_bytes) = base64::engine::general_purpose::STANDARD.decode(b64) {
                if cover_bytes.len() > 500 {
                    let ext = enriched.get("coverExt").and_then(|v| v.as_str()).unwrap_or(".jpg");
                    let cover_filename = format!("cover{ext}");
                    let cover_file_abs = book_dir_abs.join(&cover_filename);
                    if fs::write(&cover_file_abs, cover_bytes).is_ok() {
                        cover_path_rel = Some(
                            book_dir_rel
                                .join(cover_filename)
                                .to_string_lossy()
                                .to_string(),
                        );
                    }
                }
            }
        }

        let mut book = Book::new(title, author);
        book.id = book_id.clone();
        book.publisher = publisher;
        book.publish_date = publish_date;
        book.isbn = isbn;
        book.series = meta.series;
        book.series_index = meta.series_index;
        book.description = description;
        book.cover_path = cover_path_rel;

        for tag_name in tags_list {
            let trimmed = tag_name.trim();
            if !trimmed.is_empty() {
                let tag = Tag::new(trimmed);
                let _ = self.tag_repo.save(&tag);
                book.add_tag(tag);
            }
        }

        let format = BookFormat::new(
            ext,
            target_filename,
            target_filepath_rel.to_string_lossy().to_string(),
            file_size,
        );
        book.add_format(format);

        let saved = self.book_repo.save(&book)?;

        if let Some(ref sr) = self.sync_repo {
            let event = vellum_domain::SyncEvent::new(
                &self.client_id,
                vellum_domain::SyncAction::BookCreated {
                    book: saved.clone(),
                    formats: saved.formats.clone(),
                    tags: saved.tags.clone(),
                },
            );
            let _ = sr.record_event(&event);
        }

        Ok(saved)
    }

    pub fn update_book(&self, updated: &Book) -> VellumResult<Book> {
        let existing = self
            .book_repo
            .find_by_id(&updated.id)?
            .ok_or_else(|| VellumError::NotFound(format!("Livre introuvable: {}", updated.id)))?;

        let old_author = existing.author.clone();
        let old_title = existing.title.clone();
        let author_changed = old_author != updated.author;
        let title_changed = old_title != updated.title;

        let mut book = updated.clone();

        if (author_changed || title_changed) && !book.formats.is_empty() {
            let first_fmt = &book.formats[0];
            let old_first_file = self.library_path.join(&first_fmt.filepath);
            if let Some(old_book_dir) = old_first_file.parent() {
                if old_book_dir.exists() {
                    let clean_author = Self::sanitize_filename(&book.author);
                    let clean_title = Self::sanitize_filename(&book.title);
                    let new_book_dir_rel = PathBuf::from(&clean_author)
                        .join(format!("{clean_title} ({})", book.id));
                    let new_book_dir_abs = self.library_path.join(&new_book_dir_rel);

                    if let Some(parent) = new_book_dir_abs.parent() {
                        let _ = fs::create_dir_all(parent);
                    }

                    if fs::rename(old_book_dir, &new_book_dir_abs).is_ok() {
                        for fmt in &mut book.formats {
                            let ext = Path::new(&fmt.filename)
                                .extension()
                                .and_then(|e| e.to_str())
                                .unwrap_or(&fmt.format);
                            let new_filename = format!("{clean_title} ({}).{ext}", book.id);
                            let old_file_path = new_book_dir_abs.join(&fmt.filename);
                            let new_file_path = new_book_dir_abs.join(&new_filename);

                            if old_file_path.exists() {
                                let _ = fs::rename(&old_file_path, &new_file_path);
                            }

                            fmt.filename = new_filename.clone();
                            fmt.filepath = new_book_dir_rel
                                .join(new_filename)
                                .to_string_lossy()
                                .to_string();
                        }

                        if let Some(ref cover) = book.cover_path {
                            let cover_filename = Path::new(cover)
                                .file_name()
                                .and_then(|f| f.to_str())
                                .unwrap_or("cover.jpg");
                            book.cover_path = Some(
                                new_book_dir_rel
                                    .join(cover_filename)
                                    .to_string_lossy()
                                    .to_string(),
                            );
                        }

                        if let Some(old_parent) = old_book_dir.parent() {
                            if let Ok(mut entries) = fs::read_dir(old_parent) {
                                if entries.next().is_none() {
                                    let _ = fs::remove_dir(old_parent);
                                }
                            }
                        }
                    }
                }
            }
        }

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

    pub fn delete_book(&self, book_id: &str, delete_files: bool) -> VellumResult<()> {
        let book_opt = self.book_repo.find_by_id(book_id)?;
        if let Some(book) = book_opt {
            if delete_files && !book.formats.is_empty() {
                let first_fmt = &book.formats[0];
                let file_path = self.library_path.join(&first_fmt.filepath);
                if let Some(book_dir) = file_path.parent() {
                    if book_dir.exists() {
                        let _ = fs::remove_dir_all(book_dir);
                        if let Some(author_dir) = book_dir.parent() {
                            if let Ok(mut entries) = fs::read_dir(author_dir) {
                                if entries.next().is_none() {
                                    let _ = fs::remove_dir(author_dir);
                                }
                            }
                        }
                    }
                }
            }
        }

        self.book_repo.delete(book_id)?;

        if let Some(ref sr) = self.sync_repo {
            let event = vellum_domain::SyncEvent::new(
                &self.client_id,
                vellum_domain::SyncAction::BookDeleted {
                    book_id: book_id.to_string(),
                },
            );
            let _ = sr.record_event(&event);
        }

        Ok(())
    }

    pub fn add_format_to_book(
        &self,
        book_id: &str,
        new_format_file: &Path,
    ) -> VellumResult<Book> {
        let mut book = self
            .book_repo
            .find_by_id(book_id)?
            .ok_or_else(|| VellumError::NotFound(format!("Livre introuvable: {book_id}")))?;

        if !new_format_file.exists() {
            return Err(VellumError::NotFound(format!(
                "Fichier introuvable: {}",
                new_format_file.display()
            )));
        }

        let ext = new_format_file
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if ext.is_empty() {
            return Err(VellumError::Unsupported(
                "Extension de fichier absente".to_string(),
            ));
        }

        let book_dir_abs = if !book.formats.is_empty() {
            let first = &book.formats[0];
            let first_file = self.library_path.join(&first.filepath);
            first_file
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| self.library_path.clone())
        } else {
            let clean_author = Self::sanitize_filename(&book.author);
            let clean_title = Self::sanitize_filename(&book.title);
            let dir = self
                .library_path
                .join(&clean_author)
                .join(format!("{clean_title} ({})", book.id));
            fs::create_dir_all(&dir)?;
            dir
        };

        let target_filename = format!("{}.{ext}", Self::sanitize_filename(&book.title));
        let dest_file_abs = book_dir_abs.join(&target_filename);
        fs::copy(new_format_file, &dest_file_abs)?;

        let dest_file_rel = dest_file_abs
            .strip_prefix(&self.library_path)
            .unwrap_or(&dest_file_abs)
            .to_string_lossy()
            .to_string();

        let file_size = fs::metadata(&dest_file_abs)
            .map(|m| m.len())
            .unwrap_or(0);

        book.formats.retain(|f| !f.format.eq_ignore_ascii_case(&ext));
        let fmt = BookFormat::new(ext, target_filename, dest_file_rel, file_size);
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

    pub fn remove_format_from_book(
        &self,
        book_id: &str,
        format_str: &str,
        delete_file: bool,
    ) -> VellumResult<Book> {
        let mut book = self
            .book_repo
            .find_by_id(book_id)?
            .ok_or_else(|| VellumError::NotFound(format!("Livre introuvable: {book_id}")))?;

        if let Some(pos) = book
            .formats
            .iter()
            .position(|f| f.format.eq_ignore_ascii_case(format_str))
        {
            let removed = book.formats.remove(pos);
            if delete_file {
                let file_path = self.library_path.join(&removed.filepath);
                if file_path.exists() {
                    let _ = fs::remove_file(file_path);
                }
            }
        }

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

    pub fn update_reading_progress(
        &self,
        book_id: &str,
        chapter: i32,
        total: i32,
        percent: f64,
        finished: bool,
    ) -> VellumResult<Book> {
        let mut book = self
            .book_repo
            .find_by_id(book_id)?
            .ok_or_else(|| VellumError::NotFound(format!("Livre introuvable: {book_id}")))?;

        book.current_chapter_index = chapter;
        book.total_chapters = total;
        book.progress_percent = percent.clamp(0.0, 100.0);
        book.is_finished = finished;
        book.last_read_at = Some(Utc::now().to_rfc3339());

        let saved = self.book_repo.save(&book)?;

        if let Some(ref sr) = self.sync_repo {
            let event = vellum_domain::SyncEvent::new(
                &self.client_id,
                vellum_domain::SyncAction::ProgressUpdated {
                    book_id: saved.id.clone(),
                    progress_percent: saved.progress_percent,
                    current_chapter_index: saved.current_chapter_index,
                    total_chapters: saved.total_chapters,
                    last_read_at: saved.last_read_at.clone().unwrap_or_default(),
                    is_finished: saved.is_finished,
                },
            );
            let _ = sr.record_event(&event);
        }

        Ok(saved)
    }

    pub fn get_reading_stats(&self) -> VellumResult<ReadingStats> {
        let all_books = self.book_repo.find_all()?;
        let total = all_books.len();
        let finished = all_books.iter().filter(|b| b.is_finished).count();
        let in_progress = all_books
            .iter()
            .filter(|b| b.progress_percent > 0.0 && !b.is_finished)
            .count();
        let tag_distribution = self.get_tags_with_counts()?;

        let mut recently_read: Vec<Book> = all_books
            .into_iter()
            .filter(|b| b.last_read_at.is_some())
            .collect();
        recently_read.sort_by(|a, b| b.last_read_at.cmp(&a.last_read_at));
        recently_read.truncate(5);

        Ok(ReadingStats {
            total_books: total,
            finished_books: finished,
            in_progress_books: in_progress,
            tag_distribution,
            recently_read,
        })
    }

    pub fn get_file_for_format(&self, format: &BookFormat) -> PathBuf {
        self.library_path.join(&format.filepath)
    }

    pub fn get_cover_file(&self, book: &Book) -> Option<PathBuf> {
        book.cover_path
            .as_ref()
            .map(|rel| self.library_path.join(rel))
            .filter(|p| p.exists())
    }

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

    pub fn list_all_series(&self) -> VellumResult<Vec<(String, usize)>> {
        self.book_repo.list_all_series()
    }

    pub fn bulk_update_books(
        &self,
        book_ids: &[String],
        author: Option<&str>,
        series: Option<&str>,
        auto_index_series: bool,
        add_tags: &[String],
        remove_tags: &[String],
    ) -> VellumResult<Vec<Book>> {
        let mut updated_list = Vec::new();
        for (idx, id) in book_ids.iter().enumerate() {
            if let Some(mut book) = self.book_repo.find_by_id(id)? {
                if let Some(a) = author {
                    if !a.trim().is_empty() {
                        book.author = a.trim().to_string();
                    }
                }
                if let Some(s) = series {
                    book.series = if s.trim().is_empty() { None } else { Some(s.trim().to_string()) };
                }
                if auto_index_series && book.series.is_some() {
                    book.series_index = Some((idx + 1) as f64);
                }
                for t in add_tags {
                    let clean = t.trim();
                    if !clean.is_empty() {
                        let tag = Tag::new(clean);
                        let _ = self.tag_repo.save(&tag);
                        book.add_tag(tag);
                    }
                }
                for r in remove_tags {
                    book.tags.retain(|t| !t.name.eq_ignore_ascii_case(r.trim()));
                }
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
                updated_list.push(saved);
            }
        }
        Ok(updated_list)
    }

    pub fn bulk_delete_books(&self, book_ids: &[String], delete_files: bool) -> VellumResult<()> {
        for id in book_ids {
            let _ = self.delete_book(id, delete_files);
        }
        Ok(())
    }

    pub fn update_book_metadata(
        &self,
        book_id: &str,
        title: &str,
        author: &str,
        publisher: Option<&str>,
        description: Option<&str>,
        tag_names: Vec<String>,
    ) -> VellumResult<Book> {
        self.update_book_metadata_full(
            book_id,
            title,
            author,
            publisher,
            description,
            None,
            None,
            None,
            None,
            tag_names,
        )
    }

    pub fn update_book_metadata_full(
        &self,
        book_id: &str,
        title: &str,
        author: &str,
        publisher: Option<&str>,
        description: Option<&str>,
        isbn: Option<&str>,
        publish_date: Option<&str>,
        series: Option<&str>,
        series_index: Option<f64>,
        tag_names: Vec<String>,
    ) -> VellumResult<Book> {
        let mut book = self
            .book_repo
            .find_by_id(book_id)?
            .ok_or_else(|| VellumError::NotFound(format!("Livre introuvable: {book_id}")))?;

        if !title.trim().is_empty() {
            book.title = title.trim().to_string();
        }
        if !author.trim().is_empty() {
            book.author = author.trim().to_string();
        }
        book.publisher = publisher.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        book.description = description.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        if let Some(i) = isbn {
            book.isbn = if i.trim().is_empty() { None } else { Some(i.trim().to_string()) };
        }
        if let Some(pd) = publish_date {
            book.publish_date = if pd.trim().is_empty() { None } else { Some(pd.trim().to_string()) };
        }
        if let Some(s) = series {
            book.series = if s.trim().is_empty() { None } else { Some(s.trim().to_string()) };
        }
        book.series_index = series_index;

        book.tags.clear();
        for t_name in tag_names {
            let clean = t_name.trim();
            if !clean.is_empty() {
                let tag = Tag::new(clean);
                let _ = self.tag_repo.save(&tag);
                book.add_tag(tag);
            }
        }

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

    pub fn enrich_book(&self, book_id: &str) -> VellumResult<Book> {
        let mut book = self
            .book_repo
            .find_by_id(book_id)?
            .ok_or_else(|| VellumError::NotFound(format!("Livre introuvable: {book_id}")))?;

        let mut input_meta: HashMap<String, serde_json::Value> = HashMap::new();
        input_meta.insert("title".to_string(), serde_json::json!(book.title));
        input_meta.insert("author".to_string(), serde_json::json!(book.author));
        if let Some(ref publ) = book.publisher {
            input_meta.insert("publisher".to_string(), serde_json::json!(publ));
        }
        if let Some(ref date) = book.publish_date {
            input_meta.insert("publishDate".to_string(), serde_json::json!(date));
        }
        if let Some(ref desc) = book.description {
            input_meta.insert("description".to_string(), serde_json::json!(desc));
        }
        if let Some(ref isbn) = book.isbn {
            input_meta.insert("isbn".to_string(), serde_json::json!(isbn));
        }
        let current_tags: Vec<String> = book.tags.iter().map(|t| t.name.clone()).collect();
        input_meta.insert("tags".to_string(), serde_json::json!(current_tags));

        let source_file = book.formats.first().map(|f| self.library_path.join(&f.filepath));

        let enriched = self
            .plugin_service
            .run_import_enrichers(source_file.as_deref(), input_meta);

        if let Some(t) = enriched.get("title").and_then(|v| v.as_str()) {
            if !t.trim().is_empty() {
                book.title = t.trim().to_string();
            }
        }
        if let Some(a) = enriched.get("author").and_then(|v| v.as_str()) {
            if !a.trim().is_empty() {
                book.author = a.trim().to_string();
            }
        }
        if let Some(p) = enriched.get("publisher").and_then(|v| v.as_str()) {
            if !p.trim().is_empty() {
                book.publisher = Some(p.trim().to_string());
            }
        }
        if let Some(d) = enriched.get("publishDate").and_then(|v| v.as_str()) {
            if !d.trim().is_empty() {
                book.publish_date = Some(d.trim().to_string());
            }
        }
        if let Some(desc) = enriched.get("description").and_then(|v| v.as_str()) {
            if !desc.trim().is_empty() {
                book.description = Some(desc.trim().to_string());
            }
        }
        if let Some(isbn_str) = enriched.get("isbn").and_then(|v| v.as_str()) {
            if !isbn_str.trim().is_empty() {
                book.isbn = Some(isbn_str.trim().to_string());
            }
        }

        if let Some(tags_val) = enriched.get("tags").and_then(|v| v.as_array()) {
            for tv in tags_val {
                if let Some(t_str) = tv.as_str().map(str::trim).filter(|s| !s.is_empty()) {
                    if !book.tags.iter().any(|t| t.name.eq_ignore_ascii_case(t_str)) {
                        let tag = Tag::new(t_str);
                        let _ = self.tag_repo.save(&tag);
                        book.add_tag(tag);
                    }
                }
            }
        }

        // Cover download if present in enriched metadata
        if let Some(b64) = enriched.get("coverData").and_then(|v| v.as_str()) {
            use base64::Engine;
            if let Ok(cover_bytes) = base64::engine::general_purpose::STANDARD.decode(b64) {
                if cover_bytes.len() > 500 {
                    let ext = enriched.get("coverExt").and_then(|v| v.as_str()).unwrap_or(".jpg");
                    let clean_author = Self::sanitize_filename(&book.author);
                    let clean_title = Self::sanitize_filename(&book.title);
                    let book_dir_rel = PathBuf::from(&clean_author).join(format!("{clean_title} ({})", book.id));
                    let book_dir_abs = self.library_path.join(&book_dir_rel);
                    let _ = fs::create_dir_all(&book_dir_abs);
                    let cover_filename = format!("cover{ext}");
                    let cover_abs = book_dir_abs.join(&cover_filename);
                    if fs::write(&cover_abs, cover_bytes).is_ok() {
                        book.cover_path = Some(book_dir_rel.join(&cover_filename).to_string_lossy().to_string());
                    }
                }
            }
        }

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

    pub fn update_book_cover(&self, book_id: &str, new_cover_path: &Path) -> VellumResult<Book> {
        let mut book = self
            .book_repo
            .find_by_id(book_id)?
            .ok_or_else(|| VellumError::NotFound(format!("Livre introuvable: {book_id}")))?;

        if !new_cover_path.exists() {
            return Err(VellumError::NotFound(format!(
                "Fichier image introuvable: {}",
                new_cover_path.display()
            )));
        }

        let book_dir_abs = if !book.formats.is_empty() {
            let first = &book.formats[0];
            let first_file = self.library_path.join(&first.filepath);
            first_file
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| self.library_path.clone())
        } else {
            self.library_path.clone()
        };

        let ext = new_cover_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("jpg")
            .to_lowercase();

        let cover_dest_abs = book_dir_abs.join(format!("cover.{ext}"));
        fs::copy(new_cover_path, &cover_dest_abs)?;

        let cover_dest_rel = cover_dest_abs
            .strip_prefix(&self.library_path)
            .unwrap_or(&cover_dest_abs)
            .to_string_lossy()
            .to_string();

        book.cover_path = Some(cover_dest_rel);
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

    pub fn translate_book(
        &self,
        book_id: &str,
        target_lang: &str,
        provider: &str,
        model: &str,
        base_url: Option<&str>,
        api_key: Option<&str>,
        bilingual_mode: bool,
    ) -> VellumResult<Book> {
        let book = self
            .book_repo
            .find_by_id(book_id)?
            .ok_or_else(|| VellumError::NotFound(format!("Livre introuvable: {book_id}")))?;

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
            target_lang
        );
        let new_book_dir = self
            .library_path
            .join(&clean_author)
            .join(format!("{clean_title} ({})", uuid::Uuid::new_v4()));
        fs::create_dir_all(&new_book_dir)?;

        let target_filename = format!("{clean_title}.epub");
        let dest_abs = new_book_dir.join(&target_filename);

        let mut config: HashMap<String, String> = HashMap::new();
        config.insert("targetLanguage".to_string(), target_lang.to_string());
        config.insert("provider".to_string(), provider.to_string());
        config.insert("model".to_string(), model.to_string());
        if let Some(u) = base_url {
            config.insert("baseUrl".to_string(), u.to_string());
        }
        if let Some(k) = api_key {
            config.insert("apiKey".to_string(), k.to_string());
        }
        config.insert("bilingualMode".to_string(), bilingual_mode.to_string());

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
        let tag_tr = vellum_domain::Tag::new(format!("Traduction {target_lang}"));
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
}
