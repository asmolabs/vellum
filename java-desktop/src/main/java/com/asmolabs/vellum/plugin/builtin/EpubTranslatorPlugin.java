package com.asmolabs.vellum.plugin.builtin;

import com.asmolabs.vellum.plugin.BookTransformationPlugin;
import com.asmolabs.vellum.plugin.ConfigField;
import com.asmolabs.vellum.plugin.PluginDescriptor;
import com.asmolabs.vellum.plugin.PluginType;
import com.asmolabs.vellum.plugin.builtin.translator.EpubTranslationPipeline;
import com.asmolabs.vellum.plugin.builtin.translator.LlmClient;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.stereotype.Component;

import java.io.File;
import java.util.List;
import java.util.Map;

@Component
public class EpubTranslatorPlugin implements BookTransformationPlugin {

    private static final Logger log = LoggerFactory.getLogger(EpubTranslatorPlugin.class);

    private final PluginDescriptor descriptor;

    public EpubTranslatorPlugin() {
        this.descriptor = new PluginDescriptor(
                "epub-translator-fr",
                "Traducteur EPUB Multi-Agents",
                "Pipeline de traduction littéraire avancée d'ebooks EPUB via LLM (Ollama, LM Studio, OpenAI) avec agents Planificateur (charte éditoriale), Glossaire terminologique et Relecteur QA.",
                "1.0.0",
                PluginType.TRANSFORMATION,
                false,
                null,
                List.of(
                        new ConfigField("target_language", "Langue cible de traduction", "select", "Français", "Langue vers laquelle traduire les livres EPUB.", List.of("Français", "Anglais", "Espagnol", "Allemand", "Italien")),
                        new ConfigField("ai_provider", "Fournisseur d'IA", "select", "Local Ollama", "Sélectionnez le moteur LLM d'exécution.", List.of("Local Ollama", "LM Studio", "OpenAI (GPT-4o)", "DeepL API", "Custom OpenAI-compatible")),
                        new ConfigField("api_url", "URL de l'API LLM (Optionnel)", "text", "http://localhost:11434/v1", "URL de base de l'API OpenAI-compatible (ex: http://localhost:11434/v1 ou http://127.0.0.1:1234/v1)."),
                        new ConfigField("model_name", "Nom du modèle LLM", "text", "mistral", "Nom du modèle chargé (ex: mistral, llama3, qwen2.5, gpt-4o, local-model)."),
                        new ConfigField("api_token", "Jeton / Clé API (Optionnel pour local)", "password", "", "Clé d'authentification Bearer (obligatoire pour OpenAI ou service distant)."),
                        new ConfigField("temperature", "Température de l'IA", "slider", 0.3, "Précision vs créativité du modèle lors de la traduction.", 0.0, 1.0, 0.05),
                        new ConfigField("enable_planner", "Agent Planificateur (Charte Éditoriale)", "boolean", true, "Analyse le style du roman et conçoit automatiquement une charte éditoriale (tutoiement, registre, ton)."),
                        new ConfigField("enable_glossary", "Agent Terminologue (Glossaire)", "boolean", true, "Extrait et enrichit dynamiquement les noms de personnages, lieux et termes récurrents pour assurer leur cohérence."),
                        new ConfigField("enable_correction", "Passe de révision stylistique", "boolean", false, "Effectue une seconde passe de relecture et fluidité sur chaque paragraphe traduit."),
                        new ConfigField("enable_qa", "Agent Assurance Qualité (QA)", "boolean", false, "Valide chaque chapitre traduit, vérifie les omissions et applique les corrections recommandées."),
                        new ConfigField("custom_prompt", "Instructions personnalisées (Prompt)", "textarea", "", "Consignes de traduction supplémentaires (ex: tutoiement/vouvoiement, ton littéraire, contraintes lexicales).")
                )
        );
    }

    @Override
    public PluginDescriptor getDescriptor() {
        return descriptor;
    }

    @Override
    public File transform(File inputFile, Map<String, Object> config) throws Exception {
        String baseName = inputFile.getName();
        int dot = baseName.lastIndexOf('.');
        String prefix = (dot > 0) ? baseName.substring(0, dot) : baseName;
        String ext = (dot > 0) ? baseName.substring(dot) : ".epub";

        String targetLang = String.valueOf(config.getOrDefault("target_language", "FR"));
        String langSuffix = "Français".equalsIgnoreCase(targetLang) || "FR".equalsIgnoreCase(targetLang) ? "FR" : targetLang.toUpperCase();

        File outputFile = new File(inputFile.getParentFile(), prefix + " - " + langSuffix + ext);

        log.info("Démarrage du pipeline de traduction littéraire EPUB pour : {}", inputFile.getName());
        EpubTranslationPipeline.PipelineStats stats = EpubTranslationPipeline.translateEpub(inputFile, outputFile, config);
        log.info("Pipeline de traduction achevé avec succès. Résultat : {} (Stats: {} chapitres, {} segments)",
                outputFile.getAbsolutePath(), stats.totalChapters(), stats.segmentsTranslated());

        return outputFile;
    }

    @Override
    public TestResult testConfiguration(Map<String, Object> config) {
        String provider = String.valueOf(config.getOrDefault("ai_provider", config.getOrDefault("ai_model", "Local Ollama")));
        String token = String.valueOf(config.getOrDefault("api_token", "")).trim();

        if ("Default AI".equalsIgnoreCase(provider)) {
            return TestResult.ok("Moteur par défaut prêt (Default AI).");
        }

        if ("OpenAI (GPT-4o)".equalsIgnoreCase(provider) || "GPT-4o".equalsIgnoreCase(provider) || "DeepL API".equalsIgnoreCase(provider)) {
            if (token.isBlank()) {
                return TestResult.error("Une clé d'API / jeton est obligatoire pour utiliser " + provider + ".");
            }
            return TestResult.ok("Clé configurée pour le service " + provider + ".");
        }

        try {
            LlmClient client = EpubTranslationPipeline.buildLlmClient(config);
            boolean reachable = client.testConnection();

            if (reachable) {
                return TestResult.ok("Connexion réussie au fournisseur " + provider + " !");
            } else {
                return TestResult.error("Le serveur " + provider + " ne répond pas à l'adresse spécifiée. Vérifiez qu'il est bien démarré.");
            }
        } catch (Exception e) {
            return TestResult.error("Erreur lors de la vérification de " + provider + " : " + e.getMessage());
        }
    }
}
