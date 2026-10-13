package com.asmolabs.vellum.converter;

import com.asmolabs.vellum.plugin.converter.BookConverterPlugin;
import com.asmolabs.vellum.service.PluginService;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.stereotype.Service;

import java.io.File;
import java.util.*;

@Service
public class ConversionService {

    private static final Logger log = LoggerFactory.getLogger(ConversionService.class);

    private final PluginService pluginService;

    public ConversionService(PluginService pluginService) {
        this.pluginService = pluginService;
    }

    public List<String> getAvailableTargetFormats(String sourceFormat) {
        if (sourceFormat == null) return List.of();
        String src = sourceFormat.toLowerCase();
        Set<String> targets = new LinkedHashSet<>();
        for (BookConverterPlugin c : pluginService.getEnabledConverters()) {
            if (c.getSourceFormat().equalsIgnoreCase(src)) {
                targets.add(c.getTargetFormat().toLowerCase());
            }
        }
        return new ArrayList<>(targets);
    }

    public boolean canConvert(String fromExt, String toExt) {
        if (fromExt == null || toExt == null) return false;
        return pluginService.getConverterPlugin(fromExt, toExt).isPresent();
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

        BookConverterPlugin converter = pluginService.getConverterPlugin(fromExt, toExt)
                .orElseThrow(() -> new UnsupportedOperationException(
                        String.format("Aucun convertisseur actif disponible pour %s -> %s", fromExt.toUpperCase(), toExt.toUpperCase())));

        String baseName = filename.substring(0, dot);
        File tempOutput = File.createTempFile("vellum_conv_" + baseName + "_", "." + toExt);
        tempOutput.deleteOnExit();

        Map<String, Object> pluginConfig = new HashMap<>(pluginService.getPluginParams(converter.getId()));
        if (options != null) {
            pluginConfig.putAll(options);
        }

        log.info("Converting {} -> {} using plugin {} ({}) for {}", fromExt, toExt, converter.getName(), converter.getId(), inputFile.getName());
        return converter.convert(inputFile, tempOutput, pluginConfig);
    }

    public File convert(File inputFile, String targetFormat) throws Exception {
        return convert(inputFile, targetFormat, Map.of());
    }
}
