// ============================================
// config.rs - Configuration Management (CORRECTED)
// ============================================

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Main configuration struct
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub gemini_api_key: String,
    #[serde(default = "default_model")]
    pub gemini_model: String,
    pub signal_number: String,
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
    #[serde(default)]
    pub ssh_key_path: Option<String>,
    #[serde(default)]
    pub ssh_verify_host_keys: bool,
    #[serde(default)]
    pub allowed_ssh_hosts: Vec<String>,
    #[serde(default)]
    pub ssh_timeout: u64,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub authorized_numbers: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    pub file_path: Option<PathBuf>,
    #[serde(default)]
    pub json: bool,
    #[serde(default = "default_true")]
    pub with_timestamps: bool,
    #[serde(default)]
    pub with_caller: bool,
}

fn default_model() -> String {
    "gemini-2.5-flash".to_string()
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_true() -> bool {
    true
}

impl Config {
    /// Returns the Gemini model (with fallback)
    pub fn get_model(&self) -> &str {
        if self.gemini_model.is_empty() {
            "gemini-2.5-flash"
        } else {
            &self.gemini_model
        }
    }
}
