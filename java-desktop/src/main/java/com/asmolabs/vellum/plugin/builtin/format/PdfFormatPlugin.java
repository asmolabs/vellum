package com.asmolabs.vellum.plugin.builtin.format;

import com.asmolabs.vellum.plugin.PluginDescriptor;
import com.asmolabs.vellum.plugin.PluginType;
import com.asmolabs.vellum.plugin.format.BookFormatPlugin;
import com.asmolabs.vellum.plugin.format.ExtractedMetadata;
import org.apache.pdfbox.Loader;
import org.apache.pdfbox.pdmodel.PDDocument;
import org.apache.pdfbox.pdmodel.PDDocumentInformation;
import org.apache.pdfbox.rendering.PDFRenderer;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.stereotype.Component;

import javax.imageio.ImageIO;
import java.awt.image.BufferedImage;
import java.io.ByteArrayOutputStream;
import java.io.File;
import java.util.ArrayList;
import java.util.List;

@Component
public class PdfFormatPlugin implements BookFormatPlugin {

    private static final Logger log = LoggerFactory.getLogger(PdfFormatPlugin.class);

    private final PluginDescriptor descriptor;

    public PdfFormatPlugin() {
        this.descriptor = new PluginDescriptor(
                "format-pdf",
                "Lecteur de format PDF (Adobe / Document)",
                "Support des documents PDF avec rendu haute fidélité de la couverture et extraction des métadonnées intégrées via Apache PDFBox.",
                "1.0.0",
                PluginType.FORMAT,
                false,
                null,
                List.of("pdf"),
                List.of()
        );
    }

    @Override
    public PluginDescriptor getDescriptor() {
        return descriptor;
    }

    @Override
    public List<String> getSupportedExtensions() {
        return List.of("pdf");
    }

    @Override
    public ExtractedMetadata extractMetadata(File pdfFile) throws Exception {
        String title = null;
        String author = null;
        String publisher = null;
        String publishDate = null;
        String description = null;
        List<String> tags = new ArrayList<>();
        tags.add("PDF");

        byte[] coverData = null;
        String coverExt = ".jpg";

        try (PDDocument doc = Loader.loadPDF(pdfFile)) {
            PDDocumentInformation info = doc.getDocumentInformation();
            if (info != null) {
                if (info.getTitle() != null && !info.getTitle().isBlank()) {
                    title = info.getTitle().trim();
                }
                if (info.getAuthor() != null && !info.getAuthor().isBlank()) {
                    author = info.getAuthor().trim();
                }
                if (info.getProducer() != null && !info.getProducer().isBlank()) {
                    publisher = info.getProducer().trim();
                }
                if (info.getSubject() != null && !info.getSubject().isBlank()) {
                    description = info.getSubject().trim();
                }
                if (info.getCreationDate() != null) {
                    publishDate = String.valueOf(info.getCreationDate().get(java.util.Calendar.YEAR));
                }
            }

            // Render first page as cover image
            if (doc.getNumberOfPages() > 0) {
                PDFRenderer renderer = new PDFRenderer(doc);
                BufferedImage image = renderer.renderImageWithDPI(0, 100);
                ByteArrayOutputStream baos = new ByteArrayOutputStream();
                ImageIO.write(image, "JPEG", baos);
                coverData = baos.toByteArray();
            }
        }

        return new ExtractedMetadata(title, author, publisher, publishDate, description, tags, coverData, coverExt);
    }
}
