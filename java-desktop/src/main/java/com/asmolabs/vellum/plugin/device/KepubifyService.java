package com.asmolabs.vellum.plugin.device;

import java.io.*;
import java.nio.charset.StandardCharsets;
import java.util.Enumeration;
import java.util.regex.Matcher;
import java.util.regex.Pattern;
import java.util.zip.CRC32;
import java.util.zip.ZipEntry;
import java.util.zip.ZipFile;
import java.util.zip.ZipOutputStream;

public class KepubifyService {

    private static final Pattern P_PATTERN = Pattern.compile("(?is)(<p\\b[^>]*>)(.*?)(</p>)");

    /**
     * Converts a standard EPUB to a KEPUB (Kobo EPUB) by injecting koboSpan tags
     * inside paragraph elements (<p>), activating Kobo ACCESS engine with per-chapter page numbering.
     */
    public static void convertEpubToKepub(File inputEpub, File outputKepub) throws IOException {
        File parent = outputKepub.getParentFile();
        if (parent != null) {
            parent.mkdirs();
        }

        File tempFile = File.createTempFile("vellum_kepub_", ".zip");
        try {
            try (ZipFile zipIn = new ZipFile(inputEpub);
                 ZipOutputStream zipOut = new ZipOutputStream(new BufferedOutputStream(new FileOutputStream(tempFile)))) {

                Enumeration<? extends ZipEntry> entries = zipIn.entries();
                int chapterIdx = 0;

                while (entries.hasMoreElements()) {
                    ZipEntry entry = entries.nextElement();
                    String name = entry.getName();

                    if (entry.isDirectory()) {
                        ZipEntry dirEntry = new ZipEntry(name.endsWith("/") ? name : name + "/");
                        zipOut.putNextEntry(dirEntry);
                        zipOut.closeEntry();
                        continue;
                    }

                    boolean isHtml = name.endsWith(".xhtml") || name.endsWith(".html") || name.endsWith(".htm");

                    if ("mimetype".equals(name)) {
                        byte[] bytes;
                        try (InputStream is = zipIn.getInputStream(entry)) {
                            bytes = is.readAllBytes();
                        }
                        ZipEntry mimeEntry = new ZipEntry("mimetype");
                        mimeEntry.setMethod(ZipEntry.STORED);
                        mimeEntry.setSize(bytes.length);
                        mimeEntry.setCompressedSize(bytes.length);
                        CRC32 crc = new CRC32();
                        crc.update(bytes);
                        mimeEntry.setCrc(crc.getValue());

                        zipOut.putNextEntry(mimeEntry);
                        zipOut.write(bytes);
                        zipOut.closeEntry();
                    } else if (isHtml) {
                        chapterIdx++;
                        String content;
                        try (InputStream is = zipIn.getInputStream(entry)) {
                            content = new String(is.readAllBytes(), StandardCharsets.UTF_8);
                        }

                        String modified = injectKoboSpans(content, chapterIdx);
                        byte[] bytes = modified.getBytes(StandardCharsets.UTF_8);

                        ZipEntry htmlEntry = new ZipEntry(name);
                        htmlEntry.setMethod(ZipEntry.DEFLATED);
                        zipOut.putNextEntry(htmlEntry);
                        zipOut.write(bytes);
                        zipOut.closeEntry();
                    } else {
                        ZipEntry otherEntry = new ZipEntry(name);
                        otherEntry.setMethod(ZipEntry.DEFLATED);
                        zipOut.putNextEntry(otherEntry);
                        try (InputStream is = zipIn.getInputStream(entry)) {
                            is.transferTo(zipOut);
                        }
                        zipOut.closeEntry();
                    }
                }
            }

            java.nio.file.Files.copy(tempFile.toPath(), outputKepub.toPath(), java.nio.file.StandardCopyOption.REPLACE_EXISTING);
        } finally {
            tempFile.delete();
        }
    }

    public static String injectKoboSpans(String html, int chapterIdx) {
        StringBuilder result = new StringBuilder(html.length() + 1024);
        Matcher matcher = P_PATTERN.matcher(html);
        int cursor = 0;
        int spanIdx = 0;

        while (matcher.find()) {
            result.append(html, cursor, matcher.start());

            String pStart = matcher.group(1);
            String pBody = matcher.group(2);
            String pEnd = matcher.group(3);

            spanIdx++;
            result.append(pStart);
            if (!pBody.isBlank() && !pBody.contains("koboSpan")) {
                result.append("<span class=\"koboSpan\" id=\"kobo.")
                        .append(chapterIdx).append(".").append(spanIdx).append("\">")
                        .append(pBody)
                        .append("</span>");
            } else {
                result.append(pBody);
            }
            result.append(pEnd);

            cursor = matcher.end();
        }

        result.append(html.substring(cursor));
        return result.toString();
    }
}
