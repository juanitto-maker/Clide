// ============================================
// gemini.rs - Google Gemini API Client (CORRECTED)
// ============================================

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

const GEMINI_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models";

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
    #[serde(rename = "systemInstruction")]
    system_instruction: Option<Content>,
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
    #[serde(rename = "responseMimeType")]
    response_mime_type: String,
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

    pub async fn analyze_command(&self, command: &str, context: &str) -> Result<CommandAnalysis> {
        let prompt = format!(
            "Context of previous conversation:\n{}\n\nUser input: \"{}\"\n\n\
            Analyze the input and decide if it's safe to run as a shell command.",
            context, command
        );

        let url = format!(
            "{}/{}:generateContent?key={}",
            GEMINI_BASE_URL, self.model, self.api_key
        );

        let request = GeminiRequest {
            contents: vec![Content {
                parts: vec![Part { text: prompt }],
            }],
            generation_config: GenerationConfig {
                temperature: self.temperature,
                max_output_tokens: self.max_tokens,
                response_mime_type: "application/json".to_string(),
            },
            system_instruction: Some(Content {
                parts: vec![Part { text: self.system_prompt.clone() }],
            }),
        };

        let response = self.client.post(&url).json(&request).send().await?;
        let body: GeminiResponse = response.json().await?;

        let json_text = body.candidates.first()
            .context("No response from Gemini")?
            .content.parts.first()
            .context("Empty response parts")?
            .text.clone();

        // FIXED: Added explicit type parameters to the Result
        let analysis: CommandAnalysis = serde_json::from_str(&json_text)
            .map(|a| Ok::<CommandAnalysis, anyhow::Error>(a))?
            .context("Failed to parse analysis JSON")?;

        Ok(analysis)
    }

    pub async fn generate(&self, prompt: &str) -> Result<String> {
        let url = format!(
            "{}/{}:generateContent?key={}",
            GEMINI_BASE_URL, self.model, self.api_key
        );

        let request = GeminiRequest {
            contents: vec![Content {
                parts: vec![Part { text: prompt.to_string() }],
            }],
            generation_config: GenerationConfig {
                temperature: self.temperature,
                max_output_tokens: self.max_tokens,
                response_mime_type: "text/plain".to_string(),
            },
            system_instruction: None,
        };

        let response = self.client.post(&url).json(&request).send().await?;
        let body: GeminiResponse = response.json().await?;

        Ok(body.candidates[0].content.parts[0].text.clone())
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CommandAnalysis {
    pub safe: bool,
    pub risk_level: String,
    pub explanation: String,
    pub suggestion: Option<String>,
}
