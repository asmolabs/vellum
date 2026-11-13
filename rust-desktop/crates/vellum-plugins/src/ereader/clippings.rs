use chrono::{DateTime, NaiveDateTime, Utc};
use vellum_domain::DeviceAnnotation;

/// Parses Amazon Kindle "My Clippings.txt" file into structured `DeviceAnnotation` items.
/// Format of each clipping:
///
/// Book Title (Author Name)
/// - Your Highlight on page 12 | location 180-182 | Added on Tuesday, 10 March 2026 14:22:10
///
/// The highlighted text from the book.
/// ==========
pub fn parse_kindle_clippings(content: &str) -> Vec<DeviceAnnotation> {
    let mut annotations = Vec::new();

    // Standard Kindle separator is "=========="
    let blocks: Vec<&str> = content.split("==========").collect();

    for block in blocks {
        let lines: Vec<&str> = block
            .lines()
            .map(|l| l.trim_matches('\u{feff}').trim())
            .filter(|l| !l.is_empty())
            .collect();

        if lines.len() < 2 {
            continue;
        }

        // Line 1: Title and Author
        // e.g. "Dune (Frank Herbert)" or "Les Misérables"
        let title_line = lines[0];
        let (title, author) = parse_title_and_author(title_line);

        // Line 2: Metadata (Highlight/Note, page/location, timestamp)
        // e.g. "- Your Highlight on page 42 | location 120-125 | Added on Monday, 15 October 2026..."
        let meta_line = lines[1];
        let location = extract_location(meta_line);
        let created_at = extract_date(meta_line);

        // Lines 3+: Highlighted text or user note
        let text_body = if lines.len() > 2 {
            lines[2..].join("\n")
        } else {
            String::new()
        };

        if !text_body.is_empty() {
            let is_note = meta_line.to_lowercase().contains("note");
            annotations.push(DeviceAnnotation {
                book_title: title,
                book_author: author,
                text: if is_note { String::new() } else { text_body.clone() },
                note: if is_note { Some(text_body) } else { None },
                location,
                created_at,
            });
        }
    }

    annotations
}

fn parse_title_and_author(line: &str) -> (String, Option<String>) {
    if let Some(open_paren) = line.rfind('(') {
        if let Some(close_paren) = line.rfind(')') {
            if close_paren > open_paren {
                let title = line[..open_paren].trim().to_string();
                let author = line[open_paren + 1..close_paren].trim().to_string();
                return (title, if author.is_empty() { None } else { Some(author) });
            }
        }
    }
    (line.trim().to_string(), None)
}

fn extract_location(line: &str) -> Option<String> {
    for part in line.split('|') {
        let p_clean = part.trim();
        if p_clean.to_lowercase().contains("location")
            || p_clean.to_lowercase().contains("emplacement")
            || p_clean.to_lowercase().contains("page")
        {
            return Some(p_clean.trim_start_matches('-').trim().to_string());
        }
    }
    None
}

fn extract_date(line: &str) -> Option<DateTime<Utc>> {
    // Looks for "Added on ..." or "Ajouté le ..."
    let marker = if let Some(pos) = line.find("Added on ") {
        pos + "Added on ".len()
    } else {
        let pos = line.find("Ajouté le ")?;
        pos + "Ajouté le ".len()
    };

    let date_str = line[marker..].trim();
    // Try RFC3339 or common formats
    if let Ok(dt) = DateTime::parse_from_rfc3339(date_str) {
        return Some(dt.with_timezone(&Utc));
    }

    // Try common format: "Monday, October 15, 2026 2:30:15 PM"
    if let Ok(ndt) = NaiveDateTime::parse_from_str(date_str, "%A, %B %d, %Y %I:%M:%S %p") {
        return Some(DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_kindle_clippings() {
        let sample = r#"
Fondation (Isaac Asimov)
- Votre surlignement au format page 42 | emplacement 640-642 | Ajouté le lundi 15 octobre 2026 14:20:00

La violence est le dernier refuge de l'incompétence.
==========
1984 (George Orwell)
- Your Note on Location 123 | Added on Monday, October 15, 2026 2:30:15 PM

Une remarque essentielle.
==========
"#;

        let res = parse_kindle_clippings(sample);
        assert_eq!(res.len(), 2);
        assert_eq!(res[0].book_title, "Fondation");
        assert_eq!(res[0].book_author.as_deref(), Some("Isaac Asimov"));
        assert!(res[0].text.contains("violence est le dernier refuge"));
        assert!(res[0].note.is_none());

        assert_eq!(res[1].book_title, "1984");
        assert_eq!(res[1].book_author.as_deref(), Some("George Orwell"));
        assert_eq!(res[1].note.as_deref(), Some("Une remarque essentielle."));
    }
}
