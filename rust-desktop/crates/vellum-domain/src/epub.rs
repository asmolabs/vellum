use serde::{Deserialize, Serialize};

/// Represents an entry in the Table of Contents (TOC) of an EPUB.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpubTocEntry {
    pub id: String,
    pub label: String,
    pub href: String,
    pub play_order: usize,
    pub children: Vec<EpubTocEntry>,
}

impl EpubTocEntry {
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        href: impl Into<String>,
        play_order: usize,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            href: href.into(),
            play_order,
            children: Vec::new(),
        }
    }
}

/// Severity level of a detected EPUB issue
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EpubIssueSeverity {
    Error,
    Warning,
    Info,
}

/// A diagnostic finding in an EPUB structure
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpubIssue {
    pub severity: EpubIssueSeverity,
    pub code: String,
    pub message: String,
    pub file: Option<String>,
}

/// Structural diagnostics and quality report for an EPUB archive
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpubDiagnostics {
    pub epub_version: String,
    pub total_files: usize,
    pub total_size_bytes: u64,
    pub manifest_count: usize,
    pub spine_count: usize,
    pub toc_entries_count: usize,
    pub has_cover: bool,
    pub issues: Vec<EpubIssue>,
    pub is_valid: bool,
}
