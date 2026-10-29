package com.asmolabs.vellum.ui.components;

import com.asmolabs.vellum.config.AppProperties;
import com.asmolabs.vellum.domain.entity.Book;
import com.asmolabs.vellum.service.BookService;
import com.asmolabs.vellum.service.LitestreamService;
import com.asmolabs.vellum.service.PluginService;
import com.asmolabs.vellum.ui.events.UiEvents;
import javafx.application.Platform;
import javafx.scene.Node;
import javafx.scene.Scene;
import javafx.scene.control.Alert;
import javafx.scene.control.ButtonBase;
import javafx.scene.control.ButtonType;
import javafx.scene.control.ComboBoxBase;
import javafx.scene.control.Slider;
import javafx.scene.control.TextInputControl;
import javafx.scene.input.KeyCode;
import javafx.scene.input.KeyEvent;
import javafx.scene.layout.BorderPane;
import javafx.scene.layout.VBox;
import org.springframework.context.ApplicationEventPublisher;
import org.springframework.context.event.EventListener;
import org.springframework.context.annotation.Lazy;
import org.springframework.stereotype.Component;

import java.util.List;
import java.util.Optional;

@Component
@Lazy
public class MainView extends BorderPane {

    private final BookService bookService;
    private final LitestreamService litestreamService;
    private final PluginService pluginService;
    private final AppProperties appProperties;
    private final ApplicationEventPublisher eventPublisher;

    private SidebarView sidebarView;
    private HeaderView headerView;
    private BooksGridView booksGridView;
    private DetailDrawerView detailDrawerView;

    private String currentSearch = null;
    private String currentTag = null;
    private String currentStatus = "ALL";

    public MainView(
            BookService bookService,
            LitestreamService litestreamService,
            PluginService pluginService,
            AppProperties appProperties,
            ApplicationEventPublisher eventPublisher
    ) {
        this.bookService = bookService;
        this.litestreamService = litestreamService;
        this.pluginService = pluginService;
        this.appProperties = appProperties;
        this.eventPublisher = eventPublisher;

        initUI();
        refreshLibrary();
    }

    private void initUI() {
        this.getStyleClass().add("app-container");

        // 1. Sidebar on Left
        sidebarView = new SidebarView(bookService, appProperties, eventPublisher);
        this.setLeft(sidebarView);

        // 2. Main Content Area (Header + Books Grid)
        booksGridView = new BooksGridView(bookService, eventPublisher);
        headerView = new HeaderView(litestreamService, eventPublisher, isGrid -> booksGridView.setViewMode(isGrid));

        VBox centerBox = new VBox(headerView, booksGridView);
        VBox.setVgrow(booksGridView, javafx.scene.layout.Priority.ALWAYS);
        this.setCenter(centerBox);

        // 3. Detail Drawer on Right
        detailDrawerView = new DetailDrawerView(bookService, pluginService, eventPublisher);
        this.setRight(detailDrawerView);

        // 4. Global Keyboard Shortcuts
        this.sceneProperty().addListener((obs, oldScene, newScene) -> {
            if (newScene != null) {
                setupKeyboardShortcuts(newScene);
            }
        });
        if (this.getScene() != null) {
            setupKeyboardShortcuts(this.getScene());
        }
    }

    private void setupKeyboardShortcuts(Scene scene) {
        scene.addEventFilter(KeyEvent.KEY_PRESSED, event -> {
            // Cmd+F or Ctrl+F -> Focus Search
            if (event.isShortcutDown() && event.getCode() == KeyCode.F) {
                headerView.focusSearchField();
                event.consume();
                return;
            }

            // Cmd+, or Ctrl+, -> Open Settings
            if (event.isShortcutDown() && event.getCode() == KeyCode.COMMA) {
                eventPublisher.publishEvent(new UiEvents.OpenSettingsRequested());
                event.consume();
                return;
            }

            // If focus is in a text input or interactive control, let it handle keys
            Node focusOwner = scene.getFocusOwner();
            if (isInputOrControlFocused(focusOwner)) {
                return;
            }

            // Escape -> Close drawer
            if (event.getCode() == KeyCode.ESCAPE) {
                if (detailDrawerView.isVisible()) {
                    detailDrawerView.hideDrawer();
                    event.consume();
                    return;
                }
            }

            // Space or Enter -> Open selected book in reader
            if (event.getCode() == KeyCode.SPACE || event.getCode() == KeyCode.ENTER) {
                Book book = detailDrawerView.getCurrentBook();
                if (book != null) {
                    eventPublisher.publishEvent(new UiEvents.BookOpened(book));
                    event.consume();
                    return;
                }
            }

            // Delete or Backspace -> Delete selected book
            if (event.getCode() == KeyCode.DELETE || event.getCode() == KeyCode.BACK_SPACE) {
                Book book = detailDrawerView.getCurrentBook();
                if (book != null) {
                    eventPublisher.publishEvent(new UiEvents.BookDeleteRequested(book));
                    event.consume();
                }
            }
        });
    }

    private boolean isInputOrControlFocused(Node node) {
        if (node == null) return false;
        return node instanceof TextInputControl
                || node instanceof ButtonBase
                || node instanceof ComboBoxBase
                || node instanceof Slider;
    }

    public void refreshLibrary() {
        List<Book> books = bookService.getBooks(currentSearch, currentTag);
        if (currentStatus != null && !currentStatus.equalsIgnoreCase("ALL")) {
            books = books.stream().filter(b -> {
                double progress = b.getProgressPercent();
                boolean finished = b.isFinished();
                return switch (currentStatus.toUpperCase()) {
                    case "READING" -> !finished && progress > 0;
                    case "FINISHED" -> finished;
                    case "UNREAD" -> !finished && progress <= 0;
                    default -> true;
                };
            }).toList();
        }
        booksGridView.updateBooks(books);
        sidebarView.refresh();
        headerView.updateLitestreamStatus();
    }

    @EventListener
    public void onFilterChanged(UiEvents.FilterChanged event) {
        if (event.search() != null) {
            this.currentSearch = event.search();
        }
        if (event.tag() != null) {
            this.currentTag = event.tag();
        }
        if (event.status() != null) {
            this.currentStatus = event.status();
        }
        refreshLibrary();
    }

    @EventListener
    public void onLibraryChanged(UiEvents.LibraryChanged event) {
        refreshLibrary();
    }

    @EventListener
    public void onBookSelected(UiEvents.BookSelected event) {
        detailDrawerView.showBook(event.book());
    }

    @EventListener
    public void onBookOpened(UiEvents.BookOpened event) {
        Platform.runLater(() -> ReaderDialog.show(this.getScene().getWindow(), event.book(), bookService));
    }

    @EventListener
    public void onBookEditRequested(UiEvents.BookEditRequested event) {
        Platform.runLater(() -> EditBookDialog.show(this.getScene().getWindow(), event.book(), bookService, pluginService, updated -> {
            refreshLibrary();
            detailDrawerView.showBook(updated);
        }));
    }

    @EventListener
    public void onBookDeleteRequested(UiEvents.BookDeleteRequested event) {
        Platform.runLater(() -> {
            Alert confirm = new Alert(
                    Alert.AlertType.CONFIRMATION,
                    "Êtes-vous sûr de vouloir supprimer définitivement « " + event.book().getTitle() + " » et ses fichiers ?",
                    ButtonType.YES, ButtonType.NO
            );
            confirm.setTitle("Confirmer la suppression");
            confirm.setHeaderText("Suppression du livre");

            Optional<ButtonType> result = confirm.showAndWait();
            if (result.isPresent() && result.get() == ButtonType.YES) {
                try {
                    bookService.deleteBook(event.book().getId());
                    detailDrawerView.hideDrawer();
                    refreshLibrary();
                } catch (Exception e) {
                    Alert err = new Alert(Alert.AlertType.ERROR, "Erreur lors de la suppression : " + e.getMessage(), ButtonType.OK);
                    err.showAndWait();
                }
            }
        });
    }

    @EventListener
    public void onBookPluginRequested(UiEvents.BookPluginRequested event) {
        Thread.ofVirtual().start(() -> {
            try {
                Book transformed = bookService.runTransformationPlugin(event.book().getId(), event.pluginId());
                Platform.runLater(() -> {
                    refreshLibrary();
                    detailDrawerView.showBook(transformed);
                    boolean isSameBook = transformed.getId().equals(event.book().getId());
                    String msg = isSameBook
                            ? "Format converti et rattaché avec succès à la fiche de « " + transformed.getTitle() + " » !"
                            : "Plugin exécuté avec succès !\nNouveau livre créé : " + transformed.getTitle();
                    Alert info = new Alert(Alert.AlertType.INFORMATION, msg, ButtonType.OK);
                    info.showAndWait();
                });
            } catch (Exception e) {
                Platform.runLater(() -> {
                    Alert err = new Alert(Alert.AlertType.ERROR, "Erreur lors de l'exécution du plugin : " + e.getMessage(), ButtonType.OK);
                    err.showAndWait();
                });
            }
        });
    }

    @EventListener
    public void onOpenSettingsRequested(UiEvents.OpenSettingsRequested event) {
        Platform.runLater(() -> SettingsDialog.show(this.getScene().getWindow(), appProperties, litestreamService, pluginService, this::refreshLibrary));
    }

    @EventListener
    public void onOpenStatsRequested(UiEvents.OpenStatsRequested event) {
        Platform.runLater(() -> StatsDialog.show(this.getScene().getWindow(), bookService));
    }
}
