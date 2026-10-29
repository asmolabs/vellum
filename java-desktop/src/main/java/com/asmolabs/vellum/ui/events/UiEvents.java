package com.asmolabs.vellum.ui.events;

import com.asmolabs.vellum.domain.entity.Book;

public class UiEvents {

    public record BookSelected(Book book) {}
    public record BookOpened(Book book) {}
    public record BookEditRequested(Book book) {}
    public record BookDeleteRequested(Book book) {}
    public record BookPluginRequested(Book book, String pluginId) {}
    public record LibraryChanged() {}
    public record FilterChanged(String search, String tag, String status) {
        public FilterChanged(String search, String tag) {
            this(search, tag, null);
        }
    }
    public record OpenSettingsRequested() {}
    public record OpenStatsRequested() {}
    public record DeviceSyncRequested() {}
}
