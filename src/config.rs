// ============================================
// config.rs - Configuration (YAML, ~/.clide/config.yaml)
// ============================================

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::hosts;
use crate::pass_store;

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

    /// Maximum tool-call steps per agent task (default 40)
    #[serde(default = "default_agent_steps")]
    pub max_agent_steps: usize,

    /// Per-command timeout in seconds for run_command calls (default 120).
    /// Skills override this with their own `timeout` field.
    #[serde(default = "default_command_timeout")]
    pub command_timeout: u64,

    #[serde(default)]
    pub logging: LoggingConfig,

    /// Optional path to a markdown file whose contents are injected into the
    /// agent's system prompt on every startup.  Supports `~` for the home
    /// directory.  Example: `~/.clide/context.md`
    #[serde(default)]
    pub context_file: Option<String>,

    /// Fallback model to use when the primary model fails or for complex tasks.
    /// Example: "gemini-2.5-pro" for automatic escalation.
    #[serde(default)]
    pub fallback_model: Option<String>,

    /// Whether to automatically escalate to the fallback model when the primary
    /// model fails twice on the same task (default: true if fallback_model is set).
    #[serde(default = "default_auto_escalate")]
    pub auto_escalate: bool,

    /// Number of conversations between automatic summarizations (default: 5).
    #[serde(default = "default_summarize_interval")]
    pub summarize_interval: usize,

    /// Whether to extract structured facts from conversations (default: true).
    #[serde(default = "default_true")]
    pub extract_facts: bool,

    /// Whether to enable self-reflection/verification after task completion (default: true).
    #[serde(default = "default_true")]
    pub self_reflection: bool,

    /// Regex patterns for blocked commands. These are checked in addition to
    /// the simple string-match `blocked_commands` list.
    #[serde(default = "default_blocked_patterns")]
    pub blocked_patterns: Vec<String>,

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
    40
}

fn default_command_timeout() -> u64 {
    120
}

fn default_blocked_commands() -> Vec<String> {
    vec![
        "rm -rf /".to_string(),
        "mkfs".to_string(),
        "dd if=".to_string(),
    ]
}

fn default_auto_escalate() -> bool {
    true
}

fn default_summarize_interval() -> usize {
    5
}

fn default_true() -> bool {
    true
}

fn default_blocked_patterns() -> Vec<String> {
    vec![
        // Destructive filesystem operations
        r"rm\s+(-[a-zA-Z]*f[a-zA-Z]*\s+)?/\s*$".to_string(),
        r"rm\s+-[a-zA-Z]*r[a-zA-Z]*\s+/\s*$".to_string(),
        r"chmod\s+(-R\s+)?777\s+/".to_string(),
        r"chown\s+-R\s+.*\s+/\s*$".to_string(),
        r"mkfs\.\w+".to_string(),
        r"dd\s+.*if=.*of=/dev/".to_string(),
        // Fork bombs and resource exhaustion
        r":\(\)\s*\{.*\}.*:".to_string(),
        r"\.\s*/dev/sda".to_string(),
        // Credential exfiltration
        r"curl\s+.*[-d].*password".to_string(),
        r"wget\s+.*password".to_string(),
        // Dangerous redirects
        r">\s*/dev/sd[a-z]".to_string(),
        r">\s*/etc/passwd".to_string(),
        r">\s*/etc/shadow".to_string(),
        // Disable firewall entirely
        r"(ufw|iptables)\s+(disable|--flush|-F)".to_string(),
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

        // ── 3. Inject named host entries as ${HOST_<NICK>_IP} etc. ────────────
        // This runs silently — missing hosts.yaml is not an error.
        if let Ok(host_map) = hosts::load() {
            hosts::inject_into_secrets(&host_map, &mut cfg.secrets);
        }

        // ── 4. Resolve any "pass:..." references via GNU pass ─────────────────
        // Optional: requires `pkg install gnupg pass` and a GPG key.
        // Values that can't be resolved are left as-is with a stderr warning.
        pass_store::resolve_all(&mut cfg.secrets);

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

    /// Load the contents of the configured `context_file`, if any.
    /// Returns `None` when no file is configured or the file cannot be read.
    /// Tildes in the path are expanded to `$HOME`.
    pub fn load_context_file(&self) -> Option<String> {
        let raw_path = self.context_file.as_deref()?;
        if raw_path.is_empty() {
            return None;
        }
        let expanded = if raw_path.starts_with('~') {
            let home = std::env::var("HOME").unwrap_or_default();
            raw_path.replacen('~', &home, 1)
        } else {
            raw_path.to_string()
        };
        match std::fs::read_to_string(&expanded) {
            Ok(content) if !content.trim().is_empty() => Some(content),
            Ok(_) => {
                eprintln!("Warning: context_file {:?} is empty, skipping", expanded);
                None
            }
            Err(e) => {
                eprintln!("Warning: could not read context_file {:?}: {}", expanded, e);
                None
            }
        }
    }

    /// Check if a Telegram username is in the authorized_users list.
    /// Comparison is case-insensitive because Telegram usernames are case-insensitive
    /// (the API may return them in a different case than the user configured).
    pub fn is_authorized(&self, user: &str) -> bool {
        let lower = user.to_lowercase();
        self.authorized_users.iter().any(|u| u.to_lowercase() == lower)
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
