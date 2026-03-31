// ============================================
// provider.rs - Provider Abstraction & Cascade
// ============================================
// Tries providers in quality order, falling through on rate limits (429),
// server errors (5xx), or auth errors (401/403).

use anyhow::Result;
use log::{info, warn};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;

use crate::openai_client::OpenAIClient;

/// HTTP timeout for Gemini API requests.
const GEMINI_REQUEST_TIMEOUT: Duration = Duration::from_secs(90);

/// Wire format the provider speaks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ApiType {
    OpenAI,
    Gemini,
}

/// Configuration for a single provider, as loaded from config.yaml.
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub name: String,
    pub api_type: ApiType,
    pub api_base: String,
    pub api_key: String,
    pub model: String,
    pub rpm_limit: u32,
    pub rpd_limit: u32,
}

/// YAML-friendly version that uses `api_key_env` for deferred resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfigYaml {
    pub name: String,
    pub api_type: ApiType,
    pub api_base: String,
    /// The name of the key in secrets.yaml / env vars (e.g. "GROQ_API_KEY").
    pub api_key_env: String,
    pub model: String,
    #[serde(default = "default_rpm")]
    pub rpm: u32,
    #[serde(default = "default_rpd")]
    pub rpd: u32,
}

fn default_rpm() -> u32 {
    30
}
fn default_rpd() -> u32 {
    1000
}

/// The cascade: tries providers in order, falls through on errors.
pub struct ProviderCascade {
    providers: Vec<ProviderEntry>,
}

struct ProviderEntry {
    config: ProviderConfig,
    /// Only set for OpenAI-type providers.
    openai_client: Option<OpenAIClient>,
    /// Shared reqwest client for Gemini calls.
    http_client: Client,
}

/// Unified response from any provider, already in a format the agent can use.
#[derive(Debug)]
pub struct CascadeResponse {
    /// The provider that successfully answered.
    pub provider_name: String,
    /// HTTP status (200 on success).
    pub status: u16,
    /// Tool calls in OpenAI format (may be empty).
    pub tool_calls: Vec<CascadeToolCall>,
    /// Text content (final answer), if any.
    pub text: Option<String>,
    /// Raw response value (for debugging / Gemini-specific extraction).
    pub raw: Value,
}

#[derive(Debug, Clone)]
pub struct CascadeToolCall {
    pub id: String,
    pub function_name: String,
    pub arguments: String,
}

impl ProviderCascade {
    /// Build a cascade from resolved provider configs.
    /// Providers with empty API keys are silently skipped.
    pub fn new(configs: Vec<ProviderConfig>) -> Self {
        let providers = configs
            .into_iter()
            .filter(|c| !c.api_key.is_empty())
            .map(|c| {
                let openai_client = if c.api_type == ApiType::OpenAI {
                    Some(OpenAIClient::new(
                        c.api_key.clone(),
                        c.api_base.clone(),
                        c.model.clone(),
                        c.name.clone(),
                    ))
                } else {
                    None
                };
                let http_client = Client::builder()
                    .timeout(GEMINI_REQUEST_TIMEOUT)
                    .build()
                    .unwrap_or_else(|_| Client::new());
                ProviderEntry {
                    config: c,
                    openai_client,
                    http_client,
                }
            })
            .collect::<Vec<_>>();

        if providers.is_empty() {
            warn!("ProviderCascade: no providers configured with valid API keys!");
        } else {
            info!(
                "ProviderCascade: {} provider(s) loaded: {}",
                providers.len(),
                providers
                    .iter()
                    .map(|p| format!("{} ({})", p.config.name, p.config.model))
                    .collect::<Vec<_>>()
                    .join(" → ")
            );
        }

        Self { providers }
    }

    /// Return the number of active providers.
    pub fn len(&self) -> usize {
        self.providers.len()
    }

    /// Summary string for logging / debug.
    pub fn summary(&self) -> String {
        self.providers
            .iter()
            .map(|p| p.config.name.clone())
            .collect::<Vec<_>>()
            .join(" → ")
    }

    /// Call the cascade with OpenAI-format messages and tools.
    ///
    /// For OpenAI-type providers the messages/tools are sent as-is.
    /// For Gemini-type providers they are translated to Gemini's format.
    ///
    /// `system_prompt` is extracted separately because Gemini uses
    /// `system_instruction` while OpenAI uses a system role message.
    pub async fn call(
        &self,
        system_prompt: &str,
        messages: &[Value],
        tools: &[Value],
    ) -> Result<CascadeResponse> {
        let mut last_error: Option<anyhow::Error> = None;

        for entry in &self.providers {
            let result = match entry.config.api_type {
                ApiType::OpenAI => {
                    self.call_openai(entry, system_prompt, messages, tools)
                        .await
                }
                ApiType::Gemini => {
                    self.call_gemini(entry, system_prompt, messages, tools)
                        .await
                }
            };

            match result {
                Ok(resp) => {
                    if resp.status == 200 {
                        info!("[Cascade] {} responded successfully", entry.config.name);
                        return Ok(resp);
                    }
                    // Extract error detail from response for logging
                    let detail = resp.raw["error"]["message"]
                        .as_str()
                        .or_else(|| resp.raw["error"].as_str())
                        .unwrap_or("")
                        .chars()
                        .take(200)
                        .collect::<String>();

                    // Cascade on retryable errors
                    match resp.status {
                        429 => {
                            warn!(
                                "[{}] Rate limited (429), trying next. {}",
                                entry.config.name, detail
                            );
                        }
                        401 | 403 => {
                            warn!(
                                "[{}] Auth error ({}), trying next. {}",
                                entry.config.name, resp.status, detail
                            );
                        }
                        s if s >= 500 => {
                            warn!(
                                "[{}] Server error ({}), trying next. {}",
                                entry.config.name, resp.status, detail
                            );
                        }
                        _ => {
                            warn!(
                                "[{}] HTTP {}, trying next. {}",
                                entry.config.name, resp.status, detail
                            );
                        }
                    }
                    last_error =
                        Some(anyhow::anyhow!("[{}] HTTP {}: {}", entry.config.name, resp.status, detail));
                }
                Err(e) => {
                    warn!("[{}] Request failed: {}, trying next provider", entry.config.name, e);
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("No providers configured")))
    }

    /// Call a Gemini-only endpoint for non-tool calls (reflection, fact extraction, summaries).
    /// Falls through cascade like `call()` but without tools.
    pub async fn call_simple(
        &self,
        system_prompt: &str,
        messages: &[Value],
    ) -> Result<CascadeResponse> {
        self.call(system_prompt, messages, &[]).await
    }

    // ── OpenAI path ──────────────────────────────────────────────────────────

    async fn call_openai(
        &self,
        entry: &ProviderEntry,
        system_prompt: &str,
        messages: &[Value],
        tools: &[Value],
    ) -> Result<CascadeResponse> {
        let client = entry
            .openai_client
            .as_ref()
            .expect("OpenAI client must exist for OpenAI-type provider");

        // Prepend system message
        let mut full_messages = vec![json!({"role": "system", "content": system_prompt})];
        full_messages.extend_from_slice(messages);

        let resp = client.chat_completion(&full_messages, tools).await?;

        Ok(CascadeResponse {
            provider_name: entry.config.name.clone(),
            status: resp.status,
            tool_calls: resp
                .tool_calls
                .into_iter()
                .map(|tc| CascadeToolCall {
                    id: tc.id,
                    function_name: tc.function_name,
                    arguments: tc.arguments,
                })
                .collect(),
            text: resp.text,
            raw: resp.raw,
        })
    }

    // ── Gemini path ──────────────────────────────────────────────────────────

    async fn call_gemini(
        &self,
        entry: &ProviderEntry,
        system_prompt: &str,
        messages: &[Value],
        tools: &[Value],
    ) -> Result<CascadeResponse> {
        let url = format!(
            "{}/models/{}:generateContent?key={}",
            entry.config.api_base.trim_end_matches('/'),
            entry.config.model,
            entry.config.api_key
        );

        // Convert OpenAI messages → Gemini contents
        let contents = openai_messages_to_gemini(messages);

        // Convert OpenAI tools → Gemini tools
        let gemini_tools = openai_tools_to_gemini(tools);

        let mut body = json!({
            "system_instruction": {
                "parts": [{"text": system_prompt}]
            },
            "contents": contents,
        });

        if !gemini_tools.is_empty() {
            body["tools"] = json!([{"function_declarations": gemini_tools}]);
        }

        let resp = entry
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .await?;

        let status = resp.status().as_u16();
        let body_text = resp.text().await?;
        let raw: Value = serde_json::from_str(&body_text).unwrap_or_else(|_| {
            json!({"error": {"message": format!("Non-JSON: {}", &body_text[..body_text.len().min(500)])}})
        });

        if status != 200 {
            return Ok(CascadeResponse {
                provider_name: entry.config.name.clone(),
                status,
                tool_calls: vec![],
                text: None,
                raw,
            });
        }

        // Check for Gemini API-level errors in 200 responses
        if raw.get("error").is_some() {
            let code = raw["error"]["code"].as_u64().unwrap_or(500) as u16;
            return Ok(CascadeResponse {
                provider_name: entry.config.name.clone(),
                status: code,
                tool_calls: vec![],
                text: None,
                raw,
            });
        }

        // Parse Gemini response → CascadeResponse
        let parts = raw["candidates"][0]["content"]["parts"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        let mut tool_calls = Vec::new();
        let mut text = None;

        for part in &parts {
            if let Some(fc) = part.get("functionCall") {
                let name = fc["name"].as_str().unwrap_or("").to_string();
                let args = fc["args"].clone();
                // Gemini returns args as an object; serialize to string for consistency
                let arguments = serde_json::to_string(&args).unwrap_or_else(|_| "{}".to_string());
                tool_calls.push(CascadeToolCall {
                    id: format!("gemini_{}", tool_calls.len()),
                    function_name: name,
                    arguments,
                });
            }
            if let Some(t) = part.get("text") {
                text = t.as_str().map(|s| s.to_string());
            }
        }

        Ok(CascadeResponse {
            provider_name: entry.config.name.clone(),
            status: 200,
            tool_calls,
            text,
            raw,
        })
    }
}

// ── Format translation: OpenAI → Gemini ──────────────────────────────────────

/// Convert OpenAI-format messages to Gemini `contents` array.
fn openai_messages_to_gemini(messages: &[Value]) -> Vec<Value> {
    let mut contents = Vec::new();

    for msg in messages {
        let role = msg["role"].as_str().unwrap_or("user");

        match role {
            "system" => {
                // System messages are handled via system_instruction, skip here
            }
            "user" => {
                let mut parts = Vec::new();
                if let Some(content) = msg["content"].as_str() {
                    parts.push(json!({"text": content}));
                } else if let Some(content_arr) = msg["content"].as_array() {
                    // Multi-modal content (text + images)
                    for item in content_arr {
                        if item["type"].as_str() == Some("text") {
                            parts.push(json!({"text": item["text"]}));
                        } else if item["type"].as_str() == Some("image_url") {
                            // Convert OpenAI image_url format to Gemini inlineData
                            if let Some(url) = item["image_url"]["url"].as_str() {
                                if let Some(b64_data) = parse_data_url(url) {
                                    parts.push(json!({"inlineData": {
                                        "mimeType": b64_data.0,
                                        "data": b64_data.1
                                    }}));
                                }
                            }
                        }
                    }
                }
                if !parts.is_empty() {
                    contents.push(json!({"role": "user", "parts": parts}));
                }
            }
            "assistant" => {
                let mut parts = Vec::new();
                if let Some(content) = msg["content"].as_str() {
                    if !content.is_empty() {
                        parts.push(json!({"text": content}));
                    }
                }
                // Convert tool_calls → Gemini functionCall parts
                if let Some(tcs) = msg["tool_calls"].as_array() {
                    for tc in tcs {
                        let name = tc["function"]["name"].as_str().unwrap_or("");
                        let args_str = tc["function"]["arguments"].as_str().unwrap_or("{}");
                        let args: Value =
                            serde_json::from_str(args_str).unwrap_or_else(|_| json!({}));
                        parts.push(json!({"functionCall": {"name": name, "args": args}}));
                    }
                }
                if !parts.is_empty() {
                    contents.push(json!({"role": "model", "parts": parts}));
                }
            }
            "tool" => {
                // Convert OpenAI tool response → Gemini functionResponse
                let tool_call_id = msg["tool_call_id"].as_str().unwrap_or("");
                let content = msg["content"].as_str().unwrap_or("");
                // Extract function name from the tool_call_id or use a generic name
                let name = msg["name"].as_str().unwrap_or(
                    // Try to match by tool_call_id in previous messages (best effort)
                    tool_call_id,
                );
                contents.push(json!({
                    "role": "user",
                    "parts": [{"functionResponse": {
                        "name": name,
                        "response": {"output": content}
                    }}]
                }));
            }
            _ => {
                // Unknown role — treat as user text
                if let Some(content) = msg["content"].as_str() {
                    contents.push(json!({"role": "user", "parts": [{"text": content}]}));
                }
            }
        }
    }

    contents
}

/// Convert OpenAI tools array to Gemini function_declarations.
fn openai_tools_to_gemini(tools: &[Value]) -> Vec<Value> {
    tools
        .iter()
        .filter_map(|tool| {
            if tool["type"].as_str() == Some("function") {
                Some(tool["function"].clone())
            } else {
                None
            }
        })
        .collect()
}

/// Parse a data URL like "data:image/jpeg;base64,/9j/..." into (mime_type, base64_data).
fn parse_data_url(url: &str) -> Option<(String, String)> {
    if !url.starts_with("data:") {
        return None;
    }
    let rest = &url[5..];
    let semicolon = rest.find(';')?;
    let mime = &rest[..semicolon];
    let after_semi = &rest[semicolon + 1..];
    if !after_semi.starts_with("base64,") {
        return None;
    }
    let data = &after_semi[7..];
    Some((mime.to_string(), data.to_string()))
}
