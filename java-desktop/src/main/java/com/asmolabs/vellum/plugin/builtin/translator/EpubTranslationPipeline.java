package com.asmolabs.vellum.plugin.builtin.translator;

import org.jsoup.nodes.Document;
import org.jsoup.nodes.Element;
import org.jsoup.nodes.TextNode;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.io.File;
import java.io.FileOutputStream;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.time.Duration;
import java.util.*;
import java.util.regex.Matcher;
import java.util.regex.Pattern;
import java.util.zip.CRC32;
import java.util.zip.ZipEntry;
import java.util.zip.ZipFile;
import java.util.zip.ZipOutputStream;

public class EpubTranslationPipeline {

    private static final Logger log = LoggerFactory.getLogger(EpubTranslationPipeline.class);

    public record PipelineStats(
            int totalChapters,
            int chaptersTranslated,
            int cacheHits,
            int segmentsTranslated,
            int segmentErrors
    ) {}

    public static PipelineStats translateEpub(File inputFile, File outputFile, Map<String, Object> config) throws Exception {
        long startTime = System.currentTimeMillis();
        String filename = inputFile.getName();
        String baseName = filename.replaceFirst("[.][^.]+$", "");
        File parentDir = inputFile.getParentFile();

        // 1. Configure paths & cache
        File cacheDir = new File(parentDir, ".cache_traduction_" + baseName);
        File segmentsCacheDir = new File(cacheDir, "segments");
        segmentsCacheDir.mkdirs();

        File charterFile = new File(parentDir, "editorial_charter_" + baseName + ".txt");
        File glossaryFile = new File(parentDir, "glossary_" + baseName + ".txt");

        // 2. Resolve LLM client
        LlmClient llmClient = buildLlmClient(config);

        TranslationSegmentCache segmentCache = new TranslationSegmentCache(segmentsCacheDir);
        EditorialCharterPlanner charterPlanner = new EditorialCharterPlanner(llmClient, charterFile);
        GlossaryManager glossaryManager = new GlossaryManager(llmClient, glossaryFile);
        QualityAssuranceValidator qaValidator = new QualityAssuranceValidator(llmClient);

        boolean enableCorrection = Boolean.parseBoolean(String.valueOf(config.getOrDefault("enable_correction", "false")));
        boolean enableQa = Boolean.parseBoolean(String.valueOf(config.getOrDefault("enable_qa", "false")));
        boolean enablePlanner = Boolean.parseBoolean(String.valueOf(config.getOrDefault("enable_planner", "true")));
        boolean enableGlossary = Boolean.parseBoolean(String.valueOf(config.getOrDefault("enable_glossary", "true")));
        int chunkSize = 500;
        if (config.containsKey("chunk_size")) {
            try {
                chunkSize = ((Number) config.get("chunk_size")).intValue();
            } catch (Exception ignored) {}
        }
        String targetLanguage = String.valueOf(config.getOrDefault("target_language", "Français"));
        String customPrompt = String.valueOf(config.getOrDefault("custom_prompt", ""));

        // 3. Inspect EPUB to extract Title, Author, Excerpt
        String title = baseName;
        String author = "Inconnu";
        String bookExcerpt = "";

        try (ZipFile zipIn = new ZipFile(inputFile)) {
            String opfPath = findOpfPath(zipIn);
            if (opfPath != null) {
                ZipEntry opfEntry = zipIn.getEntry(opfPath);
                if (opfEntry != null) {
                    try (InputStream is = zipIn.getInputStream(opfEntry)) {
                        String opfXml = new String(is.readAllBytes(), StandardCharsets.UTF_8);
                        Matcher tMatcher = Pattern.compile("<dc:title[^>]*>(.*?)</dc:title>", Pattern.CASE_INSENSITIVE).matcher(opfXml);
                        if (tMatcher.find()) title = tMatcher.group(1).trim();

                        Matcher aMatcher = Pattern.compile("<dc:creator[^>]*>(.*?)</dc:creator>", Pattern.CASE_INSENSITIVE).matcher(opfXml);
                        if (aMatcher.find()) author = aMatcher.group(1).trim();
                    }
                }
            }

            // Find first real chapter text for excerpt
            List<? extends ZipEntry> htmlEntries = zipIn.stream()
                    .filter(e -> !e.isDirectory() && (e.getName().endsWith(".xhtml") || e.getName().endsWith(".html")) && !e.getName().toLowerCase().contains("toc"))
                    .toList();

            if (!htmlEntries.isEmpty()) {
                try (InputStream is = zipIn.getInputStream(htmlEntries.get(0))) {
                    String firstChapterHtml = new String(is.readAllBytes(), StandardCharsets.UTF_8);
                    bookExcerpt = EpubHtmlParser.extractVisibleText(firstChapterHtml);
                }
            }
        }

        log.info("Livre détecté pour traduction : « {} » par {}", title, author);

        // 4. Initialize charter and glossary
        String charter = enablePlanner ? charterPlanner.getOrGenerateCharter(title, author, bookExcerpt) : EditorialCharterPlanner.getDefaultCharter();
        String glossary = enableGlossary ? glossaryManager.getOrGenerateGlossary(title, author) : GlossaryManager.getDefaultGlossary();

        int totalChapters = 0;
        int chaptersTranslated = 0;
        int cacheHits = 0;
        int segmentsTranslated = 0;
        int segmentErrors = 0;

        // 5. Execute translation and package output EPUB
        try (ZipFile zipIn = new ZipFile(inputFile);
             ZipOutputStream zos = new ZipOutputStream(new FileOutputStream(outputFile))) {

            List<? extends ZipEntry> allEntries = zipIn.stream().toList();

            // First pass: copy mimetype uncompressed if present
            ZipEntry mimeEntry = zipIn.getEntry("mimetype");
            if (mimeEntry != null) {
                byte[] mimeBytes;
                try (InputStream is = zipIn.getInputStream(mimeEntry)) {
                    mimeBytes = is.readAllBytes();
                }
                ZipEntry newMime = new ZipEntry("mimetype");
                newMime.setMethod(ZipEntry.STORED);
                newMime.setSize(mimeBytes.length);
                newMime.setCompressedSize(mimeBytes.length);
                CRC32 crc = new CRC32();
                crc.update(mimeBytes);
                newMime.setCrc(crc.getValue());
                zos.putNextEntry(newMime);
                zos.write(mimeBytes);
                zos.closeEntry();
            }

            for (ZipEntry entry : allEntries) {
                String name = entry.getName();
                if ("mimetype".equals(name)) continue;

                if (name.endsWith(".html") || name.endsWith(".xhtml")) {
                    totalChapters++;
                    String cacheFileName = name.replace('/', '_') + ".xhtml";
                    File chapterCacheFile = new File(cacheDir, cacheFileName);

                    String finalHtml;
                    if (chapterCacheFile.exists() && chapterCacheFile.length() > 0) {
                        cacheHits++;
                        finalHtml = Files.readString(chapterCacheFile.toPath(), StandardCharsets.UTF_8);
                        log.info("[CACHE] Chapitre déjà traduit récupéré : {}", name);
                    } else {
                        chaptersTranslated++;
                        String originalHtml;
                        try (InputStream is = zipIn.getInputStream(entry)) {
                            originalHtml = new String(is.readAllBytes(), StandardCharsets.UTF_8);
                        }

                        log.info("Traduction du chapitre : {}...", name);
                        Document doc = EpubHtmlParser.parseHtml(originalHtml);
                        Element rootNode = doc.body() != null ? doc.body() : doc;

                        if (rootNode != null) {
                            List<TextNode> textNodes = EpubHtmlParser.collectTextNodes(rootNode);
                            for (int i = 0; i < textNodes.size(); i++) {
                                TextNode tn = textNodes.get(i);
                                String origText = tn.text();

                                if (!EpubHtmlParser.shouldTranslate(origText)) {
                                    continue;
                                }

                                String cached = segmentCache.get(origText);
                                if (cached != null) {
                                    tn.text(EpubHtmlParser.preserveWhitespace(origText, cached));
                                    continue;
                                }

                                EpubHtmlParser.Context ctx = EpubHtmlParser.getContext(textNodes, i, 300);
                                List<String> chunks = EpubHtmlParser.splitTextIntoChunks(origText.trim(), chunkSize);
                                StringBuilder translatedChunks = new StringBuilder();

                                for (String chunk : chunks) {
                                    try {
                                        String transChunk = translateSegment(llmClient, chunk, ctx, charter, glossary, targetLanguage, customPrompt, enableCorrection);
                                        segmentsTranslated++;
                                        if (translatedChunks.length() > 0) translatedChunks.append(" ");
                                        translatedChunks.append(transChunk);
                                    } catch (Exception ex) {
                                        segmentErrors++;
                                        log.warn("Erreur sur le segment '{}': {}", chunk, ex.getMessage());
                                        if (translatedChunks.length() > 0) translatedChunks.append(" ");
                                        translatedChunks.append(chunk);
                                    }
                                }

                                String translatedWhole = translatedChunks.toString();
                                segmentCache.put(origText, translatedWhole);
                                tn.text(EpubHtmlParser.preserveWhitespace(origText, translatedWhole));
                            }

                            finalHtml = doc.outerHtml();

                            // QA Validation
                            if (enableQa) {
                                QualityAssuranceValidator.QaReport report = qaValidator.validateChapter(
                                        EpubHtmlParser.extractVisibleText(originalHtml),
                                        EpubHtmlParser.extractVisibleText(finalHtml),
                                        charter,
                                        glossary
                                );
                                log.info("[QA] Rapport pour {} : {} (Valide={})", name, report.report(), report.valid());
                            }

                            // Dynamic Glossary Enrichment
                            if (enableGlossary) {
                                glossaryManager.enrichGlossary(originalHtml, finalHtml);
                            }

                            // Cache chapter file
                            try {
                                Files.writeString(chapterCacheFile.toPath(), finalHtml, StandardCharsets.UTF_8);
                            } catch (Exception ignored) {}
                        } else {
                            finalHtml = originalHtml;
                        }
                    }

                    writeZipEntry(zos, name, finalHtml.getBytes(StandardCharsets.UTF_8));
                } else if (name.endsWith(".opf")) {
                    // Update metadata in OPF
                    byte[] bytes;
                    try (InputStream is = zipIn.getInputStream(entry)) {
                        bytes = is.readAllBytes();
                    }
                    String opfStr = new String(bytes, StandardCharsets.UTF_8);
                    opfStr = opfStr.replaceAll("(?i)<dc:language>[^<]+</dc:language>", "<dc:language>fr</dc:language>");
                    writeZipEntry(zos, name, opfStr.getBytes(StandardCharsets.UTF_8));
                } else {
                    // Raw passthrough for images, css, fonts, ncx
                    byte[] bytes;
                    try (InputStream is = zipIn.getInputStream(entry)) {
                        bytes = is.readAllBytes();
                    }
                    writeZipEntry(zos, name, bytes);
                }
            }
        }

        long durationSec = (System.currentTimeMillis() - startTime) / 1000;
        log.info("Traduction terminée en {}s pour « {} ». Chapitres: {}, Segments: {}, Erreurs: {}",
                durationSec, title, totalChapters, segmentsTranslated, segmentErrors);

        return new PipelineStats(totalChapters, chaptersTranslated, cacheHits, segmentsTranslated, segmentErrors);
    }

    private static String translateSegment(
            LlmClient llmClient,
            String text,
            EpubHtmlParser.Context ctx,
            String charter,
            String glossary,
            String targetLanguage,
            String customPrompt,
            boolean enableCorrection
    ) throws Exception {
        if (llmClient == null) {
            return text;
        }

        String systemPrompt = String.format("""
                Tu es un traducteur littéraire expert vers le %s.
                Traduis UNIQUEMENT le texte fourni sous "TEXTE À TRADUIRE". Ne traduis pas le contexte. Ne commente pas. Ne continue pas l'histoire.
                Respecte la ponctuation, le ton narratif et renvoie uniquement la traduction brute fidèle et fluide.

                CHARTE ÉDITORIALE DU LIVRE :
                %s

                GLOSSAIRE ET TERMINOLOGIE DU ROMAN :
                %s
                %s
                """, targetLanguage, charter, glossary, (customPrompt != null && !customPrompt.isBlank()) ? "\nCONSIGNES SPÉCIFIQUES :\n" + customPrompt : "");

        StringBuilder userPrompt = new StringBuilder();
        if (ctx != null && (!ctx.before().isEmpty() || !ctx.after().isEmpty())) {
            userPrompt.append("CONTEXTE DU PASSAGE (NE PAS TRADUIRE, FOURNI UNIQUEMENT POUR LE SENS DU TEXTE) :\n");
            if (!ctx.before().isEmpty()) userPrompt.append("[Avant] : ").append(ctx.before()).append("\n");
            if (!ctx.after().isEmpty()) userPrompt.append("[Après] : ").append(ctx.after()).append("\n");
            userPrompt.append("\nTEXTE À TRADUIRE (TRADUIS UNIQUEMENT CE TEXTE) :\n").append(text);
        } else {
            userPrompt.append("TEXTE À TRADUIRE (TRADUIS UNIQUEMENT CE TEXTE SANS AUCUN COMMENTAIRE) :\n").append(text);
        }

        String rawTranslation = llmClient.generateCompletion(systemPrompt, userPrompt.toString());
        if (rawTranslation == null || rawTranslation.isBlank()) {
            return text;
        }

        if (enableCorrection) {
            String corrSystem = String.format("""
                    Tu es un correcteur littéraire et expert stylistique en %s.
                    Relis la traduction par rapport au texte original et améliore la fluidité sans changer le sens.
                    Respecte scrupuleusement la charte et le glossaire.
                    Réponds UNIQUEMENT avec le texte final corrigé.
                    """, targetLanguage);
            String corrUser = String.format("Original :\n%s\n\nTraduction brute :\n%s", text, rawTranslation);
            try {
                String corrected = llmClient.generateCompletion(corrSystem, corrUser);
                if (corrected != null && !corrected.isBlank()) {
                    return corrected.trim();
                }
            } catch (Exception ignored) {}
        }

        return rawTranslation.trim();
    }

    private static String findOpfPath(ZipFile zip) {
        ZipEntry container = zip.getEntry("META-INF/container.xml");
        if (container == null) return null;
        try (InputStream is = zip.getInputStream(container)) {
            String xml = new String(is.readAllBytes(), StandardCharsets.UTF_8);
            Matcher m = Pattern.compile("full-path=[\"']([^\"']+)[\"']").matcher(xml);
            if (m.find()) {
                return m.group(1);
            }
        } catch (Exception ignored) {}
        return null;
    }

    private static void writeZipEntry(ZipOutputStream zos, String name, byte[] content) throws Exception {
        ZipEntry entry = new ZipEntry(name);
        zos.putNextEntry(entry);
        zos.write(content);
        zos.closeEntry();
    }

    public static LlmClient buildLlmClient(Map<String, Object> config) {
        String provider = String.valueOf(config.getOrDefault("ai_provider", config.getOrDefault("ai_model", "Local Ollama")));
        String apiUrl = String.valueOf(config.getOrDefault("api_url", ""));
        String modelName = String.valueOf(config.getOrDefault("model_name", ""));
        String token = String.valueOf(config.getOrDefault("api_token", ""));
        double temp = 0.3;
        if (config.containsKey("temperature")) {
            try {
                temp = ((Number) config.get("temperature")).doubleValue();
            } catch (Exception ignored) {}
        }

        String effectiveUrl;
        String effectiveModel;

        if ("LM Studio".equalsIgnoreCase(provider)) {
            effectiveUrl = !apiUrl.isBlank() ? apiUrl : "http://127.0.0.1:1234/v1";
            effectiveModel = !modelName.isBlank() ? modelName : "local-model";
        } else if ("OpenAI (GPT-4o)".equalsIgnoreCase(provider) || "GPT-4o".equalsIgnoreCase(provider)) {
            effectiveUrl = !apiUrl.isBlank() ? apiUrl : "https://api.openai.com/v1";
            effectiveModel = !modelName.isBlank() ? modelName : "gpt-4o";
        } else if ("DeepL API".equalsIgnoreCase(provider)) {
            effectiveUrl = !apiUrl.isBlank() ? apiUrl : "https://api.deepl.com/v1";
            effectiveModel = !modelName.isBlank() ? modelName : "default";
        } else if ("Custom OpenAI-compatible".equalsIgnoreCase(provider)) {
            effectiveUrl = !apiUrl.isBlank() ? apiUrl : "http://localhost:11434/v1";
            effectiveModel = !modelName.isBlank() ? modelName : "default";
        } else {
            // Local Ollama / Default AI
            effectiveUrl = !apiUrl.isBlank() ? apiUrl : "http://localhost:11434/v1";
            effectiveModel = !modelName.isBlank() ? modelName : "mistral";
        }

        return new LlmClient(effectiveUrl, effectiveModel, token, temp, Duration.ofSeconds(90));
    }
}
