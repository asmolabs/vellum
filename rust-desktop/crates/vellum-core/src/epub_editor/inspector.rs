use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use roxmltree::Document;
use tempfile::NamedTempFile;
use vellum_domain::{EpubDiagnostics, EpubIssue, EpubIssueSeverity, VellumError, VellumResult};
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

use super::{map_zip, normalize_zip_path, EpubEditor};

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

        map_zip(zip_out.start_file("mimetype", stored_opt))?;
        zip_out.write_all(b"application/epub+zip")?;

        // 2. Copy all legitimate files, skipping defunct DRM / junk files
        for i in 0..zip_in.len() {
            let mut entry = map_zip(zip_in.by_index(i))?;
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
                map_zip(zip_out.add_directory(&name, SimpleFileOptions::default()))?;
                continue;
            }

            map_zip(zip_out.start_file(&name, deflated_opt))?;
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;
            zip_out.write_all(&buf)?;
        }

        map_zip(zip_out.finish())?;
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


}
