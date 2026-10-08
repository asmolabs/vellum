package com.asmolabs.vellum.ui.components;

import com.asmolabs.vellum.domain.entity.Book;
import com.asmolabs.vellum.service.BookService;
import javafx.collections.FXCollections;
import javafx.geometry.Insets;
import javafx.geometry.Pos;
import javafx.scene.Scene;
import javafx.scene.chart.PieChart;
import javafx.scene.control.Button;
import javafx.scene.control.Label;
import javafx.scene.control.ScrollPane;
import javafx.scene.layout.*;
import javafx.stage.Modality;
import javafx.stage.Stage;
import javafx.stage.Window;

import java.util.Map;

public class StatsDialog {

    public static void show(Window owner, BookService bookService) {
        Stage stage = new Stage();
        stage.initOwner(owner);
        stage.initModality(Modality.APPLICATION_MODAL);
        stage.setTitle("Statistiques de lecture — Vellum");

        BorderPane root = new BorderPane();
        root.getStyleClass().add("dialog-container");
        root.setPadding(new Insets(24));

        BookService.ReadingStats stats = bookService.getReadingStats();

        // 1. KPI Cards Header
        HBox kpiBox = new HBox(16);
        kpiBox.setAlignment(Pos.CENTER);

        VBox totalCard = createKpiCard("TOTAL LIVRES", String.valueOf(stats.totalBooks()), "#8b5cf6");
        VBox finishedCard = createKpiCard("LIVRES TERMINÉS", String.valueOf(stats.finishedBooks()), "#10b981");
        VBox inProgressCard = createKpiCard("EN COURS", String.valueOf(stats.inProgressBooks()), "#38bdf8");
        long unread = Math.max(0, stats.totalBooks() - stats.finishedBooks() - stats.inProgressBooks());
        VBox unreadCard = createKpiCard("NON COMMENCÉS", String.valueOf(unread), "#9ca3af");

        kpiBox.getChildren().addAll(totalCard, finishedCard, inProgressCard, unreadCard);

        // 2. Charts & Activity Center
        HBox centerContent = new HBox(20);
        centerContent.setPadding(new Insets(20, 0, 10, 0));

        // PieChart
        PieChart pieChart = new PieChart();
        pieChart.setTitle("Répartition par Genre & Tag");
        pieChart.setLabelsVisible(true);
        pieChart.setLegendVisible(false);
        pieChart.setPrefSize(380, 280);

        javafx.collections.ObservableList<PieChart.Data> pieData = FXCollections.observableArrayList();
        for (Map.Entry<String, Long> e : stats.tagDistribution().entrySet()) {
            pieData.add(new PieChart.Data(e.getKey() + " (" + e.getValue() + ")", e.getValue()));
        }
        pieChart.setData(pieData);

        // Recent Activity Box
        VBox recentBox = new VBox(10);
        recentBox.setPrefWidth(320);
        recentBox.setStyle("-fx-background-color: rgba(255, 255, 255, 0.03); -fx-border-color: rgba(255, 255, 255, 0.08); -fx-border-radius: 8; -fx-padding: 14;");

        Label recentTitle = new Label("DERNIÈRES LECTURES");
        recentTitle.setStyle("-fx-font-size: 11px; -fx-font-weight: bold; -fx-text-fill: -color-text-muted;");
        recentBox.getChildren().add(recentTitle);

        if (stats.recentlyRead().isEmpty()) {
            Label empty = new Label("Aucune lecture récente enregistrée.");
            empty.setStyle("-fx-text-fill: -color-text-muted; -fx-font-size: 12px;");
            recentBox.getChildren().add(empty);
        } else {
            for (Book b : stats.recentlyRead()) {
                HBox rRow = new HBox(8);
                rRow.setAlignment(Pos.CENTER_LEFT);
                VBox tBox = new VBox(2);
                Label tLbl = new Label(b.getTitle());
                tLbl.setStyle("-fx-font-weight: bold; -fx-text-fill: -color-text-primary; -fx-font-size: 12px;");
                Label pLbl = new Label(b.isFinished() ? "Terminé (100%)" : "Avancement : " + (int)b.getProgressPercent() + "%");
                pLbl.setStyle("-fx-text-fill: " + (b.isFinished() ? "#10b981" : "#a78bfa") + "; -fx-font-size: 11px;");
                tBox.getChildren().addAll(tLbl, pLbl);
                HBox.setHgrow(tBox, Priority.ALWAYS);
                rRow.getChildren().add(tBox);
                recentBox.getChildren().add(rRow);
            }
        }

        centerContent.getChildren().addAll(pieChart, recentBox);

        ScrollPane scroll = new ScrollPane(new VBox(16, kpiBox, centerContent));
        scroll.setFitToWidth(true);
        scroll.setStyle("-fx-background-color: transparent; -fx-background: transparent;");

        // Bottom close button
        HBox bottom = new HBox();
        bottom.setAlignment(Pos.CENTER_RIGHT);
        bottom.setPadding(new Insets(16, 0, 0, 0));

        Button closeBtn = new Button("Fermer");
        closeBtn.getStyleClass().add("btn-secondary");
        closeBtn.setOnAction(e -> stage.close());
        bottom.getChildren().add(closeBtn);

        root.setCenter(scroll);
        root.setBottom(bottom);

        Scene scene = new Scene(root, 780, 520);
        scene.getStylesheets().add(StatsDialog.class.getResource("/styles/theme.css").toExternalForm());
        stage.setScene(scene);
        stage.show();
    }

    private static VBox createKpiCard(String label, String value, String colorHex) {
        VBox card = new VBox(6);
        card.setAlignment(Pos.CENTER);
        card.setPrefSize(160, 85);
        card.setStyle("-fx-background-color: rgba(255, 255, 255, 0.03); -fx-border-color: rgba(255, 255, 255, 0.08); -fx-border-radius: 10; -fx-background-radius: 10; -fx-padding: 12;");

        Label valLbl = new Label(value);
        valLbl.setStyle("-fx-font-size: 24px; -fx-font-weight: bold; -fx-text-fill: " + colorHex + ";");

        Label nameLbl = new Label(label);
        nameLbl.setStyle("-fx-font-size: 10px; -fx-font-weight: bold; -fx-text-fill: -color-text-muted;");

        card.getChildren().addAll(valLbl, nameLbl);
        return card;
    }
}
