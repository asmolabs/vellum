use regex::Regex;
use serde_json::{json, Value};
use std::time::Duration;
use tracing::warn;
use vellum_domain::{VellumError, VellumResult};

#[derive(Clone)]
pub struct LlmClient {
    pub provider: String,
    pub base_url: String,
    pub model_name: String,
    pub api_key: String,
    pub temperature: f64,
    pub timeout: Duration,
}

pub type LlmTranslationClient = LlmClient;

impl LlmClient {
    pub fn new(
        provider: impl Into<String>,
        base_url: impl Into<String>,
        model_name: impl Into<String>,
        api_key: Option<String>,
        temperature: f64,
    ) -> Self {
        let raw_url = base_url.into();
        let trimmed_url = raw_url.trim().trim_end_matches('/');
        let normalized_url = if trimmed_url.is_empty() {
            "http://localhost:11434/v1".to_string()
        } else {
            trimmed_url.to_string()
        };

        let m = model_name.into();
        let model = if m.trim().is_empty() {
            "default".to_string()
        } else {
            m.trim().to_string()
        };

        Self {
            provider: provider.into(),
            base_url: normalized_url,
            model_name: model,
            api_key: api_key.unwrap_or_default().trim().to_string(),
            temperature,
            timeout: Duration::from_secs(60),
        }
    }

    pub fn chat(&self, system_prompt: &str, user_prompt: &str) -> VellumResult<String> {
        self.generate_completion(system_prompt, user_prompt)
    }

    pub fn generate_completion(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> VellumResult<String> {
        let endpoint = if self.base_url.ends_with("/chat/completions") {
            self.base_url.clone()
        } else {
            format!("{}/chat/completions", self.base_url)
        };

        let mut messages = Vec::new();
        if !system_prompt.trim().is_empty() {
            messages.push(json!({ "role": "system", "content": system_prompt }));
        }
        messages.push(json!({ "role": "user", "content": user_prompt }));

        let payload = json!({
            "model": self.model_name,
            "messages": messages,
            "temperature": self.temperature,
            "stream": false
        });

        let client = reqwest::blocking::Client::builder()
            .timeout(self.timeout)
            .build()
            .map_err(|e| VellumError::Plugin(e.to_string()))?;

        let mut req = client.post(&endpoint).json(&payload);
        if !self.api_key.is_empty() {
            req = req.bearer_auth(&self.api_key);
        }

        let resp = req.send().map_err(|e| VellumError::Plugin(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(VellumError::Plugin(format!(
                "Erreur appel LLM HTTP {}",
                resp.status()
            )));
        }

        let body: Value = resp
            .json()
            .map_err(|e| VellumError::Plugin(e.to_string()))?;

        if let Some(content) = body
            .get("choices")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|first| {
                first
                    .get("message")
                    .and_then(|m| m.get("content"))
                    .and_then(|c| c.as_str())
                    .or_else(|| first.get("text").and_then(|t| t.as_str()))
            })
        {
            return Ok(clean_model_artifacts(content));
        }

        if let Some(resp_str) = body.get("response").and_then(|r| r.as_str()) {
            return Ok(clean_model_artifacts(resp_str));
        }

        Err(VellumError::Plugin(
            "Format de réponse LLM inattendu".to_string(),
        ))
    }

    pub fn test_connection(&self) -> bool {
        let models_url = if self.base_url.ends_with("/v1") {
            format!("{}/models", self.base_url)
        } else {
            format!(
                "{}/models",
                self.base_url.trim_end_matches("/chat/completions")
            )
        };

        let Ok(client) = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(4))
            .build()
        else {
            return false;
        };

        let mut req = client.get(&models_url);
        if !self.api_key.is_empty() {
            req = req.bearer_auth(&self.api_key);
        }

        match req.send() {
            Ok(res) if res.status().is_success() => true,
            _ => match self.generate_completion("", "Bonjour ! Réponds par le seul mot OK.") {
                Ok(s) => !s.trim().is_empty(),
                Err(e) => {
                    warn!(
                        "Test de connexion LLM échoué vers {} : {}",
                        self.base_url, e
                    );
                    false
                }
            },
        }
    }
}

pub fn clean_model_artifacts(text: &str) -> String {
    let mut s = text.to_string();

    if s.contains("<channel|>") {
        if let Some(last) = s.split("<channel|>").last() {
            s = last.to_string();
        }
    }

    static THOUGHT_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?s)<\|channel\|>thought.*?$").expect("valid thought regex")
    });
    static FINAL_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?s)<\|channel\|>final.*?$").expect("valid final regex")
    });
    static ASSISTANT_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?s)<\|channel\|>assistant.*?$").expect("valid assistant regex")
    });
    static THINKING_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?is)^\s*Thinking Process:.*?\n\n").expect("valid thinking regex")
    });
    static FENCE_START_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^```[a-zA-Z]*\s*").expect("valid fence start regex")
    });

    s = THOUGHT_RE.replace_all(&s, "").to_string();
    s = FINAL_RE.replace_all(&s, "").to_string();
    s = ASSISTANT_RE.replace_all(&s, "").to_string();
    s = THINKING_RE.replace_all(&s, "").to_string();
    s = s
        .replace("<|channel|>", "")
        .replace("<start_of_turn>", "")
        .replace("</start_of_turn>", "")
        .replace("<end_of_turn>", "");

    let trimmed = s.trim();
    if trimmed.starts_with("```") && trimmed.ends_with("```") {
        let without_start = FENCE_START_RE.replace(trimmed, "");
        return without_start.trim_end_matches("```").trim().to_string();
    }

    trimmed.to_string()
}

pub fn parse_numbered_response(response: &str, expected_count: usize) -> Vec<String> {
    static NUMBERED_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?m)^\s*\[(\d+)\]\s*(.*)$").expect("valid numbered line regex")
    });
    let re = &*NUMBERED_RE;
    let mut map = std::collections::BTreeMap::new();
    let mut current_idx = None;
    let mut current_text = String::new();

    for line in response.lines() {
        if let Some(caps) = re.captures(line) {
            if let Some(idx) = current_idx {
                map.insert(idx, current_text.trim().to_string());
                current_text.clear();
            }
            if let Ok(num) = caps[1].parse::<usize>() {
                current_idx = Some(num);
                current_text.push_str(&caps[2]);
            }
        } else if current_idx.is_some() {
            if !current_text.is_empty() {
                current_text.push('\n');
            }
            current_text.push_str(line);
        }
    }
    if let Some(idx) = current_idx {
        map.insert(idx, current_text.trim().to_string());
    }

    let mut result = Vec::with_capacity(expected_count);
    for i in 1..=expected_count {
        if let Some(text) = map.get(&i) {
            result.push(text.clone());
        } else {
            result.push(String::new());
        }
    }
    result
}
