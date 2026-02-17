// ============================================
// config.rs - Configuration Loader (UPDATED)
// ============================================

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    // Gemini API
    pub gemini_api_key: String,
    #[serde(default = "default_model")]
    pub gemini_model: String,

    // Signal
    pub signal_number: String,

    // Security
    #[serde(default)]
    pub authorized_numbers: Vec<String>,
    #[serde(default)]
    pub require_confirmation: bool,
    #[serde(default = "default_confirmation_timeout")]
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

    // SSH
    #[serde(default)]
    pub ssh_key_path: Option<String>,
    #[serde(default = "default_ssh_verify")]
    pub ssh_verify_host_keys: bool,
    #[serde(default)]
    pub allowed_ssh_hosts: Vec<String>,
    #[serde(default = "default_ssh_timeout")]
    pub ssh_timeout: u64,

    // Logging
    #[serde(default)]
    pub logging: LoggingConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default)]
    pub file_path: Option<String>,
    #[serde(default)]
    pub json: bool,
}

fn default_model() -> String { "gemini-2.5-flash".to_string() }
fn default_confirmation_timeout() -> u64 { 60 }
fn default_ssh_verify() -> bool { true }
fn default_ssh_timeout() -> u64 { 30 }
fn default_log_level() -> String { "info".to_string() }

impl Config {
    pub fn load(path: Option<PathBuf>) -> Result<Self> {
        let path = path.unwrap_or_else(|| {
            dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join(".clide/config.yaml")
        });

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file at {:?}", path))?;
        let mut config: Config = serde_yaml::from_str(&content)
            .context("Failed to parse YAML configuration")?;

        // Expand ${ENV_VAR} placeholders
        if config.gemini_api_key.starts_with("${") && config.gemini_api_key.ends_with("}") {
            let var_name = &config.gemini_api_key[2..config.gemini_api_key.len()-1];
            config.gemini_api_key = std::env::var(var_name)
                .with_context(|| format!("Environment variable {} not set", var_name))?;
        }

        Ok(config)
    }

    pub fn logging_level(&self) -> &str {
        &self.logging.level
    }
}
