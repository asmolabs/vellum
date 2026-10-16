package com.asmolabs.vellum.ui.components;

import com.asmolabs.vellum.domain.entity.Book;
import com.asmolabs.vellum.domain.entity.BookFormat;
import com.asmolabs.vellum.plugin.builtin.converter.MobiToEpubConverterPlugin;
import com.asmolabs.vellum.service.BookService;
import javafx.application.Platform;
import javafx.geometry.Insets;
import javafx.geometry.Pos;
import javafx.scene.Scene;
import javafx.scene.control.*;
import javafx.scene.input.KeyCode;
import javafx.scene.layout.*;
import javafx.scene.web.WebView;
import javafx.stage.Modality;
import javafx.stage.Stage;
import javafx.stage.Window;
import org.apache.pdfbox.Loader;
import org.apache.pdfbox.pdmodel.PDDocument;
import org.apache.pdfbox.rendering.PDFRenderer;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import javax.imageio.ImageIO;
import java.awt.image.BufferedImage;
import java.io.*;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.util.*;
import java.util.zip.ZipEntry;
import java.util.zip.ZipFile;

public class ReaderDialog {

    private static final Logger log = LoggerFactory.getLogger(ReaderDialog.class);

    public enum ReaderTheme {
        DARK("🌙 Sombre", "#121212", "#1e1e1e", "#d1d5db", "#f3f4f6", "#a78bfa"),
        LIGHT("☀️ Clair", "#fdfbf7", "#f3eee5", "#1f2937", "#111827", "#7c3aed"),
        SEPIA("📜 Sépia", "#f4ecd8", "#e8dcbe", "#433422", "#291e10", "#9333ea");

        final String label;
        final String bgColor;
        final String barBg;
        final String textColor;
        final String headingColor;
        final String accentColor;

        ReaderTheme(String label, String bgColor, String barBg, String textColor, String headingColor, String accentColor) {
            this.label = label;
            this.bgColor = bgColor;
            this.barBg = barBg;
            this.textColor = textColor;
            this.headingColor = headingColor;
            this.accentColor = accentColor;
        }
    }

    public static void show(Window owner, Book book, BookService bookService) {
        if (book.getFormats().isEmpty()) {
            return;
        }

        Stage stage = new Stage();
        stage.initOwner(owner);
        stage.initModality(Modality.APPLICATION_MODAL);
        stage.setTitle("Vellum Lecteur — " + book.getTitle());

        BorderPane root = new BorderPane();
        root.setStyle("-fx-background-color: #121212;");

        // Reader state
        final ReaderTheme[] currentTheme = {ReaderTheme.DARK};
        final int[] fontSize = {17};
        final boolean[] isSerif = {true};
        final int[] currentPage = {Math.max(0, book.getCurrentChapterIndex())};
        final BookFormat[] activeFormat = {book.getFormats().get(0)};

        // Controls
        HBox topBar = new HBox(12);
        topBar.setAlignment(Pos.CENTER_LEFT);
        topBar.setPadding(new Insets(8, 16, 8, 16));

        VBox titleBox = new VBox(2);
        Label titleLabel = new Label(book.getTitle());
        titleLabel.setStyle("-fx-font-weight: bold; -fx-font-size: 13px;");

        Label authorLabel = new Label(book.getAuthor());
        authorLabel.setStyle("-fx-font-size: 11px;");
        titleBox.getChildren().addAll(titleLabel, authorLabel);

        // Format switch if multiple formats available
        ComboBox<String> formatCombo = new ComboBox<>();
        if (book.getFormats().size() > 1) {
            for (BookFormat f : book.getFormats()) {
                formatCombo.getItems().add(f.getFormat().toUpperCase());
            }
            formatCombo.setValue(activeFormat[0].getFormat().toUpperCase());
            formatCombo.getStyleClass().add("combo-box");
        }

        Region spacer1 = new Region();
        HBox.setHgrow(spacer1, Priority.ALWAYS);

        // Progress indicators
        ProgressBar progressBar = new ProgressBar(0);
        progressBar.setPrefWidth(120);
        progressBar.setStyle("-fx-accent: #8b5cf6;");

        Label progressLabel = new Label("0%");
        progressLabel.setStyle("-fx-font-size: 11px; -fx-font-weight: bold;");

        Button prevBtn = new Button("◀");
        prevBtn.setTooltip(new Tooltip("Page précédente (Flèche gauche)"));
        prevBtn.setStyle("-fx-cursor: hand; -fx-padding: 4 10; -fx-font-weight: bold;");

        Button nextBtn = new Button("▶");
        nextBtn.setTooltip(new Tooltip("Page suivante (Flèche droite)"));
        nextBtn.setStyle("-fx-cursor: hand; -fx-padding: 4 10; -fx-font-weight: bold;");

        // Typography controls
        Button fontSmallerBtn = new Button("A-");
        fontSmallerBtn.setTooltip(new Tooltip("Diminuer la taille du texte"));
        fontSmallerBtn.setStyle("-fx-cursor: hand; -fx-font-size: 11px;");

        Button fontBiggerBtn = new Button("A+");
        fontBiggerBtn.setTooltip(new Tooltip("Agrandir la taille du texte"));
        fontBiggerBtn.setStyle("-fx-cursor: hand; -fx-font-size: 11px;");

        Button fontTypeBtn = new Button("Serif");
        fontTypeBtn.setTooltip(new Tooltip("Basculer entre police Serif et Sans-serif"));
        fontTypeBtn.setStyle("-fx-cursor: hand; -fx-font-size: 11px;");

        // Theme toggle
        ComboBox<ReaderTheme> themeCombo = new ComboBox<>();
        themeCombo.getItems().addAll(ReaderTheme.values());
        themeCombo.setValue(ReaderTheme.DARK);
        themeCombo.setCellFactory(lv -> new ListCell<>() {
            @Override
            protected void updateItem(ReaderTheme item, boolean empty) {
                super.updateItem(item, empty);
                setText(empty || item == null ? "" : item.label);
            }
        });
        themeCombo.setButtonCell(new ListCell<>() {
            @Override
            protected void updateItem(ReaderTheme item, boolean empty) {
                super.updateItem(item, empty);
                setText(empty || item == null ? "" : item.label);
            }
        });

        Button closeBtn = new Button("Fermer");
        closeBtn.setOnAction(e -> stage.close());

        topBar.getChildren().addAll(
                titleBox,
                (book.getFormats().size() > 1 ? formatCombo : new Region()),
                spacer1,
                progressBar, progressLabel,
                prevBtn, nextBtn,
                fontSmallerBtn, fontBiggerBtn, fontTypeBtn,
                themeCombo,
                closeBtn
        );
        root.setTop(topBar);

        // Center content area
        WebView webView = new WebView();
        root.setCenter(webView);

        // Provider initialization
        final BookContentProvider[] provider = {createContentProvider(bookService.getFileForFormat(activeFormat[0]))};

        Runnable applyThemeToBar = () -> {
            ReaderTheme t = currentTheme[0];
            topBar.setStyle("-fx-background-color: " + t.barBg + "; -fx-border-color: rgba(255,255,255,0.08); -fx-border-width: 0 0 1 0;");
            titleLabel.setStyle("-fx-font-weight: bold; -fx-font-size: 13px; -fx-text-fill: " + t.headingColor + ";");
            authorLabel.setStyle("-fx-font-size: 11px; -fx-text-fill: " + t.textColor + "; opacity: 0.8;");
            progressLabel.setStyle("-fx-font-size: 11px; -fx-font-weight: bold; -fx-text-fill: " + t.accentColor + ";");
            root.setStyle("-fx-background-color: " + t.bgColor + ";");
        };

        Runnable updateProgressAndPersist = () -> {
            int total = provider[0].getTotalPages();
            int cur = currentPage[0];
            double pct = total > 0 ? ((cur + 1.0) / total) : 1.0;
            progressBar.setProgress(pct);
            progressLabel.setText(String.format("%d / %d (%d%%)", cur + 1, total, (int) (pct * 100)));

            // Save progress in background
            Thread.ofVirtual().start(() -> {
                try {
                    bookService.updateReadingProgress(book.getId(), cur, total);
                } catch (Exception ignored) {}
            });
        };

        Runnable loadCurrentPage = () -> {
            int total = provider[0].getTotalPages();
            if (total == 0) {
                webView.getEngine().loadContent(getEmptyHtml(activeFormat[0], currentTheme[0]));
                prevBtn.setDisable(true);
                nextBtn.setDisable(true);
            } else {
                if (currentPage[0] >= total) {
                    currentPage[0] = Math.max(0, total - 1);
                }
                String pageHtml = provider[0].getPageHtml(currentPage[0], currentTheme[0], fontSize[0], isSerif[0]);
                webView.getEngine().loadContent(pageHtml);
                prevBtn.setDisable(currentPage[0] <= 0);
                nextBtn.setDisable(currentPage[0] >= total - 1);
                updateProgressAndPersist.run();
            }
        };

        // Navigation handlers
        prevBtn.setOnAction(e -> {
            if (currentPage[0] > 0) {
                currentPage[0]--;
                loadCurrentPage.run();
            }
        });

        nextBtn.setOnAction(e -> {
            if (currentPage[0] < provider[0].getTotalPages() - 1) {
                currentPage[0]++;
                loadCurrentPage.run();
            }
        });

        // Typography handlers
        fontSmallerBtn.setOnAction(e -> {
            if (fontSize[0] > 12) {
                fontSize[0] -= 2;
                loadCurrentPage.run();
            }
        });

        fontBiggerBtn.setOnAction(e -> {
            if (fontSize[0] < 32) {
                fontSize[0] += 2;
                loadCurrentPage.run();
            }
        });

        fontTypeBtn.setOnAction(e -> {
            isSerif[0] = !isSerif[0];
            fontTypeBtn.setText(isSerif[0] ? "Serif" : "Sans");
            loadCurrentPage.run();
        });

        themeCombo.setOnAction(e -> {
            currentTheme[0] = themeCombo.getValue();
            applyThemeToBar.run();
            loadCurrentPage.run();
        });

        if (book.getFormats().size() > 1) {
            formatCombo.setOnAction(e -> {
                String chosenExt = formatCombo.getValue();
                for (BookFormat f : book.getFormats()) {
                    if (f.getFormat().equalsIgnoreCase(chosenExt)) {
                        provider[0].close();
                        activeFormat[0] = f;
                        provider[0] = createContentProvider(bookService.getFileForFormat(f));
                        currentPage[0] = 0;
                        loadCurrentPage.run();
                        break;
                    }
                }
            });
        }

        applyThemeToBar.run();
        loadCurrentPage.run();

        Scene scene = new Scene(root, 1050, 780);

        // Keyboard shortcuts
        scene.setOnKeyPressed(ke -> {
            if (ke.getCode() == KeyCode.LEFT || ke.getCode() == KeyCode.PAGE_UP) {
                prevBtn.fire();
                ke.consume();
            } else if (ke.getCode() == KeyCode.RIGHT || ke.getCode() == KeyCode.PAGE_DOWN) {
                nextBtn.fire();
                ke.consume();
            } else if (ke.getCode() == KeyCode.ESCAPE) {
                stage.close();
                ke.consume();
            }
        });

        stage.setOnHidden(e -> provider[0].close());

        stage.setScene(scene);
        stage.show();
    }

    public interface BookContentProvider {
        int getTotalPages();
        String getPageHtml(int pageIndex, ReaderTheme theme, int fontSize, boolean isSerif);
        default void close() {}
    }

    private static BookContentProvider createContentProvider(File file) {
        if (file == null || !file.exists()) {
            return new EmptyContentProvider();
        }

        String name = file.getName().toLowerCase();
        try {
            if (name.endsWith(".epub")) {
                return new EpubContentProvider(file);
            } else if (name.endsWith(".pdf")) {
                return new PdfContentProvider(file);
            } else if (name.endsWith(".cbz") || name.endsWith(".cbr")) {
                return new CbzContentProvider(file);
            } else if (name.endsWith(".mobi") || name.endsWith(".azw") || name.endsWith(".azw3")) {
                return new MobiContentProvider(file);
            } else if (name.endsWith(".txt") || name.endsWith(".md")) {
                return new TextContentProvider(file);
            }
        } catch (Exception e) {
            log.warn("Erreur lors de l'initialisation du lecteur pour {}: {}", file.getName(), e.getMessage());
        }

        return new EmptyContentProvider();
    }

    // 1. EPUB Content Provider
    public static class EpubContentProvider implements BookContentProvider {
        private final List<String> chapters = new ArrayList<>();

        public EpubContentProvider(File file) {
            try (ZipFile zip = new ZipFile(file)) {
                Enumeration<? extends ZipEntry> entries = zip.entries();
                while (entries.hasMoreElements()) {
                    ZipEntry entry = entries.nextElement();
                    String name = entry.getName().toLowerCase();
                    if (!entry.isDirectory() && (name.endsWith(".html") || name.endsWith(".xhtml") || name.endsWith(".htm"))) {
                        try (InputStream is = zip.getInputStream(entry)) {
                            chapters.add(new String(is.readAllBytes(), StandardCharsets.UTF_8));
                        }
                    }
                }
            } catch (Exception e) {
                log.warn("Échec lecture chapitres EPUB: {}", e.getMessage());
            }
        }

        @Override
        public int getTotalPages() {
            return chapters.size();
        }

        @Override
        public String getPageHtml(int pageIndex, ReaderTheme theme, int fontSize, boolean isSerif) {
            if (pageIndex < 0 || pageIndex >= chapters.size()) return "";
            return wrapWithReaderTheme(chapters.get(pageIndex), theme, fontSize, isSerif);
        }
    }

    // 2. PDF Content Provider (Page-by-page rendering with PDFBox)
    public static class PdfContentProvider implements BookContentProvider {
        private PDDocument document;
        private PDFRenderer renderer;
        private int totalPages = 0;
        private final Map<Integer, String> pageCache = new HashMap<>();

        public PdfContentProvider(File file) {
            try {
                this.document = Loader.loadPDF(file);
                this.renderer = new PDFRenderer(document);
                this.totalPages = document.getNumberOfPages();
            } catch (Exception e) {
                log.warn("Impossible d'ouvrir le fichier PDF {}: {}", file.getName(), e.getMessage());
            }
        }

        @Override
        public int getTotalPages() {
            return totalPages;
        }

        @Override
        public String getPageHtml(int pageIndex, ReaderTheme theme, int fontSize, boolean isSerif) {
            if (pageIndex < 0 || pageIndex >= totalPages || document == null) return "";

            String b64Image = pageCache.computeIfAbsent(pageIndex, idx -> {
                try {
                    BufferedImage bim = renderer.renderImageWithDPI(idx, 130);
                    ByteArrayOutputStream baos = new ByteArrayOutputStream();
                    ImageIO.write(bim, "jpg", baos);
                    return Base64.getEncoder().encodeToString(baos.toByteArray());
                } catch (Exception e) {
                    log.warn("Erreur de rendu de page PDF {}: {}", idx, e.getMessage());
                    return "";
                }
            });

            return String.format("""
                <html>
                <body style="background-color: %s; margin: 0; padding: 20px; display: flex; justify-content: center; align-items: center; min-height: 90vh;">
                    <img src="data:image/jpeg;base64,%s" style="max-height: 90vh; max-width: 95vw; object-fit: contain; box-shadow: 0 4px 20px rgba(0,0,0,0.5); border-radius: 4px;"/>
                </body>
                </html>
                """, theme.bgColor, b64Image);
        }

        @Override
        public void close() {
            if (document != null) {
                try {
                    document.close();
                } catch (Exception ignored) {}
            }
        }
    }

    // 3. CBZ / CBR Comic Content Provider
    public static class CbzContentProvider implements BookContentProvider {
        private final List<byte[]> imageBytesList = new ArrayList<>();
        private final List<String> imageMimeList = new ArrayList<>();

        public CbzContentProvider(File file) {
            try (ZipFile zip = new ZipFile(file)) {
                List<? extends ZipEntry> entries = zip.stream()
                        .filter(e -> !e.isDirectory() && isImage(e.getName()))
                        .sorted(Comparator.comparing(ZipEntry::getName, String.CASE_INSENSITIVE_ORDER))
                        .toList();

                for (ZipEntry entry : entries) {
                    try (InputStream is = zip.getInputStream(entry)) {
                        imageBytesList.add(is.readAllBytes());
                        imageMimeList.add(entry.getName().toLowerCase().endsWith(".png") ? "png" : "jpeg");
                    }
                }
            } catch (Exception e) {
                log.warn("Échec lecture archive BD {}: {}", file.getName(), e.getMessage());
            }
        }

        @Override
        public int getTotalPages() {
            return imageBytesList.size();
        }

        @Override
        public String getPageHtml(int pageIndex, ReaderTheme theme, int fontSize, boolean isSerif) {
            if (pageIndex < 0 || pageIndex >= imageBytesList.size()) return "";
            byte[] bytes = imageBytesList.get(pageIndex);
            String mime = imageMimeList.get(pageIndex);
            String b64 = Base64.getEncoder().encodeToString(bytes);

            return String.format("""
                <html>
                <body style="background-color: %s; margin: 0; padding: 15px; display: flex; justify-content: center; align-items: center; min-height: 92vh;">
                    <img src="data:image/%s;base64,%s" style="max-height: 92vh; max-width: 96vw; object-fit: contain; box-shadow: 0 4px 20px rgba(0,0,0,0.7); border-radius: 4px;"/>
                </body>
                </html>
                """, theme.bgColor, mime, b64);
        }

        private boolean isImage(String name) {
            String l = name.toLowerCase();
            return l.endsWith(".jpg") || l.endsWith(".jpeg") || l.endsWith(".png") || l.endsWith(".webp");
        }
    }

    // 4. MOBI Content Provider
    public static class MobiContentProvider implements BookContentProvider {
        private final List<String> chapters = new ArrayList<>();

        public MobiContentProvider(File file) {
            try (RandomAccessFile raf = new RandomAccessFile(file, "r")) {
                raf.seek(76);
                int numRecords = raf.readUnsignedShort();
                long[] recordOffsets = new long[numRecords];
                for (int i = 0; i < numRecords; i++) {
                    recordOffsets[i] = raf.readInt() & 0xFFFFFFFFL;
                    raf.skipBytes(4);
                }

                if (numRecords > 0) {
                    raf.seek(recordOffsets[0]);
                    int compression = raf.readUnsignedShort();
                    raf.skipBytes(6);
                    int textRecordCount = raf.readUnsignedShort();

                    ByteArrayOutputStream textStream = new ByteArrayOutputStream();
                    int limit = Math.min(textRecordCount + 1, numRecords);
                    for (int i = 1; i < limit; i++) {
                        long start = recordOffsets[i];
                        long end = (i + 1 < numRecords) ? recordOffsets[i + 1] : raf.length();
                        int len = (int) (end - start);
                        if (len <= 0) continue;

                        raf.seek(start);
                        byte[] raw = new byte[len];
                        raf.readFully(raw);

                        if (compression == 2) {
                            textStream.write(MobiToEpubConverterPlugin.decompressPalmDoc(raw));
                        } else if (compression == 1) {
                            textStream.write(raw);
                        }
                    }

                    String fullHtml = textStream.toString(StandardCharsets.UTF_8);
                    if (fullHtml.isBlank()) {
                        fullHtml = "<p>Contenu non extrait.</p>";
                    }
                    chapters.add(fullHtml);
                }
            } catch (Exception e) {
                log.warn("Échec lecture MOBI {}: {}", file.getName(), e.getMessage());
            }
        }

        @Override
        public int getTotalPages() {
            return chapters.size();
        }

        @Override
        public String getPageHtml(int pageIndex, ReaderTheme theme, int fontSize, boolean isSerif) {
            if (pageIndex < 0 || pageIndex >= chapters.size()) return "";
            return wrapWithReaderTheme(chapters.get(pageIndex), theme, fontSize, isSerif);
        }
    }

    // 5. Plain Text / Markdown Content Provider
    public static class TextContentProvider implements BookContentProvider {
        private final List<String> chapters = new ArrayList<>();

        public TextContentProvider(File file) {
            try {
                String content = Files.readString(file.toPath(), StandardCharsets.UTF_8);
                String[] parts = content.split("(?m)^(?=#+ |Chapitre |Chapter )");
                if (parts.length > 0) {
                    for (String part : parts) {
                        if (!part.trim().isEmpty()) {
                            StringBuilder html = new StringBuilder();
                            for (String line : part.split("\\r?\\n")) {
                                String t = line.trim();
                                if (t.startsWith("# ")) {
                                    html.append("<h1>").append(escapeHtml(t.substring(2))).append("</h1>\n");
                                } else if (t.startsWith("## ")) {
                                    html.append("<h2>").append(escapeHtml(t.substring(3))).append("</h2>\n");
                                } else if (!t.isEmpty()) {
                                    html.append("<p>").append(escapeHtml(t)).append("</p>\n");
                                }
                            }
                            chapters.add(html.toString());
                        }
                    }
                } else {
                    chapters.add("<p>" + escapeHtml(content) + "</p>");
                }
            } catch (Exception e) {
                log.warn("Échec lecture texte {}: {}", file.getName(), e.getMessage());
            }
        }

        @Override
        public int getTotalPages() {
            return chapters.size();
        }

        @Override
        public String getPageHtml(int pageIndex, ReaderTheme theme, int fontSize, boolean isSerif) {
            if (pageIndex < 0 || pageIndex >= chapters.size()) return "";
            return wrapWithReaderTheme(chapters.get(pageIndex), theme, fontSize, isSerif);
        }

        private String escapeHtml(String text) {
            return text.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;");
        }
    }

    // Fallback Empty Provider
    public static class EmptyContentProvider implements BookContentProvider {
        @Override
        public int getTotalPages() {
            return 0;
        }

        @Override
        public String getPageHtml(int pageIndex, ReaderTheme theme, int fontSize, boolean isSerif) {
            return "";
        }
    }

    private static String wrapWithReaderTheme(String html, ReaderTheme theme, int fontSize, boolean isSerif) {
        String fontFamily = isSerif ? "Georgia, 'Times New Roman', serif" : "'Inter', -apple-system, BlinkMacSystemFont, sans-serif";

        String styleBlock = String.format("""
            <style>
                body {
                    background-color: %s !important;
                    color: %s !important;
                    font-family: %s !important;
                    font-size: %dpx !important;
                    line-height: 1.8 !important;
                    padding: 35px 50px !important;
                    max-width: 820px !important;
                    margin: 0 auto !important;
                }
                h1, h2, h3, h4, h5, h6 {
                    color: %s !important;
                    font-weight: 700 !important;
                }
                a {
                    color: %s !important;
                    text-decoration: underline !important;
                }
                img {
                    max-width: 100%% !important;
                    height: auto !important;
                    display: block !important;
                    margin: 20px auto !important;
                    border-radius: 6px !important;
                }
                p {
                    margin-bottom: 1.2em !important;
                    text-align: justify !important;
                }
            </style>
        """, theme.bgColor, theme.textColor, fontFamily, fontSize, theme.headingColor, theme.accentColor);

        if (html.contains("</head>")) {
            return html.replace("</head>", styleBlock + "</head>");
        } else {
            return "<html><head>" + styleBlock + "</head><body>" + html + "</body></html>";
        }
    }

    private static String getEmptyHtml(BookFormat format, ReaderTheme theme) {
        return String.format("""
            <html>
            <body style="background-color: %s; color: %s; font-family: sans-serif; padding: 50px; text-align: center;">
                <h2>Document non visualisable</h2>
                <p>Format : %s (%d Ko)</p>
                <p style="opacity: 0.7;">Ce format peut être ouvert directement dans votre lecteur externe préféré.</p>
            </body>
            </html>
        """, theme.bgColor, theme.textColor, format.getFormat().toUpperCase(), format.getFilesize() / 1024);
    }
}
