package com.asmolabs.vellum.plugin.format;

import com.asmolabs.vellum.plugin.VellumPlugin;

import java.io.File;
import java.util.List;

public interface BookFormatPlugin extends VellumPlugin {

    /**
     * List of lowercase file extensions supported by this plugin, without leading dot (e.g. "epub", "pdf", "cbz").
     */
    List<String> getSupportedExtensions();

    /**
     * Extracts book metadata and cover image data from the book file.
     *
     * @param file The book file
     * @return Extracted metadata and cover
     * @throws Exception If parsing fails
     */
    ExtractedMetadata extractMetadata(File file) throws Exception;

    /**
     * Returns true if this format plugin can process the given file.
     */
    default boolean canHandle(File file) {
        if (file == null || !file.exists()) return false;
        String name = file.getName().toLowerCase();
        int dot = name.lastIndexOf('.');
        if (dot < 0) return false;
        String ext = name.substring(dot + 1);
        return getSupportedExtensions().stream().anyMatch(e -> e.equalsIgnoreCase(ext));
    }
}
