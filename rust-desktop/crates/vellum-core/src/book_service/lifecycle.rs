use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use chrono::Utc;
use uuid::Uuid;

use vellum_domain::{Book, BookFormat, ReadingStats, Tag, VellumError, VellumResult};

use super::BookService;

impl BookService {
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
        let favorite_books = all_books
            .iter()
            .filter(|b| {
                b.rating.unwrap_or(0) >= 4
                    || b.tags.iter().any(|t| {
                        let n = t.name.to_lowercase();
                        n.contains("coup de c") || n.contains("favori")
                    })
            })
            .count();

        let mut total_reading_hours = 0.0;
        let mut activity_by_day: HashMap<String, usize> = HashMap::new();

        for b in &all_books {
            let book_hours = if b.is_finished {
                6.0
            } else {
                (b.progress_percent / 100.0) * 6.0
            };
            total_reading_hours += book_hours;

            if let Some(ref date_str) = b.last_read_at {
                if date_str.len() >= 10 {
                    let day = &date_str[..10];
                    *activity_by_day.entry(day.to_string()).or_insert(0) += 1;
                }
            } else if b.added_at.len() >= 10 {
                let day = &b.added_at[..10];
                *activity_by_day.entry(day.to_string()).or_insert(0) += 1;
            }
        }

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
            favorite_books,
            total_reading_hours: (total_reading_hours * 10.0).round() / 10.0,
            tag_distribution,
            recently_read,
            activity_by_day,
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


}
