package com.asmolabs.vellum.plugin.device;

import com.asmolabs.vellum.domain.entity.Book;
import com.asmolabs.vellum.plugin.VellumPlugin;

import java.io.File;
import java.util.List;
import java.util.Map;
import java.util.Optional;

public interface DeviceSyncPlugin extends VellumPlugin {

    DeviceKind getDeviceKind();

    List<File> getDefaultMountPaths();

    Optional<File> detectDevice(File customMount);

    List<DeviceBookInfo> listBooks(File mountPoint) throws Exception;

    File exportBook(Book book, File sourceFile, File mountPoint, Map<String, Object> config) throws Exception;

    List<DeviceReadingProgress> getReadingProgress(File mountPoint) throws Exception;

    List<DeviceAnnotation> getAnnotations(File mountPoint) throws Exception;
}
