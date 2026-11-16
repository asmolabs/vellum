use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use vellum_domain::{
    BookConverterPlugin, PluginDescriptor, PluginType, TestResult, VellumError, VellumPlugin,
    VellumResult,
};
use zip::ZipArchive;

pub struct EpubToTextConverterPlugin {
    descriptor: PluginDescriptor,
}

impl Default for EpubToTextConverterPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl EpubToTextConverterPlugin {
    pub fn new() -> Self {
        Self {
            descriptor: PluginDescriptor {
                id: "converter-epub-to-txt".to_string(),
                name: "Convertisseur Texte (EPUB ➔ TXT/MD)".to_string(),
                description: "Extrait l'ensemble des chapitres d'un livre EPUB en document texte ou Markdown épuré.".to_string(),
                version: "1.0.0".to_string(),
                plugin_type: PluginType::Conversion,
                external: false,
                command: None,
                supported_extensions: Vec::new(),
                source_format: Some("epub".to_string()),
                target_format: Some("txt".to_string()),
                config_schema: Vec::new(),
            },
        }
    }
}

impl VellumPlugin for EpubToTextConverterPlugin {
    fn descriptor(&self) -> &PluginDescriptor {
        &self.descriptor
    }

    fn test_configuration(&self, _config: &HashMap<String, Value>) -> TestResult {
        TestResult::ok("Extracteur de texte EPUB prêt (décompression et parsing XHTML intégrés).")
    }

    fn as_converter_plugin(&self) -> Option<&dyn BookConverterPlugin> {
        Some(self)
    }
}

impl BookConverterPlugin for EpubToTextConverterPlugin {
    fn source_format(&self) -> &str {
        "epub"
    }

    fn target_format(&self) -> &str {
        "txt"
    }

    fn can_convert(&self, from_ext: &str, to_ext: &str) -> bool {
        from_ext.eq_ignore_ascii_case("epub")
            && (to_ext.eq_ignore_ascii_case("txt") || to_ext.eq_ignore_ascii_case("md"))
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

        let mut chapter_paths: Vec<String> = (0..zip.len())
            .filter_map(|i| {
                zip.by_index(i)
                    .ok()
                    .and_then(|e| e.name().ok().map(|n| n.to_string()))
            })
            .filter(|n| n.ends_with(".html") || n.ends_with(".xhtml") || n.ends_with(".htm"))
            .collect();
        chapter_paths.sort();

        let mut out_text = String::new();
        for chap in chapter_paths {
            if let Ok(mut entry) = zip.by_name(&chap) {
                let mut html = String::new();
                if entry.read_to_string(&mut html).is_ok() {
                    let plain = html_to_plain_text(&html);
                    if !plain.is_empty() {
                        if !out_text.is_empty() {
                            out_text.push_str("\n\n---\n\n");
                        }
                        out_text.push_str(&plain);
                    }
                }
            }
        }

        std::fs::write(output_file, out_text)?;
        Ok(output_file.to_path_buf())
    }
}

pub fn html_to_plain_text(html: &str) -> String {
    use std::sync::LazyLock;

    static STYLE_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?is)<style[^>]*>.*?</style>").expect("valid regex"));
    static SCRIPT_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?is)<script[^>]*>.*?</script>").expect("valid regex"));
    static HEAD_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?is)<head[^>]*>.*?</head>").expect("valid regex"));
    static H1_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?is)<h1[^>]*>(.*?)</h1>").expect("valid regex"));
    static H2_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?is)<h2[^>]*>(.*?)</h2>").expect("valid regex"));
    static H3_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?is)<h3[^>]*>(.*?)</h3>").expect("valid regex"));
    static P_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)</?p[^>]*>").expect("valid regex"));
    static BR_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)<br\s*/?>").expect("valid regex"));
    static TAG_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]+>").expect("valid regex"));
    static MULTI_NL_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\n{3,}").expect("valid regex"));

    let style_re = &*STYLE_RE;
    let script_re = &*SCRIPT_RE;
    let head_re = &*HEAD_RE;
    let h1_re = &*H1_RE;
    let h2_re = &*H2_RE;
    let h3_re = &*H3_RE;
    let p_re = &*P_RE;
    let br_re = &*BR_RE;
    let tag_re = &*TAG_RE;
    let multi_nl = &*MULTI_NL_RE;

    let s = head_re.replace_all(html, "");
    let s = style_re.replace_all(&s, "");
    let s = script_re.replace_all(&s, "");
    let s = h1_re.replace_all(&s, "\n\n# $1\n\n");
    let s = h2_re.replace_all(&s, "\n\n## $1\n\n");
    let s = h3_re.replace_all(&s, "\n\n### $1\n\n");
    let s = p_re.replace_all(&s, "\n");
    let s = br_re.replace_all(&s, "\n");
    let s = tag_re.replace_all(&s, "");

    let s = s
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#39;", "'");

    multi_nl.replace_all(&s, "\n\n").trim().to_string()
}
