package com.asmolabs.vellum.converter.builtin;

import com.asmolabs.vellum.converter.FormatConverter;
import org.apache.pdfbox.pdmodel.PDDocument;
import org.apache.pdfbox.pdmodel.PDPage;
import org.apache.pdfbox.pdmodel.PDPageContentStream;
import org.apache.pdfbox.pdmodel.common.PDRectangle;
import org.apache.pdfbox.pdmodel.graphics.image.PDImageXObject;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.stereotype.Component;

import java.io.File;
import java.io.IOException;
import java.io.InputStream;
import java.util.Comparator;
import java.util.List;
import java.util.Map;
import java.util.zip.ZipEntry;
import java.util.zip.ZipFile;

@Component
public class CbzToPdfConverter implements FormatConverter {

    private static final Logger log = LoggerFactory.getLogger(CbzToPdfConverter.class);

    @Override
    public String getSourceFormat() {
        return "cbz";
    }

    @Override
    public String getTargetFormat() {
        return "pdf";
    }

    @Override
    public File convert(File inputFile, File outputFile, Map<String, Object> options) throws Exception {
        try (ZipFile zip = new ZipFile(inputFile);
             PDDocument doc = new PDDocument()) {

            List<? extends ZipEntry> imageEntries = zip.stream()
                    .filter(e -> !e.isDirectory() && isImage(e.getName()))
                    .sorted(Comparator.comparing(ZipEntry::getName, String.CASE_INSENSITIVE_ORDER))
                    .toList();

            if (imageEntries.isEmpty()) {
                throw new IOException("Aucune page image trouvée dans le fichier CBZ : " + inputFile.getName());
            }

            for (ZipEntry entry : imageEntries) {
                byte[] bytes;
                try (InputStream is = zip.getInputStream(entry)) {
                    bytes = is.readAllBytes();
                }
                try {
                    PDImageXObject img = PDImageXObject.createFromByteArray(doc, bytes, entry.getName());
                    float width = img.getWidth();
                    float height = img.getHeight();

                    PDPage page = new PDPage(new PDRectangle(width, height));
                    doc.addPage(page);

                    try (PDPageContentStream cs = new PDPageContentStream(doc, page)) {
                        cs.drawImage(img, 0, 0, width, height);
                    }
                } catch (Exception e) {
                    log.warn("Ignorer l'image non supportée {} dans {}: {}", entry.getName(), inputFile.getName(), e.getMessage());
                }
            }

            doc.save(outputFile);
        }
        return outputFile;
    }

    private boolean isImage(String name) {
        String lower = name.toLowerCase();
        return lower.endsWith(".jpg") || lower.endsWith(".jpeg") || lower.endsWith(".png")
                || lower.endsWith(".webp") || lower.endsWith(".bmp") || lower.endsWith(".gif");
    }
}
