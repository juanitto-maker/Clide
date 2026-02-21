// ============================================
// config.rs - Configuration (YAML, ~/.clide/config.yaml)
// ============================================

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub gemini_api_key: String,
    #[serde(default = "default_model")]
    pub gemini_model: String,

    /// Which messaging platform(s) to use: "matrix", "telegram", or "both"
    #[serde(default = "default_platform")]
    pub platform: String,

    // Matrix/Element settings (required when platform is "matrix" or "both")
    #[serde(default)]
    pub matrix_homeserver: String,
    #[serde(default)]
    pub matrix_user: String,
    #[serde(default)]
    pub matrix_access_token: String,
    #[serde(default)]
    pub matrix_room_id: String,

    // Telegram settings (required when platform is "telegram" or "both")
    #[serde(default)]
    pub telegram_bot_token: String,

    // Additional AI provider keys — loaded from secrets.yaml or env vars.
    // These are available as ${ANTHROPIC_API_KEY} etc. in skill commands.
    #[serde(default)]
    pub anthropic_api_key: String,
    #[serde(default)]
    pub openai_api_key: String,
    #[serde(default)]
    pub groq_api_key: String,
    #[serde(default)]
    pub xai_api_key: String,

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

    /// Maximum tool-call steps per agent task (default 20)
    #[serde(default = "default_agent_steps")]
    pub max_agent_steps: usize,

    #[serde(default)]
    pub logging: LoggingConfig,

    /// All secrets from ~/.clide/secrets.yaml plus env overrides.
    /// Available as ${KEY_NAME} placeholders in skill commands.
    /// Never serialised back to disk.
    #[serde(skip)]
    pub secrets: HashMap<String, String>,
}

fn default_model() -> String {
    "gemini-2.5-flash".to_string()
}

fn default_platform() -> String {
    "matrix".to_string()
}

fn default_timeout() -> u64 {
    60
}

fn default_agent_steps() -> usize {
    20
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
    /// Load config from ~/.clide/config.yaml, then overlay secrets.yaml,
    /// then override with environment variables (highest priority).
    pub fn load() -> anyhow::Result<Self> {
        let path = Self::path();
        let raw = std::fs::read_to_string(&path)
            .map_err(|e| {
                anyhow::anyhow!(
                    "Cannot read config {:?}: {}\nRun installer or copy config.example.yaml",
                    path,
                    e,
                )
            })?;
        // Strip control characters that YAML doesn't allow.
        // Only tab (0x09) and newline (0x0A) are valid control chars in YAML.
        // This silently removes \r (Windows line endings) and stray control
        // chars that can creep in when copy-pasting tokens from apps like
        // Telegram, email clients, or web browsers.
        let content: String = raw
            .chars()
            .filter(|&c| c == '\t' || c == '\n' || (c >= ' ' && c != '\x7f'))
            .collect();
        let mut cfg: Config = serde_yaml::from_str(&content).map_err(|e| {
            anyhow::anyhow!(
                "{}\n\nHint: Make sure all values (especially matrix_access_token and matrix_room_id) \
are wrapped in double quotes in your config file.\n\
Example:  matrix_access_token: \"syt_abc123...\"\n\
Tokens or IDs containing special characters (like ':') must be quoted.",
                e
            )
        })?;

        // ── 1. Load ~/.clide/secrets.yaml (optional) ──────────────────────────
        let secrets_path = Self::secrets_path();
        if secrets_path.exists() {
            match std::fs::read_to_string(&secrets_path) {
                Ok(raw_secrets) => {
                    let secrets_content = raw_secrets.replace('\r', "");
                    match serde_yaml::from_str::<HashMap<String, String>>(&secrets_content) {
                        Ok(map) => {
                            // Apply known keys to their first-class config fields.
                            apply_secret(&map, "GEMINI_API_KEY",        &mut cfg.gemini_api_key);
                            apply_secret(&map, "MATRIX_ACCESS_TOKEN",   &mut cfg.matrix_access_token);
                            apply_secret(&map, "TELEGRAM_BOT_TOKEN",    &mut cfg.telegram_bot_token);
                            apply_secret(&map, "ANTHROPIC_API_KEY",     &mut cfg.anthropic_api_key);
                            apply_secret(&map, "OPENAI_API_KEY",        &mut cfg.openai_api_key);
                            apply_secret(&map, "GROQ_API_KEY",          &mut cfg.groq_api_key);
                            apply_secret(&map, "XAI_API_KEY",           &mut cfg.xai_api_key);
                            // Store everything — custom secrets are available in skills too.
                            cfg.secrets = map;
                        }
                        Err(e) => {
                            eprintln!("Warning: could not parse {:?}: {}", secrets_path, e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Warning: could not read {:?}: {}", secrets_path, e);
                }
            }
        }

        // ── 2. Environment variables (highest priority) ────────────────────────
        override_from_env("GEMINI_API_KEY",       &mut cfg.gemini_api_key,       &mut cfg.secrets);
        override_from_env("MATRIX_ACCESS_TOKEN",  &mut cfg.matrix_access_token,  &mut cfg.secrets);
        override_from_env("TELEGRAM_BOT_TOKEN",   &mut cfg.telegram_bot_token,   &mut cfg.secrets);
        override_from_env("ANTHROPIC_API_KEY",    &mut cfg.anthropic_api_key,    &mut cfg.secrets);
        override_from_env("OPENAI_API_KEY",       &mut cfg.openai_api_key,       &mut cfg.secrets);
        override_from_env("GROQ_API_KEY",         &mut cfg.groq_api_key,         &mut cfg.secrets);
        override_from_env("XAI_API_KEY",          &mut cfg.xai_api_key,          &mut cfg.secrets);

        // Sync first-class fields back into the secrets map so skills can
        // reference them as ${GEMINI_API_KEY} etc. even when set in config.yaml.
        sync_to_secrets(&cfg.gemini_api_key,       "GEMINI_API_KEY",       &mut cfg.secrets);
        sync_to_secrets(&cfg.matrix_access_token,  "MATRIX_ACCESS_TOKEN",  &mut cfg.secrets);
        sync_to_secrets(&cfg.telegram_bot_token,   "TELEGRAM_BOT_TOKEN",   &mut cfg.secrets);
        sync_to_secrets(&cfg.anthropic_api_key,    "ANTHROPIC_API_KEY",    &mut cfg.secrets);
        sync_to_secrets(&cfg.openai_api_key,       "OPENAI_API_KEY",       &mut cfg.secrets);
        sync_to_secrets(&cfg.groq_api_key,         "GROQ_API_KEY",         &mut cfg.secrets);
        sync_to_secrets(&cfg.xai_api_key,          "XAI_API_KEY",          &mut cfg.secrets);

        Ok(cfg)
    }

    pub fn path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_default();
        PathBuf::from(home).join(".clide/config.yaml")
    }

    /// Path to the optional centralised secrets file.
    pub fn secrets_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_default();
        PathBuf::from(home).join(".clide/secrets.yaml")
    }

    pub fn get_model(&self) -> &str {
        &self.gemini_model
    }

    pub fn is_authorized(&self, user: &str) -> bool {
        self.authorized_users.contains(&user.to_string())
    }
}

// ── Private helpers ────────────────────────────────────────────────────────

/// If `map` contains `key` with a non-empty value, copy it into `field`.
fn apply_secret(map: &HashMap<String, String>, key: &str, field: &mut String) {
    if let Some(v) = map.get(key) {
        if !v.is_empty() {
            *field = v.clone();
        }
    }
}

/// If the environment variable `key` is set and non-empty, override both the
/// config `field` and the `secrets` map entry.
fn override_from_env(key: &str, field: &mut String, secrets: &mut HashMap<String, String>) {
    if let Ok(v) = std::env::var(key) {
        if !v.is_empty() {
            *field = v.clone();
            secrets.insert(key.to_string(), v);
        }
    }
}

/// If `value` is non-empty and not yet present in `secrets`, add it so that
/// values set directly in config.yaml are also reachable as ${KEY} in skills.
fn sync_to_secrets(value: &str, key: &str, secrets: &mut HashMap<String, String>) {
    if !value.is_empty() {
        secrets.entry(key.to_string()).or_insert_with(|| value.to_string());
    }
}
