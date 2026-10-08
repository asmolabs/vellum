package com.asmolabs.vellum.service;

import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.stereotype.Service;
import org.w3c.dom.Document;
import org.w3c.dom.Element;
import org.w3c.dom.NodeList;

import javax.xml.parsers.DocumentBuilder;
import javax.xml.parsers.DocumentBuilderFactory;
import org.apache.pdfbox.Loader;
import org.apache.pdfbox.pdmodel.PDDocument;
import org.apache.pdfbox.pdmodel.PDDocumentInformation;
import org.apache.pdfbox.rendering.PDFRenderer;

import javax.imageio.ImageIO;
import java.awt.image.BufferedImage;
import java.io.ByteArrayOutputStream;
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
        } else if (".cbz".equals(ext) || ".cbr".equals(ext)) {
            try {
                return parseCbz(file);
            } catch (Exception e) {
                log.warn("Error parsing CBZ metadata for {}: {}", file.getAbsolutePath(), e.getMessage());
            }
        } else if (".pdf".equals(ext)) {
            try {
                return parsePdf(file);
            } catch (Exception e) {
                log.warn("Error parsing PDF metadata for {}: {}", file.getAbsolutePath(), e.getMessage());
            }
        }

        // Fallback parser from filename: "Author - Title.ext"
        String baseName = filename.contains(".") ? filename.substring(0, filename.lastIndexOf('.')) : filename;
        String[] parts = baseName.split(" - ", 2);
        String author = parts.length > 1 ? parts[0].trim() : "Inconnu";
        String title = parts.length > 1 ? parts[1].trim() : baseName.trim();

        return new ExtractedMetadata(title, author, null, null, null, new ArrayList<>(), null, null);
    }

    private ExtractedMetadata parsePdf(File file) throws Exception {
        String filename = file.getName();
        String baseName = filename.contains(".") ? filename.substring(0, filename.lastIndexOf('.')) : filename;
        String[] parts = baseName.split(" - ", 2);
        String defaultAuthor = parts.length > 1 ? parts[0].trim() : "Inconnu";
        String defaultTitle = parts.length > 1 ? parts[1].trim() : baseName.trim();

        try (PDDocument document = Loader.loadPDF(file)) {
            PDDocumentInformation info = document.getDocumentInformation();
            String title = (info != null && info.getTitle() != null && !info.getTitle().isBlank()) ? info.getTitle().trim() : defaultTitle;
            String author = (info != null && info.getAuthor() != null && !info.getAuthor().isBlank()) ? info.getAuthor().trim() : defaultAuthor;
            String subject = (info != null) ? info.getSubject() : null;
            String producer = (info != null) ? info.getProducer() : null;

            List<String> tags = new ArrayList<>();
            tags.add("PDF");
            if (info != null && info.getKeywords() != null && !info.getKeywords().isBlank()) {
                String[] kw = info.getKeywords().split("[,;]");
                for (String k : kw) {
                    if (!k.trim().isBlank()) tags.add(k.trim());
                }
            }

            // Render first page as cover
            byte[] coverData = null;
            if (document.getNumberOfPages() > 0) {
                try {
                    PDFRenderer renderer = new PDFRenderer(document);
                    BufferedImage image = renderer.renderImageWithDPI(0, 110);
                    ByteArrayOutputStream baos = new ByteArrayOutputStream();
                    ImageIO.write(image, "jpg", baos);
                    coverData = baos.toByteArray();
                } catch (Exception e) {
                    log.warn("Could not render first page of PDF: {}", e.getMessage());
                }
            }

            return new ExtractedMetadata(title, author, producer, null, subject, tags, coverData, ".jpg");
        }
    }

    private ExtractedMetadata parseCbz(File file) throws Exception {
        String filename = file.getName();
        String baseName = filename.contains(".") ? filename.substring(0, filename.lastIndexOf('.')) : filename;
        String[] parts = baseName.split(" - ", 2);
        String defaultAuthor = parts.length > 1 ? parts[0].trim() : "Inconnu";
        String defaultTitle = parts.length > 1 ? parts[1].trim() : baseName.trim();

        String title = defaultTitle;
        String author = defaultAuthor;
        String description = null;
        List<String> tags = new ArrayList<>(List.of("BD / Manga"));
        byte[] coverData = null;
        String coverExt = ".jpg";

        try (ZipFile zip = new ZipFile(file)) {
            // Find ComicInfo.xml if present
            ZipEntry comicInfo = zip.getEntry("ComicInfo.xml");
            if (comicInfo != null) {
                try (InputStream is = zip.getInputStream(comicInfo)) {
                    DocumentBuilderFactory dbf = DocumentBuilderFactory.newInstance();
                    Document doc = dbf.newDocumentBuilder().parse(is);
                    String xmlTitle = getFirstElementText(doc, "Title");
                    String series = getFirstElementText(doc, "Series");
                    String number = getFirstElementText(doc, "Number");
                    String writer = getFirstElementText(doc, "Writer");
                    String summary = getFirstElementText(doc, "Summary");
                    String genre = getFirstElementText(doc, "Genre");

                    if (xmlTitle != null && !xmlTitle.isBlank()) {
                        title = (series != null ? series + " - " : "") + xmlTitle + (number != null ? " #" + number : "");
                    }
                    if (writer != null && !writer.isBlank()) author = writer;
                    if (summary != null) description = summary;
                    if (genre != null) {
                        for (String g : genre.split("[,;]")) {
                            if (!g.trim().isBlank()) tags.add(g.trim());
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
