// ============================================
// bot.rs - Clide Bot Core (UPDATED)
// ============================================

use anyhow::{Context, Result};
use tracing::{error, info, warn};

use crate::config::Config;
use crate::gemini::{CommandAnalysis, GeminiClient};
use crate::memory::Memory;
use crate::skills::SkillManager;
use crate::executor::Executor;

pub struct Bot {
    pub config: Config,
    pub memory: Memory,
    pub gemini: GeminiClient,
    pub executor: Executor,
    pub skills: SkillManager,
}

impl Bot {
    pub fn new(
        config: Config,
        memory: Memory,
        executor: Executor,
        skills: SkillManager,
    ) -> Result<Self> {
        let model = config.get_model(); // now uses Config method
        let api_key = config.gemini_api_key.clone();

        let gemini = GeminiClient::new(
            api_key,
            model,
            0.0,     // temperature
            1024,    // max_tokens
            "Analyze user input and decide if safe to run as a shell command.".to_string(),
        );

        Ok(Self {
            config,
            memory,
            gemini,
            executor,
            skills,
        })
    }

    pub fn is_authorized(&self, sender: &str) -> bool {
        if self.config.authorized_numbers.is_empty() {
            return true; // empty = allow all
        }
        self.config.authorized_numbers.contains(&sender.to_string())
    }

    pub async fn analyze_command(&self, command: &str, context: &str) -> Result<CommandAnalysis> {
        self.gemini.analyze_command(command, context).await
    }

    pub async fn execute_skill(
        &self,
        skill_name: &str,
        params: &std::collections::HashMap<String, String>,
    ) -> Result<()> {
        let res = self.skills.execute_skill(skill_name, params, &self.executor).await?;
        if res.success {
            info!("Skill '{}' executed successfully.", skill_name);
        } else {
            warn!("Skill '{}' failed during execution.", skill_name);
        }
        Ok(())
    }
}

// --- Helper extension for Config ---
impl Config {
    pub fn get_model(&self) -> String {
        self.gemini_model.clone().unwrap_or_else(|| "gemini-2.5-flash".to_string())
    }
}
