use crate::entity::Book;
use std::path::PathBuf;

pub const DEFAULT_NAMING_TEMPLATE: &str =
    "{author}/{if_series:{series}/T.{series_index:02} - }{title}";

/// Evaluates a book against a naming template and produces a relative PathBuf.
/// Supported tokens:
/// - `{title}`
/// - `{author}`
/// - `{series}`
/// - `{series_index}`
/// - `{series_index:02}` (padded to 2 digits)
/// - `{series_index:03}` (padded to 3 digits)
/// - `{isbn}`
/// - `{publisher}`
/// - `{if_series:content}` (evaluated only if series is present and non-empty)
pub struct NamingTemplate;

impl NamingTemplate {
    pub fn format_filename(book: &Book, template: &str, extension: &str) -> PathBuf {
        let tmpl = if template.trim().is_empty() {
            DEFAULT_NAMING_TEMPLATE
        } else {
            template.trim()
        };

        let evaluated = Self::evaluate_template(book, tmpl);
        let ext = extension.trim_start_matches('.').to_lowercase();

        // Split into directory components and filename
        let parts: Vec<&str> = evaluated
            .split(['/', '\\'])
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();

        if parts.is_empty() {
            let safe_title = Self::sanitize_component(&book.title);
            return PathBuf::from(format!("{safe_title}.{ext}"));
        }

        let mut path = PathBuf::new();
        for (i, part) in parts.iter().enumerate() {
            let safe_part = Self::sanitize_component(part);
            if i == parts.len() - 1 {
                let filename = if safe_part.to_lowercase().ends_with(&format!(".{ext}")) {
                    safe_part
                } else {
                    format!("{safe_part}.{ext}")
                };
                path.push(filename);
            } else {
                path.push(safe_part);
            }
        }

        path
    }

    fn evaluate_template(book: &Book, template: &str) -> String {
        let processed = Self::process_conditionals(book, template);

        let has_series = book
            .series
            .as_deref()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        let series_val = if has_series {
            Self::sanitize_component(book.series.as_deref().unwrap_or("").trim())
        } else {
            String::new()
        };

        let (idx_standard, idx_02, idx_03) = if let Some(idx) = book.series_index {
            let standard = if idx.fract() == 0.0 {
                format!("{}", idx as i64)
            } else {
                format!("{idx}")
            };
            let p02 = if idx.fract() == 0.0 {
                format!("{:02}", idx as i64)
            } else {
                format!("{idx:04.1}")
            };
            let p03 = if idx.fract() == 0.0 {
                format!("{:03}", idx as i64)
            } else {
                format!("{idx:05.1}")
            };
            (standard, p02, p03)
        } else {
            (String::new(), String::new(), String::new())
        };

        let author_val = if book.author.trim().is_empty() {
            "Inconnu".to_string()
        } else {
            Self::sanitize_component(book.author.trim())
        };

        let title_val = if book.title.trim().is_empty() {
            "SansTitre".to_string()
        } else {
            Self::sanitize_component(book.title.trim())
        };

        let isbn_val = book
            .isbn
            .as_deref()
            .map(|s| Self::sanitize_component(s.trim()))
            .unwrap_or_default();
        let publisher_val = book
            .publisher
            .as_deref()
            .map(|s| Self::sanitize_component(s.trim()))
            .unwrap_or_default();

        processed
            .replace("{title}", &title_val)
            .replace("{author}", &author_val)
            .replace("{series}", &series_val)
            .replace("{series_index:03}", &idx_03)
            .replace("{series_index:02}", &idx_02)
            .replace("{series_index}", &idx_standard)
            .replace("{isbn}", &isbn_val)
            .replace("{publisher}", &publisher_val)
    }

    fn process_conditionals(book: &Book, template: &str) -> String {
        let has_series = book
            .series
            .as_deref()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);

        let mut result = String::with_capacity(template.len());
        let mut rest = template;

        while let Some(start_idx) = rest.find("{if_series:") {
            result.push_str(&rest[..start_idx]);
            let after_prefix = &rest[start_idx + "{if_series:".len()..];

            // Match closing brace with depth tracking to allow nested {series}, {series_index}, etc.
            let mut depth = 1;
            let mut match_end = None;
            for (idx, ch) in after_prefix.char_indices() {
                if ch == '{' {
                    depth += 1;
                } else if ch == '}' {
                    depth -= 1;
                    if depth == 0 {
                        match_end = Some(idx);
                        break;
                    }
                }
            }

            if let Some(end_idx) = match_end {
                let inner_content = &after_prefix[..end_idx];
                if has_series {
                    result.push_str(inner_content);
                }
                rest = &after_prefix[end_idx + 1..];
            } else {
                result.push_str(&rest[start_idx..]);
                rest = "";
                break;
            }
        }

        result.push_str(rest);
        result
    }

    fn sanitize_component(s: &str) -> String {
        let cleaned: String = s
            .chars()
            .map(|c| match c {
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
                _ => c,
            })
            .collect();
        let trimmed = cleaned.trim().trim_matches('.');
        if trimmed.is_empty() {
            "Inconnu".to_string()
        } else {
            trimmed.to_string()
        }
    }
}
