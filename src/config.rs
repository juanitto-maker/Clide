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

    // Matrix/Element settings
    pub matrix_homeserver: String,
    pub matrix_user: String,
    pub matrix_access_token: String,
    pub matrix_room_id: String,

    #[serde(default)]
    pub require_confirmation: bool,
    #[serde(default = "default_timeout")]
    pub confirmation_timeout: u64,

    // Matrix user IDs allowed to send commands to the bot
    #[serde(default)]
    pub authorized_users: Vec<String>,

    // Commands blocked from execution in executor.rs
    #[serde(default = "default_blocked_commands")]
    pub blocked_commands: Vec<String>,

    #[serde(default)]
    pub logging: LoggingConfig,
}

fn default_model() -> String {
    "gemini-2.0-flash".to_string()
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
            .map_err(|e| {
                anyhow::anyhow!(
                    "Cannot read config {:?}: {}\nRun installer or copy config.example.yaml",
                    path,
                    e,
                )
            })?;
        let mut cfg: Config = serde_yaml::from_str(&content)?;

        // Allow env vars to override sensitive values
        if let Ok(key) = std::env::var("GEMINI_API_KEY") {
            if !key.is_empty() {
                cfg.gemini_api_key = key;
            }
        }
        if let Ok(token) = std::env::var("MATRIX_ACCESS_TOKEN") {
            if !token.is_empty() {
                cfg.matrix_access_token = token;
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

    pub fn is_authorized(&self, user: &str) -> bool {
        self.authorized_users.contains(&user.to_string())
    }
}
