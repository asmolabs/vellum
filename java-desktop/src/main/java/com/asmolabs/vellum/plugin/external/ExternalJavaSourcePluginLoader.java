package com.asmolabs.vellum.plugin.external;

import com.asmolabs.vellum.plugin.VellumPlugin;
import com.asmolabs.vellum.plugin.format.BookFormatPlugin;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import javax.tools.*;
import java.io.File;
import java.lang.reflect.Modifier;
import java.net.URL;
import java.net.URLClassLoader;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.util.*;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

public class ExternalJavaSourcePluginLoader {

    private static final Logger log = LoggerFactory.getLogger(ExternalJavaSourcePluginLoader.class);

    private static final Pattern PACKAGE_PATTERN = Pattern.compile("^\\s*package\\s+([a-zA-Z0-9_.]+)\\s*;", Pattern.MULTILINE);
    private static final Pattern CLASS_PATTERN = Pattern.compile("(?:public\\s+)?class\\s+([A-Za-z0-9_]+)");

    public static List<VellumPlugin> loadFromSource(File pluginsDir) {
        List<VellumPlugin> loaded = new ArrayList<>();
        if (pluginsDir == null || !pluginsDir.exists() || !pluginsDir.isDirectory()) {
            return loaded;
        }

        List<File> javaFiles = findJavaFiles(pluginsDir);
        if (javaFiles.isEmpty()) {
            return loaded;
        }

        JavaCompiler compiler = ToolProvider.getSystemJavaCompiler();
        if (compiler == null) {
            log.warn("JDK JavaCompiler not found (running under JRE?). Dynamic .java plugins will not be compiled.");
            return loaded;
        }

        File cacheDir = new File(pluginsDir, ".cache/java-classes");
        if (!cacheDir.exists()) {
            cacheDir.mkdirs();
        }

        // Determine which files require compilation
        List<File> toCompile = new ArrayList<>();
        for (File f : javaFiles) {
            if (needsRecompilation(f, cacheDir)) {
                toCompile.add(f);
            }
        }

        if (!toCompile.isEmpty()) {
            boolean success = compileFiles(compiler, toCompile, cacheDir, pluginsDir);
            if (!success) {
                log.warn("Dynamic compilation failed for some .java files in {}", pluginsDir.getAbsolutePath());
            }
        }

        // Load classes from cacheDir
        try {
            URL[] urls = new URL[]{cacheDir.toURI().toURL()};
            URLClassLoader classLoader = new URLClassLoader(urls, ExternalJavaSourcePluginLoader.class.getClassLoader());

            for (File javaFile : javaFiles) {
                try {
                    String content = Files.readString(javaFile.toPath(), StandardCharsets.UTF_8);
                    Matcher pkgMatcher = PACKAGE_PATTERN.matcher(content);
                    String pkg = pkgMatcher.find() ? pkgMatcher.group(1).trim() : null;

                    Matcher clsMatcher = CLASS_PATTERN.matcher(content);
                    while (clsMatcher.find()) {
                        String simpleName = clsMatcher.group(1);
                        String fqcn = (pkg != null && !pkg.isBlank()) ? pkg + "." + simpleName : simpleName;

                        try {
                            Class<?> clazz = Class.forName(fqcn, true, classLoader);
                            if (VellumPlugin.class.isAssignableFrom(clazz) && !Modifier.isAbstract(clazz.getModifiers())) {
                                VellumPlugin plugin = (VellumPlugin) clazz.getDeclaredConstructor().newInstance();
                                VellumPlugin wrapped = ExternalJarPluginLoader.wrapPlugin(plugin);
                                loaded.add(wrapped);
                                log.info("Dynamically compiled and loaded external Java plugin: {} ({}) from {}",
                                        wrapped.getName(), wrapped.getId(), javaFile.getName());
                            }
                        } catch (ClassNotFoundException ignored) {
                            // Class not in output (might have failed compilation)
                        } catch (Throwable t) {
                            log.warn("Failed instantiating dynamic plugin class {} from {}: {}", fqcn, javaFile.getName(), t.getMessage());
                        }
                    }
                } catch (Exception e) {
                    log.warn("Error parsing dynamic Java source {}: {}", javaFile.getName(), e.getMessage());
                }
            }
        } catch (Exception e) {
            log.error("Failed creating ClassLoader for dynamic Java plugins: {}", e.getMessage(), e);
        }

        return loaded;
    }

    private static List<File> findJavaFiles(File dir) {
        List<File> result = new ArrayList<>();
        File[] files = dir.listFiles();
        if (files == null) return result;

        for (File file : files) {
            if (file.isDirectory()) {
                if (!file.getName().startsWith(".") && !file.getName().equalsIgnoreCase("build") && !file.getName().equalsIgnoreCase("target")) {
                    File[] subFiles = file.listFiles((d, name) -> name.endsWith(".java"));
                    if (subFiles != null) {
                        result.addAll(Arrays.asList(subFiles));
                    }
                }
            } else if (file.getName().endsWith(".java")) {
                result.add(file);
            }
        }
        return result;
    }

    private static boolean needsRecompilation(File javaFile, File cacheDir) {
        String baseName = javaFile.getName().substring(0, javaFile.getName().length() - 5);
        List<File> matches = findClassFilesNamed(cacheDir, baseName + ".class");
        if (matches.isEmpty()) {
            return true;
        }
        for (File cf : matches) {
            if (javaFile.lastModified() > cf.lastModified()) {
                return true;
            }
        }
        return false;
    }

    private static List<File> findClassFilesNamed(File dir, String filename) {
        List<File> matches = new ArrayList<>();
        File[] files = dir.listFiles();
        if (files == null) return matches;
        for (File f : files) {
            if (f.isDirectory()) {
                matches.addAll(findClassFilesNamed(f, filename));
            } else if (f.getName().equals(filename)) {
                matches.add(f);
            }
        }
        return matches;
    }

    private static boolean compileFiles(JavaCompiler compiler, List<File> files, File outputDir, File pluginsDir) {
        DiagnosticCollector<JavaFileObject> diagnostics = new DiagnosticCollector<>();
        try (StandardJavaFileManager fileManager = compiler.getStandardFileManager(diagnostics, null, StandardCharsets.UTF_8)) {
            Iterable<? extends JavaFileObject> compilationUnits = fileManager.getJavaFileObjectsFromFiles(files);

            StringBuilder cp = new StringBuilder();
            String sysCp = System.getProperty("java.class.path");
            if (sysCp != null && !sysCp.isBlank()) {
                cp.append(sysCp);
            }

            // Add BookFormatPlugin and VellumPlugin locations
            for (Class<?> clazz : List.of(BookFormatPlugin.class, VellumPlugin.class)) {
                try {
                    URL loc = clazz.getProtectionDomain().getCodeSource().getLocation();
                    if (loc != null) {
                        File locFile = new File(loc.toURI());
                        if (!cp.isEmpty()) cp.append(File.pathSeparator);
                        cp.append(locFile.getAbsolutePath());
                    }
                } catch (Exception ignored) {}
            }

            // Include any JARs in plugins directory
            File[] jarFiles = pluginsDir.listFiles((d, name) -> name.endsWith(".jar"));
            if (jarFiles != null) {
                for (File jar : jarFiles) {
                    if (!cp.isEmpty()) cp.append(File.pathSeparator);
                    cp.append(jar.getAbsolutePath());
                }
            }

            List<String> options = List.of(
                    "-d", outputDir.getAbsolutePath(),
                    "-cp", cp.toString()
            );

            JavaCompiler.CompilationTask task = compiler.getTask(null, fileManager, diagnostics, options, null, compilationUnits);
            boolean success = Boolean.TRUE.equals(task.call());
            if (!success) {
                for (Diagnostic<? extends JavaFileObject> d : diagnostics.getDiagnostics()) {
                    log.error("Compilation error in {}:{}: {}",
                            d.getSource() != null ? d.getSource().getName() : "?",
                            d.getLineNumber(),
                            d.getMessage(Locale.getDefault()));
                }
            }
            return success;
        } catch (Exception e) {
            log.error("Failed executing dynamic Java compilation: {}", e.getMessage(), e);
            return false;
        }
    }
}
