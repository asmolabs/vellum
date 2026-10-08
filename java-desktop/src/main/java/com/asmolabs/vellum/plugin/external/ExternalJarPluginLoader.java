package com.asmolabs.vellum.plugin.external;

import com.asmolabs.vellum.plugin.PluginDescriptor;
import com.asmolabs.vellum.plugin.PluginType;
import com.asmolabs.vellum.plugin.VellumPlugin;
import com.asmolabs.vellum.plugin.format.BookFormatPlugin;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.io.File;
import java.io.InputStream;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.net.URL;
import java.net.URLClassLoader;
import java.util.*;
import java.util.jar.JarEntry;
import java.util.jar.JarFile;

public class ExternalJarPluginLoader {

    private static final Logger log = LoggerFactory.getLogger(ExternalJarPluginLoader.class);

    public static List<VellumPlugin> loadFromJar(File jarFile) {
        List<VellumPlugin> loaded = new ArrayList<>();
        if (!jarFile.exists() || !jarFile.getName().endsWith(".jar")) {
            return loaded;
        }

        try {
            URL[] urls = new URL[]{jarFile.toURI().toURL()};
            URLClassLoader classLoader = new URLClassLoader(urls, ExternalJarPluginLoader.class.getClassLoader());

            // 1. ServiceLoader for VellumPlugin
            ServiceLoader<VellumPlugin> vellumLoader = ServiceLoader.load(VellumPlugin.class, classLoader);
            for (VellumPlugin plugin : vellumLoader) {
                loaded.add(wrapPlugin(plugin));
                log.info("Loaded external plugin via ServiceLoader<VellumPlugin>: {} from {}", plugin.getId(), jarFile.getName());
            }

            // 2. ServiceLoader for BookFormatPlugin
            ServiceLoader<BookFormatPlugin> formatLoader = ServiceLoader.load(BookFormatPlugin.class, classLoader);
            for (BookFormatPlugin formatPlugin : formatLoader) {
                if (loaded.stream().noneMatch(p -> p.getId().equalsIgnoreCase(formatPlugin.getId()))) {
                    loaded.add(wrapPlugin(formatPlugin));
                    log.info("Loaded external format plugin via ServiceLoader<BookFormatPlugin>: {} from {}", formatPlugin.getId(), jarFile.getName());
                }
            }

            // 3. Fallback: check META-INF/vellum-plugin.properties
            if (loaded.isEmpty()) {
                try (JarFile jf = new JarFile(jarFile)) {
                    JarEntry entry = jf.getJarEntry("META-INF/vellum-plugin.properties");
                    if (entry != null) {
                        Properties props = new Properties();
                        try (InputStream is = jf.getInputStream(entry)) {
                            props.load(is);
                        }
                        List<String> candidateClasses = new ArrayList<>();
                        for (String key : List.of("format.class", "plugin.class", "plugins", "plugins.classes")) {
                            String val = props.getProperty(key);
                            if (val != null && !val.isBlank()) {
                                for (String item : val.split("[,;]")) {
                                    if (!item.trim().isBlank()) {
                                        candidateClasses.add(item.trim());
                                    }
                                }
                            }
                        }

                        for (String className : candidateClasses) {
                            try {
                                Class<?> clazz = Class.forName(className, true, classLoader);
                                if (VellumPlugin.class.isAssignableFrom(clazz) && !Modifier.isAbstract(clazz.getModifiers())) {
                                    VellumPlugin plugin = (VellumPlugin) clazz.getDeclaredConstructor().newInstance();
                                    loaded.add(wrapPlugin(plugin));
                                    log.info("Loaded external plugin via properties: {} from {}", plugin.getId(), jarFile.getName());
                                }
                            } catch (Throwable t) {
                                log.warn("Failed loading class {} declared in properties of {}: {}", className, jarFile.getName(), t.getMessage());
                            }
                        }
                    }
                }
            }

            // 4. Fallback: scan all .class entries in the JAR
            if (loaded.isEmpty()) {
                try (JarFile jf = new JarFile(jarFile)) {
                    Enumeration<JarEntry> entries = jf.entries();
                    while (entries.hasMoreElements()) {
                        JarEntry entry = entries.nextElement();
                        String name = entry.getName();
                        if (name.endsWith(".class") && !name.contains("$") && !name.startsWith("META-INF/")) {
                            String className = name.substring(0, name.length() - 6).replace('/', '.');
                            try {
                                Class<?> clazz = Class.forName(className, false, classLoader);
                                if (VellumPlugin.class.isAssignableFrom(clazz) && !Modifier.isAbstract(clazz.getModifiers())) {
                                    VellumPlugin plugin = (VellumPlugin) clazz.getDeclaredConstructor().newInstance();
                                    loaded.add(wrapPlugin(plugin));
                                    log.info("Auto-discovered and loaded external plugin class {} from {}", className, jarFile.getName());
                                }
                            } catch (Throwable ignored) {
                                // Skip classes that cannot be initialized or do not have public default constructor
                            }
                        }
                    }
                }
            }
        } catch (Throwable t) {
            log.warn("Failed loading plugins from JAR {}: {}", jarFile.getName(), t.getMessage());
        }

        return loaded;
    }

    public static VellumPlugin wrapPlugin(VellumPlugin original) {
        if (original == null) return null;
        PluginDescriptor orig = original.getDescriptor();
        if (orig != null && orig.external()) {
            return original;
        }

        PluginType type = orig != null ? orig.type() : null;
        List<String> extensions = orig != null && orig.supportedExtensions() != null ? orig.supportedExtensions() : List.of();

        if (original instanceof BookFormatPlugin bfp) {
            type = PluginType.FORMAT;
            if (extensions.isEmpty()) {
                extensions = bfp.getSupportedExtensions();
            }
        } else if (type == null) {
            type = PluginType.TRANSFORMATION;
        }

        PluginDescriptor updatedDescriptor = new PluginDescriptor(
                orig != null ? orig.id() : original.getClass().getSimpleName().toLowerCase(),
                orig != null ? orig.name() : original.getClass().getSimpleName(),
                orig != null ? orig.description() : "Plugin Java externe",
                orig != null ? orig.version() : "1.0.0",
                type,
                true, // flagged as external
                orig != null ? orig.command() : null,
                extensions,
                orig != null && orig.configSchema() != null ? orig.configSchema() : List.of()
        );

        Set<Class<?>> interfaces = new LinkedHashSet<>();
        Class<?> cur = original.getClass();
        while (cur != null && cur != Object.class) {
            for (Class<?> iface : cur.getInterfaces()) {
                if (VellumPlugin.class.isAssignableFrom(iface)) {
                    interfaces.add(iface);
                }
            }
            cur = cur.getSuperclass();
        }
        interfaces.add(VellumPlugin.class);
        if (original instanceof BookFormatPlugin) {
            interfaces.add(BookFormatPlugin.class);
        }

        ClassLoader cl = original.getClass().getClassLoader() != null
                ? original.getClass().getClassLoader()
                : ExternalJarPluginLoader.class.getClassLoader();

        return (VellumPlugin) Proxy.newProxyInstance(
                cl,
                interfaces.toArray(new Class<?>[0]),
                (proxy, method, args) -> {
                    if ("getDescriptor".equals(method.getName()) && (args == null || args.length == 0)) {
                        return updatedDescriptor;
                    }
                    if ("getId".equals(method.getName()) && (args == null || args.length == 0)) {
                        return updatedDescriptor.id();
                    }
                    if ("getName".equals(method.getName()) && (args == null || args.length == 0)) {
                        return updatedDescriptor.name();
                    }
                    return method.invoke(original, args);
                }
        );
    }
}
