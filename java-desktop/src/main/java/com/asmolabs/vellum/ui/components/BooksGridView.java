package com.asmolabs.vellum.ui.components;

import com.asmolabs.vellum.domain.entity.Book;
import com.asmolabs.vellum.domain.entity.BookFormat;
import com.asmolabs.vellum.service.BookService;
import com.asmolabs.vellum.ui.events.UiEvents;
import javafx.application.Platform;
import javafx.beans.property.SimpleStringProperty;
import javafx.geometry.Insets;
import javafx.geometry.Pos;
import javafx.scene.control.*;
import javafx.scene.image.Image;
import javafx.scene.image.ImageView;
import javafx.scene.layout.*;
import org.springframework.context.ApplicationEventPublisher;

import java.io.File;
import java.io.FileInputStream;
import java.util.List;

public class BooksGridView extends StackPane {

    private final BookService bookService;
    private final ApplicationEventPublisher eventPublisher;

    private FlowPane gridFlowPane;
    private TableView<Book> tableView;
    private ScrollPane scrollPane;
    private VBox emptyStateBox;

    private boolean isGridView = true;
    private Book selectedBook;

    public BooksGridView(BookService bookService, ApplicationEventPublisher eventPublisher) {
        this.bookService = bookService;
        this.eventPublisher = eventPublisher;

        initUI();
    }

    private void initUI() {
        // Grid View Container
        gridFlowPane = new FlowPane(20, 20);
        gridFlowPane.setPadding(new Insets(10));
        gridFlowPane.setAlignment(Pos.TOP_LEFT);

        scrollPane = new ScrollPane(gridFlowPane);
        scrollPane.getStyleClass().add("books-scroll-pane");
        scrollPane.setFitToWidth(true);

        // List / Table View Container
        tableView = new TableView<>();
        tableView.getStyleClass().add("nav-list-view");

        TableColumn<Book, String> titleCol = new TableColumn<>("Titre");
        titleCol.setCellValueFactory(c -> new SimpleStringProperty(c.getValue().getTitle()));
        titleCol.setPrefWidth(260);

        TableColumn<Book, String> authorCol = new TableColumn<>("Auteur");
        authorCol.setCellValueFactory(c -> new SimpleStringProperty(c.getValue().getAuthor()));
        authorCol.setPrefWidth(180);

        TableColumn<Book, String> formatCol = new TableColumn<>("Formats");
        formatCol.setCellValueFactory(c -> {
            List<String> fmts = c.getValue().getFormats().stream().map(BookFormat::getFormat).map(String::toUpperCase).toList();
            return new SimpleStringProperty(String.join(", ", fmts));
        });
        formatCol.setPrefWidth(100);

        TableColumn<Book, String> dateCol = new TableColumn<>("Date d'ajout");
        dateCol.setCellValueFactory(c -> new SimpleStringProperty(
                c.getValue().getAddedAt() != null ? c.getValue().getAddedAt().substring(0, Math.min(10, c.getValue().getAddedAt().length())) : ""
        ));
        dateCol.setPrefWidth(120);

        tableView.getColumns().addAll(titleCol, authorCol, formatCol, dateCol);
        tableView.getSelectionModel().selectedItemProperty().addListener((obs, oldV, newV) -> {
            if (newV != null) {
                selectedBook = newV;
                eventPublisher.publishEvent(new UiEvents.BookSelected(newV));
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

        // Empty State
        emptyStateBox = new VBox(12);
        emptyStateBox.setAlignment(Pos.CENTER);
        Label emptyIcon = new Label("📖");
        emptyIcon.setStyle("-fx-font-size: 48px; -fx-text-fill: -color-text-muted;");
        Label emptyTitle = new Label("Aucun livre dans la bibliothèque");
        emptyTitle.setStyle("-fx-font-size: 16px; -fx-font-weight: bold; -fx-text-fill: -color-text-primary;");
        Label emptyDesc = new Label("Cliquez sur « Importer un livre » dans la barre latérale pour ajouter vos fichiers EPUB, MOBI ou AZW3.");
        emptyDesc.setStyle("-fx-font-size: 12px; -fx-text-fill: -color-text-muted;");

        emptyStateBox.getChildren().addAll(emptyIcon, emptyTitle, emptyDesc);
        emptyStateBox.setVisible(false);

        this.getChildren().addAll(scrollPane, tableView, emptyStateBox);
        setViewMode(true);
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

            // Update Table
            tableView.getItems().setAll(books);

            // Update Grid
            gridFlowPane.getChildren().clear();
            for (Book book : books) {
                VBox card = createBookCard(book);
                gridFlowPane.getChildren().add(card);
            }
        });
    }

    private VBox createBookCard(Book book) {
        VBox card = new VBox(8);
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

        // Info
        Label titleLabel = new Label(book.getTitle());
        titleLabel.getStyleClass().add("card-title");
        titleLabel.setMaxWidth(160);

        Label authorLabel = new Label(book.getAuthor());
        authorLabel.getStyleClass().add("card-author");
        authorLabel.setMaxWidth(160);

        HBox formatsRow = new HBox(4);
        for (BookFormat fmt : book.getFormats()) {
            Label b = new Label(fmt.getFormat().toUpperCase());
            b.getStyleClass().add("format-badge");
            formatsRow.getChildren().add(b);
        }

        card.getChildren().addAll(coverBox, titleLabel, authorLabel, formatsRow);

        // Selection & Double click
        card.setOnMouseClicked(e -> {
            selectedBook = book;
            // Clear other cards styles
            for (var node : gridFlowPane.getChildren()) {
                node.getStyleClass().remove("book-card-selected");
            }
            card.getStyleClass().add("book-card-selected");

            eventPublisher.publishEvent(new UiEvents.BookSelected(book));

            if (e.getClickCount() == 2) {
                eventPublisher.publishEvent(new UiEvents.BookOpened(book));
            }
        });

        return card;
    }
}
