package com.asmolabs.vellum.plugin.builtin;

import com.asmolabs.vellum.plugin.ConfigField;
import com.asmolabs.vellum.plugin.MetadataEnricherPlugin;
import com.asmolabs.vellum.plugin.PluginDescriptor;
import com.asmolabs.vellum.plugin.PluginType;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import org.jsoup.Jsoup;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.stereotype.Component;

import java.io.File;
import java.net.URI;
import java.net.URLEncoder;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.nio.charset.StandardCharsets;
import java.time.Duration;
import java.util.*;

@Component
public class GoogleBooksEnricherPlugin implements MetadataEnricherPlugin {

    private static final Logger log = LoggerFactory.getLogger(GoogleBooksEnricherPlugin.class);

    private final PluginDescriptor descriptor;
    private final ObjectMapper objectMapper;
    private final HttpClient httpClient;

    public record BookMetadataDTO(
            String title,
            List<String> authors,
            String publisher,
            String publishedDate,
            String description,
            List<String> categories,
            String coverUrl,
            String isbn
    ) {}

    public GoogleBooksEnricherPlugin() {
        this.objectMapper = new ObjectMapper();
        this.httpClient = HttpClient.newBuilder()
                .connectTimeout(Duration.ofSeconds(5))
                .followRedirects(HttpClient.Redirect.NORMAL)
                .build();

        this.descriptor = new PluginDescriptor(
                "google-books-enricher",
                "Google Books Metadata Enricher",
                "Enrichit automatiquement les métadonnées (titre, auteur, résumé, éditeur, date, tags, couverture HD) via Google Books API avec secours Open Library.",
                "1.1.0",
                PluginType.IMPORT,
                false,
                null,
                List.of(
                        new ConfigField("api_key", "Clé API Google Books", "password", "", "Insérez votre clé d'API Google Books (optionnelle, recommandée si le quota public est épuisé)."),
                        new ConfigField("language", "Langue des métadonnées", "select", "fr", "Langue préférée pour les résumés et résultats.", List.of("fr", "en", "es", "de", "all")),
                        new ConfigField("auto_add_tags", "Ajouter automatiquement des tags", "boolean", true, "Extraire et appliquer automatiquement les catégories littéraires comme tags."),
                        new ConfigField("fetch_cover", "Télécharger la couverture", "boolean", true, "Télécharger automatiquement la jaquette si le livre n'en possède pas."),
                        new ConfigField("overwrite_existing", "Écraser les champs existants", "boolean", false, "Remplacer les champs déjà renseignés (titre, description, etc.)."),
                        new ConfigField("fallback_openlibrary", "Secours Open Library", "boolean", true, "Interroger Open Library si Google Books est indisponible ou sans résultat.")
                )
        );
    }

    @Override
    public PluginDescriptor getDescriptor() {
        return descriptor;
    }

    @Override
    public Map<String, Object> enrichMetadata(File bookFile, Map<String, Object> currentMetadata, Map<String, Object> config) {
        Map<String, Object> enriched = new HashMap<>(currentMetadata);

        String currentTitle = cleanString(enriched.get("title"));
        String currentAuthor = cleanString(enriched.get("author"));
        String currentIsbn = cleanString(enriched.get("isbn"));

        // If title is missing or is just a filename, fallback to bookFile name without ext
        if (currentTitle.isBlank() && bookFile != null) {
            String name = bookFile.getName();
            int dot = name.lastIndexOf('.');
            currentTitle = dot > 0 ? name.substring(0, dot) : name;
        }

        String apiKey = String.valueOf(config.getOrDefault("api_key", "")).trim();
        String language = String.valueOf(config.getOrDefault("language", "fr")).trim();
        boolean autoAddTags = Boolean.parseBoolean(String.valueOf(config.getOrDefault("auto_add_tags", true)));
        boolean fetchCover = Boolean.parseBoolean(String.valueOf(config.getOrDefault("fetch_cover", true)));
        boolean overwriteExisting = Boolean.parseBoolean(String.valueOf(config.getOrDefault("overwrite_existing", false)));
        boolean fallbackOpenLibrary = Boolean.parseBoolean(String.valueOf(config.getOrDefault("fallback_openlibrary", true)));

        BookMetadataDTO match = null;

        // 1. Try Google Books
        try {
            match = queryGoogleBooks(currentIsbn, currentTitle, currentAuthor, language, apiKey);
        } catch (Exception e) {
            log.warn("Google Books query failed for '{}' ('{}'): {}", currentTitle, currentAuthor, e.getMessage());
        }

        // 2. Fallback to Open Library if no match or error
        if (match == null && fallbackOpenLibrary) {
            try {
                match = queryOpenLibrary(currentIsbn, currentTitle, currentAuthor);
                if (match != null) {
                    log.info("Found book metadata via Open Library fallback for '{}'", currentTitle);
                }
            } catch (Exception e) {
                log.warn("Open Library fallback failed for '{}': {}", currentTitle, e.getMessage());
            }
        }

        if (match == null) {
            log.debug("No online metadata found for '{}'", currentTitle);
            return enriched;
        }

        // Apply metadata fields
        if (match.title() != null && !match.title().isBlank()) {
            if (overwriteExisting || currentTitle.isBlank() || isGenericValue(currentTitle)) {
                enriched.put("title", match.title());
            }
        }

        if (match.authors() != null && !match.authors().isEmpty()) {
            if (overwriteExisting || currentAuthor.isBlank() || isGenericValue(currentAuthor)) {
                enriched.put("author", String.join(", ", match.authors()));
            }
        }

        if (match.publisher() != null && !match.publisher().isBlank()) {
            if (overwriteExisting || cleanString(enriched.get("publisher")).isBlank()) {
                enriched.put("publisher", match.publisher());
            }
        }

        if (match.publishedDate() != null && !match.publishedDate().isBlank()) {
            if (overwriteExisting || cleanString(enriched.get("publishDate")).isBlank()) {
                enriched.put("publishDate", match.publishedDate());
            }
        }

        if (match.description() != null && !match.description().isBlank()) {
            if (overwriteExisting || cleanString(enriched.get("description")).isBlank()) {
                enriched.put("description", match.description());
            }
        }

        if (match.isbn() != null && !match.isbn().isBlank()) {
            if (overwriteExisting || currentIsbn.isBlank()) {
                enriched.put("isbn", match.isbn());
            }
        }

        // Apply tags / categories
        if (autoAddTags && match.categories() != null && !match.categories().isEmpty()) {
            @SuppressWarnings("unchecked")
            List<String> tags = new ArrayList<>((List<String>) enriched.getOrDefault("tags", new ArrayList<String>()));
            for (String cat : match.categories()) {
                if (!cat.isBlank() && !tags.contains(cat)) {
                    tags.add(cat);
                }
            }
            enriched.put("tags", tags);
        }

        // Download cover image if required and available
        boolean hasExistingCover = enriched.get("coverData") != null;
        if (fetchCover && match.coverUrl() != null && (!hasExistingCover || overwriteExisting)) {
            try {
                byte[] coverBytes = downloadImage(match.coverUrl());
                if (coverBytes != null && coverBytes.length > 500) {
                    enriched.put("coverData", coverBytes);
                    enriched.put("coverExt", detectImageExtension(coverBytes, match.coverUrl()));
                    log.info("Downloaded cover image ({} bytes) for '{}'", coverBytes.length, currentTitle);
                }
            } catch (Exception e) {
                log.warn("Failed to download cover from {}: {}", match.coverUrl(), e.getMessage());
            }
        }

        log.debug("GoogleBooksEnricherPlugin successfully enriched metadata for '{}'", currentTitle);
        return enriched;
    }

    public BookMetadataDTO queryGoogleBooks(String isbn, String title, String author, String language, String apiKey) throws Exception {
        String queryParam = null;
        String cleanIsbn = isbn != null ? isbn.replaceAll("[^0-9Xx]", "") : "";

        if (cleanIsbn.length() == 10 || cleanIsbn.length() == 13) {
            queryParam = "isbn:" + cleanIsbn;
        } else if (title != null && !title.isBlank() && !isGenericValue(title)) {
            StringBuilder qb = new StringBuilder();
            qb.append("intitle:\"").append(cleanTerm(title)).append("\"");
            if (author != null && !author.isBlank() && !isGenericValue(author)) {
                qb.append("+inauthor:\"").append(cleanTerm(author)).append("\"");
            }
            queryParam = qb.toString();
        }

        if (queryParam == null || queryParam.isBlank()) {
            return null;
        }

        StringBuilder urlBuilder = new StringBuilder("https://www.googleapis.com/books/v1/volumes?q=");
        urlBuilder.append(URLEncoder.encode(queryParam, StandardCharsets.UTF_8));
        urlBuilder.append("&maxResults=5");

        if (language != null && !language.isBlank() && !"all".equalsIgnoreCase(language)) {
            urlBuilder.append("&langRestrict=").append(URLEncoder.encode(language, StandardCharsets.UTF_8));
        }

        if (apiKey != null && !apiKey.isBlank()) {
            urlBuilder.append("&key=").append(URLEncoder.encode(apiKey, StandardCharsets.UTF_8));
        }

        HttpRequest request = HttpRequest.newBuilder()
                .uri(URI.create(urlBuilder.toString()))
                .timeout(Duration.ofSeconds(5))
                .header("User-Agent", "Vellum-Ebook-Manager/1.0")
                .GET()
                .build();

        HttpResponse<String> response = httpClient.send(request, HttpResponse.BodyHandlers.ofString(StandardCharsets.UTF_8));

        if (response.statusCode() == 429 || response.statusCode() == 403) {
            log.warn("Google Books quota exceeded / restricted (HTTP {}).", response.statusCode());
            return null;
        }

        if (response.statusCode() != 200) {
            log.warn("Google Books returned unexpected HTTP {}.", response.statusCode());
            return null;
        }

        JsonNode root = objectMapper.readTree(response.body());
        JsonNode items = root.path("items");
        if (!items.isArray() || items.isEmpty()) {
            // If intitle+inauthor had no results, try a relaxed query with title only
            if (queryParam.contains("+inauthor:")) {
                String relaxedQuery = "intitle:\"" + cleanTerm(title) + "\"";
                String relaxedUrl = "https://www.googleapis.com/books/v1/volumes?q="
                        + URLEncoder.encode(relaxedQuery, StandardCharsets.UTF_8)
                        + "&maxResults=5"
                        + (language != null && !"all".equalsIgnoreCase(language) ? "&langRestrict=" + language : "")
                        + (apiKey != null && !apiKey.isBlank() ? "&key=" + apiKey : "");

                HttpRequest relReq = HttpRequest.newBuilder()
                        .uri(URI.create(relaxedUrl))
                        .timeout(Duration.ofSeconds(5))
                        .header("User-Agent", "Vellum-Ebook-Manager/1.0")
                        .GET()
                        .build();
                HttpResponse<String> relResp = httpClient.send(relReq, HttpResponse.BodyHandlers.ofString(StandardCharsets.UTF_8));
                if (relResp.statusCode() == 200) {
                    JsonNode relRoot = objectMapper.readTree(relResp.body());
                    items = relRoot.path("items");
                }
            }
        }

        if (!items.isArray() || items.isEmpty()) {
            return null;
        }

        JsonNode volumeInfo = items.get(0).path("volumeInfo");
        return parseGoogleVolumeInfo(volumeInfo);
    }

    private BookMetadataDTO parseGoogleVolumeInfo(JsonNode volumeInfo) {
        String title = volumeInfo.path("title").asText(null);
        String subtitle = volumeInfo.path("subtitle").asText(null);
        if (title != null && subtitle != null && !subtitle.isBlank()) {
            title = title + " : " + subtitle;
        }

        List<String> authors = new ArrayList<>();
        JsonNode authorsNode = volumeInfo.path("authors");
        if (authorsNode.isArray()) {
            for (JsonNode a : authorsNode) {
                String name = a.asText("").trim();
                if (!name.isBlank()) {
                    authors.add(name);
                }
            }
        }

        String publisher = volumeInfo.path("publisher").asText(null);
        String publishedDate = volumeInfo.path("publishedDate").asText(null);

        String rawDesc = volumeInfo.path("description").asText(null);
        String description = null;
        if (rawDesc != null && !rawDesc.isBlank()) {
            description = Jsoup.parse(rawDesc).text();
        }

        // ISBN
        String foundIsbn = null;
        JsonNode idList = volumeInfo.path("industryIdentifiers");
        if (idList.isArray()) {
            for (JsonNode idNode : idList) {
                String type = idNode.path("type").asText("");
                String val = idNode.path("identifier").asText("").trim();
                if ("ISBN_13".equalsIgnoreCase(type)) {
                    foundIsbn = val;
                    break;
                } else if ("ISBN_10".equalsIgnoreCase(type) && foundIsbn == null) {
                    foundIsbn = val;
                }
            }
        }

        // Categories / Tags
        List<String> categories = new ArrayList<>();
        JsonNode catNode = volumeInfo.path("categories");
        if (catNode.isArray()) {
            for (JsonNode c : catNode) {
                String text = c.asText("").trim();
                for (String part : text.split("[/,;]")) {
                    String clean = cleanTag(part);
                    if (!clean.isBlank() && !categories.contains(clean)) {
                        categories.add(clean);
                    }
                }
            }
        }

        // Image Cover URL
        String coverUrl = null;
        JsonNode images = volumeInfo.path("imageLinks");
        if (images.isObject()) {
            if (images.hasNonNull("extraLarge")) {
                coverUrl = images.get("extraLarge").asText();
            } else if (images.hasNonNull("large")) {
                coverUrl = images.get("large").asText();
            } else if (images.hasNonNull("medium")) {
                coverUrl = images.get("medium").asText();
            } else if (images.hasNonNull("thumbnail")) {
                coverUrl = images.get("thumbnail").asText();
            } else if (images.hasNonNull("smallThumbnail")) {
                coverUrl = images.get("smallThumbnail").asText();
            }
        }

        if (coverUrl != null) {
            coverUrl = coverUrl.replace("http://", "https://");
        }

        return new BookMetadataDTO(title, authors, publisher, publishedDate, description, categories, coverUrl, foundIsbn);
    }

    public BookMetadataDTO queryOpenLibrary(String isbn, String title, String author) throws Exception {
        String cleanIsbn = isbn != null ? isbn.replaceAll("[^0-9Xx]", "") : "";

        if (cleanIsbn.length() == 10 || cleanIsbn.length() == 13) {
            String url = "https://openlibrary.org/api/books?bibkeys=ISBN:" + cleanIsbn + "&format=json&jscmd=data";
            HttpRequest req = HttpRequest.newBuilder()
                    .uri(URI.create(url))
                    .timeout(Duration.ofSeconds(5))
                    .header("User-Agent", "Vellum/1.0 (https://github.com/asmolabs/library)")
                    .GET()
                    .build();
            HttpResponse<String> resp = httpClient.send(req, HttpResponse.BodyHandlers.ofString(StandardCharsets.UTF_8));
            if (resp.statusCode() == 200) {
                JsonNode root = objectMapper.readTree(resp.body());
                JsonNode bookData = root.path("ISBN:" + cleanIsbn);
                if (bookData.isObject()) {
                    return parseOpenLibraryData(bookData, cleanIsbn);
                }
            }
        }

        if (title == null || title.isBlank() || isGenericValue(title)) {
            return null;
        }

        StringBuilder urlBuilder = new StringBuilder("https://openlibrary.org/search.json?title=");
        urlBuilder.append(URLEncoder.encode(title, StandardCharsets.UTF_8));
        if (author != null && !author.isBlank() && !isGenericValue(author)) {
            urlBuilder.append("&author=").append(URLEncoder.encode(author, StandardCharsets.UTF_8));
        }
        urlBuilder.append("&limit=3");

        HttpRequest req = HttpRequest.newBuilder()
                .uri(URI.create(urlBuilder.toString()))
                .timeout(Duration.ofSeconds(5))
                .header("User-Agent", "Vellum/1.0 (https://github.com/asmolabs/library)")
                .GET()
                .build();
        HttpResponse<String> resp = httpClient.send(req, HttpResponse.BodyHandlers.ofString(StandardCharsets.UTF_8));
        if (resp.statusCode() != 200) {
            return null;
        }

        JsonNode root = objectMapper.readTree(resp.body());
        JsonNode docs = root.path("docs");
        if (!docs.isArray() || docs.isEmpty()) {
            return null;
        }

        JsonNode doc = docs.get(0);
        String bookTitle = doc.path("title").asText(null);

        List<String> authors = new ArrayList<>();
        JsonNode aNames = doc.path("author_name");
        if (aNames.isArray()) {
            for (JsonNode an : aNames) {
                String a = an.asText("").trim();
                if (!a.isBlank()) authors.add(a);
            }
        }

        String publisher = null;
        JsonNode pubs = doc.path("publisher");
        if (pubs.isArray() && !pubs.isEmpty()) {
            publisher = pubs.get(0).asText(null);
        }

        String publishedDate = null;
        if (doc.hasNonNull("first_publish_year")) {
            publishedDate = String.valueOf(doc.get("first_publish_year").asInt());
        }

        String foundIsbn = null;
        JsonNode isbns = doc.path("isbn");
        if (isbns.isArray() && !isbns.isEmpty()) {
            foundIsbn = isbns.get(0).asText(null);
        }

        List<String> categories = new ArrayList<>();
        JsonNode subjects = doc.path("subject");
        if (subjects.isArray()) {
            for (JsonNode s : subjects) {
                String sub = cleanTag(s.asText(""));
                if (!sub.isBlank() && !categories.contains(sub) && categories.size() < 6) {
                    categories.add(sub);
                }
            }
        }

        String coverUrl = null;
        if (doc.hasNonNull("cover_i")) {
            long coverId = doc.get("cover_i").asLong();
            if (coverId > 0) {
                coverUrl = "https://covers.openlibrary.org/b/id/" + coverId + "-L.jpg";
            }
        }

        return new BookMetadataDTO(bookTitle, authors, publisher, publishedDate, null, categories, coverUrl, foundIsbn);
    }

    private BookMetadataDTO parseOpenLibraryData(JsonNode bookData, String fallbackIsbn) {
        String title = bookData.path("title").asText(null);
        String subtitle = bookData.path("subtitle").asText(null);
        if (title != null && subtitle != null && !subtitle.isBlank()) {
            title = title + " : " + subtitle;
        }

        List<String> authors = new ArrayList<>();
        JsonNode aList = bookData.path("authors");
        if (aList.isArray()) {
            for (JsonNode a : aList) {
                String name = a.path("name").asText("").trim();
                if (!name.isBlank()) authors.add(name);
            }
        }

        String publisher = null;
        JsonNode pubList = bookData.path("publishers");
        if (pubList.isArray() && !pubList.isEmpty()) {
            publisher = pubList.get(0).path("name").asText(null);
        }

        String publishedDate = bookData.path("publish_date").asText(null);

        List<String> categories = new ArrayList<>();
        JsonNode subjList = bookData.path("subjects");
        if (subjList.isArray()) {
            for (JsonNode s : subjList) {
                String name = cleanTag(s.path("name").asText(""));
                if (!name.isBlank() && !categories.contains(name) && categories.size() < 6) {
                    categories.add(name);
                }
            }
        }

        String coverUrl = null;
        JsonNode coverObj = bookData.path("cover");
        if (coverObj.isObject()) {
            if (coverObj.hasNonNull("large")) {
                coverUrl = coverObj.get("large").asText();
            } else if (coverObj.hasNonNull("medium")) {
                coverUrl = coverObj.get("medium").asText();
            }
        }

        return new BookMetadataDTO(title, authors, publisher, publishedDate, null, categories, coverUrl, fallbackIsbn);
    }

    public byte[] downloadImage(String imageUrl) throws Exception {
        HttpRequest req = HttpRequest.newBuilder()
                .uri(URI.create(imageUrl))
                .timeout(Duration.ofSeconds(6))
                .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
                .GET()
                .build();
        HttpResponse<byte[]> res = httpClient.send(req, HttpResponse.BodyHandlers.ofByteArray());
        if (res.statusCode() == 200 && res.body() != null && res.body().length > 500) {
            return res.body();
        }
        return null;
    }

    private String detectImageExtension(byte[] data, String url) {
        if (data.length > 3 && data[0] == (byte) 0x89 && data[1] == (byte) 0x50 && data[2] == (byte) 0x4E && data[3] == (byte) 0x47) {
            return ".png";
        }
        if (url != null && url.toLowerCase().contains(".png")) {
            return ".png";
        }
        return ".jpg";
    }

    private String cleanString(Object obj) {
        return obj != null ? String.valueOf(obj).trim() : "";
    }

    private String cleanTerm(String term) {
        return term.replaceAll("[\"'\\[\\]()]", " ").replaceAll("\\s+", " ").trim();
    }

    private String cleanTag(String tag) {
        String t = tag.trim();
        if (t.length() > 30) {
            t = t.substring(0, 30);
        }
        if (t.length() < 2) return "";
        return Character.toUpperCase(t.charAt(0)) + t.substring(1);
    }

    private boolean isGenericValue(String str) {
        if (str == null) return true;
        String s = str.trim().toLowerCase();
        return s.isBlank() || s.equals("inconnu") || s.equals("unknown") || s.equals("sans titre") || s.equals("untitled");
    }

    @Override
    public TestResult testConfiguration(Map<String, Object> config) {
        String apiKey = String.valueOf(config.getOrDefault("api_key", "")).trim();
        try {
            String urlStr = "https://www.googleapis.com/books/v1/volumes?q=test&maxResults=1"
                    + (!apiKey.isBlank() ? "&key=" + apiKey : "");

            HttpRequest request = HttpRequest.newBuilder()
                    .uri(URI.create(urlStr))
                    .timeout(Duration.ofSeconds(4))
                    .header("User-Agent", "Vellum-Ebook-Manager/1.0")
                    .GET()
                    .build();

            HttpResponse<String> response = httpClient.send(request, HttpResponse.BodyHandlers.ofString(StandardCharsets.UTF_8));
            if (response.statusCode() == 200) {
                return TestResult.ok("Connexion réussie à Google Books API (HTTP 200) !" + (!apiKey.isBlank() ? " Clé API valide." : " (Mode public)"));
            } else if (response.statusCode() == 429) {
                return TestResult.ok("Quota public Google Books dépassé (HTTP 429). Le secours Open Library prendra le relais automatiquement, ou renseignez une clé API pour un accès Google sans quota.");
            } else if (response.statusCode() == 400 && !apiKey.isBlank()) {
                return TestResult.error("Clé API Google Books invalide ou refusée (HTTP 400).");
            } else if (response.statusCode() == 403) {
                return TestResult.ok("Accès restreint Google Books (HTTP 403). Le secours Open Library assurera l'enrichissement automatique.");
            } else {
                return TestResult.error("Réponse inattendue de Google Books : HTTP " + response.statusCode());
            }
        } catch (Exception e) {
            return TestResult.error("Impossible de joindre Google Books : " + (e.getMessage() != null ? e.getMessage() : e.getClass().getSimpleName()));
        }
    }
}
