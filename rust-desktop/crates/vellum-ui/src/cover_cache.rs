//! Cache mémoire et disque (miniatures WebP) pour les couvertures de livres dans l'UI.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, RwLock};

use base64::Engine;
use vellum_core::book_service::ensure_thumbnail_for_cover;

const MAX_MEMORY_ENTRIES: usize = 2_000;

type CoverCacheKey = (String, String, bool);
type CoverCacheMap = HashMap<CoverCacheKey, Option<String>>;

static COVER_DATA_CACHE: LazyLock<RwLock<CoverCacheMap>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

fn default_library_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".vellum").join("library"))
}

fn encode_file_to_data_uri(path: &Path) -> Option<String> {
    if !path.exists() {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("jpeg")
        .to_lowercase();
    let mime = match ext.as_str() {
        "png" => "image/png",
        "webp" => "image/webp",
        _ => "image/jpeg",
    };
    Some(format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(&bytes)
    ))
}

/// Résout l'URI affichable d'une couverture (`prefer_thumbnail = true` utilise le cache WebP 300x450).
pub fn resolve_cover_src(
    book_id: &str,
    cover_path: Option<&str>,
    prefer_thumbnail: bool,
) -> Option<String> {
    let raw = cover_path?;
    if raw.starts_with("data:") || raw.starts_with("http") {
        return Some(raw.to_string());
    }

    let key = (book_id.to_string(), raw.to_string(), prefer_thumbnail);
    if let Ok(guard) = COVER_DATA_CACHE.read() {
        if let Some(cached) = guard.get(&key) {
            return cached.clone();
        }
    }

    let library_dir = default_library_dir()?;
    let target_file = if prefer_thumbnail {
        ensure_thumbnail_for_cover(&library_dir, book_id, raw).or_else(|| {
            if Path::new(raw).is_absolute() {
                Some(PathBuf::from(raw))
            } else {
                Some(library_dir.join(raw))
            }
        })
    } else if Path::new(raw).is_absolute() {
        Some(PathBuf::from(raw))
    } else {
        Some(library_dir.join(raw))
    };

    let resolved = target_file.as_deref().and_then(encode_file_to_data_uri);

    if let Ok(mut guard) = COVER_DATA_CACHE.write() {
        if guard.len() >= MAX_MEMORY_ENTRIES {
            guard.clear();
        }
        guard.insert(key, resolved.clone());
    }

    resolved
}

/// Invalide les entrées de cache mémoire liées à `book_id` (après changement de couverture ou suppression).
pub fn invalidate_book_cover(book_id: &str) {
    if let Ok(mut guard) = COVER_DATA_CACHE.write() {
        guard.retain(|(id, _, _), _| id != book_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_cover_src_passes_through_urls_and_data_uris() {
        let http = resolve_cover_src("b1", Some("https://example.com/cov.jpg"), true);
        assert_eq!(http.as_deref(), Some("https://example.com/cov.jpg"));

        let data = resolve_cover_src("b1", Some("data:image/webp;base64,AAAA"), true);
        assert_eq!(data.as_deref(), Some("data:image/webp;base64,AAAA"));

        assert!(resolve_cover_src("b1", None, true).is_none());
    }

    #[test]
    fn test_resolve_cover_src_caches_and_invalidates_local_file() {
        let dir =
            std::env::temp_dir().join(format!("vellum-cover-cache-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let cover_path = dir.join("cover.png");
        // Write a tiny valid 1x1 PNG
        let tiny_png: &[u8] = &[
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
            0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78,
            0x9C, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00,
            0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];
        std::fs::write(&cover_path, tiny_png).unwrap();

        let p_str = cover_path.to_string_lossy().to_string();
        let first = resolve_cover_src("test-cache-book", Some(&p_str), false).unwrap();
        assert!(first.starts_with("data:image/png;base64,"));

        // Remove file: cached value should still hit memory cache
        std::fs::remove_file(&cover_path).unwrap();
        let cached = resolve_cover_src("test-cache-book", Some(&p_str), false);
        assert_eq!(cached.as_deref(), Some(first.as_str()));

        // After invalidation, deleted file resolves to None
        invalidate_book_cover("test-cache-book");
        assert!(resolve_cover_src("test-cache-book", Some(&p_str), false).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
