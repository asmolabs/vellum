package com.asmolabs.vellum.plugin.builtin.format;

import com.asmolabs.vellum.plugin.PluginDescriptor;
import com.asmolabs.vellum.plugin.PluginType;
import com.asmolabs.vellum.plugin.format.BookFormatPlugin;
import com.asmolabs.vellum.plugin.format.ExtractedMetadata;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.stereotype.Component;
import org.w3c.dom.Document;
import org.w3c.dom.Element;
import org.w3c.dom.NodeList;

import javax.xml.parsers.DocumentBuilder;
import javax.xml.parsers.DocumentBuilderFactory;
import java.io.File;
import java.io.InputStream;
import java.net.URLDecoder;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.List;
import java.util.zip.ZipEntry;
import java.util.zip.ZipFile;

@Component
public class EpubFormatPlugin implements BookFormatPlugin {

    private static final Logger log = LoggerFactory.getLogger(EpubFormatPlugin.class);

    private final PluginDescriptor descriptor;

    public EpubFormatPlugin() {
        this.descriptor = new PluginDescriptor(
                "format-epub",
                "Lecteur de format EPUB (Standard)",
                "Support natif des livres numériques au format EPUB 2 et EPUB 3 avec extraction des métadonnées Dublin Core et couvertures.",
                "1.0.0",
                PluginType.FORMAT,
                false,
                null,
                List.of("epub"),
                List.of()
        );
    }

    @Override
    public PluginDescriptor getDescriptor() {
        return descriptor;
    }

    @Override
    public List<String> getSupportedExtensions() {
        return List.of("epub");
    }

    @Override
    public ExtractedMetadata extractMetadata(File epubFile) throws Exception {
        try (ZipFile zip = new ZipFile(epubFile)) {
            // 1. Read META-INF/container.xml
            ZipEntry containerEntry = zip.getEntry("META-INF/container.xml");
            if (containerEntry == null) {
                throw new IllegalArgumentException("Invalid EPUB: missing META-INF/container.xml");
            }

            DocumentBuilderFactory factory = DocumentBuilderFactory.newInstance();
            factory.setNamespaceAware(true);
            DocumentBuilder builder = factory.newDocumentBuilder();

            Document containerDoc;
            try (InputStream is = zip.getInputStream(containerEntry)) {
                containerDoc = builder.parse(is);
            }

            NodeList rootfiles = containerDoc.getElementsByTagName("rootfile");
            if (rootfiles.getLength() == 0) {
                throw new IllegalArgumentException("Invalid EPUB: no rootfile in container.xml");
            }

            String opfPath = ((Element) rootfiles.item(0)).getAttribute("full-path");

            // 2. Read OPF file
            ZipEntry opfEntry = zip.getEntry(opfPath);
            if (opfEntry == null) {
                opfPath = URLDecoder.decode(opfPath, StandardCharsets.UTF_8);
                opfEntry = zip.getEntry(opfPath);
            }
            if (opfEntry == null) {
                throw new IllegalArgumentException("Invalid EPUB: OPF package file not found at " + opfPath);
            }

            Document opfDoc;
            try (InputStream is = zip.getInputStream(opfEntry)) {
                opfDoc = builder.parse(is);
            }

            // 3. Extract Dublin Core Metadata
            String title = getFirstElementText(opfDoc, "title");
            String author = getFirstElementText(opfDoc, "creator");
            String publisher = getFirstElementText(opfDoc, "publisher");
            String date = getFirstElementText(opfDoc, "date");
            String description = getFirstElementText(opfDoc, "description");

            List<String> tags = new ArrayList<>();
            NodeList subjectNodes = opfDoc.getElementsByTagNameNS("*", "subject");
            if (subjectNodes.getLength() == 0) {
                subjectNodes = opfDoc.getElementsByTagName("dc:subject");
            }
            for (int i = 0; i < subjectNodes.getLength(); i++) {
                String tag = subjectNodes.item(i).getTextContent().trim();
                if (!tag.isBlank() && !tags.contains(tag)) {
                    tags.add(tag);
                }
            }

            // Extract ISBN/Identifier
            String isbn = null;
            NodeList idNodes = opfDoc.getElementsByTagNameNS("*", "identifier");
            if (idNodes.getLength() == 0) {
                idNodes = opfDoc.getElementsByTagName("dc:identifier");
            }
            for (int i = 0; i < idNodes.getLength(); i++) {
                Element el = (Element) idNodes.item(i);
                String idText = el.getTextContent().trim();
                String scheme = el.getAttribute("opf:scheme");
                String idAttr = el.getAttribute("id");
                boolean isIsbn = "isbn".equalsIgnoreCase(scheme)
                        || idAttr.toLowerCase().contains("isbn")
                        || idText.toLowerCase().contains("isbn")
                        || idText.matches("^(97[89])?\\d{9}[\\dXx]$");
                if (isIsbn) {
                    String clean = idText.replaceAll("[^0-9Xx]", "");
                    if (clean.length() == 10 || clean.length() == 13) {
                        isbn = clean;
                        break;
                    }
                }
            }

            // 4. Extract Cover Image
            byte[] coverData = null;
            String coverExt = null;

            String coverHref = findCoverHref(opfDoc);
            if (coverHref != null) {
                Path opfParent = Paths.get(opfPath).getParent();
                String resolvedCoverPath;
                if (opfParent != null) {
                    resolvedCoverPath = opfParent.resolve(coverHref).normalize().toString().replace("\\", "/");
                } else {
                    resolvedCoverPath = coverHref;
                }

                ZipEntry coverEntry = zip.getEntry(resolvedCoverPath);
                if (coverEntry == null) {
                    coverEntry = zip.getEntry(URLDecoder.decode(resolvedCoverPath, StandardCharsets.UTF_8));
                }

                if (coverEntry != null) {
                    try (InputStream is = zip.getInputStream(coverEntry)) {
                        coverData = is.readAllBytes();
                        coverExt = getFileExtension(coverEntry.getName());
                    }
                }
            }

            return new ExtractedMetadata(title, author, publisher, date, description, tags, coverData, coverExt, isbn);
        }
    }

    private String findCoverHref(Document opfDoc) {
        // Strategy A: <item properties="cover-image" ...>
        NodeList items = opfDoc.getElementsByTagName("item");
        for (int i = 0; i < items.getLength(); i++) {
            Element item = (Element) items.item(i);
            String properties = item.getAttribute("properties");
            if (properties != null && properties.contains("cover-image")) {
                return item.getAttribute("href");
            }
        }

        // Strategy B: <meta name="cover" content="item_id" />
        NodeList metas = opfDoc.getElementsByTagName("meta");
        String coverId = null;
        for (int i = 0; i < metas.getLength(); i++) {
            Element meta = (Element) metas.item(i);
            if ("cover".equalsIgnoreCase(meta.getAttribute("name"))) {
                coverId = meta.getAttribute("content");
                break;
            }
        }

        if (coverId != null) {
            for (int i = 0; i < items.getLength(); i++) {
                Element item = (Element) items.item(i);
                if (coverId.equals(item.getAttribute("id"))) {
                    return item.getAttribute("href");
                }
            }
        }

        // Strategy C: item with id containing "cover"
        for (int i = 0; i < items.getLength(); i++) {
            Element item = (Element) items.item(i);
            String id = item.getAttribute("id").toLowerCase();
            String mediaType = item.getAttribute("media-type");
            if (id.contains("cover") && mediaType != null && mediaType.startsWith("image/")) {
                return item.getAttribute("href");
            }
        }

        return null;
    }

    private String getFirstElementText(Document doc, String localName) {
        NodeList nodes = doc.getElementsByTagNameNS("*", localName);
        if (nodes.getLength() == 0) {
            nodes = doc.getElementsByTagName("dc:" + localName);
        }
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
