package com.asmolabs.vellum.plugin;

import com.fasterxml.jackson.annotation.JsonIgnoreProperties;

import java.util.List;

@JsonIgnoreProperties(ignoreUnknown = true)
public record PluginDescriptor(
        String id,
        String name,
        String description,
        String version,
        PluginType type,
        boolean external,
        String command, // Only for external script plugins
        List<ConfigField> configSchema
) {
    public PluginDescriptor {
        if (configSchema == null) {
            configSchema = List.of();
        }
        if (version == null || version.isBlank()) {
            version = "1.0.0";
        }
    }

    public String getTypeString() {
        return type != null ? type.getValue() : "transformation";
    }
}
