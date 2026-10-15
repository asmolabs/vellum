package com.asmolabs.vellum.plugin.builtin.translator;

import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.io.File;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.util.LinkedHashSet;
import java.util.Set;

public class GlossaryManager {

    private static final Logger log = LoggerFactory.getLogger(GlossaryManager.class);

    private final LlmClient llmClient;
    private final File glossaryFile;
    private String currentGlossary = "";

    public GlossaryManager(LlmClient llmClient, File glossaryFile) {
        this.llmClient = llmClient;
        this.glossaryFile = glossaryFile;
    }

    public synchronized String getOrGenerateGlossary(String title, String author) {
        if (glossaryFile != null && glossaryFile.exists() && glossaryFile.length() > 0) {
            try {
                String existing = Files.readString(glossaryFile.toPath(), StandardCharsets.UTF_8).trim();
                if (!existing.isBlank()) {
                    log.info("Glossaire existant chargé depuis : {}", glossaryFile.getName());
                    this.currentGlossary = existing;
                    return existing;
                }
            } catch (Exception e) {
                log.warn("Impossible de lire le glossaire {}: {}", glossaryFile.getName(), e.getMessage());
            }
        }

        String generated = generateGlossaryWithLlm(title, author);
        this.currentGlossary = generated;

        if (glossaryFile != null) {
            try {
                Files.writeString(glossaryFile.toPath(), generated, StandardCharsets.UTF_8);
                log.info("Glossaire sauvegardé dans : {}", glossaryFile.getAbsolutePath());
            } catch (Exception e) {
                log.warn("Impossible d'enregistrer le glossaire dans {}: {}", glossaryFile.getName(), e.getMessage());
            }
        }

        return generated;
    }

    private String generateGlossaryWithLlm(String title, String author) {
        if (llmClient == null) {
            return getDefaultGlossary();
        }

        String systemPrompt = "Tu es un expert littéraire et terminologue.";
        String userPrompt = String.format("""
                Génère un glossaire de traduction de l'anglais vers le français pour le livre suivant :
                Titre : %s
                Auteur : %s

                Identifie les personnages principaux, les lieux clés et les termes spécifiques propres à l'intrigue et propose leur traduction recommandée en français.
                Formatte le résultat exactement sous cette forme :
                - "Nom original" -> "Nom traduit" (Explication rapide si nécessaire)

                Ne mets aucun autre texte d'introduction ou de conclusion. Réponds uniquement avec le glossaire formaté.
                """, title != null ? title : "Inconnu", author != null ? author : "Inconnu");

        try {
            log.info("Génération du glossaire par l'agent terminologique pour « {} »...", title);
            String result = llmClient.generateCompletion(systemPrompt, userPrompt);
            if (result != null && !result.isBlank()) {
                return result.trim();
            }
        } catch (Exception e) {
            log.warn("Échec de génération du glossaire par le LLM: {}", e.getMessage());
        }

        return getDefaultGlossary();
    }

    public synchronized void enrichGlossary(String originalText, String translatedText) {
        if (llmClient == null || originalText == null || translatedText == null) return;
        if (originalText.length() < 100 || translatedText.length() < 100) return;

        String safeOrig = originalText.substring(0, Math.min(originalText.length(), 2000));
        String safeTrad = translatedText.substring(0, Math.min(translatedText.length(), 2000));

        String systemPrompt = "Tu es un Agent de Terminologie Littéraire.";
        String userPrompt = String.format("""
                Compare l'original et la traduction ci-dessous pour identifier d'éventuels nouveaux noms de personnages principaux, noms de lieux importants, ou termes clés récurrents propres à l'intrigue qui ne figurent pas encore dans le glossaire actuel.

                Glossaire actuel :
                \"\"\"
                %s
                \"\"\"

                Texte Original (EN) :
                \"\"\"
                %s
                \"\"\"

                Texte Traduit (FR) :
                \"\"\"
                %s
                \"\"\"

                S'il y a de nouveaux termes pertinents à ajouter, renvoie-les sous ce format exact (un par ligne) :
                - "Nom original" -> "Nom traduit" (Explication rapide si nécessaire)

                Si aucun nouveau terme important ne doit être ajouté, réponds simplement par "RIEN". Ne mets aucune introduction ni conclusion.
                """, currentGlossary, safeOrig, safeTrad);

        try {
            String response = llmClient.generateCompletion(systemPrompt, userPrompt).trim();
            if (!response.isBlank() && !response.equalsIgnoreCase("RIEN") && !response.contains("RIEN")) {
                Set<String> newLines = new LinkedHashSet<>();
                for (String line : response.split("\\r?\\n")) {
                    String tr = line.trim();
                    if (tr.startsWith("-") && tr.contains("->") && !currentGlossary.contains(tr)) {
                        newLines.add(tr);
                    }
                }

                if (!newLines.isEmpty()) {
                    log.info("Agent Glossaire : {} nouveaux termes détectés.", newLines.size());
                    StringBuilder sb = new StringBuilder(currentGlossary);
                    for (String item : newLines) {
                        sb.append("\n").append(item);
                    }
                    this.currentGlossary = sb.toString().trim();

                    if (glossaryFile != null) {
                        Files.writeString(glossaryFile.toPath(), this.currentGlossary, StandardCharsets.UTF_8);
                    }
                }
            }
        } catch (Exception e) {
            log.debug("Enrichissement dynamique du glossaire ignoré: {}", e.getMessage());
        }
    }

    public synchronized String getCurrentGlossary() {
        return (currentGlossary != null && !currentGlossary.isBlank()) ? currentGlossary : getDefaultGlossary();
    }

    public static String getDefaultGlossary() {
        return """
                - "Wand" -> "Baguette"
                - CONTEXTE : Roman littéraire, veiller à l'harmonie des noms propres non traduits et toponymes universels.
                """.stripIndent();
    }
}
