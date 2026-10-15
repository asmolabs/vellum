package com.asmolabs.vellum.plugin.builtin.translator;

import java.io.File;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.security.MessageDigest;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;

public class TranslationSegmentCache {

    private final File cacheDirectory;
    private final Map<String, String> memoryCache = new ConcurrentHashMap<>();

    public TranslationSegmentCache(File cacheDirectory) {
        this.cacheDirectory = cacheDirectory;
        if (cacheDirectory != null) {
            cacheDirectory.mkdirs();
        }
    }

    public String get(String sourceText) {
        if (sourceText == null || sourceText.isBlank()) return null;
        String trimmed = sourceText.trim();

        if (memoryCache.containsKey(trimmed)) {
            return memoryCache.get(trimmed);
        }

        if (cacheDirectory != null) {
            File file = getSegmentFile(trimmed);
            if (file.exists() && file.isFile()) {
                try {
                    String cached = Files.readString(file.toPath(), StandardCharsets.UTF_8);
                    memoryCache.put(trimmed, cached);
                    return cached;
                } catch (Exception ignored) {}
            }
        }

        return null;
    }

    public void put(String sourceText, String translatedText) {
        if (sourceText == null || sourceText.isBlank() || translatedText == null) return;
        String trimmed = sourceText.trim();
        memoryCache.put(trimmed, translatedText);

        if (cacheDirectory != null) {
            File file = getSegmentFile(trimmed);
            try {
                file.getParentFile().mkdirs();
                Files.writeString(file.toPath(), translatedText, StandardCharsets.UTF_8);
            } catch (Exception ignored) {}
        }
    }

    private File getSegmentFile(String text) {
        String hash = md5(text);
        return new File(cacheDirectory, hash + ".txt");
    }

    private static String md5(String input) {
        try {
            MessageDigest md = MessageDigest.getInstance("MD5");
            byte[] bytes = md.digest(input.getBytes(StandardCharsets.UTF_8));
            StringBuilder sb = new StringBuilder();
            for (byte b : bytes) {
                sb.append(String.format("%02x", b));
            }
            return sb.toString();
        } catch (Exception e) {
            return Integer.toHexString(input.hashCode());
        }
    }
}
