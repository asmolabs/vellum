use std::collections::HashSet;

use regex::Regex;

use super::glossary_manager::GlossaryManager;

#[derive(Debug, Clone)]
pub struct QaIssue {
    pub paragraph_index: usize,
    pub category: String,
    pub message: String,
}

pub struct QualityAssuranceValidator;

impl QualityAssuranceValidator {
    pub fn validate_paragraph(
        paragraph_index: usize,
        source_marked: &str,
        translated_marked: &str,
        glossary: Option<&GlossaryManager>,
    ) -> Vec<QaIssue> {
        let mut issues = Vec::new();

        if source_marked.trim().is_empty() {
            return issues;
        }

        if translated_marked.trim().is_empty() {
            issues.push(QaIssue {
                paragraph_index,
                category: "EMPTY_TRANSLATION".to_string(),
                message: "La traduction est vide alors que la source contient du texte.".to_string(),
            });
            return issues;
        }

        // 1. Check inline markers {0}...{/0}
        let source_markers = Self::extract_markers(source_marked);
        let target_markers = Self::extract_markers(translated_marked);
        if source_markers != target_markers {
            issues.push(QaIssue {
                paragraph_index,
                category: "MARKER_MISMATCH".to_string(),
                message: format!(
                    "Les marqueurs inline diffèrent : attendu {:?}, obtenu {:?}",
                    source_markers, target_markers
                ),
            });
        }

        // 2. Check abnormal length ratio
        let src_len = source_marked.chars().count();
        let dst_len = translated_marked.chars().count();
        if src_len >= 40 {
            let ratio = dst_len as f64 / src_len as f64;
            if ratio < 0.35 {
                issues.push(QaIssue {
                    paragraph_index,
                    category: "TRUNCATED".to_string(),
                    message: format!("Traduction anormalement courte (ratio {ratio:.2})"),
                });
            } else if ratio > 2.8 {
                issues.push(QaIssue {
                    paragraph_index,
                    category: "HALLUCINATION_LENGTH".to_string(),
                    message: format!("Traduction anormalement longue (ratio {ratio:.2})"),
                });
            }
        }

        // 3. Check untranslated copy
        if src_len >= 50 && source_marked.trim() == translated_marked.trim() {
            issues.push(QaIssue {
                paragraph_index,
                category: "UNTRANSLATED".to_string(),
                message: "Le paragraphe semble non traduit (identique à la source).".to_string(),
            });
        }

        // 4. Glossary adherence check
        if let Some(g) = glossary {
            let src_lower = source_marked.to_lowercase();
            let dst_lower = translated_marked.to_lowercase();
            for (src_term, dst_term) in g.terms() {
                if src_term.len() >= 4
                    && src_lower.contains(&src_term.to_lowercase())
                    && !dst_lower.contains(&dst_term.to_lowercase())
                {
                    issues.push(QaIssue {
                        paragraph_index,
                        category: "GLOSSARY_MISSED".to_string(),
                        message: format!(
                            "Le terme '{src_term}' devrait être traduit par '{dst_term}'."
                        ),
                    });
                }
            }
        }

        issues
    }

    pub fn repair_missing_markers(source_marked: &str, translated_marked: &str) -> String {
        let source_markers = Self::extract_markers(source_marked);
        let target_markers = Self::extract_markers(translated_marked);
        if source_markers == target_markers {
            return translated_marked.to_string();
        }

        // Remove any broken or partial markers in translated_marked so restore_inline_tags remains valid HTML
        let marker_re = Regex::new(r"\{/?\d+\}").expect("valid regex");
        marker_re.replace_all(translated_marked, "").into_owned()
    }

    fn extract_markers(text: &str) -> HashSet<String> {
        let marker_re = Regex::new(r"\{/?\d+\}").expect("valid regex");
        marker_re
            .find_iter(text)
            .map(|m| m.as_str().to_string())
            .collect()
    }
}
