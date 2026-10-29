package com.asmolabs.vellum.ui.components;

import com.asmolabs.vellum.domain.entity.Book;
import com.asmolabs.vellum.plugin.device.ConnectedDevice;
import com.asmolabs.vellum.plugin.device.DeviceAnnotation;
import com.asmolabs.vellum.plugin.device.DeviceBookInfo;
import com.asmolabs.vellum.plugin.device.DeviceReadingProgress;
import com.asmolabs.vellum.service.DeviceService;
import com.asmolabs.vellum.ui.events.UiEvents;
import javafx.application.Platform;
import javafx.beans.property.SimpleStringProperty;
import javafx.collections.FXCollections;
import javafx.geometry.Insets;
import javafx.geometry.Pos;
import javafx.scene.Scene;
import javafx.scene.control.*;
import javafx.scene.input.Clipboard;
import javafx.scene.input.ClipboardContent;
import javafx.scene.layout.*;
import javafx.stage.FileChooser;
import javafx.stage.Modality;
import javafx.stage.Stage;
import javafx.stage.Window;
import org.springframework.context.ApplicationEventPublisher;

import java.io.File;
import java.nio.file.Files;
import java.util.List;

public class DeviceSyncDialog {

    public static void show(Window owner, DeviceService deviceService, ApplicationEventPublisher eventPublisher) {
        Stage stage = new Stage();
        stage.initOwner(owner);
        stage.initModality(Modality.APPLICATION_MODAL);
        stage.setTitle("Synchronisation Liseuse (Kobo & Kindle)");

        BorderPane root = new BorderPane();
        root.getStyleClass().add("dialog-container");
        root.setPadding(new Insets(20));

        // Top: Device selection & Detection Status
        VBox topBox = new VBox(12);
        topBox.setPadding(new Insets(0, 0, 16, 0));

        HBox devHeader = new HBox(12);
        devHeader.setAlignment(Pos.CENTER_LEFT);

        Label iconLbl = new Label("📱");
        iconLbl.setStyle("-fx-font-size: 24px;");

        VBox titleBox = new VBox(2);
        Label titleLbl = new Label("Liseuse Connectée");
        titleLbl.setStyle("-fx-font-size: 16px; -fx-font-weight: bold; -fx-text-fill: -color-text;");
        Label subtitleLbl = new Label("Gérez vos livres, progressez dans vos lectures et sauvegardez vos surlignages.");
        subtitleLbl.setStyle("-fx-font-size: 11px; -fx-text-fill: -color-text-muted;");
        titleBox.getChildren().addAll(titleLbl, subtitleLbl);

        Region spacer = new Region();
        HBox.setHgrow(spacer, Priority.ALWAYS);

        ComboBox<ConnectedDevice> deviceCombo = new ComboBox<>();
        deviceCombo.getStyleClass().add("combo-box");
        deviceCombo.setPromptText("Aucune liseuse détectée");
        deviceCombo.setCellFactory(lv -> new ListCell<>() {
            @Override
            protected void updateItem(ConnectedDevice item, boolean empty) {
                super.updateItem(item, empty);
                if (empty || item == null) {
                    setText(null);
                } else {
                    setText("📱 " + item.name() + " (" + item.bookCount() + " livres)");
                }
            }
        });
        deviceCombo.setButtonCell(new ListCell<>() {
            @Override
            protected void updateItem(ConnectedDevice item, boolean empty) {
                super.updateItem(item, empty);
                if (empty || item == null) {
                    setText("Recherche de liseuse...");
                } else {
                    setText("📱 " + item.name() + " (" + item.bookCount() + " livres)");
                }
            }
        });

        Button refreshBtn = new Button("🔄 Actualiser");
        refreshBtn.getStyleClass().add("btn-secondary");

        devHeader.getChildren().addAll(iconLbl, titleBox, spacer, deviceCombo, refreshBtn);
        topBox.getChildren().add(devHeader);

        // Center: TabPane for Books, Reading Progress, Annotations
        TabPane tabPane = new TabPane();
        tabPane.getStyleClass().add("plugin-tab-pane");

        // Tab 1: Books on device
        Tab booksTab = new Tab("📚 Livres sur l'appareil");
        booksTab.setClosable(false);

        TableView<DeviceBookInfo> booksTable = new TableView<>();
        booksTable.getStyleClass().add("table-view");

        TableColumn<DeviceBookInfo, String> colTitle = new TableColumn<>("Titre");
        colTitle.setCellValueFactory(c -> new SimpleStringProperty(c.getValue().title()));
        colTitle.setPrefWidth(260);

        TableColumn<DeviceBookInfo, String> colAuthor = new TableColumn<>("Auteur");
        colAuthor.setCellValueFactory(c -> new SimpleStringProperty(c.getValue().author()));
        colAuthor.setPrefWidth(160);

        TableColumn<DeviceBookInfo, String> colFormat = new TableColumn<>("Format");
        colFormat.setCellValueFactory(c -> new SimpleStringProperty(c.getValue().format().toUpperCase()));
        colFormat.setPrefWidth(80);

        TableColumn<DeviceBookInfo, String> colSize = new TableColumn<>("Taille");
        colSize.setCellValueFactory(c -> {
            long kb = c.getValue().fileSize() / 1024;
            return new SimpleStringProperty(kb > 1024 ? String.format("%.1f Mo", kb / 1024.0) : kb + " Ko");
        });
        colSize.setPrefWidth(90);

        TableColumn<DeviceBookInfo, String> colProg = new TableColumn<>("Avancement");
        colProg.setCellValueFactory(c -> {
            Double p = c.getValue().progressPercent();
            return new SimpleStringProperty(p != null && p > 0 ? String.format("%.0f %%", p) : "—");
        });
        colProg.setPrefWidth(100);

        booksTable.getColumns().addAll(colTitle, colAuthor, colFormat, colSize, colProg);

        Button importAllBtn = new Button("📥 Importer les nouveaux livres dans Vellum");
        importAllBtn.getStyleClass().add("btn-primary");

        VBox booksBox = new VBox(10, booksTable, importAllBtn);
        booksBox.setPadding(new Insets(12));
        VBox.setVgrow(booksTable, Priority.ALWAYS);
        booksTab.setContent(booksBox);

        // Tab 2: Reading progress sync
        Tab progressTab = new Tab("🔄 Avancement de lecture");
        progressTab.setClosable(false);

        VBox progBox = new VBox(14);
        progBox.setPadding(new Insets(16));

        Label progInfo = new Label("Synchronisez automatiquement vos pourcentages de lecture depuis votre liseuse vers Vellum.");
        progInfo.setStyle("-fx-text-fill: -color-text-secondary; -fx-font-size: 13px;");

        Button syncProgressBtn = new Button("⚡ Synchroniser l'avancement maintenant");
        syncProgressBtn.getStyleClass().add("btn-primary");

        ListView<String> progressList = new ListView<>();
        progressList.getStyleClass().add("list-view");
        VBox.setVgrow(progressList, Priority.ALWAYS);

        progBox.getChildren().addAll(progInfo, syncProgressBtn, progressList);
        progressTab.setContent(progBox);

        // Tab 3: Annotations & Highlights
        Tab notesTab = new Tab("📝 Citations & Surlignages");
        notesTab.setClosable(false);

        VBox notesBox = new VBox(12);
        notesBox.setPadding(new Insets(12));

        ListView<DeviceAnnotation> notesList = new ListView<>();
        notesList.getStyleClass().add("list-view");
        VBox.setVgrow(notesList, Priority.ALWAYS);

        notesList.setCellFactory(lv -> new ListCell<>() {
            @Override
            protected void updateItem(DeviceAnnotation item, boolean empty) {
                super.updateItem(item, empty);
                if (empty || item == null) {
                    setGraphic(null);
                } else {
                    VBox card = new VBox(4);
                    card.setStyle("-fx-background-color: rgba(255,255,255,0.03); -fx-padding: 8 12; -fx-background-radius: 6;");

                    Label tLbl = new Label("📖 " + item.bookTitle() + (item.bookAuthor() != null ? " — " + item.bookAuthor() : ""));
                    tLbl.setStyle("-fx-font-weight: bold; -fx-text-fill: #c084fc; -fx-font-size: 12px;");

                    Label bodyLbl = new Label("« " + (item.text() != null && !item.text().isBlank() ? item.text() : item.note()) + " »");
                    bodyLbl.setWrapText(true);
                    bodyLbl.setStyle("-fx-text-fill: #e5e7eb; -fx-font-style: italic; -fx-font-size: 12px;");

                    HBox sub = new HBox(12);
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

                    card.getChildren().addAll(tLbl, bodyLbl, sub);
                    setGraphic(card);
                }
            }
        });

        HBox notesActions = new HBox(10);
        notesActions.setAlignment(Pos.CENTER_RIGHT);

        Button copyNotesBtn = new Button("📋 Copier tout");
        copyNotesBtn.getStyleClass().add("btn-secondary");

        Button exportMdBtn = new Button("💾 Exporter en Markdown (.md)");
        exportMdBtn.getStyleClass().add("btn-primary");

        notesActions.getChildren().addAll(copyNotesBtn, exportMdBtn);
        notesBox.getChildren().addAll(notesList, notesActions);
        notesTab.setContent(notesBox);

        tabPane.getTabs().addAll(booksTab, progressTab, notesTab);

        // Actions: Refresh Devices Logic
        Runnable reloadDevices = () -> {
            Thread.ofVirtual().start(() -> {
                List<ConnectedDevice> list = deviceService.detectConnectedDevices();
                Platform.runLater(() -> {
                    deviceCombo.setItems(FXCollections.observableArrayList(list));
                    if (!list.isEmpty()) {
                        deviceCombo.setValue(list.get(0));
                    } else {
                        deviceCombo.setValue(null);
                        deviceCombo.setPromptText("Aucune liseuse connectée");
                        booksTable.getItems().clear();
                        notesList.getItems().clear();
                    }
                });
            });
        };

        refreshBtn.setOnAction(e -> reloadDevices.run());

        // On device selected
        deviceCombo.setOnAction(e -> {
            ConnectedDevice dev = deviceCombo.getValue();
            if (dev == null) return;

            Thread.ofVirtual().start(() -> {
                try {
                    List<DeviceBookInfo> devBooks = deviceService.listDeviceBooks(dev.kind(), dev.mountPoint());
                    List<DeviceAnnotation> devNotes = deviceService.getAnnotations(dev.kind(), dev.mountPoint());

                    Platform.runLater(() -> {
                        booksTable.setItems(FXCollections.observableArrayList(devBooks));
                        notesList.setItems(FXCollections.observableArrayList(devNotes));
                    });
                } catch (Exception ex) {
                    Platform.runLater(() -> {
                        Alert alert = new Alert(Alert.AlertType.ERROR, "Erreur de lecture de la liseuse: " + ex.getMessage(), ButtonType.OK);
                        alert.showAndWait();
                    });
                }
            });
        });

        // Import books action
        importAllBtn.setOnAction(e -> {
            ConnectedDevice dev = deviceCombo.getValue();
            if (dev == null) return;

            importAllBtn.setDisable(true);
            Thread.ofVirtual().start(() -> {
                try {
                    List<Book> imported = deviceService.importBooksFromDevice(dev.kind(), dev.mountPoint());
                    Platform.runLater(() -> {
                        importAllBtn.setDisable(false);
                        eventPublisher.publishEvent(new UiEvents.LibraryChanged());
                        Alert ok = new Alert(Alert.AlertType.INFORMATION,
                                imported.isEmpty()
                                        ? "Tous les livres de la liseuse sont déjà dans votre bibliothèque Vellum !"
                                        : imported.size() + " livre(s) importé(s) avec succès dans Vellum !",
                                ButtonType.OK);
                        ok.showAndWait();
                    });
                } catch (Exception ex) {
                    Platform.runLater(() -> {
                        importAllBtn.setDisable(false);
                        Alert err = new Alert(Alert.AlertType.ERROR, "Échec d'importation : " + ex.getMessage(), ButtonType.OK);
                        err.showAndWait();
                    });
                }
            });
        });

        // Sync progress action
        syncProgressBtn.setOnAction(e -> {
            ConnectedDevice dev = deviceCombo.getValue();
            if (dev == null) return;

            syncProgressBtn.setDisable(true);
            Thread.ofVirtual().start(() -> {
                try {
                    List<DeviceReadingProgress> updated = deviceService.syncReadingProgress(dev.kind(), dev.mountPoint());
                    Platform.runLater(() -> {
                        syncProgressBtn.setDisable(false);
                        eventPublisher.publishEvent(new UiEvents.LibraryChanged());
                        progressList.getItems().clear();
                        if (updated.isEmpty()) {
                            progressList.getItems().add("✓ Aucun nouvel avancement plus récent trouvé sur la liseuse.");
                        } else {
                            for (DeviceReadingProgress p : updated) {
                                progressList.getItems().add(String.format("📖 %s : %.0f %% (Terminé: %s)", p.bookTitle(), p.progressPercent(), p.isFinished() ? "Oui" : "Non"));
                            }
                        }
                    });
                } catch (Exception ex) {
                    Platform.runLater(() -> {
                        syncProgressBtn.setDisable(false);
                        Alert err = new Alert(Alert.AlertType.ERROR, "Échec de synchronisation : " + ex.getMessage(), ButtonType.OK);
                        err.showAndWait();
                    });
                }
            });
        });

        // Copy notes action
        copyNotesBtn.setOnAction(e -> {
            StringBuilder sb = new StringBuilder();
            for (DeviceAnnotation a : notesList.getItems()) {
                sb.append("## ").append(a.bookTitle()).append("\n");
                if (a.bookAuthor() != null) sb.append("*").append(a.bookAuthor()).append("*\n");
                if (a.text() != null && !a.text().isBlank()) sb.append("> ").append(a.text()).append("\n\n");
                if (a.note() != null) sb.append("**Note :** ").append(a.note()).append("\n\n");
                sb.append("---\n\n");
            }
            ClipboardContent cc = new ClipboardContent();
            cc.putString(sb.toString());
            Clipboard.getSystemClipboard().setContent(cc);
            Alert info = new Alert(Alert.AlertType.INFORMATION, "Toutes les citations ont été copiées dans le presse-papier !", ButtonType.OK);
            info.showAndWait();
        });

        // Export markdown action
        exportMdBtn.setOnAction(e -> {
            FileChooser fc = new FileChooser();
            fc.setTitle("Exporter les citations et surlignages");
            fc.setInitialFileName("citations_liseuse.md");
            fc.getExtensionFilters().add(new FileChooser.ExtensionFilter("Markdown (*.md)", "*.md"));
            File file = fc.showSaveDialog(stage);
            if (file != null) {
                try {
                    StringBuilder sb = new StringBuilder();
                    sb.append("# 📖 Citations & Surlignages de la liseuse\n\n");
                    for (DeviceAnnotation a : notesList.getItems()) {
                        sb.append("### ").append(a.bookTitle()).append("\n");
                        if (a.bookAuthor() != null) sb.append("*").append(a.bookAuthor()).append("*\n");
                        if (a.location() != null) sb.append("📍 *").append(a.location()).append("*\n\n");
                        if (a.text() != null && !a.text().isBlank()) sb.append("> ").append(a.text()).append("\n\n");
                        if (a.note() != null) sb.append("**Note :** ").append(a.note()).append("\n\n");
                        sb.append("---\n\n");
                    }
                    Files.writeString(file.toPath(), sb.toString());
                    Alert ok = new Alert(Alert.AlertType.INFORMATION, "Citations exportées avec succès vers " + file.getName() + " !", ButtonType.OK);
                    ok.showAndWait();
                } catch (Exception ex) {
                    Alert err = new Alert(Alert.AlertType.ERROR, "Erreur d'exportation : " + ex.getMessage(), ButtonType.OK);
                    err.showAndWait();
                }
            }
        });

        // Close button
        Button closeBtn = new Button("Fermer");
        closeBtn.getStyleClass().add("btn-secondary");
        closeBtn.setOnAction(e -> stage.close());

        HBox bottomBar = new HBox(closeBtn);
        bottomBar.setAlignment(Pos.CENTER_RIGHT);
        bottomBar.setPadding(new Insets(16, 0, 0, 0));

        root.setTop(topBox);
        root.setCenter(tabPane);
        root.setBottom(bottomBar);

        Scene scene = new Scene(root, 760, 560);
        scene.getStylesheets().add(DeviceSyncDialog.class.getResource("/styles/theme.css").toExternalForm());
        stage.setScene(scene);

        reloadDevices.run();
        stage.show();
    }
}
