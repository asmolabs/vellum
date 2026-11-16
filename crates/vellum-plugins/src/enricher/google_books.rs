use serde_json::Value;
use tracing::warn;

use super::helpers::{clean_tag, clean_term, is_generic_value, strip_html, url_encode};
use super::BookMetadataDto;

pub(super) fn query_google_books(
    client: &reqwest::blocking::Client,
    isbn: Option<&str>,
    title: Option<&str>,
    author: Option<&str>,
    language: &str,
    api_key: &str,
) -> Result<Option<BookMetadataDto>, String> {
    let clean_isbn = isbn
        .map(|s| {
            s.chars()
                .filter(|c| c.is_ascii_digit() || *c == 'X' || *c == 'x')
                .collect::<String>()
        })
        .unwrap_or_default();

    let query_param = if clean_isbn.len() == 10 || clean_isbn.len() == 13 {
        format!("isbn:{clean_isbn}")
    } else if let Some(t) = title.filter(|s| !s.trim().is_empty() && !is_generic_value(s)) {
        let clean_t = clean_term(t);
        if let Some(a) = author.filter(|s| !s.trim().is_empty() && !is_generic_value(s)) {
            format!("intitle:\"{}\"+inauthor:\"{}\"", clean_t, clean_term(a))
        } else {
            format!("intitle:\"{clean_t}\"")
        }
    } else {
        return Ok(None);
    };

    let execute_search = |q: &str| -> Result<Option<Value>, String> {
        let mut url = format!(
            "https://www.googleapis.com/books/v1/volumes?q={}&maxResults=5",
            url_encode(q)
        );
        if !language.is_empty() && !language.eq_ignore_ascii_case("all") {
            url.push_str(&format!("&langRestrict={}", url_encode(language)));
        }
        if !api_key.is_empty() {
            url.push_str(&format!("&key={}", url_encode(api_key)));
        }

        let resp = client
            .get(&url)
            .send()
            .map_err(|e| format!("HTTP request failed: {e}"))?;

        let status = resp.status().as_u16();
        if status == 429 || status == 403 {
            warn!("Google Books quota exceeded / restricted (HTTP {status})");
            return Ok(None);
        }
        if status != 200 {
            warn!("Google Books returned unexpected HTTP {status}");
            return Ok(None);
        }

        let body: Value = resp
            .json()
            .map_err(|e| format!("Failed to parse Google Books JSON: {e}"))?;
        Ok(Some(body))
    };

    let mut body = execute_search(&query_param)?;

    // Relaxed search if combined query yielded 0 results
    if let Some(ref b) = body {
        let items_len = b
            .get("items")
            .and_then(|v| v.as_array())
            .map(|a| a.len())
            .unwrap_or(0);
        if items_len == 0 && query_param.contains("+inauthor:") {
            if let Some(t) = title {
                let relaxed_query = format!("intitle:\"{}\"", clean_term(t));
                if let Ok(Some(rel_body)) = execute_search(&relaxed_query) {
                    body = Some(rel_body);
                }
            }
        }
    }

    let Some(root) = body else {
        return Ok(None);
    };

    let Some(items) = root.get("items").and_then(|v| v.as_array()) else {
        return Ok(None);
    };

    if items.is_empty() {
        return Ok(None);
    }

    let volume_info = &items[0]["volumeInfo"];
    Ok(Some(parse_google_volume_info(volume_info)))
}

pub(super) fn parse_google_volume_info(volume_info: &Value) -> BookMetadataDto {
    let raw_title = volume_info.get("title").and_then(|v| v.as_str());
    let subtitle = volume_info.get("subtitle").and_then(|v| v.as_str());
    let title = match (raw_title, subtitle) {
        (Some(t), Some(s)) if !s.trim().is_empty() => Some(format!("{t} : {s}")),
        (Some(t), _) => Some(t.to_string()),
        _ => None,
    };

    let authors: Vec<String> = volume_info
        .get("authors")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|a| a.as_str().map(str::trim))
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();

    let publisher = volume_info
        .get("publisher")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let published_date = volume_info
        .get("publishedDate")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let description = volume_info
        .get("description")
        .and_then(|v| v.as_str())
        .map(strip_html)
        .filter(|s| !s.trim().is_empty());

    let mut found_isbn = None;
    if let Some(id_list) = volume_info
        .get("industryIdentifiers")
        .and_then(|v| v.as_array())
    {
        for id_node in id_list {
            let id_type = id_node.get("type").and_then(|v| v.as_str()).unwrap_or("");
            let val = id_node
                .get("identifier")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            if id_type.eq_ignore_ascii_case("ISBN_13") {
                found_isbn = Some(val.to_string());
                break;
            } else if id_type.eq_ignore_ascii_case("ISBN_10") && found_isbn.is_none() {
                found_isbn = Some(val.to_string());
            }
        }
    }

    let mut categories = Vec::new();
    if let Some(cat_node) = volume_info.get("categories").and_then(|v| v.as_array()) {
        for c in cat_node {
            if let Some(text) = c.as_str() {
                for part in text.split(&['/', ';', ','][..]) {
                    let clean = clean_tag(part);
                    if !clean.is_empty() && !categories.iter().any(|existing| existing == &clean) {
                        categories.push(clean);
                    }
                }
            }
        }
    }

    let cover_url = volume_info
        .get("imageLinks")
        .and_then(|links| {
            links
                .get("extraLarge")
                .or_else(|| links.get("large"))
                .or_else(|| links.get("medium"))
                .or_else(|| links.get("thumbnail"))
                .or_else(|| links.get("smallThumbnail"))
        })
        .and_then(|v| v.as_str())
        .map(|u| u.replace("http://", "https://"));

    BookMetadataDto {
        title,
        authors,
        publisher,
        published_date,
        description,
        categories,
        cover_url,
        isbn: found_isbn,
    }
}
