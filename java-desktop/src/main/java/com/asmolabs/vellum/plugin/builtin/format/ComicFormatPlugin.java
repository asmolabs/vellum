package com.asmolabs.vellum.plugin.builtin.format;

import com.asmolabs.vellum.plugin.PluginDescriptor;
import com.asmolabs.vellum.plugin.PluginType;
import com.asmolabs.vellum.plugin.format.BookFormatPlugin;
import com.asmolabs.vellum.plugin.format.ExtractedMetadata;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.stereotype.Component;
import org.w3c.dom.Document;
import org.w3c.dom.NodeList;

import javax.xml.parsers.DocumentBuilder;
import javax.xml.parsers.DocumentBuilderFactory;
import java.io.File;
import java.io.InputStream;
import java.util.*;
import java.util.zip.ZipEntry;
import java.util.zip.ZipFile;

@Component
public class ComicFormatPlugin implements BookFormatPlugin {

    private static final Logger log = LoggerFactory.getLogger(ComicFormatPlugin.class);

    private final PluginDescriptor descriptor;

    public ComicFormatPlugin() {
        this.descriptor = new PluginDescriptor(
                "format-comic",
                "Lecteur de Bandes Dessinées (CBZ / CBR)",
                "Support des archives de bande dessinée CBZ et CBR avec extraction de ComicInfo.xml et de la première page en couverture.",
                "1.0.0",
                PluginType.FORMAT,
                false,
                null,
                List.of("cbz", "cbr"),
                List.of()
        );
    }

    @Override
    public PluginDescriptor getDescriptor() {
        return descriptor;
    }

    @Override
    public List<String> getSupportedExtensions() {
        return List.of("cbz", "cbr");
    }

    @Override
    public ExtractedMetadata extractMetadata(File cbzFile) throws Exception {
        String title = null;
        String author = null;
        String description = null;
        List<String> tags = new ArrayList<>();
        tags.add("BD / Manga");

        byte[] coverData = null;
        String coverExt = ".jpg";

        try (ZipFile zip = new ZipFile(cbzFile)) {
            // Check for ComicInfo.xml
            ZipEntry comicInfoEntry = zip.getEntry("ComicInfo.xml");
            if (comicInfoEntry == null) {
                Enumeration<? extends ZipEntry> entries = zip.entries();
                while (entries.hasMoreElements()) {
                    ZipEntry e = entries.nextElement();
                    if (e.getName().equalsIgnoreCase("ComicInfo.xml")) {
                        comicInfoEntry = e;
                        break;
                    }
                }
            }

            if (comicInfoEntry != null) {
                try (InputStream is = zip.getInputStream(comicInfoEntry)) {
                    DocumentBuilderFactory factory = DocumentBuilderFactory.newInstance();
                    DocumentBuilder builder = factory.newDocumentBuilder();
                    Document doc = builder.parse(is);

                    String series = getFirstElementText(doc, "Series");
                    String number = getFirstElementText(doc, "Number");
                    String xmlTitle = getFirstElementText(doc, "Title");
                    String writer = getFirstElementText(doc, "Writer");
                    String summary = getFirstElementText(doc, "Summary");
                    String genre = getFirstElementText(doc, "Genre");

                    if (xmlTitle != null && !xmlTitle.isBlank()) {
                        title = (series != null ? series + " - " : "") + xmlTitle + (number != null ? " #" + number : "");
                    } else if (series != null && !series.isBlank()) {
                        title = series + (number != null ? " #" + number : "");
                    }

                    if (writer != null && !writer.isBlank()) author = writer;
                    if (summary != null) description = summary;
                    if (genre != null) {
                        for (String g : genre.split("[,;]")) {
                            if (!g.trim().isBlank() && !tags.contains(g.trim())) {
                                tags.add(g.trim());
                            }
                        }
                    }
                } catch (Exception ignored) {}
            }

            // Find first image entry as cover
            List<ZipEntry> imageEntries = new ArrayList<>();
            Enumeration<? extends ZipEntry> entries = zip.entries();
            while (entries.hasMoreElements()) {
                ZipEntry e = entries.nextElement();
                String name = e.getName().toLowerCase();
                if (name.endsWith(".jpg") || name.endsWith(".jpeg") || name.endsWith(".png") || name.endsWith(".webp")) {
                    imageEntries.add(e);
                }
            }
            imageEntries.sort(Comparator.comparing(ZipEntry::getName));

            if (!imageEntries.isEmpty()) {
                ZipEntry firstImg = imageEntries.get(0);
                try (InputStream is = zip.getInputStream(firstImg)) {
                    coverData = is.readAllBytes();
                    coverExt = getFileExtension(firstImg.getName());
                    if (coverExt.isBlank()) coverExt = ".jpg";
                }
            }
        }

        return new ExtractedMetadata(title, author, null, null, description, tags, coverData, coverExt);
    }

    private String getFirstElementText(Document doc, String localName) {
        NodeList nodes = doc.getElementsByTagName(localName);
        if (nodes.getLength() > 0) {
            return nodes.item(0).getTextContent().trim();
        }
        return null;
    }

    private String getFileExtension(String filename) {
        int dot = filename.lastIndexOf('.');
        return (dot > 0) ? filename.substring(dot).toLowerCase() : "";
    }
}
