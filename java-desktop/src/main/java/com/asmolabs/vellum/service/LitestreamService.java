package com.asmolabs.vellum.service;

import com.asmolabs.vellum.config.AppProperties;
import jakarta.annotation.PostConstruct;
import jakarta.annotation.PreDestroy;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.stereotype.Service;

import java.io.File;
import java.io.FileWriter;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.concurrent.atomic.AtomicBoolean;

@Service
public class LitestreamService {

    private static final Logger log = LoggerFactory.getLogger(LitestreamService.class);

    private final AppProperties appProperties;

    @Value("${spring.datasource.url}")
    private String dbUrl;

    private Process litestreamProcess;
    private final AtomicBoolean isReplicating = new AtomicBoolean(false);
    private String lastStatusMessage = "Litestream initialisé";

    public enum Status {
        RUNNING,
        STOPPED,
        BINARY_NOT_FOUND,
        DISABLED,
        ERROR
    }

    public LitestreamService(AppProperties appProperties) {
        this.appProperties = appProperties;
    }

    @PostConstruct
    public void init() {
        if (appProperties.getLitestream().isEnabled()) {
            startReplication();
        } else {
            lastStatusMessage = "Litestream désactivé dans la configuration";
        }
    }

    @PreDestroy
    public void shutdown() {
        stopReplication();
    }

    public synchronized Status getStatus() {
        if (!appProperties.getLitestream().isEnabled()) {
            return Status.DISABLED;
        }
        if (!isLitestreamInstalled()) {
            return Status.BINARY_NOT_FOUND;
        }
        if (litestreamProcess != null && litestreamProcess.isAlive()) {
            return Status.RUNNING;
        }
        return Status.STOPPED;
    }

    public String getLastStatusMessage() {
        return lastStatusMessage;
    }

    public boolean isLitestreamInstalled() {
        try {
            Process process = new ProcessBuilder(appProperties.getLitestream().getBinaryPath(), "version").start();
            int exitCode = process.waitFor();
            return exitCode == 0;
        } catch (Exception e) {
            return false;
        }
    }

    public File getDatabaseFile() {
        if (dbUrl.startsWith("jdbc:sqlite:")) {
            return new File(dbUrl.substring("jdbc:sqlite:".length()));
        }
        return new File(System.getProperty("user.home") + "/.vellum/metadata.db");
    }

    public File generateConfigFile() throws IOException {
        File dbFile = getDatabaseFile();
        File configDir = dbFile.getParentFile() != null ? dbFile.getParentFile() : new File(".");
        File configFile = new File(configDir, "litestream.yml");

        String replicaPath = appProperties.getLitestream().getReplicaPath();
        File replicaDir = new File(replicaPath);
        if (!replicaDir.exists()) {
            replicaDir.mkdirs();
        }

        String yaml = """
            # Litestream SQLite Continuous Replication Configuration
            dbs:
              - path: "%s"
                replicas:
                  - path: "%s"
                    sync-interval: 1s
            """.formatted(
                dbFile.getAbsolutePath().replace('\\', '/'),
                replicaDir.getAbsolutePath().replace('\\', '/')
        );

        try (FileWriter writer = new FileWriter(configFile)) {
            writer.write(yaml);
        }

        log.info("Generated Litestream configuration at {}", configFile.getAbsolutePath());
        return configFile;
    }

    public synchronized boolean startReplication() {
        if (!isLitestreamInstalled()) {
            lastStatusMessage = "Binaire 'litestream' introuvable dans le PATH.";
            log.warn(lastStatusMessage);
            return false;
        }

        try {
            File configFile = generateConfigFile();
            ProcessBuilder pb = new ProcessBuilder(
                    appProperties.getLitestream().getBinaryPath(),
                    "replicate",
                    "-config",
                    configFile.getAbsolutePath()
            );
            pb.redirectErrorStream(true);

            litestreamProcess = pb.start();
            isReplicating.set(true);
            lastStatusMessage = "Réplication active vers " + appProperties.getLitestream().getReplicaPath();
            log.info("Litestream replication started successfully.");

            Thread.ofVirtual().start(() -> {
                try {
                    litestreamProcess.waitFor();
                    isReplicating.set(false);
                    lastStatusMessage = "Litestream s'est arrêté.";
                } catch (InterruptedException ignored) {
                }
            });

            return true;
        } catch (Exception e) {
            lastStatusMessage = "Erreur lors du démarrage de Litestream: " + e.getMessage();
            log.error("Failed to start Litestream", e);
            return false;
        }
    }

    public synchronized void stopReplication() {
        if (litestreamProcess != null && litestreamProcess.isAlive()) {
            litestreamProcess.destroy();
            try {
                litestreamProcess.waitFor();
            } catch (InterruptedException ignored) {
            }
            litestreamProcess = null;
            isReplicating.set(false);
            lastStatusMessage = "Réplication arrêtée";
            log.info("Litestream replication stopped.");
        }
    }

    public boolean restoreLatestBackup(File targetDbFile) {
        if (!isLitestreamInstalled()) return false;
        try {
            File configFile = generateConfigFile();
            ProcessBuilder pb = new ProcessBuilder(
                    appProperties.getLitestream().getBinaryPath(),
                    "restore",
                    "-config",
                    configFile.getAbsolutePath(),
                    "-o",
                    targetDbFile.getAbsolutePath(),
                    getDatabaseFile().getAbsolutePath()
            );
            Process p = pb.start();
            int code = p.waitFor();
            return code == 0;
        } catch (Exception e) {
            log.error("Failed to restore from Litestream", e);
            return false;
        }
    }
}
