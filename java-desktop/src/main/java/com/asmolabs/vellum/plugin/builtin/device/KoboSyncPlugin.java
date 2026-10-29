package com.asmolabs.vellum.plugin.builtin.device;

import com.asmolabs.vellum.domain.entity.Book;
import com.asmolabs.vellum.plugin.ConfigField;
import com.asmolabs.vellum.plugin.PluginDescriptor;
import com.asmolabs.vellum.plugin.PluginType;
import com.asmolabs.vellum.plugin.device.*;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.stereotype.Component;

import java.io.File;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.StandardCopyOption;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.ResultSet;
import java.sql.Statement;
import java.util.*;

@Component
public class KoboSyncPlugin implements DeviceSyncPlugin {

    private static final Logger log = LoggerFactory.getLogger(KoboSyncPlugin.class);

    private final PluginDescriptor descriptor;

    public KoboSyncPlugin() {
        this.descriptor = new PluginDescriptor(
                "device-kobo",
                "Kobo eReader Sync",
                "Synchronisation complète avec les liseuses Kobo : export KePub ultra-rapide, gestion de la pagination ACCESS, import des livres et des annotations.",
                "1.0.0",
                PluginType.DEVICE,
                false,
                null,
                List.of(
                        new ConfigField("customMountPath", "Point de montage personnalisé", "string", "", "Chemin absolu vers la liseuse si non détectée automatiquement (ex: /Volumes/KOBOeReader)."),
                        new ConfigField("exportFormat", "Format d'exportation", "select", "kepub", "Format privilégié lors du transfert vers Kobo (KePub active la pagination rapide par chapitre).", List.of("kepub", "epub")),
                        new ConfigField("targetDirectory", "Dossier de stockage", "string", "Books", "Sous-dossier où copier les livres sur la Kobo (par défaut : Books).")
                )
        );
    }

    @Override
    public PluginDescriptor getDescriptor() {
        return descriptor;
    }

    @Override
    public DeviceKind getDeviceKind() {
        return DeviceKind.KOBO;
    }

    @Override
    public List<File> getDefaultMountPaths() {
        List<File> paths = new ArrayList<>();
        paths.add(new File("/Volumes/KOBOeReader"));
        paths.add(new File("/media/KOBOeReader"));

        String user = System.getenv("USER");
        if (user != null && !user.isBlank()) {
            paths.add(new File("/media/" + user + "/KOBOeReader"));
            paths.add(new File("/run/media/" + user + "/KOBOeReader"));
        }

        // Windows drives
        for (char d = 'D'; d <= 'I'; d++) {
            paths.add(new File(d + ":\\"));
        }

        return paths;
    }

    @Override
    public Optional<File> detectDevice(File customMount) {
        if (customMount != null && customMount.exists()) {
            if (isKoboDirectory(customMount)) {
                return Optional.of(customMount);
            }
        }

        for (File p : getDefaultMountPaths()) {
            if (p.exists() && isKoboDirectory(p)) {
                return Optional.of(p);
            }
        }

        return Optional.empty();
    }

    private boolean isKoboDirectory(File dir) {
        File koboDir = new File(dir, ".kobo");
        if (!koboDir.exists()) return false;
        File db = new File(koboDir, "KoboReader.sqlite");
        File version = new File(koboDir, "version");
        return db.exists() || version.exists();
    }

    @Override
    public List<DeviceBookInfo> listBooks(File mountPoint) throws Exception {
        List<DeviceBookInfo> books = new ArrayList<>();
        File dbFile = new File(mountPoint, ".kobo" + File.separator + "KoboReader.sqlite");

        if (dbFile.exists()) {
            try {
                String jdbcUrl = "jdbc:sqlite:" + dbFile.getAbsolutePath();
                try (Connection conn = DriverManager.getConnection(jdbcUrl);
                     Statement stmt = conn.createStatement()) {

                    String sql = "SELECT ContentID, Title, Attribution, ___PercentRead, MimeType FROM content WHERE ContentType = 6";
                    try (ResultSet rs = stmt.executeQuery(sql)) {
                        while (rs.next()) {
                            String contentId = rs.getString("ContentID");
                            String title = rs.getString("Title");
                            String author = rs.getString("Attribution");
                            double pct = rs.getDouble("___PercentRead");

                            if (contentId != null && !contentId.isBlank()) {
                                File bookFile = new File(contentId);
                                if (!bookFile.isAbsolute()) {
                                    bookFile = new File(mountPoint, contentId);
                                }
                                long size = bookFile.exists() ? bookFile.length() : 0;
                                String ext = getExtension(bookFile.getName());

                                books.add(new DeviceBookInfo(
                                        title != null && !title.isBlank() ? title : bookFile.getName(),
                                        author != null ? author : "Inconnu",
                                        bookFile.getName(),
                                        contentId,
                                        size,
                                        ext,
                                        pct >= 0 ? pct : null
                                ));
                            }
                        }
                    }
                }
                return books;
            } catch (Exception e) {
                log.warn("Kobo SQLite read failed, falling back to filesystem scan: {}", e.getMessage());
            }
        }

        // Filesystem scan fallback
        File booksDir = new File(mountPoint, "Books");
        File scanDir = booksDir.exists() ? booksDir : mountPoint;
        scanDirectoryForBooks(scanDir, mountPoint, books);
        return books;
    }

    private void scanDirectoryForBooks(File dir, File root, List<DeviceBookInfo> books) {
        File[] files = dir.listFiles();
        if (files == null) return;
        for (File f : files) {
            if (f.getName().startsWith(".")) continue;
            if (f.isDirectory()) {
                scanDirectoryForBooks(f, root, books);
            } else {
                String name = f.getName().toLowerCase();
                if (name.endsWith(".epub") || name.endsWith(".pdf") || name.endsWith(".cbz") || name.endsWith(".txt")) {
                    String relPath = root.toPath().relativize(f.toPath()).toString();
                    String ext = getExtension(f.getName());
                    books.add(new DeviceBookInfo(
                            f.getName(),
                            "Liseuse Kobo",
                            f.getName(),
                            relPath,
                            f.length(),
                            ext,
                            null
                    ));
                }
            }
        }
    }

    @Override
    public File exportBook(Book book, File sourceFile, File mountPoint, Map<String, Object> config) throws Exception {
        String targetDirName = String.valueOf(config.getOrDefault("targetDirectory", "Books"));
        String exportFormat = String.valueOf(config.getOrDefault("exportFormat", "kepub")).toLowerCase();

        String cleanAuthor = sanitizeFilename(book.getAuthor());
        String cleanTitle = sanitizeFilename(book.getTitle());

        File destDir = new File(mountPoint, targetDirName + File.separator + cleanAuthor);
        destDir.mkdirs();

        String srcName = sourceFile.getName().toLowerCase();
        File destFile;

        if ("kepub".equals(exportFormat) && srcName.endsWith(".epub") && !srcName.endsWith(".kepub.epub")) {
            destFile = new File(destDir, cleanTitle + ".kepub.epub");
            log.info("Converting EPUB to KEPUB for Kobo: {}", destFile.getName());
            KepubifyService.convertEpubToKepub(sourceFile, destFile);
        } else {
            String ext = getExtension(sourceFile.getName());
            destFile = new File(destDir, cleanTitle + "." + ext);
            Files.copy(sourceFile.toPath(), destFile.toPath(), StandardCopyOption.REPLACE_EXISTING);
        }

        return destFile;
    }

    @Override
    public List<DeviceReadingProgress> getReadingProgress(File mountPoint) throws Exception {
        List<DeviceReadingProgress> list = new ArrayList<>();
        File dbFile = new File(mountPoint, ".kobo" + File.separator + "KoboReader.sqlite");
        if (!dbFile.exists()) return list;

        String jdbcUrl = "jdbc:sqlite:" + dbFile.getAbsolutePath();
        try (Connection conn = DriverManager.getConnection(jdbcUrl);
             Statement stmt = conn.createStatement()) {

            String sql = "SELECT Title, ___PercentRead, ReadStatus FROM content WHERE ContentType = 6 AND ___PercentRead > 0";
            try (ResultSet rs = stmt.executeQuery(sql)) {
                while (rs.next()) {
                    String title = rs.getString("Title");
                    double pct = rs.getDouble("___PercentRead");
                    int readStatus = rs.getInt("ReadStatus");
                    boolean finished = readStatus == 2 || pct >= 99.0;
                    if (title != null && !title.isBlank()) {
                        list.add(new DeviceReadingProgress(title, pct, finished, null));
                    }
                }
            }
        } catch (Exception e) {
            log.warn("Failed to get reading progress from Kobo: {}", e.getMessage());
        }
        return list;
    }

    @Override
    public List<DeviceAnnotation> getAnnotations(File mountPoint) throws Exception {
        List<DeviceAnnotation> list = new ArrayList<>();
        File dbFile = new File(mountPoint, ".kobo" + File.separator + "KoboReader.sqlite");
        if (!dbFile.exists()) return list;

        String jdbcUrl = "jdbc:sqlite:" + dbFile.getAbsolutePath();
        try (Connection conn = DriverManager.getConnection(jdbcUrl);
             Statement stmt = conn.createStatement()) {

            String sql = "SELECT VolumeID, Text, Annotation, DateCreated FROM Bookmark WHERE Type IN ('highlight', 'note') ORDER BY DateCreated DESC";
            try (ResultSet rs = stmt.executeQuery(sql)) {
                while (rs.next()) {
                    String volId = rs.getString("VolumeID");
                    String text = rs.getString("Text");
                    String note = rs.getString("Annotation");
                    String date = rs.getString("DateCreated");

                    String title = volId != null ? new File(volId).getName().replace(".kepub.epub", "").replace(".epub", "") : "Inconnu";
                    list.add(new DeviceAnnotation(
                            title,
                            null,
                            text != null ? text : "",
                            note,
                            null,
                            date
                    ));
                }
            }
        } catch (Exception e) {
            log.warn("Failed to get annotations from Kobo: {}", e.getMessage());
        }
        return list;
    }

    @Override
    public TestResult testConfiguration(Map<String, Object> config) {
        String customStr = String.valueOf(config.getOrDefault("customMountPath", "")).trim();
        File custom = !customStr.isBlank() ? new File(customStr) : null;
        Optional<File> detected = detectDevice(custom);

        if (detected.isPresent()) {
            return TestResult.ok("Liseuse Kobo détectée avec succès sur : " + detected.get().getAbsolutePath());
        } else {
            return TestResult.error("Aucune liseuse Kobo détectée. Branchez votre Kobo en USB ou renseignez le chemin de montage.");
        }
    }

    private String getExtension(String name) {
        int dot = name.lastIndexOf('.');
        return dot > 0 ? name.substring(dot + 1) : "epub";
    }

    private String sanitizeFilename(String name) {
        if (name == null) return "Inconnu";
        return name.replaceAll("[\\\\/:*?\"<>|]", "_").trim();
    }
}
