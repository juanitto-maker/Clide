// ============================================
// config.rs - Configuration Management (FIXED)
// ============================================

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub gemini_api_key: String,
    #[serde(default)]
    pub gemini_model: Option<String>,
    pub signal_number: String,

    // Security
    #[serde(default)]
    pub require_confirmation: bool,
    #[serde(default)]
    pub confirmation_timeout: u64,
    #[serde(default)]
    pub allow_commands: bool,
    #[serde(default)]
    pub deny_by_default: bool,
    #[serde(default)]
    pub allowed_commands: Vec<String>,
    #[serde(default)]
    pub blocked_commands: Vec<String>,
    #[serde(default)]
    pub dry_run: bool,

    // Signal authorized numbers
    #[serde(default)]
    pub authorized_numbers: Vec<String>,

    // SSH
    #[serde(default)]
    pub ssh_verify_host_keys: bool,
    #[serde(default)]
    pub allowed_ssh_hosts: Vec<String>,
    #[serde(default)]
    pub ssh_timeout: u64,

    #[serde(default)]
    pub logging: LoggingConfig,
}

impl Config {
    /// Get the Gemini model, defaulting to "gemini-2.5-flash"
    pub fn get_model(&self) -> String {
        self.gemini_model.clone().unwrap_or_else(|| "gemini-2.5-flash".to_string())
    }

    /// Load from YAML file
    pub fn load(path: &PathBuf) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)?;
        let cfg: Config = serde_yaml::from_str(&content)?;
        Ok(cfg)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct LoggingConfig {
    #[serde(default = "default_level")]
    pub level: String,
    #[serde(default)]
    pub file_path: Option<String>,
    #[serde(default)]
    pub json: bool,
}

fn default_level() -> String {
    "info".to_string()
}
