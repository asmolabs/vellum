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
    private final com.asmolabs.vellum.converter.ConversionService conversionService;
    private final AppProperties appProperties;

    public BookService(
            BookRepository bookRepository,
            BookFormatRepository formatRepository,
            TagRepository tagRepository,
            MetadataService metadataService,
            PluginService pluginService,
            com.asmolabs.vellum.converter.ConversionService conversionService,
            AppProperties appProperties
    ) {
        this.bookRepository = bookRepository;
        this.formatRepository = formatRepository;
        this.tagRepository = tagRepository;
        this.metadataService = metadataService;
        this.pluginService = pluginService;
        this.conversionService = conversionService;
        this.appProperties = appProperties;

        // Ensure library directory exists
        ensureLibraryDirectory();
    }

    public com.asmolabs.vellum.converter.ConversionService getConversionService() {
        return conversionService;
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
        if (!metadataService.isFormatSupported(sourceFile)) {
            throw new IllegalArgumentException("Format non supporté: ." + sourceExt + " (aucun plugin de format activé).");
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
        return runTransformationPlugin(bookId, pluginId, Map.of());
    }

    @Transactional
    public Book runTransformationPlugin(String bookId, String pluginId, Map<String, Object> customParams, boolean attachToSameBook) throws Exception {
        Map<String, Object> params = new HashMap<>();
        if (customParams != null) {
            params.putAll(customParams);
        }
        params.put("attach_to_book", attachToSameBook);
        return runTransformationPlugin(bookId, pluginId, params);
    }

    @Transactional
    public Book runTransformationPlugin(String bookId, String pluginId, Map<String, Object> customParams) throws Exception {
        Book book = bookRepository.findById(bookId)
                .orElseThrow(() -> new IllegalArgumentException("Livre introuvable: " + bookId));

        if (book.getFormats().isEmpty()) {
            throw new IllegalStateException("Aucun format disponible pour la transformation");
        }

        BookFormat firstFmt = book.getFormats().get(0);
        File inputFile = new File(appProperties.getLibraryPath(), firstFmt.getFilepath());

        File outputFile = pluginService.runTransformationPlugin(inputFile, pluginId, customParams);

        String sourceExt = getExtension(inputFile.getName());
        String ext = getExtension(outputFile.getName());

        Map<String, Object> pluginParams = new HashMap<>(pluginService.getPluginParams(pluginId));
        if (customParams != null) {
            pluginParams.putAll(customParams);
        }
        boolean attachToExisting = Boolean.parseBoolean(String.valueOf(pluginParams.getOrDefault("attach_to_book", "false")));

        if (attachToExisting) {
            // Attach format directly to the existing book
            File bookDirAbs = inputFile.getParentFile();
            String destFilename;
            if (sourceExt.equalsIgnoreCase(ext)) {
                String tag = pluginId.contains("translator") ? "Traduit" : "Transformé";
                destFilename = sanitizeFilename(book.getTitle()) + " (" + tag + ")." + ext;
            } else {
                destFilename = sanitizeFilename(book.getTitle()) + "." + ext;
            }
            File destFileAbs = new File(bookDirAbs, destFilename);
            Files.move(outputFile.toPath(), destFileAbs.toPath(), StandardCopyOption.REPLACE_EXISTING);

            File libDir = new File(appProperties.getLibraryPath());
            String destFileRel = libDir.toPath().relativize(destFileAbs.toPath()).toString();
            long fileSize = destFileAbs.length();

            book.getFormats().removeIf(f -> f.getFilepath().equalsIgnoreCase(destFileRel));
            BookFormat fmt = new BookFormat(ext, destFilename, destFileRel, fileSize);
            book.addFormat(fmt);

            if (pluginId.contains("translator")) {
                Tag traduitTag = tagRepository.findById("Traduit").orElseGet(() -> tagRepository.save(new Tag("Traduit")));
                book.addTag(traduitTag);
            }

            return bookRepository.save(book);
        }

        String newBookId = UUID.randomUUID().toString();
        String cleanAuthor = sanitizeFilename(book.getAuthor());
        String cleanTitle = sanitizeFilename(book.getTitle() + " (Transformé)");

        String newBookDirRel = cleanAuthor + File.separator + cleanTitle + " (" + newBookId + ")";
        File newBookDirAbs = new File(appProperties.getLibraryPath(), newBookDirRel);
        newBookDirAbs.mkdirs();

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

        String suffix;
        if (pluginId.contains("translator")) {
            suffix = " (Traduit)";
            Tag traduitTag = tagRepository.findById("Traduit").orElseGet(() -> tagRepository.save(new Tag("Traduit")));
            newBook.addTag(traduitTag);
        } else if (!sourceExt.equalsIgnoreCase(ext)) {
            suffix = " (" + ext.toUpperCase() + ")";
            Tag convertTag = tagRepository.findById("Converti").orElseGet(() -> tagRepository.save(new Tag("Converti")));
            newBook.addTag(convertTag);
        } else {
            suffix = " (Transformé)";
        }
        newBook.setTitle(book.getTitle() + suffix);
        newBook.setAuthor(book.getAuthor());
        newBook.setPublisher(book.getPublisher());
        newBook.setPublishDate(book.getPublishDate());
        newBook.setDescription(book.getDescription());
        newBook.setCoverPath(coverPathRel);

        for (Tag tag : book.getTags()) {
            newBook.addTag(tag);
        }

        BookFormat fmt = new BookFormat(ext, filename, destFileRel, fileSize);
        newBook.addFormat(fmt);

        return bookRepository.save(newBook);
    }

    @Transactional
    public Book updateReadingProgress(String bookId, int chapterIndex, int totalChapters) {
        Book book = bookRepository.findById(bookId)
                .orElseThrow(() -> new IllegalArgumentException("Livre introuvable: " + bookId));

        book.setCurrentChapterIndex(chapterIndex);
        book.setTotalChapters(totalChapters);
        double pct = (totalChapters > 0) ? Math.min(100.0, ((chapterIndex + 1.0) / totalChapters) * 100.0) : 0.0;
        book.setProgressPercent(Math.round(pct * 10.0) / 10.0);
        book.setLastReadAt(java.time.Instant.now().toString());
        if (chapterIndex >= totalChapters - 1 && totalChapters > 0) {
            book.setFinished(true);
        }

        return bookRepository.save(book);
    }

    @Transactional
    public void bulkAddTag(List<String> bookIds, String tagName) {
        if (bookIds == null || bookIds.isEmpty() || tagName == null || tagName.isBlank()) return;
        Tag tag = tagRepository.findById(tagName.trim())
                .orElseGet(() -> tagRepository.save(new Tag(tagName.trim())));

        for (String id : bookIds) {
            bookRepository.findById(id).ifPresent(b -> {
                b.addTag(tag);
                bookRepository.save(b);
            });
        }
    }

    @Transactional
    public void bulkDeleteBooks(List<String> bookIds) {
        if (bookIds == null || bookIds.isEmpty()) return;
        for (String id : bookIds) {
            try {
                deleteBook(id);
            } catch (Exception e) {
                log.warn("Failed to delete book {}: {}", id, e.getMessage());
            }
        }
    }

    public record ReadingStats(
            long totalBooks,
            long finishedBooks,
            long inProgressBooks,
            Map<String, Long> tagDistribution,
            List<Book> recentlyRead
    ) {}

    public ReadingStats getReadingStats() {
        List<Book> allBooks = bookRepository.findAll();
        long finished = allBooks.stream().filter(Book::isFinished).count();
        long inProgress = allBooks.stream().filter(b -> b.getProgressPercent() > 0 && !b.isFinished()).count();
        Map<String, Long> tags = getTagsWithCounts();

        List<Book> recent = allBooks.stream()
                .filter(b -> b.getLastReadAt() != null)
                .sorted(Comparator.comparing(Book::getLastReadAt).reversed())
                .limit(5)
                .toList();

        return new ReadingStats(allBooks.size(), finished, inProgress, tags, recent);
    }

    @Transactional
    public Book addFormatToBook(String bookId, File newFormatFile) throws IOException {
        Book book = bookRepository.findById(bookId)
                .orElseThrow(() -> new IllegalArgumentException("Livre introuvable: " + bookId));

        if (newFormatFile == null || !newFormatFile.exists()) {
            throw new IllegalArgumentException("Fichier source introuvable");
        }

        String ext = getExtension(newFormatFile.getName()).toLowerCase();
        if (ext.isBlank()) {
            throw new IllegalArgumentException("Le fichier n'a pas d'extension valide");
        }

        File bookDirAbs;
        if (!book.getFormats().isEmpty()) {
            BookFormat first = book.getFormats().get(0);
            File firstFile = new File(appProperties.getLibraryPath(), first.getFilepath());
            bookDirAbs = firstFile.getParentFile();
        } else {
            String cleanAuthor = sanitizeFilename(book.getAuthor());
            String cleanTitle = sanitizeFilename(book.getTitle());
            String rel = cleanAuthor + File.separator + cleanTitle + " (" + book.getId() + ")";
            bookDirAbs = new File(appProperties.getLibraryPath(), rel);
            bookDirAbs.mkdirs();
        }

        String targetFilename = sanitizeFilename(book.getTitle()) + "." + ext;
        File destFileAbs = new File(bookDirAbs, targetFilename);
        Files.copy(newFormatFile.toPath(), destFileAbs.toPath(), StandardCopyOption.REPLACE_EXISTING);

        File libDir = new File(appProperties.getLibraryPath());
        String destFileRel = libDir.toPath().relativize(destFileAbs.toPath()).toString();
        long fileSize = destFileAbs.length();

        book.getFormats().removeIf(f -> f.getFormat().equalsIgnoreCase(ext));
        BookFormat fmt = new BookFormat(ext, targetFilename, destFileRel, fileSize);
        book.addFormat(fmt);

        return bookRepository.save(book);
    }

    @Transactional
    public Book deleteFormatFromBook(String bookId, String formatExt) throws IOException {
        Book book = bookRepository.findById(bookId)
                .orElseThrow(() -> new IllegalArgumentException("Livre introuvable: " + bookId));

        if (book.getFormats().size() <= 1) {
            throw new IllegalStateException("Impossible de supprimer le seul format restant du livre.");
        }

        Optional<BookFormat> toRemove = book.getFormats().stream()
                .filter(f -> f.getFormat().equalsIgnoreCase(formatExt))
                .findFirst();

        if (toRemove.isPresent()) {
            BookFormat fmt = toRemove.get();
            File f = new File(appProperties.getLibraryPath(), fmt.getFilepath());
            if (f.exists()) {
                f.delete();
            }
            book.getFormats().remove(fmt);
            return bookRepository.save(book);
        }

        return book;
    }

    @Transactional
    public Book convertAndAttachFormat(String bookId, String targetFormat) throws Exception {
        Book book = bookRepository.findById(bookId)
                .orElseThrow(() -> new IllegalArgumentException("Livre introuvable: " + bookId));

        if (book.getFormats().isEmpty()) {
            throw new IllegalStateException("Aucun format disponible pour la conversion");
        }

        BookFormat sourceFmt = book.getFormats().stream()
                .filter(f -> conversionService.canConvert(f.getFormat(), targetFormat))
                .findFirst()
                .orElse(book.getFormats().get(0));

        File inputFile = getFileForFormat(sourceFmt);
        File convertedFile = conversionService.convert(inputFile, targetFormat, Map.of("title", book.getTitle(), "author", book.getAuthor()));

        return addFormatToBook(bookId, convertedFile);
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
