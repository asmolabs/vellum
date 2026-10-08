package com.asmolabs.vellum.plugin;

import java.io.File;
import java.util.Map;

public interface MetadataEnricherPlugin extends VellumPlugin {

    /**
     * Enriches book metadata during import.
     *
     * @param bookFile The temporary imported book file
     * @param currentMetadata The currently extracted metadata map (title, author, publisher, date, tags, description, etc.)
     * @param config User configuration for this plugin
     * @return Enriched metadata map
     */
    Map<String, Object> enrichMetadata(File bookFile, Map<String, Object> currentMetadata, Map<String, Object> config);
}
