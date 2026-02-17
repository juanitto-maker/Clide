// ============================================
// bot.rs - Clide Bot (CORRECTED)
// ============================================

use anyhow::Result;
use tracing::{debug, error, info, warn};

use crate::config::Config;
use crate::executor::Executor;
use crate::gemini::{CommandAnalysis, GeminiClient};

pub struct Bot {
    pub config: Config,
    pub executor: Executor,
    pub gemini: GeminiClient,
}

impl Bot {
    pub fn new(config: Config, executor: Executor) -> Self {
        let gemini_model = config.get_model().to_string();
        let gemini = GeminiClient::new(
            config.gemini_api_key.clone(),
            gemini_model,
            0.7,
            1024,
            "You are a safe assistant.".to_string(),
        );

        Self {
            config,
            executor,
            gemini,
        }
    }

    pub fn is_authorized(&self, sender: &str) -> bool {
        if self.config.authorized_numbers.is_empty() {
            true
        } else {
            self.config.authorized_numbers.contains(&sender.to_string())
        }
    }

    pub async fn analyze_command(&self, command: &str, context: &str) -> Result<CommandAnalysis> {
        self.gemini.analyze_command(command, context).await
    }

    pub async fn execute(&self, command: &str) -> Result<String> {
        if !self.config.allow_commands {
            return Err(anyhow::anyhow!("Command execution not allowed"));
        }
        let res = self.executor.execute(command).await?;
        Ok(res.output())
    }
}
