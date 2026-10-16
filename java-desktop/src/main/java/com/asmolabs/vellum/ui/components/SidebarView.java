package com.asmolabs.vellum.ui.components;

import com.asmolabs.vellum.config.AppProperties;
import com.asmolabs.vellum.service.BookService;
import com.asmolabs.vellum.ui.events.UiEvents;
import javafx.application.Platform;
import javafx.geometry.Insets;
import javafx.geometry.Pos;
import javafx.scene.control.*;
import javafx.scene.layout.*;
import javafx.stage.DirectoryChooser;
import javafx.stage.FileChooser;
import org.springframework.context.ApplicationEventPublisher;

import java.io.File;
import java.util.List;
import java.util.Map;

public class SidebarView extends VBox {

    private final BookService bookService;
    private final AppProperties appProperties;
    private final ApplicationEventPublisher eventPublisher;

    private Label totalCountBadge;
    private ListView<TagItem> tagsListView;
    private Label pathValueLabel;
    private String selectedTag = "Tous";

    public record TagItem(String name, long count) {
        @Override
        public String toString() {
            return name;
        }
    }

    public SidebarView(BookService bookService, AppProperties appProperties, ApplicationEventPublisher eventPublisher) {
        this.bookService = bookService;
        this.appProperties = appProperties;
        this.eventPublisher = eventPublisher;

        initUI();
        refresh();
    }

    private void initUI() {
        this.getStyleClass().add("sidebar");
        this.setPrefWidth(280);
        this.setMinWidth(280);
        this.setSpacing(18);

        // 1. Logo Section
        HBox logoBox = new HBox(10);
        logoBox.setAlignment(Pos.CENTER_LEFT);

        Label logoIcon = new Label("✦");
        logoIcon.getStyleClass().add("logo-icon");

        Label logoText = new Label("Vellum");
        logoText.getStyleClass().add("logo-label");

        logoBox.getChildren().addAll(logoIcon, logoText);

        // 2. Import Button
        Button importBtn = new Button("＋  Importer un livre");
        importBtn.getStyleClass().add("btn-primary");
        importBtn.setMaxWidth(Double.MAX_VALUE);
        importBtn.setOnAction(e -> handleImport());

        // 3. Library Path Box
        VBox pathBox = new VBox(4);
        pathBox.getStyleClass().add("path-box");

        Label pathTitle = new Label("DOSSIER BIBLIOTHÈQUE");
        pathTitle.setStyle("-fx-font-size: 10px; -fx-font-weight: bold; -fx-text-fill: -color-text-muted;");

        pathValueLabel = new Label(shortenPath(appProperties.getLibraryPath()));
        pathValueLabel.getStyleClass().add("path-text");

        Button changePathBtn = new Button("Changer d'emplacement");
        changePathBtn.getStyleClass().add("btn-secondary");
        changePathBtn.setMaxWidth(Double.MAX_VALUE);
        changePathBtn.setStyle("-fx-font-size: 11px; -fx-padding: 4 8 4 8;");
        changePathBtn.setOnAction(e -> handleChangeLibraryPath());

        pathBox.getChildren().addAll(pathTitle, pathValueLabel, changePathBtn);

        // 4. Navigation Section ("Tous les livres" & "Statistiques")
        Label navTitle = new Label("BIBLIOTHÈQUE");
        navTitle.getStyleClass().add("sidebar-section-title");

        HBox allBooksItem = new HBox(8);
        allBooksItem.setAlignment(Pos.CENTER_LEFT);
        allBooksItem.setStyle("-fx-background-color: rgba(139, 92, 246, 0.15); -fx-padding: 8 10 8 10; -fx-background-radius: 6; -fx-cursor: hand;");

        Label allBooksLabel = new Label("📚  Tous les livres");
        allBooksLabel.setStyle("-fx-text-fill: #ffffff; -fx-font-weight: bold; -fx-font-size: 13px;");
        HBox.setHgrow(allBooksLabel, Priority.ALWAYS);

        totalCountBadge = new Label("0");
        totalCountBadge.getStyleClass().add("badge-count");

        allBooksItem.getChildren().addAll(allBooksLabel, totalCountBadge);
        allBooksItem.setOnMouseClicked(e -> {
            selectedTag = "Tous";
            tagsListView.getSelectionModel().clearSelection();
            eventPublisher.publishEvent(new UiEvents.FilterChanged(null, "Tous"));
        });

        HBox statsItem = new HBox(8);
        statsItem.setAlignment(Pos.CENTER_LEFT);
        statsItem.setStyle("-fx-background-color: rgba(255, 255, 255, 0.03); -fx-padding: 8 10 8 10; -fx-background-radius: 6; -fx-cursor: hand;");

        Label statsLabel = new Label("📊  Statistiques & Lectures");
        statsLabel.setStyle("-fx-text-fill: -color-text-secondary; -fx-font-size: 13px;");
        statsItem.getChildren().add(statsLabel);
        statsItem.setOnMouseClicked(e -> eventPublisher.publishEvent(new UiEvents.OpenStatsRequested()));

        // 5. Tags List Section
        Label tagsTitle = new Label("GENRES & TAGS");
        tagsTitle.getStyleClass().add("sidebar-section-title");

        tagsListView = new ListView<>();
        tagsListView.getStyleClass().add("nav-list-view");
        VBox.setVgrow(tagsListView, Priority.ALWAYS);

        tagsListView.setCellFactory(lv -> new ListCell<>() {
            @Override
            protected void updateItem(TagItem item, boolean empty) {
                super.updateItem(item, empty);
                if (empty || item == null) {
                    setText(null);
                    setGraphic(null);
                } else {
                    HBox box = new HBox(8);
                    box.setAlignment(Pos.CENTER_LEFT);
                    Label nameLbl = new Label("🏷 " + item.name());
                    nameLbl.setStyle("-fx-text-fill: -color-text-secondary; -fx-font-size: 12px;");
                    HBox.setHgrow(nameLbl, Priority.ALWAYS);

                    Label countLbl = new Label(String.valueOf(item.count()));
                    countLbl.getStyleClass().add("badge-count");

                    box.getChildren().addAll(nameLbl, countLbl);
                    setGraphic(box);
                }
            }
        });

        tagsListView.getSelectionModel().selectedItemProperty().addListener((obs, oldV, newV) -> {
            if (newV != null) {
                selectedTag = newV.name();
                eventPublisher.publishEvent(new UiEvents.FilterChanged(null, selectedTag));
            }
        });

        this.getChildren().addAll(logoBox, importBtn, pathBox, navTitle, allBooksItem, statsItem, tagsTitle, tagsListView);
    }

    public void refresh() {
        Platform.runLater(() -> {
            pathValueLabel.setText(shortenPath(appProperties.getLibraryPath()));
            long total = bookService.getTotalBooksCount();
            totalCountBadge.setText(String.valueOf(total));

            Map<String, Long> tagCounts = bookService.getTagsWithCounts();
            tagsListView.getItems().clear();
            for (Map.Entry<String, Long> entry : tagCounts.entrySet()) {
                tagsListView.getItems().add(new TagItem(entry.getKey(), entry.getValue()));
            }
        });
    }

    private void handleImport() {
        FileChooser fc = new FileChooser();
        fc.setTitle("Importer un livre ou document dans Vellum");
        fc.getExtensionFilters().addAll(
                new FileChooser.ExtensionFilter("Tous les formats supportés (*.epub, *.pdf, *.cbz, *.cbr, *.mobi)", "*.epub", "*.pdf", "*.cbz", "*.cbr", "*.mobi", "*.azw3"),
                new FileChooser.ExtensionFilter("Ebooks EPUB (*.epub)", "*.epub"),
                new FileChooser.ExtensionFilter("Documents PDF (*.pdf)", "*.pdf"),
                new FileChooser.ExtensionFilter("Bandes Dessinées & Mangas (*.cbz, *.cbr)", "*.cbz", "*.cbr"),
                new FileChooser.ExtensionFilter("Tous les fichiers", "*.*")
        );

        List<File> files = fc.showOpenMultipleDialog(this.getScene().getWindow());
        if (files != null && !files.isEmpty()) {
            Thread.ofVirtual().start(() -> {
                for (File file : files) {
                    try {
                        bookService.importBook(file);
                    } catch (Exception ex) {
                        Platform.runLater(() -> {
                            Alert a = new Alert(Alert.AlertType.ERROR, "Erreur lors de l'import de " + file.getName() + " : " + ex.getMessage(), ButtonType.OK);
                            a.showAndWait();
                        });
                    }
                }
                eventPublisher.publishEvent(new UiEvents.LibraryChanged());
            });
        }
    }

    private void handleChangeLibraryPath() {
        DirectoryChooser dc = new DirectoryChooser();
        dc.setTitle("Sélectionner un nouveau répertoire de bibliothèque");
        File dir = dc.showDialog(this.getScene().getWindow());
        if (dir != null) {
            appProperties.setLibraryPath(dir.getAbsolutePath());
            bookService.ensureLibraryDirectory();
            refresh();
            eventPublisher.publishEvent(new UiEvents.LibraryChanged());
        }
    }

    private String shortenPath(String path) {
        if (path == null) return "";
        if (path.length() > 32) {
            return "..." + path.substring(path.length() - 29);
        }
        return path;
    }
}
