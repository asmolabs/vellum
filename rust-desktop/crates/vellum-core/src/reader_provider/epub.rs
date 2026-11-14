use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use regex::Regex;
use zip::ZipArchive;

use super::html::{
    extract_chapter_title, html_to_plain_text, natural_sort_key, normalize_zip_path,
    paginate_html_into_pages, replace_image_sources, sanitize_reader_html, wrap_with_reader_theme,
};
use super::{BookContentProvider, PageInfo, ReaderPage, DEFAULT_TARGET_CHARS_PER_PAGE};

pub struct EpubContentProvider {
    pages: Vec<ReaderPage>,
    toc: Vec<(usize, String)>,
    chapters: Vec<(String, String)>,
}

impl EpubContentProvider {
    pub fn new(path: &Path) -> Self {
        let mut raw_chapters: Vec<(String, String)> = Vec::new();

        if let Ok(file) = File::open(path) {
            if let Ok(mut archive) = ZipArchive::new(file) {
                // 1. Extract and index all images in the EPUB as Base64 data URIs
                let mut image_map: HashMap<String, String> = HashMap::new();
                for i in 0..archive.len() {
                    if let Ok(mut f) = archive.by_index(i) {
                        let name = f.name().map(|n| n.to_string()).unwrap_or_default();
                        let lower = name.to_lowercase();
                        let mime = if lower.ends_with(".png") {
                            Some("image/png")
                        } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
                            Some("image/jpeg")
                        } else if lower.ends_with(".webp") {
                            Some("image/webp")
                        } else if lower.ends_with(".gif") {
                            Some("image/gif")
                        } else if lower.ends_with(".svg") {
                            Some("image/svg+xml")
                        } else {
                            None
                        };

                        if let Some(mime_type) = mime {
                            let mut bytes = Vec::new();
                            if f.read_to_end(&mut bytes).is_ok() && !bytes.is_empty() {
                                let data_uri =
                                    format!("data:{};base64,{}", mime_type, BASE64.encode(&bytes));
                                image_map.insert(name.clone(), data_uri.clone());
                                image_map.insert(lower.clone(), data_uri.clone());

                                if let Some(file_name) =
                                    Path::new(&name).file_name().and_then(|fn_| fn_.to_str())
                                {
                                    image_map.insert(file_name.to_string(), data_uri.clone());
                                    image_map.insert(file_name.to_lowercase(), data_uri.clone());
                                }

                                let segments: Vec<&str> = name.split('/').collect();
                                for idx in 1..segments.len() {
                                    let subpath = segments[idx..].join("/");
                                    image_map.insert(subpath.clone(), data_uri.clone());
                                    image_map.insert(subpath.to_lowercase(), data_uri.clone());
                                }
                            }
                        }
                    }
                }

                // 2. Identify OPF package file
                let opf_path = {
                    let mut p = None;
                    if let Ok(mut container) = archive.by_name("META-INF/container.xml") {
                        let mut content = String::new();
                        if container.read_to_string(&mut content).is_ok() {
                            let re_root =
                                Regex::new(r#"<rootfile[^>]+full-path=["']([^"']+)["']"#).unwrap();
                            if let Some(caps) = re_root.captures(&content) {
                                p = Some(caps[1].to_string());
                            }
                        }
                    }
                    if p.is_none() {
                        for i in 0..archive.len() {
                            if let Ok(f) = archive.by_index(i) {
                                let name = f.name().map(|n| n.to_string()).unwrap_or_default();
                                if name.to_lowercase().ends_with(".opf") {
                                    p = Some(name);
                                    break;
                                }
                            }
                        }
                    }
                    p
                };

                // 3. Resolve chapter ordering from OPF spine
                let mut ordered_chapter_names: Vec<String> = Vec::new();
                if let Some(opf_file_name) = &opf_path {
                    if let Ok(mut opf_file) = archive.by_name(opf_file_name) {
                        let mut opf_xml = String::new();
                        if opf_file.read_to_string(&mut opf_xml).is_ok() {
                            let opf_dir = Path::new(opf_file_name)
                                .parent()
                                .and_then(|p| p.to_str())
                                .unwrap_or("");

                            let mut manifest: HashMap<String, String> = HashMap::new();
                            let re_item = Regex::new(r#"<item\s+[^>]*>"#).unwrap();
                            let re_id = Regex::new(r#"id=["']([^"']+)["']"#).unwrap();
                            let re_href = Regex::new(r#"href=["']([^"']+)["']"#).unwrap();

                            for mat in re_item.find_iter(&opf_xml) {
                                let item_tag = mat.as_str();
                                if let (Some(id_caps), Some(href_caps)) =
                                    (re_id.captures(item_tag), re_href.captures(item_tag))
                                {
                                    let id = id_caps[1].to_string();
                                    let href = href_caps[1].to_string();
                                    let full_path = normalize_zip_path(opf_dir, &href);
                                    manifest.insert(id, full_path);
                                }
                            }

                            let re_spine = Regex::new(r#"(?s)<spine[^>]*>(.*?)</spine>"#).unwrap();
                            if let Some(spine_caps) = re_spine.captures(&opf_xml) {
                                let spine_content = &spine_caps[1];
                                let re_itemref =
                                    Regex::new(r#"<itemref\s+[^>]*idref=["']([^"']+)["']"#)
                                        .unwrap();
                                for caps in re_itemref.captures_iter(spine_content) {
                                    let idref = &caps[1];
                                    if let Some(target_file) = manifest.get(idref) {
                                        let lower = target_file.to_lowercase();
                                        if lower.ends_with(".xhtml")
                                            || lower.ends_with(".html")
                                            || lower.ends_with(".htm")
                                        {
                                            ordered_chapter_names.push(target_file.clone());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Fallback to natural alphabetical sorting if no spine was retrieved
                if ordered_chapter_names.is_empty() {
                    let mut names: Vec<String> = Vec::new();
                    for i in 0..archive.len() {
                        if let Ok(file) = archive.by_index(i) {
                            let name = file.name().map(|n| n.to_string()).unwrap_or_default();
                            let lower = name.to_lowercase();
                            if (lower.ends_with(".xhtml")
                                || lower.ends_with(".html")
                                || lower.ends_with(".htm"))
                                && !lower.contains("toc")
                                && !lower.contains("cover")
                            {
                                names.push(name);
                            }
                        }
                    }
                    names.sort_by_key(|a| natural_sort_key(a));
                    ordered_chapter_names = names;
                }

                // 4. Load raw chapters
                for name in ordered_chapter_names {
                    if let Ok(mut ch_file) = archive.by_name(&name) {
                        let mut raw_content = String::new();
                        if ch_file.read_to_string(&mut raw_content).is_ok()
                            && !raw_content.trim().is_empty()
                        {
                            let processed_html =
                                replace_image_sources(&raw_content, &name, &image_map);
                            let title = extract_chapter_title(&processed_html, raw_chapters.len());
                            raw_chapters.push((title, processed_html));
                        }
                    }
                }
            }
        }

        // 5. Paginate chapters into discrete eReader pages (~1300 chars target)
        let mut pages = Vec::new();
        let mut toc = Vec::new();
        let mut chapters = Vec::new();

        for (title, ch_html) in raw_chapters {
            let ch_pages = paginate_html_into_pages(&ch_html, DEFAULT_TARGET_CHARS_PER_PAGE);
            if ch_pages.is_empty() {
                continue;
            }
            chapters.push((title.clone(), sanitize_reader_html(&ch_html)));
            let ch_idx = chapters.len() - 1;
            let start_page_index = pages.len();
            toc.push((start_page_index, title.clone()));

            let total_in_ch = ch_pages.len();
            for (p_idx, page_html) in ch_pages.into_iter().enumerate() {
                pages.push(ReaderPage {
                    html: page_html,
                    chapter_index: ch_idx,
                    chapter_title: title.clone(),
                    page_in_chapter: p_idx + 1,
                    total_in_chapter: total_in_ch,
                });
            }
        }

        Self {
            pages,
            toc,
            chapters,
        }
    }
}

impl BookContentProvider for EpubContentProvider {
    fn total_pages(&self) -> usize {
        self.pages.len()
    }

    fn get_page_html(
        &self,
        page_index: usize,
        theme: &str,
        font_size: u32,
        is_serif: bool,
    ) -> String {
        if page_index >= self.pages.len() {
            return String::new();
        }
        wrap_with_reader_theme(&self.pages[page_index].html, theme, font_size, is_serif)
    }

    fn get_section_title(&self, page_index: usize) -> Option<String> {
        self.pages.get(page_index).map(|p| {
            if p.total_in_chapter > 1 {
                format!(
                    "{} ({}/{})",
                    p.chapter_title, p.page_in_chapter, p.total_in_chapter
                )
            } else {
                p.chapter_title.clone()
            }
        })
    }

    fn get_page_info(&self, page_index: usize) -> Option<PageInfo> {
        self.pages.get(page_index).map(|p| PageInfo {
            chapter_index: p.chapter_index,
            chapter_title: p.chapter_title.clone(),
            page_in_chapter: p.page_in_chapter,
            total_in_chapter: p.total_in_chapter,
            global_page: page_index + 1,
            total_pages: self.pages.len(),
        })
    }

    fn get_table_of_contents(&self) -> Vec<(usize, String)> {
        self.toc.clone()
    }

    impl_chapter_api!();
}
// 2. CBZ / CBR Provider
