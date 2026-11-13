use std::collections::BTreeMap;

use vellum_domain::{CustomColumnDefinition, CustomColumnType, VellumError, VellumResult};
use vellum_storage::CustomColumnRepository;

/// Service métier pour la définition et l'évaluation des colonnes personnalisées.
#[derive(Clone)]
pub struct CustomColumnService {
    repo: CustomColumnRepository,
}

impl CustomColumnService {
    pub fn new(repo: CustomColumnRepository) -> Self {
        Self { repo }
    }

    /// Normalise et valide une clé de colonne personnalisée (`[a-z0-9_]+`).
    pub fn normalize_key(raw: &str) -> VellumResult<String> {
        let key: String = raw
            .trim()
            .to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect();
        let key = key.trim_matches('_').to_string();
        if key.is_empty() {
            return Err(VellumError::InvalidArgument(
                "La clé de colonne personnalisée ne peut pas être vide".to_string(),
            ));
        }
        Ok(key)
    }

    /// Crée et persiste une nouvelle définition de colonne personnalisée.
    pub fn define_column(
        &self,
        key: &str,
        label: &str,
        column_type: CustomColumnType,
    ) -> VellumResult<CustomColumnDefinition> {
        let norm_key = Self::normalize_key(key)?;
        let label = label.trim();
        if label.is_empty() {
            return Err(VellumError::InvalidArgument(
                "Le libellé de la colonne personnalisée ne peut pas être vide".to_string(),
            ));
        }
        if self.repo.find_by_key(&norm_key)?.is_some() {
            return Err(VellumError::InvalidArgument(format!(
                "Une colonne avec la clé '{norm_key}' existe déjà"
            )));
        }

        let def = CustomColumnDefinition::new(norm_key, label, column_type);
        self.repo.save_definition(&def)?;
        Ok(def)
    }

    pub fn list_columns(&self) -> VellumResult<Vec<CustomColumnDefinition>> {
        self.repo.list_definitions()
    }

    pub fn delete_column(&self, id: &str) -> VellumResult<bool> {
        self.repo.delete_definition(id)
    }

    /// Définit (ou efface si vide) la valeur d'une colonne personnalisée pour un livre.
    pub fn set_book_value(&self, book_id: &str, key: &str, value: &str) -> VellumResult<()> {
        let norm_key = Self::normalize_key(key)?;
        let def = self
            .repo
            .find_by_key(&norm_key)?
            .ok_or_else(|| VellumError::NotFound(format!("Colonne personnalisée '{norm_key}'")))?;

        let trimmed = value.trim();
        if trimmed.is_empty() {
            self.repo.clear_value(book_id, &def.id)?;
            return Ok(());
        }

        let normalized_val = Self::validate_value(def.column_type, trimmed)?;
        self.repo.set_value(book_id, &def.id, &normalized_val)
    }

    pub fn get_book_values(&self, book_id: &str) -> VellumResult<BTreeMap<String, String>> {
        self.repo.get_values_for_book(book_id)
    }

    fn validate_value(column_type: CustomColumnType, raw: &str) -> VellumResult<String> {
        match column_type {
            CustomColumnType::Text | CustomColumnType::Date => Ok(raw.to_string()),
            CustomColumnType::Number => {
                let parsed: f64 = raw.parse().map_err(|_| {
                    VellumError::InvalidArgument(format!("Nombre invalide : '{raw}'"))
                })?;
                if !parsed.is_finite() {
                    return Err(VellumError::InvalidArgument(
                        "Le nombre doit être fini".to_string(),
                    ));
                }
                Ok(parsed.to_string())
            }
            CustomColumnType::Boolean => match raw.to_lowercase().as_str() {
                "true" | "1" | "oui" | "yes" => Ok("true".to_string()),
                "false" | "0" | "non" | "no" => Ok("false".to_string()),
                _ => Err(VellumError::InvalidArgument(format!(
                    "Booléen invalide : '{raw}'"
                ))),
            },
            CustomColumnType::Rating => {
                let rating: u8 = raw.parse().map_err(|_| {
                    VellumError::InvalidArgument(format!("Note invalide (0-5) : '{raw}'"))
                })?;
                if rating > 5 {
                    return Err(VellumError::InvalidArgument(
                        "La note doit être comprise entre 0 et 5".to_string(),
                    ));
                }
                Ok(rating.to_string())
            }
        }
    }
}

