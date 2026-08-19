package com.asmolabs.vellum.service;

import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.stereotype.Service;
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
import java.util.*;
import java.util.zip.ZipEntry;
import java.util.zip.ZipFile;

@Service
public class MetadataService {

    private static final Logger log = LoggerFactory.getLogger(MetadataService.class);

    public record ExtractedMetadata(
            String title,
            String author,
            String publisher,
            String publishDate,
            String description,
            List<String> tags,
            byte[] coverData,
            String coverExt
    ) {}

    public ExtractedMetadata parseMetadata(File file) {
        String filename = file.getName();
        String ext = getFileExtension(filename).toLowerCase();

        if (".epub".equals(ext)) {
            try {
                return parseEpub(file);
            } catch (Exception e) {
                log.warn("Error parsing EPUB metadata for {}: {}", file.getAbsolutePath(), e.getMessage());
            }
        }

        // Fallback parser from filename: "Author - Title.ext"
        String baseName = filename.contains(".") ? filename.substring(0, filename.lastIndexOf('.')) : filename;
        String[] parts = baseName.split(" - ", 2);
        String author = parts.length > 1 ? parts[0].trim() : "Inconnu";
        String title = parts.length > 1 ? parts[1].trim() : baseName.trim();

        return new ExtractedMetadata(title, author, null, null, null, new ArrayList<>(), null, null);
    }

    private ExtractedMetadata parseEpub(File epubFile) throws Exception {
        try (ZipFile zip = new ZipFile(epubFile)) {
            // 1. Read META-INF/container.xml
            ZipEntry containerEntry = zip.getEntry("META-INF/container.xml");
            if (containerEntry == null) {
                throw new IllegalStateException("META-INF/container.xml not found in EPUB");
            }

            DocumentBuilderFactory dbf = DocumentBuilderFactory.newInstance();
            dbf.setNamespaceAware(true);
            DocumentBuilder db = dbf.newDocumentBuilder();

            Document containerDoc;
            try (InputStream is = zip.getInputStream(containerEntry)) {
                containerDoc = db.parse(is);
            }

            NodeList rootfileNodes = containerDoc.getElementsByTagNameNS("*", "rootfile");
            if (rootfileNodes.getLength() == 0) {
                rootfileNodes = containerDoc.getElementsByTagName("rootfile");
            }
            if (rootfileNodes.getLength() == 0) {
                throw new IllegalStateException("rootfile entry not found in container.xml");
            }

            Element rootfileEl = (Element) rootfileNodes.item(0);
            String opfPath = rootfileEl.getAttribute("full-path");
            if (opfPath == null || opfPath.isBlank()) {
                throw new IllegalStateException("full-path attribute missing in container.xml");
            }

            // 2. Read OPF File
            ZipEntry opfEntry = zip.getEntry(opfPath);
            if (opfEntry == null) {
                // Try case-insensitive or forward slashes
                opfEntry = findZipEntryIgnoreCase(zip, opfPath);
            }
            if (opfEntry == null) {
                throw new IllegalStateException("OPF file not found at " + opfPath);
            }

            Path opfDirPath = Paths.get(opfPath).getParent();
            String opfDir = opfDirPath != null ? opfDirPath.toString().replace('\\', '/') : "";

            Document opfDoc;
            try (InputStream is = zip.getInputStream(opfEntry)) {
                opfDoc = db.parse(is);
            }

            // 3. Extract Dublin Core Metadata
            String title = getFirstElementText(opfDoc, "title");
            if (title == null || title.isBlank()) title = "Sans titre";

            String author = getFirstElementText(opfDoc, "creator");
            if (author == null || author.isBlank()) author = "Inconnu";

            String publisher = getFirstElementText(opfDoc, "publisher");
            String publishDate = getFirstElementText(opfDoc, "date");
            String description = getFirstElementText(opfDoc, "description");
            List<String> tags = getAllElementsText(opfDoc, "subject");

            // 4. Extract Cover Image
            byte[] coverData = null;
            String coverExt = ".jpg";

            String coverId = null;
            NodeList metaNodes = opfDoc.getElementsByTagNameNS("*", "meta");
            for (int i = 0; i < metaNodes.getLength(); i++) {
                Element metaEl = (Element) metaNodes.item(i);
                if ("cover".equalsIgnoreCase(metaEl.getAttribute("name"))) {
                    coverId = metaEl.getAttribute("content");
                    break;
                }
            }

            String coverHref = null;
            NodeList itemNodes = opfDoc.getElementsByTagNameNS("*", "item");
            if (itemNodes.getLength() == 0) {
                itemNodes = opfDoc.getElementsByTagName("item");
            }

            if (coverId != null) {
                for (int i = 0; i < itemNodes.getLength(); i++) {
                    Element itemEl = (Element) itemNodes.item(i);
                    if (coverId.equalsIgnoreCase(itemEl.getAttribute("id"))) {
                        coverHref = itemEl.getAttribute("href");
                        break;
                    }
                }
            }

            // Fallback: search for item with properties/id/href containing 'cover'
            if (coverHref == null) {
                for (int i = 0; i < itemNodes.getLength(); i++) {
                    Element itemEl = (Element) itemNodes.item(i);
                    String id = itemEl.getAttribute("id").toLowerCase();
                    String props = itemEl.getAttribute("properties").toLowerCase();
                    String href = itemEl.getAttribute("href").toLowerCase();
                    String mediaType = itemEl.getAttribute("media-type").toLowerCase();

                    if ((id.contains("cover") || props.contains("cover") || href.contains("cover"))
                            && mediaType.startsWith("image/")) {
                        coverHref = itemEl.getAttribute("href");
                        break;
                    }
                }
            }

            if (coverHref != null) {
                coverHref = URLDecoder.decode(coverHref, StandardCharsets.UTF_8);
                String fullCoverPath = opfDir.isEmpty() ? coverHref : opfDir + "/" + coverHref;
                fullCoverPath = normalizeZipPath(fullCoverPath);

                ZipEntry coverEntry = zip.getEntry(fullCoverPath);
                if (coverEntry == null) {
                    coverEntry = findZipEntryIgnoreCase(zip, fullCoverPath);
                }

                if (coverEntry != null) {
                    try (InputStream is = zip.getInputStream(coverEntry)) {
                        coverData = is.readAllBytes();
                        coverExt = getFileExtension(coverHref);
                        if (coverExt.isBlank()) coverExt = ".jpg";
                    }
                }
            }

            return new ExtractedMetadata(title, author, publisher, publishDate, description, tags, coverData, coverExt);
        }
    }

    private String getFirstElementText(Document doc, String localName) {
        NodeList nodes = doc.getElementsByTagNameNS("*", localName);
        if (nodes.getLength() == 0) {
            nodes = doc.getElementsByTagName("dc:" + localName);
        }
        if (nodes.getLength() == 0) {
            nodes = doc.getElementsByTagName(localName);
        }
        if (nodes.getLength() > 0) {
            String text = nodes.item(0).getTextContent();
            return text != null ? text.trim() : null;
        }
        return null;
    }

    private List<String> getAllElementsText(Document doc, String localName) {
        List<String> results = new ArrayList<>();
        NodeList nodes = doc.getElementsByTagNameNS("*", localName);
        if (nodes.getLength() == 0) {
            nodes = doc.getElementsByTagName("dc:" + localName);
        }
        if (nodes.getLength() == 0) {
            nodes = doc.getElementsByTagName(localName);
        }
        for (int i = 0; i < nodes.getLength(); i++) {
            String text = nodes.item(i).getTextContent();
            if (text != null && !text.isBlank()) {
                results.add(text.trim());
            }
        }
        return results;
    }

    private ZipEntry findZipEntryIgnoreCase(ZipFile zip, String path) {
        String normalized = path.replace('\\', '/');
        Enumeration<? extends ZipEntry> entries = zip.entries();
        while (entries.hasMoreElements()) {
            ZipEntry e = entries.nextElement();
            if (e.getName().equalsIgnoreCase(normalized)) {
                return e;
            }
        }
        return null;
    }

    private String normalizeZipPath(String path) {
        Path p = Paths.get(path).normalize();
        return p.toString().replace('\\', '/');
    }

    private String getFileExtension(String name) {
        int idx = name.lastIndexOf('.');
        return (idx >= 0) ? name.substring(idx) : "";
    }
}
