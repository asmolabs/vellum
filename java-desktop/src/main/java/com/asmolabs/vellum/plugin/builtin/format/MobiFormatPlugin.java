package com.asmolabs.vellum.plugin.builtin.format;

import com.asmolabs.vellum.plugin.PluginDescriptor;
import com.asmolabs.vellum.plugin.PluginType;
import com.asmolabs.vellum.plugin.format.BookFormatPlugin;
import com.asmolabs.vellum.plugin.format.ExtractedMetadata;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.stereotype.Component;

import java.io.File;
import java.io.RandomAccessFile;
import java.nio.charset.Charset;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;

@Component
public class MobiFormatPlugin implements BookFormatPlugin {

    private static final Logger log = LoggerFactory.getLogger(MobiFormatPlugin.class);

    private final PluginDescriptor descriptor;

    public MobiFormatPlugin() {
        this.descriptor = new PluginDescriptor(
                "format-mobi",
                "Lecteur de format Kindle (MOBI / AZW3)",
                "Support natif des livres numériques Amazon Kindle (MOBI, AZW, AZW3/KF8) avec extraction des métadonnées EXTH et de la couverture.",
                "1.0.0",
                PluginType.FORMAT,
                false,
                null,
                List.of("mobi", "azw3", "azw"),
                List.of()
        );
    }

    @Override
    public PluginDescriptor getDescriptor() {
        return descriptor;
    }

    @Override
    public List<String> getSupportedExtensions() {
        return List.of("mobi", "azw3", "azw");
    }

    @Override
    public ExtractedMetadata extractMetadata(File file) throws Exception {
        String title = null;
        String author = null;
        String publisher = null;
        String publishDate = null;
        String description = null;
        List<String> tags = new ArrayList<>();
        tags.add("Kindle");

        byte[] coverData = null;
        String coverExt = ".jpg";

        try (RandomAccessFile raf = new RandomAccessFile(file, "r")) {
            long fileLength = raf.length();
            if (fileLength < 78) {
                throw new IllegalArgumentException("Fichier trop court pour un format MOBI/PDB valide");
            }

            // 1. Read Palm Database Header (PDB)
            raf.seek(60);
            byte[] typeBytes = new byte[4];
            byte[] creatorBytes = new byte[4];
            raf.readFully(typeBytes);
            raf.readFully(creatorBytes);
            String dbType = new String(typeBytes, StandardCharsets.ISO_8859_1);
            String dbCreator = new String(creatorBytes, StandardCharsets.ISO_8859_1);

            raf.seek(76);
            int numRecords = raf.readUnsignedShort();
            if (numRecords <= 0) {
                throw new IllegalArgumentException("Aucun enregistrement PDB trouvé dans le fichier MOBI");
            }

            // Read record offset table
            long[] recordOffsets = new long[numRecords];
            for (int i = 0; i < numRecords; i++) {
                recordOffsets[i] = raf.readInt() & 0xFFFFFFFFL;
                raf.skipBytes(4); // attributes & unique ID
            }

            // 2. Read Record 0 (PalmDOC + MOBI Header)
            long rec0Offset = recordOffsets[0];
            if (rec0Offset + 16 > fileLength) {
                throw new IllegalArgumentException("En-tête Record 0 invalide");
            }

            // MOBI Header starts at rec0Offset + 16
            raf.seek(rec0Offset + 16);
            byte[] mobiMagic = new byte[4];
            raf.readFully(mobiMagic);
            String magic = new String(mobiMagic, StandardCharsets.US_ASCII);
            if (!"MOBI".equals(magic)) {
                log.warn("Magic MOBI manquant dans Record 0 (valeur: {}), tentative de lecture générique", magic);
            }

            int mobiHeaderLength = raf.readInt();
            int mobiType = raf.readInt();
            int textEncodingCode = raf.readInt();
            Charset textCharset = (textEncodingCode == 65001) ? StandardCharsets.UTF_8 : StandardCharsets.ISO_8859_1;

            // Title offset and length
            // Offset 84 in Record 0 is titleOffset (from rec0Offset)
            long titleOffsetPos = rec0Offset + 16 + 68; // 84
            long titleLenPos = rec0Offset + 16 + 72; // 88
            int titleOffset = 0;
            int titleLen = 0;

            if (titleOffsetPos + 8 <= fileLength) {
                raf.seek(titleOffsetPos);
                titleOffset = raf.readInt();
                titleLen = raf.readInt();
            }

            if (titleOffset > 0 && titleLen > 0 && (rec0Offset + titleOffset + titleLen <= fileLength)) {
                raf.seek(rec0Offset + titleOffset);
                byte[] titleBytes = new byte[titleLen];
                raf.readFully(titleBytes);
                title = new String(titleBytes, textCharset).trim();
            }

            // First image record index
            int firstImageIndex = -1;
            long firstImagePos = rec0Offset + 16 + 92; // 108
            if (firstImagePos + 4 <= fileLength) {
                raf.seek(firstImagePos);
                firstImageIndex = raf.readInt();
            }

            // EXTH flags
            long exthFlagPos = rec0Offset + 16 + 112; // 128
            int exthFlags = 0;
            if (exthFlagPos + 4 <= fileLength) {
                raf.seek(exthFlagPos);
                exthFlags = raf.readInt();
            }

            int coverOffset = -1;

            // 3. Read EXTH Header if present
            boolean hasExth = (exthFlags & 0x40) != 0;
            long exthOffset = rec0Offset + 16 + mobiHeaderLength;

            if (hasExth && exthOffset + 12 <= fileLength) {
                raf.seek(exthOffset);
                byte[] exthMagic = new byte[4];
                raf.readFully(exthMagic);
                if ("EXTH".equals(new String(exthMagic, StandardCharsets.US_ASCII))) {
                    int exthLength = raf.readInt();
                    int exthCount = raf.readInt();

                    long currentPos = raf.getFilePointer();
                    for (int i = 0; i < exthCount && currentPos < exthOffset + exthLength; i++) {
                        raf.seek(currentPos);
                        int tag = raf.readInt();
                        int len = raf.readInt();
                        int dataLen = len - 8;

                        if (dataLen > 0 && currentPos + len <= fileLength) {
                            byte[] valBytes = new byte[dataLen];
                            raf.readFully(valBytes);

                            switch (tag) {
                                case 100 -> { // Author
                                    String a = new String(valBytes, textCharset).trim();
                                    if (author == null) author = a;
                                }
                                case 101 -> publisher = new String(valBytes, textCharset).trim();
                                case 103 -> description = new String(valBytes, textCharset).trim();
                                case 105 -> { // Subject / Tag
                                    String sub = new String(valBytes, textCharset).trim();
                                    if (!sub.isBlank() && !tags.contains(sub)) tags.add(sub);
                                }
                                case 106 -> publishDate = new String(valBytes, textCharset).trim();
                                case 201 -> { // Cover offset
                                    if (dataLen == 4) {
                                        coverOffset = ((valBytes[0] & 0xFF) << 24)
                                                | ((valBytes[1] & 0xFF) << 16)
                                                | ((valBytes[2] & 0xFF) << 8)
                                                | (valBytes[3] & 0xFF);
                                    }
                                }
                                case 503 -> { // Updated Title if available
                                    String t = new String(valBytes, textCharset).trim();
                                    if (!t.isBlank()) title = t;
                                }
                            }
                        }
                        currentPos += len;
                    }
                }
            }

            // 4. Extract Cover image if index is valid
            if (coverOffset >= 0 && firstImageIndex >= 0) {
                int coverRecordIdx = firstImageIndex + coverOffset;
                if (coverRecordIdx < numRecords) {
                    long imgStart = recordOffsets[coverRecordIdx];
                    long imgEnd = (coverRecordIdx + 1 < numRecords) ? recordOffsets[coverRecordIdx + 1] : fileLength;
                    long imgSize = imgEnd - imgStart;

                    if (imgSize > 4 && imgSize < 20_000_000) {
                        raf.seek(imgStart);
                        coverData = new byte[(int) imgSize];
                        raf.readFully(coverData);

                        // Detect PNG or JPEG
                        if (coverData.length > 3 && (coverData[0] & 0xFF) == 0x89 && (coverData[1] & 0xFF) == 0x50) {
                            coverExt = ".png";
                        } else {
                            coverExt = ".jpg";
                        }
                    }
                }
            }
        }

        return new ExtractedMetadata(title, author, publisher, publishDate, description, tags, coverData, coverExt);
    }
}
