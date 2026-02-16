// ============================================
// config.rs - Configuration Management (CORRECTED)
// ============================================

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    // API Keys
    pub gemini_api_key: String,
    pub signal_number: String,

    // Security
    #[serde(default)]
    pub authorized_numbers: Vec<String>,
    #[serde(default)]
    pub require_confirmation: bool,
    #[serde(default = "default_timeout")]
    pub confirmation_timeout: u64,
    #[serde(default = "default_true")]
    pub allow_commands: bool,
    #[serde(default)]
    pub deny_by_default: bool,
    #[serde(default)]
    pub allowed_commands: Vec<String>,
    #[serde(default = "default_blocked_commands")]
    pub blocked_commands: Vec<String>,
    #[serde(default)]
    pub dry_run: bool,

    // SSH
    #[serde(default)]
    pub ssh_key_path: Option<String>,
    #[serde(default = "default_true")]
    pub ssh_verify_host_keys: bool,
    #[serde(default)]
    pub allowed_ssh_hosts: Vec<String>,
    #[serde(default = "default_ssh_timeout")]
    pub ssh_timeout: u64,

    // Logging
    #[serde(default)]
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default)]
    pub file_path: Option<String>,
    #[serde(default)]
    pub json: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            file_path: None,
            json: false,
        }
    }
}

fn default_timeout() -> u64 { 60 }
fn default_ssh_timeout() -> u64 { 30 }
fn default_true() -> bool { true }
fn default_log_level() -> String { "info".to_string() }
fn default_blocked_commands() -> Vec<String> {
    vec!["rm -rf /".to_string(), "mkfs".to_string(), "dd".to_string()]
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .context("Failed to read config file")?;
        
        // Substitute environment variables
        let substituted = substitute_env_vars(&content);
        
        let config: Config = serde_yaml::from_str(&substituted)
            .context("Failed to parse config YAML")?;
            
        Ok(config)
    }
}

fn substitute_env_vars(text: &str) -> String {
    let mut result = text.to_string();
    let re = regex::Regex::new(r"\$\{([^}]+)\}").unwrap();
    
    for cap in re.captures_iter(text) {
        let full_match = &cap[0];
        let var_name = &cap[1];
        
        if let Ok(value) = std::env::var(var_name) {
            result = result.replace(full_match, &value);
        }
    }
    result
}
