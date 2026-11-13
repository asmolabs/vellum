use regex::Regex;
use serde::{Deserialize, Serialize};
use vellum_domain::{VellumError, VellumResult};

use super::BookService;

/// Champ cible d'une opération de recherche/remplacement par regex en masse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegexTargetField {
    Title,
    Author,
    Series,
    Publisher,
}

/// Requête de modification en masse par expression régulière.
#[derive(Debug, Clone)]
pub struct BulkRegexRequest {
    pub book_ids: Vec<String>,
    pub field: RegexTargetField,
    pub pattern: String,
    pub replacement: String,
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BulkRegexChange {
    pub book_id: String,
    pub before: String,
    pub after: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BulkRegexReport {
    pub matched_count: usize,
    pub updated_count: usize,
    pub changes: Vec<BulkRegexChange>,
}

impl BookService {
    /// Applique (ou prévisualise si `dry_run`) un rechercher/remplacer par expression régulière sur un lot de livres.
    pub fn bulk_regex_replace(&self, req: BulkRegexRequest) -> VellumResult<BulkRegexReport> {
        let re = Regex::new(&req.pattern).map_err(|e| {
            VellumError::InvalidArgument(format!("Expression régulière invalide : {e}"))
        })?;

        let mut report = BulkRegexReport::default();

        for book_id in &req.book_ids {
            let Some(mut book) = self.book_repo.find_by_id(book_id)? else {
                continue;
            };

            let before = match req.field {
                RegexTargetField::Title => book.title.clone(),
                RegexTargetField::Author => book.author.clone(),
                RegexTargetField::Series => book.series.clone().unwrap_or_default(),
                RegexTargetField::Publisher => book.publisher.clone().unwrap_or_default(),
            };

            if !re.is_match(&before) {
                continue;
            }

            let after = re.replace_all(&before, req.replacement.as_str()).to_string();
            if after == before {
                continue;
            }

            report.matched_count += 1;
            report.changes.push(BulkRegexChange {
                book_id: book.id.clone(),
                before,
                after: after.clone(),
            });

            if !req.dry_run {
                match req.field {
                    RegexTargetField::Title => book.title = after,
                    RegexTargetField::Author => book.author = after,
                    RegexTargetField::Series => {
                        book.series = if after.trim().is_empty() {
                            None
                        } else {
                            Some(after)
                        };
                    }
                    RegexTargetField::Publisher => {
                        book.publisher = if after.trim().is_empty() {
                            None
                        } else {
                            Some(after)
                        };
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
                report.updated_count += 1;
            }
        }

        Ok(report)
    }
}
