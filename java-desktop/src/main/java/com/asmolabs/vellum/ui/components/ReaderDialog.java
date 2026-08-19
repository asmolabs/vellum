package com.asmolabs.vellum.ui.components;

import com.asmolabs.vellum.domain.entity.Book;
import com.asmolabs.vellum.domain.entity.BookFormat;
import com.asmolabs.vellum.service.BookService;
import javafx.geometry.Insets;
import javafx.geometry.Pos;
import javafx.scene.Scene;
import javafx.scene.control.Button;
import javafx.scene.control.Label;
import javafx.scene.layout.BorderPane;
import javafx.scene.layout.HBox;
import javafx.scene.layout.Priority;
import javafx.scene.web.WebView;
import javafx.stage.Modality;
import javafx.stage.Stage;
import javafx.stage.Window;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.io.File;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Enumeration;
import java.util.List;
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
        stage.setTitle("Vellum Lecteur - " + book.getTitle());

        BorderPane root = new BorderPane();
        root.setStyle("-fx-background-color: #121212;");

        // Top Navigation Bar
        HBox topBar = new HBox(12);
        topBar.setAlignment(Pos.CENTER_LEFT);
        topBar.setPadding(new Insets(12, 16, 12, 16));
        topBar.setStyle("-fx-background-color: #1a1a1a; -fx-border-color: #2a2a2a; -fx-border-width: 0 0 1 0;");

        Label titleLabel = new Label(book.getTitle() + " — " + book.getAuthor());
        titleLabel.setStyle("-fx-font-weight: bold; -fx-text-fill: #e0e0e0; -fx-font-size: 14px;");

        HBox spacer = new HBox();
        HBox.setHgrow(spacer, Priority.ALWAYS);

        Button prevBtn = new Button("◀ Chapitre précédent");
        prevBtn.getStyleClass().add("btn-secondary");

        Button nextBtn = new Button("Chapitre suivant ▶");
        nextBtn.getStyleClass().add("btn-secondary");

        Button closeBtn = new Button("Fermer");
        closeBtn.getStyleClass().add("btn-secondary");
        closeBtn.setOnAction(e -> stage.close());

        topBar.getChildren().addAll(titleLabel, spacer, prevBtn, nextBtn, closeBtn);
        root.setTop(topBar);

        // WebView for EPUB content
        WebView webView = new WebView();
        webView.setStyle("-fx-background-color: #121212;");
        root.setCenter(webView);

        // Extract chapters
        List<String> chapters = extractHtmlChapters(bookFile);
        final int[] currentChapter = {0};

        Runnable loadCurrent = () -> {
            if (chapters.isEmpty()) {
                webView.getEngine().loadContent("""
                    <html>
                    <body style="background-color: #121212; color: #d1d5db; font-family: sans-serif; padding: 40px; text-align: center;">
                        <h2>Prévisualisation du livre</h2>
                        <p>Format: %s (%d Ko)</p>
                        <p style="color: #9ca3af;">Le contenu formaté peut être lu dans votre visionneuse d'ebooks préférée.</p>
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

        Scene scene = new Scene(root, 960, 720);
        scene.getStylesheets().add(ReaderDialog.class.getResource("/styles/theme.css").toExternalForm());
        stage.setScene(scene);
        stage.show();
    }

    private static List<String> extractHtmlChapters(File file) {
        List<String> chapters = new ArrayList<>();
        if (!file.getName().toLowerCase().endsWith(".epub")) {
            return chapters;
        }

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
            log.warn("Could not extract chapters from {}: {}", file.getName(), e.getMessage());
        }
        return chapters;
    }

    private static String injectDarkReaderTheme(String html) {
        String darkCss = """
            <style>
                body {
                    background-color: #121212 !important;
                    color: #d1d5db !important;
                    font-family: 'Inter', -apple-system, sans-serif !important;
                    font-size: 17px !important;
                    line-height: 1.7 !important;
                    padding: 40px 60px !important;
                    max-width: 800px !important;
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
