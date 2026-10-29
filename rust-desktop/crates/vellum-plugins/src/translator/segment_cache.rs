use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use md5::{Digest, Md5};

pub struct TranslationSegmentCache {
    cache_dir: Option<PathBuf>,
    memory_cache: Mutex<HashMap<String, String>>,
    model_key: String,
    target_lang: String,
}

impl TranslationSegmentCache {
    pub fn new(cache_dir: Option<PathBuf>, model_key: &str, target_lang: &str) -> Self {
        if let Some(ref dir) = cache_dir {
            let _ = fs::create_dir_all(dir);
        }
        Self {
            cache_dir,
            memory_cache: Mutex::new(HashMap::new()),
            model_key: model_key.to_string(),
            target_lang: target_lang.to_string(),
        }
    }

    pub fn get(&self, source_text: &str) -> Option<String> {
        if source_text.trim().is_empty() {
            return None;
        }
        let hash = self.compute_key(source_text);
        if let Ok(guard) = self.memory_cache.lock() {
            if let Some(cached) = guard.get(&hash) {
                return Some(cached.clone());
            }
        }

        if let Some(ref dir) = self.cache_dir {
            let path = Self::file_for_hash(dir, &hash);
            if path.exists() {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(mut guard) = self.memory_cache.lock() {
                        guard.insert(hash, content.clone());
                    }
                    return Some(content);
                }
            }
        }
        None
    }

    pub fn put(&self, source_text: &str, translated_text: &str) {
        if source_text.trim().is_empty() || translated_text.trim().is_empty() {
            return;
        }
        let hash = self.compute_key(source_text);
        if let Ok(mut guard) = self.memory_cache.lock() {
            guard.insert(hash.clone(), translated_text.to_string());
        }
        if let Some(ref dir) = self.cache_dir {
            let path = Self::file_for_hash(dir, &hash);
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::write(path, translated_text);
        }
    }

    pub fn compute_key(&self, source_text: &str) -> String {
        let mut hasher = Md5::new();
        let payload = format!(
            "{}:{}:{}",
            self.model_key,
            self.target_lang,
            source_text.trim()
        );
        hasher.update(payload.as_bytes());
        let digest = hasher.finalize();
        format!("{digest:x}")
    }

    fn file_for_hash(base: &Path, hash: &str) -> PathBuf {
        let sub = if hash.len() >= 2 { &hash[..2] } else { "00" };
        base.join(sub).join(format!("{hash}.txt"))
    }
}
