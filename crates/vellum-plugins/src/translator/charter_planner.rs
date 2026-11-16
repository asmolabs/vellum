use vellum_domain::VellumResult;

use super::llm_client::LlmTranslationClient;

pub struct EditorialCharterPlanner;

impl EditorialCharterPlanner {
    pub fn generate_editorial_charter(
        llm: &LlmTranslationClient,
        sample_paragraphs: &[String],
        target_language: &str,
        style_notes: &str,
    ) -> VellumResult<String> {
        if sample_paragraphs.is_empty() {
            return Ok(Self::default_charter(target_language, style_notes));
        }

        let sample_count = sample_paragraphs.len().min(12);
        let sample_text = sample_paragraphs[..sample_count].join("\n\n");

        let system_prompt = format!(
            "Tu es un directeur littéraire et traducteur d'édition expérimenté.\n\
             Analyse cet extrait de livre et rédige une CHARTE ÉDITORIALE DE TRADUCTION concise \
             (maximum 15 lignes) pour traduire cette œuvre en {target_language}.\n\
             La charte doit définir clairement :\n\
             1. Le genre littéraire et le registre de langue (soutenu, courant, familier, lyrique, technique...).\n\
             2. Le système des temps verbaux en {target_language} (ex: passé simple/imparfait vs présent de narration).\n\
             3. La convention typographique des dialogues (ex: tirets cadratins « — » ou guillemets français « ... »).\n\
             4. Le ton général, le rythme des phrases et les pièges de calque syntaxique à éviter.\n\
             5. La politique de traitement des noms propres, lieux et termes culturels.\n\
             {}\n\
             Réponds uniquement avec la charte éditoriale structurée, prête à être injectée comme consigne au traducteur.",
            if style_notes.trim().is_empty() {
                String::new()
            } else {
                format!("Consignes spécifiques de l'utilisateur : {style_notes}")
            }
        );

        match llm.chat(&system_prompt, &sample_text) {
            Ok(response) if !response.trim().is_empty() => Ok(response.trim().to_string()),
            _ => Ok(Self::default_charter(target_language, style_notes)),
        }
    }

    pub fn default_charter(target_language: &str, style_notes: &str) -> String {
        let mut charter = format!(
            "CHARTE ÉDITORIALE ({target_language}) :\n\
             - Registre : Littéraire fluide, naturel et idiomatique en {target_language}.\n\
             - Syntaxe : Éviter tout calque mot-à-mot de l'anglais ; privilégier la cadence naturelle du {target_language}.\n\
             - Dialogues : Respecter la ponctuation et le niveau de langue propre à chaque personnage.\n\
             - Balisage : Conserver strictement les marqueurs inline {{0}}...{{/0}} autour des segments mis en forme."
        );
        if !style_notes.trim().is_empty() {
            charter.push_str("\n- Notes de style : ");
            charter.push_str(style_notes.trim());
        }
        charter
    }
}
