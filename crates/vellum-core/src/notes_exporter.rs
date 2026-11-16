//! Export des notes, surlignages et signets au format Markdown (compatible Obsidian / Notion).

use std::fs;
use std::path::Path;

use vellum_domain::{Book, VellumResult};

use crate::BookUserData;

/// Exportateur de notes et annotations de lecture au format Markdown.
pub struct NotesExporter;

impl NotesExporter {
    /// Génère un document Markdown structuré avec frontmatter YAML, surlignages et signets.
    pub fn to_markdown(book: &Book, user_data: &BookUserData) -> String {
        let mut out = String::new();
        out.push_str("---\n");
        out.push_str(&format!("title: \"{}\"\n", escape_yaml(&book.title)));
        out.push_str(&format!("author: \"{}\"\n", escape_yaml(&book.author)));
        if let Some(ref s) = book.series {
            out.push_str(&format!("series: \"{}\"\n", escape_yaml(s)));
        }
        if let Some(ref isbn) = book.isbn {
            out.push_str(&format!("isbn: \"{}\"\n", escape_yaml(isbn)));
        }
        if !book.tags.is_empty() {
            let tags_yaml: Vec<String> = book
                .tags
                .iter()
                .map(|t| format!("\"{}\"", escape_yaml(&t.name)))
                .collect();
            out.push_str(&format!("tags: [{}]\n", tags_yaml.join(", ")));
        }
        out.push_str(&format!("progress: {:.1}%\n", book.progress_percent));
        out.push_str("---\n\n");

        out.push_str(&format!("# {} — {}\n\n", book.title, book.author));

        if !user_data.highlights.is_empty() {
            out.push_str("## Surlignages & Notes\n\n");
            for h in &user_data.highlights {
                out.push_str(&format!(
                    "- **{}** *(couleur : {}, {})*\n",
                    h.chapter_title, h.color, h.created_at
                ));
                for line in h.selected_text.lines() {
                    out.push_str(&format!("  > {}\n", line));
                }
                if let Some(ref note) = h.note {
                    let trimmed = note.trim();
                    if !trimmed.is_empty() {
                        out.push_str(&format!("  - 📝 *Note :* {}\n", trimmed));
                    }
                }
                out.push('\n');
            }
        }

        if !user_data.bookmarks.is_empty() {
            out.push_str("## Signets\n\n");
            for bm in &user_data.bookmarks {
                out.push_str(&format!(
                    "- **{}** (page {}, {:.1}%) — \"{}\"\n",
                    bm.chapter_title,
                    bm.page_in_chapter + 1,
                    bm.progress_percent,
                    bm.preview.trim()
                ));
            }
            out.push('\n');
        }

        if user_data.highlights.is_empty() && user_data.bookmarks.is_empty() {
            out.push_str("*Aucun surlignage ni signet enregistré pour cet ouvrage.*\n");
        }

        out
    }

    /// Exporte les notes Markdown vers un fichier sur disque.
    pub fn export_to_file(book: &Book, user_data: &BookUserData, dest: &Path) -> VellumResult<()> {
        let md = Self::to_markdown(book, user_data);
        if let Some(parent) = dest.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::write(dest, md)?;
        Ok(())
    }
}

fn escape_yaml(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
