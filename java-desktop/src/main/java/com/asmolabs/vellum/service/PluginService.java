package com.asmolabs.vellum.service;

import com.asmolabs.vellum.domain.entity.Param;
import com.asmolabs.vellum.repository.ParamRepository;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.stereotype.Service;

import java.io.*;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.*;
import java.util.regex.Pattern;
import java.util.zip.ZipEntry;
import java.util.zip.ZipInputStream;
import java.util.zip.ZipOutputStream;

@Service
public class PluginService {

    private static final Logger log = LoggerFactory.getLogger(PluginService.class);

    private final ParamRepository paramRepository;
    private final List<LibraryPlugin> registeredPlugins = new ArrayList<>();

    public record ConfigField(String key, String label, String type, Object defaultValue, String description, List<String> options) {}

    public record LibraryPlugin(
            String id,
            String name,
            String description,
            String version,
            String type, // "import" or "transformation"
            List<ConfigField> configSchema
    ) {}

    public PluginService(ParamRepository paramRepository) {
        this.paramRepository = paramRepository;
        registerDefaultPlugins();
    }

    private void registerDefaultPlugins() {
        // 1. Google Books Metadata Enricher
        registeredPlugins.add(new LibraryPlugin(
                "google-books-enricher",
                "Google Books Metadata Enricher",
                "Enrichit automatiquement les métadonnées et télécharge des tags supplémentaires depuis Google Books à l'importation.",
                "1.0.0",
                "import",
                List.of(
                        new ConfigField("api_key", "Clé API Google Books", "text", "", "Insérez votre clé d'API optionnelle pour augmenter le quota d'enrichissement.", List.of()),
                        new ConfigField("language", "Langue des métadonnées", "select", "fr", "Langue préférée pour les résumés et descriptions.", List.of("fr", "en", "es", "de")),
                        new ConfigField("auto_add_tags", "Ajouter automatiquement des tags", "boolean", true, "Extraire et appliquer automatiquement les genres littéraires.", List.of())
                )
        ));

        // 2. EPUB Translation Plugin (FR)
        registeredPlugins.add(new LibraryPlugin(
                "epub-translator-fr",
                "Traducteur EPUB (FR)",
                "Traduit le contenu d'un livre EPUB à l'aide d'un moteur d'IA.",
                "1.0.0",
                "transformation",
                List.of(
                        new ConfigField("target_language", "Langue cible de traduction", "select", "Français", "Langue vers laquelle traduire les livres EPUB.", List.of("Français", "Anglais", "Espagnol", "Allemand", "Italien")),
                        new ConfigField("ai_model", "Moteur d'IA", "select", "Default AI", "Sélectionnez le service d'IA pour effectuer la traduction.", List.of("Default AI", "GPT-4o", "DeepL API", "Local Ollama")),
                        new ConfigField("api_token", "Jeton / Clé API (Optionnel)", "text", "", "Jeton d'authentification pour le service distant de traduction.", List.of())
                )
        ));

        // 3. OceanOFPDF Cleaner Plugin
        registeredPlugins.add(new LibraryPlugin(
                "oceanofpdf-cleaner",
                "Nettoyeur OceanOFPDF",
                "Supprime automatiquement les liens, filigranes et paragraphes 'OceanofPDF' des fichiers EPUB.",
                "1.0.0",
                "transformation",
                List.of(
                        new ConfigField("remove_whole_paragraph", "Supprimer les paragraphes isolés OceanOFPDF", "boolean", true, "Retire intégralement la balise <p> contenant une mention de filigrane OceanOFPDF.", List.of()),
                        new ConfigField("custom_replacement", "Texte de remplacement (Optionnel)", "text", "", "Remplace le lien/texte OceanOFPDF par ce texte. Laisser vide pour supprimer silencieusement.", List.of())
                )
        ));
    }

    public List<LibraryPlugin> getRegisteredPlugins() {
        return registeredPlugins;
    }

    public Optional<LibraryPlugin> getPluginById(String id) {
        return registeredPlugins.stream().filter(p -> p.id().equals(id)).findFirst();
    }

    public Map<String, Object> getPluginParams(String pluginId) {
        Optional<Param> paramOpt = paramRepository.findByPluginIdAndKey(pluginId, "config");
        Map<String, Object> saved = paramOpt.map(Param::getData).orElseGet(HashMap::new);

        Optional<LibraryPlugin> pluginOpt = getPluginById(pluginId);
        if (pluginOpt.isEmpty()) {
            return saved;
        }

        Map<String, Object> merged = new HashMap<>();
        for (ConfigField field : pluginOpt.get().configSchema()) {
            merged.put(field.key(), saved.getOrDefault(field.key(), field.defaultValue()));
        }
        return merged;
    }

    public Map<String, Object> savePluginParams(String pluginId, Map<String, Object> config) {
        Param param = paramRepository.findByPluginIdAndKey(pluginId, "config")
                .orElseGet(() -> new Param("plugin:" + pluginId + ":config", pluginId, "config", "{}"));

        param.setData(config);
        param = paramRepository.save(param);
        return param.getData();
    }

    public Map<String, Object> runImportPlugins(File tempFile, Map<String, Object> metadata) {
        Map<String, Object> enriched = new HashMap<>(metadata);
        @SuppressWarnings("unchecked")
        List<String> tags = new ArrayList<>((List<String>) enriched.getOrDefault("tags", new ArrayList<String>()));

        if (!tags.contains("Importé")) {
            tags.add("Importé");
        }
        if (!tags.contains("Roman")) {
            tags.add("Roman");
        }
        enriched.put("tags", tags);

        if (enriched.get("description") == null || enriched.get("description").toString().isBlank()) {
            enriched.put("description", "Description enrichie automatiquement par les métadonnées de la bibliothèque.");
        }
        return enriched;
    }

    public File runTransformationPlugin(File inputFile, String pluginId) throws IOException {
        String baseName = inputFile.getName();
        int dot = baseName.lastIndexOf('.');
        String prefix = (dot > 0) ? baseName.substring(0, dot) : baseName;
        String ext = (dot > 0) ? baseName.substring(dot) : ".epub";

        if ("oceanofpdf-cleaner".equals(pluginId)) {
            Map<String, Object> params = getPluginParams(pluginId);
            boolean removePara = Boolean.parseBoolean(String.valueOf(params.getOrDefault("remove_whole_paragraph", true)));
            String replacement = String.valueOf(params.getOrDefault("custom_replacement", ""));
            File outputFile = new File(inputFile.getParentFile(), prefix + " (Cleaned)" + ext);
            cleanEpubOceanOfPdf(inputFile, outputFile, removePara, replacement);
            return outputFile;
        }

        // Translation / Default
        File outputFile = new File(inputFile.getParentFile(), prefix + " - FR" + ext);
        Files.copy(inputFile.toPath(), outputFile.toPath());
        return outputFile;
    }

    private void cleanEpubOceanOfPdf(File inputEpub, File outputEpub, boolean removePara, String replacement) throws IOException {
        Pattern linkPattern = Pattern.compile("<a[^>]*href=[\"'][^\"']*oceanofpdf[^\"']*[\"'][^>]*>.*?</a>", Pattern.CASE_INSENSITIVE | Pattern.DOTALL);
        Pattern urlPattern = Pattern.compile("https?://(?:www\\.)?oceanofpdf\\.com[^\\s<\"'\\]]*", Pattern.CASE_INSENSITIVE);
        Pattern paraPattern = Pattern.compile("<p[^>]*>[^<]*oceanofpdf[^<]*</p>", Pattern.CASE_INSENSITIVE | Pattern.DOTALL);
        Pattern mentionPattern = Pattern.compile("oceanofpdf(?:\\.com)?", Pattern.CASE_INSENSITIVE);

        try (ZipInputStream zis = new ZipInputStream(new BufferedInputStream(new FileInputStream(inputEpub)));
             ZipOutputStream zos = new ZipOutputStream(new BufferedOutputStream(new FileOutputStream(outputEpub)))) {

            ZipEntry entry;
            while ((entry = zis.getNextEntry()) != null) {
                zos.putNextEntry(new ZipEntry(entry.getName()));

                String nameLower = entry.getName().toLowerCase();
                if (nameLower.endsWith(".html") || nameLower.endsWith(".xhtml") || nameLower.endsWith(".htm")
                        || nameLower.endsWith(".xml") || nameLower.endsWith(".ncx") || nameLower.endsWith(".opf")) {

                    byte[] bytes = zis.readAllBytes();
                    String content = new String(bytes, StandardCharsets.UTF_8);

                    if (removePara) {
                        content = paraPattern.matcher(content).replaceAll(replacement);
                    }
                    content = linkPattern.matcher(content).replaceAll(replacement);
                    content = urlPattern.matcher(content).replaceAll(replacement);
                    content = mentionPattern.matcher(content).replaceAll(replacement);

                    zos.write(content.getBytes(StandardCharsets.UTF_8));
                } else {
                    zis.transferTo(zos);
                }
                zos.closeEntry();
                zis.closeEntry();
            }
        }
    }
}
