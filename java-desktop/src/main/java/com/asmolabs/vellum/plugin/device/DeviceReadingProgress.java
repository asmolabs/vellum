package com.asmolabs.vellum.plugin.device;

public record DeviceReadingProgress(
        String bookTitle,
        double progressPercent,
        boolean isFinished,
        String lastReadAt
) {}
