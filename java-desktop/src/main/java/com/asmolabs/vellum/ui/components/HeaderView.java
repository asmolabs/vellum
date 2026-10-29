package com.asmolabs.vellum.ui.components;

import com.asmolabs.vellum.service.LitestreamService;
import com.asmolabs.vellum.ui.events.UiEvents;
import javafx.application.Platform;
import javafx.geometry.Pos;
import javafx.scene.control.Button;
import javafx.scene.control.Label;
import javafx.scene.control.TextField;
import javafx.scene.control.ToggleButton;
import javafx.scene.control.ToggleGroup;
import javafx.scene.layout.HBox;
import javafx.scene.layout.Priority;
import org.springframework.context.ApplicationEventPublisher;

import java.util.function.Consumer;

public class HeaderView extends HBox {

    private final LitestreamService litestreamService;
    private final ApplicationEventPublisher eventPublisher;
    private final Consumer<Boolean> onViewModeChanged;

    private TextField searchField;
    private Label litestreamBadge;
    private boolean isGridView = true;

    public HeaderView(LitestreamService litestreamService, ApplicationEventPublisher eventPublisher, Consumer<Boolean> onViewModeChanged) {
        this.litestreamService = litestreamService;
        this.eventPublisher = eventPublisher;
        this.onViewModeChanged = onViewModeChanged;

        initUI();
    }

    private void initUI() {
        this.getStyleClass().add("header-bar");
        this.setSpacing(16);
        this.setAlignment(Pos.CENTER_LEFT);

        // Search Bar
        searchField = new TextField();
        searchField.setPromptText("🔍  Rechercher par titre, auteur, description...");
        searchField.getStyleClass().add("search-field");
        searchField.setPrefWidth(420);
        HBox.setHgrow(searchField, Priority.ALWAYS);

        searchField.textProperty().addListener((obs, oldV, newV) -> {
            eventPublisher.publishEvent(new UiEvents.FilterChanged(newV, null));
        });

        // Status Filter Chips
        HBox statusFilters = new HBox(4);
        statusFilters.setAlignment(Pos.CENTER_LEFT);

        ToggleGroup statusGroup = new ToggleGroup();
        ToggleButton allBtn = createStatusFilterButton("Tous", "ALL", statusGroup, true);
        ToggleButton readingBtn = createStatusFilterButton("📖 En cours", "READING", statusGroup, false);
        ToggleButton finishedBtn = createStatusFilterButton("✓ Terminés", "FINISHED", statusGroup, false);
        ToggleButton unreadBtn = createStatusFilterButton("⏳ À lire", "UNREAD", statusGroup, false);
        statusFilters.getChildren().addAll(allBtn, readingBtn, finishedBtn, unreadBtn);

        // Litestream Backup Status Badge
        litestreamBadge = new Label("⚡ Litestream");
        litestreamBadge.setStyle("-fx-background-color: rgba(16, 185, 129, 0.15); -fx-border-color: rgba(16, 185, 129, 0.4); -fx-border-radius: 12; -fx-background-radius: 12; -fx-text-fill: #34d399; -fx-font-size: 11px; -fx-padding: 4 10 4 10; -fx-cursor: hand;");
        litestreamBadge.setOnMouseClicked(e -> eventPublisher.publishEvent(new UiEvents.OpenSettingsRequested()));

        // View Mode Switcher
        HBox viewControls = new HBox(4);
        Button gridBtn = new Button("⊞ Grille");
        gridBtn.getStyleClass().add("icon-btn");
        gridBtn.setStyle("-fx-border-color: -color-accent;");

        Button listBtn = new Button("☰ Liste");
        listBtn.getStyleClass().add("icon-btn");

        gridBtn.setOnAction(e -> {
            isGridView = true;
            gridBtn.setStyle("-fx-border-color: -color-accent;");
            listBtn.setStyle("-fx-border-color: -color-border;");
            if (onViewModeChanged != null) onViewModeChanged.accept(true);
        });

        listBtn.setOnAction(e -> {
            isGridView = false;
            listBtn.setStyle("-fx-border-color: -color-accent;");
            gridBtn.setStyle("-fx-border-color: -color-border;");
            if (onViewModeChanged != null) onViewModeChanged.accept(false);
        });

        viewControls.getChildren().addAll(gridBtn, listBtn);

        // Device Sync Button
        Button deviceBtn = new Button("📱 Liseuse");
        deviceBtn.getStyleClass().add("btn-secondary");
        deviceBtn.setStyle("-fx-border-color: rgba(139, 92, 246, 0.4); -fx-text-fill: #c084fc; -fx-cursor: hand;");
        deviceBtn.setOnAction(e -> eventPublisher.publishEvent(new UiEvents.DeviceSyncRequested()));

        // Settings Button
        Button settingsBtn = new Button("⚙ Paramètres");
        settingsBtn.getStyleClass().add("btn-secondary");
        settingsBtn.setOnAction(e -> eventPublisher.publishEvent(new UiEvents.OpenSettingsRequested()));

        this.getChildren().addAll(searchField, statusFilters, litestreamBadge, deviceBtn, viewControls, settingsBtn);
        updateLitestreamStatus();
    }

    private ToggleButton createStatusFilterButton(String label, String statusKey, ToggleGroup group, boolean selected) {
        ToggleButton btn = new ToggleButton(label);
        btn.setToggleGroup(group);
        btn.setSelected(selected);
        btn.getStyleClass().add("btn-secondary");
        btn.setStyle("-fx-cursor: hand; -fx-font-size: 11px; -fx-padding: 3 8;");
        btn.setOnAction(e -> {
            if (btn.isSelected()) {
                eventPublisher.publishEvent(new UiEvents.FilterChanged(searchField.getText(), null, statusKey));
            } else if (group.getSelectedToggle() == null) {
                btn.setSelected(true);
            }
        });
        return btn;
    }

    public void focusSearchField() {
        if (searchField != null) {
            searchField.requestFocus();
            searchField.selectAll();
        }
    }

    public void updateLitestreamStatus() {
        Platform.runLater(() -> {
            LitestreamService.Status st = litestreamService.getStatus();
            if (st == LitestreamService.Status.RUNNING) {
                litestreamBadge.setText("⚡ Réplication active");
                litestreamBadge.setStyle("-fx-background-color: rgba(16, 185, 129, 0.15); -fx-border-color: rgba(16, 185, 129, 0.4); -fx-border-radius: 12; -fx-background-radius: 12; -fx-text-fill: #34d399; -fx-font-size: 11px; -fx-padding: 4 10 4 10; -fx-cursor: hand;");
            } else if (st == LitestreamService.Status.DISABLED) {
                litestreamBadge.setText("☁ Sauvegarde locale");
                litestreamBadge.setStyle("-fx-background-color: rgba(255, 255, 255, 0.05); -fx-border-color: rgba(255, 255, 255, 0.1); -fx-border-radius: 12; -fx-background-radius: 12; -fx-text-fill: #9ca3af; -fx-font-size: 11px; -fx-padding: 4 10 4 10; -fx-cursor: hand;");
            } else {
                litestreamBadge.setText("⚠️ Litestream inactif");
                litestreamBadge.setStyle("-fx-background-color: rgba(245, 158, 11, 0.15); -fx-border-color: rgba(245, 158, 11, 0.4); -fx-border-radius: 12; -fx-background-radius: 12; -fx-text-fill: #fbbf24; -fx-font-size: 11px; -fx-padding: 4 10 4 10; -fx-cursor: hand;");
            }
        });
    }

    public String getSearchQuery() {
        return searchField.getText();
    }
}
