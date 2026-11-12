use std::path::Path;
use vellum_domain::{
    Book, DuplicatePair, IncompleteBookIssue, LibraryHealthReport, MissingFileIssue,
};

/// Normalise une chaîne de caractères pour la comparaison tolérante :
/// minuscules, suppression des accents basiques, suppression de la ponctuation
/// et élimination des mots de bruit (tome, vol, edition, collector, french, etc.).
pub fn normalize_string(input: &str) -> String {
    let lower = input.to_lowercase();
    let mut clean = String::with_capacity(lower.len());

    for ch in lower.chars() {
        match ch {
            'à' | 'â' | 'ä' => clean.push('a'),
            'é' | 'è' | 'ê' | 'ë' => clean.push('e'),
            'î' | 'ï' => clean.push('i'),
            'ô' | 'ö' => clean.push('o'),
            'ù' | 'û' | 'ü' => clean.push('u'),
            'ç' => clean.push('c'),
            'œ' => {
                clean.push('o');
                clean.push('e');
            }
            'æ' => {
                clean.push('a');
                clean.push('e');
            }
            c if c.is_alphanumeric() => clean.push(c),
            _ => clean.push(' '),
        }
    }

    // Liste des mots de bruit à ignorer pour les titres et éditions
    let noise_words: &[&str] = &[
        "tome", "tomes", "tom", "vol", "volume", "volumes", "livre", "book",
        "part", "partie", "edition", "editions", "ed", "collector", "french",
        "francais", "francaise", "english", "anglais", "anglaise", "version",
        "vf", "vost", "vostfr", "integrale", "integral", "complete", "serie",
        "the", "le", "la", "les", "un", "une", "des", "du", "de", "d", "l",
        "ebook", "epub", "pdf",
    ];

    let tokens: Vec<&str> = clean
        .split_whitespace()
        .filter(|w| !noise_words.contains(w) && w.len() > 1)
        .collect();

    tokens.join(" ")
}

/// Distance de Levenshtein optimisée en mémoire O(min(m, n)).
pub fn levenshtein(s1: &str, s2: &str) -> usize {
    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();

    let m = s1_chars.len();
    let n = s2_chars.len();

    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }

    let mut prev_row: Vec<usize> = (0..=n).collect();
    let mut curr_row: Vec<usize> = vec![0; n + 1];

    for i in 1..=m {
        curr_row[0] = i;
        for j in 1..=n {
            let cost = if s1_chars[i - 1] == s2_chars[j - 1] {
                0
            } else {
                1
            };
            curr_row[j] = (prev_row[j] + 1)
                .min(curr_row[j - 1] + 1)
                .min(prev_row[j - 1] + cost);
        }
        prev_row.copy_from_slice(&curr_row);
    }

    prev_row[n]
}

/// Ratio de similarité de Levenshtein entre 0.0 et 1.0.
pub fn levenshtein_ratio(s1: &str, s2: &str) -> f64 {
    if s1 == s2 {
        return 1.0;
    }
    let max_len = s1.chars().count().max(s2.chars().count());
    if max_len == 0 {
        return 1.0;
    }
    let dist = levenshtein(s1, s2);
    1.0 - (dist as f64 / max_len as f64)
}

/// Token Sort Ratio : découpe les mots, les trie alphabétiquement et compare.
/// Utile pour comparer "Frank Herbert" et "Herbert, Frank".
pub fn token_sort_ratio(s1: &str, s2: &str) -> f64 {
    let mut words1: Vec<&str> = s1.split_whitespace().collect();
    let mut words2: Vec<&str> = s2.split_whitespace().collect();

    words1.sort_unstable();
    words2.sort_unstable();

    let sorted1 = words1.join(" ");
    let sorted2 = words2.join(" ");

    levenshtein_ratio(&sorted1, &sorted2)
}

/// Token Set Ratio : intersection des mots pour détecter les inclusions
/// (ex: "1984" inclus dans "1984 Nouvelle Édition").
pub fn token_set_ratio(s1: &str, s2: &str) -> f64 {
    let set1: std::collections::BTreeSet<&str> = s1.split_whitespace().collect();
    let set2: std::collections::BTreeSet<&str> = s2.split_whitespace().collect();

    if set1.is_empty() || set2.is_empty() {
        return 0.0;
    }

    let intersection: Vec<&str> = set1.intersection(&set2).copied().collect();
    if intersection.is_empty() {
        return 0.0;
    }

    let min_len = set1.len().min(set2.len());
    let match_ratio = intersection.len() as f64 / min_len as f64;

    let diff1: Vec<&str> = set1.difference(&set2).copied().collect();
    let diff2: Vec<&str> = set2.difference(&set1).copied().collect();

    let str_inter = intersection.join(" ");
    let str_diff = [diff1, diff2].concat().join(" ");

    let combined_full = if str_diff.is_empty() {
        str_inter.clone()
    } else {
        format!("{str_inter} {str_diff}")
    };

    levenshtein_ratio(&str_inter, &combined_full).max(match_ratio * 0.9)
}

/// Calcule la similarité entre deux livres.
/// Renvoie (score_entre_0_et_1, raison).
pub fn compute_similarity(b1: &Book, b2: &Book) -> (f64, String) {
    if b1.id == b2.id {
        return (0.0, String::new());
    }

    // 1. Détection stricte par ISBN si renseigné des deux côtés
    if let (Some(isbn1), Some(isbn2)) = (&b1.isbn, &b2.isbn) {
        let clean1: String = isbn1.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
        let clean2: String = isbn2.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
        if clean1.len() >= 10 && clean1 == clean2 {
            return (1.0, "Code ISBN strictement identique".to_string());
        }
    }

    // 2. Normalisation du titre et de l'auteur
    let t1 = normalize_string(&b1.title);
    let t2 = normalize_string(&b2.title);

    let a1 = normalize_string(&b1.author);
    let a2 = normalize_string(&b2.author);

    // Titres originaux normalisés
    let raw_t1 = b1.title.trim().to_lowercase();
    let raw_t2 = b2.title.trim().to_lowercase();

    // Ratios sur les titres
    let title_ratio = levenshtein_ratio(&t1, &t2)
        .max(token_sort_ratio(&t1, &t2))
        .max(token_set_ratio(&t1, &t2))
        .max(levenshtein_ratio(&raw_t1, &raw_t2));

    // Ratios sur les auteurs
    let author_ratio = if a1.is_empty() || a2.is_empty() {
        0.5
    } else {
        levenshtein_ratio(&a1, &a2).max(token_sort_ratio(&a1, &a2))
    };

    // Cas 1 : Titres et auteurs très proches
    if title_ratio >= 0.85 && author_ratio >= 0.65 {
        let combined = title_ratio * 0.7 + author_ratio * 0.3;
        let pct = (combined * 100.0).round();
        return (
            combined,
            format!("Titre et auteur quasi-identiques ({pct}%)"),
        );
    }

    // Cas 2 : Titres identiques à 95%+ même si auteur imprécis
    if title_ratio >= 0.94 {
        let combined = title_ratio * 0.85 + author_ratio * 0.15;
        let pct = (combined * 100.0).round();
        return (
            combined,
            format!("Titres extrêmement similaires ({pct}%)"),
        );
    }

    (0.0, String::new())
}

/// Vérifie si l'auteur est considéré comme manquant ou générique
pub fn is_unknown_author(author: &str) -> bool {
    let clean = author.trim().to_lowercase();
    clean.is_empty()
        || clean == "unknown"
        || clean == "inconnu"
        || clean == "auteur inconnu"
        || clean == "anonyme"
        || clean == "various"
        || clean == "n/a"
}

/// Analyse complète de l'état de santé de la bibliothèque.
pub fn audit_library(books: &[Book], library_path: &Path) -> LibraryHealthReport {
    let total_books = books.len();
    let mut total_formats = 0;

    let mut duplicate_pairs = Vec::new();
    let mut incomplete_issues = Vec::new();
    let mut missing_file_issues = Vec::new();

    // 1. Détection des doublons (paires uniques)
    for i in 0..books.len() {
        for j in (i + 1)..books.len() {
            let b1 = &books[i];
            let b2 = &books[j];

            let (score, reason) = compute_similarity(b1, b2);
            if score >= 0.80 {
                duplicate_pairs.push(DuplicatePair {
                    primary_book: b1.clone(),
                    duplicate_book: b2.clone(),
                    similarity_score: score,
                    reason,
                });
            }
        }
    }

    // Trier les doublons par similarité décroissante
    duplicate_pairs.sort_by(|a, b| {
        b.similarity_score
            .partial_cmp(&a.similarity_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // 2. Détection des fiches incomplètes et fichiers manquants
    let mut clean_books_count = 0;

    for book in books {
        total_formats += book.formats.len();

        let missing_cover = book.cover_path.is_none() || {
            if let Some(ref cp) = book.cover_path {
                !library_path.join(cp).exists() && !Path::new(cp).exists()
            } else {
                true
            }
        };

        let missing_author = is_unknown_author(&book.author);
        let missing_description = book
            .description
            .as_deref()
            .map(|d| d.trim().is_empty())
            .unwrap_or(true);
        let missing_isbn = book
            .isbn
            .as_deref()
            .map(|i| i.trim().is_empty())
            .unwrap_or(true);
        let missing_tags = book.tags.is_empty();

        let has_incomplete = missing_cover
            || missing_author
            || missing_description
            || missing_isbn
            || missing_tags;

        if has_incomplete {
            incomplete_issues.push(IncompleteBookIssue {
                book: book.clone(),
                missing_cover,
                missing_author,
                missing_description,
                missing_isbn,
                missing_tags,
            });
        }

        // Vérification des fichiers de formats sur disque
        let mut book_has_missing_file = false;
        for fmt in &book.formats {
            let file_exists = library_path.join(&fmt.filepath).exists()
                || Path::new(&fmt.filepath).exists();

            if !file_exists {
                book_has_missing_file = true;
                missing_file_issues.push(MissingFileIssue {
                    book_id: book.id.clone(),
                    book_title: book.title.clone(),
                    format: fmt.format.clone(),
                    expected_path: fmt.filepath.clone(),
                });
            }
        }

        // Livre considéré comme "parfaitement sain"
        let is_in_dup = duplicate_pairs
            .iter()
            .any(|d| d.primary_book.id == book.id || d.duplicate_book.id == book.id);

        if !has_incomplete && !book_has_missing_file && !is_in_dup {
            clean_books_count += 1;
        }
    }

    // 3. Calcul du score de santé global (0 à 100%)
    let overall_score = if total_books == 0 {
        100
    } else {
        let mut score: f64 = 100.0;

        // Pénalité par doublon (jusqu'à -25 points)
        let dup_penalty = (duplicate_pairs.len() as f64 * 5.0).min(25.0);
        score -= dup_penalty;

        // Pénalité couvertures manquantes (jusqu'à -25 points)
        let missing_covers_count = incomplete_issues.iter().filter(|i| i.missing_cover).count();
        let cover_penalty = (missing_covers_count as f64 / total_books as f64) * 25.0;
        score -= cover_penalty;

        // Pénalité auteurs manquants/inconnus (jusqu'à -20 points)
        let missing_authors_count = incomplete_issues.iter().filter(|i| i.missing_author).count();
        let author_penalty = (missing_authors_count as f64 / total_books as f64) * 20.0;
        score -= author_penalty;

        // Pénalité descriptions manquantes (jusqu'à -10 points)
        let missing_desc_count = incomplete_issues.iter().filter(|i| i.missing_description).count();
        let desc_penalty = (missing_desc_count as f64 / total_books as f64) * 10.0;
        score -= desc_penalty;

        // Pénalité fichiers introuvables (jusqu'à -20 points)
        if total_formats > 0 {
            let files_penalty =
                (missing_file_issues.len() as f64 / total_formats as f64) * 20.0;
            score -= files_penalty;
        }

        score.round().clamp(0.0, 100.0) as u32
    };

    LibraryHealthReport {
        overall_score,
        total_books,
        total_formats,
        duplicate_pairs,
        incomplete_issues,
        missing_file_issues,
        clean_books_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenshtein_and_ratios() {
        assert_eq!(levenshtein("chat", "chat"), 0);
        assert_eq!(levenshtein("chat", "chats"), 1);
        assert_eq!(levenshtein("dune", "done"), 1);

        let ratio = levenshtein_ratio("dune", "dune");
        assert!((ratio - 1.0).abs() < f64::EPSILON);

        // Token sort ratio test: "Frank Herbert" vs "Herbert Frank"
        let r = token_sort_ratio("frank herbert", "herbert frank");
        assert!((r - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_normalize_string_strips_noise() {
        assert_eq!(
            normalize_string("Dune - Tome 1 (Édition Collector) [French]"),
            "dune"
        );
        assert_eq!(
            normalize_string("Le Seigneur des Anneaux - Intégrale"),
            "seigneur anneaux"
        );
    }

    #[test]
    fn test_duplicate_detection() {
        let b1 = Book::new("Dune", "Frank Herbert");
        let mut b2 = Book::new("Dune - Tome 1 (Collector)", "Herbert Frank");
        b2.id = "other-id".to_string();

        let (score, reason) = compute_similarity(&b1, &b2);
        assert!(score >= 0.85, "Score was {score}");
        assert!(!reason.is_empty());
    }

    #[test]
    fn test_audit_library_health_report() {
        let tmp = std::env::temp_dir().join(format!("vellum_test_audit_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);

        let mut b1 = Book::new("1984", "George Orwell");
        b1.description = Some("Un grand roman dystopique.".to_string());
        b1.isbn = Some("9780451524935".to_string());
        b1.tags = vec![vellum_domain::Tag::new("SF")];

        // b2 is duplicate of b1
        let mut b2 = Book::new("1984 [Édition Française]", "George Orwell");
        b2.id = "b2-id".to_string();

        let report = audit_library(&[b1, b2], &tmp);
        assert_eq!(report.total_books, 2);
        assert_eq!(report.duplicate_pairs.len(), 1);
        assert!(report.overall_score < 100);

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
