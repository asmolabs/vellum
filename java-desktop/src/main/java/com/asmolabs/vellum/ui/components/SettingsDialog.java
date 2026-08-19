package com.asmolabs.vellum.ui.components;

import com.asmolabs.vellum.config.AppProperties;
import com.asmolabs.vellum.service.LitestreamService;
import com.asmolabs.vellum.service.PluginService;
import javafx.geometry.Insets;
import javafx.geometry.Pos;
import javafx.scene.Scene;
import javafx.scene.control.*;
import javafx.scene.layout.*;
import javafx.stage.DirectoryChooser;
import javafx.stage.Modality;
import javafx.stage.Stage;
import javafx.stage.Window;

import java.io.File;
import java.util.HashMap;
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
        VBox pluginsBox = new VBox(20);
        pluginsBox.setPadding(new Insets(20, 10, 20, 10));

        Map<String, Map<String, Control>> pluginInputs = new HashMap<>();

        for (PluginService.LibraryPlugin plugin : pluginService.getRegisteredPlugins()) {
            VBox pBox = new VBox(8);
            pBox.setStyle("-fx-background-color: rgba(255, 255, 255, 0.03); -fx-border-color: rgba(255, 255, 255, 0.08); -fx-border-radius: 8; -fx-padding: 12;");

            Label pName = new Label(plugin.name() + " (v" + plugin.version() + ")");
            pName.setStyle("-fx-font-weight: bold; -fx-font-size: 13px; -fx-text-fill: #f3f4f6;");

            Label pDesc = new Label(plugin.description());
            pDesc.setWrapText(true);
            pDesc.setStyle("-fx-font-size: 11px; -fx-text-fill: #9ca3af;");

            pBox.getChildren().addAll(pName, pDesc);

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
                } else {
                    TextField tf = new TextField(val != null ? String.valueOf(val) : "");
                    tf.setPromptText(field.description());
                    tf.getStyleClass().add("text-field");
                    fieldControls.put(field.key(), tf);
                    pBox.getChildren().addAll(fLbl, tf);
                }
            }

            pluginInputs.put(plugin.id(), fieldControls);
            pluginsBox.getChildren().add(pBox);
        }

        ScrollPane pluginsScroll = new ScrollPane(pluginsBox);
        pluginsScroll.setFitToWidth(true);
        pluginsScroll.setStyle("-fx-background-color: transparent; -fx-background: transparent;");
        pluginsTab.setContent(pluginsScroll);

        tabPane.getTabs().addAll(genTab, liteTab, pluginsTab);

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
