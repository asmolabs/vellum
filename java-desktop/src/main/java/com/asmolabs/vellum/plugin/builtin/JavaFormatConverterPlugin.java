package com.asmolabs.vellum.plugin.builtin;

import com.asmolabs.vellum.converter.ConversionService;
import com.asmolabs.vellum.plugin.BookTransformationPlugin;
import com.asmolabs.vellum.plugin.ConfigField;
import com.asmolabs.vellum.plugin.PluginDescriptor;
import com.asmolabs.vellum.plugin.PluginType;
import org.springframework.stereotype.Component;

import java.io.File;
import java.util.List;
import java.util.Map;

@Component
public class JavaFormatConverterPlugin implements BookTransformationPlugin {

    private final ConversionService conversionService;
    private final PluginDescriptor descriptor;

    public JavaFormatConverterPlugin(ConversionService conversionService) {
        this.conversionService = conversionService;
        this.descriptor = new PluginDescriptor(
                "java-format-converter",
                "Convertisseur de Formats (Pur Java)",
                "Convertit vos livres entre formats (EPUB, PDF, CBZ, MOBI, TXT, MD) en 100% Java sans outils externes.",
                "1.0.0",
                PluginType.TRANSFORMATION,
                false,
                null,
                List.of(),
                List.of(
                        new ConfigField("target_format", "Format cible", "select", "pdf", "Format dans lequel convertir le document.", List.of("pdf", "epub", "cbz", "txt", "md")),
                        new ConfigField("attach_to_book", "Attacher à la fiche du livre", "boolean", true, "Ajoute le format résultant au livre existant au lieu de créer une copie séparée."),
                        new ConfigField("dpi", "Qualité de rendu (DPI pour PDF -> CBZ)", "slider", 150.0, "Résolution des images lors de la conversion PDF vers BD (72 à 300 DPI).", 72.0, 300.0, 10.0)
                )
        );
    }

    @Override
    public PluginDescriptor getDescriptor() {
        return descriptor;
    }

    @Override
    public File transform(File inputFile, Map<String, Object> config) throws Exception {
        String targetFormat = String.valueOf(config.getOrDefault("target_format", "pdf")).toLowerCase().trim();
        return conversionService.convert(inputFile, targetFormat, config);
    }

    @Override
    public TestResult testConfiguration(Map<String, Object> config) {
        String targetFormat = String.valueOf(config.getOrDefault("target_format", "pdf")).toLowerCase().trim();
        List<String> supportedTargets = List.of("pdf", "epub", "cbz", "txt", "md");
        if (!supportedTargets.contains(targetFormat)) {
            return TestResult.error("Format cible non supporté : " + targetFormat);
        }
        return TestResult.ok("Moteurs de conversion Java prêts (CBZ<->PDF, EPUB<->TXT, MOBI->EPUB, EPUB->PDF) vers " + targetFormat.toUpperCase() + " !");
    }
}
