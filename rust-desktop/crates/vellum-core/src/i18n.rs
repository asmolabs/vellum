//! Service d'internationalisation (i18n) embarquant les catalogues de traduction FR/EN.

use std::collections::HashMap;
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

const FR_JSON: &str = include_str!("../locales/fr.json");
const EN_JSON: &str = include_str!("../locales/en.json");

static FR_CATALOG: LazyLock<HashMap<String, String>> =
    LazyLock::new(|| serde_json::from_str(FR_JSON).expect("valid fr.json locale"));
static EN_CATALOG: LazyLock<HashMap<String, String>> =
    LazyLock::new(|| serde_json::from_str(EN_JSON).expect("valid en.json locale"));

/// Langue supportée par l'interface et les messages applicatifs de Vellum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Locale {
    #[default]
    Fr,
    En,
}

impl Locale {
    /// Code ISO court (`"fr"`, `"en"`).
    pub fn as_code(self) -> &'static str {
        match self {
            Locale::Fr => "fr",
            Locale::En => "en",
        }
    }

    /// Libellé natif pour l'affichage dans les paramètres.
    pub fn display_name(self) -> &'static str {
        match self {
            Locale::Fr => "Français",
            Locale::En => "English",
        }
    }

    /// Parse un code de langue (`"fr"`, `"en"`, `"en_US"`, `"fr-FR"`), avec repli sur `Fr`.
    pub fn parse_code(code: &str) -> Self {
        let lower = code.trim().to_ascii_lowercase();
        if lower.starts_with("en") {
            Locale::En
        } else {
            Locale::Fr
        }
    }

    /// Détecte la locale système à partir des variables `LC_ALL` / `LANG`.
    pub fn detect_system() -> Self {
        let raw = std::env::var("LC_ALL")
            .or_else(|_| std::env::var("LANG"))
            .unwrap_or_default();
        Self::parse_code(&raw)
    }

    /// Retourne toutes les locales disponibles.
    pub fn all() -> &'static [Locale] {
        &[Locale::Fr, Locale::En]
    }
}

/// Retourne la traduction associée à `key` pour `locale` (avec repli sur `Fr`, puis sur `key`).
pub fn translate<'a>(locale: Locale, key: &'a str) -> &'a str {
    let catalog = match locale {
        Locale::Fr => &*FR_CATALOG,
        Locale::En => &*EN_CATALOG,
    };
    catalog
        .get(key)
        .or_else(|| FR_CATALOG.get(key))
        .map(String::as_str)
        .unwrap_or(key)
}

/// Retourne la traduction formatée en remplaçant les placeholders `{name}`.
pub fn translate_fmt(locale: Locale, key: &str, args: &[(&str, &str)]) -> String {
    let mut out = translate(locale, key).to_string();
    for (k, v) in args {
        out = out.replace(&format!("{{{k}}}"), v);
    }
    out
}

/// Retourne le dictionnaire complet de la locale (utile pour le Web Companion).
pub fn catalog_for(locale: Locale) -> &'static HashMap<String, String> {
    match locale {
        Locale::Fr => &FR_CATALOG,
        Locale::En => &EN_CATALOG,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_i18n_catalogs_and_formatting() {
        assert_eq!(translate(Locale::Fr, "nav.library"), "Bibliothèque");
        assert_eq!(translate(Locale::En, "nav.library"), "Library");
        assert_eq!(
            translate_fmt(Locale::En, "status.books_count", &[("count", "42")]),
            "42 book(s)"
        );
        assert_eq!(Locale::parse_code("en_US.UTF-8"), Locale::En);
        assert_eq!(Locale::parse_code("fr_FR.UTF-8"), Locale::Fr);

        // Vérifie la parité des clés entre fr.json et en.json
        for key in FR_CATALOG.keys() {
            assert!(
                EN_CATALOG.contains_key(key),
                "Clé manquante dans en.json: {key}"
            );
        }
    }
}

