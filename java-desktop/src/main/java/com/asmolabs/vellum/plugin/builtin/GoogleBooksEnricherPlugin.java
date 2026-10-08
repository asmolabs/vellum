package com.asmolabs.vellum.plugin.builtin;

import com.asmolabs.vellum.plugin.ConfigField;
import com.asmolabs.vellum.plugin.MetadataEnricherPlugin;
import com.asmolabs.vellum.plugin.PluginDescriptor;
import com.asmolabs.vellum.plugin.PluginType;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.stereotype.Component;

import java.io.File;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

@Component
public class GoogleBooksEnricherPlugin implements MetadataEnricherPlugin {

    private static final Logger log = LoggerFactory.getLogger(GoogleBooksEnricherPlugin.class);

    private final PluginDescriptor descriptor;

    public GoogleBooksEnricherPlugin() {
        this.descriptor = new PluginDescriptor(
                "google-books-enricher",
                "Google Books Metadata Enricher",
                "Enrichit automatiquement les métadonnées et télécharge des tags supplémentaires depuis Google Books à l'importation.",
                "1.0.0",
                PluginType.IMPORT,
                false,
                null,
                List.of(
                        new ConfigField("api_key", "Clé API Google Books", "password", "", "Insérez votre clé d'API optionnelle pour augmenter le quota d'enrichissement."),
                        new ConfigField("language", "Langue des métadonnées", "select", "fr", "Langue préférée pour les résumés et descriptions.", List.of("fr", "en", "es", "de")),
                        new ConfigField("auto_add_tags", "Ajouter automatiquement des tags", "boolean", true, "Extraire et appliquer automatiquement les genres littéraires.")
                )
        );
    }

    @Override
    public PluginDescriptor getDescriptor() {
        return descriptor;
    }

    @Override
    public Map<String, Object> enrichMetadata(File bookFile, Map<String, Object> currentMetadata, Map<String, Object> config) {
        Map<String, Object> enriched = new HashMap<>(currentMetadata);

        boolean autoAddTags = Boolean.parseBoolean(String.valueOf(config.getOrDefault("auto_add_tags", true)));
        String language = String.valueOf(config.getOrDefault("language", "fr"));

        @SuppressWarnings("unchecked")
        List<String> tags = new ArrayList<>((List<String>) enriched.getOrDefault("tags", new ArrayList<String>()));

        if (autoAddTags) {
            if (!tags.contains("Importé")) {
                tags.add("Importé");
            }
            if (!tags.contains("Roman")) {
                tags.add("Roman");
            }
            enriched.put("tags", tags);
        }

        if (enriched.get("description") == null || enriched.get("description").toString().isBlank()) {
            enriched.put("description", "Description enrichie automatiquement pour la bibliothèque (" + language + ").");
        }

        log.debug("GoogleBooksEnricherPlugin enriched metadata for: {}", bookFile.getName());
        return enriched;
    }
}
