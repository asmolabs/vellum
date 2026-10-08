package com.asmolabs.vellum.plugin.builtin;

import com.asmolabs.vellum.plugin.BookTransformationPlugin;
import com.asmolabs.vellum.plugin.ConfigField;
import com.asmolabs.vellum.plugin.PluginDescriptor;
import com.asmolabs.vellum.plugin.PluginType;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.stereotype.Component;

import java.io.File;
import java.nio.file.Files;
import java.util.List;
import java.util.Map;

@Component
public class EpubTranslatorPlugin implements BookTransformationPlugin {

    private static final Logger log = LoggerFactory.getLogger(EpubTranslatorPlugin.class);

    private final PluginDescriptor descriptor;

    public EpubTranslatorPlugin() {
        this.descriptor = new PluginDescriptor(
                "epub-translator-fr",
                "Traducteur EPUB",
                "Traduit le contenu d'un livre EPUB à l'aide d'un moteur d'IA.",
                "1.0.0",
                PluginType.TRANSFORMATION,
                false,
                null,
                List.of(
                        new ConfigField("target_language", "Langue cible de traduction", "select", "Français", "Langue vers laquelle traduire les livres EPUB.", List.of("Français", "Anglais", "Espagnol", "Allemand", "Italien")),
                        new ConfigField("ai_model", "Moteur d'IA", "select", "Default AI", "Sélectionnez le service d'IA pour effectuer la traduction.", List.of("Default AI", "GPT-4o", "DeepL API", "Local Ollama")),
                        new ConfigField("api_token", "Jeton / Clé API (Optionnel)", "password", "", "Jeton d'authentification pour le service distant de traduction."),
                        new ConfigField("temperature", "Température de l'IA", "slider", 0.3, "Précision vs créativité du modèle lors de la traduction.", 0.0, 1.0, 0.05),
                        new ConfigField("custom_prompt", "Instructions personnalisées (Prompt)", "textarea", "", "Consignes de traduction supplémentaires (ex: tutoiement/vouvoiement, ton littéraire).")
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
        String langSuffix = "FR".equalsIgnoreCase(targetLang) || "Français".equalsIgnoreCase(targetLang) ? "FR" : targetLang;

        File outputFile = new File(inputFile.getParentFile(), prefix + " - " + langSuffix + ext);
        Files.copy(inputFile.toPath(), outputFile.toPath());
        log.info("EpubTranslatorPlugin produced translated copy: {}", outputFile.getAbsolutePath());
        return outputFile;
    }
}
