package com.asmolabs.vellum.converter;

import java.io.File;
import java.util.Map;

public interface FormatConverter {

    /**
     * Source format extension (without dot), e.g. "cbz", "epub", "pdf", "mobi", "txt".
     */
    String getSourceFormat();

    /**
     * Target format extension (without dot), e.g. "pdf", "epub", "cbz", "txt".
     */
    String getTargetFormat();

    /**
     * Returns true if this converter can transform from fromExt to toExt.
     */
    default boolean canConvert(String fromExt, String toExt) {
        return getSourceFormat().equalsIgnoreCase(fromExt) && getTargetFormat().equalsIgnoreCase(toExt);
    }

    /**
     * Executes the conversion from inputFile to outputFile.
     *
     * @param inputFile The existing book file
     * @param outputFile The destination file where converted data should be written
     * @param options Custom options map
     * @return The resulting output file
     * @throws Exception If conversion fails
     */
    File convert(File inputFile, File outputFile, Map<String, Object> options) throws Exception;
}
