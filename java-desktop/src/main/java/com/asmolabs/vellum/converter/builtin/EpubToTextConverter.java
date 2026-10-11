package com.asmolabs.vellum.converter.builtin;

import com.asmolabs.vellum.converter.FormatConverter;
import org.springframework.stereotype.Component;

import java.io.File;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.util.*;
import java.util.regex.Matcher;
import java.util.regex.Pattern;
import java.util.zip.ZipEntry;
import java.util.zip.ZipFile;

@Component
public class EpubToTextConverter implements FormatConverter {

    @Override
    public String getSourceFormat() {
        return "epub";
    }

    @Override
    public String getTargetFormat() {
        return "txt";
    }

    @Override
    public boolean canConvert(String fromExt, String toExt) {
        return "epub".equalsIgnoreCase(fromExt) && ("txt".equalsIgnoreCase(toExt) || "md".equalsIgnoreCase(toExt));
    }

    @Override
    public File convert(File inputFile, File outputFile, Map<String, Object> options) throws Exception {
        StringBuilder sb = new StringBuilder();

        try (ZipFile zip = new ZipFile(inputFile)) {
            String opfPath = findOpfPath(zip);
            List<String> chapterPaths = opfPath != null ? parseSpine(zip, opfPath) : List.of();

            if (chapterPaths.isEmpty()) {
                chapterPaths = zip.stream()
                        .map(ZipEntry::getName)
                        .filter(n -> n.endsWith(".html") || n.endsWith(".xhtml"))
                        .sorted()
                        .toList();
            }

            for (String chapPath : chapterPaths) {
                ZipEntry entry = zip.getEntry(chapPath);
                if (entry != null) {
                    try (InputStream is = zip.getInputStream(entry)) {
                        String html = new String(is.readAllBytes(), StandardCharsets.UTF_8);
                        String text = htmlToPlainText(html);
                        if (!text.isBlank()) {
                            sb.append(text).append("\n\n---\n\n");
                        }
                    }
                }
            }
        }

        Files.writeString(outputFile.toPath(), sb.toString(), StandardCharsets.UTF_8);
        return outputFile;
    }

    private String findOpfPath(ZipFile zip) {
        ZipEntry container = zip.getEntry("META-INF/container.xml");
        if (container == null) return null;
        try (InputStream is = zip.getInputStream(container)) {
            String xml = new String(is.readAllBytes(), StandardCharsets.UTF_8);
            Matcher m = Pattern.compile("full-path=[\"']([^\"']+)[\"']").matcher(xml);
            if (m.find()) {
                return m.group(1);
            }
        } catch (Exception ignored) {}
        return null;
    }

    private List<String> parseSpine(ZipFile zip, String opfPath) {
        List<String> chapters = new ArrayList<>();
        ZipEntry opfEntry = zip.getEntry(opfPath);
        if (opfEntry == null) return chapters;

        String baseDir = opfPath.contains("/") ? opfPath.substring(0, opfPath.lastIndexOf('/') + 1) : "";

        try (InputStream is = zip.getInputStream(opfEntry)) {
            String xml = new String(is.readAllBytes(), StandardCharsets.UTF_8);

            Map<String, String> manifest = new HashMap<>();
            Matcher itemMatcher = Pattern.compile("<item[^>]+id=[\"']([^\"']+)[\"'][^>]+href=[\"']([^\"']+)[\"'][^>]*>", Pattern.CASE_INSENSITIVE).matcher(xml);
            while (itemMatcher.find()) {
                manifest.put(itemMatcher.group(1), itemMatcher.group(2));
            }
            if (manifest.isEmpty()) {
                Matcher altItem = Pattern.compile("<item[^>]+href=[\"']([^\"']+)[\"'][^>]+id=[\"']([^\"']+)[\"'][^>]*>", Pattern.CASE_INSENSITIVE).matcher(xml);
                while (altItem.find()) {
                    manifest.put(altItem.group(2), altItem.group(1));
                }
            }

            Matcher spineMatcher = Pattern.compile("<itemref[^>]+idref=[\"']([^\"']+)[\"'][^>]*>", Pattern.CASE_INSENSITIVE).matcher(xml);
            while (spineMatcher.find()) {
                String idref = spineMatcher.group(1);
                String href = manifest.get(idref);
                if (href != null) {
                    chapters.add(baseDir + href);
                }
            }
        } catch (Exception ignored) {}

        return chapters;
    }

    private String htmlToPlainText(String html) {
        // Strip scripts and styles
        String clean = html.replaceAll("(?is)<style[^>]*>.*?</style>", "")
                .replaceAll("(?is)<script[^>]*>.*?</script>", "");

        // Convert common structural tags
        clean = clean.replaceAll("(?i)<h1[^>]*>(.*?)</h1>", "\n\n# $1\n\n");
        clean = clean.replaceAll("(?i)<h2[^>]*>(.*?)</h2>", "\n\n## $1\n\n");
        clean = clean.replaceAll("(?i)<h3[^>]*>(.*?)</h3>", "\n\n### $1\n\n");
        clean = clean.replaceAll("(?i)<p[^>]*>", "\n");
        clean = clean.replaceAll("(?i)</p>", "\n");
        clean = clean.replaceAll("(?i)<br\\s*/?>", "\n");
        clean = clean.replaceAll("(?i)<li[^>]*>", "\n* ");

        // Strip remaining HTML tags
        clean = clean.replaceAll("<[^>]+>", "");

        // Decode basic HTML entities
        clean = clean.replace("&nbsp;", " ")
                .replace("&amp;", "&")
                .replace("&lt;", "<")
                .replace("&gt;", ">")
                .replace("&quot;", "\"")
                .replace("&#39;", "'")
                .replace("&rsquo;", "’")
                .replace("&lsquo;", "‘")
                .replace("&ldquo;", "“")
                .replace("&rdquo;", "”");

        // Clean up excessive blank lines
        clean = clean.replaceAll("\n{3,}", "\n\n").trim();
        return clean;
    }
}
