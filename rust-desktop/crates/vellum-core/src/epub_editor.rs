use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use regex::Regex;
use roxmltree::{Document, Node};
use tempfile::NamedTempFile;
use vellum_domain::{
    EpubDiagnostics, EpubIssue, EpubIssueSeverity, EpubTocEntry, VellumError, VellumResult,
};
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

/// EPUB Inspector, Linter & Table of Contents (TOC) Editor
pub struct EpubEditor;

impl EpubEditor {
    /// Performs deep structural inspection and linting of an EPUB file
    pub fn inspect(epub_path: &Path) -> VellumResult<EpubDiagnostics> {
        let meta = std::fs::metadata(epub_path).map_err(|e| {
            VellumError::NotFound(format!("Fichier EPUB introuvable: {e}"))
        })?;
        let total_size_bytes = meta.len();

        let file = File::open(epub_path)?;
        let mut zip = ZipArchive::new(file)
            .map_err(|e| VellumError::InvalidArgument(format!("Archive zip corrompue: {e}")))?;

        let mut issues = Vec::new();
        let total_files = zip.len();

        // 1. Check mimetype entry
        let mut has_valid_mimetype = false;
        if total_files > 0 {
            if let Ok(mut first_entry) = zip.by_index(0) {
                let name = first_entry.name().map(|n| n.to_string()).unwrap_or_default();
                if name == "mimetype" {
                    let mut content = String::new();
                    if first_entry.read_to_string(&mut content).is_ok()
                        && content.trim() == "application/epub+zip"
                    {
                        has_valid_mimetype = true;
                        if first_entry.compression() != zip::CompressionMethod::Stored {
                            issues.push(EpubIssue {
                                severity: EpubIssueSeverity::Warning,
                                code: "MIMETYPE_COMPRESSED".to_string(),
                                message: "Le fichier 'mimetype' est compressé alors que le standard EPUB exige qu'il soit stocké sans compression.".to_string(),
                                file: Some("mimetype".to_string()),
                            });
                        }
                    } else {
                        issues.push(EpubIssue {
                            severity: EpubIssueSeverity::Error,
                            code: "INVALID_MIMETYPE_CONTENT".to_string(),
                            message: format!("Contenu du mimetype invalide: '{content}' (attendu: 'application/epub+zip')."),
                            file: Some("mimetype".to_string()),
                        });
                    }
                } else {
                    issues.push(EpubIssue {
                        severity: EpubIssueSeverity::Warning,
                        code: "MIMETYPE_NOT_FIRST".to_string(),
                        message: format!("Le premier fichier dans l'archive est '{name}' au lieu de 'mimetype'."),
                        file: Some(name),
                    });
                }
            }
        } else {
            issues.push(EpubIssue {
                severity: EpubIssueSeverity::Error,
                code: "EMPTY_ARCHIVE".to_string(),
                message: "L'archive EPUB est complètement vide.".to_string(),
                file: None,
            });
            return Ok(EpubDiagnostics {
                epub_version: "Inconnue".to_string(),
                total_files: 0,
                total_size_bytes,
                manifest_count: 0,
                spine_count: 0,
                toc_entries_count: 0,
                has_cover: false,
                issues,
                is_valid: false,
            });
        }

        if !has_valid_mimetype && !issues.iter().any(|i| i.code.starts_with("MIMETYPE")) {
            issues.push(EpubIssue {
                severity: EpubIssueSeverity::Error,
                code: "MISSING_MIMETYPE".to_string(),
                message: "Le fichier 'mimetype' est absent de l'archive.".to_string(),
                file: None,
            });
        }

        // 2. Check container.xml
        let mut opf_path = None;
        match zip.by_name("META-INF/container.xml") {
            Ok(mut container_file) => {
                let mut xml = String::new();
                if container_file.read_to_string(&mut xml).is_ok() {
                    if let Ok(doc) = Document::parse(&xml) {
                        if let Some(rf) = doc.descendants().find(|n| n.has_tag_name("rootfile")) {
                            if let Some(fp) = rf.attribute("full-path") {
                                opf_path = Some(fp.to_string());
                            }
                        }
                    }
                }
                if opf_path.is_none() {
                    issues.push(EpubIssue {
                        severity: EpubIssueSeverity::Error,
                        code: "INVALID_CONTAINER_XML".to_string(),
                        message: "Le fichier META-INF/container.xml ne spécifie aucun 'rootfile' valide.".to_string(),
                        file: Some("META-INF/container.xml".to_string()),
                    });
                }
            }
            Err(_) => {
                issues.push(EpubIssue {
                    severity: EpubIssueSeverity::Error,
                    code: "MISSING_CONTAINER_XML".to_string(),
                    message: "Le descripteur META-INF/container.xml est introuvable.".to_string(),
                    file: Some("META-INF/container.xml".to_string()),
                });
            }
        }

        // Check for DRM / encryption files
        if zip.by_name("META-INF/encryption.xml").is_ok() {
            issues.push(EpubIssue {
                severity: EpubIssueSeverity::Warning,
                code: "DRM_ENCRYPTION_DETECTED".to_string(),
                message: "Présence d'un descripteur de chiffrement META-INF/encryption.xml (DRM potentiel).".to_string(),
                file: Some("META-INF/encryption.xml".to_string()),
            });
        }

        if zip.by_name("META-INF/rights.xml").is_ok() {
            issues.push(EpubIssue {
                severity: EpubIssueSeverity::Info,
                code: "RIGHTS_XML_PRESENT".to_string(),
                message: "Fichier de droits META-INF/rights.xml détecté.".to_string(),
                file: Some("META-INF/rights.xml".to_string()),
            });
        }

        // Fallback for OPF if container.xml failed
        if opf_path.is_none() {
            for i in 0..zip.len() {
                if let Ok(entry) = zip.by_index(i) {
                    let name = entry.name().map(|n| n.to_string()).unwrap_or_default();
                    if name.to_lowercase().ends_with(".opf") {
                        opf_path = Some(name);
                        break;
                    }
                }
            }
        }

        let opf_full_path = match opf_path {
            Some(p) => p,
            None => {
                issues.push(EpubIssue {
                    severity: EpubIssueSeverity::Error,
                    code: "MISSING_OPF".to_string(),
                    message: "Aucun fichier de package (.opf) trouvé dans l'EPUB.".to_string(),
                    file: None,
                });
                return Ok(EpubDiagnostics {
                    epub_version: "Inconnue".to_string(),
                    total_files,
                    total_size_bytes,
                    manifest_count: 0,
                    spine_count: 0,
                    toc_entries_count: 0,
                    has_cover: false,
                    issues,
                    is_valid: false,
                });
            }
        };

        // 3. Parse OPF file
        let opf_dir = Path::new(&opf_full_path)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("");

        let mut opf_xml = String::new();
        match zip.by_name(&opf_full_path) {
            Ok(mut opf_file) => {
                if let Err(e) = opf_file.read_to_string(&mut opf_xml) {
                    issues.push(EpubIssue {
                        severity: EpubIssueSeverity::Error,
                        code: "OPF_READ_ERROR".to_string(),
                        message: format!("Impossible de lire le fichier OPF '{opf_full_path}': {e}"),
                        file: Some(opf_full_path.clone()),
                    });
                }
            }
            Err(_) => {
                issues.push(EpubIssue {
                    severity: EpubIssueSeverity::Error,
                    code: "OPF_NOT_FOUND".to_string(),
                    message: format!("Le fichier OPF déclaré '{opf_full_path}' est introuvable dans le zip."),
                    file: Some(opf_full_path.clone()),
                });
            }
        }

        let mut epub_version = "2.0".to_string();
        let mut manifest_count = 0usize;
        let mut spine_count = 0usize;
        let mut has_cover = false;
        let mut manifest_items: HashMap<String, String> = HashMap::new(); // id -> full_path_in_zip
        let mut manifest_hrefs: Vec<String> = Vec::new();
        let mut ncx_path: Option<String> = None;
        let mut nav_path: Option<String> = None;

        if !opf_xml.is_empty() {
            match Document::parse(&opf_xml) {
                Ok(doc) => {
                    if let Some(pkg) = doc.descendants().find(|n| n.has_tag_name("package")) {
                        if let Some(ver) = pkg.attribute("version") {
                            epub_version = ver.to_string();
                        }
                    }

                    // Manifest
                    for item in doc.descendants().filter(|n| n.has_tag_name("item")) {
                        manifest_count += 1;
                        let id = item.attribute("id").unwrap_or("").to_string();
                        let href = item.attribute("href").unwrap_or("");
                        let media_type = item.attribute("media-type").unwrap_or("");
                        let properties = item.attribute("properties").unwrap_or("");

                        let resolved = normalize_zip_path(opf_dir, href);
                        manifest_items.insert(id.clone(), resolved.clone());
                        manifest_hrefs.push(resolved.clone());

                        if properties.contains("cover-image") || id.to_lowercase().contains("cover") {
                            has_cover = true;
                        }

                        if media_type == "application/x-dtbncx+xml" || href.ends_with(".ncx") {
                            ncx_path = Some(resolved.clone());
                        }

                        if properties.contains("nav") {
                            nav_path = Some(resolved.clone());
                        }

                        // Check if file physically exists in the zip
                        if zip.by_name(&resolved).is_err() {
                            issues.push(EpubIssue {
                                severity: EpubIssueSeverity::Error,
                                code: "MISSING_MANIFEST_FILE".to_string(),
                                message: format!("Fichier référencé dans le manifest mais introuvable dans l'archive: '{resolved}'"),
                                file: Some(resolved),
                            });
                        }
                    }

                    // Check metadata cover tag (EPUB 2)
                    if !has_cover {
                        for meta_tag in doc.descendants().filter(|n| n.has_tag_name("meta")) {
                            if meta_tag.attribute("name") == Some("cover") {
                                if let Some(content_id) = meta_tag.attribute("content") {
                                    if manifest_items.contains_key(content_id) {
                                        has_cover = true;
                                        break;
                                    }
                                }
                            }
                        }
                    }

                    // Spine
                    if let Some(spine) = doc.descendants().find(|n| n.has_tag_name("spine")) {
                        if let Some(toc_id) = spine.attribute("toc") {
                            if let Some(target) = manifest_items.get(toc_id) {
                                ncx_path = Some(target.clone());
                            }
                        }

                        for itemref in spine.descendants().filter(|n| n.has_tag_name("itemref")) {
                            spine_count += 1;
                            let idref = itemref.attribute("idref").unwrap_or("");
                            if !manifest_items.contains_key(idref) {
                                issues.push(EpubIssue {
                                    severity: EpubIssueSeverity::Error,
                                    code: "UNRESOLVED_SPINE_IDREF".to_string(),
                                    message: format!("Élément de spine avec idref inconnu: '{idref}'"),
                                    file: Some(opf_full_path.clone()),
                                });
                            }
                        }
                    }
                }
                Err(e) => {
                    issues.push(EpubIssue {
                        severity: EpubIssueSeverity::Error,
                        code: "OPF_XML_MALFORMED".to_string(),
                        message: format!("Le fichier OPF contient un XML malformé: {e}"),
                        file: Some(opf_full_path.clone()),
                    });
                }
            }
        }

        // 4. Check for orphan files in the ZIP archive
        for i in 0..zip.len() {
            if let Ok(entry) = zip.by_index(i) {
                let name = entry.name().map(|n| n.to_string()).unwrap_or_default();
                if name.ends_with('/') {
                    continue; // skip directory entries
                }
                if name == "mimetype"
                    || name.starts_with("META-INF/")
                    || name == opf_full_path
                    || manifest_hrefs.contains(&name)
                {
                    continue;
                }

                // Junk or orphan file
                let is_junk = name.contains(".DS_Store") || name.contains("Thumbs.db");
                issues.push(EpubIssue {
                    severity: if is_junk {
                        EpubIssueSeverity::Info
                    } else {
                        EpubIssueSeverity::Warning
                    },
                    code: "ORPHAN_ZIP_FILE".to_string(),
                    message: format!("Fichier présent dans l'archive mais non déclaré dans le manifest: '{name}'"),
                    file: Some(name),
                });
            }
        }

        // 5. Count TOC entries
        let mut toc_entries_count = 0usize;
        if let Some(ncx) = &ncx_path {
            if let Ok(mut f) = zip.by_name(ncx) {
                let mut xml = String::new();
                if f.read_to_string(&mut xml).is_ok() {
                    if let Ok(doc) = Document::parse(&xml) {
                        toc_entries_count = doc.descendants().filter(|n| n.has_tag_name("navPoint")).count();
                    }
                }
            }
        } else if let Some(nav) = &nav_path {
            if let Ok(mut f) = zip.by_name(nav) {
                let mut xml = String::new();
                if f.read_to_string(&mut xml).is_ok() {
                    if let Ok(doc) = Document::parse(&xml) {
                        toc_entries_count = doc.descendants().filter(|n| n.has_tag_name("li")).count();
                    }
                }
            }
        }

        if toc_entries_count == 0 {
            issues.push(EpubIssue {
                severity: EpubIssueSeverity::Warning,
                code: "MISSING_TOC".to_string(),
                message: "Aucune Table des Matières (TOC / toc.ncx / nav.xhtml) exploitable trouvée.".to_string(),
                file: None,
            });
        }

        if !has_cover {
            issues.push(EpubIssue {
                severity: EpubIssueSeverity::Info,
                code: "NO_COVER_DECLARED".to_string(),
                message: "Aucune couverture n'est explicitement déclarée dans les métadonnées de l'EPUB.".to_string(),
                file: None,
            });
        }

        let is_valid = !issues
            .iter()
            .any(|i| i.severity == EpubIssueSeverity::Error);

        Ok(EpubDiagnostics {
            epub_version,
            total_files,
            total_size_bytes,
            manifest_count,
            spine_count,
            toc_entries_count,
            has_cover,
            issues,
            is_valid,
        })
    }

    /// Extracts the hierarchical Table of Contents (TOC) from an EPUB
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
            let mut f = zip.by_name(&opf_path).map_err(|_| {
                VellumError::NotFound(format!("OPF introuvable: {opf_path}"))
            })?;
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
            let mut f = zip_in.by_name(&opf_path).map_err(|_| {
                VellumError::NotFound(format!("OPF introuvable: {opf_path}"))
            })?;
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
            let relative_ncx_href = if !opf_dir.is_empty() && ncx_full_path.starts_with(&format!("{opf_dir}/")) {
                ncx_full_path.trim_start_matches(&format!("{opf_dir}/")).to_string()
            } else {
                ncx_full_path.clone()
            };

            let item_decl = format!(r#"<item id="ncx" href="{relative_ncx_href}" media-type="application/x-dtbncx+xml"/>"#);
            let mut modified_opf = opf_xml.replacen("</manifest>", &format!("  {item_decl}\n  </manifest>"), 1);
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
            let mut entry = zip_in.by_index(i)?;
            let name = entry.name().map(|n| n.to_string()).unwrap_or_default();

            if entry.is_dir() {
                zip_out.add_directory(&name, SimpleFileOptions::default())?;
                continue;
            }

            let options = if name == "mimetype" {
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored)
            } else {
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated)
            };

            zip_out.start_file(&name, options)?;

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
            zip_out.start_file(
                &ncx_full_path,
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated),
            )?;
            zip_out.write_all(ncx_xml.as_bytes())?;
        }

        zip_out.finish()?;
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
            let mut f = zip.by_name(&opf_path)?;
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

            let relative_href = if !opf_dir.is_empty() && file_path.starts_with(&format!("{opf_dir}/")) {
                file_path.trim_start_matches(&format!("{opf_dir}/")).to_string()
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
                let title_re = Regex::new(r#"(?is)<title[^>]*>(.*?)</title>"#).unwrap();
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

    /// Automatically repairs common EPUB flaws:
    /// - Re-packs mimetype as file 0 without compression
    /// - Strips defunct or corrupted DRM descriptors
    /// - Cleans junk files (.DS_Store, Thumbs.db)
    /// - Ensures a valid Table of Contents exists
    pub fn clean_and_repair(epub_path: &Path) -> VellumResult<EpubDiagnostics> {
        let diag_before = Self::inspect(epub_path)?;

        let file = File::open(epub_path)?;
        let mut zip_in = ZipArchive::new(file)
            .map_err(|e| VellumError::InvalidArgument(format!("Archive zip invalide: {e}")))?;

        let temp_file = NamedTempFile::new()?;
        let mut zip_out = ZipWriter::new(temp_file.reopen()?);

        // 1. Write pristine uncompressed mimetype first
        let stored_opt = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        let deflated_opt = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        zip_out.start_file("mimetype", stored_opt)?;
        zip_out.write_all(b"application/epub+zip")?;

        // 2. Copy all legitimate files, skipping defunct DRM / junk files
        for i in 0..zip_in.len() {
            let mut entry = zip_in.by_index(i)?;
            let name = entry.name().map(|n| n.to_string()).unwrap_or_default();

            if name == "mimetype" {
                continue; // already written first
            }

            // Strip junk and corrupted DRM
            if name.contains(".DS_Store")
                || name.contains("Thumbs.db")
                || name == "META-INF/encryption.xml"
                || name == "META-INF/rights.xml"
            {
                continue;
            }

            if entry.is_dir() {
                zip_out.add_directory(&name, SimpleFileOptions::default())?;
                continue;
            }

            zip_out.start_file(&name, deflated_opt)?;
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;
            zip_out.write_all(&buf)?;
        }

        zip_out.finish()?;
        std::fs::copy(temp_file.path(), epub_path)?;

        // If TOC was missing, generate one from headings
        if diag_before.toc_entries_count == 0 {
            if let Ok(generated_toc) = Self::generate_toc_from_headings(epub_path) {
                if !generated_toc.is_empty() {
                    let _ = Self::save_toc(epub_path, &generated_toc);
                }
            }
        }

        // Re-inspect and return updated report
        Self::inspect(epub_path)
    }

    // --- Private Helper Methods ---

    fn find_opf_path(zip: &mut ZipArchive<File>) -> VellumResult<String> {
        if let Ok(mut container) = zip.by_name("META-INF/container.xml") {
            let mut xml = String::new();
            if container.read_to_string(&mut xml).is_ok() {
                if let Ok(doc) = Document::parse(&xml) {
                    if let Some(rf) = doc.descendants().find(|n| n.has_tag_name("rootfile")) {
                        if let Some(fp) = rf.attribute("full-path") {
                            return Ok(fp.to_string());
                        }
                    }
                }
            }
        }

        for i in 0..zip.len() {
            if let Ok(entry) = zip.by_index(i) {
                let name = entry.name().map(|n| n.to_string()).unwrap_or_default();
                if name.to_lowercase().ends_with(".opf") {
                    return Ok(name);
                }
            }
        }

        Err(VellumError::NotFound("Fichier OPF introuvable dans l'EPUB".into()))
    }

    fn parse_ncx_navmap(doc: &Document, ncx_dir: &str) -> Vec<EpubTocEntry> {
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

    fn parse_epub3_nav(doc: &Document, nav_dir: &str) -> Vec<EpubTocEntry> {
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

    fn generate_fallback_toc(
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
                            let re = Regex::new(r#"(?is)<h[1-2][^>]*>(.*?)</h[1-2]>"#).unwrap();
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

    fn render_ncx_xml(toc: &[EpubTocEntry], doc_title: &str) -> String {
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

fn normalize_zip_path(base_dir: &str, rel_path: &str) -> String {
    let clean_rel = rel_path.trim_start_matches("./").replace('\\', "/");
    if base_dir.is_empty() {
        clean_rel
    } else {
        let joined = Path::new(base_dir).join(&clean_rel);
        joined.to_string_lossy().replace('\\', "/")
    }
}

fn strip_html_tags(s: &str) -> String {
    let tag_re = Regex::new(r#"(?is)<[^>]*>"#).unwrap();
    let unescaped = tag_re.replace_all(s, " ");
    unescaped
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    fn create_test_epub(with_toc: bool, has_issues: bool) -> NamedTempFile {
        let file = NamedTempFile::new().unwrap();
        let mut zip = ZipWriter::new(file.reopen().unwrap());

        let stored = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        let deflated = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        // 1. mimetype
        zip.start_file("mimetype", stored).unwrap();
        zip.write_all(b"application/epub+zip").unwrap();

        // 2. container.xml
        zip.start_file("META-INF/container.xml", deflated).unwrap();
        zip.write_all(
            br#"<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#,
        )
        .unwrap();

        // 3. content.opf
        zip.start_file("OEBPS/content.opf", deflated).unwrap();
        let opf_toc_attr = if with_toc { r#"toc="ncx""# } else { "" };
        let opf_ncx_item = if with_toc {
            r#"<item id="ncx" href="toc.ncx" media-type="application/x-dtbncx+xml"/>"#
        } else {
            ""
        };

        let opf = format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="2.0" unique-identifier="BookId">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Test Book</dc:title>
    <dc:creator>Test Author</dc:creator>
    <meta name="cover" content="cover-image"/>
  </metadata>
  <manifest>
    <item id="cover-image" href="cover.jpg" media-type="image/jpeg"/>
    <item id="ch1" href="ch1.xhtml" media-type="application/xhtml+xml"/>
    <item id="ch2" href="ch2.xhtml" media-type="application/xhtml+xml"/>
    {opf_ncx_item}
  </manifest>
  <spine {opf_toc_attr}>
    <itemref idref="ch1"/>
    <itemref idref="ch2"/>
  </spine>
</package>"#
        );
        zip.write_all(opf.as_bytes()).unwrap();

        // 4. Content files
        zip.start_file("OEBPS/cover.jpg", deflated).unwrap();
        zip.write_all(b"fake image bytes").unwrap();

        zip.start_file("OEBPS/ch1.xhtml", deflated).unwrap();
        zip.write_all(
            r#"<!DOCTYPE html><html><head><title>Chapitre 1</title></head><body><h1>Chapitre 1 : Le Réveil</h1><p>Texte</p></body></html>"#.as_bytes(),
        )
        .unwrap();

        zip.start_file("OEBPS/ch2.xhtml", deflated).unwrap();
        zip.write_all(
            r#"<!DOCTYPE html><html><head><title>Chapitre 2</title></head><body><h2 id="sec2">Chapitre 2 : L'Aventure</h2><p>Suite</p></body></html>"#.as_bytes(),
        )
        .unwrap();

        // Optional TOC
        if with_toc {
            zip.start_file("OEBPS/toc.ncx", deflated).unwrap();
            zip.write_all(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<ncx xmlns="http://www.daisy.org/z3986/2005/ncx/" version="2005-1">
  <docTitle><text>Test Book</text></docTitle>
  <navMap>
    <navPoint id="np-1" playOrder="1">
      <navLabel><text>Chapitre 1 : Le Réveil</text></navLabel>
      <content src="ch1.xhtml"/>
    </navPoint>
    <navPoint id="np-2" playOrder="2">
      <navLabel><text>Chapitre 2 : L'Aventure</text></navLabel>
      <content src="ch2.xhtml#sec2"/>
    </navPoint>
  </navMap>
</ncx>"#.as_bytes(),
            )
            .unwrap();
        }

        // Optional Issues (e.g. orphan junk file, DRM file)
        if has_issues {
            zip.start_file("OEBPS/.DS_Store", deflated).unwrap();
            zip.write_all(b"junk").unwrap();
            zip.start_file("META-INF/encryption.xml", deflated).unwrap();
            zip.write_all(b"<encryption></encryption>").unwrap();
        }

        zip.finish().unwrap();
        file
    }

    #[test]
    fn test_inspect_clean_epub() {
        let file = create_test_epub(true, false);
        let diag = EpubEditor::inspect(file.path()).expect("Inspect failed");

        assert!(diag.is_valid);
        assert_eq!(diag.epub_version, "2.0");
        assert_eq!(diag.manifest_count, 4);
        assert_eq!(diag.spine_count, 2);
        assert_eq!(diag.toc_entries_count, 2);
        assert!(diag.has_cover);
        assert!(!diag.issues.iter().any(|i| i.severity == EpubIssueSeverity::Error));
    }

    #[test]
    fn test_extract_and_save_toc() {
        let file = create_test_epub(true, false);
        let toc = EpubEditor::extract_toc(file.path()).expect("Extract TOC failed");

        assert_eq!(toc.len(), 2);
        assert_eq!(toc[0].label, "Chapitre 1 : Le Réveil");
        assert_eq!(toc[1].label, "Chapitre 2 : L'Aventure");
        assert_eq!(toc[1].href, "OEBPS/ch2.xhtml#sec2");

        // Modify label and save
        let mut modified_toc = toc;
        modified_toc[0].label = "Chapitre 1 : Modifié".to_string();

        EpubEditor::save_toc(file.path(), &modified_toc).expect("Save TOC failed");

        // Re-extract and verify
        let re_extracted = EpubEditor::extract_toc(file.path()).expect("Re-extract failed");
        assert_eq!(re_extracted[0].label, "Chapitre 1 : Modifié");
    }

    #[test]
    fn test_generate_toc_from_headings() {
        let file = create_test_epub(false, false);
        let generated = EpubEditor::generate_toc_from_headings(file.path()).unwrap();

        assert_eq!(generated.len(), 2);
        assert_eq!(generated[0].label, "Chapitre 1 : Le Réveil");
        assert_eq!(generated[1].label, "Chapitre 2 : L'Aventure");
    }

    #[test]
    fn test_clean_and_repair() {
        let file = create_test_epub(false, true);
        let diag_before = EpubEditor::inspect(file.path()).unwrap();
        assert!(diag_before.issues.iter().any(|i| i.code == "DRM_ENCRYPTION_DETECTED"));
        assert!(diag_before.issues.iter().any(|i| i.code == "ORPHAN_ZIP_FILE"));

        let diag_after = EpubEditor::clean_and_repair(file.path()).unwrap();
        assert!(!diag_after.issues.iter().any(|i| i.code == "DRM_ENCRYPTION_DETECTED"));
        assert!(!diag_after.issues.iter().any(|i| i.code == "ORPHAN_ZIP_FILE"));
        assert!(diag_after.toc_entries_count >= 2);
    }
}
