// ============================================
// config.rs - Configuration Management
// ============================================

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use dirs::home_dir;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    // API Configuration
    pub gemini_api_key: Option<String>,
    pub gemini_model: String,
    pub temperature: f32,
    pub max_tokens: usize,
    
    // Signal Configuration
    pub signal_number: String,
    
    // Bot Settings
    pub allow_commands: bool,
    pub require_confirmation: bool,
    pub log_level: String,
    
    // System Settings
    pub system_prompt: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            gemini_api_key: None,
            gemini_model: "gemini-2.5-flash".to_string(), // ✅ Updated to 2.5-flash
            temperature: 0.7,
            max_tokens: 2048,
            signal_number: "+1234567890".to_string(),
            allow_commands: true,
            require_confirmation: false,
            log_level: "info".to_string(),
            system_prompt: "You are a helpful AI assistant.".to_string(),
        }
    }
}

impl Config {
    pub fn load(path: Option<PathBuf>) -> Result<Self> {
        let config_path = path.unwrap_or_else(|| {
            let mut p = home_dir().expect("Could not find home directory");
            p.push(".clide");
            p.push("config.yaml");
            p
        });

        // Load config file
        let config_str = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config file: {:?}", config_path))?;
        
        let mut config: Config = serde_yaml::from_str(&config_str)
            .context("Failed to parse config YAML")?;

        // ✅ SECURITY: Check environment variable first, then config file
        config.gemini_api_key = std::env::var("GEMINI_API_KEY")
            .ok()
            .or(config.gemini_api_key);

        // Validate API key exists
        if config.gemini_api_key.is_none() {
            anyhow::bail!(
                "No Gemini API key found!\n\
                 Set it via:\n\
                 1. Environment: export GEMINI_API_KEY='your-key'\n\
                 2. Config file: {:?}",
                config_path
            );
        }

        Ok(config)
    }

    pub fn get_api_key(&self) -> &str {
        self.gemini_api_key.as_ref().expect("API key not set")
    }

    pub fn show(&self) {
        println!("📋 Current Configuration:");
        println!("  Model: {}", self.gemini_model);
        println!("  Temperature: {}", self.temperature);
        println!("  Max Tokens: {}", self.max_tokens);
        println!("  Signal: {}", self.signal_number);
        println!("  Allow Commands: {}", self.allow_commands);
        println!("  Require Confirmation: {}", self.require_confirmation);
        println!("  Log Level: {}", self.log_level);
        println!("  API Key: {}", 
            if self.gemini_api_key.is_some() { "✅ Set" } else { "❌ Not set" }
        );
    }
}
