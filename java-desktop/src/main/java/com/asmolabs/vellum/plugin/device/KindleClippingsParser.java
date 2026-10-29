package com.asmolabs.vellum.plugin.device;

import java.io.File;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.util.ArrayList;
import java.util.List;

public class KindleClippingsParser {

    /**
     * Parses Amazon Kindle "My Clippings.txt" into a list of DeviceAnnotation items.
     */
    public static List<DeviceAnnotation> parseFile(File file) throws IOException {
        if (!file.exists()) {
            return List.of();
        }
        String content = Files.readString(file.toPath(), StandardCharsets.UTF_8);
        return parse(content);
    }

    public static List<DeviceAnnotation> parse(String content) {
        List<DeviceAnnotation> annotations = new ArrayList<>();
        if (content == null || content.isBlank()) {
            return annotations;
        }

        String[] blocks = content.split("==========");
        for (String block : blocks) {
            String[] lines = block.lines()
                    .map(l -> l.replace("\uFEFF", "").trim())
                    .filter(l -> !l.isEmpty())
                    .toArray(String[]::new);

            if (lines.length < 2) {
                continue;
            }

            // Line 1: Title and Author, e.g. "Dune (Frank Herbert)"
            String titleLine = lines[0];
            String title = titleLine;
            String author = null;
            int openParen = titleLine.lastIndexOf('(');
            int closeParen = titleLine.lastIndexOf(')');
            if (openParen > 0 && closeParen > openParen) {
                title = titleLine.substring(0, openParen).trim();
                author = titleLine.substring(openParen + 1, closeParen).trim();
            }

            // Line 2: Meta (Highlight/Note, page/location, timestamp)
            String metaLine = lines[1];
            String location = extractLocation(metaLine);
            String date = extractDate(metaLine);

            // Lines 3+: Body
            StringBuilder body = new StringBuilder();
            for (int i = 2; i < lines.length; i++) {
                if (i > 2) body.append("\n");
                body.append(lines[i]);
            }
            String bodyText = body.toString().trim();

            if (!bodyText.isEmpty()) {
                boolean isNote = metaLine.toLowerCase().contains("note");
                annotations.add(new DeviceAnnotation(
                        title,
                        author,
                        isNote ? "" : bodyText,
                        isNote ? bodyText : null,
                        location,
                        date
                ));
            }
        }

        return annotations;
    }

    private static String extractLocation(String line) {
        for (String part : line.split("\\|")) {
            String clean = part.trim().toLowerCase();
            if (clean.contains("location") || clean.contains("emplacement") || clean.contains("page")) {
                return part.trim().replaceAll("^-\\s*", "");
            }
        }
        return null;
    }

    private static String extractDate(String line) {
        int pos = line.indexOf("Added on ");
        if (pos < 0) {
            pos = line.indexOf("Ajouté le ");
        }
        if (pos >= 0) {
            int markerLen = line.contains("Added on ") ? "Added on ".length() : "Ajouté le ".length();
            return line.substring(pos + markerLen).trim();
        }
        return null;
    }
}
