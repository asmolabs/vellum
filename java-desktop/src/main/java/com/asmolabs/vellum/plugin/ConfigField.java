package com.asmolabs.vellum.plugin;

import com.fasterxml.jackson.annotation.JsonIgnoreProperties;

import java.util.List;

@JsonIgnoreProperties(ignoreUnknown = true)
public record ConfigField(
        String key,
        String label,
        String type, // "text", "password", "secret", "boolean", "select", "file", "directory", "slider", "number", "textarea"
        Object defaultValue,
        String description,
        List<String> options,
        Double min,
        Double max,
        Double step
) {
    public ConfigField {
        if (options == null) {
            options = List.of();
        }
    }

    public ConfigField(String key, String label, String type, Object defaultValue, String description, List<String> options) {
        this(key, label, type, defaultValue, description, options, null, null, null);
    }

    public ConfigField(String key, String label, String type, Object defaultValue, String description) {
        this(key, label, type, defaultValue, description, List.of(), null, null, null);
    }

    public ConfigField(String key, String label, String type, Object defaultValue, String description, double min, double max, double step) {
        this(key, label, type, defaultValue, description, List.of(), min, max, step);
    }
}
