package com.asmolabs.vellum.config;

import org.springframework.boot.context.properties.ConfigurationProperties;
import org.springframework.stereotype.Component;

import java.io.File;

@Component
@ConfigurationProperties(prefix = "vellum")
public class AppProperties {

    private String libraryPath = System.getProperty("user.home") + File.separator + ".vellum" + File.separator + "library";
    private LitestreamConfig litestream = new LitestreamConfig();

    public String getLibraryPath() {
        return libraryPath;
    }

    public void setLibraryPath(String libraryPath) {
        this.libraryPath = libraryPath;
    }

    public LitestreamConfig getLitestream() {
        return litestream;
    }

    public void setLitestream(LitestreamConfig litestream) {
        this.litestream = litestream;
    }

    public static class LitestreamConfig {
        private boolean enabled = false;
        private String replicaType = "file";
        private String replicaPath = System.getProperty("user.home") + File.separator + ".vellum" + File.separator + "backups";
        private String binaryPath = "litestream";

        public boolean isEnabled() {
            return enabled;
        }

        public void setEnabled(boolean enabled) {
            this.enabled = enabled;
        }

        public String getReplicaType() {
            return replicaType;
        }

        public void setReplicaType(String replicaType) {
            this.replicaType = replicaType;
        }

        public String getReplicaPath() {
            return replicaPath;
        }

        public void setReplicaPath(String replicaPath) {
            this.replicaPath = replicaPath;
        }

        public String getBinaryPath() {
            return binaryPath;
        }

        public void setBinaryPath(String binaryPath) {
            this.binaryPath = binaryPath;
        }
    }
}
