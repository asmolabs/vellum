use std::collections::HashMap;
use std::fs::File;
use std::io::Write;

use tempfile::TempDir;
use vellum_domain::{
    BookConverterPlugin, BookFormatPlugin, BookTransformationPlugin,
};
use vellum_plugins::cleaner::OceanOfPdfCleanerPlugin;
use vellum_plugins::converter::epub_to_pdf::EpubToPdfConverterPlugin;
use vellum_plugins::converter::epub_to_text::EpubToTextConverterPlugin;
use vellum_plugins::converter::mobi_to_epub::decompress_palmdoc;
use vellum_plugins::converter::text_to_epub::TextToEpubConverterPlugin;
use vellum_plugins::external::ExternalPluginLoader;
use vellum_plugins::format::audiobook::AudiobookFormatPlugin;
use vellum_plugins::format::epub::EpubFormatPlugin;
use vellum_plugins::translator::glossary_manager::GlossaryManager;
use vellum_plugins::translator::html_parser::{
    build_bilingual_chapter, extract_paragraphs, rebuild_chapter_html, restore_inline_tags,
};
use vellum_plugins::translator::qa_validator::QualityAssuranceValidator;
use vellum_plugins::translator::segment_cache::TranslationSegmentCache;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

#[test]
fn test_epub_format_extraction_and_cleaning() {
    let temp = TempDir::new().expect("tempdir");
    let epub_path = temp.path().join("dirty.epub");

    {
        let f = File::create(&epub_path).expect("create epub");
        let mut zip = ZipWriter::new(f);
        let opts = SimpleFileOptions::default();

        zip.start_file("META-INF/container.xml", opts).unwrap();
        zip.write_all(
            br#"<?xml version="1.0"?>
            <container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
              <rootfiles>
                <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
              </rootfiles>
            </container>"#,
        )
        .unwrap();

        zip.start_file("OEBPS/content.opf", opts).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="utf-8"?>
            <package xmlns="http://www.idpf.org/2007/opf" version="2.0">
              <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
                <dc:title>Dune - OceanofPDF.com</dc:title>
                <dc:creator>Frank Herbert</dc:creator>
                <dc:language>en</dc:language>
                <dc:description>Downloaded from OceanofPDF.com for free</dc:description>
              </metadata>
            </package>"#,
        )
        .unwrap();

        zip.start_file("OEBPS/chapter1.xhtml", opts).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="utf-8"?>
            <html xmlns="http://www.w3.org/1999/xhtml">
              <body>
                <p>In the beginning was the desert.</p>
                <p>OceanofPDF.com</p>
                <p>Arrakis awaits.</p>
              </body>
            </html>"#,
        )
        .unwrap();
        zip.finish().unwrap();
    }

    let epub_plugin = EpubFormatPlugin::new();
    let meta = epub_plugin
        .extract_metadata(&epub_path, &HashMap::new())
        .expect("extract metadata");
    assert_eq!(meta.author.as_deref(), Some("Frank Herbert"));

    let cleaner = OceanOfPdfCleanerPlugin::new();
    let cleaned_path = cleaner
        .transform(&epub_path, &HashMap::new())
        .expect("clean epub");

    let cleaned_meta = epub_plugin
        .extract_metadata(&cleaned_path, &HashMap::new())
        .expect("extract cleaned metadata");
    assert_eq!(cleaned_meta.title.as_deref(), Some("Dune"));

    let txt_converter = EpubToTextConverterPlugin::new();
    let txt_path = temp.path().join("cleaned.txt");
    txt_converter
        .convert(&cleaned_path, &txt_path, &HashMap::new())
        .expect("convert epub to txt");

    let text_content = std::fs::read_to_string(&txt_path).expect("read txt");
    assert!(text_content.contains("In the beginning was the desert."));
    assert!(!text_content.to_lowercase().contains("oceanofpdf"));
}

#[test]
fn test_text_to_epub_and_epub_to_pdf_conversion() {
    let temp = TempDir::new().expect("tempdir");
    let txt_file = temp.path().join("Foundation.txt");
    std::fs::write(
        &txt_file,
        "Hari Seldon looked out over Trantor.\n\nPsychohistory predicted the fall of the Empire.",
    )
    .unwrap();

    let epub_file = temp.path().join("Foundation.epub");
    let txt_to_epub = TextToEpubConverterPlugin::new();
    txt_to_epub
        .convert(&txt_file, &epub_file, &HashMap::new())
        .expect("txt to epub");
    assert!(epub_file.exists());

    let pdf_file = temp.path().join("Foundation.pdf");
    let epub_to_pdf = EpubToPdfConverterPlugin::new();
    epub_to_pdf
        .convert(&epub_file, &pdf_file, &HashMap::new())
        .expect("epub to pdf");
    assert!(pdf_file.exists());
    assert!(std::fs::metadata(&pdf_file).unwrap().len() > 200);
}

#[test]
fn test_palmdoc_decompression() {
    let raw = b"Hello World";
    let decompressed = decompress_palmdoc(raw);
    assert_eq!(&decompressed, b"Hello World");
}

#[test]
fn test_translator_html_parser_qa_glossary_and_cache() {
    let html = r#"<html><body><h1>Chapter 1</h1><p>She whispered <em>quietly</em> in the dark.</p></body></html>"#;
    let paragraphs = extract_paragraphs(html);
    assert_eq!(paragraphs.len(), 2);
    assert_eq!(paragraphs[0].marked_text, "Chapter 1");
    assert_eq!(
        paragraphs[1].marked_text,
        "She whispered {0}quietly{/0} in the dark."
    );

    let restored = restore_inline_tags(
        "Elle murmura {0}doucement{/0} dans l'obscurité.",
        &paragraphs[1].inline_tags,
    );
    assert_eq!(
        restored,
        "Elle murmura <em>doucement</em> dans l'obscurité."
    );

    let translated_inners = vec!["Chapitre 1".to_string(), restored];
    let rebuilt = rebuild_chapter_html(html, &paragraphs, &translated_inners);
    assert!(rebuilt.contains("<h1>Chapitre 1</h1>"));
    assert!(rebuilt.contains("<em>doucement</em>"));

    let bilingual = build_bilingual_chapter(html, &paragraphs, &translated_inners);
    assert!(bilingual.contains("vellum-bilingual-source"));
    assert!(bilingual.contains("vellum-bilingual-target"));

    let mut glossary = GlossaryManager::new();
    glossary.load_from_config("Winterfell -> Winterfell\nNight's Watch -> Garde de Nuit");
    let issues = QualityAssuranceValidator::validate_paragraph(
        0,
        "The Night's Watch protects the realm.",
        "Les patrouilleurs protègent le royaume.",
        Some(&glossary),
    );
    assert!(!issues.is_empty());

    let temp = TempDir::new().expect("tempdir");
    let cache = TranslationSegmentCache::new(Some(temp.path().to_path_buf()), "qwen2.5", "Français");
    cache.put("Hello world", "Bonjour le monde");
    assert_eq!(
        cache.get("Hello world").as_deref(),
        Some("Bonjour le monde")
    );
}

#[test]
fn test_audiobook_id3v2_metadata() {
    let temp = TempDir::new().expect("tempdir");
    let mp3_path = temp.path().join("sample.mp3");

    // Build minimal ID3v2.3 header with TIT2 ("Hyperion") and TPE1 ("Dan Simmons")
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"ID3");
    bytes.push(3); // v2.3
    bytes.push(0);
    bytes.push(0); // flags

    let mut frames = Vec::new();
    // TIT2 frame
    frames.extend_from_slice(b"TIT2");
    let title_payload = b"\x00Hyperion";
    frames.extend_from_slice(&(title_payload.len() as u32).to_be_bytes());
    frames.extend_from_slice(&[0, 0]);
    frames.extend_from_slice(title_payload);

    // TPE1 frame
    frames.extend_from_slice(b"TPE1");
    let author_payload = b"\x00Dan Simmons";
    frames.extend_from_slice(&(author_payload.len() as u32).to_be_bytes());
    frames.extend_from_slice(&[0, 0]);
    frames.extend_from_slice(author_payload);

    // Synchsafe size of frames
    let len = frames.len() as u32;
    bytes.push(((len >> 21) & 0x7F) as u8);
    bytes.push(((len >> 14) & 0x7F) as u8);
    bytes.push(((len >> 7) & 0x7F) as u8);
    bytes.push((len & 0x7F) as u8);
    bytes.extend_from_slice(&frames);

    std::fs::write(&mp3_path, &bytes).unwrap();

    let plugin = AudiobookFormatPlugin::new();
    let meta = plugin.extract_metadata(&mp3_path, &HashMap::new()).expect("extract mp3");
    assert_eq!(meta.title.as_deref(), Some("Hyperion"));
    assert_eq!(meta.author.as_deref(), Some("Dan Simmons"));
}

#[test]
fn test_external_plugin_loader_creates_and_loads_sample_plugin() {
    let temp = TempDir::new().expect("tempdir");
    let loader = ExternalPluginLoader::new(temp.path().to_path_buf());
    let loaded = loader.load_external_plugins();
    assert!(!loaded.is_empty());
    assert_eq!(loaded[0].descriptor().id, "ext-sample-stamper");
    let test_res = loaded[0].test_plugin(&HashMap::new());
    assert!(test_res.success);
}
