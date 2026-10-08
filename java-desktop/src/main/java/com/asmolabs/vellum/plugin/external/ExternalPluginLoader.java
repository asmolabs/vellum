package com.asmolabs.vellum.plugin.external;

import com.asmolabs.vellum.config.AppProperties;
import com.asmolabs.vellum.plugin.*;
import com.fasterxml.jackson.databind.ObjectMapper;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.stereotype.Component;

import java.io.File;
import java.io.IOException;
import java.nio.file.Files;
import java.util.*;

@Component
public class ExternalPluginLoader {

    private static final Logger log = LoggerFactory.getLogger(ExternalPluginLoader.class);
    private static final ObjectMapper objectMapper = new ObjectMapper();

    private final AppProperties appProperties;

    public ExternalPluginLoader(AppProperties appProperties) {
        this.appProperties = appProperties;
    }

    public File getPluginsDirectory() {
        File dir = new File(appProperties.getPluginsPath());
        if (!dir.exists()) {
            dir.mkdirs();
            createSamplePluginDocumentation(dir);
        }
        return dir;
    }

    public List<VellumPlugin> loadExternalPlugins() {
        File pluginsDir = getPluginsDirectory();
        List<VellumPlugin> plugins = new ArrayList<>();

        File[] files = pluginsDir.listFiles();
        if (files == null) {
            return plugins;
        }

        for (File file : files) {
            if (file.isDirectory()) {
                File manifest = new File(file, "plugin.json");
                if (manifest.exists()) {
                    try {
                        PluginDescriptor desc = objectMapper.readValue(manifest, PluginDescriptor.class);
                        // Ensure flagged as external
                        PluginDescriptor extDesc = new PluginDescriptor(
                                desc.id(),
                                desc.name(),
                                desc.description(),
                                desc.version(),
                                desc.type() != null ? desc.type() : PluginType.TRANSFORMATION,
                                true,
                                desc.command(),
                                desc.configSchema()
                        );
                        plugins.add(new ExternalScriptPlugin(file, extDesc));
                        log.info("Discovered external script plugin: {} ({}) in {}", extDesc.name(), extDesc.id(), file.getName());
                    } catch (Exception e) {
                        log.error("Failed to parse plugin.json in {}: {}", file.getAbsolutePath(), e.getMessage());
                    }
                }
            } else if (file.getName().endsWith(".jar")) {
                List<VellumPlugin> jarPlugins = ExternalJarPluginLoader.loadFromJar(file);
                plugins.addAll(jarPlugins);
            }
        }

        return plugins;
    }

    private void createSamplePluginDocumentation(File pluginsDir) {
        try {
            File readme = new File(pluginsDir, "README.md");
            if (!readme.exists()) {
                String content = """
                    # 🔌 Vellum External Plugins Directory
                    
                    Déposez vos plugins personnalisés dans ce dossier pour étendre Vellum !
                    
                    ## 1. Plugins sous forme de Scripts (Python, Bash, Node, binaire...)
                    Créez un sous-dossier (ex: `my-custom-plugin/`) contenant un fichier `plugin.json` :
                    
                    ```json
                    {
                      "id": "my-custom-plugin",
                      "name": "Mon Nettoyeur Personnalisé",
                      "description": "Script qui applique un traitement sur les fichiers EPUB",
                      "version": "1.0.0",
                      "type": "transformation",
                      "command": "python3 script.py",
                      "configSchema": [
                        { "key": "remove_ads", "label": "Supprimer les publicités", "type": "boolean", "defaultValue": true, "description": "Activer la suppression" },
                        { "key": "api_key", "label": "Clé secrète", "type": "password", "defaultValue": "", "description": "Clé d'API masquée" },
                        { "key": "calibre_path", "label": "Binaire Calibre", "type": "file", "defaultValue": "", "description": "Chemin vers l'exécutable" },
                        { "key": "export_dir", "label": "Dossier d'export", "type": "directory", "defaultValue": "", "description": "Dossier cible" },
                        { "key": "threshold", "label": "Seuil de sensibilité", "type": "slider", "defaultValue": 0.5, "min": 0.0, "max": 1.0, "step": 0.05, "description": "Sensibilité" },
                        { "key": "custom_rules", "label": "Règles personnalisées", "type": "textarea", "defaultValue": "", "description": "Règles multilignes" }
                      ]
                    }
                    ```
                    
                    Types de champs supportés pour l'UI :
                    * `text` : champ texte simple
                    * `password` / `secret` : champ masqué avec bouton œil 👁️
                    * `boolean` : case à cocher
                    * `select` : menu déroulant (avec tableau `options`)
                    * `file` : sélecteur de fichier avec bouton "Parcourir..."
                    * `directory` : sélecteur de dossier avec bouton "Parcourir..."
                    * `slider` / `number` : curseur numérique (avec `min`, `max`, `step`)
                    * `textarea` : zone de texte multiligne
                    
                    Lors de l'exécution, Vellum passe les arguments :
                    * `--input <chemin_fichier_source>`
                    * `--output <chemin_fichier_cible>`
                    * `--config <json_des_options>`
                    
                    Et définit les variables d'environnement correspondantes :
                    * `VELLUM_INPUT_FILE`
                    * `VELLUM_OUTPUT_FILE`
                    * `VELLUM_CONFIG_JSON`
                    
                    ## 2. Plugins sous forme de JARs Java
                    Déposez simplement votre fichier `*.jar` implémentant l'interface `com.asmolabs.vellum.plugin.VellumPlugin` dans ce dossier.
                    """.stripIndent();
                Files.writeString(readme.toPath(), content);
            }

            // Create an example plugin directory
            File exampleDir = new File(pluginsDir, "example-python-plugin");
            if (!exampleDir.exists()) {
                exampleDir.mkdirs();
                File manifest = new File(exampleDir, "plugin.json");
                String sampleJson = """
                    {
                      "id": "example-python-plugin",
                      "name": "Exemple Script Python (External)",
                      "description": "Démonstration d'un plugin externe exécutant un script Python avec des paramètres.",
                      "version": "1.0.0",
                      "type": "transformation",
                      "command": "python3 transform.py",
                      "configSchema": [
                        {
                          "key": "prefix_title",
                          "label": "Préfixer le fichier",
                          "type": "boolean",
                          "defaultValue": true,
                          "description": "Ajoute un préfixe de test"
                        }
                      ]
                    }
                    """.stripIndent();
                Files.writeString(manifest.toPath(), sampleJson);

                File script = new File(exampleDir, "transform.py");
                String pyCode = """
                    import sys
                    import os
                    import shutil
                    import argparse

                    parser = argparse.ArgumentParser()
                    parser.add_argument("--input", required=True)
                    parser.add_argument("--output", required=True)
                    parser.add_argument("--config", default="{}")
                    args = parser.parse_args()

                    # Copie simplement le fichier pour l'exemple
                    shutil.copyfile(args.input, args.output)
                    print(f"Plugin execute avec succes pour {args.input}")
                    """.stripIndent();
                Files.writeString(script.toPath(), pyCode);
            }
        } catch (IOException e) {
            log.warn("Could not create plugin sample documentation: {}", e.getMessage());
        }
    }
}
