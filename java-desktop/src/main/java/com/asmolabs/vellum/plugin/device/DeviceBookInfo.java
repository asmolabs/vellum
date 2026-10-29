package com.asmolabs.vellum.plugin.device;

public record DeviceBookInfo(
        String title,
        String author,
        String filename,
        String relativePath,
        long fileSize,
        String format,
        Double progressPercent
) {}
