package com.asmolabs.vellum.plugin.builtin.converter;

import com.asmolabs.vellum.plugin.ConfigField;
import com.asmolabs.vellum.plugin.PluginDescriptor;
import com.asmolabs.vellum.plugin.PluginType;
import com.asmolabs.vellum.plugin.converter.BookConverterPlugin;
import org.springframework.stereotype.Component;

import java.io.File;
import java.io.FileOutputStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.util.*;
import java.util.zip.CRC32;
import java.util.zip.ZipEntry;
import java.util.zip.ZipOutputStream;

@Component
public class TextToEpubConverterPlugin implements BookConverterPlugin {

    private final PluginDescriptor descriptor = new PluginDescriptor(
            "converter-txt-to-epub",
            "Générateur EPUB (TXT/MD ➔ EPUB)",
            "Structure un document texte brut ou Markdown en véritable livre numérique EPUB avec table des matières et chapitres.",
            "1.0.0",
            PluginType.CONVERSION,
            false,
            null,
            List.of(),
            "txt",
            "epub",
            List.of(
                    new ConfigField("font_family", "Police de lecture", "select", "Georgia, serif", "Famille de police injectée dans la feuille de style.", List.of("Georgia, serif", "sans-serif", "monospace"))
            )
    );

    @Override
    public PluginDescriptor getDescriptor() {
        return descriptor;
    }

    @Override
    public String getSourceFormat() {
        return "txt";
    }

    @Override
    public String getTargetFormat() {
        return "epub";
    }

    @Override
    public boolean canConvert(String fromExt, String toExt) {
        return ("txt".equalsIgnoreCase(fromExt) || "md".equalsIgnoreCase(fromExt)) && "epub".equalsIgnoreCase(toExt);
    }

    @Override
    public File convert(File inputFile, File outputFile, Map<String, Object> config) throws Exception {
        String content = Files.readString(inputFile.toPath(), StandardCharsets.UTF_8);
        String bookTitle = inputFile.getName().replaceFirst("[.][^.]+$", "");
        String bookAuthor = "Auteur inconnu";

        if (config != null) {
            if (config.containsKey("title")) bookTitle = String.valueOf(config.get("title"));
            if (config.containsKey("author")) bookAuthor = String.valueOf(config.get("author"));
        }

        String fontFamily = (config != null && config.containsKey("font_family"))
                ? String.valueOf(config.get("font_family"))
                : "Georgia, serif";

        List<Chapter> chapters = splitIntoChapters(content, bookTitle);

        try (ZipOutputStream zos = new ZipOutputStream(new FileOutputStream(outputFile))) {
            // 1. mimetype (MUST be first entry, uncompressed)
            byte[] mimeBytes = "application/epub+zip".getBytes(StandardCharsets.US_ASCII);
            ZipEntry mimeEntry = new ZipEntry("mimetype");
            mimeEntry.setMethod(ZipEntry.STORED);
            mimeEntry.setSize(mimeBytes.length);
            mimeEntry.setCompressedSize(mimeBytes.length);
            CRC32 crc = new CRC32();
            crc.update(mimeBytes);
            mimeEntry.setCrc(crc.getValue());
            zos.putNextEntry(mimeEntry);
            zos.write(mimeBytes);
            zos.closeEntry();

            // 2. META-INF/container.xml
            String containerXml = """
                <?xml version="1.0" encoding="UTF-8"?>
                <container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
                  <rootfiles>
                    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
                  </rootfiles>
                </container>
                """.stripIndent();
            writeZipEntry(zos, "META-INF/container.xml", containerXml);

            // 3. OEBPS/stylesheet.css
            String css = String.format("""
                body {
                  font-family: %s;
                  line-height: 1.6;
                  margin: 5%%;
                  color: #222;
                }
                h1, h2, h3 {
                  font-family: sans-serif;
                  color: #111;
                  margin-top: 1.5em;
                  margin-bottom: 0.5em;
                }
                p {
                  margin: 0 0 1em 0;
                  text-indent: 1.2em;
                }
                """, fontFamily);
            writeZipEntry(zos, "OEBPS/stylesheet.css", css);

            // 4. Chapters XHTML
            for (int i = 0; i < chapters.size(); i++) {
                Chapter c = chapters.get(i);
                String chapXhtml = buildChapterXhtml(c);
                writeZipEntry(zos, String.format("OEBPS/chapter_%03d.xhtml", i + 1), chapXhtml);
            }

            // 5. OEBPS/content.opf
            String opf = buildOpf(bookTitle, bookAuthor, chapters);
            writeZipEntry(zos, "OEBPS/content.opf", opf);

            // 6. OEBPS/toc.ncx
            String ncx = buildNcx(bookTitle, chapters);
            writeZipEntry(zos, "OEBPS/toc.ncx", ncx);
        }

        return outputFile;
    }

    @Override
    public TestResult testConfiguration(Map<String, Object> config) {
        return TestResult.ok("Générateur EPUB prêt (packaging OCF 2.0 / XHTML pur Java).");
    }

    private void writeZipEntry(ZipOutputStream zos, String name, String content) throws Exception {
        ZipEntry entry = new ZipEntry(name);
        zos.putNextEntry(entry);
        zos.write(content.getBytes(StandardCharsets.UTF_8));
        zos.closeEntry();
    }

    private record Chapter(String title, List<String> paragraphs) {}

    private List<Chapter> splitIntoChapters(String content, String defaultTitle) {
        List<Chapter> chapters = new ArrayList<>();
        String[] lines = content.split("\\r?\\n");

        String currentTitle = defaultTitle;
        List<String> currentParagraphs = new ArrayList<>();
        StringBuilder currentP = new StringBuilder();

        for (String line : lines) {
            String trimmed = line.trim();
            if (trimmed.startsWith("# ") || trimmed.startsWith("## ") || trimmed.toUpperCase().startsWith("CHAPITRE ") || trimmed.toUpperCase().startsWith("CHAPTER ")) {
                if (currentP.length() > 0) {
                    currentParagraphs.add(currentP.toString());
                    currentP.setLength(0);
                }
                if (!currentParagraphs.isEmpty()) {
                    chapters.add(new Chapter(currentTitle, new ArrayList<>(currentParagraphs)));
                    currentParagraphs.clear();
                }
                currentTitle = trimmed.replaceFirst("^#+\\s*", "");
            } else if (trimmed.isBlank()) {
                if (currentP.length() > 0) {
                    currentParagraphs.add(currentP.toString());
                    currentP.setLength(0);
                }
            } else {
                if (currentP.length() > 0) currentP.append(" ");
                currentP.append(trimmed);
            }
        }

        if (currentP.length() > 0) {
            currentParagraphs.add(currentP.toString());
        }
        if (!currentParagraphs.isEmpty() || chapters.isEmpty()) {
            chapters.add(new Chapter(currentTitle, currentParagraphs));
        }

        return chapters;
    }

    private String buildChapterXhtml(Chapter c) {
        StringBuilder sb = new StringBuilder();
        sb.append("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
        sb.append("<!DOCTYPE html>\n");
        sb.append("<html xmlns=\"http://www.w3.org/1999/xhtml\">\n<head>\n");
        sb.append("<title>").append(escapeXml(c.title())).append("</title>\n");
        sb.append("<link rel=\"stylesheet\" type=\"text/css\" href=\"stylesheet.css\"/>\n");
        sb.append("</head>\n<body>\n");
        sb.append("<h1>").append(escapeXml(c.title())).append("</h1>\n");
        for (String p : c.paragraphs()) {
            sb.append("<p>").append(escapeXml(p)).append("</p>\n");
        }
        sb.append("</body>\n</html>");
        return sb.toString();
    }

    private String buildOpf(String title, String author, List<Chapter> chapters) {
        String uuid = UUID.randomUUID().toString();
        StringBuilder sb = new StringBuilder();
        sb.append("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        sb.append("<package xmlns=\"http://www.idpf.org/2007/opf\" unique-identifier=\"BookId\" version=\"2.0\">\n");
        sb.append("  <metadata xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\n");
        sb.append("    <dc:title>").append(escapeXml(title)).append("</dc:title>\n");
        sb.append("    <dc:creator>").append(escapeXml(author)).append("</dc:creator>\n");
        sb.append("    <dc:identifier id=\"BookId\">urn:uuid:").append(uuid).append("</dc:identifier>\n");
        sb.append("    <dc:language>fr</dc:language>\n");
        sb.append("  </metadata>\n");
        sb.append("  <manifest>\n");
        sb.append("    <item id=\"ncx\" href=\"toc.ncx\" media-type=\"application/x-dtbncx+xml\"/>\n");
        sb.append("    <item id=\"css\" href=\"stylesheet.css\" media-type=\"text/css\"/>\n");
        for (int i = 0; i < chapters.size(); i++) {
            sb.append(String.format("    <item id=\"chap%03d\" href=\"chapter_%03d.xhtml\" media-type=\"application/xhtml+xml\"/>\n", i + 1, i + 1));
        }
        sb.append("  </manifest>\n");
        sb.append("  <spine toc=\"ncx\">\n");
        for (int i = 0; i < chapters.size(); i++) {
            sb.append(String.format("    <itemref idref=\"chap%03d\"/>\n", i + 1));
        }
        sb.append("  </spine>\n");
        sb.append("</package>");
        return sb.toString();
    }

    private String buildNcx(String title, List<Chapter> chapters) {
        StringBuilder sb = new StringBuilder();
        sb.append("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        sb.append("<ncx xmlns=\"http://www.daisy.org/z3986/2005/ncx/\" version=\"2005-1\">\n");
        sb.append("  <docTitle><text>").append(escapeXml(title)).append("</text></docTitle>\n");
        sb.append("  <navMap>\n");
        for (int i = 0; i < chapters.size(); i++) {
            sb.append(String.format("    <navPoint id=\"nav%03d\" playOrder=\"%d\">\n", i + 1, i + 1));
            sb.append("      <navLabel><text>").append(escapeXml(chapters.get(i).title())).append("</text></navLabel>\n");
            sb.append(String.format("      <content src=\"chapter_%03d.xhtml\"/>\n", i + 1));
            sb.append("    </navPoint>\n");
        }
        sb.append("  </navMap>\n");
        sb.append("</ncx>");
        return sb.toString();
    }

    private String escapeXml(String text) {
        if (text == null) return "";
        return text.replace("&", "&amp;")
                .replace("<", "&lt;")
                .replace(">", "&gt;")
                .replace("\"", "&quot;")
                .replace("'", "&apos;");
    }
}
