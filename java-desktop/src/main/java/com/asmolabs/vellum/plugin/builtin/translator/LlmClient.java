package com.asmolabs.vellum.plugin.builtin.translator;

import com.fasterxml.jackson.core.type.TypeReference;
import com.fasterxml.jackson.databind.ObjectMapper;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.time.Duration;
import java.util.*;
import java.util.regex.Pattern;

public class LlmClient {

    private static final Logger log = LoggerFactory.getLogger(LlmClient.class);
    private static final ObjectMapper json = new ObjectMapper();

    private final HttpClient httpClient;
    private final String baseUrl;
    private final String modelName;
    private final String apiKey;
    private final double temperature;
    private final Duration timeout;

    public LlmClient(String baseUrl, String modelName, String apiKey, double temperature, Duration timeout) {
        this.httpClient = HttpClient.newBuilder()
                .connectTimeout(Duration.ofSeconds(10))
                .build();
        this.baseUrl = normalizeBaseUrl(baseUrl);
        this.modelName = (modelName != null && !modelName.isBlank()) ? modelName.trim() : "default";
        this.apiKey = (apiKey != null) ? apiKey.trim() : "";
        this.temperature = temperature;
        this.timeout = (timeout != null) ? timeout : Duration.ofSeconds(60);
    }

    private static String normalizeBaseUrl(String url) {
        if (url == null || url.isBlank()) {
            return "http://localhost:11434/v1";
        }
        String u = url.trim();
        while (u.endsWith("/")) {
            u = u.substring(0, u.length() - 1);
        }
        return u;
    }

    public String generateCompletion(String systemPrompt, String userPrompt) throws Exception {
        String endpoint = baseUrl.endsWith("/chat/completions") ? baseUrl : baseUrl + "/chat/completions";

        List<Map<String, String>> messages = new ArrayList<>();
        if (systemPrompt != null && !systemPrompt.isBlank()) {
            messages.add(Map.of("role", "system", "content", systemPrompt));
        }
        messages.add(Map.of("role", "user", "content", userPrompt));

        Map<String, Object> payload = new HashMap<>();
        payload.put("model", modelName);
        payload.put("messages", messages);
        payload.put("temperature", temperature);
        payload.put("stream", false);

        String jsonBody = json.writeValueAsString(payload);

        HttpRequest.Builder reqBuilder = HttpRequest.newBuilder()
                .uri(URI.create(endpoint))
                .timeout(timeout)
                .header("Content-Type", "application/json")
                .POST(HttpRequest.BodyPublishers.ofString(jsonBody));

        if (!apiKey.isBlank()) {
            reqBuilder.header("Authorization", "Bearer " + apiKey);
        }

        HttpResponse<String> response = httpClient.send(reqBuilder.build(), HttpResponse.BodyHandlers.ofString());

        if (response.statusCode() >= 200 && response.statusCode() < 300) {
            String body = response.body();
            Map<String, Object> map = json.readValue(body, new TypeReference<>() {});

            if (map.containsKey("choices")) {
                List<?> choices = (List<?>) map.get("choices");
                if (!choices.isEmpty()) {
                    Map<?, ?> first = (Map<?, ?>) choices.get(0);
                    if (first.containsKey("message")) {
                        Map<?, ?> msg = (Map<?, ?>) first.get("message");
                        String content = String.valueOf(msg.get("content"));
                        return cleanModelArtifacts(content);
                    } else if (first.containsKey("text")) {
                        return cleanModelArtifacts(String.valueOf(first.get("text")));
                    }
                }
            } else if (map.containsKey("response")) {
                // Native Ollama format /api/generate fallback
                return cleanModelArtifacts(String.valueOf(map.get("response")));
            }

            throw new RuntimeException("Format de réponse LLM inattendu : " + body);
        } else {
            throw new RuntimeException("Erreur appel LLM HTTP " + response.statusCode() + " : " + response.body());
        }
    }

    public static String cleanModelArtifacts(String text) {
        if (text == null) return "";
        String s = text;

        if (s.contains("<channel|>")) {
            String[] parts = s.split("<channel\\|>");
            s = parts[parts.length - 1];
        }

        s = s.replaceAll("(?s)<\\|channel\\>thought.*?$", "");
        s = s.replaceAll("(?s)<\\|channel\\>final.*?$", "");
        s = s.replaceAll("(?s)<\\|channel\\>assistant.*?$", "");
        s = s.replace("<|channel|>", "");
        s = s.replace("<start_of_turn>", "");
        s = s.replace("</start_of_turn>", "");
        s = s.replace("<end_of_turn>", "");
        s = s.replaceAll("(?im)^\\s*user\\s*$", "");
        s = s.replaceAll("(?im)^\\s*model\\s*$", "");
        s = s.replaceAll("(?is)^\\s*Thinking Process:.*?\n\n", "");

        // Remove wrapping markdown code fences if output was wrapped in ```
        if (s.startsWith("```") && s.endsWith("```")) {
            s = s.replaceFirst("^```[a-zA-Z]*\\s*", "");
            s = s.replaceFirst("\\s*```$", "");
        }

        return s.trim();
    }

    public boolean testConnection() {
        try {
            // Check /models endpoint
            String modelsUrl = baseUrl.endsWith("/v1") ? baseUrl + "/models" : baseUrl.replaceFirst("/chat/completions$", "") + "/models";
            HttpRequest.Builder b = HttpRequest.newBuilder()
                    .uri(URI.create(modelsUrl))
                    .timeout(Duration.ofSeconds(4))
                    .GET();
            if (!apiKey.isBlank()) {
                b.header("Authorization", "Bearer " + apiKey);
            }
            HttpResponse<String> res = httpClient.send(b.build(), HttpResponse.BodyHandlers.ofString());
            if (res.statusCode() == 200) return true;

            // Fallback: test minimal completion
            String result = generateCompletion("", "Bonjour ! Réponds par le seul mot OK.");
            return result != null && !result.isBlank();
        } catch (Exception e) {
            log.warn("Test de connexion LLM échoué vers {} : {}", baseUrl, e.getMessage());
            return false;
        }
    }
}
