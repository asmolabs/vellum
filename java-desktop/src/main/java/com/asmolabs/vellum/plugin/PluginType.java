package com.asmolabs.vellum.plugin;

import com.fasterxml.jackson.annotation.JsonCreator;
import com.fasterxml.jackson.annotation.JsonValue;

public enum PluginType {
    IMPORT("import"),
    TRANSFORMATION("transformation"),
    FORMAT("format"),
    CONVERSION("conversion");

    private final String value;

    PluginType(String value) {
        this.value = value;
    }

    @JsonValue
    public String getValue() {
        return value;
    }

    @JsonCreator
    public static PluginType fromString(String val) {
        if (val == null) return TRANSFORMATION;
        for (PluginType t : values()) {
            if (t.value.equalsIgnoreCase(val) || t.name().equalsIgnoreCase(val)) {
                return t;
            }
        }
        return TRANSFORMATION;
    }
}
