package com.asmolabs.vellum.plugin.converter;

import com.asmolabs.vellum.plugin.VellumPlugin;

import java.io.File;
import java.util.Map;

public interface BookConverterPlugin extends VellumPlugin {

    /**
     * Source format extension (e.g. "cbz", "epub", "pdf", "mobi", "txt").
     */
    String getSourceFormat();

    /**
     * Target format extension (e.g. "pdf", "cbz", "txt", "epub").
     */
    String getTargetFormat();

    /**
     * Returns true if this converter plugin can transform from fromExt to toExt.
     */
    default boolean canConvert(String fromExt, String toExt) {
        if (fromExt == null || toExt == null) return false;
        return getSourceFormat().equalsIgnoreCase(fromExt) && getTargetFormat().equalsIgnoreCase(toExt);
    }

    /**
     * Converts the book file from source format to target format.
     *
     * @param inputFile The source book file
     * @param outputFile The destination file to write converted data to
     * @param config Configuration parameters
     * @return The resulting output file
     * @throws Exception If conversion fails
     */
    File convert(File inputFile, File outputFile, Map<String, Object> config) throws Exception;
}
