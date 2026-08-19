package com.asmolabs.vellum.ui.components;

import com.asmolabs.vellum.domain.entity.Book;
import com.asmolabs.vellum.domain.entity.Tag;
import com.asmolabs.vellum.service.BookService;
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
import javafx.stage.Window;

import java.io.File;
import java.io.FileInputStream;
import java.util.ArrayList;
import java.util.List;
import java.util.function.Consumer;

public class EditBookDialog {

    public static void show(Window owner, Book book, BookService bookService, Consumer<Book> onSaved) {
        Stage stage = new Stage();
        stage.initOwner(owner);
        stage.initModality(Modality.APPLICATION_MODAL);
        stage.setTitle("Modifier les métadonnées — " + book.getTitle());

        BorderPane root = new BorderPane();
        root.getStyleClass().add("dialog-container");
        root.setPadding(new Insets(24));

        GridPane grid = new GridPane();
        grid.setHgap(16);
        grid.setVgap(14);

        ColumnConstraints col1 = new ColumnConstraints(120);
        ColumnConstraints col2 = new ColumnConstraints(380);
        col2.setHgrow(Priority.ALWAYS);
        grid.getColumnConstraints().addAll(col1, col2);

        // Fields
        TextField titleField = new TextField(book.getTitle() != null ? book.getTitle() : "");
        titleField.getStyleClass().add("text-field");

        TextField authorField = new TextField(book.getAuthor() != null ? book.getAuthor() : "");
        authorField.getStyleClass().add("text-field");

        TextField publisherField = new TextField(book.getPublisher() != null ? book.getPublisher() : "");
        publisherField.getStyleClass().add("text-field");

        TextField dateField = new TextField(book.getPublishDate() != null ? book.getPublishDate() : "");
        dateField.getStyleClass().add("text-field");

        TextArea descArea = new TextArea(book.getDescription() != null ? book.getDescription() : "");
        descArea.setPrefRowCount(4);
        descArea.setWrapText(true);
        descArea.getStyleClass().add("text-area");

        // Tags List & Tag input
        List<String> currentTags = new ArrayList<>(book.getTags().stream().map(Tag::getName).toList());
        FlowPane tagsFlow = new FlowPane(6, 6);
        TextField newTagField = new TextField();
        newTagField.setPromptText("Nouveau tag (Appuyer sur Entrée)...");
        newTagField.getStyleClass().add("text-field");

        Runnable refreshTags = () -> {
            tagsFlow.getChildren().clear();
            for (String t : currentTags) {
                HBox tagPill = new HBox(4);
                tagPill.setAlignment(Pos.CENTER);
                tagPill.getStyleClass().add("tag-badge");
                Label tLabel = new Label(t);
                tLabel.setStyle("-fx-text-fill: #c084fc;");
                Button remBtn = new Button("✕");
                remBtn.setStyle("-fx-background-color: transparent; -fx-padding: 0 0 0 4; -fx-text-fill: #e0e0e0; -fx-cursor: hand; -fx-font-size: 10px;");
                remBtn.setOnAction(e -> {
                    currentTags.remove(t);
                    tagsFlow.getChildren().remove(tagPill);
                });
                tagPill.getChildren().addAll(tLabel, remBtn);
                tagsFlow.getChildren().add(tagPill);
            }
        };
        refreshTags.run();

        newTagField.setOnAction(e -> {
            String val = newTagField.getText().trim();
            if (!val.isBlank() && !currentTags.contains(val)) {
                currentTags.add(val);
                newTagField.clear();
                refreshTags.run();
            }
        });

        VBox tagsBox = new VBox(8, tagsFlow, newTagField);

        // Cover section
        ImageView coverPreview = new ImageView();
        coverPreview.setFitWidth(90);
        coverPreview.setFitHeight(135);
        coverPreview.setPreserveRatio(true);

        File currentCover = bookService.getCoverFile(book);
        if (currentCover != null && currentCover.exists()) {
            try {
                coverPreview.setImage(new Image(new FileInputStream(currentCover)));
            } catch (Exception ignored) {}
        }

        final File[] selectedNewCover = {null};
        Button changeCoverBtn = new Button("Changer la couverture...");
        changeCoverBtn.getStyleClass().add("btn-secondary");
        changeCoverBtn.setOnAction(e -> {
            FileChooser fc = new FileChooser();
            fc.setTitle("Sélectionner une nouvelle image de couverture");
            fc.getExtensionFilters().add(new FileChooser.ExtensionFilter("Images", "*.jpg", "*.jpeg", "*.png", "*.webp"));
            File f = fc.showOpenDialog(stage);
            if (f != null) {
                selectedNewCover[0] = f;
                try {
                    coverPreview.setImage(new Image(new FileInputStream(f)));
                } catch (Exception ignored) {}
            }
        });

        HBox coverBox = new HBox(12, coverPreview, changeCoverBtn);
        coverBox.setAlignment(Pos.CENTER_LEFT);

        // Assemble Grid
        int row = 0;
        grid.add(new Label("Titre :"), 0, row);
        grid.add(titleField, 1, row++);

        grid.add(new Label("Auteur :"), 0, row);
        grid.add(authorField, 1, row++);

        grid.add(new Label("Éditeur :"), 0, row);
        grid.add(publisherField, 1, row++);

        grid.add(new Label("Date de pub. :"), 0, row);
        grid.add(dateField, 1, row++);

        grid.add(new Label("Description :"), 0, row);
        grid.add(descArea, 1, row++);

        grid.add(new Label("Tags / Genres :"), 0, row);
        grid.add(tagsBox, 1, row++);

        grid.add(new Label("Couverture :"), 0, row);
        grid.add(coverBox, 1, row++);

        // Buttons Bar
        HBox actions = new HBox(12);
        actions.setAlignment(Pos.CENTER_RIGHT);
        actions.setPadding(new Insets(16, 0, 0, 0));

        Button cancelBtn = new Button("Annuler");
        cancelBtn.getStyleClass().add("btn-secondary");
        cancelBtn.setOnAction(e -> stage.close());

        Button saveBtn = new Button("Enregistrer les modifications");
        saveBtn.getStyleClass().add("btn-primary");
        saveBtn.setOnAction(e -> {
            try {
                Book updated = bookService.updateBook(
                        book.getId(),
                        titleField.getText().trim(),
                        authorField.getText().trim(),
                        publisherField.getText().trim(),
                        dateField.getText().trim(),
                        descArea.getText().trim(),
                        currentTags
                );

                if (selectedNewCover[0] != null) {
                    updated = bookService.updateCover(updated.getId(), selectedNewCover[0]);
                }

                if (onSaved != null) {
                    onSaved.accept(updated);
                }
                stage.close();
            } catch (Exception ex) {
                Alert alert = new Alert(Alert.AlertType.ERROR, "Erreur lors de la sauvegarde : " + ex.getMessage(), ButtonType.OK);
                alert.showAndWait();
            }
        });

        actions.getChildren().addAll(cancelBtn, saveBtn);

        root.setCenter(grid);
        root.setBottom(actions);

        Scene scene = new Scene(root, 620, 680);
        scene.getStylesheets().add(EditBookDialog.class.getResource("/styles/theme.css").toExternalForm());
        stage.setScene(scene);
        stage.show();
    }
}
