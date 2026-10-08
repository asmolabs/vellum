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
public class AudiobookFormatPlugin implements BookFormatPlugin {

    private static final Logger log = LoggerFactory.getLogger(AudiobookFormatPlugin.class);

    private final PluginDescriptor descriptor;

    public AudiobookFormatPlugin() {
        this.descriptor = new PluginDescriptor(
                "format-audiobook",
                "Lecteur de Livres Audio (M4B / M4A / MP3)",
                "Support natif des livres audio aux formats M4B, M4A et MP3 avec extraction des tags ID3v2 / MP4 (titre, narrateur, durée, chapitres et pochette).",
                "1.0.0",
                PluginType.FORMAT,
                false,
                null,
                List.of("m4b", "m4a", "mp3"),
                List.of()
        );
    }

    @Override
    public PluginDescriptor getDescriptor() {
        return descriptor;
    }

    @Override
    public List<String> getSupportedExtensions() {
        return List.of("m4b", "m4a", "mp3");
    }

    @Override
    public ExtractedMetadata extractMetadata(File file) throws Exception {
        String nameLower = file.getName().toLowerCase();
        if (nameLower.endsWith(".mp3")) {
            return parseMp3(file);
        } else {
            return parseM4b(file);
        }
    }

    private ExtractedMetadata parseMp3(File file) throws Exception {
        String title = null;
        String author = null;
        String publisher = null;
        String publishDate = null;
        String description = null;
        List<String> tags = new ArrayList<>();
        tags.add("Livre Audio");

        byte[] coverData = null;
        String coverExt = ".jpg";

        try (RandomAccessFile raf = new RandomAccessFile(file, "r")) {
            if (raf.length() < 10) {
                return new ExtractedMetadata(title, author, publisher, publishDate, description, tags, coverData, coverExt);
            }

            byte[] header = new byte[10];
            raf.readFully(header);

            if (header[0] == 'I' && header[1] == 'D' && header[2] == '3') {
                int majorVersion = header[3];
                // Syncsafe integer for size
                int tagSize = ((header[6] & 0x7F) << 21) | ((header[7] & 0x7F) << 14) | ((header[8] & 0x7F) << 7) | (header[9] & 0x7F);

                long endPos = Math.min(raf.length(), 10L + tagSize);

                while (raf.getFilePointer() + 10 <= endPos) {
                    byte[] frameIdBytes = new byte[4];
                    raf.readFully(frameIdBytes);
                    String frameId = new String(frameIdBytes, StandardCharsets.US_ASCII);

                    if (frameIdBytes[0] == 0) break; // Padding reached

                    int frameSize;
                    if (majorVersion == 4) {
                        // Syncsafe in ID3v2.4
                        byte[] sz = new byte[4];
                        raf.readFully(sz);
                        frameSize = ((sz[0] & 0x7F) << 21) | ((sz[1] & 0x7F) << 14) | ((sz[2] & 0x7F) << 7) | (sz[3] & 0x7F);
                    } else {
                        frameSize = raf.readInt();
                    }

                    raf.skipBytes(2); // flags

                    if (frameSize <= 0 || raf.getFilePointer() + frameSize > endPos) {
                        break;
                    }

                    byte[] data = new byte[frameSize];
                    raf.readFully(data);

                    switch (frameId) {
                        case "TIT2" -> title = decodeTextFrame(data);
                        case "TPE1", "TPE2" -> {
                            String a = decodeTextFrame(data);
                            if (author == null || author.isBlank()) author = a;
                        }
                        case "TALB" -> publisher = decodeTextFrame(data);
                        case "TYER", "TDRC" -> publishDate = decodeTextFrame(data);
                        case "TCON" -> {
                            String genre = decodeTextFrame(data);
                            if (genre != null && !genre.isBlank() && !tags.contains(genre)) tags.add(genre);
                        }
                        case "COMM" -> description = decodeCommentFrame(data);
                        case "APIC" -> {
                            CoverResult cov = decodeApicFrame(data);
                            if (cov != null) {
                                coverData = cov.data;
                                coverExt = cov.ext;
                            }
                        }
                    }
                }
            }
        }

        return new ExtractedMetadata(title, author, publisher, publishDate, description, tags, coverData, coverExt);
    }

    private ExtractedMetadata parseM4b(File file) throws Exception {
        String title = null;
        String author = null;
        String publisher = null;
        String publishDate = null;
        String description = null;
        List<String> tags = new ArrayList<>();
        tags.add("Livre Audio");

        byte[] coverData = null;
        String coverExt = ".jpg";

        try (RandomAccessFile raf = new RandomAccessFile(file, "r")) {
            long ilstOffset = findIlstAtom(raf, 0, raf.length());
            if (ilstOffset > 0) {
                raf.seek(ilstOffset);
                int ilstSize = raf.readInt();
                raf.skipBytes(4); // "ilst"

                long endIlst = ilstOffset + ilstSize;
                while (raf.getFilePointer() + 8 < endIlst) {
                    long itemStart = raf.getFilePointer();
                    int itemSize = raf.readInt();
                    if (itemSize <= 8 || itemStart + itemSize > endIlst) break;

                    byte[] nameBytes = new byte[4];
                    raf.readFully(nameBytes);
                    String atomName = new String(nameBytes, StandardCharsets.ISO_8859_1);

                    // Find "data" sub-atom inside this item
                    long dataAtomPos = findDataSubAtom(raf, itemStart + 8, itemStart + itemSize);
                    if (dataAtomPos > 0) {
                        raf.seek(dataAtomPos);
                        int dataSize = raf.readInt();
                        raf.skipBytes(4); // "data"
                        int typeFlags = raf.readInt();
                        raf.skipBytes(4); // locale

                        int payloadLen = dataSize - 16;
                        if (payloadLen > 0) {
                            byte[] payload = new byte[payloadLen];
                            raf.readFully(payload);

                            if ("covr".equals(atomName)) {
                                coverData = payload;
                                if (payload.length > 3 && (payload[0] & 0xFF) == 0x89 && (payload[1] & 0xFF) == 0x50) {
                                    coverExt = ".png";
                                } else {
                                    coverExt = ".jpg";
                                }
                            } else {
                                String text = new String(payload, StandardCharsets.UTF_8).trim();
                                if (atomName.endsWith("nam")) title = text;
                                else if (atomName.endsWith("ART") || atomName.endsWith("aut")) author = text;
                                else if (atomName.endsWith("alb")) publisher = text;
                                else if (atomName.endsWith("day")) publishDate = text;
                                else if (atomName.endsWith("cmt") || "desc".equals(atomName)) description = text;
                                else if (atomName.endsWith("gen") && !text.isBlank() && !tags.contains(text)) tags.add(text);
                            }
                        }
                    }

                    raf.seek(itemStart + itemSize);
                }
            }
        }

        return new ExtractedMetadata(title, author, publisher, publishDate, description, tags, coverData, coverExt);
    }

    private long findIlstAtom(RandomAccessFile raf, long start, long length) throws Exception {
        raf.seek(start);
        long end = start + length;

        while (raf.getFilePointer() + 8 <= end) {
            long atomPos = raf.getFilePointer();
            long atomSize = raf.readInt() & 0xFFFFFFFFL;
            byte[] typeBytes = new byte[4];
            raf.readFully(typeBytes);
            String type = new String(typeBytes, StandardCharsets.ISO_8859_1);

            if (atomSize == 1) { // 64-bit size
                atomSize = raf.readLong();
            } else if (atomSize == 0) {
                atomSize = end - atomPos;
            }

            if ("ilst".equals(type)) {
                return atomPos;
            }

            if ("moov".equals(type) || "udta".equals(type) || "meta".equals(type)) {
                // If "meta", skip 4 bytes version/flags
                long innerStart = atomPos + 8;
                if ("meta".equals(type)) innerStart += 4;
                long res = findIlstAtom(raf, innerStart, atomSize - (innerStart - atomPos));
                if (res > 0) return res;
            }

            if (atomSize <= 0 || atomPos + atomSize > end) break;
            raf.seek(atomPos + atomSize);
        }
        return -1;
    }

    private long findDataSubAtom(RandomAccessFile raf, long start, long end) throws Exception {
        raf.seek(start);
        while (raf.getFilePointer() + 8 <= end) {
            long pos = raf.getFilePointer();
            int sz = raf.readInt();
            byte[] typeBytes = new byte[4];
            raf.readFully(typeBytes);
            String type = new String(typeBytes, StandardCharsets.ISO_8859_1);
            if ("data".equals(type)) {
                return pos;
            }
            if (sz <= 0 || pos + sz > end) break;
            raf.seek(pos + sz);
        }
        return -1;
    }

    private String decodeTextFrame(byte[] data) {
        if (data.length <= 1) return "";
        int encoding = data[0];
        Charset cs = switch (encoding) {
            case 1 -> StandardCharsets.UTF_16;
            case 2 -> StandardCharsets.UTF_16BE;
            case 3 -> StandardCharsets.UTF_8;
            default -> StandardCharsets.ISO_8859_1;
        };
        return new String(data, 1, data.length - 1, cs).trim().replace("\0", "");
    }

    private String decodeCommentFrame(byte[] data) {
        if (data.length <= 4) return "";
        // 1 byte encoding, 3 bytes language, description\0, text
        int encoding = data[0];
        Charset cs = (encoding == 1 || encoding == 2) ? StandardCharsets.UTF_16 : StandardCharsets.UTF_8;
        return new String(data, 4, data.length - 4, cs).trim().replace("\0", "");
    }

    private record CoverResult(byte[] data, String ext) {}

    private CoverResult decodeApicFrame(byte[] data) {
        if (data.length < 10) return null;
        int offset = 1;
        // Read MIME type
        StringBuilder mime = new StringBuilder();
        while (offset < data.length && data[offset] != 0) {
            mime.append((char) data[offset]);
            offset++;
        }
        offset++; // skip null
        if (offset >= data.length) return null;
        offset++; // picture type byte

        // Skip description (null terminated)
        while (offset < data.length && data[offset] != 0) {
            offset++;
        }
        offset++; // skip null
        if (offset < data.length && data[offset] == 0) offset++; // if double null

        int imgLen = data.length - offset;
        if (imgLen <= 4) return null;

        byte[] img = new byte[imgLen];
        System.arraycopy(data, offset, img, 0, imgLen);
        String ext = mime.toString().toLowerCase().contains("png") ? ".png" : ".jpg";
        return new CoverResult(img, ext);
    }
}
