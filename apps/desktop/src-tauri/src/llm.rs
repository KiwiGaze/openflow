//! Optional LLM refinement over any OpenAI-compatible chat endpoint.
//!
//! One client covers all providers: Ollama (`/v1` compatibility layer),
//! llama.cpp's `llama-server`, LM Studio, OpenAI, Groq, OpenRouter, etc.
//! Nothing here runs unless the user explicitly configures a provider.

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::settings::{LlmConfig, LlmProviderKind};

#[derive(Debug, Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    temperature: f32,
    stream: bool,
}

#[derive(Debug, Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatChoiceMessage,
}

#[derive(Debug, Deserialize)]
struct ChatChoiceMessage {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiErrorBody {
    error: Option<ApiErrorDetail>,
}

#[derive(Debug, Deserialize)]
struct ApiErrorDetail {
    message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmTestResult {
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Deserialize)]
struct OllamaTags {
    models: Vec<OllamaModel>,
}

#[derive(Debug, Deserialize)]
struct OllamaModel {
    name: String,
}

/// Joins the user's endpoint root with `/chat/completions`.
///
/// A bare origin gets the conventional `/v1` inserted
/// (`http://host:11434` → `http://host:11434/v1/chat/completions`).
/// Any explicit path is trusted as-is: version segments differ per
/// provider (`/v1`, `/openai/v1`, `/api/coding/paas/v4`, `/v1beta/openai`),
/// so guessing beyond the path the user gave produces 404s.
pub fn chat_completions_url(base_url: &str) -> String {
    let base = base_url.trim().trim_end_matches('/');
    let has_path = base
        .split_once("://")
        .map(|(_, rest)| rest.contains('/'))
        .unwrap_or(false);
    if has_path {
        format!("{base}/chat/completions")
    } else {
        format!("{base}/v1/chat/completions")
    }
}

pub struct LlmClient {
    http: reqwest::Client,
}

impl Default for LlmClient {
    fn default() -> Self {
        Self::new()
    }
}

impl LlmClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
        }
    }

    /// One chat completion; returns the assistant text.
    pub async fn chat(&self, cfg: &LlmConfig, system: &str, user: &str) -> AppResult<String> {
        if cfg.provider == LlmProviderKind::None {
            return Err(AppError::Llm("no AI provider configured".into()));
        }
        if cfg.model.trim().is_empty() {
            return Err(AppError::Llm("no model configured".into()));
        }

        let body = ChatRequest {
            model: &cfg.model,
            messages: vec![
                ChatMessage {
                    role: "system",
                    content: system,
                },
                ChatMessage {
                    role: "user",
                    content: user,
                },
            ],
            // Low temperature: this is an editing task, not creative writing.
            temperature: 0.2,
            stream: false,
        };

        let mut request = self
            .http
            .post(chat_completions_url(&cfg.base_url))
            .timeout(std::time::Duration::from_secs(cfg.timeout_secs.max(5)))
            .json(&body);
        if !cfg.api_key.trim().is_empty() {
            request = request.bearer_auth(cfg.api_key.trim());
        }

        let response = request.send().await.map_err(|e| {
            if e.is_timeout() {
                AppError::Llm(format!("provider timed out after {}s", cfg.timeout_secs))
            } else if e.is_connect() {
                AppError::Llm(connect_hint(cfg))
            } else {
                AppError::Llm(format!("request failed: {e}"))
            }
        })?;

        let status = response.status();
        let raw = response
            .text()
            .await
            .map_err(|e| AppError::Llm(format!("invalid response: {e}")))?;

        if !status.is_success() {
            let detail = serde_json::from_str::<ApiErrorBody>(&raw)
                .ok()
                .and_then(|b| b.error)
                .and_then(|e| e.message)
                .unwrap_or_else(|| truncate(&raw, 200));
            return Err(AppError::Llm(format!(
                "provider returned {status}: {detail}"
            )));
        }

        let parsed: ChatResponse = serde_json::from_str(&raw)
            .map_err(|e| AppError::Llm(format!("unexpected response shape: {e}")))?;
        let content = parsed
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content)
            .unwrap_or_default();
        let cleaned = strip_code_fence(content.trim());
        if cleaned.is_empty() {
            return Err(AppError::Llm("provider returned an empty response".into()));
        }
        Ok(cleaned)
    }

    pub async fn test(&self, cfg: &LlmConfig) -> LlmTestResult {
        match self
            .chat(
                cfg,
                "You are a connectivity check. Reply with exactly: OK",
                "ping",
            )
            .await
        {
            Ok(_) => LlmTestResult {
                ok: true,
                message: format!("Connected — {} responded.", cfg.model),
            },
            Err(err) => LlmTestResult {
                ok: false,
                message: err.to_string(),
            },
        }
    }

    /// Lists locally installed Ollama models via `GET /api/tags`.
    pub async fn list_ollama_models(&self, base_url: &str) -> AppResult<Vec<String>> {
        let url = format!("{}/api/tags", base_url.trim().trim_end_matches('/'));
        let response = self
            .http
            .get(url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() {
                    AppError::Llm("could not reach Ollama — is it running?".into())
                } else {
                    AppError::Llm(format!("request failed: {e}"))
                }
            })?
            .error_for_status()
            .map_err(|e| AppError::Llm(format!("Ollama returned an error: {e}")))?;
        let tags: OllamaTags = response
            .json()
            .await
            .map_err(|e| AppError::Llm(format!("unexpected /api/tags response: {e}")))?;
        Ok(tags.models.into_iter().map(|m| m.name).collect())
    }
}

fn connect_hint(cfg: &LlmConfig) -> String {
    match cfg.provider {
        LlmProviderKind::Ollama => {
            format!(
                "could not connect to Ollama at {} — is `ollama serve` running?",
                cfg.base_url
            )
        }
        _ => format!("could not connect to {}", cfg.base_url),
    }
}

/// Small models sometimes wrap their answer in a markdown code fence.
fn strip_code_fence(text: &str) -> String {
    let trimmed = text.trim();
    if let Some(inner) = trimmed.strip_prefix("```") {
        if let Some(end) = inner.rfind("```") {
            let inner = &inner[..end];
            // Drop an optional language tag on the first line.
            let inner = inner
                .split_once('\n')
                .map(|(_, rest)| rest)
                .unwrap_or(inner);
            return inner.trim().to_string();
        }
    }
    trimmed.to_string()
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        let mut end = max;
        while !s.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}…", &s[..end])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_builder_inserts_v1_only_for_bare_origins() {
        assert_eq!(
            chat_completions_url("http://localhost:11434"),
            "http://localhost:11434/v1/chat/completions"
        );
        assert_eq!(
            chat_completions_url("http://localhost:11434/"),
            "http://localhost:11434/v1/chat/completions"
        );
        assert_eq!(
            chat_completions_url("https://api.openai.com"),
            "https://api.openai.com/v1/chat/completions"
        );
    }

    #[test]
    fn url_builder_trusts_an_explicit_path() {
        assert_eq!(
            chat_completions_url("https://api.groq.com/openai/v1"),
            "https://api.groq.com/openai/v1/chat/completions"
        );
        assert_eq!(
            chat_completions_url("http://localhost:1234/v1/"),
            "http://localhost:1234/v1/chat/completions"
        );
        // Version segments are not always `/v1`: z.ai uses `/v4`,
        // Gemini's OpenAI compatibility layer uses `/v1beta/openai`.
        assert_eq!(
            chat_completions_url("https://api.z.ai/api/coding/paas/v4"),
            "https://api.z.ai/api/coding/paas/v4/chat/completions"
        );
        assert_eq!(
            chat_completions_url("https://generativelanguage.googleapis.com/v1beta/openai"),
            "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions"
        );
    }

    #[test]
    fn request_serializes_to_openai_shape() {
        let body = ChatRequest {
            model: "qwen2.5:3b",
            messages: vec![
                ChatMessage {
                    role: "system",
                    content: "sys",
                },
                ChatMessage {
                    role: "user",
                    content: "hello",
                },
            ],
            temperature: 0.2,
            stream: false,
        };
        let value = serde_json::to_value(&body).unwrap();
        assert_eq!(value["model"], "qwen2.5:3b");
        assert_eq!(value["messages"][0]["role"], "system");
        assert_eq!(value["messages"][1]["content"], "hello");
        assert_eq!(value["stream"], false);
    }

    #[test]
    fn response_parsing_extracts_content() {
        let raw = r#"{"choices":[{"message":{"role":"assistant","content":"Cleaned text."}}]}"#;
        let parsed: ChatResponse = serde_json::from_str(raw).unwrap();
        assert_eq!(
            parsed.choices[0].message.content.as_deref(),
            Some("Cleaned text.")
        );
    }

    #[test]
    fn strips_code_fences_but_keeps_plain_text() {
        assert_eq!(strip_code_fence("plain"), "plain");
        assert_eq!(strip_code_fence("```\nfenced\n```"), "fenced");
        assert_eq!(strip_code_fence("```text\nfenced\n```"), "fenced");
    }

    #[test]
    fn ollama_tags_parse() {
        let raw = r#"{"models":[{"name":"qwen2.5:3b","size":1},{"name":"llama3.2:3b","size":2}]}"#;
        let tags: OllamaTags = serde_json::from_str(raw).unwrap();
        let names: Vec<_> = tags.models.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, vec!["qwen2.5:3b", "llama3.2:3b"]);
    }
}
