// ============================================
// config.rs - Configuration Management
// ============================================
// Loads and validates configuration from YAML file
// Supports environment variable substitution

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

    // Execution
    #[serde(default)]
    pub execution: ExecutionConfig,

    // Gemini
    #[serde(default)]
    pub gemini: GeminiConfig,

    // Rate Limiting
    #[serde(default)]
    pub rate_limit: RateLimitConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    pub file: Option<String>,
    #[serde(default = "default_max_size")]
    pub max_size: String,
    #[serde(default = "default_max_backups")]
    pub max_backups: usize,
    #[serde(default = "default_true")]
    pub compress: bool,
    #[serde(default = "default_true")]
    pub log_commands: bool,
    #[serde(default)]
    pub log_output: bool,
    #[serde(default)]
    pub log_api_calls: bool,
    #[serde(default)]
    pub json_format: bool,
    #[serde(default = "default_true")]
    pub timestamps: bool,
    #[serde(default)]
    pub caller_info: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    #[serde(default = "default_execution_timeout")]
    pub timeout: u64,
    #[serde(default = "default_working_dir")]
    pub working_dir: String,
    #[serde(default = "default_shell")]
    pub shell: String,
    #[serde(default = "default_shell_flags")]
    pub shell_flags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiConfig {
    #[serde(default = "default_gemini_model")]
    pub model: String,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
    #[serde(default = "default_system_prompt")]
    pub system_prompt: String,
    #[serde(default = "default_retry_count")]
    pub retry_count: usize,
    #[serde(default = "default_retry_delay")]
    pub retry_delay: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    #[serde(default = "default_commands_per_minute")]
    pub commands_per_minute: usize,
    #[serde(default = "default_api_calls_per_minute")]
    pub api_calls_per_minute: usize,
    #[serde(default = "default_ssh_per_minute")]
    pub ssh_per_minute: usize,
}

// Default value functions
fn default_true() -> bool { true }
fn default_timeout() -> u64 { 60 }
fn default_ssh_timeout() -> u64 { 30 }
fn default_execution_timeout() -> u64 { 300 }
fn default_log_level() -> String { "info".to_string() }
fn default_max_size() -> String { "100MB".to_string() }
fn default_max_backups() -> usize { 10 }
fn default_working_dir() -> String { "~".to_string() }
fn default_shell() -> String { "/bin/bash".to_string() }
fn default_shell_flags() -> Vec<String> { vec!["-c".to_string()] }
fn default_gemini_model() -> String { "gemini-pro".to_string() }
fn default_temperature() -> f32 { 0.7 }
fn default_max_tokens() -> usize { 2048 }
fn default_retry_count() -> usize { 3 }
fn default_retry_delay() -> u64 { 2 }
fn default_commands_per_minute() -> usize { 60 }
fn default_api_calls_per_minute() -> usize { 60 }
fn default_ssh_per_minute() -> usize { 30 }

fn default_system_prompt() -> String {
    "You are Clide, an AI assistant that helps execute terminal commands. \
    Be concise and helpful. When suggesting commands, explain what they do. \
    Always prioritize safety and ask for confirmation for destructive operations.".to_string()
}

fn default_blocked_commands() -> Vec<String> {
    vec![
        "rm -rf /*".to_string(),
        "rm -rf ~/*".to_string(),
        "mkfs.*".to_string(),
        "dd if=*".to_string(),
        "chmod 777 *".to_string(),
        "chmod -R 777 *".to_string(),
        "passwd*".to_string(),
        "userdel*".to_string(),
        "shutdown*".to_string(),
        "reboot*".to_string(),
        "init 0".to_string(),
        "init 6".to_string(),
        ":(){ :|:& };:".to_string(), // Fork bomb
    ]
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            file: None,
            max_size: default_max_size(),
            max_backups: default_max_backups(),
            compress: true,
            log_commands: true,
            log_output: false,
            log_api_calls: false,
            json_format: false,
            timestamps: true,
            caller_info: false,
        }
    }
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            timeout: default_execution_timeout(),
            working_dir: default_working_dir(),
            shell: default_shell(),
            shell_flags: default_shell_flags(),
        }
    }
}

impl Default for GeminiConfig {
    fn default() -> Self {
        Self {
            model: default_gemini_model(),
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
            system_prompt: default_system_prompt(),
            retry_count: default_retry_count(),
            retry_delay: default_retry_delay(),
        }
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            commands_per_minute: default_commands_per_minute(),
            api_calls_per_minute: default_api_calls_per_minute(),
            ssh_per_minute: default_ssh_per_minute(),
        }
    }
}

impl Config {
    /// Load configuration from YAML file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        
        // Expand ~ in path
        let expanded_path = expand_tilde(path);
        
        let contents = std::fs::read_to_string(&expanded_path)
            .context(format!("Failed to read config file: {:?}", expanded_path))?;
        
        // Substitute environment variables
        let contents = substitute_env_vars(&contents);
        
        let config: Config = serde_yaml::from_str(&contents)
            .context("Failed to parse config file")?;
        
        // Validate
        config.validate()?;
        
        Ok(config)
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        // Check API key
        if self.gemini_api_key.is_empty() || self.gemini_api_key == "YOUR_GEMINI_API_KEY_HERE" {
            anyhow::bail!("Gemini API key not configured");
        }

        // Check Signal number
        if self.signal_number.is_empty() || !self.signal_number.starts_with('+') {
            anyhow::bail!("Invalid Signal number format (should start with +)");
        }

        Ok(())
    }

    /// Get log file path with expansion
    pub fn log_file_path(&self) -> Option<PathBuf> {
        self.logging.file.as_ref().map(|p| expand_tilde(Path::new(p)))
    }

    /// Get SSH key path with expansion
    pub fn ssh_key_path(&self) -> Option<PathBuf> {
        self.ssh_key_path.as_ref().map(|p| expand_tilde(Path::new(p)))
    }
}

/// Expand ~ to home directory
fn expand_tilde(path: &Path) -> PathBuf {
    if path.starts_with("~") {
        if let Some(home) = dirs::home_dir() {
            let path_str = path.to_string_lossy();
            let expanded = path_str.replacen("~", &home.to_string_lossy(), 1);
            return PathBuf::from(expanded);
        }
    }
    path.to_path_buf()
}

/// Substitute environment variables in format ${VAR_NAME}
fn substitute_env_vars(text: &str) -> String {
    let mut result = text.to_string();
    
    // Find all ${VAR_NAME} patterns
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_substitute_env_vars() {
        std::env::set_var("TEST_VAR", "hello");
        let result = substitute_env_vars("Value is ${TEST_VAR}!");
        assert_eq!(result, "Value is hello!");
    }

    #[test]
    fn test_expand_tilde() {
        let path = Path::new("~/test");
        let expanded = expand_tilde(path);
        assert!(!expanded.to_string_lossy().contains('~'));
    }

    #[test]
    fn test_default_config() {
        let logging = LoggingConfig::default();
        assert_eq!(logging.level, "info");
        assert!(logging.log_commands);
    }
}
