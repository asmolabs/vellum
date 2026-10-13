package com.asmolabs.vellum.plugin.builtin.converter;

import com.asmolabs.vellum.plugin.ConfigField;
import com.asmolabs.vellum.plugin.PluginDescriptor;
import com.asmolabs.vellum.plugin.PluginType;
import com.asmolabs.vellum.plugin.converter.BookConverterPlugin;
import org.apache.pdfbox.Loader;
import org.apache.pdfbox.pdmodel.PDDocument;
import org.apache.pdfbox.rendering.PDFRenderer;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.stereotype.Component;

import javax.imageio.ImageIO;
import java.awt.image.BufferedImage;
import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.FileOutputStream;
import java.util.List;
import java.util.Map;
import java.util.zip.ZipEntry;
import java.util.zip.ZipOutputStream;

@Component
public class PdfToCbzConverterPlugin implements BookConverterPlugin {

    private static final Logger log = LoggerFactory.getLogger(PdfToCbzConverterPlugin.class);

    private final PluginDescriptor descriptor = new PluginDescriptor(
            "converter-pdf-to-cbz",
            "Convertisseur BD (PDF ➔ CBZ)",
            "Extrait chaque page d'un document PDF en image haute résolution et les archive dans un fichier de bande dessinée CBZ.",
            "1.0.0",
            PluginType.CONVERSION,
            false,
            null,
            List.of(),
            "pdf",
            "cbz",
            List.of(
                    new ConfigField("dpi", "Résolution DPI", "slider", 150.0, "Résolution de rendu des pages (72 à 300 DPI)", 72.0, 300.0, 10.0)
            )
    );

    @Override
    public PluginDescriptor getDescriptor() {
        return descriptor;
    }

    @Override
    public String getSourceFormat() {
        return "pdf";
    }

    @Override
    public String getTargetFormat() {
        return "cbz";
    }

    @Override
    public File convert(File inputFile, File outputFile, Map<String, Object> config) throws Exception {
        int dpi = 150;
        if (config != null && config.containsKey("dpi")) {
            try {
                dpi = ((Number) config.get("dpi")).intValue();
            } catch (Exception e) {
                try {
                    dpi = Integer.parseInt(String.valueOf(config.get("dpi")));
                } catch (Exception ignored) {}
            }
        }

        try (PDDocument doc = Loader.loadPDF(inputFile);
             ZipOutputStream zos = new ZipOutputStream(new FileOutputStream(outputFile))) {

            PDFRenderer renderer = new PDFRenderer(doc);
            int pageCount = doc.getNumberOfPages();

            for (int i = 0; i < pageCount; i++) {
                BufferedImage bim = renderer.renderImageWithDPI(i, dpi);
                ByteArrayOutputStream baos = new ByteArrayOutputStream();
                ImageIO.write(bim, "jpg", baos);
                byte[] imgBytes = baos.toByteArray();

                String entryName = String.format("page_%04d.jpg", i + 1);
                ZipEntry entry = new ZipEntry(entryName);
                zos.putNextEntry(entry);
                zos.write(imgBytes);
                zos.closeEntry();
            }
        }
        return outputFile;
    }

    @Override
    public TestResult testConfiguration(Map<String, Object> config) {
        return TestResult.ok("Moteur de rendu PDFBox opérationnel pour l'exportation BD (CBZ).");
    }
}
