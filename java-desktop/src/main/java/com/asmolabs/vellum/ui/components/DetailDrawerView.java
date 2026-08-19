package com.asmolabs.vellum.ui.components;

import com.asmolabs.vellum.domain.entity.Book;
import com.asmolabs.vellum.domain.entity.BookFormat;
import com.asmolabs.vellum.domain.entity.Tag;
import com.asmolabs.vellum.service.BookService;
import com.asmolabs.vellum.service.PluginService;
import com.asmolabs.vellum.ui.events.UiEvents;
import javafx.application.Platform;
import javafx.geometry.Insets;
import javafx.geometry.Pos;
import javafx.scene.control.*;
import javafx.scene.image.Image;
import javafx.scene.image.ImageView;
import javafx.scene.layout.*;
import org.springframework.context.ApplicationEventPublisher;

import java.awt.Desktop;
import java.io.File;
import java.io.FileInputStream;
import java.util.List;

public class DetailDrawerView extends VBox {

    private final BookService bookService;
    private final PluginService pluginService;
    private final ApplicationEventPublisher eventPublisher;

    private Book currentBook;
    private ImageView coverImageView;
    private Label titleLabel;
    private Label authorLabel;
    private Label metaInfoLabel;
    private Label descLabel;
    private FlowPane tagsFlow;
    private VBox formatsBox;
    private MenuButton pluginMenuBtn;

    public DetailDrawerView(BookService bookService, PluginService pluginService, ApplicationEventPublisher eventPublisher) {
        this.bookService = bookService;
        this.pluginService = pluginService;
        this.eventPublisher = eventPublisher;

        initUI();
    }

    private void initUI() {
        this.getStyleClass().add("detail-drawer");
        this.setPrefWidth(360);
        this.setMinWidth(360);
        this.setSpacing(14);
        this.setVisible(false);
        this.setManaged(false);

        // Header: Close Button
        HBox topBar = new HBox();
        topBar.setAlignment(Pos.CENTER_RIGHT);

        Button closeBtn = new Button("✕");
        closeBtn.getStyleClass().add("icon-btn");
        closeBtn.setStyle("-fx-font-size: 14px; -fx-padding: 4 8 4 8;");
        closeBtn.setOnAction(e -> hideDrawer());
        topBar.getChildren().add(closeBtn);

        // Cover Image
        StackPane coverWrapper = new StackPane();
        coverWrapper.setPrefHeight(220);
        coverWrapper.setMaxHeight(220);
        coverWrapper.setStyle("-fx-background-color: #1e1e28; -fx-background-radius: 10; -fx-alignment: center;");

        coverImageView = new ImageView();
        coverImageView.setFitWidth(150);
        coverImageView.setFitHeight(220);
        coverImageView.setPreserveRatio(true);
        coverWrapper.getChildren().add(coverImageView);

        // Title and Author
        titleLabel = new Label();
        titleLabel.getStyleClass().add("detail-title");
        titleLabel.setWrapText(true);

        authorLabel = new Label();
        authorLabel.getStyleClass().add("detail-author");

        metaInfoLabel = new Label();
        metaInfoLabel.setStyle("-fx-font-size: 11px; -fx-text-fill: -color-text-muted;");

        // Action Buttons Row 1
        HBox row1 = new HBox(8);
        Button readBtn = new Button("▶  Lire");
        readBtn.getStyleClass().add("btn-primary");
        readBtn.setMaxWidth(Double.MAX_VALUE);
        HBox.setHgrow(readBtn, Priority.ALWAYS);
        readBtn.setOnAction(e -> {
            if (currentBook != null) {
                eventPublisher.publishEvent(new UiEvents.BookOpened(currentBook));
            }
        });

        Button editBtn = new Button("✎  Modifier");
        editBtn.getStyleClass().add("btn-secondary");
        editBtn.setMaxWidth(Double.MAX_VALUE);
        HBox.setHgrow(editBtn, Priority.ALWAYS);
        editBtn.setOnAction(e -> {
            if (currentBook != null) {
                eventPublisher.publishEvent(new UiEvents.BookEditRequested(currentBook));
            }
        });

        row1.getChildren().addAll(readBtn, editBtn);

        // Action Buttons Row 2
        HBox row2 = new HBox(8);

        pluginMenuBtn = new MenuButton("⚙ Plugins & Outils");
        pluginMenuBtn.getStyleClass().add("btn-secondary");
        pluginMenuBtn.setMaxWidth(Double.MAX_VALUE);
        HBox.setHgrow(pluginMenuBtn, Priority.ALWAYS);

        Button deleteBtn = new Button("🗑");
        deleteBtn.getStyleClass().add("btn-danger");
        deleteBtn.setTooltip(new Tooltip("Supprimer ce livre"));
        deleteBtn.setOnAction(e -> {
            if (currentBook != null) {
                eventPublisher.publishEvent(new UiEvents.BookDeleteRequested(currentBook));
            }
        });

        row2.getChildren().addAll(pluginMenuBtn, deleteBtn);

        // Description Section
        Label descHeader = new Label("DESCRIPTION");
        descHeader.getStyleClass().add("detail-section-heading");

        descLabel = new Label();
        descLabel.getStyleClass().add("detail-description");
        descLabel.setWrapText(true);

        ScrollPane descScroll = new ScrollPane(descLabel);
        descScroll.setFitToWidth(true);
        descScroll.setMaxHeight(140);
        descScroll.setStyle("-fx-background-color: transparent; -fx-background: transparent;");

        // Tags Section
        Label tagsHeader = new Label("TAGS & GENRES");
        tagsHeader.getStyleClass().add("detail-section-heading");

        tagsFlow = new FlowPane(6, 6);

        // Formats Section
        Label formatsHeader = new Label("FICHIERS FORMATS");
        formatsHeader.getStyleClass().add("detail-section-heading");

        formatsBox = new VBox(6);

        VBox contentBox = new VBox(12,
                topBar,
                coverWrapper,
                titleLabel,
                authorLabel,
                metaInfoLabel,
                row1,
                row2,
                descHeader,
                descScroll,
                tagsHeader,
                tagsFlow,
                formatsHeader,
                formatsBox
        );

        ScrollPane mainScroll = new ScrollPane(contentBox);
        mainScroll.setFitToWidth(true);
        mainScroll.setStyle("-fx-background-color: transparent; -fx-background: transparent;");
        VBox.setVgrow(mainScroll, Priority.ALWAYS);

        this.getChildren().add(mainScroll);
    }

    public void showBook(Book book) {
        this.currentBook = book;
        if (book == null) {
            hideDrawer();
            return;
        }

        Platform.runLater(() -> {
            titleLabel.setText(book.getTitle());
            authorLabel.setText(book.getAuthor());

            StringBuilder meta = new StringBuilder();
            if (book.getPublisher() != null && !book.getPublisher().isBlank()) {
                meta.append("Éditeur : ").append(book.getPublisher()).append("  •  ");
            }
            if (book.getPublishDate() != null && !book.getPublishDate().isBlank()) {
                meta.append("Date : ").append(book.getPublishDate());
            }
            metaInfoLabel.setText(meta.toString());

            String desc = book.getDescription();
            descLabel.setText((desc != null && !desc.isBlank()) ? desc : "Aucune description renseignée pour ce livre.");

            // Cover
            File coverFile = bookService.getCoverFile(book);
            if (coverFile != null && coverFile.exists()) {
                try {
                    coverImageView.setImage(new Image(new FileInputStream(coverFile)));
                } catch (Exception e) {
                    coverImageView.setImage(null);
                }
            } else {
                coverImageView.setImage(null);
            }

            // Tags
            tagsFlow.getChildren().clear();
            for (Tag tag : book.getTags()) {
                Label tLbl = new Label(tag.getName());
                tLbl.getStyleClass().add("tag-badge");
                tagsFlow.getChildren().add(tLbl);
            }

            // Formats
            formatsBox.getChildren().clear();
            for (BookFormat fmt : book.getFormats()) {
                HBox fRow = new HBox(8);
                fRow.setAlignment(Pos.CENTER_LEFT);
                fRow.setStyle("-fx-background-color: rgba(255, 255, 255, 0.03); -fx-padding: 6 10 6 10; -fx-background-radius: 6;");

                Label badge = new Label(fmt.getFormat().toUpperCase());
                badge.getStyleClass().add("format-badge");

                long kb = fmt.getFilesize() != null ? fmt.getFilesize() / 1024 : 0;
                Label sizeLbl = new Label(kb > 1024 ? String.format("%.1f Mo", kb / 1024.0) : kb + " Ko");
                sizeLbl.setStyle("-fx-font-size: 11px; -fx-text-fill: -color-text-muted;");
                HBox.setHgrow(sizeLbl, Priority.ALWAYS);

                Button openFolderBtn = new Button("📂 Ouvrir dossier");
                openFolderBtn.setStyle("-fx-background-color: transparent; -fx-border-color: -color-border; -fx-border-radius: 4; -fx-text-fill: -color-text-secondary; -fx-font-size: 10px; -fx-cursor: hand;");
                openFolderBtn.setOnAction(e -> {
                    try {
                        File bookFile = bookService.getFileForFormat(fmt);
                        if (bookFile.exists() && Desktop.isDesktopSupported()) {
                            Desktop.getDesktop().open(bookFile.getParentFile());
                        }
                    } catch (Exception ignored) {}
                });

                fRow.getChildren().addAll(badge, sizeLbl, openFolderBtn);
                formatsBox.getChildren().add(fRow);
            }

            // Plugins menu
            pluginMenuBtn.getItems().clear();
            for (PluginService.LibraryPlugin plugin : pluginService.getRegisteredPlugins()) {
                if ("transformation".equalsIgnoreCase(plugin.type())) {
                    MenuItem item = new MenuItem("⚡ " + plugin.name());
                    item.setOnAction(e -> eventPublisher.publishEvent(new UiEvents.BookPluginRequested(book, plugin.id())));
                    pluginMenuBtn.getItems().add(item);
                }
            }

            this.setVisible(true);
            this.setManaged(true);
        });
    }

    public void hideDrawer() {
        this.currentBook = null;
        this.setVisible(false);
        this.setManaged(false);
    }
}
