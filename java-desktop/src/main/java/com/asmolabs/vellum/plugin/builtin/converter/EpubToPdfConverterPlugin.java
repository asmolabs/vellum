package com.asmolabs.vellum.plugin.builtin.converter;

import com.asmolabs.vellum.plugin.ConfigField;
import com.asmolabs.vellum.plugin.PluginDescriptor;
import com.asmolabs.vellum.plugin.PluginType;
import com.asmolabs.vellum.plugin.converter.BookConverterPlugin;
import org.apache.pdfbox.pdmodel.PDDocument;
import org.apache.pdfbox.pdmodel.PDPage;
import org.apache.pdfbox.pdmodel.PDPageContentStream;
import org.apache.pdfbox.pdmodel.common.PDRectangle;
import org.apache.pdfbox.pdmodel.font.PDFont;
import org.apache.pdfbox.pdmodel.font.PDType1Font;
import org.apache.pdfbox.pdmodel.font.Standard14Fonts;
import org.springframework.stereotype.Component;

import java.io.File;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.util.*;
import java.util.zip.ZipEntry;
import java.util.zip.ZipFile;

@Component
public class EpubToPdfConverterPlugin implements BookConverterPlugin {

    private final PluginDescriptor descriptor = new PluginDescriptor(
            "converter-epub-to-pdf",
            "Générateur Document (EPUB ➔ PDF)",
            "Convertit et pagine le texte d'un livre EPUB en document PDF A4 prêt à l'impression avec gestion des sauts de page.",
            "1.0.0",
            PluginType.CONVERSION,
            false,
            null,
            List.of(),
            "epub",
            "pdf",
            List.of(
                    new ConfigField("font_size", "Taille du texte", "number", 10.0, "Taille de police en points (8 à 16).", List.of(), 8.0, 16.0, 1.0)
            )
    );

    @Override
    public PluginDescriptor getDescriptor() {
        return descriptor;
    }

    @Override
    public String getSourceFormat() {
        return "epub";
    }

    @Override
    public String getTargetFormat() {
        return "pdf";
    }

    @Override
    public File convert(File inputFile, File outputFile, Map<String, Object> config) throws Exception {
        List<String> textLines = extractTextLinesFromEpub(inputFile);

        float fontSize = 10f;
        if (config != null && config.containsKey("font_size")) {
            try {
                fontSize = ((Number) config.get("font_size")).floatValue();
            } catch (Exception e) {
                try {
                    fontSize = Float.parseFloat(String.valueOf(config.get("font_size")));
                } catch (Exception ignored) {}
            }
        }

        try (PDDocument doc = new PDDocument()) {
            PDFont font = new PDType1Font(Standard14Fonts.FontName.HELVETICA);
            PDFont boldFont = new PDType1Font(Standard14Fonts.FontName.HELVETICA_BOLD);

            float leading = fontSize * 1.4f;
            float margin = 50f;
            PDRectangle mediaBox = PDRectangle.A4;
            float width = mediaBox.getWidth();
            float height = mediaBox.getHeight();
            float contentWidth = width - 2 * margin;

            PDPage page = new PDPage(mediaBox);
            doc.addPage(page);

            PDPageContentStream cs = new PDPageContentStream(doc, page);
            float y = height - margin;

            for (String rawLine : textLines) {
                String line = rawLine.trim();
                boolean isHeader = line.startsWith("#");

                if (isHeader) {
                    line = line.replaceFirst("^#+\\s*", "");
                    y -= leading * 1.5f;
                }

                if (line.isBlank()) {
                    y -= leading * 0.7f;
                    continue;
                }

                PDFont currentFont = isHeader ? boldFont : font;
                float currentFontSize = isHeader ? fontSize * 1.4f : fontSize;
                float currentLeading = isHeader ? leading * 1.3f : leading;

                List<String> wrapped = wrapLine(line, currentFont, currentFontSize, contentWidth);

                for (String wLine : wrapped) {
                    if (y - currentLeading < margin) {
                        cs.close();
                        page = new PDPage(mediaBox);
                        doc.addPage(page);
                        cs = new PDPageContentStream(doc, page);
                        y = height - margin;
                    }

                    cs.beginText();
                    cs.setFont(currentFont, currentFontSize);
                    cs.newLineAtOffset(margin, y);
                    cs.showText(cleanTextForPdf(wLine));
                    cs.endText();
                    y -= currentLeading;
                }
            }

            cs.close();
            doc.save(outputFile);
        }

        return outputFile;
    }

    @Override
    public TestResult testConfiguration(Map<String, Object> config) {
        return TestResult.ok("Moteur de pagination PDFBox prêt (génération A4 vectorielle).");
    }

    private List<String> extractTextLinesFromEpub(File epubFile) {
        List<String> lines = new ArrayList<>();
        try (ZipFile zip = new ZipFile(epubFile)) {
            List<? extends ZipEntry> htmlEntries = zip.stream()
                    .filter(e -> !e.isDirectory() && (e.getName().endsWith(".xhtml") || e.getName().endsWith(".html")))
                    .sorted(Comparator.comparing(ZipEntry::getName))
                    .toList();

            for (ZipEntry entry : htmlEntries) {
                try (InputStream is = zip.getInputStream(entry)) {
                    String html = new String(is.readAllBytes(), StandardCharsets.UTF_8);
                    String text = htmlToPlainText(html);
                    for (String l : text.split("\\r?\\n")) {
                        lines.add(l);
                    }
                }
            }
        } catch (Exception ignored) {}
        return lines;
    }

    private String htmlToPlainText(String html) {
        String clean = html.replaceAll("(?is)<style[^>]*>.*?</style>", "")
                .replaceAll("(?is)<script[^>]*>.*?</script>", "");

        clean = clean.replaceAll("(?i)<h[1-3][^>]*>(.*?)</h[1-3]>", "\n\n# $1\n\n");
        clean = clean.replaceAll("(?i)<p[^>]*>", "\n");
        clean = clean.replaceAll("(?i)</p>", "\n");
        clean = clean.replaceAll("(?i)<br\\s*/?>", "\n");
        clean = clean.replaceAll("<[^>]+>", "");

        clean = clean.replace("&nbsp;", " ")
                .replace("&amp;", "&")
                .replace("&lt;", "<")
                .replace("&gt;", ">")
                .replace("&quot;", "\"")
                .replace("&#39;", "'")
                .replace("&rsquo;", "'")
                .replace("&lsquo;", "'");

        return clean.trim();
    }

    private List<String> wrapLine(String text, PDFont font, float fontSize, float maxWidth) {
        List<String> result = new ArrayList<>();
        String[] words = text.split("\\s+");
        StringBuilder current = new StringBuilder();

        for (String word : words) {
            String candidate = current.isEmpty() ? word : current + " " + word;
            try {
                float w = (font.getStringWidth(cleanTextForPdf(candidate)) / 1000f) * fontSize;
                if (w > maxWidth && !current.isEmpty()) {
                    result.add(current.toString());
                    current = new StringBuilder(word);
                } else {
                    current = new StringBuilder(candidate);
                }
            } catch (Exception e) {
                current.append(" ").append(word);
            }
        }
        if (!current.isEmpty()) {
            result.add(current.toString());
        }
        return result;
    }

    private String cleanTextForPdf(String text) {
        StringBuilder sb = new StringBuilder();
        for (char c : text.toCharArray()) {
            if (c >= 32 && c <= 255) {
                sb.append(c);
            } else if (c == '’' || c == '‘') {
                sb.append('\'');
            } else if (c == '“' || c == '”') {
                sb.append('"');
            } else if (c == '—' || c == '–') {
                sb.append('-');
            } else if (c == '…') {
                sb.append("...");
            } else if (c > 255) {
                sb.append(' ');
            }
        }
        return sb.toString();
    }
}
