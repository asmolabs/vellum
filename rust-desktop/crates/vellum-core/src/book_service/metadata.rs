use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use vellum_domain::{Book, Tag, VellumError, VellumResult};

use super::{BookService, UpdateMetadataRequest};

impl BookService {
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
                    book.series = if s.trim().is_empty() {
                        None
                    } else {
                        Some(s.trim().to_string())
                    };
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
        self.update_book_metadata_full(UpdateMetadataRequest {
            book_id,
            title,
            author,
            publisher,
            description,
            tag_names,
            ..Default::default()
        })
    }

    pub fn update_book_metadata_full(&self, req: UpdateMetadataRequest<'_>) -> VellumResult<Book> {
        let mut book = self
            .book_repo
            .find_by_id(req.book_id)?
            .ok_or_else(|| VellumError::NotFound(format!("Livre introuvable: {}", req.book_id)))?;

        if !req.title.trim().is_empty() {
            book.title = req.title.trim().to_string();
        }
        if !req.author.trim().is_empty() {
            book.author = req.author.trim().to_string();
        }
        book.publisher = req
            .publisher
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        book.description = req
            .description
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        if let Some(i) = req.isbn {
            book.isbn = if i.trim().is_empty() {
                None
            } else {
                Some(i.trim().to_string())
            };
        }
        if let Some(pd) = req.publish_date {
            book.publish_date = if pd.trim().is_empty() {
                None
            } else {
                Some(pd.trim().to_string())
            };
        }
        if let Some(s) = req.series {
            book.series = if s.trim().is_empty() {
                None
            } else {
                Some(s.trim().to_string())
            };
        }
        book.series_index = req.series_index;

        book.tags.clear();
        for t_name in req.tag_names {
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

        let source_file = book
            .formats
            .first()
            .map(|f| self.library_path.join(&f.filepath));

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
                    let ext = enriched
                        .get("coverExt")
                        .and_then(|v| v.as_str())
                        .unwrap_or(".jpg");
                    let clean_author = Self::sanitize_filename(&book.author);
                    let clean_title = Self::sanitize_filename(&book.title);
                    let book_dir_rel =
                        PathBuf::from(&clean_author).join(format!("{clean_title} ({})", book.id));
                    let book_dir_abs = self.library_path.join(&book_dir_rel);
                    let _ = fs::create_dir_all(&book_dir_abs);
                    let cover_filename = format!("cover{ext}");
                    let cover_abs = book_dir_abs.join(&cover_filename);
                    if fs::write(&cover_abs, cover_bytes).is_ok() {
                        book.cover_path = Some(
                            book_dir_rel
                                .join(&cover_filename)
                                .to_string_lossy()
                                .to_string(),
                        );
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
}
