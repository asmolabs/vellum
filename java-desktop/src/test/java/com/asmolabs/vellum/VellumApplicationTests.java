package com.asmolabs.vellum;

import com.asmolabs.vellum.config.AppProperties;
import com.asmolabs.vellum.domain.entity.Book;
import com.asmolabs.vellum.domain.entity.Tag;
import com.asmolabs.vellum.repository.BookRepository;
import com.asmolabs.vellum.repository.TagRepository;
import com.asmolabs.vellum.service.BookService;
import com.asmolabs.vellum.service.LitestreamService;
import com.asmolabs.vellum.service.MetadataService;
import com.asmolabs.vellum.service.PluginService;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.boot.test.context.SpringBootTest;

import java.io.File;
import java.io.FileOutputStream;
import java.nio.file.Path;
import java.util.List;
import java.util.Map;
import java.util.zip.ZipEntry;
import java.util.zip.ZipOutputStream;

import static org.junit.jupiter.api.Assertions.*;

@SpringBootTest
class VellumApplicationTests {

    @Autowired
    private BookRepository bookRepository;

    @Autowired
    private TagRepository tagRepository;

    @Autowired
    private BookService bookService;

    @Autowired
    private MetadataService metadataService;

    @Autowired
    private PluginService pluginService;

    @Autowired
    private LitestreamService litestreamService;

    @Autowired
    private AppProperties appProperties;

    @Autowired
    private com.asmolabs.vellum.converter.ConversionService conversionService;

    @TempDir
    Path tempDir;

    @org.junit.jupiter.api.BeforeAll
    static void initJfx() {
        try {
            javafx.application.Platform.startup(() -> {});
        } catch (IllegalStateException ignored) {
            // Already initialized
        }
    }

    @BeforeEach
    void setUp() {
        appProperties.setLibraryPath(tempDir.resolve("library").toString());
        appProperties.setPluginsPath(tempDir.resolve("plugins").toString());
        bookService.ensureLibraryDirectory();
        bookRepository.deleteAll();
    }

    @Test
    void testSpringContextLoads() {
        assertNotNull(bookRepository);
        assertNotNull(bookService);
        assertNotNull(metadataService);
        assertNotNull(pluginService);
        assertNotNull(litestreamService);
    }

    @Test
    void testLitestreamConfigGeneration() throws Exception {
        File configFile = litestreamService.generateConfigFile();
        assertTrue(configFile.exists());
        assertTrue(configFile.length() > 0);
    }

    @Test
    void testPluginServiceRegistryAndParams() {
        var plugins = pluginService.getRegisteredPlugins();
        assertFalse(plugins.isEmpty());
        assertTrue(plugins.stream().anyMatch(p -> "oceanofpdf-cleaner".equals(p.id())));
        assertFalse(plugins.stream().filter(p -> "oceanofpdf-cleaner".equals(p.id())).findFirst().orElseThrow().external());

        Map<String, Object> params = pluginService.getPluginParams("oceanofpdf-cleaner");
        assertNotNull(params);

        pluginService.savePluginParams("oceanofpdf-cleaner", Map.of("custom_replacement", "[Cleaned]"));
        Map<String, Object> updated = pluginService.getPluginParams("oceanofpdf-cleaner");
        assertEquals("[Cleaned]", updated.get("custom_replacement"));
    }

    @Test
    void testPluginEnableDisable() {
        assertTrue(pluginService.isPluginEnabled("oceanofpdf-cleaner"));
        pluginService.setPluginEnabled("oceanofpdf-cleaner", false);
        assertFalse(pluginService.isPluginEnabled("oceanofpdf-cleaner"));

        pluginService.setPluginEnabled("oceanofpdf-cleaner", true);
        assertTrue(pluginService.isPluginEnabled("oceanofpdf-cleaner"));
    }

    @Test
    void testExternalPluginDiscoveryAndExecution() throws Exception {
        // 1. Create external plugin directory with plugin.json and bash script
        File extPluginsDir = new File(appProperties.getPluginsPath());
        File myPluginDir = new File(extPluginsDir, "test-script-plugin");
        myPluginDir.mkdirs();

        String pluginJson = """
            {
              "id": "test-script-plugin",
              "name": "Test External Script",
              "description": "Script de test pour verifier le chargement dynamique",
              "version": "1.0.0",
              "type": "transformation",
              "command": "sh script.sh",
              "configSchema": [
                {
                  "key": "custom_opt",
                  "label": "Option personnalisee",
                  "type": "text",
                  "defaultValue": "val",
                  "description": "Description"
                },
                {
                  "key": "api_secret",
                  "label": "Cle API",
                  "type": "password",
                  "defaultValue": "secret123",
                  "description": "Cle d'acces"
                },
                {
                  "key": "threshold",
                  "label": "Seuil",
                  "type": "slider",
                  "defaultValue": 0.75,
                  "min": 0.0,
                  "max": 1.0,
                  "step": 0.05,
                  "description": "Valeur du seuil"
                },
                {
                  "key": "custom_rules",
                  "label": "Regles",
                  "type": "textarea",
                  "defaultValue": "rule1\\nrule2",
                  "description": "Regles multilignes"
                }
              ]
            }
            """.stripIndent();
        java.nio.file.Files.writeString(new File(myPluginDir, "plugin.json").toPath(), pluginJson);

        String scriptContent = """
            #!/bin/sh
            while [ "$#" -gt 0 ]; do
              case "$1" in
                --input) INPUT="$2"; shift 2;;
                --output) OUTPUT="$2"; shift 2;;
                --config) CONFIG="$2"; shift 2;;
                *) shift;;
              esac
            done
            cp "$INPUT" "$OUTPUT"
            """.stripIndent();
        File scriptFile = new File(myPluginDir, "script.sh");
        java.nio.file.Files.writeString(scriptFile.toPath(), scriptContent);

        // 2. Reload plugins
        pluginService.reloadPlugins();

        // 3. Verify external plugin registered
        var plugins = pluginService.getRegisteredPlugins();
        var extPlugin = plugins.stream().filter(p -> "test-script-plugin".equals(p.id())).findFirst();
        assertTrue(extPlugin.isPresent());
        assertTrue(extPlugin.get().external());
        assertEquals("Test External Script", extPlugin.get().name());
        assertEquals(4, extPlugin.get().configSchema().size());
        assertTrue(extPlugin.get().configSchema().stream().anyMatch(f -> "slider".equals(f.type()) && f.min() == 0.0 && f.max() == 1.0));
        assertTrue(extPlugin.get().configSchema().stream().anyMatch(f -> "password".equals(f.type())));
        assertTrue(extPlugin.get().configSchema().stream().anyMatch(f -> "textarea".equals(f.type())));

        // 4. Test transformation execution
        File dummyInput = new File(tempDir.toFile(), "source.epub");
        java.nio.file.Files.writeString(dummyInput.toPath(), "test epub content");

        File transformed = pluginService.runTransformationPlugin(dummyInput, "test-script-plugin");
        assertNotNull(transformed);
        assertTrue(transformed.exists());
        assertEquals("test epub content", java.nio.file.Files.readString(transformed.toPath()));
    }

    @Test
    void testBookFormatPluginsAndExternalFormatExtensibility() throws Exception {
        // 1. Verify default internal format plugins
        var supportedExts = metadataService.getSupportedExtensions();
        assertTrue(supportedExts.contains("epub"));
        assertTrue(supportedExts.contains("pdf"));
        assertTrue(supportedExts.contains("cbz"));
        assertTrue(supportedExts.contains("cbr"));
        assertTrue(supportedExts.contains("mobi"));
        assertTrue(supportedExts.contains("azw3"));
        assertTrue(supportedExts.contains("m4b"));
        assertTrue(supportedExts.contains("mp3"));

        // 1b. Test Audiobook ID3v2 format parsing
        File mockMp3 = new File(tempDir.toFile(), "audiobook.mp3");
        try (var fos = new java.io.FileOutputStream(mockMp3)) {
            byte[] id3Header = new byte[]{'I', 'D', '3', 3, 0, 0, 0, 0, 0, 60};
            fos.write(id3Header);
            byte[] tit2 = new byte[]{'T', 'I', 'T', '2', 0, 0, 0, 12, 0, 0, 0, 'A', 'u', 'd', 'i', 'o', ' ', 'L', 'i', 'v', 'r', 'e'};
            fos.write(tit2);
            byte[] tpe1 = new byte[]{'T', 'P', 'E', '1', 0, 0, 0, 10, 0, 0, 0, 'N', 'a', 'r', 'r', 'a', 't', 'e', 'u', 'r'};
            fos.write(tpe1);
        }
        assertTrue(metadataService.isFormatSupported(mockMp3));
        MetadataService.ExtractedMetadata mp3Meta = metadataService.parseMetadata(mockMp3);
        assertEquals("Audio Livre", mp3Meta.title());
        assertEquals("Narrateur", mp3Meta.author());
        assertTrue(mp3Meta.tags().contains("Livre Audio"));

        // 2. Create external format plugin for a new format (.fb2)
        File extPluginsDir = new File(appProperties.getPluginsPath());
        File fb2PluginDir = new File(extPluginsDir, "fb2-format-plugin");
        fb2PluginDir.mkdirs();

        String pluginJson = """
            {
              "id": "fb2-format-plugin",
              "name": "FictionBook 2 Format Plugin",
              "description": "Support du format FB2",
              "version": "1.0.0",
              "type": "format",
              "supportedExtensions": ["fb2"],
              "command": "sh parse_fb2.sh"
            }
            """.stripIndent();
        java.nio.file.Files.writeString(new File(fb2PluginDir, "plugin.json").toPath(), pluginJson);

        String scriptContent = """
            #!/bin/sh
            while [ "$#" -gt 0 ]; do
              case "$1" in
                --input) INPUT="$2"; shift 2;;
                --metadata-out) META_OUT="$2"; shift 2;;
                --cover-out) COVER_OUT="$2"; shift 2;;
                *) shift;;
              esac
            done
            echo '{"title":"Guerre et Paix","author":"Leon Tolstoi","publisher":"Test Ed","tags":["Classique","Russe"]}' > "$META_OUT"
            """.stripIndent();
        File scriptFile = new File(fb2PluginDir, "parse_fb2.sh");
        java.nio.file.Files.writeString(scriptFile.toPath(), scriptContent);

        // 3. Reload plugins
        pluginService.reloadPlugins();

        // 4. Verify FB2 is now recognized dynamically!
        File mockFb2 = new File(tempDir.toFile(), "book.fb2");
        java.nio.file.Files.writeString(mockFb2.toPath(), "<FictionBook>...</FictionBook>");

        assertTrue(metadataService.isFormatSupported(mockFb2));
        assertTrue(metadataService.getSupportedExtensions().contains("fb2"));

        MetadataService.ExtractedMetadata meta = metadataService.parseMetadata(mockFb2);
        assertNotNull(meta);
        assertEquals("Guerre et Paix", meta.title());
        assertEquals("Leon Tolstoi", meta.author());
        assertTrue(meta.tags().contains("Classique"));
    }

    @Test
    void testMetadataServiceFallbackParsing() {
        File dummyFile = new File(tempDir.toFile(), "Victor Hugo - Les Miserables.epub");
        MetadataService.ExtractedMetadata meta = metadataService.parseMetadata(dummyFile);

        assertEquals("Victor Hugo", meta.author());
        assertEquals("Les Miserables", meta.title());
    }

    @Test
    void testBookCrudAndTagCount() throws Exception {
        // Create mock EPUB zip file
        File mockEpub = new File(tempDir.toFile(), "Albert Camus - L'Etranger.epub");
        try (ZipOutputStream zos = new ZipOutputStream(new FileOutputStream(mockEpub))) {
            zos.putNextEntry(new ZipEntry("META-INF/container.xml"));
            String containerXml = """
<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
   <rootfiles>
      <rootfile full-path="content.opf" media-type="application/oebps-package+xml"/>
   </rootfiles>
</container>
""".trim();
            zos.write(containerXml.getBytes());
            zos.closeEntry();

            zos.putNextEntry(new ZipEntry("content.opf"));
            String opfXml = """
<?xml version="1.0"?>
<package xmlns="http://www.idpf.org/2007/opf" version="2.0">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>L'Étranger</dc:title>
    <dc:creator>Albert Camus</dc:creator>
    <dc:publisher>Gallimard</dc:publisher>
    <dc:date>1942</dc:date>
    <dc:subject>Philosophie</dc:subject>
    <dc:subject>Classique</dc:subject>
    <dc:description>Un chef-d'œuvre de la littérature française.</dc:description>
  </metadata>
  <manifest>
    <item id="c1" href="chap1.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
</package>
""".trim();
            zos.write(opfXml.getBytes());
            zos.closeEntry();
        }

        // Import
        Book imported = bookService.importBook(mockEpub);
        assertNotNull(imported);
        assertEquals("L'Étranger", imported.getTitle());
        assertEquals("Albert Camus", imported.getAuthor());
        assertEquals(1, imported.getFormats().size());

        // Verify search
        List<Book> searchResults = bookService.getBooks("Camus", null);
        assertFalse(searchResults.isEmpty());
        assertEquals("L'Étranger", searchResults.get(0).getTitle());

        // Verify tag counts
        Map<String, Long> tagCounts = bookService.getTagsWithCounts();
        assertTrue(tagCounts.containsKey("Philosophie"));

        // Update
        Book updated = bookService.updateBook(
                imported.getId(),
                "L'Étranger (Édition Définitive)",
                "Albert Camus",
                "Gallimard",
                "1942",
                "Description mise à jour",
                List.of("Philosophie", "Prix Nobel")
        );
        assertEquals("L'Étranger (Édition Définitive)", updated.getTitle());

        // Test Reading Progress
        Book progBook = bookService.updateReadingProgress(updated.getId(), 2, 10);
        assertEquals(2, progBook.getCurrentChapterIndex());
        assertEquals(30.0, progBook.getProgressPercent());
        assertFalse(progBook.isFinished());

        BookService.ReadingStats stats = bookService.getReadingStats();
        assertEquals(1, stats.totalBooks());
        assertEquals(1, stats.inProgressBooks());

        // Test Bulk Operations
        bookService.bulkAddTag(List.of(updated.getId()), "Favoris");
        Book withFav = bookRepository.findById(updated.getId()).orElseThrow();
        assertTrue(withFav.getTags().stream().anyMatch(t -> "Favoris".equals(t.getName())));

        // Delete
        bookService.deleteBook(imported.getId());
        assertTrue(bookRepository.findById(imported.getId()).isEmpty());
    }

    @Autowired
    private com.asmolabs.vellum.web.opds.OpdsController opdsController;

    @Test
    void testOpdsCatalogFeed() {
        String rootFeed = opdsController.getRootCatalog();
        assertNotNull(rootFeed);
        assertTrue(rootFeed.contains("Vellum Desktop Ebook Library"));
        assertTrue(rootFeed.contains("/opds/books"));

        String booksFeed = opdsController.getBooksCatalog(null, null);
        assertNotNull(booksFeed);
        assertTrue(booksFeed.contains("<feed"));
    }

    @Test
    void testJavaFormatPluginsJarAndDynamicSource() throws Exception {
        File extPluginsDir = new File(appProperties.getPluginsPath());
        File txtPluginDir = new File(extPluginsDir, "txt-format-plugin");
        txtPluginDir.mkdirs();

        // 1. Create a dynamic .java source plugin
        String javaSource = """
            package custom.test;

            import com.asmolabs.vellum.plugin.PluginDescriptor;
            import com.asmolabs.vellum.plugin.PluginType;
            import com.asmolabs.vellum.plugin.format.BookFormatPlugin;
            import com.asmolabs.vellum.plugin.format.ExtractedMetadata;
            import java.io.File;
            import java.nio.file.Files;
            import java.util.List;

            public class DynamicTextFormatPlugin implements BookFormatPlugin {
                @Override
                public PluginDescriptor getDescriptor() {
                    return new PluginDescriptor("dynamic-txt-format", "Dynamic Text Format", "Plugin Java dynamique pour TXT", "1.0.0", PluginType.FORMAT, true, null, List.of("txt"), List.of());
                }

                @Override
                public List<String> getSupportedExtensions() {
                    return List.of("txt");
                }

                @Override
                public ExtractedMetadata extractMetadata(File file) throws Exception {
                    String text = Files.readString(file.toPath());
                    return new ExtractedMetadata("Titre Dynamique Java", "Auteur Dynamique", "Editeur Java", "2026", text, List.of("Brut", "Java"), null, null);
                }
            }
            """.stripIndent();

        File javaFile = new File(txtPluginDir, "DynamicTextFormatPlugin.java");
        java.nio.file.Files.writeString(javaFile.toPath(), javaSource);

        // 2. Reload plugins - triggers dynamic JDK compilation
        pluginService.reloadPlugins();

        // 3. Verify .txt is now supported dynamically
        assertTrue(metadataService.getSupportedExtensions().contains("txt"));
        File sampleTxt = new File(tempDir.toFile(), "sample.txt");
        java.nio.file.Files.writeString(sampleTxt.toPath(), "Ceci est un document texte simple.");

        assertTrue(metadataService.isFormatSupported(sampleTxt));
        MetadataService.ExtractedMetadata meta = metadataService.parseMetadata(sampleTxt);
        assertNotNull(meta);
        assertEquals("Titre Dynamique Java", meta.title());
        assertEquals("Auteur Dynamique", meta.author());
        assertEquals("Editeur Java", meta.publisher());
        assertTrue(meta.tags().contains("Java"));

        // 4. Test external JAR packaging with properties/scanning fallback
        File cacheClass = new File(extPluginsDir, ".cache/java-classes/custom/test/DynamicTextFormatPlugin.class");
        assertTrue(cacheClass.exists(), "Dynamic compilation should have produced the .class file");

        File jarFile = new File(extPluginsDir, "packaged-format.jar");
        try (var jos = new java.util.jar.JarOutputStream(new java.io.FileOutputStream(jarFile))) {
            jos.putNextEntry(new java.util.zip.ZipEntry("META-INF/vellum-plugin.properties"));
            jos.write("format.class=custom.test.DynamicTextFormatPlugin\n".getBytes(java.nio.charset.StandardCharsets.UTF_8));
            jos.closeEntry();

            jos.putNextEntry(new java.util.zip.ZipEntry("custom/test/DynamicTextFormatPlugin.class"));
            jos.write(java.nio.file.Files.readAllBytes(cacheClass.toPath()));
            jos.closeEntry();
        }

        List<com.asmolabs.vellum.plugin.VellumPlugin> jarPlugins = com.asmolabs.vellum.plugin.external.ExternalJarPluginLoader.loadFromJar(jarFile);
        assertFalse(jarPlugins.isEmpty());
        assertTrue(jarPlugins.get(0) instanceof com.asmolabs.vellum.plugin.format.BookFormatPlugin);
        assertTrue(jarPlugins.get(0).getDescriptor().external());
        assertEquals("dynamic-txt-format", jarPlugins.get(0).getId());
    }

    @Test
    void testPluginConfigurationDiagnostics() {
        // 1. Test built-in translator with Default AI
        var okResult = pluginService.testPluginConfiguration("epub-translator-fr", Map.of("ai_model", "Default AI"));
        assertTrue(okResult.success());
        assertTrue(okResult.message().contains("Default AI"));

        // 2. Test built-in translator with GPT-4o without token (expected failure)
        var failResult = pluginService.testPluginConfiguration("epub-translator-fr", Map.of("ai_model", "GPT-4o", "api_token", ""));
        assertFalse(failResult.success());
        assertTrue(failResult.message().contains("obligatoire"));

        // 3. Test built-in translator with GPT-4o with token
        var tokenOkResult = pluginService.testPluginConfiguration("epub-translator-fr", Map.of("ai_model", "GPT-4o", "api_token", "sk-12345"));
        assertTrue(tokenOkResult.success());

        // 4. Test unknown plugin
        var notFound = pluginService.testPluginConfiguration("unknown-plugin-id", Map.of());
        assertFalse(notFound.success());
        assertTrue(notFound.message().contains("introuvable"));
    }

    @Test
    void testPureJavaFormatConversionEngines() throws Exception {
        // 1. Verify converter engine matrix
        assertTrue(conversionService.canConvert("cbz", "pdf"));
        assertTrue(conversionService.canConvert("pdf", "cbz"));
        assertTrue(conversionService.canConvert("epub", "txt"));
        assertTrue(conversionService.canConvert("txt", "epub"));
        assertTrue(conversionService.canConvert("mobi", "epub"));
        assertTrue(conversionService.canConvert("epub", "pdf"));

        // 2. Test TXT -> EPUB conversion
        File srcTxt = new File(tempDir.toFile(), "roman.txt");
        java.nio.file.Files.writeString(srcTxt.toPath(), "# Chapitre 1\nPremier paragraphe de test.\n\n# Chapitre 2\nSecond paragraphe de test.");
        File convertedEpub = conversionService.convert(srcTxt, "epub");
        assertNotNull(convertedEpub);
        assertTrue(convertedEpub.exists());
        assertTrue(convertedEpub.length() > 0);

        // 3. Test EPUB -> TXT conversion
        File convertedTxt = conversionService.convert(convertedEpub, "txt");
        assertNotNull(convertedTxt);
        assertTrue(convertedTxt.exists());
        String txtText = java.nio.file.Files.readString(convertedTxt.toPath());
        assertTrue(txtText.contains("Chapitre 1"));
        assertTrue(txtText.contains("Premier paragraphe"));

        // 4. Test CBZ -> PDF conversion
        File mockCbz = new File(tempDir.toFile(), "manga.cbz");
        try (var zos = new java.util.zip.ZipOutputStream(new java.io.FileOutputStream(mockCbz))) {
            zos.putNextEntry(new java.util.zip.ZipEntry("page_01.jpg"));
            var bim = new java.awt.image.BufferedImage(80, 80, java.awt.image.BufferedImage.TYPE_INT_RGB);
            javax.imageio.ImageIO.write(bim, "jpg", zos);
            zos.closeEntry();
        }
        File convertedPdf = conversionService.convert(mockCbz, "pdf");
        assertNotNull(convertedPdf);
        assertTrue(convertedPdf.exists());
        assertTrue(convertedPdf.length() > 0);

        // 5. Test BookService transformation with attach_to_book: true
        Book imported = bookService.importBook(convertedEpub);
        assertEquals(1, imported.getFormats().size());
        assertEquals("epub", imported.getFormats().get(0).getFormat());

        // Configure plugin params to convert to txt and attach
        pluginService.savePluginParams("java-format-converter", Map.of("target_format", "txt", "attach_to_book", true));

        Book updatedBook = bookService.runTransformationPlugin(imported.getId(), "java-format-converter");
        assertEquals(imported.getId(), updatedBook.getId(), "Should remain the same book entity when attached");
        assertEquals(2, updatedBook.getFormats().size(), "Book should now have both EPUB and TXT formats");

        var formats = updatedBook.getFormats().stream().map(f -> f.getFormat().toLowerCase()).toList();
        assertTrue(formats.contains("epub"));
        assertTrue(formats.contains("txt"));
    }
}
