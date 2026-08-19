package com.asmolabs.vellum;

import com.asmolabs.vellum.ui.components.MainView;
import javafx.application.Application;
import javafx.application.Platform;
import javafx.scene.Scene;
import javafx.scene.image.Image;
import javafx.stage.Stage;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.boot.builder.SpringApplicationBuilder;
import org.springframework.boot.autoconfigure.SpringBootApplication;
import org.springframework.context.ConfigurableApplicationContext;

@SpringBootApplication
public class VellumApplication extends Application {

    private static final Logger log = LoggerFactory.getLogger(VellumApplication.class);

    private static ConfigurableApplicationContext springContext;

    public static void main(String[] args) {
        log.info("Launching Vellum Desktop (Java 25 / Spring Boot / JavaFX)...");
        Application.launch(VellumApplication.class, args);
    }

    @Override
    public void init() {
        log.info("Initializing Spring Context in JavaFX init()...");
        springContext = new SpringApplicationBuilder(VellumApplication.class)
                .headless(false)
                .run(getParameters().getRaw().toArray(new String[0]));
    }

    @Override
    public void start(Stage primaryStage) {
        log.info("Starting JavaFX Primary Stage...");
        try {
            MainView mainView = springContext.getBean(MainView.class);

            Scene scene = new Scene(mainView, 1280, 800);
            scene.getStylesheets().add(getClass().getResource("/styles/theme.css").toExternalForm());

            primaryStage.setTitle("Vellum — Bibliothèque Ebook Desktop");
            primaryStage.setMinWidth(960);
            primaryStage.setMinHeight(600);
            primaryStage.setScene(scene);

            primaryStage.setOnCloseRequest(e -> {
                Platform.exit();
            });

            primaryStage.show();
        } catch (Exception e) {
            log.error("Fatal error during JavaFX startup", e);
            throw e;
        }
    }

    @Override
    public void stop() {
        log.info("Stopping Vellum application and closing Spring context...");
        if (springContext != null && springContext.isActive()) {
            springContext.close();
        }
        Platform.exit();
    }
}
