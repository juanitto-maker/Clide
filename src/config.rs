use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub gemini_api_key: String,
    pub gemini_model: String,
    pub signal_number: String,

    pub require_confirmation: bool,
    pub confirmation_timeout: u64,

    pub authorized_numbers: Vec<String>,

    #[serde(default)]
    pub logging: LoggingConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct LoggingConfig {
    pub level: String,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let path = Self::path();
        let content = std::fs::read_to_string(path)?;
        let cfg: Config = toml::from_str(&content)?;
        Ok(cfg)
    }

    pub fn path() -> PathBuf {
        let home = std::env::var("HOME").unwrap();
        PathBuf::from(home).join(".clide/config.toml")
    }

    pub fn get_model(&self) -> &str {
        &self.gemini_model
    }

    pub fn is_authorized(&self, number: &str) -> bool {
        self.authorized_numbers.contains(&number.to_string())
    }
}
