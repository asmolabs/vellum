package com.asmolabs.vellum.domain.entity;

import com.fasterxml.jackson.annotation.JsonIgnore;
import jakarta.persistence.*;
import java.util.Objects;
import java.util.UUID;

@Entity
@Table(name = "book_formats")
public class BookFormat {

    @Id
    @Column(name = "id", nullable = false)
    private String id = UUID.randomUUID().toString();

    @Column(name = "book_id", insertable = false, updatable = false)
    private String bookId;

    @Column(name = "format", nullable = false)
    private String format; // 'epub', 'mobi', 'azw3'

    @Column(name = "filename", nullable = false)
    private String filename;

    @Column(name = "filepath", nullable = false)
    private String filepath;

    @Column(name = "filesize", nullable = false)
    private Long filesize;

    @ManyToOne(fetch = FetchType.LAZY)
    @JoinColumn(name = "book_id", nullable = false)
    @JsonIgnore
    private Book book;

    public BookFormat() {
    }

    public BookFormat(String format, String filename, String filepath, Long filesize) {
        this.id = UUID.randomUUID().toString();
        this.format = format;
        this.filename = filename;
        this.filepath = filepath;
        this.filesize = filesize;
    }

    public String getId() {
        return id;
    }

    public void setId(String id) {
        this.id = id;
    }

    public String getBookId() {
        return bookId;
    }

    public void setBookId(String bookId) {
        this.bookId = bookId;
    }

    public String getFormat() {
        return format;
    }

    public void setFormat(String format) {
        this.format = format;
    }

    public String getFilename() {
        return filename;
    }

    public void setFilename(String filename) {
        this.filename = filename;
    }

    public String getFilepath() {
        return filepath;
    }

    public void setFilepath(String filepath) {
        this.filepath = filepath;
    }

    public Long getFilesize() {
        return filesize;
    }

    public void setFilesize(Long filesize) {
        this.filesize = filesize;
    }

    public Book getBook() {
        return book;
    }

    public void setBook(Book book) {
        this.book = book;
    }

    @Override
    public boolean equals(Object o) {
        if (this == o) return true;
        if (o == null || getClass() != o.getClass()) return false;
        BookFormat that = (BookFormat) o;
        return Objects.equals(id, that.id);
    }

    @Override
    public int hashCode() {
        return Objects.hash(id);
    }
}
