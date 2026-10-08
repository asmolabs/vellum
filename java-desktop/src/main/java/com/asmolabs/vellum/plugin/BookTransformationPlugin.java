package com.asmolabs.vellum.plugin;

import java.io.File;
import java.util.Map;

public interface BookTransformationPlugin extends VellumPlugin {

    /**
     * Transforms an existing book file (cleaning, converting, translating, etc.).
     *
     * @param inputFile The source book file
     * @param config User configuration for this plugin
     * @return The transformed output file
     * @throws Exception If transformation fails
     */
    File transform(File inputFile, Map<String, Object> config) throws Exception;
}
