// ============================================
// openai_client.rs - OpenAI-Compatible API Client
// ============================================
// Speaks the OpenAI chat completions API format used by Groq, Cerebras,
// OpenRouter, and any other OpenAI-compatible provider.

use anyhow::{Context, Result};
use log::{info, warn};
use reqwest::Client;
use serde_json::{json, Value};

pub struct OpenAIClient {
    client: Client,
    api_key: String,
    api_base: String,
    model: String,
    provider_name: String,
}

/// Parsed response from an OpenAI-compatible chat completion endpoint.
#[derive(Debug)]
pub struct OpenAIResponse {
    /// HTTP status code from the upstream provider.
    pub status: u16,
    /// If the model returned tool calls, they are here.
    pub tool_calls: Vec<ToolCall>,
    /// If the model returned a text response, it is here.
    pub text: Option<String>,
    /// Raw response body (for debugging).
    pub raw: Value,
}

#[derive(Debug, Clone)]
pub struct ToolCall {
    pub id: String,
    pub function_name: String,
    pub arguments: String,
}

impl OpenAIClient {
    pub fn new(
        api_key: String,
        api_base: String,
        model: String,
        provider_name: String,
    ) -> Self {
        Self {
            client: Client::new(),
            api_key,
            api_base,
            model,
            provider_name,
        }
    }

    pub fn provider_name(&self) -> &str {
        &self.provider_name
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    /// Send a chat completion request to the OpenAI-compatible endpoint.
    ///
    /// `messages` should be in OpenAI message format (role/content/tool_calls/tool_call_id).
    /// `tools` should be in OpenAI tools format (type: function, function: {name, description, parameters}).
    pub async fn chat_completion(
        &self,
        messages: &[Value],
        tools: &[Value],
    ) -> Result<OpenAIResponse> {
        let url = format!("{}/chat/completions", self.api_base.trim_end_matches('/'));

        let mut body = json!({
            "model": self.model,
            "messages": messages,
        });

        if !tools.is_empty() {
            body["tools"] = json!(tools);
        }

        info!("[{}] Sending chat completion ({} messages)", self.provider_name, messages.len());

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .context(format!("[{}] HTTP request failed", self.provider_name))?;

        let status = resp.status().as_u16();

        let body_text = resp
            .text()
            .await
            .context(format!("[{}] Failed to read response body", self.provider_name))?;

        let raw: Value = serde_json::from_str(&body_text).unwrap_or_else(|_| {
            json!({"error": format!("Non-JSON response: {}", &body_text[..body_text.len().min(500)])})
        });

        if status != 200 {
            let error_msg = raw["error"]["message"]
                .as_str()
                .or_else(|| raw["error"].as_str())
                .unwrap_or("Unknown error");
            warn!(
                "[{}] HTTP {} — {}",
                self.provider_name, status, error_msg
            );
            return Ok(OpenAIResponse {
                status,
                tool_calls: vec![],
                text: None,
                raw,
            });
        }

        // Parse successful response
        let message = &raw["choices"][0]["message"];

        let mut tool_calls = Vec::new();
        if let Some(tcs) = message["tool_calls"].as_array() {
            for tc in tcs {
                let id = tc["id"].as_str().unwrap_or("").to_string();
                let function_name = tc["function"]["name"].as_str().unwrap_or("").to_string();
                let arguments = tc["function"]["arguments"]
                    .as_str()
                    .unwrap_or("{}")
                    .to_string();
                tool_calls.push(ToolCall {
                    id,
                    function_name,
                    arguments,
                });
            }
        }

        let text = message["content"].as_str().map(|s| s.to_string());

        info!(
            "[{}] Response: {} tool_call(s), text={}",
            self.provider_name,
            tool_calls.len(),
            text.is_some()
        );

        Ok(OpenAIResponse {
            status,
            tool_calls,
            text,
            raw,
        })
    }
}
