package com.asmolabs.vellum.ui.components;

import com.asmolabs.vellum.domain.entity.Book;
import com.asmolabs.vellum.service.BookService;
import com.asmolabs.vellum.service.PluginService;
import javafx.application.Platform;
import javafx.geometry.Insets;
import javafx.geometry.Pos;
import javafx.scene.Scene;
import javafx.scene.control.*;
import javafx.scene.layout.*;
import javafx.stage.Modality;
import javafx.stage.Stage;
import javafx.stage.Window;

import java.util.*;
import java.util.function.Consumer;

public class PluginRunDialog {

    public static void show(
            Window owner,
            Book book,
            String pluginId,
            PluginService pluginService,
            BookService bookService,
            Consumer<Book> onCompleted
    ) {
        var pluginOpt = pluginService.getRegisteredPlugins().stream()
                .filter(p -> p.id().equals(pluginId))
                .findFirst();

        if (pluginOpt.isEmpty()) {
            new Alert(Alert.AlertType.ERROR, "Plugin introuvable : " + pluginId).showAndWait();
            return;
        }

        PluginService.LibraryPlugin plugin = pluginOpt.get();
        Map<String, Object> currentParams = pluginService.getPluginParams(pluginId);

        Stage stage = new Stage();
        stage.initOwner(owner);
        stage.initModality(Modality.APPLICATION_MODAL);
        stage.setTitle("Exécution — " + plugin.name());

        VBox root = new VBox(16);
        root.setPadding(new Insets(20));
        root.setStyle("-fx-background-color: #121212;");

        // Header
        VBox headerBox = new VBox(6);
        HBox titleRow = new HBox(8);
        titleRow.setAlignment(Pos.CENTER_LEFT);

        Label iconLbl = new Label(plugin.external() ? "🔌" : "⚡");
        iconLbl.setStyle("-fx-font-size: 18px;");

        Label titleLbl = new Label(plugin.name());
        titleLbl.setStyle("-fx-font-weight: bold; -fx-font-size: 15px; -fx-text-fill: #f3f4f6;");

        Label badge = new Label(plugin.version());
        badge.setStyle("-fx-background-color: rgba(139, 92, 246, 0.2); -fx-text-fill: #c084fc; -fx-padding: 2 6; -fx-background-radius: 4; -fx-font-size: 10px;");

        titleRow.getChildren().addAll(iconLbl, titleLbl, badge);

        Label descLbl = new Label(plugin.description());
        descLbl.setWrapText(true);
        descLbl.setStyle("-fx-font-size: 11px; -fx-text-fill: #9ca3af;");

        Label targetBookLbl = new Label("Livre cible : « " + book.getTitle() + " » (" + book.getAuthor() + ")");
        targetBookLbl.setStyle("-fx-font-size: 12px; -fx-font-weight: bold; -fx-text-fill: #a78bfa;");

        headerBox.getChildren().addAll(titleRow, descLbl, targetBookLbl);
        root.getChildren().add(headerBox);

        // Form Fields in a ScrollPane
        VBox formBox = new VBox(12);
        formBox.setStyle("-fx-background-color: rgba(255, 255, 255, 0.03); -fx-border-color: rgba(255, 255, 255, 0.08); -fx-border-radius: 8; -fx-padding: 14;");

        Map<String, Control> fieldControls = new HashMap<>();

        for (PluginService.ConfigField field : plugin.configSchema()) {
            VBox fGroup = new VBox(4);
            Label fLbl = new Label(field.label());
            fLbl.setStyle("-fx-font-size: 11px; -fx-font-weight: bold; -fx-text-fill: #e5e7eb;");

            Object val = currentParams.getOrDefault(field.key(), field.defaultValue());

            if ("boolean".equalsIgnoreCase(field.type())) {
                CheckBox cb = new CheckBox(field.description());
                cb.setSelected(Boolean.parseBoolean(String.valueOf(val)));
                cb.setStyle("-fx-text-fill: #d1d5db; -fx-font-size: 12px;");
                fieldControls.put(field.key(), cb);
                fGroup.getChildren().addAll(fLbl, cb);
            } else if ("select".equalsIgnoreCase(field.type())) {
                ComboBox<String> combo = new ComboBox<>();
                combo.getItems().addAll(field.options());
                combo.setValue(String.valueOf(val));
                combo.setMaxWidth(Double.MAX_VALUE);
                combo.getStyleClass().add("combo-box");
                fieldControls.put(field.key(), combo);

                Label subDesc = new Label(field.description());
                subDesc.setStyle("-fx-font-size: 10px; -fx-text-fill: #6b7280;");
                fGroup.getChildren().addAll(fLbl, combo, subDesc);
            } else if ("slider".equalsIgnoreCase(field.type()) || "number".equalsIgnoreCase(field.type())) {
                double min = field.min() != null ? field.min() : 0.0;
                double max = field.max() != null ? field.max() : 100.0;
                double curVal = 0.0;
                try {
                    curVal = Double.parseDouble(String.valueOf(val));
                } catch (Exception ignored) {}

                Slider slider = new Slider(min, max, curVal);
                slider.setBlockIncrement(field.step() != null ? field.step() : 1.0);
                Label valLbl = new Label(String.format("%.2f", curVal));
                valLbl.setStyle("-fx-text-fill: #c084fc; -fx-font-size: 11px; -fx-font-weight: bold;");

                slider.valueProperty().addListener((obs, old, n) -> valLbl.setText(String.format("%.2f", n.doubleValue())));

                HBox sRow = new HBox(8, slider, valLbl);
                sRow.setAlignment(Pos.CENTER_LEFT);
                HBox.setHgrow(slider, Priority.ALWAYS);
                fieldControls.put(field.key(), slider);

                Label subDesc = new Label(field.description());
                subDesc.setStyle("-fx-font-size: 10px; -fx-text-fill: #6b7280;");
                fGroup.getChildren().addAll(fLbl, sRow, subDesc);
            } else if ("textarea".equalsIgnoreCase(field.type())) {
                TextArea ta = new TextArea(val != null ? String.valueOf(val) : "");
                ta.setPromptText(field.description());
                ta.setPrefRowCount(3);
                ta.setWrapText(true);
                ta.getStyleClass().add("text-area");
                fieldControls.put(field.key(), ta);
                fGroup.getChildren().addAll(fLbl, ta);
            } else {
                TextField tf = new TextField(val != null ? String.valueOf(val) : "");
                tf.setPromptText(field.description());
                tf.getStyleClass().add("text-field");
                fieldControls.put(field.key(), tf);
                fGroup.getChildren().addAll(fLbl, tf);
            }

            formBox.getChildren().add(fGroup);
        }

        // Additional Execution Options
        VBox execOptions = new VBox(6);
        CheckBox attachCb = new CheckBox("Attacher le format résultant à cette fiche de livre (multi-formats)");
        attachCb.setSelected(Boolean.parseBoolean(String.valueOf(currentParams.getOrDefault("attach_to_book", "true"))));
        attachCb.setStyle("-fx-text-fill: #d1d5db; -fx-font-size: 12px;");

        CheckBox rememberCb = new CheckBox("Enregistrer ces paramètres comme configuration par défaut");
        rememberCb.setSelected(false);
        rememberCb.setStyle("-fx-text-fill: #9ca3af; -fx-font-size: 11px;");

        execOptions.getChildren().addAll(attachCb, rememberCb);
        formBox.getChildren().addAll(new Separator(), execOptions);

        ScrollPane scroll = new ScrollPane(formBox);
        scroll.setFitToWidth(true);
        scroll.setStyle("-fx-background-color: transparent; -fx-background: transparent;");
        scroll.setPrefHeight(340);
        VBox.setVgrow(scroll, Priority.ALWAYS);
        root.getChildren().add(scroll);

        // Progress bar container (hidden by default)
        VBox progressBox = new VBox(8);
        progressBox.setAlignment(Pos.CENTER);
        progressBox.setVisible(false);
        progressBox.setManaged(false);

        ProgressBar pBar = new ProgressBar();
        pBar.setProgress(ProgressBar.INDETERMINATE_PROGRESS);
        pBar.setMaxWidth(Double.MAX_VALUE);
        pBar.setStyle("-fx-accent: #8b5cf6;");

        Label statusLbl = new Label("Exécution du plugin en cours... Veuillez patienter.");
        statusLbl.setStyle("-fx-text-fill: #c084fc; -fx-font-size: 11px;");
        progressBox.getChildren().addAll(pBar, statusLbl);
        root.getChildren().add(progressBox);

        // Action Buttons
        HBox actionsRow = new HBox(10);
        actionsRow.setAlignment(Pos.CENTER_RIGHT);

        Button cancelBtn = new Button("Annuler");
        cancelBtn.getStyleClass().add("btn-secondary");
        cancelBtn.setOnAction(e -> stage.close());

        Button runBtn = new Button("🚀 Lancer l'exécution");
        runBtn.getStyleClass().add("btn-primary");
        runBtn.setStyle("-fx-background-color: #8b5cf6; -fx-text-fill: white; -fx-font-weight: bold; -fx-cursor: hand; -fx-padding: 8 16;");

        runBtn.setOnAction(e -> {
            // Collect run options
            Map<String, Object> runConfig = new HashMap<>();
            for (Map.Entry<String, Control> entry : fieldControls.entrySet()) {
                Control c = entry.getValue();
                if (c instanceof CheckBox cb) {
                    runConfig.put(entry.getKey(), cb.isSelected());
                } else if (c instanceof ComboBox<?> combo) {
                    runConfig.put(entry.getKey(), combo.getValue());
                } else if (c instanceof Slider sl) {
                    runConfig.put(entry.getKey(), sl.getValue());
                } else if (c instanceof TextInputControl tic) {
                    runConfig.put(entry.getKey(), tic.getText());
                }
            }
            runConfig.put("attach_to_book", attachCb.isSelected());

            if (rememberCb.isSelected()) {
                pluginService.savePluginParams(pluginId, runConfig);
            }

            // Switch to executing state
            scroll.setDisable(true);
            runBtn.setDisable(true);
            cancelBtn.setDisable(true);
            progressBox.setVisible(true);
            progressBox.setManaged(true);

            Thread.ofVirtual().start(() -> {
                try {
                    Book updated = bookService.runTransformationPlugin(book.getId(), pluginId, runConfig);
                    Platform.runLater(() -> {
                        stage.close();
                        if (onCompleted != null) {
                            onCompleted.accept(updated);
                        }
                        boolean isSame = updated.getId().equals(book.getId());
                        String msg = isSame
                                ? "Format transformé et rattaché avec succès à la fiche de « " + updated.getTitle() + " » !"
                                : "Plugin exécuté avec succès !\nNouveau livre créé : " + updated.getTitle();
                        Alert alert = new Alert(Alert.AlertType.INFORMATION, msg, ButtonType.OK);
                        alert.showAndWait();
                    });
                } catch (Exception ex) {
                    Platform.runLater(() -> {
                        scroll.setDisable(false);
                        runBtn.setDisable(false);
                        cancelBtn.setDisable(false);
                        progressBox.setVisible(false);
                        progressBox.setManaged(false);
                        Alert err = new Alert(Alert.AlertType.ERROR, "Erreur lors de l'exécution : " + ex.getMessage(), ButtonType.OK);
                        err.showAndWait();
                    });
                }
            });
        });

        actionsRow.getChildren().addAll(cancelBtn, runBtn);
        root.getChildren().add(actionsRow);

        Scene scene = new Scene(root, 520, 560);
        scene.getStylesheets().add(PluginRunDialog.class.getResource("/styles/theme.css").toExternalForm());
        stage.setScene(scene);
        stage.show();
    }
}
