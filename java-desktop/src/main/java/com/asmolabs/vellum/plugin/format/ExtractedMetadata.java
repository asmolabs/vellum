package com.asmolabs.vellum.plugin.format;

import java.util.List;

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
    public ExtractedMetadata {
        if (tags == null) {
            tags = List.of();
        }
    }
}
