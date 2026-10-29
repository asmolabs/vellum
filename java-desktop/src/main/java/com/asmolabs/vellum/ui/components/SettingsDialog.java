package com.asmolabs.vellum.ui.components;

import com.asmolabs.vellum.config.AppProperties;
import com.asmolabs.vellum.service.LitestreamService;
import com.asmolabs.vellum.service.PluginService;
import javafx.application.Platform;
import javafx.geometry.Insets;
import javafx.geometry.Pos;
import javafx.scene.Scene;
import javafx.scene.control.*;
import javafx.scene.layout.*;
import javafx.stage.DirectoryChooser;
import javafx.stage.FileChooser;
import javafx.stage.Modality;
import javafx.stage.Stage;
import javafx.stage.Window;

import java.awt.Desktop;
import java.io.File;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

public class SettingsDialog {

    public static void show(Window owner, AppProperties appProperties, LitestreamService litestreamService, PluginService pluginService, Runnable onUpdated) {
        Stage stage = new Stage();
        stage.initOwner(owner);
        stage.initModality(Modality.APPLICATION_MODAL);
        stage.setTitle("Paramètres de Vellum");

        BorderPane root = new BorderPane();
        root.getStyleClass().add("dialog-container");
        root.setPadding(new Insets(20));

        TabPane tabPane = new TabPane();
        tabPane.setTabClosingPolicy(TabPane.TabClosingPolicy.UNAVAILABLE);

        // Tab 1: General & Library
        Tab genTab = new Tab("Général & Bibliothèque");
        VBox genBox = new VBox(16);
        genBox.setPadding(new Insets(20, 10, 20, 10));

        Label libPathLabel = new Label("Emplacement de la bibliothèque :");
        libPathLabel.getStyleClass().add("form-label");

        TextField libPathField = new TextField(appProperties.getLibraryPath());
        libPathField.getStyleClass().add("text-field");
        HBox.setHgrow(libPathField, Priority.ALWAYS);

        Button browseLibBtn = new Button("Parcourir...");
        browseLibBtn.getStyleClass().add("btn-secondary");
        browseLibBtn.setOnAction(e -> {
            DirectoryChooser dc = new DirectoryChooser();
            dc.setTitle("Sélectionner le répertoire de la bibliothèque");
            File f = dc.showDialog(stage);
            if (f != null) {
                libPathField.setText(f.getAbsolutePath());
            }
        });

        HBox libBox = new HBox(8, libPathField, browseLibBtn);
        genBox.getChildren().addAll(libPathLabel, libBox);
        genTab.setContent(genBox);

        // Tab 2: Litestream & SQLite Continuous Backup
        Tab liteTab = new Tab("Litestream & Sauvegarde");
        VBox liteBox = new VBox(16);
        liteBox.setPadding(new Insets(20, 10, 20, 10));

        CheckBox enableLitestream = new CheckBox("Activer la réplication continue Litestream (SQLite WAL)");
        enableLitestream.setSelected(appProperties.getLitestream().isEnabled());
        enableLitestream.setStyle("-fx-text-fill: #f3f4f6; -fx-font-weight: bold;");

        Label statusLbl = new Label("Statut : " + litestreamService.getStatus() + " — " + litestreamService.getLastStatusMessage());
        statusLbl.setStyle("-fx-text-fill: #a78bfa; -fx-font-size: 12px;");

        Label binaryLabel = new Label("Chemin du binaire Litestream :");
        binaryLabel.getStyleClass().add("form-label");
        TextField binaryField = new TextField(appProperties.getLitestream().getBinaryPath());
        binaryField.getStyleClass().add("text-field");

        Label replicaLabel = new Label("Dossier de réplication / Sauvegarde (Local ou S3) :");
        replicaLabel.getStyleClass().add("form-label");
        TextField replicaField = new TextField(appProperties.getLitestream().getReplicaPath());
        replicaField.getStyleClass().add("text-field");

        HBox liteActions = new HBox(12);
        liteActions.setAlignment(Pos.CENTER_LEFT);

        Button startStopBtn = new Button(litestreamService.getStatus() == LitestreamService.Status.RUNNING ? "Arrêter la réplication" : "Démarrer la réplication");
        startStopBtn.getStyleClass().add("btn-secondary");
        startStopBtn.setOnAction(e -> {
            if (litestreamService.getStatus() == LitestreamService.Status.RUNNING) {
                litestreamService.stopReplication();
            } else {
                appProperties.getLitestream().setEnabled(true);
                appProperties.getLitestream().setBinaryPath(binaryField.getText().trim());
                appProperties.getLitestream().setReplicaPath(replicaField.getText().trim());
                litestreamService.startReplication();
            }
            statusLbl.setText("Statut : " + litestreamService.getStatus() + " — " + litestreamService.getLastStatusMessage());
            startStopBtn.setText(litestreamService.getStatus() == LitestreamService.Status.RUNNING ? "Arrêter la réplication" : "Démarrer la réplication");
        });

        Button genConfigBtn = new Button("Générer litestream.yml");
        genConfigBtn.getStyleClass().add("btn-secondary");
        genConfigBtn.setOnAction(e -> {
            try {
                File f = litestreamService.generateConfigFile();
                Alert a = new Alert(Alert.AlertType.INFORMATION, "Configuration générée avec succès :\n" + f.getAbsolutePath(), ButtonType.OK);
                a.showAndWait();
            } catch (Exception ex) {
                Alert a = new Alert(Alert.AlertType.ERROR, "Erreur : " + ex.getMessage(), ButtonType.OK);
                a.showAndWait();
            }
        });

        liteActions.getChildren().addAll(startStopBtn, genConfigBtn);
        liteBox.getChildren().addAll(enableLitestream, statusLbl, binaryLabel, binaryField, replicaLabel, replicaField, liteActions);
        liteTab.setContent(liteBox);

        // Tab 3: Plugins Settings
        Tab pluginsTab = new Tab("Extensions & Plugins");
        VBox pluginsBox = new VBox(16);
        pluginsBox.setPadding(new Insets(20, 10, 20, 10));

        // External plugins folder banner
        VBox extBanner = new VBox(8);
        extBanner.setStyle("-fx-background-color: rgba(16, 185, 129, 0.08); -fx-border-color: rgba(16, 185, 129, 0.25); -fx-border-radius: 8; -fx-padding: 12;");
        Label extTitle = new Label("🔌 Gestion des Plugins Externes & Scripts");
        extTitle.setStyle("-fx-font-weight: bold; -fx-text-fill: #34d399; -fx-font-size: 12px;");
        Label extDesc = new Label("Déposez des scripts (Python, Bash avec plugin.json) ou des archives JAR dans le dossier ci-dessous.");
        extDesc.setStyle("-fx-text-fill: #9ca3af; -fx-font-size: 11px;");

        TextField pluginsPathField = new TextField(appProperties.getPluginsPath());
        pluginsPathField.setEditable(false);
        pluginsPathField.getStyleClass().add("text-field");
        HBox.setHgrow(pluginsPathField, Priority.ALWAYS);

        Button openFolderBtn = new Button("📂 Ouvrir dossier");
        openFolderBtn.getStyleClass().add("btn-secondary");
        openFolderBtn.setOnAction(e -> {
            try {
                File dir = pluginService.getPluginsDirectory();
                if (Desktop.isDesktopSupported()) {
                    Desktop.getDesktop().open(dir);
                }
            } catch (Exception ex) {
                Alert a = new Alert(Alert.AlertType.ERROR, "Impossible d'ouvrir le dossier : " + ex.getMessage(), ButtonType.OK);
                a.showAndWait();
            }
        });

        HBox folderRow = new HBox(8, pluginsPathField, openFolderBtn);

        extBanner.getChildren().addAll(extTitle, extDesc, folderRow);
        pluginsBox.getChildren().add(extBanner);

        Map<String, CheckBox> pluginEnabledBoxes = new HashMap<>();
        Map<String, Map<String, Control>> pluginInputs = new HashMap<>();
        VBox pluginsListContainer = new VBox(12);

        Runnable populatePluginsList = () -> {
            pluginsListContainer.getChildren().clear();
            pluginEnabledBoxes.clear();
            pluginInputs.clear();

            for (PluginService.LibraryPlugin plugin : pluginService.getRegisteredPlugins()) {
                VBox pBox = new VBox(8);
                pBox.setStyle("-fx-background-color: rgba(255, 255, 255, 0.03); -fx-border-color: rgba(255, 255, 255, 0.08); -fx-border-radius: 8; -fx-padding: 12;");

                // Header row: Name, Badge Interne/Externe, Type, and Enable checkbox
                HBox headRow = new HBox(8);
                headRow.setAlignment(Pos.CENTER_LEFT);

                Label pName = new Label(plugin.name() + " (v" + plugin.version() + ")");
                pName.setStyle("-fx-font-weight: bold; -fx-font-size: 13px; -fx-text-fill: #f3f4f6;");

                Label originBadge = new Label(plugin.external() ? "EXTERNE" : "INTERNE");
                if (plugin.external()) {
                    originBadge.setStyle("-fx-background-color: rgba(16, 185, 129, 0.2); -fx-text-fill: #34d399; -fx-padding: 2 6; -fx-background-radius: 4; -fx-font-size: 9px; -fx-font-weight: bold;");
                } else {
                    originBadge.setStyle("-fx-background-color: rgba(139, 92, 246, 0.2); -fx-text-fill: #c084fc; -fx-padding: 2 6; -fx-background-radius: 4; -fx-font-size: 9px; -fx-font-weight: bold;");
                }

                String typeStr;
                String typeStyle;
                if ("format".equalsIgnoreCase(plugin.type())) {
                    typeStr = "Support de Format";
                    typeStyle = "-fx-background-color: rgba(6, 182, 212, 0.2); -fx-text-fill: #22d3ee; -fx-padding: 2 6; -fx-background-radius: 4; -fx-font-size: 9px;";
                } else if ("import".equalsIgnoreCase(plugin.type())) {
                    typeStr = "Import / Enrichissement";
                    typeStyle = "-fx-background-color: rgba(59, 130, 246, 0.2); -fx-text-fill: #60a5fa; -fx-padding: 2 6; -fx-background-radius: 4; -fx-font-size: 9px;";
                } else if ("conversion".equalsIgnoreCase(plugin.type())) {
                    typeStr = "Conversion";
                    typeStyle = "-fx-background-color: rgba(236, 72, 153, 0.2); -fx-text-fill: #f472b6; -fx-padding: 2 6; -fx-background-radius: 4; -fx-font-size: 9px;";
                } else if ("device".equalsIgnoreCase(plugin.type())) {
                    typeStr = "Liseuse / Sync";
                    typeStyle = "-fx-background-color: rgba(16, 185, 129, 0.2); -fx-text-fill: #34d399; -fx-padding: 2 6; -fx-background-radius: 4; -fx-font-size: 9px;";
                } else {
                    typeStr = "Transformation";
                    typeStyle = "-fx-background-color: rgba(245, 158, 11, 0.2); -fx-text-fill: #fbbf24; -fx-padding: 2 6; -fx-background-radius: 4; -fx-font-size: 9px;";
                }
                Label typeBadge = new Label(typeStr);
                typeBadge.setStyle(typeStyle);

                Region spacer = new Region();
                HBox.setHgrow(spacer, Priority.ALWAYS);

                CheckBox enableCb = new CheckBox("Actif");
                enableCb.setSelected(plugin.enabled());
                enableCb.setStyle("-fx-text-fill: #e5e7eb; -fx-font-size: 11px;");
                pluginEnabledBoxes.put(plugin.id(), enableCb);

                headRow.getChildren().addAll(pName, originBadge, typeBadge, spacer, enableCb);

                Label pDesc = new Label(plugin.description());
                pDesc.setWrapText(true);
                pDesc.setStyle("-fx-font-size: 11px; -fx-text-fill: #9ca3af;");

                pBox.getChildren().addAll(headRow, pDesc);

                Map<String, Object> currentParams = pluginService.getPluginParams(plugin.id());
                Map<String, Control> fieldControls = new HashMap<>();

                for (PluginService.ConfigField field : plugin.configSchema()) {
                    Label fLbl = new Label(field.label() + " :");
                    fLbl.setStyle("-fx-font-size: 11px; -fx-text-fill: #c084fc;");

                    Object val = currentParams.getOrDefault(field.key(), field.defaultValue());

                    if ("boolean".equalsIgnoreCase(field.type())) {
                        CheckBox cb = new CheckBox(field.description());
                        cb.setSelected(Boolean.parseBoolean(String.valueOf(val)));
                        cb.setStyle("-fx-text-fill: #d1d5db; -fx-font-size: 12px;");
                        fieldControls.put(field.key(), cb);
                        pBox.getChildren().addAll(fLbl, cb);
                    } else if ("select".equalsIgnoreCase(field.type())) {
                        ComboBox<String> combo = new ComboBox<>();
                        combo.getItems().addAll(field.options());
                        combo.setValue(String.valueOf(val));
                        combo.getStyleClass().add("combo-box");
                        fieldControls.put(field.key(), combo);
                        pBox.getChildren().addAll(fLbl, combo);
                    } else if ("password".equalsIgnoreCase(field.type()) || "secret".equalsIgnoreCase(field.type())) {
                        HBox passBox = new HBox(6);
                        passBox.setAlignment(Pos.CENTER_LEFT);

                        PasswordField pf = new PasswordField();
                        pf.setText(val != null ? String.valueOf(val) : "");
                        pf.setPromptText(field.description());
                        pf.getStyleClass().add("text-field");
                        HBox.setHgrow(pf, Priority.ALWAYS);

                        TextField tf = new TextField(pf.getText());
                        tf.setPromptText(field.description());
                        tf.getStyleClass().add("text-field");
                        tf.setVisible(false);
                        tf.setManaged(false);
                        HBox.setHgrow(tf, Priority.ALWAYS);

                        pf.textProperty().bindBidirectional(tf.textProperty());

                        Button eyeBtn = new Button("👁");
                        eyeBtn.getStyleClass().add("btn-secondary");
                        eyeBtn.setOnAction(ev -> {
                            if (pf.isVisible()) {
                                pf.setVisible(false);
                                pf.setManaged(false);
                                tf.setVisible(true);
                                tf.setManaged(true);
                                eyeBtn.setText("🙈");
                            } else {
                                tf.setVisible(false);
                                tf.setManaged(false);
                                pf.setVisible(true);
                                pf.setManaged(true);
                                eyeBtn.setText("👁");
                            }
                        });

                        passBox.getChildren().addAll(pf, tf, eyeBtn);
                        fieldControls.put(field.key(), pf);
                        pBox.getChildren().addAll(fLbl, passBox);
                    } else if ("file".equalsIgnoreCase(field.type()) || "path".equalsIgnoreCase(field.type())) {
                        HBox fileBox = new HBox(6);
                        fileBox.setAlignment(Pos.CENTER_LEFT);

                        TextField tf = new TextField(val != null ? String.valueOf(val) : "");
                        tf.setPromptText(field.description());
                        tf.getStyleClass().add("text-field");
                        HBox.setHgrow(tf, Priority.ALWAYS);

                        Button browseBtn = new Button("📂 Parcourir...");
                        browseBtn.getStyleClass().add("btn-secondary");
                        browseBtn.setOnAction(ev -> {
                            FileChooser fc = new FileChooser();
                            fc.setTitle("Sélectionner un fichier");
                            File chosen = fc.showOpenDialog(stage);
                            if (chosen != null) {
                                tf.setText(chosen.getAbsolutePath());
                            }
                        });

                        fileBox.getChildren().addAll(tf, browseBtn);
                        fieldControls.put(field.key(), tf);
                        pBox.getChildren().addAll(fLbl, fileBox);
                    } else if ("directory".equalsIgnoreCase(field.type()) || "folder".equalsIgnoreCase(field.type())) {
                        HBox dirBox = new HBox(6);
                        dirBox.setAlignment(Pos.CENTER_LEFT);

                        TextField tf = new TextField(val != null ? String.valueOf(val) : "");
                        tf.setPromptText(field.description());
                        tf.getStyleClass().add("text-field");
                        HBox.setHgrow(tf, Priority.ALWAYS);

                        Button browseBtn = new Button("📂 Parcourir...");
                        browseBtn.getStyleClass().add("btn-secondary");
                        browseBtn.setOnAction(ev -> {
                            DirectoryChooser dc = new DirectoryChooser();
                            dc.setTitle("Sélectionner un dossier");
                            File chosen = dc.showDialog(stage);
                            if (chosen != null) {
                                tf.setText(chosen.getAbsolutePath());
                            }
                        });

                        dirBox.getChildren().addAll(tf, browseBtn);
                        fieldControls.put(field.key(), tf);
                        pBox.getChildren().addAll(fLbl, dirBox);
                    } else if ("slider".equalsIgnoreCase(field.type()) || "number".equalsIgnoreCase(field.type())) {
                        double min = field.min() != null ? field.min() : 0.0;
                        double max = field.max() != null ? field.max() : 1.0;
                        double cur = min;
                        try {
                            if (val != null) cur = Double.parseDouble(String.valueOf(val));
                        } catch (Exception ignored) {}

                        Slider slider = new Slider(min, max, cur);
                        slider.setShowTickLabels(false);
                        HBox.setHgrow(slider, Priority.ALWAYS);

                        Label valLbl = new Label(String.format(java.util.Locale.US, "%.2f", cur));
                        valLbl.setStyle("-fx-text-fill: #c084fc; -fx-font-weight: bold; -fx-min-width: 45px;");

                        slider.valueProperty().addListener((obs, oldV, newV) -> {
                            valLbl.setText(String.format(java.util.Locale.US, "%.2f", newV.doubleValue()));
                        });

                        HBox sliderBox = new HBox(10, slider, valLbl);
                        sliderBox.setAlignment(Pos.CENTER_LEFT);

                        Label descLbl = new Label(field.description());
                        descLbl.setStyle("-fx-text-fill: #9ca3af; -fx-font-size: 10px;");

                        fieldControls.put(field.key(), slider);
                        pBox.getChildren().addAll(fLbl, sliderBox, descLbl);
                    } else if ("textarea".equalsIgnoreCase(field.type())) {
                        TextArea ta = new TextArea(val != null ? String.valueOf(val) : "");
                        ta.setPromptText(field.description());
                        ta.setPrefRowCount(3);
                        ta.setWrapText(true);
                        ta.getStyleClass().add("text-area");
                        fieldControls.put(field.key(), ta);
                        pBox.getChildren().addAll(fLbl, ta);
                    } else {
                        TextField tf = new TextField(val != null ? String.valueOf(val) : "");
                        tf.setPromptText(field.description());
                        tf.getStyleClass().add("text-field");
                        fieldControls.put(field.key(), tf);
                        pBox.getChildren().addAll(fLbl, tf);
                    }
                }

                // Test configuration action row
                HBox testRow = new HBox(10);
                testRow.setAlignment(Pos.CENTER_LEFT);
                testRow.setStyle("-fx-padding: 4 0 0 0;");

                Button testBtn = new Button("🧪 Tester la configuration");
                testBtn.setStyle("-fx-background-color: #374151; -fx-text-fill: #e5e7eb; -fx-font-size: 11px; -fx-padding: 4 10; -fx-cursor: hand; -fx-background-radius: 4;");
                testBtn.setOnMouseEntered(e -> testBtn.setStyle("-fx-background-color: #4b5563; -fx-text-fill: #ffffff; -fx-font-size: 11px; -fx-padding: 4 10; -fx-cursor: hand; -fx-background-radius: 4;"));
                testBtn.setOnMouseExited(e -> testBtn.setStyle("-fx-background-color: #374151; -fx-text-fill: #e5e7eb; -fx-font-size: 11px; -fx-padding: 4 10; -fx-cursor: hand; -fx-background-radius: 4;"));

                Label testResultBadge = new Label();
                testResultBadge.setVisible(false);

                testBtn.setOnAction(e -> {
                    testResultBadge.setText("⏳ Test en cours...");
                    testResultBadge.setStyle("-fx-background-color: rgba(107, 114, 128, 0.2); -fx-text-fill: #9ca3af; -fx-padding: 3 8; -fx-background-radius: 4; -fx-font-size: 11px;");
                    testResultBadge.setVisible(true);

                    Map<String, Object> currentFormVals = new HashMap<>();
                    for (Map.Entry<String, Control> entry : fieldControls.entrySet()) {
                        String key = entry.getKey();
                        Control ctrl = entry.getValue();
                        if (ctrl instanceof CheckBox cb) currentFormVals.put(key, cb.isSelected());
                        else if (ctrl instanceof TextField tf) currentFormVals.put(key, tf.getText());
                        else if (ctrl instanceof TextArea ta) currentFormVals.put(key, ta.getText());
                        else if (ctrl instanceof ComboBox<?> cmb) currentFormVals.put(key, cmb.getValue());
                        else if (ctrl instanceof Slider sl) currentFormVals.put(key, sl.getValue());
                    }

                    Thread.ofVirtual().start(() -> {
                        var res = pluginService.testPluginConfiguration(plugin.id(), currentFormVals);
                        Platform.runLater(() -> {
                            testResultBadge.setVisible(true);
                            if (res.success()) {
                                testResultBadge.setText("✓ " + res.message());
                                testResultBadge.setStyle("-fx-background-color: rgba(16, 185, 129, 0.2); -fx-text-fill: #34d399; -fx-padding: 3 8; -fx-background-radius: 4; -fx-font-size: 11px; -fx-font-weight: bold;");
                            } else {
                                testResultBadge.setText("✗ " + res.message());
                                testResultBadge.setStyle("-fx-background-color: rgba(239, 68, 68, 0.2); -fx-text-fill: #f87171; -fx-padding: 3 8; -fx-background-radius: 4; -fx-font-size: 11px; -fx-font-weight: bold;");
                            }
                        });
                    });
                });

                testRow.getChildren().addAll(testBtn, testResultBadge);
                pBox.getChildren().add(testRow);

                pluginInputs.put(plugin.id(), fieldControls);
                pluginsListContainer.getChildren().add(pBox);
            }
        };

        // Reload plugins button
        Button reloadBtn = new Button("🔄 Recharger les plugins externes");
        reloadBtn.getStyleClass().add("btn-secondary");
        reloadBtn.setOnAction(e -> {
            pluginService.reloadPlugins();
            populatePluginsList.run();
        });
        folderRow.getChildren().add(reloadBtn);

        populatePluginsList.run();
        pluginsBox.getChildren().add(pluginsListContainer);

        ScrollPane pluginsScroll = new ScrollPane(pluginsBox);
        pluginsScroll.setFitToWidth(true);
        pluginsScroll.setStyle("-fx-background-color: transparent; -fx-background: transparent;");
        pluginsTab.setContent(pluginsScroll);

        // Tab 4: OPDS Server & Mobile Connectivity
        Tab opdsTab = new Tab("Serveur OPDS & Liseuses");
        VBox opdsBox = new VBox(16);
        opdsBox.setPadding(new Insets(20, 10, 20, 10));

        Label opdsTitle = new Label("Accès Mobile & Liseuses (Moon+ Reader, Librera, KOReader)");
        opdsTitle.setStyle("-fx-font-weight: bold; -fx-text-fill: #f3f4f6; -fx-font-size: 13px;");

        Label opdsDesc = new Label("Le serveur OPDS 1.2 embarqué permet à vos appareils mobiles connectés au même réseau Wi-Fi d'accéder directement à votre bibliothèque, de télécharger vos livres et de voir les couvertures.");
        opdsDesc.setWrapText(true);
        opdsDesc.setStyle("-fx-text-fill: #9ca3af; -fx-font-size: 12px;");

        String localIp = "localhost";
        try {
            localIp = java.net.InetAddress.getLocalHost().getHostAddress();
        } catch (Exception ignored) {}

        String opdsUrl = "http://" + localIp + ":8085/opds";

        Label urlLabel = new Label("URL du Catalogue OPDS :");
        urlLabel.getStyleClass().add("form-label");

        TextField urlField = new TextField(opdsUrl);
        urlField.setEditable(false);
        urlField.getStyleClass().add("text-field");
        HBox.setHgrow(urlField, Priority.ALWAYS);

        Button copyUrlBtn = new Button("📋 Copier l'URL");
        copyUrlBtn.getStyleClass().add("btn-secondary");
        copyUrlBtn.setOnAction(e -> {
            javafx.scene.input.ClipboardContent cc = new javafx.scene.input.ClipboardContent();
            cc.putString(opdsUrl);
            javafx.scene.input.Clipboard.getSystemClipboard().setContent(cc);
            copyUrlBtn.setText("✓ Copié !");
        });

        HBox urlBox = new HBox(8, urlField, copyUrlBtn);

        VBox infoCard = new VBox(6);
        infoCard.setStyle("-fx-background-color: rgba(139, 92, 246, 0.08); -fx-border-color: rgba(139, 92, 246, 0.25); -fx-border-radius: 8; -fx-padding: 12;");
        Label helpTitle = new Label("💡 Comment se connecter sur mobile / tablette :");
        helpTitle.setStyle("-fx-font-weight: bold; -fx-text-fill: #c084fc; -fx-font-size: 11px;");
        Label step1 = new Label("1. Ouvrez votre application de lecture (ex: Moon+ Reader ou Librera).");
        Label step2 = new Label("2. Allez dans 'Bibliothèques réseau' / 'Catalogue OPDS' -> 'Ajouter un catalogue'.");
        Label step3 = new Label("3. Collez l'URL ci-dessus : " + opdsUrl);
        for (Label l : List.of(step1, step2, step3)) {
            l.setStyle("-fx-text-fill: #d1d5db; -fx-font-size: 11px;");
        }
        infoCard.getChildren().addAll(helpTitle, step1, step2, step3);

        opdsBox.getChildren().addAll(opdsTitle, opdsDesc, urlLabel, urlBox, infoCard);
        opdsTab.setContent(opdsBox);

        tabPane.getTabs().addAll(genTab, liteTab, opdsTab, pluginsTab);

        // Bottom action bar
        HBox actions = new HBox(12);
        actions.setAlignment(Pos.CENTER_RIGHT);
        actions.setPadding(new Insets(16, 0, 0, 0));

        Button cancelBtn = new Button("Annuler");
        cancelBtn.getStyleClass().add("btn-secondary");
        cancelBtn.setOnAction(e -> stage.close());

        Button saveBtn = new Button("Enregistrer les paramètres");
        saveBtn.getStyleClass().add("btn-primary");
        saveBtn.setOnAction(e -> {
            appProperties.setLibraryPath(libPathField.getText().trim());
            appProperties.getLitestream().setEnabled(enableLitestream.isSelected());
            appProperties.getLitestream().setBinaryPath(binaryField.getText().trim());
            appProperties.getLitestream().setReplicaPath(replicaField.getText().trim());

            // Save plugin enabled states
            for (Map.Entry<String, CheckBox> entry : pluginEnabledBoxes.entrySet()) {
                pluginService.setPluginEnabled(entry.getKey(), entry.getValue().isSelected());
            }

            // Save plugin parameters
            for (Map.Entry<String, Map<String, Control>> entry : pluginInputs.entrySet()) {
                String pId = entry.getKey();
                Map<String, Object> newParams = new HashMap<>();
                for (Map.Entry<String, Control> fieldEntry : entry.getValue().entrySet()) {
                    String fKey = fieldEntry.getKey();
                    Control c = fieldEntry.getValue();
                    if (c instanceof CheckBox cb) {
                        newParams.put(fKey, cb.isSelected());
                    } else if (c instanceof ComboBox<?> combo) {
                        newParams.put(fKey, combo.getValue());
                    } else if (c instanceof Slider sl) {
                        newParams.put(fKey, sl.getValue());
                    } else if (c instanceof TextArea ta) {
                        newParams.put(fKey, ta.getText());
                    } else if (c instanceof TextField tf) {
                        newParams.put(fKey, tf.getText().trim());
                    }
                }
                pluginService.savePluginParams(pId, newParams);
            }

            if (onUpdated != null) {
                onUpdated.run();
            }
            stage.close();
        });

        actions.getChildren().addAll(cancelBtn, saveBtn);

        root.setCenter(tabPane);
        root.setBottom(actions);

        Scene scene = new Scene(root, 650, 600);
        scene.getStylesheets().add(SettingsDialog.class.getResource("/styles/theme.css").toExternalForm());
        stage.setScene(scene);
        stage.show();
    }
}
