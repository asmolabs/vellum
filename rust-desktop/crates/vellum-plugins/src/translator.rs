pub mod charter_planner;
pub mod glossary_manager;
pub mod html_parser;
pub mod llm_client;
pub mod pipeline;
pub mod qa_validator;
pub mod segment_cache;

use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::info;
use vellum_domain::{
    BookTransformationPlugin, ConfigField, PluginDescriptor, PluginType, TestResult, VellumPlugin,
    VellumResult,
};

pub struct EpubTranslatorPlugin {
    descriptor: PluginDescriptor,
}

impl Default for EpubTranslatorPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl EpubTranslatorPlugin {
    pub fn new() -> Self {
        Self {
            descriptor: PluginDescriptor {
                id: "epub-translator-fr".to_string(),
                name: "Traducteur EPUB Multi-Agents".to_string(),
                description: "Pipeline de traduction littéraire avancée d'ebooks EPUB via LLM (Ollama, LM Studio, OpenAI) avec agents Planificateur (charte éditoriale), Glossaire terminologique et Relecteur QA.".to_string(),
                version: "1.0.0".to_string(),
                plugin_type: PluginType::Transformation,
                external: false,
                command: None,
                supported_extensions: Vec::new(),
                source_format: None,
                target_format: None,
                config_schema: vec![
                    ConfigField::new(
                        "target_language",
                        "Langue cible de traduction",
                        "select",
                        json!("Français"),
                        "Langue vers laquelle traduire les livres EPUB.",
                    )
                    .with_options(vec!["Français", "Anglais", "Espagnol", "Allemand", "Italien"]),
                    ConfigField::new(
                        "ai_provider",
                        "Fournisseur d'IA",
                        "select",
                        json!("Local Ollama"),
                        "Sélectionnez le moteur LLM d'exécution.",
                    )
                    .with_options(vec![
                        "Local Ollama",
                        "LM Studio",
                        "OpenAI (GPT-4o)",
                        "DeepL API",
                        "Custom OpenAI-compatible",
                    ]),
                    ConfigField::new(
                        "api_url",
                        "URL de l'API LLM (Optionnel)",
                        "text",
                        json!("http://localhost:11434/v1"),
                        "URL de base de l'API OpenAI-compatible (ex: http://localhost:11434/v1 ou http://127.0.0.1:1234/v1).",
                    ),
                    ConfigField::new(
                        "model_name",
                        "Nom du modèle LLM",
                        "text",
                        json!("mistral"),
                        "Nom du modèle chargé (ex: mistral, llama3, qwen2.5, gpt-4o, local-model).",
                    ),
                    ConfigField::new(
                        "api_token",
                        "Jeton / Clé API (Optionnel pour local)",
                        "password",
                        json!(""),
                        "Clé d'authentification Bearer (obligatoire pour OpenAI ou service distant).",
                    ),
                    ConfigField::new(
                        "temperature",
                        "Température de l'IA",
                        "slider",
                        json!(0.3),
                        "Précision vs créativité du modèle lors de la traduction.",
                    )
                    .with_range(0.0, 1.0, 0.05),
                    ConfigField::new(
                        "enable_planner",
                        "Agent Planificateur (Charte Éditoriale)",
                        "boolean",
                        json!(true),
                        "Analyse le style du roman et conçoit automatiquement une charte éditoriale (tutoiement, registre, ton).",
                    ),
                    ConfigField::new(
                        "enable_glossary",
                        "Agent Terminologue (Glossaire)",
                        "boolean",
                        json!(true),
                        "Extrait et enrichit dynamiquement les noms de personnages, lieux et termes récurrents pour assurer leur cohérence.",
                    ),
                    ConfigField::new(
                        "enable_correction",
                        "Passe de révision stylistique",
                        "boolean",
                        json!(false),
                        "Effectue une seconde passe de relecture et fluidité sur chaque paragraphe traduit.",
                    ),
                    ConfigField::new(
                        "enable_qa",
                        "Agent Assurance Qualité (QA)",
                        "boolean",
                        json!(false),
                        "Valide chaque chapitre traduit, vérifie les omissions et applique les corrections recommandées.",
                    ),
                    ConfigField::new(
                        "custom_prompt",
                        "Instructions personnalisées (Prompt)",
                        "textarea",
                        json!(""),
                        "Consignes de traduction supplémentaires (ex: tutoiement/vouvoiement, ton littéraire, contraintes lexicales).",
                    ),
                ],
            },
        }
    }
}

impl VellumPlugin for EpubTranslatorPlugin {
    fn descriptor(&self) -> &PluginDescriptor {
        &self.descriptor
    }

    fn test_configuration(&self, config: &HashMap<String, Value>) -> TestResult {
        let provider = config
            .get("ai_provider")
            .or_else(|| config.get("ai_model"))
            .and_then(|v| v.as_str())
            .unwrap_or("Local Ollama");

        let token = config
            .get("api_token")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .trim();

        if provider.eq_ignore_ascii_case("Default AI") {
            return TestResult::ok("Moteur par défaut prêt (Default AI).");
        }

        if provider.eq_ignore_ascii_case("OpenAI (GPT-4o)")
            || provider.eq_ignore_ascii_case("GPT-4o")
            || provider.eq_ignore_ascii_case("DeepL API")
        {
            if token.is_empty() {
                return TestResult::error(format!(
                    "Une clé d'API / jeton est obligatoire pour utiliser {provider}."
                ));
            }
            return TestResult::ok(format!("Clé configurée pour le service {provider}."));
        }

        let str_config: HashMap<String, String> = config
            .iter()
            .map(|(k, v)| {
                let val = match v {
                    Value::String(s) => s.clone(),
                    Value::Bool(b) => b.to_string(),
                    Value::Number(n) => n.to_string(),
                    _ => v.to_string(),
                };
                (k.clone(), val)
            })
            .collect();

        let client = pipeline::build_llm_client(&str_config);
        if client.test_connection() {
            TestResult::ok(format!("Connexion réussie au fournisseur {provider} !"))
        } else {
            TestResult::error(format!(
                "Le serveur {provider} ne répond pas à l'adresse spécifiée. Vérifiez qu'il est bien démarré."
            ))
        }
    }

    fn as_transformation_plugin(&self) -> Option<&dyn BookTransformationPlugin> {
        Some(self)
    }
}

impl BookTransformationPlugin for EpubTranslatorPlugin {
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
        let target_lang = config
            .get("target_language")
            .and_then(|v| v.as_str())
            .unwrap_or("FR");
        let lang_suffix =
            if target_lang.eq_ignore_ascii_case("Français") || target_lang.eq_ignore_ascii_case("FR")
            {
                "FR".to_string()
            } else {
                target_lang.to_uppercase()
            };

        let parent = input_file.parent().unwrap_or_else(|| Path::new("."));
        let output_file = parent.join(format!("{stem} - {lang_suffix}.{ext}"));

        info!(
            "Démarrage du pipeline de traduction littéraire EPUB pour : {}",
            input_file.display()
        );
        let str_config: HashMap<String, String> = config
            .iter()
            .map(|(k, v)| {
                let val = match v {
                    Value::String(s) => s.clone(),
                    Value::Bool(b) => b.to_string(),
                    Value::Number(n) => n.to_string(),
                    _ => v.to_string(),
                };
                (k.clone(), val)
            })
            .collect();

        let stats = pipeline::translate_epub(input_file, &output_file, &str_config)?;
        info!(
            "Pipeline de traduction achevé : {} (Chapitres: {}, Paragraphes: {})",
            output_file.display(),
            stats.chapters_processed,
            stats.paragraphs_translated
        );
        Ok(output_file)
    }
}
