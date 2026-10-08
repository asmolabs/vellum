package com.asmolabs.vellum.ui.components;

import com.asmolabs.vellum.domain.entity.Book;
import com.asmolabs.vellum.domain.entity.BookFormat;
import com.asmolabs.vellum.service.BookService;
import com.asmolabs.vellum.ui.events.UiEvents;
import javafx.application.Platform;
import javafx.beans.property.SimpleObjectProperty;
import javafx.beans.property.SimpleStringProperty;
import javafx.geometry.Insets;
import javafx.geometry.Pos;
import javafx.scene.control.*;
import javafx.scene.image.Image;
import javafx.scene.image.ImageView;
import javafx.scene.input.DragEvent;
import javafx.scene.input.Dragboard;
import javafx.scene.input.TransferMode;
import javafx.scene.layout.*;
import org.springframework.context.ApplicationEventPublisher;

import java.io.File;
import java.io.FileInputStream;
import java.util.*;

public class BooksGridView extends StackPane {

    private final BookService bookService;
    private final ApplicationEventPublisher eventPublisher;

    private FlowPane gridFlowPane;
    private TableView<Book> tableView;
    private ScrollPane scrollPane;
    private VBox emptyStateBox;
    private HBox bulkActionBar;
    private Label bulkCountLabel;

    private boolean isGridView = true;
    private final Set<Book> selectedBooks = new HashSet<>();
    private final Map<Book, VBox> cardNodes = new HashMap<>();

    public BooksGridView(BookService bookService, ApplicationEventPublisher eventPublisher) {
        this.bookService = bookService;
        this.eventPublisher = eventPublisher;

        initUI();
        initDragAndDrop();
    }

    private void initUI() {
        // 1. Grid View
        gridFlowPane = new FlowPane(20, 20);
        gridFlowPane.setPadding(new Insets(10));
        gridFlowPane.setAlignment(Pos.TOP_LEFT);

        scrollPane = new ScrollPane(gridFlowPane);
        scrollPane.getStyleClass().add("books-scroll-pane");
        scrollPane.setFitToWidth(true);

        // 2. Table View
        tableView = new TableView<>();
        tableView.getStyleClass().add("table-view");
        tableView.getSelectionModel().setSelectionMode(SelectionMode.MULTIPLE);

        TableColumn<Book, Book> titleCol = new TableColumn<>("Livre");
        titleCol.setCellValueFactory(c -> new SimpleObjectProperty<>(c.getValue()));
        titleCol.setCellFactory(col -> new TableCell<>() {
            @Override
            protected void updateItem(Book book, boolean empty) {
                super.updateItem(book, empty);
                if (empty || book == null) {
                    setGraphic(null);
                    setText(null);
                } else {
                    HBox cellBox = new HBox(12);
                    cellBox.setAlignment(Pos.CENTER_LEFT);

                    ImageView thumb = new ImageView();
                    thumb.setFitWidth(32);
                    thumb.setFitHeight(46);
                    thumb.setPreserveRatio(true);

                    File coverFile = bookService.getCoverFile(book);
                    if (coverFile != null && coverFile.exists()) {
                        try {
                            thumb.setImage(new Image(new FileInputStream(coverFile)));
                        } catch (Exception ignored) {
                            thumb.setImage(null);
                        }
                    }

                    VBox textBox = new VBox(2);
                    Label titleLbl = new Label(book.getTitle());
                    titleLbl.setStyle("-fx-font-weight: bold; -fx-text-fill: #f3f4f6; -fx-font-size: 13px;");
                    Label authorLbl = new Label(book.getAuthor());
                    authorLbl.setStyle("-fx-text-fill: #9ca3af; -fx-font-size: 11px;");

                    textBox.getChildren().addAll(titleLbl, authorLbl);
                    cellBox.getChildren().addAll(thumb, textBox);
                    setGraphic(cellBox);
                    setText(null);
                }
            }
        });
        titleCol.setPrefWidth(320);

        TableColumn<Book, Book> progressCol = new TableColumn<>("Progression");
        progressCol.setCellValueFactory(c -> new SimpleObjectProperty<>(c.getValue()));
        progressCol.setCellFactory(col -> new TableCell<>() {
            @Override
            protected void updateItem(Book book, boolean empty) {
                super.updateItem(book, empty);
                if (empty || book == null) {
                    setGraphic(null);
                    setText(null);
                } else {
                    if (book.isFinished()) {
                        Label finLbl = new Label("✓ Terminé");
                        finLbl.setStyle("-fx-text-fill: #10b981; -fx-font-weight: bold; -fx-font-size: 11px;");
                        setGraphic(finLbl);
                    } else if (book.getProgressPercent() > 0) {
                        VBox box = new VBox(3);
                        box.setAlignment(Pos.CENTER_LEFT);
                        ProgressBar pb = new ProgressBar(book.getProgressPercent() / 100.0);
                        pb.setPrefWidth(100);
                        pb.setStyle("-fx-accent: #8b5cf6;");
                        Label pctLbl = new Label((int)book.getProgressPercent() + "%");
                        pctLbl.setStyle("-fx-font-size: 10px; -fx-text-fill: #a78bfa;");
                        box.getChildren().addAll(pb, pctLbl);
                        setGraphic(box);
                    } else {
                        Label unread = new Label("Non lu");
                        unread.setStyle("-fx-text-fill: #6b7280; -fx-font-size: 11px;");
                        setGraphic(unread);
                    }
                    setText(null);
                }
            }
        });
        progressCol.setPrefWidth(130);

        TableColumn<Book, Book> tagsCol = new TableColumn<>("Tags");
        tagsCol.setCellValueFactory(c -> new SimpleObjectProperty<>(c.getValue()));
        tagsCol.setCellFactory(col -> new TableCell<>() {
            @Override
            protected void updateItem(Book book, boolean empty) {
                super.updateItem(book, empty);
                if (empty || book == null) {
                    setGraphic(null);
                    setText(null);
                } else {
                    HBox box = new HBox(4);
                    box.setAlignment(Pos.CENTER_LEFT);
                    for (var t : book.getTags()) {
                        Label lbl = new Label(t.getName());
                        lbl.getStyleClass().add("tag-badge");
                        lbl.setStyle("-fx-font-size: 10px; -fx-padding: 2 6 2 6;");
                        box.getChildren().add(lbl);
                    }
                    setGraphic(box);
                    setText(null);
                }
            }
        });
        tagsCol.setPrefWidth(160);

        TableColumn<Book, Book> formatCol = new TableColumn<>("Formats");
        formatCol.setCellValueFactory(c -> new SimpleObjectProperty<>(c.getValue()));
        formatCol.setCellFactory(col -> new TableCell<>() {
            @Override
            protected void updateItem(Book book, boolean empty) {
                super.updateItem(book, empty);
                if (empty || book == null) {
                    setGraphic(null);
                    setText(null);
                } else {
                    HBox box = new HBox(4);
                    box.setAlignment(Pos.CENTER_LEFT);
                    for (BookFormat fmt : book.getFormats()) {
                        Label b = new Label(fmt.getFormat().toUpperCase());
                        b.getStyleClass().add("format-badge");
                        box.getChildren().add(b);
                    }
                    setGraphic(box);
                    setText(null);
                }
            }
        });
        formatCol.setPrefWidth(110);

        tableView.getColumns().addAll(titleCol, progressCol, tagsCol, formatCol);

        tableView.getSelectionModel().getSelectedItems().addListener((javafx.collections.ListChangeListener<Book>) c -> {
            selectedBooks.clear();
            selectedBooks.addAll(tableView.getSelectionModel().getSelectedItems());
            updateBulkActionBar();
            if (selectedBooks.size() == 1) {
                eventPublisher.publishEvent(new UiEvents.BookSelected(selectedBooks.iterator().next()));
            }
        });

        tableView.setRowFactory(tv -> {
            TableRow<Book> row = new TableRow<>();
            row.setOnMouseClicked(event -> {
                if (event.getClickCount() == 2 && (!row.isEmpty())) {
                    eventPublisher.publishEvent(new UiEvents.BookOpened(row.getItem()));
                }
            });
            return row;
        });

        // 3. Empty State
        emptyStateBox = new VBox(12);
        emptyStateBox.setAlignment(Pos.CENTER);
        Label emptyIcon = new Label("📖");
        emptyIcon.setStyle("-fx-font-size: 48px; -fx-text-fill: -color-text-muted;");
        Label emptyTitle = new Label("Aucun livre dans la bibliothèque");
        emptyTitle.setStyle("-fx-font-size: 16px; -fx-font-weight: bold; -fx-text-fill: -color-text-primary;");
        Label emptyDesc = new Label("Glissez-déposez vos fichiers EPUB, PDF, CBZ ou cliquez sur « Importer ».");
        emptyDesc.setStyle("-fx-font-size: 12px; -fx-text-fill: -color-text-muted;");
        emptyStateBox.getChildren().addAll(emptyIcon, emptyTitle, emptyDesc);
        emptyStateBox.setVisible(false);

        // 4. Floating Bulk Actions Bar
        bulkActionBar = new HBox(14);
        bulkActionBar.setAlignment(Pos.CENTER);
        bulkActionBar.setPadding(new Insets(10, 20, 10, 20));
        bulkActionBar.setStyle("-fx-background-color: #1a153b; -fx-border-color: #8b5cf6; -fx-border-radius: 24; -fx-background-radius: 24; -fx-effect: dropshadow(three-pass-box, rgba(0,0,0,0.6), 16, 0, 0, 4);");
        bulkActionBar.setMaxWidth(560);
        bulkActionBar.setMaxHeight(48);
        StackPane.setAlignment(bulkActionBar, Pos.BOTTOM_CENTER);
        StackPane.setMargin(bulkActionBar, new Insets(0, 0, 24, 0));
        bulkActionBar.setVisible(false);

        bulkCountLabel = new Label("0 livres sélectionnés");
        bulkCountLabel.setStyle("-fx-text-fill: #ffffff; -fx-font-weight: bold; -fx-font-size: 12px;");

        Button bulkTagBtn = new Button("＋ Ajouter un tag");
        bulkTagBtn.getStyleClass().add("btn-secondary");
        bulkTagBtn.setStyle("-fx-font-size: 11px; -fx-padding: 5 10 5 10;");
        bulkTagBtn.setOnAction(e -> handleBulkTag());

        Button bulkDelBtn = new Button("🗑 Supprimer");
        bulkDelBtn.getStyleClass().add("btn-danger");
        bulkDelBtn.setStyle("-fx-font-size: 11px; -fx-padding: 5 10 5 10;");
        bulkDelBtn.setOnAction(e -> handleBulkDelete());

        Button clearSelBtn = new Button("✕");
        clearSelBtn.getStyleClass().add("icon-btn");
        clearSelBtn.setOnAction(e -> clearSelection());

        bulkActionBar.getChildren().addAll(bulkCountLabel, bulkTagBtn, bulkDelBtn, clearSelBtn);

        this.getChildren().addAll(scrollPane, tableView, emptyStateBox, bulkActionBar);
        setViewMode(true);
    }

    private void initDragAndDrop() {
        this.setOnDragOver(event -> {
            if (event.getGestureSource() != this && event.getDragboard().hasFiles()) {
                event.acceptTransferModes(TransferMode.COPY_OR_MOVE);
                this.setStyle("-fx-border-color: #8b5cf6; -fx-border-width: 2; -fx-border-style: dashed;");
            }
            event.consume();
        });

        this.setOnDragExited(event -> {
            this.setStyle("");
            event.consume();
        });

        this.setOnDragDropped((DragEvent event) -> {
            Dragboard db = event.getDragboard();
            boolean success = false;
            if (db.hasFiles()) {
                List<File> files = db.getFiles();
                Thread.ofVirtual().start(() -> {
                    for (File f : files) {
                        try {
                            bookService.importBook(f);
                        } catch (Exception ex) {
                            Platform.runLater(() -> {
                                Alert a = new Alert(Alert.AlertType.ERROR, "Erreur import : " + f.getName() + " — " + ex.getMessage(), ButtonType.OK);
                                a.showAndWait();
                            });
                        }
                    }
                    eventPublisher.publishEvent(new UiEvents.LibraryChanged());
                });
                success = true;
            }
            this.setStyle("");
            event.setDropCompleted(success);
            event.consume();
        });
    }

    public void setViewMode(boolean grid) {
        this.isGridView = grid;
        scrollPane.setVisible(grid);
        scrollPane.setManaged(grid);
        tableView.setVisible(!grid);
        tableView.setManaged(!grid);
    }

    public void updateBooks(List<Book> books) {
        Platform.runLater(() -> {
            boolean empty = (books == null || books.isEmpty());
            emptyStateBox.setVisible(empty);
            scrollPane.setVisible(!empty && isGridView);
            tableView.setVisible(!empty && !isGridView);

            if (empty) return;

            tableView.getItems().setAll(books);

            gridFlowPane.getChildren().clear();
            cardNodes.clear();
            for (Book book : books) {
                VBox card = createBookCard(book);
                cardNodes.put(book, card);
                gridFlowPane.getChildren().add(card);
            }
        });
    }

    private VBox createBookCard(Book book) {
        VBox card = new VBox(6);
        card.getStyleClass().add("book-card");
        card.setPrefWidth(180);
        card.setMaxWidth(180);

        // Cover Container
        StackPane coverBox = new StackPane();
        coverBox.setPrefSize(160, 240);
        coverBox.setMaxSize(160, 240);
        coverBox.setStyle("-fx-background-color: #1e1e28; -fx-background-radius: 8;");

        File coverFile = bookService.getCoverFile(book);
        if (coverFile != null && coverFile.exists()) {
            try {
                ImageView iv = new ImageView(new Image(new FileInputStream(coverFile)));
                iv.setFitWidth(160);
                iv.setFitHeight(240);
                iv.setPreserveRatio(true);
                coverBox.getChildren().add(iv);
            } catch (Exception ignored) {
                coverBox.getChildren().add(new Label("📖"));
            }
        } else {
            VBox ph = new VBox(6);
            ph.setAlignment(Pos.CENTER);
            Label ico = new Label("📚");
            ico.setStyle("-fx-font-size: 28px; -fx-text-fill: -color-accent;");
            Label txt = new Label(book.getTitle());
            txt.setStyle("-fx-font-size: 10px; -fx-text-fill: -color-text-muted; -fx-text-alignment: center;");
            txt.setWrapText(true);
            ph.getChildren().addAll(ico, txt);
            coverBox.getChildren().add(ph);
        }

        // Completion indicator on cover top-right
        if (book.isFinished()) {
            Label badgeFin = new Label("✓ Terminé");
            badgeFin.setStyle("-fx-background-color: rgba(16, 185, 129, 0.9); -fx-text-fill: white; -fx-font-size: 10px; -fx-font-weight: bold; -fx-padding: 3 7 3 7; -fx-background-radius: 10;");
            StackPane.setAlignment(badgeFin, Pos.TOP_RIGHT);
            StackPane.setMargin(badgeFin, new Insets(6));
            coverBox.getChildren().add(badgeFin);
        }

        // Title and Author
        Label titleLabel = new Label(book.getTitle());
        titleLabel.getStyleClass().add("card-title");
        titleLabel.setMaxWidth(160);

        Label authorLabel = new Label(book.getAuthor());
        authorLabel.getStyleClass().add("card-author");
        authorLabel.setMaxWidth(160);

        // Progress bar on card if in progress
        VBox progBox = null;
        if (book.getProgressPercent() > 0 && !book.isFinished()) {
            ProgressBar pb = new ProgressBar(book.getProgressPercent() / 100.0);
            pb.setPrefWidth(160);
            pb.setMaxHeight(4);
            pb.setStyle("-fx-accent: #8b5cf6;");
            progBox = new VBox(pb);
        }

        // Format Badges Row
        HBox formatsRow = new HBox(4);
        for (BookFormat fmt : book.getFormats()) {
            Label b = new Label(fmt.getFormat().toUpperCase());
            b.getStyleClass().add("format-badge");
            formatsRow.getChildren().add(b);
        }

        if (progBox != null) {
            card.getChildren().addAll(coverBox, titleLabel, authorLabel, progBox, formatsRow);
        } else {
            card.getChildren().addAll(coverBox, titleLabel, authorLabel, formatsRow);
        }

        // Selection & Click
        card.setOnMouseClicked(e -> {
            if (e.isControlDown() || e.isShortcutDown() || e.isShiftDown()) {
                // Multi-select toggle
                if (selectedBooks.contains(book)) {
                    selectedBooks.remove(book);
                    card.getStyleClass().remove("book-card-selected");
                } else {
                    selectedBooks.add(book);
                    card.getStyleClass().add("book-card-selected");
                }
            } else {
                // Single select
                selectedBooks.clear();
                for (var node : gridFlowPane.getChildren()) {
                    node.getStyleClass().remove("book-card-selected");
                }
                selectedBooks.add(book);
                card.getStyleClass().add("book-card-selected");
                eventPublisher.publishEvent(new UiEvents.BookSelected(book));
            }

            updateBulkActionBar();

            if (e.getClickCount() == 2) {
                eventPublisher.publishEvent(new UiEvents.BookOpened(book));
            }
        });

        return card;
    }

    private void updateBulkActionBar() {
        if (selectedBooks.size() > 1) {
            bulkCountLabel.setText(selectedBooks.size() + " livres sélectionnés");
            bulkActionBar.setVisible(true);
        } else {
            bulkActionBar.setVisible(false);
        }
    }

    private void clearSelection() {
        selectedBooks.clear();
        for (var c : cardNodes.values()) {
            c.getStyleClass().remove("book-card-selected");
        }
        tableView.getSelectionModel().clearSelection();
        updateBulkActionBar();
    }

    private void handleBulkTag() {
        TextInputDialog dialog = new TextInputDialog();
        dialog.setTitle("Ajout de tag en masse");
        dialog.setHeaderText("Ajouter un tag à " + selectedBooks.size() + " livres sélectionnés");
        dialog.setContentText("Nom du tag :");
        dialog.showAndWait().ifPresent(tagName -> {
            if (!tagName.trim().isBlank()) {
                List<String> ids = selectedBooks.stream().map(Book::getId).toList();
                bookService.bulkAddTag(ids, tagName.trim());
                clearSelection();
                eventPublisher.publishEvent(new UiEvents.LibraryChanged());
            }
        });
    }

    private void handleBulkDelete() {
        Alert confirm = new Alert(
                Alert.AlertType.CONFIRMATION,
                "Voulez-vous vraiment supprimer définitivement ces " + selectedBooks.size() + " livres ?",
                ButtonType.YES, ButtonType.NO
        );
        confirm.showAndWait().ifPresent(res -> {
            if (res == ButtonType.YES) {
                List<String> ids = selectedBooks.stream().map(Book::getId).toList();
                bookService.bulkDeleteBooks(ids);
                clearSelection();
                eventPublisher.publishEvent(new UiEvents.LibraryChanged());
            }
        });
    }
}
