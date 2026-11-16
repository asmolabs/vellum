use regex::Regex;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use vellum_domain::{
    BookTransformationPlugin, ConfigField, PluginDescriptor, PluginType, VellumError, VellumPlugin,
    VellumResult,
};
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

pub struct OceanOfPdfCleanerPlugin {
    descriptor: PluginDescriptor,
}

impl Default for OceanOfPdfCleanerPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl OceanOfPdfCleanerPlugin {
    pub fn new() -> Self {
        Self {
            descriptor: PluginDescriptor {
                id: "oceanofpdf-cleaner".to_string(),
                name: "Nettoyeur OceanOFPDF".to_string(),
                description: "Supprime automatiquement les liens, filigranes et paragraphes 'OceanofPDF' des fichiers EPUB.".to_string(),
                version: "1.0.0".to_string(),
                plugin_type: PluginType::Transformation,
                external: false,
                command: None,
                supported_extensions: Vec::new(),
                source_format: None,
                target_format: None,
                config_schema: vec![
                    ConfigField::new(
                        "remove_whole_paragraph",
                        "Supprimer les paragraphes isolés OceanOFPDF",
                        "boolean",
                        json!(true),
                        "Retire intégralement la balise <p> contenant une mention de filigrane OceanOFPDF.",
                    ),
                    ConfigField::new(
                        "custom_replacement",
                        "Texte de remplacement (Optionnel)",
                        "text",
                        json!(""),
                        "Remplace le lien/texte OceanOFPDF par ce texte. Laisser vide pour supprimer silencieusement.",
                    ),
                ],
            },
        }
    }
}

impl VellumPlugin for OceanOfPdfCleanerPlugin {
    fn descriptor(&self) -> &PluginDescriptor {
        &self.descriptor
    }

    fn as_transformation_plugin(&self) -> Option<&dyn BookTransformationPlugin> {
        Some(self)
    }
}

impl BookTransformationPlugin for OceanOfPdfCleanerPlugin {
    fn transform(
        &self,
        input_file: &Path,
        config: &HashMap<String, Value>,
    ) -> VellumResult<PathBuf> {
        let stem = input_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("book");
        let ext = input_file
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("epub");
        let parent = input_file.parent().unwrap_or_else(|| Path::new("."));
        let output_file = parent.join(format!("{stem} (Cleaned).{ext}"));

        let remove_para = config
            .get("remove_whole_paragraph")
            .and_then(|v| {
                v.as_bool()
                    .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            })
            .unwrap_or(true);
        let replacement = config
            .get("custom_replacement")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        clean_epub(input_file, &output_file, remove_para, replacement)?;
        Ok(output_file)
    }
}

fn clean_epub(
    input_epub: &Path,
    output_epub: &Path,
    remove_para: bool,
    replacement: &str,
) -> VellumResult<()> {
    use std::sync::LazyLock;

    static LINK_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?is)<a[^>]*href=["'][^"']*oceanofpdf[^"']*["'][^>]*>.*?</a>"#)
            .expect("valid regex")
    });
    static URL_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?i)https?://(?:www\.)?oceanofpdf\.com[^\s<"'\]]*"#).expect("valid regex")
    });
    static PARA_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?is)<p[^>]*>[^<]*oceanofpdf[^<]*</p>").expect("valid regex")
    });
    static MENTION_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?i)\s*[-|–—:]\s*oceanofpdf(?:\.com)?|oceanofpdf(?:\.com)?\s*[-|–—:]\s*|oceanofpdf(?:\.com)?",
        )
        .expect("valid regex")
    });

    let link_re = &*LINK_RE;
    let url_re = &*URL_RE;
    let para_re = &*PARA_RE;
    let mention_re = &*MENTION_RE;

    let reader = File::open(input_epub)?;
    let mut zip_in =
        ZipArchive::new(reader).map_err(|e| VellumError::InvalidArgument(e.to_string()))?;

    let writer = File::create(output_epub)?;
    let mut zip_out = ZipWriter::new(writer);
    let opts = SimpleFileOptions::default();

    for i in 0..zip_in.len() {
        let mut entry = zip_in
            .by_index(i)
            .map_err(|e| VellumError::InvalidArgument(e.to_string()))?;
        let name = entry.name().map(|n| n.to_string()).unwrap_or_default();
        let name_lower = name.to_lowercase();

        zip_out
            .start_file(&name, opts)
            .map_err(|e| std::io::Error::other(e.to_string()))?;

        if name_lower.ends_with(".html")
            || name_lower.ends_with(".xhtml")
            || name_lower.ends_with(".htm")
            || name_lower.ends_with(".xml")
            || name_lower.ends_with(".ncx")
            || name_lower.ends_with(".opf")
        {
            let mut content = String::new();
            entry.read_to_string(&mut content)?;
            if remove_para {
                content = para_re.replace_all(&content, replacement).to_string();
            }
            content = link_re.replace_all(&content, replacement).to_string();
            content = url_re.replace_all(&content, replacement).to_string();
            content = mention_re.replace_all(&content, replacement).to_string();
            zip_out.write_all(content.as_bytes())?;
        } else {
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes)?;
            zip_out.write_all(&bytes)?;
        }
    }

    zip_out
        .finish()
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    Ok(())
}
