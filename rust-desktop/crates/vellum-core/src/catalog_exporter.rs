//! Export de catalogues de la bibliothèque (CSV, JSON, BibTeX).

use std::fs;
use std::path::Path;

use vellum_domain::{Book, VellumResult};

/// Format d'export du catalogue de bibliothèque.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogFormat {
    Csv,
    Json,
    Bibtex,
}

/// Générateur de catalogues exportables (CSV RFC 4180, JSON, BibTeX).
pub struct CatalogExporter;

impl CatalogExporter {
    /// Sérialise une liste de livres dans le format demandé.
    pub fn export_to_string(books: &[Book], format: CatalogFormat) -> VellumResult<String> {
        match format {
            CatalogFormat::Csv => Ok(Self::to_csv(books)),
            CatalogFormat::Json => Ok(serde_json::to_string_pretty(books)?),
            CatalogFormat::Bibtex => Ok(Self::to_bibtex(books)),
        }
    }

    /// Exporte le catalogue directement vers un fichier sur disque.
    pub fn export_to_file(books: &[Book], format: CatalogFormat, dest: &Path) -> VellumResult<()> {
        let content = Self::export_to_string(books, format)?;
        if let Some(parent) = dest.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::write(dest, content)?;
        Ok(())
    }

    fn to_csv(books: &[Book]) -> String {
        let mut out = String::from(
            "id,title,author,publisher,publish_date,isbn,series,series_index,tags,formats,progress_percent,is_finished\n",
        );
        for b in books {
            let tags = b
                .tags
                .iter()
                .map(|t| t.name.as_str())
                .collect::<Vec<_>>()
                .join("; ");
            let formats = b
                .formats
                .iter()
                .map(|f| f.format.as_str())
                .collect::<Vec<_>>()
                .join("; ");
            let row = [
                csv_escape(&b.id),
                csv_escape(&b.title),
                csv_escape(&b.author),
                csv_escape(b.publisher.as_deref().unwrap_or("")),
                csv_escape(b.publish_date.as_deref().unwrap_or("")),
                csv_escape(b.isbn.as_deref().unwrap_or("")),
                csv_escape(b.series.as_deref().unwrap_or("")),
                b.series_index.map(|i| i.to_string()).unwrap_or_default(),
                csv_escape(&tags),
                csv_escape(&formats),
                format!("{:.1}", b.progress_percent),
                b.is_finished.to_string(),
            ]
            .join(",");
            out.push_str(&row);
            out.push('\n');
        }
        out
    }

    fn to_bibtex(books: &[Book]) -> String {
        let mut out = String::new();
        for b in books {
            let author_slug: String = b
                .author
                .chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .take(12)
                .collect();
            let id_prefix: String = b.id.chars().filter(|c| c.is_ascii_alphanumeric()).take(8).collect();
            let key = if author_slug.is_empty() {
                format!("book_{id_prefix}")
            } else {
                format!("{}_{}", author_slug.to_lowercase(), id_prefix)
            };

            out.push_str(&format!("@book{{{key},\n"));
            out.push_str(&format!("  title = {{{}}},\n", bib_escape(&b.title)));
            out.push_str(&format!("  author = {{{}}},\n", bib_escape(&b.author)));
            if let Some(ref pub_name) = b.publisher {
                out.push_str(&format!("  publisher = {{{}}},\n", bib_escape(pub_name)));
            }
            if let Some(ref year) = b.publish_date {
                out.push_str(&format!("  year = {{{}}},\n", bib_escape(year)));
            }
            if let Some(ref isbn) = b.isbn {
                out.push_str(&format!("  isbn = {{{}}},\n", bib_escape(isbn)));
            }
            if let Some(ref series) = b.series {
                out.push_str(&format!("  series = {{{}}},\n", bib_escape(series)));
            }
            out.push_str("}\n\n");
        }
        out
    }
}

fn csv_escape(val: &str) -> String {
    if val.contains(',') || val.contains('"') || val.contains('\n') || val.contains('\r') {
        format!("\"{}\"", val.replace('"', "\"\""))
    } else {
        val.to_string()
    }
}

fn bib_escape(val: &str) -> String {
    val.replace('\\', "\\\\")
        .replace('{', "\\{")
        .replace('}', "\\}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use vellum_domain::Tag;

    #[test]
    fn test_export_csv_json_bibtex() {
        let mut b = Book::new("Dune, Tome 1", "Frank Herbert");
        b.publisher = Some("Robert Laffont".to_string());
        b.publish_date = Some("1965".to_string());
        b.isbn = Some("9780441013593".to_string());
        b.add_tag(Tag::new("Sci-Fi"));

        let csv = CatalogExporter::export_to_string(&[b.clone()], CatalogFormat::Csv).unwrap();
        assert!(csv.contains("\"Dune, Tome 1\""));
        assert!(csv.contains("Frank Herbert"));
        assert!(csv.contains("Sci-Fi"));

        let json = CatalogExporter::export_to_string(&[b.clone()], CatalogFormat::Json).unwrap();
        assert!(json.contains("9780441013593"));

        let bib = CatalogExporter::export_to_string(&[b], CatalogFormat::Bibtex).unwrap();
        assert!(bib.contains("@book{frankherbert_"));
        assert!(bib.contains("title = {Dune, Tome 1}"));
        assert!(bib.contains("year = {1965}"));
    }
}
