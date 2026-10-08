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
}
