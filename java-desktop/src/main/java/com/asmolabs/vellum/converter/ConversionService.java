package com.asmolabs.vellum.converter;

import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.stereotype.Service;

import java.io.File;
import java.io.IOException;
import java.util.*;

@Service
public class ConversionService {

    private static final Logger log = LoggerFactory.getLogger(ConversionService.class);

    private final List<FormatConverter> converters;

    public ConversionService(List<FormatConverter> converters) {
        this.converters = converters != null ? converters : List.of();
        log.info("Initialized ConversionService with {} Java format converters", this.converters.size());
    }

    public List<String> getAvailableTargetFormats(String sourceFormat) {
        if (sourceFormat == null) return List.of();
        String src = sourceFormat.toLowerCase();
        Set<String> targets = new LinkedHashSet<>();
        for (FormatConverter c : converters) {
            if (c.getSourceFormat().equalsIgnoreCase(src)) {
                targets.add(c.getTargetFormat().toLowerCase());
            }
        }
        return new ArrayList<>(targets);
    }

    public boolean canConvert(String fromExt, String toExt) {
        if (fromExt == null || toExt == null) return false;
        return converters.stream().anyMatch(c -> c.canConvert(fromExt, toExt));
    }

    public File convert(File inputFile, String targetFormat, Map<String, Object> options) throws Exception {
        if (inputFile == null || !inputFile.exists()) {
            throw new IllegalArgumentException("Fichier source introuvable : " + (inputFile != null ? inputFile.getPath() : "null"));
        }

        String filename = inputFile.getName();
        int dot = filename.lastIndexOf('.');
        if (dot < 0) {
            throw new IllegalArgumentException("Le fichier source n'a pas d'extension : " + filename);
        }
        String fromExt = filename.substring(dot + 1).toLowerCase();
        String toExt = targetFormat.toLowerCase().replaceFirst("^\\.", "");

        FormatConverter converter = converters.stream()
                .filter(c -> c.canConvert(fromExt, toExt))
                .findFirst()
                .orElseThrow(() -> new UnsupportedOperationException(
                        String.format("Aucun moteur de conversion disponible pour %s -> %s", fromExt.toUpperCase(), toExt.toUpperCase())));

        String baseName = filename.substring(0, dot);
        File tempOutput = File.createTempFile("vellum_conv_" + baseName + "_", "." + toExt);
        tempOutput.deleteOnExit();

        log.info("Converting {} -> {} using {} for {}", fromExt, toExt, converter.getClass().getSimpleName(), inputFile.getName());
        return converter.convert(inputFile, tempOutput, options != null ? options : Map.of());
    }

    public File convert(File inputFile, String targetFormat) throws Exception {
        return convert(inputFile, targetFormat, Map.of());
    }
}
