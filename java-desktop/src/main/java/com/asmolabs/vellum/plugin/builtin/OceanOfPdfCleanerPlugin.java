package com.asmolabs.vellum.plugin.builtin;

import com.asmolabs.vellum.plugin.BookTransformationPlugin;
import com.asmolabs.vellum.plugin.ConfigField;
import com.asmolabs.vellum.plugin.PluginDescriptor;
import com.asmolabs.vellum.plugin.PluginType;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.stereotype.Component;

import java.io.*;
import java.nio.charset.StandardCharsets;
import java.util.List;
import java.util.Map;
import java.util.regex.Pattern;
import java.util.zip.ZipEntry;
import java.util.zip.ZipInputStream;
import java.util.zip.ZipOutputStream;

@Component
public class OceanOfPdfCleanerPlugin implements BookTransformationPlugin {

    private static final Logger log = LoggerFactory.getLogger(OceanOfPdfCleanerPlugin.class);

    private final PluginDescriptor descriptor;

    public OceanOfPdfCleanerPlugin() {
        this.descriptor = new PluginDescriptor(
                "oceanofpdf-cleaner",
                "Nettoyeur OceanOFPDF",
                "Supprime automatiquement les liens, filigranes et paragraphes 'OceanofPDF' des fichiers EPUB.",
                "1.0.0",
                PluginType.TRANSFORMATION,
                false,
                null,
                List.of(
                        new ConfigField("remove_whole_paragraph", "Supprimer les paragraphes isolés OceanOFPDF", "boolean", true, "Retire intégralement la balise <p> contenant une mention de filigrane OceanOFPDF."),
                        new ConfigField("custom_replacement", "Texte de remplacement (Optionnel)", "text", "", "Remplace le lien/texte OceanOFPDF par ce texte. Laisser vide pour supprimer silencieusement.")
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

        File outputFile = new File(inputFile.getParentFile(), prefix + " (Cleaned)" + ext);

        boolean removePara = Boolean.parseBoolean(String.valueOf(config.getOrDefault("remove_whole_paragraph", true)));
        String replacement = String.valueOf(config.getOrDefault("custom_replacement", ""));

        cleanEpub(inputFile, outputFile, removePara, replacement);
        log.info("OceanOfPdfCleanerPlugin completed: {}", outputFile.getAbsolutePath());
        return outputFile;
    }

    private void cleanEpub(File inputEpub, File outputEpub, boolean removePara, String replacement) throws IOException {
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
