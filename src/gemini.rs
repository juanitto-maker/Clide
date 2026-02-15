// ============================================
// gemini.rs - Google Gemini API Client
// ============================================
// Direct REST API calls (no grpcio dependency!)
// Works perfectly on Termux

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};

const GEMINI_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models";

/// Gemini API client
pub struct GeminiClient {
    client: Client,
    api_key: String,
    model: String,
    temperature: f32,
    max_tokens: usize,
    system_prompt: String,
}

#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<Content>,
    #[serde(rename = "generationConfig")]
    generation_config: GenerationConfig,
}

#[derive(Debug, Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Debug, Serialize)]
struct Part {
    text: String,
}

#[derive(Debug, Serialize)]
struct GenerationConfig {
    temperature: f32,
    #[serde(rename = "maxOutputTokens")]
    max_output_tokens: usize,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    content: ResponseContent,
}

#[derive(Debug, Deserialize)]
struct ResponseContent {
    parts: Vec<ResponsePart>,
}

#[derive(Debug, Deserialize)]
struct ResponsePart {
    text: String,
}

impl GeminiClient {
    /// Create a new Gemini client
    pub fn new(
        api_key: String,
        model: String,
        temperature: f32,
        max_tokens: usize,
        system_prompt: String,
    ) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
            temperature,
            max_tokens,
            system_prompt,
        }
    }

    /// Generate text from prompt
    pub async fn generate(&self, prompt: &str) -> Result<String> {
        // Combine system prompt with user prompt
        let full_prompt = format!("{}\n\nUser: {}", self.system_prompt, prompt);

        let request = GeminiRequest {
            contents: vec![Content {
                parts: vec![Part {
                    text: full_prompt,
                }],
            }],
            generation_config: GenerationConfig {
                temperature: self.temperature,
                max_output_tokens: self.max_tokens,
            },
        };

        let url = format!(
            "{}/{}:generateContent?key={}",
            GEMINI_BASE_URL, self.model, self.api_key
        );

        debug!("Sending request to Gemini API");

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Gemini API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            error!("Gemini API error ({}): {}", status, error_text);
            anyhow::bail!("Gemini API returned error: {} - {}", status, error_text);
        }

        let gemini_response: GeminiResponse = response
            .json()
            .await
            .context("Failed to parse Gemini API response")?;

        // Extract text from response
        let text = gemini_response
            .candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .map(|p| p.text.clone())
            .unwrap_or_else(|| "No response generated".to_string());

        info!("Received response from Gemini API ({} chars)", text.len());

        Ok(text)
    }

    /// Generate with retry logic
    pub async fn generate_with_retry(
        &self,
        prompt: &str,
        max_retries: usize,
        retry_delay: u64,
    ) -> Result<String> {
        let mut attempts = 0;
        let mut last_error = None;

        while attempts < max_retries {
            match self.generate(prompt).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    attempts += 1;
                    last_error = Some(e);

                    if attempts < max_retries {
                        debug!(
                            "Gemini API call failed (attempt {}/{}), retrying in {}s...",
                            attempts, max_retries, retry_delay
                        );
                        tokio::time::sleep(tokio::time::Duration::from_secs(retry_delay)).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Failed after {} retries", max_retries)))
    }

    /// Analyze command for safety
    pub async fn analyze_command(&self, command: &str) -> Result<CommandAnalysis> {
        let prompt = format!(
            "Analyze this shell command for safety and potential risks:\n\n{}\n\n\
            Respond in JSON format with these fields:\n\
            - safe: boolean (true if safe to execute)\n\
            - risk_level: string (\"low\", \"medium\", \"high\")\n\
            - explanation: string (brief explanation)\n\
            - suggestion: string (safer alternative if applicable)",
            command
        );

        let response = self.generate(&prompt).await?;

        // Try to parse JSON response
        // In a real implementation, you'd want more robust parsing
        let analysis = serde_json::from_str::<CommandAnalysis>(&response)
            .or_else(|_| {
                // Fallback if not proper JSON
                Ok(CommandAnalysis {
                    safe: true,
                    risk_level: "unknown".to_string(),
                    explanation: response.clone(),
                    suggestion: None,
                })
            })?;

        Ok(analysis)
    }

    /// Suggest command based on natural language
    pub async fn suggest_command(&self, intent: &str) -> Result<String> {
        let prompt = format!(
            "User wants to: {}\n\n\
            Suggest a safe shell command to accomplish this. \
            Respond with ONLY the command, no explanation.",
            intent
        );

        self.generate(&prompt).await
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CommandAnalysis {
    pub safe: bool,
    pub risk_level: String,
    pub explanation: String,
    pub suggestion: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires API key
    async fn test_generate() {
        let api_key = std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY not set");
        let client = GeminiClient::new(
            api_key,
            "gemini-pro".to_string(),
            0.7,
            1024,
            "You are a helpful assistant.".to_string(),
        );

        let response = client.generate("Say hello!").await.unwrap();
        assert!(!response.is_empty());
    }

    #[test]
    fn test_client_creation() {
        let client = GeminiClient::new(
            "test_key".to_string(),
            "gemini-pro".to_string(),
            0.7,
            2048,
            "System prompt".to_string(),
        );

        assert_eq!(client.model, "gemini-pro");
        assert_eq!(client.temperature, 0.7);
    }
}
