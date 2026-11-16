use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use regex::Regex;
use roxmltree::{Document, Node};
use vellum_domain::{EpubTocEntry, VellumResult};
use zip::ZipArchive;

use super::{escape_xml, normalize_zip_path, strip_html_tags, EpubEditor};

impl EpubEditor {
    pub(super) fn parse_ncx_navmap(doc: &Document, ncx_dir: &str) -> Vec<EpubTocEntry> {
        let mut entries = Vec::new();
        if let Some(nav_map) = doc.descendants().find(|n| n.has_tag_name("navMap")) {
            for child in nav_map.children().filter(|n| n.has_tag_name("navPoint")) {
                entries.push(Self::parse_ncx_navpoint(&child, ncx_dir));
            }
        }
        entries
    }

    fn parse_ncx_navpoint(node: &Node, ncx_dir: &str) -> EpubTocEntry {
        let id = node.attribute("id").unwrap_or("").to_string();
        let play_order = node
            .attribute("playOrder")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let label = node
            .descendants()
            .find(|n| n.has_tag_name("navLabel"))
            .and_then(|n| n.descendants().find(|t| t.has_tag_name("text")))
            .and_then(|n| n.text())
            .unwrap_or("Chapitre")
            .trim()
            .to_string();

        let raw_src = node
            .descendants()
            .find(|n| n.has_tag_name("content"))
            .and_then(|n| n.attribute("src"))
            .unwrap_or("")
            .trim();

        let (src_path, src_anchor) = if let Some(pos) = raw_src.find('#') {
            (&raw_src[..pos], &raw_src[pos..])
        } else {
            (raw_src, "")
        };

        let resolved_src = if ncx_dir.is_empty() || src_path.is_empty() {
            format!("{src_path}{src_anchor}")
        } else {
            format!("{}{src_anchor}", normalize_zip_path(ncx_dir, src_path))
        };

        let children = node
            .children()
            .filter(|n| n.has_tag_name("navPoint"))
            .map(|n| Self::parse_ncx_navpoint(&n, ncx_dir))
            .collect();

        EpubTocEntry {
            id,
            label,
            href: resolved_src,
            play_order,
            children,
        }
    }

    pub(super) fn parse_epub3_nav(doc: &Document, nav_dir: &str) -> Vec<EpubTocEntry> {
        let mut entries = Vec::new();
        let nav_node = doc.descendants().find(|n| {
            n.has_tag_name("nav")
                && (n.attribute("epub:type") == Some("toc")
                    || n.attribute("type") == Some("toc")
                    || n.has_tag_name("nav"))
        });

        if let Some(nav) = nav_node {
            if let Some(ol) = nav.descendants().find(|n| n.has_tag_name("ol")) {
                let mut play_order = 1;
                for li in ol.children().filter(|n| n.has_tag_name("li")) {
                    if let Some(entry) = Self::parse_epub3_li(&li, nav_dir, &mut play_order) {
                        entries.push(entry);
                    }
                }
            }
        }

        entries
    }

    fn parse_epub3_li(node: &Node, nav_dir: &str, play_order: &mut usize) -> Option<EpubTocEntry> {
        let a = node.descendants().find(|n| n.has_tag_name("a"))?;
        let raw_href = a.attribute("href").unwrap_or("");
        let label = a.text().unwrap_or("Chapitre").trim().to_string();

        let (src_path, src_anchor) = if let Some(pos) = raw_href.find('#') {
            (&raw_href[..pos], &raw_href[pos..])
        } else {
            (raw_href, "")
        };

        let href = if nav_dir.is_empty() || src_path.is_empty() {
            format!("{src_path}{src_anchor}")
        } else {
            format!("{}{src_anchor}", normalize_zip_path(nav_dir, src_path))
        };

        let current_order = *play_order;
        *play_order += 1;

        let mut children = Vec::new();
        if let Some(nested_ol) = node.children().find(|n| n.has_tag_name("ol")) {
            for child_li in nested_ol.children().filter(|n| n.has_tag_name("li")) {
                if let Some(child_entry) = Self::parse_epub3_li(&child_li, nav_dir, play_order) {
                    children.push(child_entry);
                }
            }
        }

        Some(EpubTocEntry {
            id: format!("toc-nav-{current_order}"),
            label,
            href,
            play_order: current_order,
            children,
        })
    }

    pub(super) fn generate_fallback_toc(
        zip: &mut ZipArchive<File>,
        opf_doc: &Document,
        opf_dir: &str,
    ) -> VellumResult<Vec<EpubTocEntry>> {
        let mut manifest: HashMap<String, String> = HashMap::new();
        for item in opf_doc.descendants().filter(|n| n.has_tag_name("item")) {
            let id = item.attribute("id").unwrap_or("");
            let href = item.attribute("href").unwrap_or("");
            if !id.is_empty() && !href.is_empty() {
                manifest.insert(id.to_string(), normalize_zip_path(opf_dir, href));
            }
        }

        let mut entries = Vec::new();
        let mut play_order = 1usize;

        if let Some(spine) = opf_doc.descendants().find(|n| n.has_tag_name("spine")) {
            for itemref in spine.descendants().filter(|n| n.has_tag_name("itemref")) {
                let idref = itemref.attribute("idref").unwrap_or("");
                if let Some(path) = manifest.get(idref) {
                    let mut title = Path::new(path)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Chapitre")
                        .to_string();

                    if let Ok(mut f) = zip.by_name(path) {
                        let mut html = String::new();
                        if f.read_to_string(&mut html).is_ok() {
                            static HEADING_RE: std::sync::LazyLock<Regex> =
                                std::sync::LazyLock::new(|| {
                                    Regex::new(r#"(?is)<h[1-2][^>]*>(.*?)</h[1-2]>"#)
                                        .expect("valid heading regex")
                                });
                            let re = &*HEADING_RE;
                            if let Some(caps) = re.captures(&html) {
                                let heading = strip_html_tags(&caps[1]);
                                if !heading.is_empty() {
                                    title = heading;
                                }
                            }
                        }
                    }

                    entries.push(EpubTocEntry {
                        id: format!("navpoint-{play_order}"),
                        label: title,
                        href: path.clone(),
                        play_order,
                        children: Vec::new(),
                    });
                    play_order += 1;
                }
            }
        }

        Ok(entries)
    }

    pub(super) fn render_ncx_xml(toc: &[EpubTocEntry], doc_title: &str) -> String {
        let mut xml = String::new();
        xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str("<ncx xmlns=\"http://www.daisy.org/z3986/2005/ncx/\" version=\"2005-1\">\n");
        xml.push_str("  <head>\n");
        xml.push_str("    <meta name=\"dtb:uid\" content=\"urn:uuid:vellum-toc\"/>\n");
        xml.push_str("    <meta name=\"dtb:depth\" content=\"3\"/>\n");
        xml.push_str("    <meta name=\"dtb:totalPageCount\" content=\"0\"/>\n");
        xml.push_str("    <meta name=\"dtb:maxPageNumber\" content=\"0\"/>\n");
        xml.push_str("  </head>\n");
        xml.push_str(&format!(
            "  <docTitle><text>{}</text></docTitle>\n",
            escape_xml(doc_title)
        ));
        xml.push_str("  <navMap>\n");

        for entry in toc {
            Self::render_ncx_navpoint(&mut xml, entry, 4);
        }

        xml.push_str("  </navMap>\n");
        xml.push_str("</ncx>\n");
        xml
    }

    fn render_ncx_navpoint(xml: &mut String, entry: &EpubTocEntry, indent: usize) {
        let pad = " ".repeat(indent);
        xml.push_str(&format!(
            "{pad}<navPoint id=\"{}\" playOrder=\"{}\">\n",
            escape_xml(&entry.id),
            entry.play_order
        ));
        xml.push_str(&format!(
            "{pad}  <navLabel><text>{}</text></navLabel>\n",
            escape_xml(&entry.label)
        ));
        xml.push_str(&format!(
            "{pad}  <content src=\"{}\"/>\n",
            escape_xml(&entry.href)
        ));

        for child in &entry.children {
            Self::render_ncx_navpoint(xml, child, indent + 2);
        }

        xml.push_str(&format!("{pad}</navPoint>\n"));
    }
}
