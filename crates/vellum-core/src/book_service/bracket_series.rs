use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use vellum_domain::VellumResult;

use super::BookService;

static RE_INSIDE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\[(?P<inside>[^\]]+)\]").expect("valid regex")
});

static RE_LANG_INDEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?P<series>.+?)[- _]+(?P<lang>EN|FR|VO|US|UK|ES|DE|IT|NL|PT|RU|JA|ZH)[- _]+T?(?P<idx>\d+(?:\.\d+)?)(?P<subidx>[a-zA-Z])?$").expect("valid regex")
});

static RE_INDEX_ONLY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?P<series>.+?)[- _]+T?(?P<idx>\d+(?:\.\d+)?)(?P<subidx>[a-zA-Z])?$").expect("valid regex")
});

/// Candidat à l'extraction de série depuis un titre entre crochets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BracketSeriesCandidate {
    pub book_id: String,
    pub title: String,
    pub series: String,
    pub series_index: Option<f64>,
    pub language: Option<String>,
}

/// Rapport d'analyse de la bibliothèque pour les séries entre crochets.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BracketSeriesScanReport {
    pub total_books: usize,
    pub matched_count: usize,
    pub candidates: Vec<BracketSeriesCandidate>,
}

/// Analyse un titre de livre pour en extraire la série, le numéro de tome et la langue sans toucher au titre.
pub fn parse_bracket_series(title: &str) -> Option<(String, Option<f64>, Option<String>)> {
    let cap_inside = RE_INSIDE.captures(title.trim())?;
    let inside = cap_inside.name("inside")?.as_str().trim();
    if inside.is_empty() || inside.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    // 1. Motif avec code langue explicite (ex: [Ben Hope-En-07] ou [Avatar - EN - 01])
    if let Some(caps) = RE_LANG_INDEX.captures(inside) {
        let series = caps.name("series")?.as_str().trim_matches(&[' ', '-', '_'] as &[_]).to_string();
        let lang = caps.name("lang").map(|m| m.as_str().to_uppercase());
        let idx_str = caps.name("idx")?.as_str();
        let mut idx: f64 = idx_str.parse().ok()?;
        if let Some(sub) = caps.name("subidx") {
            let ch = sub.as_str().chars().next().unwrap_or('a').to_ascii_lowercase();
            idx += ((ch as u8).saturating_sub(b'a') + 1) as f64 * 0.1;
        }
        return Some((series, Some(idx), lang));
    }

    // 2. Motif avec série et numéro de tome (ex: [Le Virus Morningstar-03] ou [Gotrek et Felix-07])
    if let Some(caps) = RE_INDEX_ONLY.captures(inside) {
        let series = caps.name("series")?.as_str().trim_matches(&[' ', '-', '_'] as &[_]).to_string();
        let idx_str = caps.name("idx")?.as_str();
        let mut idx: f64 = idx_str.parse().ok()?;
        if let Some(sub) = caps.name("subidx") {
            let ch = sub.as_str().chars().next().unwrap_or('a').to_ascii_lowercase();
            idx += ((ch as u8).saturating_sub(b'a') + 1) as f64 * 0.1;
        }
        return Some((series, Some(idx), None));
    }

    // 3. Motif sans numéro de tome (ex: [Warhammer] ...)
    let clean = inside.trim_matches(&[' ', '-', '_'] as &[_]).to_string();
    if !clean.is_empty() {
        return Some((clean, None, None));
    }

    None
}

impl BookService {
    /// Analyse l'ensemble de la bibliothèque pour détecter tous les livres contenant un motif de série entre crochets.
    pub fn scan_bracket_series(&self) -> VellumResult<BracketSeriesScanReport> {
        let all_books = self.book_repo.find_all()?;
        let total = all_books.len();
        let mut candidates = Vec::new();

        for b in &all_books {
            if let Some((series, idx, lang)) = parse_bracket_series(&b.title) {
                candidates.push(BracketSeriesCandidate {
                    book_id: b.id.clone(),
                    title: b.title.clone(),
                    series,
                    series_index: idx,
                    language: lang,
                });
            }
        }

        Ok(BracketSeriesScanReport {
            total_books: total,
            matched_count: candidates.len(),
            candidates,
        })
    }

    /// Applique les séries et tomes déduits sur les livres sélectionnés, SANS modifier les titres.
    pub fn apply_bracket_series(&self, candidates: &[BracketSeriesCandidate]) -> VellumResult<usize> {
        let updates: Vec<(String, String, Option<f64>)> = candidates
            .iter()
            .map(|c| (c.book_id.clone(), c.series.clone(), c.series_index))
            .collect();
        self.book_repo.update_series_batch(&updates)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bracket_series() {
        // Avec langue et tome
        let res = parse_bracket_series("[Ben Hope-En-07] The Sacred Sword").unwrap();
        assert_eq!(res.0, "Ben Hope");
        assert_eq!(res.1, Some(7.0));
        assert_eq!(res.2.as_deref(), Some("EN"));

        // Avec tome seul
        let res2 = parse_bracket_series("[Le Virus Morningstar-03] Survivants").unwrap();
        assert_eq!(res2.0, "Le Virus Morningstar");
        assert_eq!(res2.1, Some(3.0));
        assert_eq!(res2.2, None);

        // Sans tome
        let res3 = parse_bracket_series("[Warhammer] Wulfrik").unwrap();
        assert_eq!(res3.0, "Warhammer");
        assert_eq!(res3.1, None);
        assert_eq!(res3.2, None);

        // Pas de crochets
        assert!(parse_bracket_series("Fondation").is_none());
    }
}
