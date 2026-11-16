//! Règles d'automatisation appliquées lors de l'import d'un livre.

use regex::Regex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use vellum_domain::{Book, Tag};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportRuleTarget {
    Title,
    Author,
    Filename,
}

/// Règle automatique évaluée à l'import d'un ouvrage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportRule {
    pub id: String,
    pub name: String,
    pub target: ImportRuleTarget,
    pub match_regex: String,
    pub add_tags: Vec<String>,
    pub set_series: Option<String>,
    pub auto_polish: bool,
    pub enabled: bool,
}

impl ImportRule {
    pub fn new(
        name: impl Into<String>,
        target: ImportRuleTarget,
        match_regex: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            target,
            match_regex: match_regex.into(),
            add_tags: Vec::new(),
            set_series: None,
            auto_polish: false,
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportRuleOutcome {
    pub matched_rule_names: Vec<String>,
    pub should_polish: bool,
}

/// Évalue et applique une liste de règles d'import sur un livre.
pub fn apply_import_rules(
    book: &mut Book,
    filename: &str,
    rules: &[ImportRule],
) -> ImportRuleOutcome {
    let mut outcome = ImportRuleOutcome::default();

    for rule in rules.iter().filter(|r| r.enabled) {
        let Ok(re) = Regex::new(&rule.match_regex) else {
            continue;
        };
        let haystack = match rule.target {
            ImportRuleTarget::Title => book.title.as_str(),
            ImportRuleTarget::Author => book.author.as_str(),
            ImportRuleTarget::Filename => filename,
        };
        if !re.is_match(haystack) {
            continue;
        }

        outcome.matched_rule_names.push(rule.name.clone());
        for tag in &rule.add_tags {
            let trimmed = tag.trim();
            if !trimmed.is_empty() {
                book.add_tag(Tag::new(trimmed));
            }
        }
        if let Some(ref series) = rule.set_series {
            let trimmed = series.trim();
            if !trimmed.is_empty() && book.series.is_none() {
                book.series = Some(trimmed.to_string());
            }
        }
        if rule.auto_polish {
            outcome.should_polish = true;
        }
    }

    outcome
}
