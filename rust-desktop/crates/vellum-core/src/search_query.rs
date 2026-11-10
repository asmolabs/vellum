use vellum_domain::Book;

#[derive(Debug, PartialEq, Clone)]
pub enum SearchToken {
    Author(String, bool),
    Title(String, bool),
    Series(String, bool),
    Tag(String, bool),
    Format(String, bool),
    Status(String, bool),
    Text(String, bool),
}

impl SearchToken {
    pub fn matches(&self, book: &Book) -> bool {
        match self {
            SearchToken::Author(val, negated) => {
                let m = book.author.to_lowercase().contains(val);
                if *negated { !m } else { m }
            }
            SearchToken::Title(val, negated) => {
                let m = book.title.to_lowercase().contains(val);
                if *negated { !m } else { m }
            }
            SearchToken::Series(val, negated) => {
                let m = book
                    .series
                    .as_deref()
                    .map(|s| s.to_lowercase().contains(val))
                    .unwrap_or(false);
                if *negated { !m } else { m }
            }
            SearchToken::Tag(val, negated) => {
                let m = book
                    .tags
                    .iter()
                    .any(|t| t.name.to_lowercase().contains(val));
                if *negated { !m } else { m }
            }
            SearchToken::Format(val, negated) => {
                let m = book
                    .formats
                    .iter()
                    .any(|f| f.format.to_lowercase().contains(val));
                if *negated { !m } else { m }
            }
            SearchToken::Status(val, negated) => {
                let m = match val.as_str() {
                    "finished" | "lu" | "termine" | "terminé" => book.is_finished,
                    "reading" | "en cours" | "cours" => book.progress_percent > 0.0 && !book.is_finished,
                    "toread" | "a lire" | "à lire" | "unread" => book.progress_percent == 0.0 && !book.is_finished,
                    _ => true,
                };
                if *negated { !m } else { m }
            }
            SearchToken::Text(val, negated) => {
                let in_title = book.title.to_lowercase().contains(val);
                let in_author = book.author.to_lowercase().contains(val);
                let in_series = book
                    .series
                    .as_deref()
                    .map(|s| s.to_lowercase().contains(val))
                    .unwrap_or(false);
                let in_tags = book
                    .tags
                    .iter()
                    .any(|t| t.name.to_lowercase().contains(val));
                let m = in_title || in_author || in_series || in_tags;
                if *negated { !m } else { m }
            }
        }
    }
}

pub struct SearchQuery {
    tokens: Vec<SearchToken>,
}

impl SearchQuery {
    pub fn parse(input: &str) -> Self {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Self { tokens: Vec::new() };
        }

        let mut tokens = Vec::new();
        let raw_parts = split_query_parts(trimmed);

        let mut i = 0;
        while i < raw_parts.len() {
            let part = &raw_parts[i];
            let lower = part.to_lowercase();

            if lower == "not" && i + 1 < raw_parts.len() {
                // Next token is negated
                let next_part = &raw_parts[i + 1];
                if let Some(tok) = parse_single_token(next_part, true) {
                    tokens.push(tok);
                }
                i += 2;
                continue;
            }

            let negated = part.starts_with('-');
            let clean_part = if negated { &part[1..] } else { part.as_str() };

            if !clean_part.is_empty() {
                if let Some(tok) = parse_single_token(clean_part, negated) {
                    tokens.push(tok);
                }
            }
            i += 1;
        }

        Self { tokens }
    }

    pub fn matches(&self, book: &Book) -> bool {
        if self.tokens.is_empty() {
            return true;
        }
        self.tokens.iter().all(|t| t.matches(book))
    }

    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }
}

fn parse_single_token(s: &str, negated: bool) -> Option<SearchToken> {
    if let Some((prefix, val)) = s.split_once(':') {
        let clean_prefix = prefix.trim().to_lowercase();
        let clean_val = val.trim().trim_matches('"').to_lowercase();
        if clean_val.is_empty() {
            return None;
        }

        match clean_prefix.as_str() {
            "author" | "auteur" => Some(SearchToken::Author(clean_val, negated)),
            "title" | "titre" => Some(SearchToken::Title(clean_val, negated)),
            "series" | "serie" | "saga" => Some(SearchToken::Series(clean_val, negated)),
            "tag" | "tags" | "genre" => Some(SearchToken::Tag(clean_val, negated)),
            "format" | "fmt" => Some(SearchToken::Format(clean_val, negated)),
            "status" | "statut" => Some(SearchToken::Status(clean_val, negated)),
            _ => Some(SearchToken::Text(s.to_lowercase(), negated)),
        }
    } else {
        let clean_val = s.trim().trim_matches('"').to_lowercase();
        if clean_val.is_empty() {
            None
        } else {
            Some(SearchToken::Text(clean_val, negated))
        }
    }
}

/// Splits a query string into parts while preserving quoted values:
/// e.g. `author:"Frank Herbert" tag:SF not status:finished`
fn split_query_parts(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in s.chars() {
        if ch == '"' {
            in_quotes = !in_quotes;
            current.push(ch);
        } else if ch.is_whitespace() && !in_quotes {
            let t = current.trim();
            if !t.is_empty() {
                parts.push(t.to_string());
            }
            current.clear();
        } else {
            current.push(ch);
        }
    }

    let t = current.trim();
    if !t.is_empty() {
        parts.push(t.to_string());
    }

    parts
}

#[cfg(test)]
mod tests {
    use super::*;
    use vellum_domain::{BookFormat, Tag};

    fn sample_book() -> Book {
        let mut b = Book::new("Dune Messiah", "Frank Herbert");
        b.series = Some("Dune".into());
        b.series_index = Some(2.0);
        b.tags.push(Tag::new("Science Fiction"));
        b.tags.push(Tag::new("Classic"));
        b.formats.push(BookFormat::new("epub", "dune2.epub", "path/to/dune2.epub", 12345));
        b.progress_percent = 45.0;
        b.is_finished = false;
        b
    }

    #[test]
    fn test_search_free_text() {
        let b = sample_book();
        let q1 = SearchQuery::parse("messiah");
        assert!(q1.matches(&b));

        let q2 = SearchQuery::parse("asimov");
        assert!(!q2.matches(&b));
    }

    #[test]
    fn test_search_calibre_syntax() {
        let b = sample_book();

        let q1 = SearchQuery::parse("author:herbert series:dune");
        assert!(q1.matches(&b));

        let q2 = SearchQuery::parse("author:herbert tag:classic format:epub");
        assert!(q2.matches(&b));

        let q3 = SearchQuery::parse("author:herbert status:reading");
        assert!(q3.matches(&b));

        let q4 = SearchQuery::parse("author:herbert status:finished");
        assert!(!q4.matches(&b));
    }

    #[test]
    fn test_search_negation() {
        let b = sample_book();

        let q1 = SearchQuery::parse("series:dune not author:asimov");
        assert!(q1.matches(&b));

        let q2 = SearchQuery::parse("series:dune -author:herbert");
        assert!(!q2.matches(&b));
    }

    #[test]
    fn test_search_quoted_strings() {
        let b = sample_book();

        let q1 = SearchQuery::parse("author:\"Frank Herbert\" tag:\"Science Fiction\"");
        assert!(q1.matches(&b));

        let q2 = SearchQuery::parse("author:\"Isaac Asimov\"");
        assert!(!q2.matches(&b));
    }
}

