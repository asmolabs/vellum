//! HTML and assets for the Vellum Web Companion & Mobile Reader

/// Retourne la page HTML principale du Web Companion.
pub fn library_page_html() -> &'static str {
    include_str!("../assets/web_companion/library.html")
}

/// Retourne la page HTML du lecteur mobile Web Companion.
pub fn reader_page_html() -> &'static str {
    include_str!("../assets/web_companion/reader.html")
}
