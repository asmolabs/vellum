package com.asmolabs.vellum.service;

import com.asmolabs.vellum.domain.entity.Param;
import com.asmolabs.vellum.plugin.*;
import com.asmolabs.vellum.plugin.external.ExternalPluginLoader;
import com.asmolabs.vellum.plugin.format.BookFormatPlugin;
import com.asmolabs.vellum.repository.ParamRepository;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.stereotype.Service;

import java.io.File;
import java.io.IOException;
import java.util.*;
import java.util.concurrent.ConcurrentHashMap;

@Service
public class PluginService {

    private static final Logger log = LoggerFactory.getLogger(PluginService.class);

    private final ParamRepository paramRepository;
    private final ExternalPluginLoader externalPluginLoader;
    private final List<VellumPlugin> internalPlugins;
    private final Map<String, VellumPlugin> allPlugins = new ConcurrentHashMap<>();

    public record ConfigField(
            String key,
            String label,
            String type,
            Object defaultValue,
            String description,
            List<String> options,
            Double min,
            Double max,
            Double step
    ) {
        public ConfigField(String key, String label, String type, Object defaultValue, String description, List<String> options) {
            this(key, label, type, defaultValue, description, options, null, null, null);
        }

        public ConfigField(String key, String label, String type, Object defaultValue, String description) {
            this(key, label, type, defaultValue, description, List.of(), null, null, null);
        }

        public static ConfigField fromPluginConfigField(com.asmolabs.vellum.plugin.ConfigField f) {
            return new ConfigField(f.key(), f.label(), f.type(), f.defaultValue(), f.description(), f.options(), f.min(), f.max(), f.step());
        }
    }

    public record LibraryPlugin(
            String id,
            String name,
            String description,
            String version,
            String type, // "import" or "transformation"
            boolean external,
            boolean enabled,
            List<ConfigField> configSchema
    ) {
        public LibraryPlugin(String id, String name, String description, String version, String type, List<ConfigField> configSchema) {
            this(id, name, description, version, type, false, true, configSchema);
        }
    }

    public PluginService(
            ParamRepository paramRepository,
            ExternalPluginLoader externalPluginLoader,
            List<VellumPlugin> internalPlugins
    ) {
        this.paramRepository = paramRepository;
        this.externalPluginLoader = externalPluginLoader;
        this.internalPlugins = internalPlugins != null ? internalPlugins : List.of();
        reloadPlugins();
    }

    public synchronized void reloadPlugins() {
        allPlugins.clear();

        // 1. Register internal Spring components
        for (VellumPlugin plugin : internalPlugins) {
            allPlugins.put(plugin.getId(), plugin);
            log.info("Registered internal plugin: {} ({})", plugin.getName(), plugin.getId());
        }

        // 2. Discover and register external plugins
        List<VellumPlugin> external = externalPluginLoader.loadExternalPlugins();
        for (VellumPlugin plugin : external) {
            allPlugins.put(plugin.getId(), plugin);
            log.info("Registered external plugin: {} ({})", plugin.getName(), plugin.getId());
        }
    }

    public File getPluginsDirectory() {
        return externalPluginLoader.getPluginsDirectory();
    }

    public List<LibraryPlugin> getRegisteredPlugins() {
        List<LibraryPlugin> list = new ArrayList<>();
        for (VellumPlugin plugin : allPlugins.values()) {
            PluginDescriptor desc = plugin.getDescriptor();
            boolean enabled = isPluginEnabled(desc.id());

            List<ConfigField> fields = desc.configSchema().stream()
                    .map(ConfigField::fromPluginConfigField)
                    .toList();

            list.add(new LibraryPlugin(
                    desc.id(),
                    desc.name(),
                    desc.description(),
                    desc.version(),
                    desc.getTypeString(),
                    desc.external(),
                    enabled,
                    fields
            ));
        }

        // Sort: internal first, then by name
        list.sort(Comparator.comparing(LibraryPlugin::external).thenComparing(LibraryPlugin::name));
        return list;
    }

    public Optional<VellumPlugin> getPluginById(String id) {
        return Optional.ofNullable(allPlugins.get(id));
    }

    public boolean isPluginEnabled(String pluginId) {
        Optional<Param> param = paramRepository.findByPluginIdAndKey(pluginId, "enabled");
        if (param.isEmpty()) {
            return true; // Enabled by default
        }
        Object val = param.get().getData().get("enabled");
        return val == null || Boolean.parseBoolean(String.valueOf(val));
    }

    public void setPluginEnabled(String pluginId, boolean enabled) {
        Param param = paramRepository.findByPluginIdAndKey(pluginId, "enabled")
                .orElseGet(() -> new Param("plugin:" + pluginId + ":enabled", pluginId, "enabled", "{}"));
        param.setData(Map.of("enabled", enabled));
        paramRepository.save(param);
    }

    public Map<String, Object> getPluginParams(String pluginId) {
        Optional<Param> paramOpt = paramRepository.findByPluginIdAndKey(pluginId, "config");
        Map<String, Object> saved = paramOpt.map(Param::getData).orElseGet(HashMap::new);

        Optional<VellumPlugin> pluginOpt = getPluginById(pluginId);
        if (pluginOpt.isEmpty()) {
            return saved;
        }

        Map<String, Object> merged = new HashMap<>();
        for (com.asmolabs.vellum.plugin.ConfigField field : pluginOpt.get().getDescriptor().configSchema()) {
            merged.put(field.key(), saved.getOrDefault(field.key(), field.defaultValue()));
        }
        return merged;
    }

    public Map<String, Object> savePluginParams(String pluginId, Map<String, Object> config) {
        Param param = paramRepository.findByPluginIdAndKey(pluginId, "config")
                .orElseGet(() -> new Param("plugin:" + pluginId + ":config", pluginId, "config", "{}"));

        param.setData(config);
        param = paramRepository.save(param);
        return param.getData();
    }

    public Map<String, Object> runImportPlugins(File tempFile, Map<String, Object> metadata) {
        Map<String, Object> enriched = new HashMap<>(metadata);

        for (VellumPlugin plugin : allPlugins.values()) {
            if (plugin instanceof MetadataEnricherPlugin enricher && isPluginEnabled(plugin.getId())) {
                try {
                    Map<String, Object> config = getPluginParams(plugin.getId());
                    enriched = enricher.enrichMetadata(tempFile, enriched, config);
                } catch (Exception e) {
                    log.warn("Error running import enricher plugin {}: {}", plugin.getId(), e.getMessage());
                }
            }
        }

        return enriched;
    }

    public File runTransformationPlugin(File inputFile, String pluginId) throws Exception {
        VellumPlugin plugin = allPlugins.get(pluginId);
        if (plugin == null) {
            throw new IllegalArgumentException("Plugin non trouvé : " + pluginId);
        }

        if (!isPluginEnabled(pluginId)) {
            throw new IllegalStateException("Le plugin " + plugin.getName() + " est désactivé.");
        }

        if (!(plugin instanceof BookTransformationPlugin transformer)) {
            throw new IllegalArgumentException("Le plugin " + pluginId + " n'est pas un plugin de transformation de livre.");
        }

        Map<String, Object> params = getPluginParams(pluginId);
        return transformer.transform(inputFile, params);
    }

    public Optional<BookFormatPlugin> getFormatPluginFor(File file) {
        if (file == null) return Optional.empty();
        return allPlugins.values().stream()
                .filter(p -> p instanceof BookFormatPlugin)
                .map(p -> (BookFormatPlugin) p)
                .filter(p -> isPluginEnabled(p.getId()))
                .filter(p -> p.canHandle(file))
                .findFirst();
    }

    public List<String> getSupportedFormatExtensions() {
        Set<String> extensions = new LinkedHashSet<>();
        for (VellumPlugin plugin : allPlugins.values()) {
            if (plugin instanceof BookFormatPlugin formatPlugin && isPluginEnabled(plugin.getId())) {
                extensions.addAll(formatPlugin.getSupportedExtensions().stream().map(String::toLowerCase).toList());
            }
        }
        return new ArrayList<>(extensions);
    }

    public <T extends VellumPlugin> List<T> getPluginsOfType(Class<T> type) {
        return allPlugins.values().stream()
                .filter(type::isInstance)
                .map(type::cast)
                .filter(p -> isPluginEnabled(p.getId()))
                .toList();
    }

    public List<com.asmolabs.vellum.plugin.converter.BookConverterPlugin> getEnabledConverters() {
        return getPluginsOfType(com.asmolabs.vellum.plugin.converter.BookConverterPlugin.class);
    }

    public Optional<com.asmolabs.vellum.plugin.converter.BookConverterPlugin> getConverterPlugin(String fromExt, String toExt) {
        if (fromExt == null || toExt == null) return Optional.empty();
        return getEnabledConverters().stream()
                .filter(c -> c.canConvert(fromExt, toExt))
                .findFirst();
    }

    public VellumPlugin.TestResult testPluginConfiguration(String pluginId, Map<String, Object> config) {
        Optional<VellumPlugin> pluginOpt = getPluginById(pluginId);
        if (pluginOpt.isEmpty()) {
            return VellumPlugin.TestResult.error("Plugin introuvable : " + pluginId);
        }
        try {
            return pluginOpt.get().testConfiguration(config != null ? config : getPluginParams(pluginId));
        } catch (Exception e) {
            return VellumPlugin.TestResult.error("Erreur lors de l'exécution du test : " + e.getMessage());
        }
    }
}
