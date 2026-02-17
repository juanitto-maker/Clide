// ============================================
// config.rs - Configuration (UPDATED)
// ============================================

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Main configuration for Clide
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    // Gemini
    pub gemini_api_key: String,
    #[serde(default)]
    pub gemini_model: Option<String>,

    // Signal
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
    pub authorized_numbers: Vec<String>,

    // Dry run
    #[serde(default)]
    pub dry_run: bool,

    // SSH
    #[serde(default)]
    pub ssh_key_path: Option<PathBuf>,
    #[serde(default)]
    pub ssh_verify_host_keys: bool,
    #[serde(default)]
    pub allowed_ssh_hosts: Vec<String>,
    #[serde(default)]
    pub ssh_timeout: u64,

    // Logging
    #[serde(default)]
    pub logging: LoggingConfig,
}

/// Logging configuration
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default)]
    pub file_path: Option<PathBuf>,
    #[serde(default)]
    pub json: bool,
    #[serde(default)]
    pub with_timestamps: bool,
    #[serde(default)]
    pub with_caller: bool,
}

fn default_log_level() -> String {
    "info".to_string()
}

// --- Config helper methods ---
impl Config {
    pub fn get_model(&self) -> String {
        self.gemini_model
            .clone()
            .unwrap_or_else(|| "gemini-2.5-flash".to_string())
    }
}
