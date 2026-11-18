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

    pub fn attach_format_to_book(&self, book_id: &str, source_file: &Path) -> VellumResult<Book> {
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

        let mut book = self
            .book_repo
            .find_by_id(book_id)?
            .ok_or_else(|| VellumError::NotFound(format!("Livre introuvable: {book_id}")))?;

        let book_dir_rel = book
            .formats
            .first()
            .and_then(|f| Path::new(&f.filepath).parent().map(Path::to_path_buf))
            .unwrap_or_else(|| {
                let clean_author = Self::sanitize_filename(&book.author);
                let clean_title = Self::sanitize_filename(&book.title);
                PathBuf::from(clean_author).join(format!("{clean_title} ({})", book.id))
            });
        let book_dir_abs = self.library_path.join(&book_dir_rel);
        fs::create_dir_all(&book_dir_abs)?;

        let clean_title = Self::sanitize_filename(&book.title);
        let has_same_ext = book
            .formats
            .iter()
            .any(|f| f.format.eq_ignore_ascii_case(&ext));
        let target_filename = if has_same_ext {
            source_file
                .file_name()
                .and_then(|n| n.to_str())
                .map(Self::sanitize_filename)
                .unwrap_or_else(|| format!("{clean_title} ({}).{ext}", Uuid::new_v4()))
        } else {
            format!("{clean_title} ({}).{ext}", book.id)
        };

        let target_abs = book_dir_abs.join(&target_filename);
        let target_rel = book_dir_rel.join(&target_filename);
        fs::copy(source_file, &target_abs)?;
        let file_size = fs::metadata(&target_abs).map(|m| m.len()).unwrap_or(0);

        book.add_format(BookFormat::new(
            ext,
            target_filename,
            target_rel.to_string_lossy().to_string(),
            file_size,
        ));

        let saved = self.book_repo.save(&book)?;
        Ok(saved)
    }

    pub fn import_folder(&self, folder: &Path) -> VellumResult<Vec<Book>> {
        if !folder.is_dir() {
            return Err(VellumError::NotFound(format!(
                "Dossier introuvable: {}",
                folder.display()
            )));
        }
        let mut entries: Vec<PathBuf> = fs::read_dir(folder)?
            .filter_map(|e| e.ok().map(|ent| ent.path()))
            .filter(|p| p.is_file())
            .collect();
        entries.sort();

        let audio_exts = ["m4b", "m4a", "mp3", "opus", "ogg", "flac", "aac", "wav"];
        let book_exts = [
            "epub", "pdf", "cbz", "cbr", "cb7", "mobi", "azw3", "txt", "fb2",
        ];

        let mut audio_files = Vec::new();
        let mut book_files = Vec::new();
        let mut cover_candidate = None;

        for p in entries {
            let fname_lower = p
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if matches!(
                fname_lower.as_str(),
                "cover.jpg" | "cover.jpeg" | "cover.png" | "cover.webp" | "folder.jpg"
            ) {
                if cover_candidate.is_none() {
                    cover_candidate = Some(p.clone());
                }
                continue;
            }
            let ext = p
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if audio_exts.contains(&ext.as_str()) {
                audio_files.push(p);
            } else if book_exts.contains(&ext.as_str()) {
                book_files.push(p);
            }
        }

        let folder_name = folder
            .file_name()
            .and_then(|n| n.to_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);

        // Dossier de livre audio multi-pistes (ex: plusieurs MP3/M4A d'un même livre audio)
        if !audio_files.is_empty() && book_files.is_empty() {
            let mut imported = self.import_book(&audio_files[0])?;
            if let Some(ref f_name) = folder_name {
                let t_low = imported.title.trim().to_ascii_lowercase();
                if t_low.starts_with("track")
                    || t_low.starts_with("cd")
                    || t_low.starts_with("part")
                    || t_low
                        .chars()
                        .all(|c| c.is_ascii_digit() || c == '-' || c == '_')
                {
                    imported.title = f_name.clone();
                    imported = self.book_repo.save(&imported)?;
                }
            }
            for track in audio_files.iter().skip(1) {
                if let Ok(updated) = self.attach_format_to_book(&imported.id, track) {
                    imported = updated;
                }
            }
            if imported.cover_path.is_none() {
                if let Some(ref cov) = cover_candidate {
                    if let Ok(updated) = self.update_book_cover(&imported.id, cov) {
                        imported = updated;
                    }
                }
            }
            return Ok(vec![imported]);
        }

        let mut results = Vec::new();
        for p in book_files.into_iter().chain(audio_files.into_iter()) {
            if let Ok(mut b) = self.import_book(&p) {
                let is_comic = b.formats.iter().any(|f| {
                    matches!(
                        f.format.to_ascii_lowercase().as_str(),
                        "cbz" | "cbr" | "cb7"
                    )
                });
                if is_comic && b.series.is_none() {
                    if let Some(ref f_name) = folder_name {
                        b.series = Some(f_name.clone());
                        if let Ok(saved) = self.book_repo.save(&b) {
                            b = saved;
                        }
                    }
                }
                results.push(b);
            }
        }
        Ok(results)
    }

    pub fn download_and_import_url(&self, url: &str, filename_hint: &str) -> VellumResult<Book> {
        let trimmed_url = url.trim();
        if !trimmed_url.starts_with("http://") && !trimmed_url.starts_with("https://") {
            return Err(VellumError::Unsupported(
                "L'URL doit commencer par http:// ou https://".to_string(),
            ));
        }
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .user_agent("Vellum/1.0")
            .build()
            .map_err(|e| {
                VellumError::Io(std::io::Error::other(format!(
                    "Client HTTP impossible à créer: {e}"
                )))
            })?;

        let resp = client.get(trimmed_url).send().map_err(|e| {
            VellumError::Io(std::io::Error::other(format!("Échec téléchargement: {e}")))
        })?;
        if !resp.status().is_success() {
            return Err(VellumError::Io(std::io::Error::other(format!(
                "Erreur HTTP {} lors du téléchargement",
                resp.status()
            ))));
        }
        let bytes = resp.bytes().map_err(|e| {
            VellumError::Io(std::io::Error::other(format!(
                "Lecture réponse échouée: {e}"
            )))
        })?;

        let clean_name = Self::sanitize_filename(filename_hint);
        let target_name = if clean_name.contains('.') {
            clean_name
        } else {
            let inferred_ext = trimmed_url
                .split('?')
                .next()
                .and_then(|p| p.rsplit('.').next())
                .filter(|e| {
                    matches!(
                        e.to_ascii_lowercase().as_str(),
                        "epub" | "pdf" | "cbz" | "cbr" | "m4b" | "mp3" | "m4a"
                    )
                })
                .unwrap_or("epub");
            format!("{clean_name}.{inferred_ext}")
        };

        let temp_dir = tempfile::tempdir()?;
        let temp_file = temp_dir.path().join(&target_name);
        fs::write(&temp_file, &bytes)?;
        self.import_book(&temp_file)
    }
}
