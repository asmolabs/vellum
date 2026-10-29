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
import javafx.scene.Scene;
import javafx.scene.control.*;
import javafx.scene.image.Image;
import javafx.scene.image.ImageView;
import javafx.scene.layout.*;
import javafx.stage.FileChooser;
import javafx.stage.Modality;
import javafx.stage.Stage;
import org.springframework.context.ApplicationEventPublisher;

import java.awt.Desktop;
import java.io.File;
import java.io.FileInputStream;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Set;
import java.util.stream.Collectors;

public class DetailDrawerView extends VBox {

    private final BookService bookService;
    private final PluginService pluginService;
    private final com.asmolabs.vellum.service.DeviceService deviceService;
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
        this(bookService, pluginService, null, eventPublisher);
    }

    public DetailDrawerView(BookService bookService, PluginService pluginService, com.asmolabs.vellum.service.DeviceService deviceService, ApplicationEventPublisher eventPublisher) {
        this.bookService = bookService;
        this.pluginService = pluginService;
        this.deviceService = deviceService;
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

        // Action Button: View Device Notes & Highlights
        Button notesBtn = new Button("📝 Notes & Citations (Liseuse)");
        notesBtn.getStyleClass().add("btn-secondary");
        notesBtn.setMaxWidth(Double.MAX_VALUE);
        notesBtn.setStyle("-fx-border-color: rgba(139, 92, 246, 0.4); -fx-text-fill: #c084fc; -fx-cursor: hand; -fx-font-size: 11px;");
        notesBtn.setOnAction(e -> {
            if (currentBook != null && deviceService != null) {
                Thread.ofVirtual().start(() -> {
                    List<com.asmolabs.vellum.plugin.device.DeviceAnnotation> notes = deviceService.getAnnotationsForBook(currentBook.getTitle(), currentBook.getAuthor());
                    Platform.runLater(() -> {
                        if (notes.isEmpty()) {
                            Alert a = new Alert(Alert.AlertType.INFORMATION, "Aucune note ou surlignement trouvé pour « " + currentBook.getTitle() + " » sur les liseuses connectées.\n(Assurez-vous que votre Kobo ou Kindle est branché en USB)", ButtonType.OK);
                            a.showAndWait();
                        } else {
                            showBookAnnotationsDialog(currentBook, notes);
                        }
                    });
                });
            } else if (deviceService == null) {
                Alert a = new Alert(Alert.AlertType.INFORMATION, "Service de synchronisation liseuse non initialisé.", ButtonType.OK);
                a.showAndWait();
            }
        });

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
                notesBtn,
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
                meta.append("Date : ").append(book.getPublishDate()).append("  •  ");
            }
            if (book.getIsbn() != null && !book.getIsbn().isBlank()) {
                meta.append("ISBN : ").append(book.getIsbn());
            }
            String metaStr = meta.toString().trim();
            if (metaStr.endsWith("•")) {
                metaStr = metaStr.substring(0, metaStr.length() - 1).trim();
            }
            metaInfoLabel.setText(metaStr);

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

                Button openFolderBtn = new Button("📂");
                openFolderBtn.setTooltip(new Tooltip("Ouvrir le dossier"));
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

                if (book.getFormats().size() > 1) {
                    Button delBtn = new Button("🗑");
                    delBtn.setTooltip(new Tooltip("Supprimer ce format"));
                    delBtn.setStyle("-fx-background-color: transparent; -fx-border-color: rgba(239, 68, 68, 0.3); -fx-border-radius: 4; -fx-text-fill: #f87171; -fx-font-size: 10px; -fx-cursor: hand;");
                    delBtn.setOnAction(e -> {
                        Alert conf = new Alert(Alert.AlertType.CONFIRMATION, "Supprimer le format " + fmt.getFormat().toUpperCase() + " de ce livre ?", ButtonType.YES, ButtonType.NO);
                        if (conf.showAndWait().orElse(ButtonType.NO) == ButtonType.YES) {
                            try {
                                Book updated = bookService.deleteFormatFromBook(book.getId(), fmt.getFormat());
                                eventPublisher.publishEvent(new UiEvents.LibraryChanged());
                                showBook(updated);
                            } catch (Exception ex) {
                                new Alert(Alert.AlertType.ERROR, "Erreur : " + ex.getMessage()).showAndWait();
                            }
                        }
                    });
                    fRow.getChildren().add(delBtn);
                }

                formatsBox.getChildren().add(fRow);
            }

            // Button: Add another format manually to this book
            Button addFmtBtn = new Button("➕ Ajouter un format...");
            addFmtBtn.setStyle("-fx-background-color: rgba(99, 102, 241, 0.15); -fx-text-fill: #a5b4fc; -fx-border-color: rgba(99, 102, 241, 0.3); -fx-border-radius: 4; -fx-font-size: 11px; -fx-cursor: hand; -fx-padding: 4 8;");
            addFmtBtn.setMaxWidth(Double.MAX_VALUE);
            addFmtBtn.setOnAction(e -> {
                FileChooser fc = new FileChooser();
                fc.setTitle("Ajouter un format à « " + book.getTitle() + " »");
                fc.getExtensionFilters().add(new FileChooser.ExtensionFilter("Livres et documents", "*.epub", "*.pdf", "*.cbz", "*.cbr", "*.mobi", "*.azw3", "*.txt", "*.md", "*.m4b", "*.mp3"));
                File chosen = fc.showOpenDialog(this.getScene().getWindow());
                if (chosen != null) {
                    try {
                        Book updated = bookService.addFormatToBook(book.getId(), chosen);
                        eventPublisher.publishEvent(new UiEvents.LibraryChanged());
                        showBook(updated);
                        Alert ok = new Alert(Alert.AlertType.INFORMATION, "Nouveau format rattaché avec succès !", ButtonType.OK);
                        ok.showAndWait();
                    } catch (Exception ex) {
                        new Alert(Alert.AlertType.ERROR, "Erreur lors de l'ajout du format : " + ex.getMessage()).showAndWait();
                    }
                }
            });
            formatsBox.getChildren().add(addFmtBtn);

            // Plugins & Conversions menu
            pluginMenuBtn.getItems().clear();

            // Direct Pure Java Conversion actions
            if (bookService.getConversionService() != null) {
                Set<String> possibleTargets = new LinkedHashSet<>();
                for (BookFormat f : book.getFormats()) {
                    possibleTargets.addAll(bookService.getConversionService().getAvailableTargetFormats(f.getFormat()));
                }
                Set<String> existingExts = book.getFormats().stream().map(f -> f.getFormat().toLowerCase()).collect(Collectors.toSet());
                possibleTargets.removeAll(existingExts);

                if (!possibleTargets.isEmpty()) {
                    Menu convertSubMenu = new Menu("🔄 Convertir & Attacher (Pur Java)");
                    for (String target : possibleTargets) {
                        MenuItem convItem = new MenuItem("Convertir en " + target.toUpperCase());
                        convItem.setOnAction(e -> {
                            Thread.ofVirtual().start(() -> {
                                try {
                                    Book updated = bookService.convertAndAttachFormat(book.getId(), target);
                                    Platform.runLater(() -> {
                                        eventPublisher.publishEvent(new UiEvents.LibraryChanged());
                                        showBook(updated);
                                        Alert ok = new Alert(Alert.AlertType.INFORMATION, "Format " + target.toUpperCase() + " converti et rattaché avec succès à la fiche !", ButtonType.OK);
                                        ok.showAndWait();
                                    });
                                } catch (Exception ex) {
                                    Platform.runLater(() -> {
                                        Alert err = new Alert(Alert.AlertType.ERROR, "Échec de conversion : " + ex.getMessage(), ButtonType.OK);
                                        err.showAndWait();
                                    });
                                }
                            });
                        });
                        convertSubMenu.getItems().add(convItem);
                    }
                    pluginMenuBtn.getItems().add(convertSubMenu);
                    pluginMenuBtn.getItems().add(new SeparatorMenuItem());
                }
            }

            // Online Metadata Enrichment
            MenuItem enrichItem = new MenuItem("✨ Enrichir métadonnées (Google Books)");
            enrichItem.setOnAction(e -> {
                Thread.ofVirtual().start(() -> {
                    try {
                        Book updated = bookService.enrichBook(book.getId());
                        Platform.runLater(() -> {
                            eventPublisher.publishEvent(new UiEvents.LibraryChanged());
                            showBook(updated);
                            Alert ok = new Alert(Alert.AlertType.INFORMATION, "Métadonnées enrichies avec succès !", ButtonType.OK);
                            ok.showAndWait();
                        });
                    } catch (Exception ex) {
                        Platform.runLater(() -> {
                            Alert err = new Alert(Alert.AlertType.ERROR, "Échec de l'enrichissement : " + ex.getMessage(), ButtonType.OK);
                            err.showAndWait();
                        });
                    }
                });
            });
            pluginMenuBtn.getItems().add(enrichItem);
            pluginMenuBtn.getItems().add(new SeparatorMenuItem());

            // Device Export Actions
            if (deviceService != null) {
                List<com.asmolabs.vellum.plugin.device.ConnectedDevice> devices = deviceService.detectConnectedDevices();
                if (!devices.isEmpty()) {
                    Menu devSubMenu = new Menu("📱 Envoyer vers la liseuse");
                    for (com.asmolabs.vellum.plugin.device.ConnectedDevice dev : devices) {
                        MenuItem devItem = new MenuItem("Envoyer vers " + dev.name());
                        devItem.setOnAction(e -> {
                            Thread.ofVirtual().start(() -> {
                                try {
                                    File exported = deviceService.exportBookToDevice(book.getId(), dev.kind(), dev.mountPoint());
                                    Platform.runLater(() -> {
                                        Alert ok = new Alert(Alert.AlertType.INFORMATION, "Livre « " + book.getTitle() + " » envoyé avec succès vers votre " + dev.name() + " !\n(" + exported.getName() + ")", ButtonType.OK);
                                        ok.showAndWait();
                                    });
                                } catch (Exception ex) {
                                    Platform.runLater(() -> {
                                        Alert err = new Alert(Alert.AlertType.ERROR, "Erreur lors du transfert : " + ex.getMessage(), ButtonType.OK);
                                        err.showAndWait();
                                    });
                                }
                            });
                        });
                        devSubMenu.getItems().add(devItem);
                    }
                    pluginMenuBtn.getItems().add(devSubMenu);
                } else {
                    MenuItem devItem = new MenuItem("📱 Connecter une liseuse...");
                    devItem.setOnAction(e -> eventPublisher.publishEvent(new UiEvents.DeviceSyncRequested()));
                    pluginMenuBtn.getItems().add(devItem);
                }
                pluginMenuBtn.getItems().add(new SeparatorMenuItem());
            }

            for (PluginService.LibraryPlugin plugin : pluginService.getRegisteredPlugins()) {
                if ("transformation".equalsIgnoreCase(plugin.type()) && plugin.enabled()) {
                    String icon = plugin.external() ? "🔌 " : "⚡ ";
                    MenuItem item = new MenuItem(icon + plugin.name());
                    item.setOnAction(e -> PluginRunDialog.show(
                            this.getScene().getWindow(),
                            book,
                            plugin.id(),
                            pluginService,
                            bookService,
                            updatedBook -> {
                                eventPublisher.publishEvent(new UiEvents.LibraryChanged());
                                showBook(updatedBook);
                            }
                    ));
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

    public Book getCurrentBook() {
        return currentBook;
    }

    private void showBookAnnotationsDialog(Book book, List<com.asmolabs.vellum.plugin.device.DeviceAnnotation> notes) {
        Stage stage = new Stage();
        if (this.getScene() != null && this.getScene().getWindow() != null) {
            stage.initOwner(this.getScene().getWindow());
        }
        stage.initModality(Modality.APPLICATION_MODAL);
        stage.setTitle("Notes & Citations — " + book.getTitle());

        VBox box = new VBox(12);
        box.getStyleClass().add("dialog-container");
        box.setPadding(new Insets(16));

        Label head = new Label("📖 " + notes.size() + " note(s) et surlignement(s) pour « " + book.getTitle() + " »");
        head.setStyle("-fx-font-weight: bold; -fx-text-fill: #c084fc; -fx-font-size: 13px;");

        ListView<com.asmolabs.vellum.plugin.device.DeviceAnnotation> lv = new ListView<>();
        lv.setCellFactory(l -> new ListCell<>() {
            @Override
            protected void updateItem(com.asmolabs.vellum.plugin.device.DeviceAnnotation item, boolean empty) {
                super.updateItem(item, empty);
                if (empty || item == null) {
                    setGraphic(null);
                } else {
                    VBox card = new VBox(4);
                    card.setStyle("-fx-background-color: rgba(255,255,255,0.03); -fx-padding: 8 10; -fx-background-radius: 6;");
                    Label quote = new Label("« " + (item.text() != null && !item.text().isBlank() ? item.text() : item.note()) + " »");
                    quote.setWrapText(true);
                    quote.setStyle("-fx-font-style: italic; -fx-text-fill: #e5e7eb; -fx-font-size: 12px;");
                    HBox sub = new HBox(8);
                    if (item.location() != null) {
                        Label loc = new Label("📍 " + item.location());
                        loc.setStyle("-fx-font-size: 10px; -fx-text-fill: -color-text-muted;");
                        sub.getChildren().add(loc);
                    }
                    if (item.createdAt() != null) {
                        Label dt = new Label("🕒 " + item.createdAt());
                        dt.setStyle("-fx-font-size: 10px; -fx-text-fill: -color-text-muted;");
                        sub.getChildren().add(dt);
                    }
                    card.getChildren().addAll(quote, sub);
                    setGraphic(card);
                }
            }
        });
        lv.getItems().addAll(notes);
        VBox.setVgrow(lv, Priority.ALWAYS);

        Button copyBtn = new Button("📋 Copier tout");
        copyBtn.getStyleClass().add("btn-secondary");
        copyBtn.setOnAction(e -> {
            StringBuilder sb = new StringBuilder("# Citations de « " + book.getTitle() + " »\n\n");
            for (com.asmolabs.vellum.plugin.device.DeviceAnnotation a : notes) {
                if (a.location() != null) sb.append("📍 *").append(a.location()).append("*\n");
                sb.append("> ").append(a.text() != null && !a.text().isBlank() ? a.text() : a.note()).append("\n\n");
            }
            javafx.scene.input.ClipboardContent cc = new javafx.scene.input.ClipboardContent();
            cc.putString(sb.toString());
            javafx.scene.input.Clipboard.getSystemClipboard().setContent(cc);
            Alert info = new Alert(Alert.AlertType.INFORMATION, "Citations copiées dans le presse-papier !", ButtonType.OK);
            info.showAndWait();
        });

        Button closeBtn = new Button("Fermer");
        closeBtn.getStyleClass().add("btn-primary");
        closeBtn.setOnAction(e -> stage.close());

        HBox actions = new HBox(10, copyBtn, closeBtn);
        actions.setAlignment(Pos.CENTER_RIGHT);

        box.getChildren().addAll(head, lv, actions);

        Scene sc = new Scene(box, 560, 420);
        sc.getStylesheets().add(DetailDrawerView.class.getResource("/styles/theme.css").toExternalForm());
        stage.setScene(sc);
        stage.show();
    }
}
