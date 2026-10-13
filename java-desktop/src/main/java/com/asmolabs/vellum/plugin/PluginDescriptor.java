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
        List<String> supportedExtensions, // For FORMAT plugins
        String sourceFormat, // For CONVERSION plugins
        String targetFormat, // For CONVERSION plugins
        List<ConfigField> configSchema
) {
    public PluginDescriptor {
        if (configSchema == null) {
            configSchema = List.of();
        }
        if (supportedExtensions == null) {
            supportedExtensions = List.of();
        }
        if (version == null || version.isBlank()) {
            version = "1.0.0";
        }
    }

    public PluginDescriptor(
            String id,
            String name,
            String description,
            String version,
            PluginType type,
            boolean external,
            String command,
            List<String> supportedExtensions,
            List<ConfigField> configSchema
    ) {
        this(id, name, description, version, type, external, command, supportedExtensions, null, null, configSchema);
    }

    public PluginDescriptor(
            String id,
            String name,
            String description,
            String version,
            PluginType type,
            boolean external,
            String command,
            List<ConfigField> configSchema
    ) {
        this(id, name, description, version, type, external, command, List.of(), null, null, configSchema);
    }

    public String getTypeString() {
        return type != null ? type.getValue() : "transformation";
    }
}
