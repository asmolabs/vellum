//! # `vellum-plugins`
//!
//! Adaptateurs et plugins intégrés ou externes de Vellum.
//!
//! Regroupe :
//! - **Formats** (`format`) : extraction de métadonnées et couvertures (EPUB, PDF, CBZ/CBR, MOBI/AZW3, Audiobooks)
//! - **Convertisseurs** (`converter`) : conversions directes et via pivot EPUB (TXT, MOBI, CBZ, PDF)
//! - **Enrichissement** (`enricher`) : récupération de métadonnées distantes (Google Books, OpenLibrary)
//! - **Liseuses** (`ereader`) : synchronisation Kobo (`KoboReader.sqlite` en lecture seule) et Kindle (`My Clippings.txt`)
//! - **Traducteur** (`translator`) : pipeline de traduction IA bilingue avec cache et glossaire
//! - **Plugins externes** (`external`) : chargeur de plugins JSON/CLI tiers

pub mod cleaner;
pub mod converter;
pub mod enricher;
pub mod ereader;
pub mod external;
pub mod format;
pub mod translator;

use std::sync::Arc;
use vellum_domain::{BookConverterPlugin, BookFormatPlugin, DeviceSyncPlugin, VellumPlugin};

pub use ereader::{KindleSyncPlugin, KoboSyncPlugin};
pub use external::ExternalPluginLoader;

pub fn built_in_format_plugins() -> Vec<Arc<dyn BookFormatPlugin>> {
    vec![
        Arc::new(format::epub::EpubFormatPlugin::new()),
        Arc::new(format::pdf::PdfFormatPlugin::new()),
        Arc::new(format::comic::ComicFormatPlugin::new()),
        Arc::new(format::mobi::MobiFormatPlugin::new()),
        Arc::new(format::audiobook::AudiobookFormatPlugin::new()),
    ]
}

pub fn built_in_converter_plugins() -> Vec<Arc<dyn BookConverterPlugin>> {
    vec![
        Arc::new(converter::cbz_to_pdf::CbzToPdfConverterPlugin::new()),
        Arc::new(converter::pdf_to_cbz::PdfToCbzConverterPlugin::new()),
        Arc::new(converter::epub_to_text::EpubToTextConverterPlugin::new()),
        Arc::new(converter::text_to_epub::TextToEpubConverterPlugin::new()),
        Arc::new(converter::mobi_to_epub::MobiToEpubConverterPlugin::new()),
        Arc::new(converter::epub_to_pdf::EpubToPdfConverterPlugin::new()),
    ]
}

pub fn built_in_device_plugins() -> Vec<Arc<dyn DeviceSyncPlugin>> {
    vec![
        Arc::new(ereader::KoboSyncPlugin::new()),
        Arc::new(ereader::KindleSyncPlugin::new()),
    ]
}

pub fn builtin_plugins() -> Vec<Arc<dyn VellumPlugin>> {
    vec![
        // Formats
        Arc::new(format::epub::EpubFormatPlugin::new()),
        Arc::new(format::pdf::PdfFormatPlugin::new()),
        Arc::new(format::comic::ComicFormatPlugin::new()),
        Arc::new(format::mobi::MobiFormatPlugin::new()),
        Arc::new(format::audiobook::AudiobookFormatPlugin::new()),
        // Converters
        Arc::new(converter::cbz_to_pdf::CbzToPdfConverterPlugin::new()),
        Arc::new(converter::pdf_to_cbz::PdfToCbzConverterPlugin::new()),
        Arc::new(converter::epub_to_text::EpubToTextConverterPlugin::new()),
        Arc::new(converter::text_to_epub::TextToEpubConverterPlugin::new()),
        Arc::new(converter::mobi_to_epub::MobiToEpubConverterPlugin::new()),
        Arc::new(converter::epub_to_pdf::EpubToPdfConverterPlugin::new()),
        // Enrichers & Transformations
        Arc::new(enricher::GoogleBooksEnricherPlugin::new()),
        Arc::new(cleaner::OceanOfPdfCleanerPlugin::new()),
        Arc::new(translator::EpubTranslatorPlugin::new()),
        // Devices (eReaders)
        Arc::new(ereader::KoboSyncPlugin::new()),
        Arc::new(ereader::KindleSyncPlugin::new()),
    ]
}

