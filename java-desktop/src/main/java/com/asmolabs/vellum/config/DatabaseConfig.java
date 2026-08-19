package com.asmolabs.vellum.config;

import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.sqlite.SQLiteConfig;
import org.sqlite.SQLiteDataSource;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;
import org.springframework.context.annotation.Primary;

import javax.sql.DataSource;
import java.io.File;

@Configuration
public class DatabaseConfig {

    private static final Logger log = LoggerFactory.getLogger(DatabaseConfig.class);

    @Value("${spring.datasource.url}")
    private String dbUrl;

    @Bean
    @Primary
    public DataSource dataSource() {
        log.info("Initializing SQLite DataSource with URL: {}", dbUrl);

        // Ensure database parent directory exists
        try {
            if (dbUrl.startsWith("jdbc:sqlite:")) {
                String filePath = dbUrl.substring("jdbc:sqlite:".length());
                File dbFile = new File(filePath);
                if (dbFile.getParentFile() != null && !dbFile.getParentFile().exists()) {
                    dbFile.getParentFile().mkdirs();
                }
            }
        } catch (Exception e) {
            log.warn("Could not ensure database directory exists: {}", e.getMessage());
        }

        SQLiteConfig config = new SQLiteConfig();
        // WAL Mode is required for concurrent reads and Litestream continuous replication
        config.setJournalMode(SQLiteConfig.JournalMode.WAL);
        config.setSynchronous(SQLiteConfig.SynchronousMode.NORMAL);
        config.enforceForeignKeys(true);
        config.setBusyTimeout(5000);

        SQLiteDataSource dataSource = new SQLiteDataSource(config);
        dataSource.setUrl(dbUrl);
        return dataSource;
    }
}
