package com.asmolabs.vellum.web.opds;

import com.asmolabs.vellum.domain.entity.Book;
import com.asmolabs.vellum.domain.entity.BookFormat;
import com.asmolabs.vellum.domain.entity.Tag;
import com.asmolabs.vellum.service.BookService;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.core.io.FileSystemResource;
import org.springframework.core.io.Resource;
import org.springframework.http.HttpHeaders;
import org.springframework.http.MediaType;
import org.springframework.http.ResponseEntity;
import org.springframework.web.bind.annotation.*;

import java.io.File;
import java.time.Instant;
import java.util.List;
import java.util.Optional;

@RestController
@RequestMapping("/opds")
public class OpdsController {

    private static final Logger log = LoggerFactory.getLogger(OpdsController.class);

    private final BookService bookService;

    public OpdsController(BookService bookService) {
        this.bookService = bookService;
    }

    @GetMapping(produces = "application/atom+xml;profile=opds-catalog;charset=UTF-8")
    public String getRootCatalog() {
        return """
            <?xml version="1.0" encoding="UTF-8"?>
            <feed xmlns="http://www.w3.org/2005/Atom"
                  xmlns:dc="http://purl.org/dc/elements/1.1/"
                  xmlns:opds="http://opds-spec.org/2010/catalog">
              <id>urn:vellum:root</id>
              <title>Vellum Desktop Ebook Library</title>
              <updated>%s</updated>
              <author><name>Vellum</name></author>
              <link rel="self" href="/opds" type="application/atom+xml;profile=opds-catalog"/>
              <link rel="start" href="/opds" type="application/atom+xml;profile=opds-catalog"/>
              
              <entry>
                <title>Tous les livres</title>
                <id>urn:vellum:all</id>
                <updated>%s</updated>
                <content type="text">Parcourir l'ensemble des livres du catalogue</content>
                <link rel="subsection" href="/opds/books" type="application/atom+xml;profile=opds-catalog"/>
              </entry>
            </feed>
            """.formatted(Instant.now().toString(), Instant.now().toString()).trim();
    }

    @GetMapping(value = "/books", produces = "application/atom+xml;profile=opds-catalog;charset=UTF-8")
    public String getBooksCatalog(
            @RequestParam(required = false) String search,
            @RequestParam(required = false) String tag
    ) {
        List<Book> books = bookService.getBooks(search, tag);
        StringBuilder sb = new StringBuilder();
        sb.append("""
            <?xml version="1.0" encoding="UTF-8"?>
            <feed xmlns="http://www.w3.org/2005/Atom"
                  xmlns:dc="http://purl.org/dc/elements/1.1/"
                  xmlns:opds="http://opds-spec.org/2010/catalog">
              <id>urn:vellum:books</id>
              <title>Vellum - Catalogue</title>
              <updated>""").append(Instant.now().toString()).append("""
              </updated>
              <author><name>Vellum</name></author>
              <link rel="self" href="/opds/books" type="application/atom+xml;profile=opds-catalog"/>
              <link rel="start" href="/opds" type="application/atom+xml;profile=opds-catalog"/>
            """);

        for (Book b : books) {
            sb.append("  <entry>\n");
            sb.append("    <title>").append(escapeXml(b.getTitle())).append("</title>\n");
            sb.append("    <id>urn:vellum:book:").append(b.getId()).append("</id>\n");
            sb.append("    <updated>").append(b.getAddedAt() != null ? b.getAddedAt() : Instant.now().toString()).append("</updated>\n");
            sb.append("    <author><name>").append(escapeXml(b.getAuthor())).append("</name></author>\n");

            if (b.getPublisher() != null && !b.getPublisher().isBlank()) {
                sb.append("    <dc:publisher>").append(escapeXml(b.getPublisher())).append("</dc:publisher>\n");
            }
            if (b.getPublishDate() != null && !b.getPublishDate().isBlank()) {
                sb.append("    <dc:date>").append(escapeXml(b.getPublishDate())).append("</dc:date>\n");
            }
            if (b.getDescription() != null && !b.getDescription().isBlank()) {
                sb.append("    <summary type=\"text\">").append(escapeXml(b.getDescription())).append("</summary>\n");
            }

            for (Tag t : b.getTags()) {
                sb.append("    <category term=\"").append(escapeXml(t.getName())).append("\" label=\"").append(escapeXml(t.getName())).append("\"/>\n");
            }

            // Cover links
            if (b.getCoverPath() != null) {
                sb.append("    <link rel=\"http://opds-spec.org/image\" href=\"/opds/covers/").append(b.getId()).append("\" type=\"image/jpeg\"/>\n");
                sb.append("    <link rel=\"http://opds-spec.org/image/thumbnail\" href=\"/opds/covers/").append(b.getId()).append("\" type=\"image/jpeg\"/>\n");
            }

            // Acquisition Download links for each format
            for (BookFormat fmt : b.getFormats()) {
                String mime = getMimeType(fmt.getFormat());
                sb.append("    <link rel=\"http://opds-spec.org/acquisition\" href=\"/opds/download/").append(b.getId()).append("/").append(fmt.getFormat().toLowerCase()).append("\" type=\"").append(mime).append("\" length=\"").append(fmt.getFilesize()).append("\"/>\n");
            }

            sb.append("  </entry>\n");
        }

        sb.append("</feed>");
        return sb.toString();
    }

    @GetMapping("/covers/{bookId}")
    public ResponseEntity<Resource> getCover(@PathVariable String bookId) {
        Optional<Book> bookOpt = bookService.getBookById(bookId);
        if (bookOpt.isEmpty()) {
            return ResponseEntity.notFound().build();
        }

        File coverFile = bookService.getCoverFile(bookOpt.get());
        if (coverFile == null || !coverFile.exists()) {
            return ResponseEntity.notFound().build();
        }

        String ext = coverFile.getName().toLowerCase();
        MediaType mediaType = ext.endsWith(".png") ? MediaType.IMAGE_PNG : MediaType.IMAGE_JPEG;
        return ResponseEntity.ok()
                .contentType(mediaType)
                .body(new FileSystemResource(coverFile));
    }

    @GetMapping("/download/{bookId}/{format}")
    public ResponseEntity<Resource> downloadBook(
            @PathVariable String bookId,
            @PathVariable String format
    ) {
        Optional<Book> bookOpt = bookService.getBookById(bookId);
        if (bookOpt.isEmpty()) {
            return ResponseEntity.notFound().build();
        }

        Book book = bookOpt.get();
        Optional<BookFormat> fmtOpt = book.getFormats().stream()
                .filter(f -> f.getFormat().equalsIgnoreCase(format))
                .findFirst();

        if (fmtOpt.isEmpty()) {
            return ResponseEntity.notFound().build();
        }

        File file = bookService.getFileForFormat(fmtOpt.get());
        if (!file.exists()) {
            return ResponseEntity.notFound().build();
        }

        return ResponseEntity.ok()
                .header(HttpHeaders.CONTENT_DISPOSITION, "attachment; filename=\"" + fmtOpt.get().getFilename() + "\"")
                .contentType(MediaType.parseMediaType(getMimeType(format)))
                .contentLength(file.length())
                .body(new FileSystemResource(file));
    }

    private String getMimeType(String format) {
        return switch (format.toLowerCase()) {
            case "epub" -> "application/epub+zip";
            case "pdf" -> "application/pdf";
            case "cbz" -> "application/x-cbz";
            case "cbr" -> "application/x-cbr";
            case "mobi" -> "application/x-mobipocket-ebook";
            case "azw3" -> "application/vnd.amazon.ebook";
            default -> "application/octet-stream";
        };
    }

    private String escapeXml(String text) {
        if (text == null) return "";
        return text.replace("&", "&amp;")
                .replace("<", "&lt;")
                .replace(">", "&gt;")
                .replace("\"", "&quot;")
                .replace("'", "&apos;");
    }
}
