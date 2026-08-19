package com.asmolabs.vellum.service;

import com.asmolabs.vellum.config.AppProperties;
import com.asmolabs.vellum.domain.entity.Book;
import com.asmolabs.vellum.domain.entity.BookFormat;
import com.asmolabs.vellum.domain.entity.Tag;
import com.asmolabs.vellum.repository.BookFormatRepository;
import com.asmolabs.vellum.repository.BookRepository;
import com.asmolabs.vellum.repository.TagRepository;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.stereotype.Service;
import org.springframework.transaction.annotation.Transactional;

import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.nio.file.*;
import java.nio.file.attribute.BasicFileAttributes;
import java.util.*;

@Service
public class BookService {

    private static final Logger log = LoggerFactory.getLogger(BookService.class);

    private final BookRepository bookRepository;
    private final BookFormatRepository formatRepository;
    private final TagRepository tagRepository;
    private final MetadataService metadataService;
    private final PluginService pluginService;
    private final AppProperties appProperties;

    public BookService(
            BookRepository bookRepository,
            BookFormatRepository formatRepository,
            TagRepository tagRepository,
            MetadataService metadataService,
            PluginService pluginService,
            AppProperties appProperties
    ) {
        this.bookRepository = bookRepository;
        this.formatRepository = formatRepository;
        this.tagRepository = tagRepository;
        this.metadataService = metadataService;
        this.pluginService = pluginService;
        this.appProperties = appProperties;

        // Ensure library directory exists
        ensureLibraryDirectory();
    }

    public void ensureLibraryDirectory() {
        File libDir = new File(appProperties.getLibraryPath());
        if (!libDir.exists()) {
            libDir.mkdirs();
        }
    }

    public String sanitizeFilename(String name) {
        if (name == null || name.isBlank()) return "Sans nom";
        return name.replaceAll("[\\\\/:*?\"<>|]", "").trim();
    }

    public List<Book> getBooks(String search, String tag) {
        String s = (search != null && !search.isBlank()) ? search.trim() : null;
        String t = (tag != null && !tag.isBlank() && !"Tous".equalsIgnoreCase(tag)) ? tag.trim() : null;
        return bookRepository.searchBooks(s, t);
    }

    public Map<String, Long> getTagsWithCounts() {
        List<TagRepository.TagCountProjection> projections = tagRepository.findTagsWithBookCounts();
        Map<String, Long> tagCounts = new LinkedHashMap<>();
        for (TagRepository.TagCountProjection p : projections) {
            tagCounts.put(p.getName(), p.getBookCount());
        }
        return tagCounts;
    }

    public long getTotalBooksCount() {
        return bookRepository.count();
    }

    public Optional<Book> getBookById(String id) {
        return bookRepository.findById(id);
    }

    public File getFileForFormat(BookFormat format) {
        return new File(appProperties.getLibraryPath(), format.getFilepath());
    }

    public File getCoverFile(Book book) {
        if (book.getCoverPath() == null || book.getCoverPath().isBlank()) {
            return null;
        }
        return new File(appProperties.getLibraryPath(), book.getCoverPath());
    }

    @Transactional
    public Book importBook(File sourceFile) throws Exception {
        if (!sourceFile.exists()) {
            throw new IllegalArgumentException("Fichier source introuvable: " + sourceFile.getAbsolutePath());
        }

        String sourceExt = getExtension(sourceFile.getName()).toLowerCase();
        if (!List.of("epub", "mobi", "azw3").contains(sourceExt)) {
            throw new IllegalArgumentException("Format non supporté: ." + sourceExt);
        }

        // 1. Parse Metadata
        MetadataService.ExtractedMetadata meta = metadataService.parseMetadata(sourceFile);

        // 2. Enriched Metadata via Plugins
        Map<String, Object> enriched = pluginService.runImportPlugins(sourceFile, Map.of(
                "title", meta.title(),
                "author", meta.author(),
                "tags", meta.tags(),
                "description", meta.description() != null ? meta.description() : ""
        ));

        String title = (String) enriched.getOrDefault("title", meta.title());
        String author = (String) enriched.getOrDefault("author", meta.author());
        @SuppressWarnings("unchecked")
        List<String> tagsList = (List<String>) enriched.getOrDefault("tags", meta.tags());
        String description = (String) enriched.getOrDefault("description", meta.description());

        String bookId = UUID.randomUUID().toString();
        String cleanAuthor = sanitizeFilename(author);
        String cleanTitle = sanitizeFilename(title);

        // Directory structure: libraryPath / Author / Title (UUID) /
        String bookDirRel = cleanAuthor + File.separator + cleanTitle + " (" + bookId + ")";
        File bookDirAbs = new File(appProperties.getLibraryPath(), bookDirRel);
        bookDirAbs.mkdirs();

        // Target book file
        String targetFilename = cleanTitle + " (" + bookId + ")." + sourceExt;
        String targetFilepathRel = bookDirRel + File.separator + targetFilename;
        File targetFilepathAbs = new File(bookDirAbs, targetFilename);

        Files.copy(sourceFile.toPath(), targetFilepathAbs.toPath(), StandardCopyOption.REPLACE_EXISTING);
        long fileSize = targetFilepathAbs.length();

        // Cover file
        String coverPathRel = null;
        if (meta.coverData() != null && meta.coverData().length > 0) {
            String coverExt = (meta.coverExt() != null && !meta.coverExt().isBlank()) ? meta.coverExt() : ".jpg";
            String coverFilename = "cover" + coverExt;
            File coverFileAbs = new File(bookDirAbs, coverFilename);
            try (FileOutputStream fos = new FileOutputStream(coverFileAbs)) {
                fos.write(meta.coverData());
            }
            coverPathRel = bookDirRel + File.separator + coverFilename;
        }

        // 3. Create Entities
        Book book = new Book();
        book.setId(bookId);
        book.setTitle(title);
        book.setAuthor(author);
        book.setPublisher(meta.publisher());
        book.setPublishDate(meta.publishDate());
        book.setDescription(description);
        book.setCoverPath(coverPathRel);

        // Tags
        for (String tagName : tagsList) {
            Tag tag = tagRepository.findById(tagName).orElseGet(() -> tagRepository.save(new Tag(tagName)));
            book.addTag(tag);
        }

        // Format
        BookFormat format = new BookFormat(sourceExt, targetFilename, targetFilepathRel, fileSize);
        book.addFormat(format);

        return bookRepository.save(book);
    }

    @Transactional
    public Book updateBook(String bookId, String title, String author, String publisher, String publishDate, String description, List<String> tags) throws IOException {
        Book book = bookRepository.findById(bookId)
                .orElseThrow(() -> new IllegalArgumentException("Livre introuvable avec l'ID: " + bookId));

        String oldAuthor = book.getAuthor();
        String oldTitle = book.getTitle();

        book.setTitle(title);
        book.setAuthor(author);
        book.setPublisher(publisher);
        book.setPublishDate(publishDate);
        book.setDescription(description);

        // Update tags
        book.getTags().clear();
        for (String tagName : tags) {
            Tag tag = tagRepository.findById(tagName).orElseGet(() -> tagRepository.save(new Tag(tagName)));
            book.addTag(tag);
        }

        // If author or title changed, rename directory and files on disk
        if (!Objects.equals(oldAuthor, author) || !Objects.equals(oldTitle, title)) {
            if (!book.getFormats().isEmpty()) {
                BookFormat firstFmt = book.getFormats().get(0);
                File oldFirstFileAbs = new File(appProperties.getLibraryPath(), firstFmt.getFilepath());
                File oldBookDirAbs = oldFirstFileAbs.getParentFile();

                if (oldBookDirAbs != null && oldBookDirAbs.exists()) {
                    String cleanAuthor = sanitizeFilename(author);
                    String cleanTitle = sanitizeFilename(title);

                    String newBookDirRel = cleanAuthor + File.separator + cleanTitle + " (" + book.getId() + ")";
                    File newBookDirAbs = new File(appProperties.getLibraryPath(), newBookDirRel);
                    newBookDirAbs.getParentFile().mkdirs();

                    Files.move(oldBookDirAbs.toPath(), newBookDirAbs.toPath(), StandardCopyOption.ATOMIC_MOVE);

                    // Clean empty old author directory if needed
                    File oldAuthorDir = oldBookDirAbs.getParentFile();
                    if (oldAuthorDir != null && oldAuthorDir.exists() && Objects.requireNonNull(oldAuthorDir.list()).length == 0) {
                        oldAuthorDir.delete();
                    }

                    // Update formats filepaths
                    for (BookFormat fmt : book.getFormats()) {
                        String ext = getExtension(fmt.getFilename());
                        String newFilename = cleanTitle + " (" + book.getId() + ")." + ext;
                        File oldFileInRenamed = new File(newBookDirAbs, new File(fmt.getFilepath()).getName());
                        File newFileInRenamed = new File(newBookDirAbs, newFilename);

                        if (oldFileInRenamed.exists()) {
                            oldFileInRenamed.renameTo(newFileInRenamed);
                        }

                        fmt.setFilename(newFilename);
                        fmt.setFilepath(newBookDirRel + File.separator + newFilename);
                    }

                    // Update cover path
                    if (book.getCoverPath() != null) {
                        String coverName = new File(book.getCoverPath()).getName();
                        book.setCoverPath(newBookDirRel + File.separator + coverName);
                    }
                }
            }
        }

        return bookRepository.save(book);
    }

    @Transactional
    public Book updateCover(String bookId, File newCoverFile) throws IOException {
        Book book = bookRepository.findById(bookId)
                .orElseThrow(() -> new IllegalArgumentException("Livre introuvable: " + bookId));

        if (book.getFormats().isEmpty()) {
            throw new IllegalStateException("Le livre n'a pas de fichier associé.");
        }

        BookFormat firstFmt = book.getFormats().get(0);
        File bookFileAbs = new File(appProperties.getLibraryPath(), firstFmt.getFilepath());
        File bookDirAbs = bookFileAbs.getParentFile();
        String bookDirRel = new File(firstFmt.getFilepath()).getParent();

        // Delete old cover
        if (book.getCoverPath() != null) {
            File oldCover = new File(appProperties.getLibraryPath(), book.getCoverPath());
            if (oldCover.exists()) {
                oldCover.delete();
            }
        }

        String ext = getExtension(newCoverFile.getName());
        String coverName = "cover." + (ext.isBlank() ? "jpg" : ext);
        File targetCover = new File(bookDirAbs, coverName);

        Files.copy(newCoverFile.toPath(), targetCover.toPath(), StandardCopyOption.REPLACE_EXISTING);
        book.setCoverPath(bookDirRel + File.separator + coverName);

        return bookRepository.save(book);
    }

    @Transactional
    public void deleteBook(String bookId) throws IOException {
        Book book = bookRepository.findById(bookId)
                .orElseThrow(() -> new IllegalArgumentException("Livre introuvable: " + bookId));

        if (!book.getFormats().isEmpty()) {
            BookFormat firstFmt = book.getFormats().get(0);
            File bookDir = new File(appProperties.getLibraryPath(), firstFmt.getFilepath()).getParentFile();
            if (bookDir != null && bookDir.exists()) {
                deleteDirectoryRecursively(bookDir.toPath());
                File authorDir = bookDir.getParentFile();
                if (authorDir != null && authorDir.exists() && Objects.requireNonNull(authorDir.list()).length == 0) {
                    authorDir.delete();
                }
            }
        }

        bookRepository.delete(book);
    }

    @Transactional
    public Book runTransformationPlugin(String bookId, String pluginId) throws Exception {
        Book book = bookRepository.findById(bookId)
                .orElseThrow(() -> new IllegalArgumentException("Livre introuvable: " + bookId));

        if (book.getFormats().isEmpty()) {
            throw new IllegalStateException("Aucun format disponible pour la transformation");
        }

        BookFormat firstFmt = book.getFormats().get(0);
        File inputFile = new File(appProperties.getLibraryPath(), firstFmt.getFilepath());

        File outputFile = pluginService.runTransformationPlugin(inputFile, pluginId);

        String newBookId = UUID.randomUUID().toString();
        String cleanAuthor = sanitizeFilename(book.getAuthor());
        String cleanTitle = sanitizeFilename(book.getTitle() + " (Transformé)");

        String newBookDirRel = cleanAuthor + File.separator + cleanTitle + " (" + newBookId + ")";
        File newBookDirAbs = new File(appProperties.getLibraryPath(), newBookDirRel);
        newBookDirAbs.mkdirs();

        String ext = getExtension(outputFile.getName());
        String filename = cleanTitle + " (" + newBookId + ")." + ext;
        File destFileAbs = new File(newBookDirAbs, filename);
        String destFileRel = newBookDirRel + File.separator + filename;

        Files.move(outputFile.toPath(), destFileAbs.toPath(), StandardCopyOption.REPLACE_EXISTING);
        long fileSize = destFileAbs.length();

        // Copy cover if existing
        String coverPathRel = null;
        if (book.getCoverPath() != null) {
            File oldCover = new File(appProperties.getLibraryPath(), book.getCoverPath());
            if (oldCover.exists()) {
                String coverExt = getExtension(book.getCoverPath());
                String coverFilename = "cover." + (coverExt.isBlank() ? "jpg" : coverExt);
                File newCoverAbs = new File(newBookDirAbs, coverFilename);
                Files.copy(oldCover.toPath(), newCoverAbs.toPath(), StandardCopyOption.REPLACE_EXISTING);
                coverPathRel = newBookDirRel + File.separator + coverFilename;
            }
        }

        Book newBook = new Book();
        newBook.setId(newBookId);
        newBook.setTitle(book.getTitle() + " (FR)");
        newBook.setAuthor(book.getAuthor());
        newBook.setPublisher(book.getPublisher());
        newBook.setPublishDate(book.getPublishDate());
        newBook.setDescription(book.getDescription());
        newBook.setCoverPath(coverPathRel);

        for (Tag tag : book.getTags()) {
            newBook.addTag(tag);
        }
        Tag traduitTag = tagRepository.findById("Traduit").orElseGet(() -> tagRepository.save(new Tag("Traduit")));
        newBook.addTag(traduitTag);

        BookFormat fmt = new BookFormat(ext, filename, destFileRel, fileSize);
        newBook.addFormat(fmt);

        return bookRepository.save(newBook);
    }

    private void deleteDirectoryRecursively(Path path) throws IOException {
        Files.walkFileTree(path, new SimpleFileVisitor<>() {
            @Override
            public FileVisitResult visitFile(Path file, BasicFileAttributes attrs) throws IOException {
                Files.delete(file);
                return FileVisitResult.CONTINUE;
            }

            @Override
            public FileVisitResult postVisitDirectory(Path dir, IOException exc) throws IOException {
                Files.delete(dir);
                return FileVisitResult.CONTINUE;
            }
        });
    }

    private String getExtension(String name) {
        int idx = name.lastIndexOf('.');
        return (idx >= 0 && idx < name.length() - 1) ? name.substring(idx + 1) : "";
    }
}
