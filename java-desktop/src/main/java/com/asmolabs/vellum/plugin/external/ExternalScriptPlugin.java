package com.asmolabs.vellum.plugin.external;

import com.asmolabs.vellum.plugin.*;
import com.fasterxml.jackson.core.type.TypeReference;
import com.fasterxml.jackson.databind.ObjectMapper;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.io.File;
import java.io.IOException;
import java.nio.file.Files;
import java.util.*;
import java.util.concurrent.TimeUnit;

public class ExternalScriptPlugin implements BookTransformationPlugin, MetadataEnricherPlugin {

    private static final Logger log = LoggerFactory.getLogger(ExternalScriptPlugin.class);
    private static final ObjectMapper objectMapper = new ObjectMapper();

    private final File pluginDirectory;
    private final PluginDescriptor descriptor;

    public ExternalScriptPlugin(File pluginDirectory, PluginDescriptor descriptor) {
        this.pluginDirectory = pluginDirectory;
        this.descriptor = descriptor;
    }

    @Override
    public PluginDescriptor getDescriptor() {
        return descriptor;
    }

    public File getPluginDirectory() {
        return pluginDirectory;
    }

    @Override
    public File transform(File inputFile, Map<String, Object> config) throws Exception {
        if (descriptor.command() == null || descriptor.command().isBlank()) {
            throw new IllegalStateException("External plugin " + getId() + " has no 'command' specified in plugin.json");
        }

        String baseName = inputFile.getName();
        int dot = baseName.lastIndexOf('.');
        String prefix = (dot > 0) ? baseName.substring(0, dot) : baseName;
        String ext = (dot > 0) ? baseName.substring(dot) : ".epub";

        File outputFile = new File(inputFile.getParentFile(), prefix + " (" + descriptor.name() + ")" + ext);
        String configJson = objectMapper.writeValueAsString(config != null ? config : Map.of());

        List<String> commandTokens = parseCommand(descriptor.command());
        List<String> fullCommand = new ArrayList<>(commandTokens);
        fullCommand.add("--input");
        fullCommand.add(inputFile.getAbsolutePath());
        fullCommand.add("--output");
        fullCommand.add(outputFile.getAbsolutePath());
        fullCommand.add("--config");
        fullCommand.add(configJson);

        log.info("Executing external transformation plugin [{}]: {}", getId(), fullCommand);

        ProcessBuilder pb = new ProcessBuilder(fullCommand);
        pb.directory(pluginDirectory);
        Map<String, String> env = pb.environment();
        env.put("VELLUM_INPUT_FILE", inputFile.getAbsolutePath());
        env.put("VELLUM_OUTPUT_FILE", outputFile.getAbsolutePath());
        env.put("VELLUM_CONFIG_JSON", configJson);
        env.put("VELLUM_PLUGIN_ID", getId());

        Process process = pb.start();

        String stdout = new String(process.getInputStream().readAllBytes());
        String stderr = new String(process.getErrorStream().readAllBytes());

        boolean finished = process.waitFor(120, TimeUnit.SECONDS);
        if (!finished) {
            process.destroyForcibly();
            throw new RuntimeException("External plugin " + getId() + " timed out after 120s");
        }

        int exitCode = process.exitValue();
        if (exitCode != 0) {
            log.error("Plugin {} failed with exit code {}. Stderr: {}", getId(), exitCode, stderr);
            throw new RuntimeException("Plugin " + getId() + " failed (code " + exitCode + "): " + stderr);
        }

        if (!outputFile.exists()) {
            // Check if plugin modified in-place or printed output file path
            String trimmedOut = stdout.trim();
            File potentialOut = new File(trimmedOut);
            if (potentialOut.exists() && potentialOut.isFile()) {
                return potentialOut;
            }
            throw new IOException("Plugin " + getId() + " succeeded but output file was not found at: " + outputFile.getAbsolutePath());
        }

        return outputFile;
    }

    @Override
    public Map<String, Object> enrichMetadata(File bookFile, Map<String, Object> currentMetadata, Map<String, Object> config) {
        if (descriptor.command() == null || descriptor.command().isBlank()) {
            return currentMetadata;
        }

        File tempIn = null;
        File tempOut = null;
        try {
            tempIn = File.createTempFile("vellum_meta_in_", ".json");
            tempOut = File.createTempFile("vellum_meta_out_", ".json");

            objectMapper.writeValue(tempIn, currentMetadata);
            String configJson = objectMapper.writeValueAsString(config != null ? config : Map.of());

            List<String> commandTokens = parseCommand(descriptor.command());
            List<String> fullCommand = new ArrayList<>(commandTokens);
            fullCommand.add("--input");
            fullCommand.add(bookFile.getAbsolutePath());
            fullCommand.add("--metadata-in");
            fullCommand.add(tempIn.getAbsolutePath());
            fullCommand.add("--metadata-out");
            fullCommand.add(tempOut.getAbsolutePath());
            fullCommand.add("--config");
            fullCommand.add(configJson);

            log.info("Executing external enricher plugin [{}]: {}", getId(), fullCommand);

            ProcessBuilder pb = new ProcessBuilder(fullCommand);
            pb.directory(pluginDirectory);
            Map<String, String> env = pb.environment();
            env.put("VELLUM_INPUT_FILE", bookFile.getAbsolutePath());
            env.put("VELLUM_CONFIG_JSON", configJson);
            env.put("VELLUM_METADATA_IN", tempIn.getAbsolutePath());
            env.put("VELLUM_METADATA_OUT", tempOut.getAbsolutePath());

            Process process = pb.start();
            String stdout = new String(process.getInputStream().readAllBytes());
            String stderr = new String(process.getErrorStream().readAllBytes());

            boolean finished = process.waitFor(60, TimeUnit.SECONDS);
            if (!finished) {
                process.destroyForcibly();
                log.warn("External enricher plugin {} timed out", getId());
                return currentMetadata;
            }

            int exitCode = process.exitValue();
            if (exitCode != 0) {
                log.warn("External enricher plugin {} failed (code {}): {}", getId(), exitCode, stderr);
                return currentMetadata;
            }

            // Read output JSON
            if (tempOut.exists() && tempOut.length() > 0) {
                return objectMapper.readValue(tempOut, new TypeReference<Map<String, Object>>() {});
            } else if (!stdout.trim().isBlank() && stdout.trim().startsWith("{")) {
                return objectMapper.readValue(stdout.trim(), new TypeReference<Map<String, Object>>() {});
            }

        } catch (Exception e) {
            log.warn("Failed executing external metadata enricher {}: {}", getId(), e.getMessage());
        } finally {
            if (tempIn != null) tempIn.delete();
            if (tempOut != null) tempOut.delete();
        }

        return currentMetadata;
    }

    private List<String> parseCommand(String commandLine) {
        List<String> tokens = new ArrayList<>();
        StringTokenizer tokenizer = new StringTokenizer(commandLine);
        while (tokenizer.hasMoreTokens()) {
            tokens.add(tokenizer.nextToken());
        }
        return tokens;
    }
}
