package com.asmolabs.vellum.plugin.external;

import com.asmolabs.vellum.plugin.VellumPlugin;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.io.File;
import java.io.InputStream;
import java.net.URL;
import java.net.URLClassLoader;
import java.util.*;
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

            // 1. Try Java ServiceLoader
            ServiceLoader<VellumPlugin> serviceLoader = ServiceLoader.load(VellumPlugin.class, classLoader);
            for (VellumPlugin plugin : serviceLoader) {
                loaded.add(plugin);
                log.info("Loaded external plugin via ServiceLoader: {} from {}", plugin.getId(), jarFile.getName());
            }

            // 2. Fallback: check META-INF/vellum-plugin.properties
            if (loaded.isEmpty()) {
                try (JarFile jf = new JarFile(jarFile)) {
                    var entry = jf.getJarEntry("META-INF/vellum-plugin.properties");
                    if (entry != null) {
                        Properties props = new Properties();
                        try (InputStream is = jf.getInputStream(entry)) {
                            props.load(is);
                        }
                        String className = props.getProperty("plugin.class");
                        if (className != null && !className.isBlank()) {
                            Class<?> clazz = Class.forName(className.trim(), true, classLoader);
                            if (VellumPlugin.class.isAssignableFrom(clazz)) {
                                VellumPlugin plugin = (VellumPlugin) clazz.getDeclaredConstructor().newInstance();
                                loaded.add(plugin);
                                log.info("Loaded external plugin via properties: {} from {}", plugin.getId(), jarFile.getName());
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
}
