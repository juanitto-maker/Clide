// ============================================
// bot.rs - Signal Bot Logic (CORRECTED)
// ============================================

use anyhow::{Context, Result};
use serde::Deserialize;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::Mutex;
use tracing::info;
use std::process::Stdio;

use crate::config::Config;
use crate::database::Database;
use crate::executor::Executor;
use crate::gemini::GeminiClient;
use crate::memory::Memory;
use crate::skills::SkillManager;
use crate::workflow::WorkflowExecutor;

pub struct Bot {
    config: Config,
    executor: Executor,
    gemini: GeminiClient,
    memory: Arc<Mutex<Memory>>,
    #[allow(dead_code)]
    skills: SkillManager,
    #[allow(dead_code)]
    workflows: WorkflowExecutor,
}

#[derive(Debug, Deserialize)]
struct SignalMessage {
    envelope: Envelope,
}

#[derive(Debug, Deserialize)]
struct Envelope {
    source: Option<String>,
    #[serde(rename = "sourceNumber")]
    source_number: Option<String>,
    message: Option<MessageData>,
}

#[derive(Debug, Deserialize)]
struct MessageData {
    message: String,
}

impl Bot {
    pub async fn new(config: Config) -> Result<Self> {
        let db_path = dirs::home_dir()
            .unwrap_or_default()
            .join(".clide/memory.db");
        let db = Database::new(db_path)?;
        
        let executor = Executor::new(config.clone());
        let gemini = GeminiClient::new(
            config.gemini_api_key.clone(),
            config.get_model(), // Uses gemini-2.5-flash from config
            0.7,
            2048,
            "You are Clide, a terminal assistant...".to_string(),
        );

        let skill_path = dirs::home_dir()
            .unwrap_or_default()
            .join(".clide/skills");
        let skills = SkillManager::new(skill_path)?;
        let workflows = WorkflowExecutor::new(executor.clone());
        let memory = Arc::new(Mutex::new(Memory::new(db)));

        Ok(Self {
            config,
            executor,
            gemini,
            memory,
            skills,
            workflows,
        })
    }

    pub async fn run(&self) -> Result<()> {
        info!("Starting Clide bot...");
        let mut process = Command::new("signal-cli")
            .arg("-a")
            .arg(&self.config.signal_number)
            .arg("jsonRpc")
            .stdout(Stdio::piped())
            .spawn()
            .context("Failed to start signal-cli. Is it installed?")?;

        let stdout = process.stdout.take().unwrap();
        let mut reader = BufReader::new(stdout).lines();

        while let Some(line) = reader.next_line().await? {
            if let Ok(msg) = serde_json::from_str::<SignalMessage>(&line) {
                if let Some(data) = msg.envelope.message {
                    let sender = msg.envelope.source_number
                        .or(msg.envelope.source)
                        .unwrap_or_default();
                    
                    if self.is_authorized(&sender) {
                        self.handle_message(&sender, &data.message).await?;
                    }
                }
            }
        }
        Ok(())
    }

    fn is_authorized(&self, sender: &str) -> bool {
        self.config.authorized_numbers.contains(&sender.to_string())
    }

    async fn handle_message(&self, sender: &str, text: &str) -> Result<()> {
        let text = text.trim();
        
        // Handle "status" command (sysinfo 0.31 fix)
        if text == "status" {
            let mut sys = sysinfo::System::new_all();
            sys.refresh_all();
            
            let load = sysinfo::System::load_average();
            let memory_used = sys.used_memory() / 1024 / 1024;
            let memory_total = sys.total_memory() / 1024 / 1024;
            let uptime = sysinfo::System::uptime(); 

            let status = format!(
                "📊 **System Status**\n\n\
                • Load: {:.2}, {:.2}, {:.2}\n\
                • Memory: {}/{} MB\n\
                • Uptime: {}h {}m",
                load.one, load.five, load.fifteen,
                memory_used, memory_total,
                uptime / 3600, (uptime % 3600) / 60
            );
            return self.send_message(sender, &status).await;
        }

        if text == "help" {
            return self.send_message(sender, &self.get_help()).await;
        }

        // Logic for AI command analysis and execution...
        let mut memory = self.memory.lock().await;
        let context = memory.get_context(sender, 5).await?;
        
        let analysis = self.gemini.analyze_command(text, &context).await?;
        
        if analysis.safe {
            if let Some(cmd) = analysis.suggestion {
                self.send_message(sender, &format!("⚙️ Executing: `{}`", cmd)).await?;
                let result = self.executor.execute(&cmd).await?;
                let output = result.output();
                self.send_message(sender, &format!("✅ Output:\n{}", output)).await?;
                memory.save_conversation(sender, text, &output, Some(&cmd), Some(result.exit_code), Some(result.duration_ms)).await?;
            }
        } else {
            self.send_message(sender, &format!("⚠️ Blocked: {}", analysis.explanation)).await?;
        }

        Ok(())
    }

    async fn send_message(&self, recipient: &str, message: &str) -> Result<()> {
        Command::new("signal-cli")
            .arg("-a").arg(&self.config.signal_number)
            .arg("send").arg("-m").arg(message).arg(recipient)
            .spawn()?.wait().await?;
        Ok(())
    }

    fn get_help(&self) -> String {
        "🚀 **Clide Help**\n- `status`: Show system info\n- `help`: This list\n- Send any command or request in plain English!".to_string()
    }
}
