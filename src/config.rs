// ============================================
// config.rs - Configuration (YAML, ~/.clide/config.yaml)
// ============================================

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub gemini_api_key: String,
    #[serde(default = "default_model")]
    pub gemini_model: String,
    pub signal_number: String,

    #[serde(default)]
    pub require_confirmation: bool,
    #[serde(default = "default_timeout")]
    pub confirmation_timeout: u64,

    #[serde(default)]
    pub authorized_numbers: Vec<String>,

    // Commands blocked from execution in executor.rs
    #[serde(default = "default_blocked_commands")]
    pub blocked_commands: Vec<String>,

    #[serde(default)]
    pub logging: LoggingConfig,
}

fn default_model() -> String {
    "gemini-1.5-flash".to_string()
}

fn default_timeout() -> u64 {
    60
}

fn default_blocked_commands() -> Vec<String> {
    vec![
        "rm -rf /".to_string(),
        "mkfs".to_string(),
        "dd if=".to_string(),
    ]
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct LoggingConfig {
    #[serde(default)]
    pub level: String,
}

impl Config {
    /// Load config from ~/.clide/config.yaml
    pub fn load() -> anyhow::Result<Self> {
        let path = Self::path();
        let content = std::fs::read_to_string(&path)
            .map_err(|e| anyhow::anyhow!("Cannot read config {:?}: {}\nRun installer or copy config.example.yaml", path, e))?;
        let mut cfg: Config = serde_yaml::from_str(&content)?;

        // Allow env var to override API key
        if let Ok(key) = std::env::var("GEMINI_API_KEY") {
            if !key.is_empty() {
                cfg.gemini_api_key = key;
            }
        }

        Ok(cfg)
    }

    pub fn path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_default();
        PathBuf::from(home).join(".clide/config.yaml")
    }

    pub fn get_model(&self) -> &str {
        &self.gemini_model
    }

    pub fn is_authorized(&self, number: &str) -> bool {
        self.authorized_numbers.contains(&number.to_string())
    }
}
