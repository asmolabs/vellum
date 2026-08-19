package com.asmolabs.vellum.domain.entity;

import com.fasterxml.jackson.core.type.TypeReference;
import com.fasterxml.jackson.databind.ObjectMapper;
import jakarta.persistence.*;
import java.time.LocalDateTime;
import java.util.HashMap;
import java.util.Map;

@Entity
@Table(name = "params")
public class Param {

    private static final ObjectMapper MAPPER = new ObjectMapper();

    @Id
    @Column(name = "id", nullable = false)
    private String id; // Format: "plugin:{plugin_id}:{key}"

    @Column(name = "plugin_id", nullable = false)
    private String pluginId;

    @Column(name = "key", nullable = false)
    private String key = "config";

    @Column(name = "value", nullable = false, columnDefinition = "TEXT")
    private String value = "{}";

    @Column(name = "updated_at")
    private LocalDateTime updatedAt = LocalDateTime.now();

    public Param() {
    }

    public Param(String id, String pluginId, String key, String value) {
        this.id = id;
        this.pluginId = pluginId;
        this.key = key;
        this.value = value;
        this.updatedAt = LocalDateTime.now();
    }

    public Map<String, Object> getData() {
        try {
            return MAPPER.readValue(value, new TypeReference<Map<String, Object>>() {});
        } catch (Exception e) {
            return new HashMap<>();
        }
    }

    public void setData(Map<String, Object> data) {
        try {
            this.value = MAPPER.writeValueAsString(data);
            this.updatedAt = LocalDateTime.now();
        } catch (Exception e) {
            this.value = "{}";
        }
    }

    public String getId() {
        return id;
    }

    public void setId(String id) {
        this.id = id;
    }

    public String getPluginId() {
        return pluginId;
    }

    public void setPluginId(String pluginId) {
        this.pluginId = pluginId;
    }

    public String getKey() {
        return key;
    }

    public void setKey(String key) {
        this.key = key;
    }

    public String getValue() {
        return value;
    }

    public void setValue(String value) {
        this.value = value;
    }

    public LocalDateTime getUpdatedAt() {
        return updatedAt;
    }

    public void setUpdatedAt(LocalDateTime updatedAt) {
        this.updatedAt = updatedAt;
    }
}
