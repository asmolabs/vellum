use regex::Regex;
use roxmltree::Document;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use tempfile::NamedTempFile;
use vellum_domain::{EpubTocEntry, VellumError, VellumResult};
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

use super::{map_zip, normalize_zip_path, strip_html_tags, EpubEditor};

impl EpubEditor {
    pub fn extract_toc(epub_path: &Path) -> VellumResult<Vec<EpubTocEntry>> {
        let file = File::open(epub_path)?;
        let mut zip = ZipArchive::new(file)
            .map_err(|e| VellumError::InvalidArgument(format!("Archive zip invalide: {e}")))?;

        let opf_path = Self::find_opf_path(&mut zip)?;
        let opf_dir = Path::new(&opf_path)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("")
            .to_string();

        let mut opf_xml = String::new();
        {
            let mut f = zip
                .by_name(&opf_path)
                .map_err(|_| VellumError::NotFound(format!("OPF introuvable: {opf_path}")))?;
            f.read_to_string(&mut opf_xml)?;
        }

        let opf_doc = Document::parse(&opf_xml)
            .map_err(|e| VellumError::InvalidArgument(format!("Erreur parsing OPF: {e}")))?;

        // Try to locate NCX
        let mut ncx_path = None;
        if let Some(spine) = opf_doc.descendants().find(|n| n.has_tag_name("spine")) {
            if let Some(toc_id) = spine.attribute("toc") {
                for item in opf_doc.descendants().filter(|n| n.has_tag_name("item")) {
                    if item.attribute("id") == Some(toc_id) {
                        if let Some(href) = item.attribute("href") {
                            ncx_path = Some(normalize_zip_path(&opf_dir, href));
                            break;
                        }
                    }
                }
            }
        }

        if ncx_path.is_none() {
            for item in opf_doc.descendants().filter(|n| n.has_tag_name("item")) {
                let media_type = item.attribute("media-type").unwrap_or("");
                let href = item.attribute("href").unwrap_or("");
                if media_type == "application/x-dtbncx+xml" || href.ends_with(".ncx") {
                    ncx_path = Some(normalize_zip_path(&opf_dir, href));
                    break;
                }
            }
        }

        // 1. Try parsing NCX
        if let Some(ncx_full_path) = &ncx_path {
            if let Ok(mut f) = zip.by_name(ncx_full_path) {
                let mut ncx_xml = String::new();
                if f.read_to_string(&mut ncx_xml).is_ok() {
                    if let Ok(ncx_doc) = Document::parse(&ncx_xml) {
                        let ncx_dir = Path::new(ncx_full_path)
                            .parent()
                            .and_then(|p| p.to_str())
                            .unwrap_or("");
                        let entries = Self::parse_ncx_navmap(&ncx_doc, ncx_dir);
                        if !entries.is_empty() {
                            return Ok(entries);
                        }
                    }
                }
            }
        }

        // 2. Try parsing EPUB 3 Nav
        for item in opf_doc.descendants().filter(|n| n.has_tag_name("item")) {
            if item.attribute("properties").unwrap_or("").contains("nav") {
                if let Some(href) = item.attribute("href") {
                    let nav_full_path = normalize_zip_path(&opf_dir, href);
                    if let Ok(mut f) = zip.by_name(&nav_full_path) {
                        let mut nav_xml = String::new();
                        if f.read_to_string(&mut nav_xml).is_ok() {
                            if let Ok(nav_doc) = Document::parse(&nav_xml) {
                                let nav_dir = Path::new(&nav_full_path)
                                    .parent()
                                    .and_then(|p| p.to_str())
                                    .unwrap_or("");
                                let entries = Self::parse_epub3_nav(&nav_doc, nav_dir);
                                if !entries.is_empty() {
                                    return Ok(entries);
                                }
                            }
                        }
                    }
                }
            }
        }

        // 3. Fallback: generate TOC from spine chapters
        Self::generate_fallback_toc(&mut zip, &opf_doc, &opf_dir)
    }

    /// Rewrites and updates the Table of Contents (TOC) inside an EPUB
    pub fn save_toc(epub_path: &Path, new_toc: &[EpubTocEntry]) -> VellumResult<()> {
        let file = File::open(epub_path)?;
        let mut zip_in = ZipArchive::new(file)
            .map_err(|e| VellumError::InvalidArgument(format!("Archive zip invalide: {e}")))?;

        let opf_path = Self::find_opf_path(&mut zip_in)?;
        let opf_dir = Path::new(&opf_path)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("")
            .to_string();

        let mut opf_xml = String::new();
        {
            let mut f = zip_in
                .by_name(&opf_path)
                .map_err(|_| VellumError::NotFound(format!("OPF introuvable: {opf_path}")))?;
            f.read_to_string(&mut opf_xml)?;
        }

        // Determine NCX path
        let mut ncx_target_path = None;
        if let Ok(opf_doc) = Document::parse(&opf_xml) {
            for item in opf_doc.descendants().filter(|n| n.has_tag_name("item")) {
                let media_type = item.attribute("media-type").unwrap_or("");
                let href = item.attribute("href").unwrap_or("");
                if media_type == "application/x-dtbncx+xml" || href.ends_with(".ncx") {
                    ncx_target_path = Some(normalize_zip_path(&opf_dir, href));
                    break;
                }
            }
        }

        let ncx_full_path = ncx_target_path.unwrap_or_else(|| {
            if opf_dir.is_empty() {
                "toc.ncx".to_string()
            } else {
                format!("{opf_dir}/toc.ncx")
            }
        });

        // Generate NCX XML
        let ncx_xml = Self::render_ncx_xml(new_toc, "Table des matières");

        // Update OPF if NCX was not registered in it
        let mut updated_opf_xml = None;
        if !opf_xml.contains("application/x-dtbncx+xml") && !opf_xml.contains(".ncx") {
            let relative_ncx_href =
                if !opf_dir.is_empty() && ncx_full_path.starts_with(&format!("{opf_dir}/")) {
                    ncx_full_path
                        .trim_start_matches(&format!("{opf_dir}/"))
                        .to_string()
                } else {
                    ncx_full_path.clone()
                };

            let item_decl = format!(
                r#"<item id="ncx" href="{relative_ncx_href}" media-type="application/x-dtbncx+xml"/>"#
            );
            let mut modified_opf =
                opf_xml.replacen("</manifest>", &format!("  {item_decl}\n  </manifest>"), 1);
            if modified_opf.contains("<spine") && !modified_opf.contains("toc=") {
                modified_opf = modified_opf.replacen("<spine", r#"<spine toc="ncx""#, 1);
            }
            updated_opf_xml = Some(modified_opf);
        }

        // Write new ZIP
        let temp_file = NamedTempFile::new()?;
        let mut zip_out = ZipWriter::new(temp_file.reopen()?);

        let mut written_ncx = false;

        for i in 0..zip_in.len() {
            let mut entry = map_zip(zip_in.by_index(i))?;
            let name = entry.name().map(|n| n.to_string()).unwrap_or_default();

            if entry.is_dir() {
                map_zip(zip_out.add_directory(&name, SimpleFileOptions::default()))?;
                continue;
            }

            let options = if name == "mimetype" {
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored)
            } else {
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated)
            };

            map_zip(zip_out.start_file(&name, options))?;

            if name == ncx_full_path {
                zip_out.write_all(ncx_xml.as_bytes())?;
                written_ncx = true;
            } else if name == opf_path && updated_opf_xml.is_some() {
                zip_out.write_all(updated_opf_xml.as_ref().unwrap().as_bytes())?;
            } else {
                let mut buf = Vec::new();
                entry.read_to_end(&mut buf)?;
                zip_out.write_all(&buf)?;
            }
        }

        if !written_ncx {
            map_zip(zip_out.start_file(
                &ncx_full_path,
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated),
            ))?;
            zip_out.write_all(ncx_xml.as_bytes())?;
        }

        map_zip(zip_out.finish())?;
        std::fs::copy(temp_file.path(), epub_path)?;

        Ok(())
    }

    /// Automatically scans chapter HTML documents for headings (h1, h2, title) to build a clean TOC
    pub fn generate_toc_from_headings(epub_path: &Path) -> VellumResult<Vec<EpubTocEntry>> {
        let file = File::open(epub_path)?;
        let mut zip = ZipArchive::new(file)
            .map_err(|e| VellumError::InvalidArgument(format!("Archive zip invalide: {e}")))?;

        let opf_path = Self::find_opf_path(&mut zip)?;
        let opf_dir = Path::new(&opf_path)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("")
            .to_string();

        let mut opf_xml = String::new();
        {
            let mut f = map_zip(zip.by_name(&opf_path))?;
            f.read_to_string(&mut opf_xml)?;
        }

        let opf_doc = Document::parse(&opf_xml)
            .map_err(|e| VellumError::InvalidArgument(format!("Erreur parsing OPF: {e}")))?;

        // Collect manifest items map: id -> resolved path
        let mut manifest: HashMap<String, String> = HashMap::new();
        for item in opf_doc.descendants().filter(|n| n.has_tag_name("item")) {
            let id = item.attribute("id").unwrap_or("");
            let href = item.attribute("href").unwrap_or("");
            if !id.is_empty() && !href.is_empty() {
                manifest.insert(id.to_string(), normalize_zip_path(&opf_dir, href));
            }
        }

        // Follow spine items
        let mut spine_files: Vec<String> = Vec::new();
        if let Some(spine) = opf_doc.descendants().find(|n| n.has_tag_name("spine")) {
            for itemref in spine.descendants().filter(|n| n.has_tag_name("itemref")) {
                let idref = itemref.attribute("idref").unwrap_or("");
                if let Some(path) = manifest.get(idref) {
                    spine_files.push(path.clone());
                }
            }
        }

        let mut entries = Vec::new();
        let mut play_order = 1usize;
        let heading_re = Regex::new(r#"(?is)<h([1-3])(\s+[^>]*)?>(.*?)</h[1-3]>"#).unwrap();
        let id_re = Regex::new(r#"id=["']([^"']+)["']"#).unwrap();

        for file_path in spine_files {
            let mut content = String::new();
            if let Ok(mut f) = zip.by_name(&file_path) {
                let _ = f.read_to_string(&mut content);
            }

            if content.trim().is_empty() {
                continue;
            }

            let relative_href =
                if !opf_dir.is_empty() && file_path.starts_with(&format!("{opf_dir}/")) {
                    file_path
                        .trim_start_matches(&format!("{opf_dir}/"))
                        .to_string()
                } else {
                    file_path.clone()
                };

            let mut file_headings = Vec::new();
            for cap in heading_re.captures_iter(&content) {
                let attrs = cap.get(2).map(|m| m.as_str()).unwrap_or("");
                let body = cap.get(3).map(|m| m.as_str()).unwrap_or("");
                let clean_text = strip_html_tags(body);

                if !clean_text.is_empty() {
                    let anchor = id_re.captures(attrs).map(|c| c[1].to_string());
                    let target_href = match anchor {
                        Some(ref a) => format!("{relative_href}#{a}"),
                        None => relative_href.clone(),
                    };

                    file_headings.push((clean_text, target_href));
                }
            }

            if !file_headings.is_empty() {
                for (heading_text, target_href) in file_headings {
                    entries.push(EpubTocEntry {
                        id: format!("heading-{play_order}"),
                        label: heading_text,
                        href: target_href,
                        play_order,
                        children: Vec::new(),
                    });
                    play_order += 1;
                }
            } else {
                // Fallback to title tag or filename
                static TITLE_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
                    Regex::new(r#"(?is)<title[^>]*>(.*?)</title>"#).expect("valid title regex")
                });
                let title_re = &*TITLE_RE;
                let title = title_re
                    .captures(&content)
                    .map(|c| strip_html_tags(&c[1]))
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| {
                        Path::new(&file_path)
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("Chapitre")
                            .to_string()
                    });

                entries.push(EpubTocEntry {
                    id: format!("section-{play_order}"),
                    label: title,
                    href: relative_href,
                    play_order,
                    children: Vec::new(),
                });
                play_order += 1;
            }
        }

        Ok(entries)
    }
}
