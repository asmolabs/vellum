use crate::converter::text_to_epub::escape_xml;
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use vellum_domain::{
    BookConverterPlugin, PluginDescriptor, PluginType, TestResult, VellumPlugin, VellumResult,
};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

pub struct MobiToEpubConverterPlugin {
    descriptor: PluginDescriptor,
}

impl Default for MobiToEpubConverterPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl MobiToEpubConverterPlugin {
    pub fn new() -> Self {
        Self {
            descriptor: PluginDescriptor {
                id: "converter-mobi-to-epub".to_string(),
                name: "Convertisseur Kindle (MOBI/AZW ➔ EPUB)".to_string(),
                description: "Décompresse les enregistrements PalmDOC LZ77 de fichiers Kindle (MOBI, AZW, AZW3) et génère un conteneur EPUB standardisé.".to_string(),
                version: "1.0.0".to_string(),
                plugin_type: PluginType::Conversion,
                external: false,
                command: None,
                supported_extensions: Vec::new(),
                source_format: Some("mobi".to_string()),
                target_format: Some("epub".to_string()),
                config_schema: Vec::new(),
            },
        }
    }
}

impl VellumPlugin for MobiToEpubConverterPlugin {
    fn descriptor(&self) -> &PluginDescriptor {
        &self.descriptor
    }

    fn test_configuration(&self, _config: &HashMap<String, Value>) -> TestResult {
        TestResult::ok("Décompresseur PalmDOC LZ77 100% Rust prêt (sans dépendance externe).")
    }

    fn as_converter_plugin(&self) -> Option<&dyn BookConverterPlugin> {
        Some(self)
    }
}

impl BookConverterPlugin for MobiToEpubConverterPlugin {
    fn source_format(&self) -> &str {
        "mobi"
    }

    fn target_format(&self) -> &str {
        "epub"
    }

    fn can_convert(&self, from_ext: &str, to_ext: &str) -> bool {
        (from_ext.eq_ignore_ascii_case("mobi")
            || from_ext.eq_ignore_ascii_case("azw")
            || from_ext.eq_ignore_ascii_case("azw3"))
            && to_ext.eq_ignore_ascii_case("epub")
    }

    fn convert(
        &self,
        input_file: &Path,
        output_file: &Path,
        _config: &HashMap<String, Value>,
    ) -> VellumResult<PathBuf> {
        let mut book_title = input_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Livre Kindle")
            .to_string();
        let book_author = "Auteur inconnu";

        let data = std::fs::read(input_file)?;
        let mut text_bytes = Vec::new();

        if data.len() >= 78 {
            let num_records = u16::from_be_bytes([data[76], data[77]]) as usize;
            if num_records > 0 && 78 + num_records * 8 <= data.len() {
                let mut offsets = Vec::with_capacity(num_records);
                for i in 0..num_records {
                    let base = 78 + i * 8;
                    let off = u32::from_be_bytes([
                        data[base],
                        data[base + 1],
                        data[base + 2],
                        data[base + 3],
                    ]) as usize;
                    offsets.push(off);
                }

                let rec0 = offsets[0];
                if rec0 + 16 <= data.len() {
                    let compression = u16::from_be_bytes([data[rec0], data[rec0 + 1]]);
                    let text_record_count =
                        u16::from_be_bytes([data[rec0 + 8], data[rec0 + 9]]) as usize;

                    if rec0 + 92 <= data.len() && &data[rec0 + 16..rec0 + 20] == b"MOBI" {
                        let name_off = u32::from_be_bytes([
                            data[rec0 + 84],
                            data[rec0 + 85],
                            data[rec0 + 86],
                            data[rec0 + 87],
                        ]) as usize;
                        let name_len = u32::from_be_bytes([
                            data[rec0 + 88],
                            data[rec0 + 89],
                            data[rec0 + 90],
                            data[rec0 + 91],
                        ]) as usize;
                        if name_len > 0
                            && name_len < 1024
                            && rec0 + name_off + name_len <= data.len()
                        {
                            let t = String::from_utf8_lossy(
                                &data[rec0 + name_off..rec0 + name_off + name_len],
                            )
                            .trim()
                            .to_string();
                            if !t.is_empty() {
                                book_title = t;
                            }
                        }
                    }

                    let limit = (text_record_count + 1).min(num_records);
                    for i in 1..limit {
                        let start = offsets[i];
                        let end = if i + 1 < num_records {
                            offsets[i + 1]
                        } else {
                            data.len()
                        };
                        if end > start && end <= data.len() {
                            let raw = &data[start..end];
                            if compression == 2 {
                                text_bytes.extend_from_slice(&decompress_palmdoc(raw));
                            } else if compression == 1 {
                                text_bytes.extend_from_slice(raw);
                            }
                        }
                    }
                }
            }
        }

        let mut raw_html = String::from_utf8_lossy(&text_bytes).trim().to_string();
        if raw_html.is_empty() {
            raw_html = "<p>Contenu extrait du fichier Kindle.</p>".to_string();
        }

        let file = File::create(output_file)?;
        let mut zip = ZipWriter::new(file);

        let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        zip.start_file("mimetype", stored)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        zip.write_all(b"application/epub+zip")?;

        let deflate = SimpleFileOptions::default();
        let container_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#;
        zip.start_file("META-INF/container.xml", deflate)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        zip.write_all(container_xml.as_bytes())?;

        zip.start_file("OEBPS/stylesheet.css", deflate)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        zip.write_all(b"body { font-family: Georgia, serif; line-height: 1.6; margin: 5%; }")?;

        let xhtml = format!(
            "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<!DOCTYPE html>\n<html xmlns=\"http://www.w3.org/1999/xhtml\">\n<head><title>{}</title><link rel=\"stylesheet\" type=\"text/css\" href=\"stylesheet.css\"/></head>\n<body>{}</body>\n</html>",
            escape_xml(&book_title),
            raw_html
        );
        zip.start_file("OEBPS/content.xhtml", deflate)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        zip.write_all(xhtml.as_bytes())?;

        let uuid = Uuid::new_v4();
        let opf = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<package xmlns=\"http://www.idpf.org/2007/opf\" unique-identifier=\"BookId\" version=\"2.0\">\n  <metadata xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\n    <dc:title>{}</dc:title>\n    <dc:creator>{}</dc:creator>\n    <dc:identifier id=\"BookId\">urn:uuid:{uuid}</dc:identifier>\n    <dc:language>fr</dc:language>\n  </metadata>\n  <manifest>\n    <item id=\"ncx\" href=\"toc.ncx\" media-type=\"application/x-dtbncx+xml\"/>\n    <item id=\"css\" href=\"stylesheet.css\" media-type=\"text/css\"/>\n    <item id=\"content\" href=\"content.xhtml\" media-type=\"application/xhtml+xml\"/>\n  </manifest>\n  <spine toc=\"ncx\"><itemref idref=\"content\"/></spine>\n</package>",
            escape_xml(&book_title),
            escape_xml(book_author)
        );
        zip.start_file("OEBPS/content.opf", deflate)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        zip.write_all(opf.as_bytes())?;

        let ncx = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<ncx xmlns=\"http://www.daisy.org/z3986/2005/ncx/\" version=\"2005-1\">\n  <docTitle><text>{0}</text></docTitle>\n  <navMap><navPoint id=\"nav1\" playOrder=\"1\"><navLabel><text>{0}</text></navLabel><content src=\"content.xhtml\"/></navPoint></navMap>\n</ncx>",
            escape_xml(&book_title)
        );
        zip.start_file("OEBPS/toc.ncx", deflate)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        zip.write_all(ncx.as_bytes())?;

        zip.finish()
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        Ok(output_file.to_path_buf())
    }
}

pub fn decompress_palmdoc(compressed: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(compressed.len() * 2);
    let mut i = 0;
    while i < compressed.len() {
        let b = compressed[i];
        i += 1;
        if b == 0 {
            out.push(0);
        } else if (1..=8).contains(&b) {
            let count = b as usize;
            for _ in 0..count {
                if i < compressed.len() {
                    out.push(compressed[i]);
                    i += 1;
                }
            }
        } else if (9..=0x7F).contains(&b) {
            out.push(b);
        } else if (0x80..=0xBF).contains(&b) {
            if i < compressed.len() {
                let b2 = compressed[i];
                i += 1;
                let pair = ((b as u16) << 8) | (b2 as u16);
                let distance = ((pair & 0x3FFC) >> 2) as usize;
                let length = ((pair & 0x0003) + 3) as usize;
                if distance > 0 && distance <= out.len() {
                    let start = out.len() - distance;
                    for k in 0..length {
                        let byte = out[start + k];
                        out.push(byte);
                    }
                }
            }
        } else {
            out.push(b' ');
            out.push(b ^ 0x80);
        }
    }
    out
}
