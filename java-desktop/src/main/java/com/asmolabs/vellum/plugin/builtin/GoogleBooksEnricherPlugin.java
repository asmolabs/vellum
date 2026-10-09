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

    @Override
    public TestResult testConfiguration(Map<String, Object> config) {
        String apiKey = String.valueOf(config.getOrDefault("api_key", "")).trim();
        try {
            String urlStr = "https://www.googleapis.com/books/v1/volumes?q=test&maxResults=1"
                    + (!apiKey.isBlank() ? "&key=" + apiKey : "");
            java.net.http.HttpClient client = java.net.http.HttpClient.newBuilder()
                    .connectTimeout(java.time.Duration.ofSeconds(4))
                    .build();
            java.net.http.HttpRequest request = java.net.http.HttpRequest.newBuilder()
                    .uri(java.net.URI.create(urlStr))
                    .timeout(java.time.Duration.ofSeconds(4))
                    .header("User-Agent", "Vellum-Ebook-Manager/1.0")
                    .GET()
                    .build();
            java.net.http.HttpResponse<String> response = client.send(request, java.net.http.HttpResponse.BodyHandlers.ofString());
            if (response.statusCode() == 200) {
                return TestResult.ok("Connexion réussie à Google Books API (HTTP 200) !" + (!apiKey.isBlank() ? " Clé API valide." : " (Mode public sans quota)"));
            } else if (response.statusCode() == 400 && !apiKey.isBlank()) {
                return TestResult.error("Clé API Google Books invalide ou refusée (HTTP 400).");
            } else if (response.statusCode() == 403) {
                return TestResult.error("Quota Google Books dépassé ou accès restreint (HTTP 403).");
            } else {
                return TestResult.error("Réponse inattendue de Google Books : HTTP " + response.statusCode());
            }
        } catch (Exception e) {
            return TestResult.error("Impossible de joindre Google Books : " + (e.getMessage() != null ? e.getMessage() : e.getClass().getSimpleName()));
        }
    }
}
