//! Gestion des formats associés à un livre, progression de lecture et statistiques.

use chrono::Utc;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use vellum_domain::{Book, BookFormat, ReadingStats, VellumError, VellumResult};

use super::BookService;

impl BookService {
    pub fn add_format_to_book(&self, book_id: &str, new_format_file: &Path) -> VellumResult<Book> {
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

        let file_size = fs::metadata(&dest_file_abs).map(|m| m.len()).unwrap_or(0);

        book.formats
            .retain(|f| !f.format.eq_ignore_ascii_case(&ext));
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
