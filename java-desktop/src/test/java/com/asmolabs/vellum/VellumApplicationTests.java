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
        bookService.ensureLibraryDirectory();
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

        Map<String, Object> params = pluginService.getPluginParams("oceanofpdf-cleaner");
        assertNotNull(params);

        pluginService.savePluginParams("oceanofpdf-cleaner", Map.of("custom_replacement", "[Cleaned]"));
        Map<String, Object> updated = pluginService.getPluginParams("oceanofpdf-cleaner");
        assertEquals("[Cleaned]", updated.get("custom_replacement"));
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

        // Delete
        bookService.deleteBook(imported.getId());
        assertTrue(bookRepository.findById(imported.getId()).isEmpty());
    }
}
