package com.asmolabs.vellum.service;

import com.asmolabs.vellum.converter.ConversionService;
import com.asmolabs.vellum.domain.entity.Book;
import com.asmolabs.vellum.domain.entity.BookFormat;
import com.asmolabs.vellum.plugin.device.*;
import com.asmolabs.vellum.repository.BookRepository;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.stereotype.Service;

import java.io.File;
import java.util.*;

@Service
public class DeviceService {

    private static final Logger log = LoggerFactory.getLogger(DeviceService.class);

    private final BookService bookService;
    private final BookRepository bookRepository;
    private final PluginService pluginService;
    private final ConversionService conversionService;

    @Autowired
    public DeviceService(
            BookService bookService,
            BookRepository bookRepository,
            PluginService pluginService,
            @Autowired(required = false) ConversionService conversionService
    ) {
        this.bookService = bookService;
        this.bookRepository = bookRepository;
        this.pluginService = pluginService;
        this.conversionService = conversionService;
    }

    public List<ConnectedDevice> detectConnectedDevices() {
        List<ConnectedDevice> devices = new ArrayList<>();
        List<DeviceSyncPlugin> plugins = pluginService.getPluginsOfType(DeviceSyncPlugin.class);

        for (DeviceSyncPlugin p : plugins) {
            if (!pluginService.isPluginEnabled(p.getId())) {
                continue;
            }

            Map<String, Object> config = pluginService.getPluginParams(p.getId());
            String customStr = String.valueOf(config.getOrDefault("customMountPath", "")).trim();
            File custom = !customStr.isBlank() ? new File(customStr) : null;

            Optional<File> mountOpt = p.detectDevice(custom);
            if (mountOpt.isPresent()) {
                File mount = mountOpt.get();
                int count = 0;
                try {
                    count = p.listBooks(mount).size();
                } catch (Exception e) {
                    log.warn("Failed to count books on {}: {}", p.getName(), e.getMessage());
                }

                devices.add(new ConnectedDevice(
                        p.getDeviceKind(),
                        p.getName(),
                        mount,
                        count
                ));
            }
        }

        return devices;
    }

    public Optional<DeviceSyncPlugin> getPluginForKind(DeviceKind kind) {
        return pluginService.getPluginsOfType(DeviceSyncPlugin.class).stream()
                .filter(p -> p.getDeviceKind() == kind && pluginService.isPluginEnabled(p.getId()))
                .findFirst();
    }

    public List<DeviceBookInfo> listDeviceBooks(DeviceKind kind, File customMount) throws Exception {
        DeviceSyncPlugin plugin = getPluginForKind(kind)
                .orElseThrow(() -> new IllegalArgumentException("Aucun plugin activé pour la liseuse " + kind.getDisplayName()));

        File mount = plugin.detectDevice(customMount)
                .orElseThrow(() -> new IllegalStateException("Liseuse " + kind.getDisplayName() + " non connectée."));

        return plugin.listBooks(mount);
    }

    public File exportBookToDevice(String bookId, DeviceKind kind, File customMount) throws Exception {
        DeviceSyncPlugin plugin = getPluginForKind(kind)
                .orElseThrow(() -> new IllegalArgumentException("Plugin non disponible pour " + kind.getDisplayName()));

        File mount = plugin.detectDevice(customMount)
                .orElseThrow(() -> new IllegalStateException("Liseuse " + kind.getDisplayName() + " non connectée."));

        Book book = bookRepository.findById(bookId)
                .orElseThrow(() -> new IllegalArgumentException("Livre introuvable: " + bookId));

        if (book.getFormats().isEmpty()) {
            throw new IllegalStateException("Le livre n'a aucun fichier disponible.");
        }

        Map<String, Object> config = pluginService.getPluginParams(plugin.getId());

        // Select optimal source format for device
        String preferredFormat = kind == DeviceKind.KOBO ? "epub" : "azw3";

        BookFormat sourceFormat = null;
        for (BookFormat fmt : book.getFormats()) {
            if (fmt.getFormat().equalsIgnoreCase(preferredFormat)) {
                sourceFormat = fmt;
                break;
            }
        }

        // Kindle fallbacks
        if (sourceFormat == null && kind == DeviceKind.KINDLE) {
            for (BookFormat fmt : book.getFormats()) {
                if (fmt.getFormat().equalsIgnoreCase("mobi") || fmt.getFormat().equalsIgnoreCase("pdf")) {
                    sourceFormat = fmt;
                    break;
                }
            }
        }

        // If preferred not found, check if we can convert on-the-fly
        File sourceFile;
        if (sourceFormat != null) {
            sourceFile = bookService.getFileForFormat(sourceFormat);
        } else {
            BookFormat first = book.getFormats().getFirst();
            File firstFile = bookService.getFileForFormat(first);

            if (conversionService != null && conversionService.canConvert(first.getFormat(), preferredFormat)) {
                log.info("Converting '{}' from {} to {} for eReader export", book.getTitle(), first.getFormat(), preferredFormat);
                sourceFile = conversionService.convert(firstFile, preferredFormat);
            } else {
                sourceFile = firstFile;
            }
        }

        return plugin.exportBook(book, sourceFile, mount, config);
    }

    public List<Book> importBooksFromDevice(DeviceKind kind, File customMount) throws Exception {
        DeviceSyncPlugin plugin = getPluginForKind(kind)
                .orElseThrow(() -> new IllegalArgumentException("Plugin introuvable pour " + kind.getDisplayName()));

        File mount = plugin.detectDevice(customMount)
                .orElseThrow(() -> new IllegalStateException("Liseuse " + kind.getDisplayName() + " non connectée."));

        List<DeviceBookInfo> deviceBooks = plugin.listBooks(mount);
        List<Book> allExisting = bookRepository.findAll();
        List<Book> imported = new ArrayList<>();

        for (DeviceBookInfo dBook : deviceBooks) {
            File fullPath = new File(dBook.relativePath());
            if (!fullPath.isAbsolute()) {
                fullPath = new File(mount, dBook.relativePath());
            }

            if (!fullPath.exists()) {
                continue;
            }

            boolean alreadyExists = allExisting.stream()
                    .anyMatch(b -> b.getTitle().equalsIgnoreCase(dBook.title()) || b.getTitle().equalsIgnoreCase(dBook.filename()));

            if (!alreadyExists) {
                try {
                    Book importedBook = bookService.importBook(fullPath);
                    imported.add(importedBook);
                    log.info("Imported book from {}: {}", kind.getDisplayName(), importedBook.getTitle());
                } catch (Exception e) {
                    log.warn("Failed to import book {} from device: {}", fullPath.getName(), e.getMessage());
                }
            }
        }

        return imported;
    }

    public List<DeviceReadingProgress> syncReadingProgress(DeviceKind kind, File customMount) throws Exception {
        DeviceSyncPlugin plugin = getPluginForKind(kind)
                .orElseThrow(() -> new IllegalArgumentException("Plugin introuvable pour " + kind.getDisplayName()));

        File mount = plugin.detectDevice(customMount)
                .orElseThrow(() -> new IllegalStateException("Liseuse " + kind.getDisplayName() + " non connectée."));

        List<DeviceReadingProgress> progresses = plugin.getReadingProgress(mount);
        List<Book> allBooks = bookRepository.findAll();
        List<DeviceReadingProgress> updated = new ArrayList<>();

        for (DeviceReadingProgress p : progresses) {
            for (Book b : allBooks) {
                if (b.getTitle().equalsIgnoreCase(p.bookTitle()) || p.bookTitle().toLowerCase().contains(b.getTitle().toLowerCase())) {
                    if (p.progressPercent() > b.getProgressPercent()) {
                        b.setProgressPercent(p.progressPercent());
                        b.setFinished(p.isFinished() || p.progressPercent() >= 99.0);
                        bookRepository.save(b);
                        updated.add(p);
                    }
                }
            }
        }

        return updated;
    }

    public List<DeviceAnnotation> getAnnotations(DeviceKind kind, File customMount) throws Exception {
        DeviceSyncPlugin plugin = getPluginForKind(kind)
                .orElseThrow(() -> new IllegalArgumentException("Plugin introuvable pour " + kind.getDisplayName()));

        File mount = plugin.detectDevice(customMount)
                .orElseThrow(() -> new IllegalStateException("Liseuse " + kind.getDisplayName() + " non connectée."));

        return plugin.getAnnotations(mount);
    }

    public List<DeviceAnnotation> getAnnotationsForBook(String bookTitle, String bookAuthor) {
        List<DeviceAnnotation> result = new ArrayList<>();
        List<ConnectedDevice> connected = detectConnectedDevices();

        for (ConnectedDevice dev : connected) {
            try {
                List<DeviceAnnotation> all = getAnnotations(dev.kind(), dev.mountPoint());
                for (DeviceAnnotation ann : all) {
                    boolean titleMatches = ann.bookTitle() != null &&
                            (ann.bookTitle().equalsIgnoreCase(bookTitle)
                                    || ann.bookTitle().toLowerCase().contains(bookTitle.toLowerCase())
                                    || bookTitle.toLowerCase().contains(ann.bookTitle().toLowerCase()));
                    if (titleMatches) {
                        result.add(ann);
                    }
                }
            } catch (Exception e) {
                log.warn("Failed to fetch annotations from {}: {}", dev.name(), e.getMessage());
            }
        }

        return result;
    }
}
