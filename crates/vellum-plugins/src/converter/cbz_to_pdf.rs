use image::GenericImageView;
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use vellum_domain::{
    BookConverterPlugin, PluginDescriptor, PluginType, TestResult, VellumError, VellumPlugin,
    VellumResult,
};
use zip::ZipArchive;

pub struct CbzToPdfConverterPlugin {
    descriptor: PluginDescriptor,
}

impl Default for CbzToPdfConverterPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl CbzToPdfConverterPlugin {
    pub fn new() -> Self {
        Self {
            descriptor: PluginDescriptor {
                id: "converter-cbz-to-pdf".to_string(),
                name: "Convertisseur BD (CBZ ➔ PDF)".to_string(),
                description: "Assemble les pages images d'une bande dessinée ou manga CBZ en document PDF pleine page.".to_string(),
                version: "1.0.0".to_string(),
                plugin_type: PluginType::Conversion,
                external: false,
                command: None,
                supported_extensions: Vec::new(),
                source_format: Some("cbz".to_string()),
                target_format: Some("pdf".to_string()),
                config_schema: Vec::new(),
            },
        }
    }
}

impl VellumPlugin for CbzToPdfConverterPlugin {
    fn descriptor(&self) -> &PluginDescriptor {
        &self.descriptor
    }

    fn test_configuration(&self, _config: &HashMap<String, Value>) -> TestResult {
        TestResult::ok("Moteur de conversion CBZ -> PDF prêt (générateur PDF Rust intégré).")
    }

    fn as_converter_plugin(&self) -> Option<&dyn BookConverterPlugin> {
        Some(self)
    }
}

impl BookConverterPlugin for CbzToPdfConverterPlugin {
    fn source_format(&self) -> &str {
        "cbz"
    }

    fn target_format(&self) -> &str {
        "pdf"
    }

    fn convert(
        &self,
        input_file: &Path,
        output_file: &Path,
        _config: &HashMap<String, Value>,
    ) -> VellumResult<PathBuf> {
        let reader = File::open(input_file)?;
        let mut zip =
            ZipArchive::new(reader).map_err(|e| VellumError::InvalidArgument(e.to_string()))?;

        let mut image_names: Vec<String> = (0..zip.len())
            .filter_map(|i| {
                zip.by_index(i)
                    .ok()
                    .and_then(|e| e.name().ok().map(|n| n.to_string()))
            })
            .filter(|name| is_image(name))
            .collect();
        image_names.sort_by_key(|a| a.to_lowercase());

        if image_names.is_empty() {
            return Err(VellumError::InvalidArgument(format!(
                "Aucune page image trouvée dans le fichier CBZ : {}",
                input_file.display()
            )));
        }

        let mut jpeg_pages: Vec<(u32, u32, Vec<u8>)> = Vec::new();
        for name in image_names {
            if let Ok(mut entry) = zip.by_name(&name) {
                let mut raw = Vec::new();
                if entry.read_to_end(&mut raw).is_ok() {
                    if let Ok(img) = image::load_from_memory(&raw) {
                        let (w, h) = img.dimensions();
                        let mut jpeg_buf = std::io::Cursor::new(Vec::new());
                        if img
                            .write_to(&mut jpeg_buf, image::ImageFormat::Jpeg)
                            .is_ok()
                        {
                            jpeg_pages.push((w.max(1), h.max(1), jpeg_buf.into_inner()));
                        }
                    }
                }
            }
        }

        if jpeg_pages.is_empty() {
            return Err(VellumError::InvalidArgument(
                "Aucune image valide n'a pu être décodée".to_string(),
            ));
        }

        let pdf_bytes = build_jpeg_pdf(&jpeg_pages);
        let mut out = File::create(output_file)?;
        out.write_all(&pdf_bytes)?;
        Ok(output_file.to_path_buf())
    }
}

fn is_image(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".png")
        || lower.ends_with(".webp")
}

fn build_jpeg_pdf(pages: &[(u32, u32, Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"%PDF-1.4\n");

    let mut offsets = Vec::new();
    // Obj 1: Catalog, Obj 2: Pages
    // For each page i (0..N):
    //   Page obj = 3 + i*3
    //   Content obj = 4 + i*3
    //   Image XObject = 5 + i*3
    let total_objs = 2 + pages.len() * 3;
    offsets.resize(total_objs + 1, 0usize);

    offsets[1] = out.len();
    out.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    let kids: Vec<String> = (0..pages.len())
        .map(|i| format!("{} 0 R", 3 + i * 3))
        .collect();
    offsets[2] = out.len();
    out.extend_from_slice(
        format!(
            "2 0 obj\n<< /Type /Pages /Kids [{}] /Count {} >>\nendobj\n",
            kids.join(" "),
            pages.len()
        )
        .as_bytes(),
    );

    for (i, (w, h, jpeg)) in pages.iter().enumerate() {
        let page_id = 3 + i * 3;
        let content_id = 4 + i * 3;
        let img_id = 5 + i * 3;

        offsets[page_id] = out.len();
        out.extend_from_slice(
            format!(
                "{page_id} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {w} {h}] /Resources << /XObject << /Im0 {img_id} 0 R >> >> /Contents {content_id} 0 R >>\nendobj\n"
            )
            .as_bytes(),
        );

        let stream_cmd = format!("q {w} 0 0 {h} 0 0 cm /Im0 Do Q\n");
        offsets[content_id] = out.len();
        out.extend_from_slice(
            format!(
                "{content_id} 0 obj\n<< /Length {} >>\nstream\n{}endstream\nendobj\n",
                stream_cmd.len(),
                stream_cmd
            )
            .as_bytes(),
        );

        offsets[img_id] = out.len();
        out.extend_from_slice(
            format!(
                "{img_id} 0 obj\n<< /Type /XObject /Subtype /Image /Width {w} /Height {h} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream\n",
                jpeg.len()
            )
            .as_bytes(),
        );
        out.extend_from_slice(jpeg);
        out.extend_from_slice(b"\nendstream\nendobj\n");
    }

    let xref_pos = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", total_objs + 1).as_bytes());
    for off in offsets.iter().skip(1) {
        out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_pos}\n%%EOF\n",
            total_objs + 1
        )
        .as_bytes(),
    );
    out
}
