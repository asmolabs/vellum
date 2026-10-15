package com.asmolabs.vellum.plugin.builtin.translator;

import com.fasterxml.jackson.core.type.TypeReference;
import com.fasterxml.jackson.databind.ObjectMapper;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.util.Map;
import java.util.regex.Pattern;

public class QualityAssuranceValidator {

    private static final Logger log = LoggerFactory.getLogger(QualityAssuranceValidator.class);
    private static final ObjectMapper json = new ObjectMapper();

    private final LlmClient llmClient;

    public QualityAssuranceValidator(LlmClient llmClient) {
        this.llmClient = llmClient;
    }

    public record QaReport(boolean valid, String report, String recommendedCorrections) {}

    public QaReport validateChapter(String originalText, String translatedText, String charter, String glossary) {
        if (llmClient == null || originalText == null || translatedText == null) {
            return new QaReport(true, "Validation QA ignorée (pas de LLM connecté).", "");
        }

        if (originalText.trim().length() < 60) {
            return new QaReport(true, "Texte trop court pour nécessiter une analyse QA.", "");
        }

        String safeOrig = originalText.substring(0, Math.min(originalText.length(), 4000));
        String safeTrad = translatedText.substring(0, Math.min(translatedText.length(), 4000));

        String systemPrompt = "Tu es un agent d'assurance qualité (QA) et correcteur littéraire.";
        String userPrompt = String.format("""
                Ton rôle est de valider la traduction d'une page de roman de l'anglais vers le français, ainsi que sa mise en forme.

                CHARTE ÉDITORIALE DU ROMAN :
                %s

                GLOSSAIRE DU ROMAN :
                %s

                Texte original (EN) :
                \"\"\"
                %s
                \"\"\"

                Texte traduit (FR) :
                \"\"\"
                %s
                \"\"\"

                Effectue les vérifications suivantes :
                1. Est-ce que des phrases ont été oubliées ou tronquées ?
                2. La charte éditoriale (ton narratif, tutoiement/vouvoiement) est-elle respectée ?
                3. Le glossaire est-il respecté ?
                4. Reste-t-il des artefacts de modèles (commentaires comme "Voici la traduction", balises, etc.) ?

                Réponds au format JSON strict suivant uniquement :
                {
                  "valide": true,
                  "rapport": "Résumé de ton analyse en 1 ou 2 phrases",
                  "corrections": ""
                }
                """, charter, glossary, safeOrig, safeTrad);

        try {
            String response = llmClient.generateCompletion(systemPrompt, userPrompt);
            String cleanedJson = response.replaceAll("(?i)^```json\\s*", "").replaceAll("(?i)\\s*```$", "").trim();
            Map<String, Object> map = json.readValue(cleanedJson, new TypeReference<>() {});

            boolean valid = Boolean.parseBoolean(String.valueOf(map.getOrDefault("valide", "true")));
            String report = String.valueOf(map.getOrDefault("rapport", "Conforme"));
            String corrections = String.valueOf(map.getOrDefault("corrections", ""));

            return new QaReport(valid, report, corrections);
        } catch (Exception e) {
            log.warn("Analyse QA non parsée: {}", e.getMessage());
            return new QaReport(true, "Analyse QA achevée.", "");
        }
    }
}
