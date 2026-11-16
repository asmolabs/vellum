use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use vellum_domain::{Book, BookFormat, Tag, VellumError, VellumResult};

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
                    let ext = enriched
                        .get("coverExt")
                        .and_then(|v| v.as_str())
                        .unwrap_or(".jpg");
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

        let rules = self.get_import_rules();
        let rule_outcome = if !rules.is_empty() {
            let fname = book
                .formats
                .first()
                .map(|f| f.filename.clone())
                .unwrap_or_default();
            let out = crate::import_rules::apply_import_rules(&mut book, &fname, &rules);
            for t in &book.tags {
                let _ = self.tag_repo.save(t);
            }
            out
        } else {
            Default::default()
        };

        let saved = self.book_repo.save(&book)?;
        let _ = self.ensure_book_thumbnail(&saved);
        if rule_outcome.should_polish {
            let _ = self.polish_book(&saved.id, crate::PolishOptions::default());
        }

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
                    let new_book_dir_rel =
                        PathBuf::from(&clean_author).join(format!("{clean_title} ({})", book.id));
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
        self.invalidate_book_thumbnail(book_id);

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
}
