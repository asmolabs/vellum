//! Génération et cache disque de miniatures WebP pour les couvertures.

use std::fs;
use std::path::{Path, PathBuf};

use image::imageops::FilterType;
use image::{ImageFormat, ImageReader};
use vellum_domain::{Book, VellumError, VellumResult};

use super::BookService;

/// Largeur maximale d'une miniature WebP (adaptée à la grille et au carrousel @2x).
pub const THUMBNAIL_MAX_WIDTH: u32 = 300;
/// Hauteur maximale d'une miniature WebP.
pub const THUMBNAIL_MAX_HEIGHT: u32 = 450;

/// Retourne le répertoire de cache des miniatures associé à `library_path`.
pub fn thumbnail_dir_for_library(library_path: &Path) -> PathBuf {
    if library_path.file_name().and_then(|n| n.to_str()) == Some("library") {
        if let Some(parent) = library_path.parent() {
            return parent.join("thumbnails");
        }
    }
    library_path.join(".thumbnails")
}

/// Retourne le chemin attendu du fichier miniature WebP pour `book_id`.
pub fn thumbnail_path_for_book(library_path: &Path, book_id: &str) -> PathBuf {
    let safe_id = BookService::sanitize_filename(book_id);
    thumbnail_dir_for_library(library_path).join(format!("{safe_id}.webp"))
}

/// Génère une miniature WebP optimisée depuis `src_abs` vers `dest_webp` via écriture atomique.
pub fn generate_webp_thumbnail(src_abs: &Path, dest_webp: &Path) -> VellumResult<()> {
    if !src_abs.exists() {
        return Err(VellumError::NotFound(format!(
            "Couverture introuvable: {}",
            src_abs.display()
        )));
    }

    if let Some(parent) = dest_webp.parent() {
        fs::create_dir_all(parent)?;
    }

    let img = ImageReader::open(src_abs)?
        .with_guessed_format()?
        .decode()
        .map_err(|e| VellumError::InvalidArgument(format!("Image illisible: {e}")))?;

    let resized = if img.width() > THUMBNAIL_MAX_WIDTH || img.height() > THUMBNAIL_MAX_HEIGHT {
        img.resize(
            THUMBNAIL_MAX_WIDTH,
            THUMBNAIL_MAX_HEIGHT,
            FilterType::Triangle,
        )
    } else {
        img
    };

    let tmp_path = dest_webp.with_extension("webp.tmp");
    resized
        .save_with_format(&tmp_path, ImageFormat::WebP)
        .map_err(|e| VellumError::InvalidArgument(format!("Encodage WebP impossible: {e}")))?;
    fs::rename(&tmp_path, dest_webp)?;
    Ok(())
}

/// Assure qu'une miniature WebP à jour existe pour `cover_path` et retourne son chemin.
pub fn ensure_thumbnail_for_cover(
    library_path: &Path,
    book_id: &str,
    cover_path: &str,
) -> Option<PathBuf> {
    if cover_path.starts_with("data:") || cover_path.starts_with("http") {
        return None;
    }

    let cover_abs = if Path::new(cover_path).is_absolute() {
        PathBuf::from(cover_path)
    } else {
        library_path.join(cover_path)
    };
    if !cover_abs.exists() {
        return None;
    }

    let dest_webp = thumbnail_path_for_book(library_path, book_id);
    let is_fresh = match (fs::metadata(&dest_webp), fs::metadata(&cover_abs)) {
        (Ok(thumb_meta), Ok(src_meta)) if thumb_meta.len() > 0 => {
            match (thumb_meta.modified(), src_meta.modified()) {
                (Ok(t_mod), Ok(s_mod)) => t_mod >= s_mod,
                _ => true,
            }
        }
        _ => false,
    };

    if is_fresh {
        return Some(dest_webp);
    }

    match generate_webp_thumbnail(&cover_abs, &dest_webp) {
        Ok(()) => Some(dest_webp),
        Err(e) => {
            tracing::debug!(
                "Échec génération miniature WebP pour {}: {e}",
                cover_abs.display()
            );
            None
        }
    }
}

impl BookService {
    /// Assure l'existence d'une miniature WebP à jour pour `book` si sa couverture est locale.
    pub fn ensure_book_thumbnail(&self, book: &Book) -> Option<PathBuf> {
        let cover_path = book.cover_path.as_deref()?;
        ensure_thumbnail_for_cover(&self.library_path, &book.id, cover_path)
    }

    /// Supprime la miniature WebP en cache pour `book_id`.
    pub fn invalidate_book_thumbnail(&self, book_id: &str) {
        let thumb = thumbnail_path_for_book(&self.library_path, book_id);
        if thumb.exists() {
            let _ = fs::remove_file(thumb);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};
    use tempfile::tempdir;

    #[test]
    fn test_generate_and_cache_webp_thumbnail_resizes_large_image() {
        let dir = tempdir().unwrap();
        let lib_dir = dir.path().join("library");
        let book_dir = lib_dir.join("Author").join("Title");
        fs::create_dir_all(&book_dir).unwrap();

        let src_cover = book_dir.join("cover.png");
        let img = ImageBuffer::from_fn(900, 1400, |x, y| {
            Rgb([(x % 255) as u8, (y % 255) as u8, 128])
        });
        img.save(&src_cover).unwrap();

        let thumb = ensure_thumbnail_for_cover(&lib_dir, "book-123", "Author/Title/cover.png")
            .expect("thumbnail should be generated");
        assert!(thumb.exists());
        assert_eq!(thumb.extension().and_then(|e| e.to_str()), Some("webp"));
        assert_eq!(thumb, dir.path().join("thumbnails").join("book-123.webp"));

        let decoded = ImageReader::open(&thumb)
            .unwrap()
            .with_guessed_format()
            .unwrap()
            .decode()
            .unwrap();
        assert!(decoded.width() <= THUMBNAIL_MAX_WIDTH);
        assert!(decoded.height() <= THUMBNAIL_MAX_HEIGHT);
    }

    #[test]
    fn test_ensure_thumbnail_ignores_remote_and_data_uris() {
        let dir = tempdir().unwrap();
        assert!(
            ensure_thumbnail_for_cover(dir.path(), "b1", "https://example.com/c.jpg").is_none()
        );
        assert!(
            ensure_thumbnail_for_cover(dir.path(), "b1", "data:image/png;base64,AAA").is_none()
        );
    }
}
