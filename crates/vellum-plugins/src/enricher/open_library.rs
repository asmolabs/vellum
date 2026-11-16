use serde_json::Value;

use super::helpers::{clean_tag, is_generic_value, url_encode};
use super::BookMetadataDto;

pub(super) fn query_open_library(
    client: &reqwest::blocking::Client,
    isbn: Option<&str>,
    title: Option<&str>,
    author: Option<&str>,
) -> Result<Option<BookMetadataDto>, String> {
    let clean_isbn = isbn
        .map(|s| {
            s.chars()
                .filter(|c| c.is_ascii_digit() || *c == 'X' || *c == 'x')
                .collect::<String>()
        })
        .unwrap_or_default();

    // Strategy A: By ISBN
    if clean_isbn.len() == 10 || clean_isbn.len() == 13 {
        let url = format!(
            "https://openlibrary.org/api/books?bibkeys=ISBN:{}&format=json&jscmd=data",
            clean_isbn
        );
        if let Ok(resp) = client.get(&url).send() {
            if resp.status().is_success() {
                if let Ok(root) = resp.json::<Value>() {
                    let key = format!("ISBN:{clean_isbn}");
                    if let Some(data) = root.get(&key) {
                        return Ok(Some(parse_open_library_data(data, &clean_isbn)));
                    }
                }
            }
        }
    }

    // Strategy B: By Title + Author search
    let Some(t) = title.filter(|s| !s.trim().is_empty() && !is_generic_value(s)) else {
        return Ok(None);
    };

    let mut search_url = format!(
        "https://openlibrary.org/search.json?title={}&limit=3",
        url_encode(t)
    );
    if let Some(a) = author.filter(|s| !s.trim().is_empty() && !is_generic_value(s)) {
        search_url.push_str(&format!("&author={}", url_encode(a)));
    }

    let resp = client
        .get(&search_url)
        .send()
        .map_err(|e| format!("Open Library search failed: {e}"))?;

    if !resp.status().is_success() {
        return Ok(None);
    }

    let root: Value = resp
        .json()
        .map_err(|e| format!("Failed to parse Open Library JSON: {e}"))?;

    let Some(docs) = root.get("docs").and_then(|v| v.as_array()) else {
        return Ok(None);
    };

    if docs.is_empty() {
        return Ok(None);
    }

    Ok(Some(parse_open_library_doc(&docs[0])))
}

pub(super) fn parse_open_library_data(data: &Value, fallback_isbn: &str) -> BookMetadataDto {
    let title = data.get("title").and_then(|v| v.as_str()).map(String::from);
    let authors: Vec<String> = data
        .get("authors")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|a| a.get("name").and_then(|n| n.as_str()).map(str::trim))
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();

    let publisher = data
        .get("publishers")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|p| p.get("name").and_then(|n| n.as_str()))
        .map(String::from);

    let published_date = data
        .get("publish_date")
        .and_then(|v| v.as_str())
        .map(String::from);

    let cover_url = data
        .get("cover")
        .and_then(|c| {
            c.get("large")
                .or_else(|| c.get("medium"))
                .or_else(|| c.get("small"))
        })
        .and_then(|v| v.as_str())
        .map(String::from);

    let mut categories = Vec::new();
    if let Some(subjects) = data.get("subjects").and_then(|v| v.as_array()) {
        for s in subjects {
            if let Some(name) = s.get("name").and_then(|n| n.as_str()) {
                let clean = clean_tag(name);
                if !clean.is_empty()
                    && categories.len() < 8
                    && !categories.iter().any(|c| c == &clean)
                {
                    categories.push(clean);
                }
            }
        }
    }

    BookMetadataDto {
        title,
        authors,
        publisher,
        published_date,
        description: None,
        categories,
        cover_url,
        isbn: Some(fallback_isbn.to_string()),
    }
}

pub(super) fn parse_open_library_doc(doc: &Value) -> BookMetadataDto {
    let title = doc.get("title").and_then(|v| v.as_str()).map(String::from);

    let authors: Vec<String> = doc
        .get("author_name")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|a| a.as_str().map(str::trim))
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();

    let publisher = doc
        .get("publisher")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.as_str())
        .map(String::from);

    let published_date = doc
        .get("first_publish_year")
        .and_then(|v| v.as_i64())
        .map(|y| y.to_string());

    let found_isbn = doc
        .get("isbn")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.as_str())
        .map(String::from);

    let mut categories = Vec::new();
    if let Some(subjects) = doc.get("subject").and_then(|v| v.as_array()) {
        for s in subjects {
            if let Some(name) = s.as_str() {
                let clean = clean_tag(name);
                if !clean.is_empty()
                    && categories.len() < 8
                    && !categories.iter().any(|c| c == &clean)
                {
                    categories.push(clean);
                }
            }
        }
    }

    let cover_url = doc
        .get("cover_i")
        .and_then(|v| v.as_i64())
        .filter(|id| *id > 0)
        .map(|id| format!("https://covers.openlibrary.org/b/id/{id}-L.jpg"));

    BookMetadataDto {
        title,
        authors,
        publisher,
        published_date,
        description: None,
        categories,
        cover_url,
        isbn: found_isbn,
    }
}
