package com.asmolabs.vellum.ui.components;

import com.asmolabs.vellum.domain.entity.Book;
import com.asmolabs.vellum.domain.entity.BookFormat;
import com.asmolabs.vellum.service.BookService;
import javafx.geometry.Insets;
import javafx.geometry.Pos;
import javafx.scene.Scene;
import javafx.scene.control.Button;
import javafx.scene.control.Label;
import javafx.scene.control.ProgressBar;
import javafx.scene.layout.BorderPane;
import javafx.scene.layout.HBox;
import javafx.scene.layout.Priority;
import javafx.scene.layout.VBox;
import javafx.scene.web.WebView;
import javafx.stage.Modality;
import javafx.stage.Stage;
import javafx.stage.Window;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.io.File;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.util.*;
import java.util.zip.ZipEntry;
import java.util.zip.ZipFile;

public class ReaderDialog {

    private static final Logger log = LoggerFactory.getLogger(ReaderDialog.class);

    public static void show(Window owner, Book book, BookService bookService) {
        if (book.getFormats().isEmpty()) {
            return;
        }

        BookFormat fmt = book.getFormats().get(0);
        File bookFile = bookService.getFileForFormat(fmt);
        if (!bookFile.exists()) {
            return;
        }

        Stage stage = new Stage();
        stage.initOwner(owner);
        stage.initModality(Modality.APPLICATION_MODAL);
        stage.setTitle("Vellum Lecteur — " + book.getTitle());

        BorderPane root = new BorderPane();
        root.setStyle("-fx-background-color: #121212;");

        // Top Navigation Bar
        HBox topBar = new HBox(14);
        topBar.setAlignment(Pos.CENTER_LEFT);
        topBar.setPadding(new Insets(10, 18, 10, 18));
        topBar.setStyle("-fx-background-color: #1a1a1a; -fx-border-color: #2a2a2a; -fx-border-width: 0 0 1 0;");

        VBox titleBox = new VBox(2);
        Label titleLabel = new Label(book.getTitle());
        titleLabel.setStyle("-fx-font-weight: bold; -fx-text-fill: #e0e0e0; -fx-font-size: 13px;");

        Label authorLabel = new Label(book.getAuthor());
        authorLabel.setStyle("-fx-text-fill: #9ca3af; -fx-font-size: 11px;");
        titleBox.getChildren().addAll(titleLabel, authorLabel);

        HBox spacer = new HBox();
        HBox.setHgrow(spacer, Priority.ALWAYS);

        // Progress elements
        ProgressBar progressBar = new ProgressBar(0);
        progressBar.setPrefWidth(140);
        progressBar.setStyle("-fx-accent: #8b5cf6;");

        Label progressLabel = new Label("0%");
        progressLabel.setStyle("-fx-text-fill: #c084fc; -fx-font-size: 11px; -fx-font-weight: bold;");

        Button prevBtn = new Button("◀ Précédent");
        prevBtn.getStyleClass().add("btn-secondary");

        Button nextBtn = new Button("Suivant ▶");
        nextBtn.getStyleClass().add("btn-secondary");

        Button closeBtn = new Button("Fermer");
        closeBtn.getStyleClass().add("btn-secondary");
        closeBtn.setOnAction(e -> stage.close());

        topBar.getChildren().addAll(titleBox, spacer, progressBar, progressLabel, prevBtn, nextBtn, closeBtn);
        root.setTop(topBar);

        // Center: WebView for content
        WebView webView = new WebView();
        webView.setStyle("-fx-background-color: #121212;");
        root.setCenter(webView);

        // Extract chapters / pages
        List<String> chapters = extractChapters(bookFile);
        int initialChapter = Math.min(Math.max(0, book.getCurrentChapterIndex()), Math.max(0, chapters.size() - 1));
        final int[] currentChapter = {initialChapter};

        Runnable updateProgressAndPersist = () -> {
            int total = chapters.size();
            int current = currentChapter[0];
            double pct = total > 0 ? ((current + 1.0) / total) : 1.0;
            progressBar.setProgress(pct);
            progressLabel.setText(String.format("Chapitre %d / %d (%d%%)", current + 1, total, (int)(pct * 100)));

            // Save in SQLite in background
            Thread.ofVirtual().start(() -> {
                try {
                    bookService.updateReadingProgress(book.getId(), current, total);
                } catch (Exception e) {
                    log.warn("Could not save reading progress: {}", e.getMessage());
                }
            });
        };

        Runnable loadCurrent = () -> {
            if (chapters.isEmpty()) {
                webView.getEngine().loadContent("""
                    <html>
                    <body style="background-color: #121212; color: #d1d5db; font-family: sans-serif; padding: 40px; text-align: center;">
                        <h2>Prévisualisation du document</h2>
                        <p>Format: %s (%d Ko)</p>
                        <p style="color: #9ca3af;">Le contenu formaté peut être ouvert dans votre lecteur favori.</p>
                    </body>
                    </html>
                """.formatted(fmt.getFormat().toUpperCase(), fmt.getFilesize() / 1024));
                prevBtn.setDisable(true);
                nextBtn.setDisable(true);
            } else {
                String content = chapters.get(currentChapter[0]);
                String styledHtml = injectDarkReaderTheme(content);
                webView.getEngine().loadContent(styledHtml);
                prevBtn.setDisable(currentChapter[0] <= 0);
                nextBtn.setDisable(currentChapter[0] >= chapters.size() - 1);
                updateProgressAndPersist.run();
            }
        };

        prevBtn.setOnAction(e -> {
            if (currentChapter[0] > 0) {
                currentChapter[0]--;
                loadCurrent.run();
            }
        });

        nextBtn.setOnAction(e -> {
            if (currentChapter[0] < chapters.size() - 1) {
                currentChapter[0]++;
                loadCurrent.run();
            }
        });

        loadCurrent.run();

        Scene scene = new Scene(root, 1000, 750);
        scene.getStylesheets().add(ReaderDialog.class.getResource("/styles/theme.css").toExternalForm());
        stage.setScene(scene);
        stage.show();
    }

    private static List<String> extractChapters(File file) {
        String name = file.getName().toLowerCase();
        if (name.endsWith(".epub")) {
            return extractEpubChapters(file);
        } else if (name.endsWith(".cbz") || name.endsWith(".cbr")) {
            return extractCbzPages(file);
        }
        return new ArrayList<>();
    }

    private static List<String> extractEpubChapters(File file) {
        List<String> chapters = new ArrayList<>();
        try (ZipFile zip = new ZipFile(file)) {
            Enumeration<? extends ZipEntry> entries = zip.entries();
            while (entries.hasMoreElements()) {
                ZipEntry entry = entries.nextElement();
                String name = entry.getName().toLowerCase();
                if (name.endsWith(".html") || name.endsWith(".xhtml") || name.endsWith(".htm")) {
                    try (InputStream is = zip.getInputStream(entry)) {
                        String html = new String(is.readAllBytes(), StandardCharsets.UTF_8);
                        chapters.add(html);
                    }
                }
            }
        } catch (Exception e) {
            log.warn("Could not extract EPUB chapters: {}", e.getMessage());
        }
        return chapters;
    }

    private static List<String> extractCbzPages(File file) {
        List<String> pages = new ArrayList<>();
        try (ZipFile zip = new ZipFile(file)) {
            List<ZipEntry> imgEntries = new ArrayList<>();
            Enumeration<? extends ZipEntry> entries = zip.entries();
            while (entries.hasMoreElements()) {
                ZipEntry entry = entries.nextElement();
                String name = entry.getName().toLowerCase();
                if (name.endsWith(".jpg") || name.endsWith(".jpeg") || name.endsWith(".png") || name.endsWith(".webp")) {
                    imgEntries.add(entry);
                }
            }
            imgEntries.sort(Comparator.comparing(ZipEntry::getName));

            for (ZipEntry imgEntry : imgEntries) {
                try (InputStream is = zip.getInputStream(imgEntry)) {
                    byte[] bytes = is.readAllBytes();
                    String b64 = Base64.getEncoder().encodeToString(bytes);
                    String ext = imgEntry.getName().toLowerCase().endsWith(".png") ? "png" : "jpeg";
                    String html = """
                        <div style="text-align:center; padding: 10px;">
                            <img src="data:image/%s;base64,%s" style="max-height: 92vh; max-width: 96vw; object-fit: contain; margin: 0 auto; box-shadow: 0 4px 20px rgba(0,0,0,0.8); border-radius: 4px;"/>
                        </div>
                    """.formatted(ext, b64);
                    pages.add(html);
                }
            }
        } catch (Exception e) {
            log.warn("Could not extract CBZ pages: {}", e.getMessage());
        }
        return pages;
    }

    private static String injectDarkReaderTheme(String html) {
        String darkCss = """
            <style>
                body {
                    background-color: #121212 !important;
                    color: #d1d5db !important;
                    font-family: 'Inter', -apple-system, BlinkMacSystemFont, sans-serif !important;
                    font-size: 17px !important;
                    line-height: 1.75 !important;
                    padding: 40px 60px !important;
                    max-width: 820px !important;
                    margin: 0 auto !important;
                }
                h1, h2, h3, h4, h5, h6 {
                    color: #f3f4f6 !important;
                    font-weight: 600 !important;
                }
                a {
                    color: #a78bfa !important;
                    text-decoration: underline !important;
                }
                img {
                    max-width: 100% !important;
                    height: auto !important;
                    display: block !important;
                    margin: 20px auto !important;
                    border-radius: 8px !important;
                }
                p {
                    margin-bottom: 1.2em !important;
                }
            </style>
        """;

        if (html.contains("</head>")) {
            return html.replace("</head>", darkCss + "</head>");
        } else {
            return darkCss + html;
        }
    }
}
