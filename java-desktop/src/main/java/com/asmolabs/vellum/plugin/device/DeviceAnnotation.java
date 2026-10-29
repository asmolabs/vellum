package com.asmolabs.vellum.plugin.device;

public record DeviceAnnotation(
        String bookTitle,
        String bookAuthor,
        String text,
        String note,
        String location,
        String createdAt
) {}
