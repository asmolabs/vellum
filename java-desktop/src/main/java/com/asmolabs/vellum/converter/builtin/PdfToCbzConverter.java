package com.asmolabs.vellum.converter.builtin;

import com.asmolabs.vellum.converter.FormatConverter;
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
import java.util.Map;
import java.util.zip.ZipEntry;
import java.util.zip.ZipOutputStream;

@Component
public class PdfToCbzConverter implements FormatConverter {

    private static final Logger log = LoggerFactory.getLogger(PdfToCbzConverter.class);

    @Override
    public String getSourceFormat() {
        return "pdf";
    }

    @Override
    public String getTargetFormat() {
        return "cbz";
    }

    @Override
    public File convert(File inputFile, File outputFile, Map<String, Object> options) throws Exception {
        int dpi = 150;
        if (options != null && options.containsKey("dpi")) {
            try {
                dpi = Integer.parseInt(String.valueOf(options.get("dpi")));
            } catch (Exception ignored) {}
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
}
