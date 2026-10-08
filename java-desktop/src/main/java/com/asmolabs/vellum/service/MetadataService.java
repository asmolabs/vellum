package com.asmolabs.vellum.service;

import com.asmolabs.vellum.plugin.format.BookFormatPlugin;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.stereotype.Service;

import java.io.File;
import java.util.ArrayList;
import java.util.List;
import java.util.Optional;

@Service
public class MetadataService {

    private static final Logger log = LoggerFactory.getLogger(MetadataService.class);

    private final PluginService pluginService;

    public record ExtractedMetadata(
            String title,
            String author,
            String publisher,
            String publishDate,
            String description,
            List<String> tags,
            byte[] coverData,
            String coverExt
    ) {
        public static ExtractedMetadata fromPluginMetadata(com.asmolabs.vellum.plugin.format.ExtractedMetadata m) {
            if (m == null) return null;
            return new ExtractedMetadata(
                    m.title(),
                    m.author(),
                    m.publisher(),
                    m.publishDate(),
                    m.description(),
                    m.tags(),
                    m.coverData(),
                    m.coverExt()
            );
        }
    }

    public MetadataService(PluginService pluginService) {
        this.pluginService = pluginService;
    }

    public boolean isFormatSupported(File file) {
        return pluginService.getFormatPluginFor(file).isPresent();
    }

    public List<String> getSupportedExtensions() {
        return pluginService.getSupportedFormatExtensions();
    }

    public ExtractedMetadata parseMetadata(File file) {
        String filename = file.getName();

        // 1. Try to find a registered BookFormatPlugin (internal or external)
        Optional<BookFormatPlugin> formatPluginOpt = pluginService.getFormatPluginFor(file);
        if (formatPluginOpt.isPresent()) {
            BookFormatPlugin plugin = formatPluginOpt.get();
            try {
                java.util.Map<String, Object> config = pluginService.getPluginParams(plugin.getId());
                com.asmolabs.vellum.plugin.format.ExtractedMetadata pluginMeta = plugin.extractMetadata(file, config);
                if (pluginMeta != null) {
                    String title = pluginMeta.title();
                    String author = pluginMeta.author();

                    // If plugin didn't extract title/author, fallback to filename parsing for those fields
                    if (title == null || title.isBlank() || author == null || author.isBlank()) {
                        String[] fallback = parseAuthorAndTitleFromFilename(filename);
                        if (title == null || title.isBlank()) title = fallback[1];
                        if (author == null || author.isBlank()) author = fallback[0];
                    }

                    return new ExtractedMetadata(
                            title,
                            author,
                            pluginMeta.publisher(),
                            pluginMeta.publishDate(),
                            pluginMeta.description(),
                            pluginMeta.tags() != null ? pluginMeta.tags() : new ArrayList<>(),
                            pluginMeta.coverData(),
                            pluginMeta.coverExt()
                    );
                }
            } catch (Exception e) {
                log.warn("Format plugin {} failed parsing {}: {}", plugin.getId(), file.getName(), e.getMessage());
            }
        }

        // 2. Fallback parser from filename: "Author - Title.ext"
        String[] fallback = parseAuthorAndTitleFromFilename(filename);
        return new ExtractedMetadata(fallback[1], fallback[0], null, null, null, new ArrayList<>(), null, null);
    }

    private String[] parseAuthorAndTitleFromFilename(String filename) {
        String baseName = filename.contains(".") ? filename.substring(0, filename.lastIndexOf('.')) : filename;
        String[] parts = baseName.split(" - ", 2);
        String author = parts.length > 1 ? parts[0].trim() : "Inconnu";
        String title = parts.length > 1 ? parts[1].trim() : baseName.trim();
        return new String[]{author, title};
    }
}
