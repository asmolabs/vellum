pub use vellum_domain::{NamingTemplate, DEFAULT_NAMING_TEMPLATE};

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use vellum_domain::Book;

    #[test]
    fn test_template_with_series() {
        let mut b = Book::new("Dune Messiah", "Frank Herbert");
        b.series = Some("Dune".into());
        b.series_index = Some(2.0);

        let path = NamingTemplate::format_filename(&b, DEFAULT_NAMING_TEMPLATE, "epub");
        assert_eq!(
            path,
            PathBuf::from("Frank Herbert/Dune/T.02 - Dune Messiah.epub")
        );
    }

    #[test]
    fn test_template_without_series() {
        let b = Book::new("Le Petit Prince", "Antoine de Saint-Exupéry");

        let path = NamingTemplate::format_filename(&b, DEFAULT_NAMING_TEMPLATE, "epub");
        assert_eq!(
            path,
            PathBuf::from("Antoine de Saint-Exupéry/Le Petit Prince.epub")
        );
    }

    #[test]
    fn test_custom_flat_template() {
        let mut b = Book::new("Fondation", "Isaac Asimov");
        b.series = Some("Cycle de Fondation".into());
        b.series_index = Some(1.0);

        let tmpl = "{author} - {if_series:[{series} #{series_index:02}] - }{title}";
        let path = NamingTemplate::format_filename(&b, tmpl, "azw3");
        assert_eq!(
            path,
            PathBuf::from("Isaac Asimov - [Cycle de Fondation #01] - Fondation.azw3")
        );
    }

    #[test]
    fn test_sanitize_special_chars() {
        let mut b = Book::new("Dune: Part 1/2? \"Yes\"", "Herbert: Frank*");
        b.series = Some("Dune/Saga".into());
        b.series_index = Some(1.5);

        let path = NamingTemplate::format_filename(&b, DEFAULT_NAMING_TEMPLATE, "kepub.epub");
        assert_eq!(
            path,
            PathBuf::from("Herbert_ Frank_/Dune_Saga/T.01.5 - Dune_ Part 1_2_ _Yes_.kepub.epub")
        );
    }
}
