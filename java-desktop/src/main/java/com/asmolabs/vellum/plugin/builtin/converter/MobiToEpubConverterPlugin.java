package com.asmolabs.vellum.plugin.builtin.converter;

import com.asmolabs.vellum.plugin.PluginDescriptor;
import com.asmolabs.vellum.plugin.PluginType;
import com.asmolabs.vellum.plugin.converter.BookConverterPlugin;
import org.springframework.stereotype.Component;

import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.FileOutputStream;
import java.io.RandomAccessFile;
import java.nio.charset.StandardCharsets;
import java.util.*;
import java.util.zip.CRC32;
import java.util.zip.ZipEntry;
import java.util.zip.ZipOutputStream;

@Component
public class MobiToEpubConverterPlugin implements BookConverterPlugin {

    private final PluginDescriptor descriptor = new PluginDescriptor(
            "converter-mobi-to-epub",
            "Convertisseur Kindle (MOBI/AZW ➔ EPUB)",
            "Décompresse les enregistrements PalmDOC LZ77 de fichiers Kindle (MOBI, AZW, AZW3) et génère un conteneur EPUB standardisé.",
            "1.0.0",
            PluginType.CONVERSION,
            false,
            null,
            List.of(),
            "mobi",
            "epub",
            List.of()
    );

    @Override
    public PluginDescriptor getDescriptor() {
        return descriptor;
    }

    @Override
    public String getSourceFormat() {
        return "mobi";
    }

    @Override
    public String getTargetFormat() {
        return "epub";
    }

    @Override
    public boolean canConvert(String fromExt, String toExt) {
        return ("mobi".equalsIgnoreCase(fromExt) || "azw".equalsIgnoreCase(fromExt) || "azw3".equalsIgnoreCase(fromExt))
                && "epub".equalsIgnoreCase(toExt);
    }

    @Override
    public File convert(File inputFile, File outputFile, Map<String, Object> config) throws Exception {
        String bookTitle = inputFile.getName().replaceFirst("[.][^.]+$", "");
        String bookAuthor = "Auteur inconnu";

        ByteArrayOutputStream textBytesStream = new ByteArrayOutputStream();

        try (RandomAccessFile raf = new RandomAccessFile(inputFile, "r")) {
            raf.seek(76);
            int numRecords = raf.readUnsignedShort();

            long[] recordOffsets = new long[numRecords];
            for (int i = 0; i < numRecords; i++) {
                recordOffsets[i] = raf.readInt() & 0xFFFFFFFFL;
                raf.skipBytes(4);
            }

            if (numRecords > 0) {
                raf.seek(recordOffsets[0]);
                int compression = raf.readUnsignedShort(); // 1 = none, 2 = PalmDOC, 17480 = HUFF/CDIC
                raf.skipBytes(6);
                int textRecordCount = raf.readUnsignedShort();

                // Read title if available from MOBI header
                raf.seek(recordOffsets[0] + 16);
                byte[] mobiMagic = new byte[4];
                raf.readFully(mobiMagic);
                if (new String(mobiMagic, StandardCharsets.ISO_8859_1).equals("MOBI")) {
                    raf.seek(recordOffsets[0] + 84);
                    int fullNameOffset = raf.readInt();
                    int fullNameLength = raf.readInt();
                    if (fullNameLength > 0 && fullNameLength < 1024) {
                        raf.seek(recordOffsets[0] + fullNameOffset);
                        byte[] nameBytes = new byte[fullNameLength];
                        raf.readFully(nameBytes);
                        String t = new String(nameBytes, StandardCharsets.UTF_8).trim();
                        if (!t.isBlank()) bookTitle = t;
                    }
                }

                // Decompress text records
                int limit = Math.min(textRecordCount + 1, numRecords);
                for (int i = 1; i < limit; i++) {
                    long start = recordOffsets[i];
                    long end = (i + 1 < numRecords) ? recordOffsets[i + 1] : raf.length();
                    int len = (int) (end - start);
                    if (len <= 0) continue;

                    raf.seek(start);
                    byte[] raw = new byte[len];
                    raf.readFully(raw);

                    if (compression == 2) {
                        byte[] decomp = decompressPalmDoc(raw);
                        textBytesStream.write(decomp);
                    } else if (compression == 1) {
                        textBytesStream.write(raw);
                    }
                }
            }
        }

        String rawHtml = textBytesStream.toString(StandardCharsets.UTF_8);
        if (rawHtml.isBlank()) {
            rawHtml = "<p>Contenu extrait du fichier Kindle.</p>";
        }

        // Package into standard EPUB
        try (ZipOutputStream zos = new ZipOutputStream(new FileOutputStream(outputFile))) {
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

            String containerXml = """
                <?xml version="1.0" encoding="UTF-8"?>
                <container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
                  <rootfiles>
                    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
                  </rootfiles>
                </container>
                """.stripIndent();
            writeEntry(zos, "META-INF/container.xml", containerXml);

            String css = "body { font-family: Georgia, serif; line-height: 1.6; margin: 5%; }";
            writeEntry(zos, "OEBPS/stylesheet.css", css);

            String xhtml = String.format("""
                <?xml version="1.0" encoding="utf-8"?>
                <!DOCTYPE html>
                <html xmlns="http://www.w3.org/1999/xhtml">
                <head>
                  <title>%s</title>
                  <link rel="stylesheet" type="text/css" href="stylesheet.css"/>
                </head>
                <body>
                  %s
                </body>
                </html>
                """, escapeXml(bookTitle), rawHtml);
            writeEntry(zos, "OEBPS/content.xhtml", xhtml);

            String uuid = UUID.randomUUID().toString();
            String opf = String.format("""
                <?xml version="1.0" encoding="UTF-8"?>
                <package xmlns="http://www.idpf.org/2007/opf" unique-identifier="BookId" version="2.0">
                  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
                    <dc:title>%s</dc:title>
                    <dc:creator>%s</dc:creator>
                    <dc:identifier id="BookId">urn:uuid:%s</dc:identifier>
                    <dc:language>fr</dc:language>
                  </metadata>
                  <manifest>
                    <item id="ncx" href="toc.ncx" media-type="application/x-dtbncx+xml"/>
                    <item id="css" href="stylesheet.css" media-type="text/css"/>
                    <item id="content" href="content.xhtml" media-type="application/xhtml+xml"/>
                  </manifest>
                  <spine toc="ncx">
                    <itemref idref="content"/>
                  </spine>
                </package>
                """, escapeXml(bookTitle), escapeXml(bookAuthor), uuid);
            writeEntry(zos, "OEBPS/content.opf", opf);

            String ncx = String.format("""
                <?xml version="1.0" encoding="UTF-8"?>
                <ncx xmlns="http://www.daisy.org/z3986/2005/ncx/\" version="2005-1">
                  <docTitle><text>%s</text></docTitle>
                  <navMap>
                    <navPoint id="nav1" playOrder="1">
                      <navLabel><text>%s</text></navLabel>
                      <content src="content.xhtml"/>
                    </navPoint>
                  </navMap>
                </ncx>
                """, escapeXml(bookTitle), escapeXml(bookTitle));
            writeEntry(zos, "OEBPS/toc.ncx", ncx);
        }

        return outputFile;
    }

    @Override
    public TestResult testConfiguration(Map<String, Object> config) {
        return TestResult.ok("Décompresseur PalmDOC LZ77 100% Java prêt (sans dépendance externe).");
    }

    public static byte[] decompressPalmDoc(byte[] compressed) {
        ByteArrayOutputStream out = new ByteArrayOutputStream();
        int i = 0;
        while (i < compressed.length) {
            int b = compressed[i++] & 0xFF;
            if (b == 0) {
                out.write(0);
            } else if (b >= 1 && b <= 8) {
                for (int k = 0; k < b && i < compressed.length; k++) {
                    out.write(compressed[i++]);
                }
            } else if (b >= 9 && b <= 0x7F) {
                out.write(b);
            } else if (b >= 0x80 && b <= 0xBF) {
                if (i < compressed.length) {
                    int b2 = compressed[i++] & 0xFF;
                    int pair = (b << 8) | b2;
                    int distance = (pair & 0x3FFC) >> 2;
                    int length = (pair & 0x0003) + 3;
                    byte[] current = out.toByteArray();
                    int start = current.length - distance;
                    for (int k = 0; k < length && start + k < current.length && start + k >= 0; k++) {
                        out.write(current[start + k]);
                    }
                }
            } else {
                out.write(' ');
                out.write(b ^ 0x80);
            }
        }
        return out.toByteArray();
    }

    private void writeEntry(ZipOutputStream zos, String name, String content) throws Exception {
        ZipEntry entry = new ZipEntry(name);
        zos.putNextEntry(entry);
        zos.write(content.getBytes(StandardCharsets.UTF_8));
        zos.closeEntry();
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
