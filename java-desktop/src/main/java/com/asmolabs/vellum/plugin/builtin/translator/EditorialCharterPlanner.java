package com.asmolabs.vellum.plugin.builtin.translator;

import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.io.File;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;

public class EditorialCharterPlanner {

    private static final Logger log = LoggerFactory.getLogger(EditorialCharterPlanner.class);

    private final LlmClient llmClient;
    private final File charterFile;

    public EditorialCharterPlanner(LlmClient llmClient, File charterFile) {
        this.llmClient = llmClient;
        this.charterFile = charterFile;
    }

    public String getOrGenerateCharter(String title, String author, String excerpt) {
        if (charterFile != null && charterFile.exists() && charterFile.length() > 0) {
            try {
                String existing = Files.readString(charterFile.toPath(), StandardCharsets.UTF_8).trim();
                if (!existing.isBlank()) {
                    log.info("Charte éditoriale existante chargée depuis : {}", charterFile.getName());
                    return existing;
                }
            } catch (Exception e) {
                log.warn("Impossible de lire le fichier de charte {}: {}", charterFile.getName(), e.getMessage());
            }
        }

        String generated = generateCharterWithLlm(title, author, excerpt);

        if (charterFile != null) {
            try {
                Files.writeString(charterFile.toPath(), generated, StandardCharsets.UTF_8);
                log.info("Charte éditoriale sauvegardée dans : {}", charterFile.getAbsolutePath());
            } catch (Exception e) {
                log.warn("Impossible d'enregistrer la charte dans {}: {}", charterFile.getName(), e.getMessage());
            }
        }

        return generated;
    }

    private String generateCharterWithLlm(String title, String author, String excerpt) {
        if (llmClient == null) {
            return getDefaultCharter();
        }

        String safeExcerpt = (excerpt != null) ? excerpt.substring(0, Math.min(excerpt.length(), 3000)) : "Roman littéraire.";

        String systemPrompt = "Tu es un directeur de collection littéraire et traducteur expert.";
        String userPrompt = String.format("""
                Analyse les métadonnées et l'extrait de texte du livre ci-dessous pour concevoir une "Charte Éditoriale et Consignes Stylistiques" pour la traduction de l'anglais vers le français.

                Titre : %s
                Auteur : %s

                Extrait du livre :
                \"\"\"
                %s
                \"\"\"

                Identifie et formule clairement les consignes sur :
                1. Le ton et le style littéraire (registre de langue, rythme des phrases).
                2. Les choix de vouvoiement ou tutoiement selon les relations entre personnages.
                3. Le traitement des expressions idiomatiques ou termes spécifiques.
                4. Les consignes typographiques importantes pour la fluidité en français.

                Réponds uniquement avec le guide de style sous forme de liste à puces synthétique. Ne mets aucune introduction ou conclusion.
                """, title != null ? title : "Inconnu", author != null ? author : "Inconnu", safeExcerpt);

        try {
            log.info("Génération de la charte éditoriale par l'agent planificateur pour « {} »...", title);
            String result = llmClient.generateCompletion(systemPrompt, userPrompt);
            if (result != null && !result.isBlank()) {
                return result.trim();
            }
        } catch (Exception e) {
            log.warn("Échec de génération de la charte éditoriale par le LLM: {}", e.getMessage());
        }

        return getDefaultCharter();
    }

    public static String getDefaultCharter() {
        return """
                - Registre : Conserver le ton littéraire original avec un français soigné et naturel.
                - Dialogue : Privilégier le tutoiement entre proches et camarades, vouvoiement avec les figures d'autorité ou inconnus.
                - Fluidité : Adapter les idiotismes en équivalents français idiomatiques plutôt qu'en calque littéral.
                - Ponctuation : Utiliser des guillemets français ou tirets cadratins pour les répliques directes.
                """.stripIndent();
    }
}
