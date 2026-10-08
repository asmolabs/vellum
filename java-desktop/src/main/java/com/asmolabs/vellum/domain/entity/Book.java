package com.asmolabs.vellum.domain.entity;

import jakarta.persistence.*;
import java.time.Instant;
import java.util.*;

@Entity
@Table(name = "books")
public class Book {

    @Id
    @Column(name = "id", nullable = false)
    private String id = UUID.randomUUID().toString();

    @Column(name = "title", nullable = false)
    private String title;

    @Column(name = "author", nullable = false)
    private String author;

    @Column(name = "publisher")
    private String publisher;

    @Column(name = "publishDate")
    private String publishDate;

    @Column(name = "description", length = 4000)
    private String description;

    @Column(name = "coverPath")
    private String coverPath;

    @Column(name = "addedAt")
    private String addedAt = Instant.now().toString();

    @Column(name = "currentChapterIndex")
    private Integer currentChapterIndex = 0;

    @Column(name = "totalChapters")
    private Integer totalChapters = 0;

    @Column(name = "progressPercent")
    private Double progressPercent = 0.0;

    @Column(name = "lastReadAt")
    private String lastReadAt;

    @Column(name = "isFinished")
    private Boolean isFinished = false;

    @Column(name = "rating")
    private Integer rating;

    @OneToMany(mappedBy = "book", cascade = CascadeType.ALL, orphanRemoval = true, fetch = FetchType.EAGER)
    private List<BookFormat> formats = new ArrayList<>();

    @ManyToMany(fetch = FetchType.EAGER)
    @JoinTable(
            name = "book_tags",
            joinColumns = @JoinColumn(name = "book_id"),
            inverseJoinColumns = @JoinColumn(name = "tag_name")
    )
    private Set<Tag> tags = new HashSet<>();

    public Book() {
    }

    public Book(String title, String author) {
        this.id = UUID.randomUUID().toString();
        this.title = title;
        this.author = author;
        this.addedAt = Instant.now().toString();
    }

    public void addFormat(BookFormat format) {
        formats.add(format);
        format.setBook(this);
    }

    public void removeFormat(BookFormat format) {
        formats.remove(format);
        format.setBook(null);
    }

    public void addTag(Tag tag) {
        tags.add(tag);
    }

    public void removeTag(Tag tag) {
        tags.remove(tag);
    }

    // Getters and Setters
    public String getId() {
        return id;
    }

    public void setId(String id) {
        this.id = id;
    }

    public String getTitle() {
        return title;
    }

    public void setTitle(String title) {
        this.title = title;
    }

    public String getAuthor() {
        return author;
    }

    public void setAuthor(String author) {
        this.author = author;
    }

    public String getPublisher() {
        return publisher;
    }

    public void setPublisher(String publisher) {
        this.publisher = publisher;
    }

    public String getPublishDate() {
        return publishDate;
    }

    public void setPublishDate(String publishDate) {
        this.publishDate = publishDate;
    }

    public String getDescription() {
        return description;
    }

    public void setDescription(String description) {
        this.description = description;
    }

    public String getCoverPath() {
        return coverPath;
    }

    public void setCoverPath(String coverPath) {
        this.coverPath = coverPath;
    }

    public String getAddedAt() {
        return addedAt;
    }

    public void setAddedAt(String addedAt) {
        this.addedAt = addedAt;
    }

    public int getCurrentChapterIndex() {
        return currentChapterIndex != null ? currentChapterIndex : 0;
    }

    public void setCurrentChapterIndex(Integer currentChapterIndex) {
        this.currentChapterIndex = currentChapterIndex;
    }

    public int getTotalChapters() {
        return totalChapters != null ? totalChapters : 0;
    }

    public void setTotalChapters(Integer totalChapters) {
        this.totalChapters = totalChapters;
    }

    public double getProgressPercent() {
        return progressPercent != null ? progressPercent : 0.0;
    }

    public void setProgressPercent(Double progressPercent) {
        this.progressPercent = progressPercent;
    }

    public String getLastReadAt() {
        return lastReadAt;
    }

    public void setLastReadAt(String lastReadAt) {
        this.lastReadAt = lastReadAt;
    }

    public boolean isFinished() {
        return isFinished != null && isFinished;
    }

    public void setFinished(Boolean finished) {
        this.isFinished = finished;
    }

    public Integer getRating() {
        return rating;
    }

    public void setRating(Integer rating) {
        this.rating = rating;
    }

    public List<BookFormat> getFormats() {
        return formats;
    }

    public void setFormats(List<BookFormat> formats) {
        this.formats = formats;
    }

    public Set<Tag> getTags() {
        return tags;
    }

    public void setTags(Set<Tag> tags) {
        this.tags = tags;
    }

    @Override
    public boolean equals(Object o) {
        if (this == o) return true;
        if (o == null || getClass() != o.getClass()) return false;
        Book book = (Book) o;
        return Objects.equals(id, book.id);
    }

    @Override
    public int hashCode() {
        return Objects.hash(id);
    }
}
