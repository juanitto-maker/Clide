// ============================================
// bot.rs - Signal Bot Logic (ENHANCED)
// ============================================
// Main bot with memory, skills, and workflow support

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

use crate::config::Config;
use crate::database::Database;
use crate::executor::{ExecutionResult, Executor};
use crate::gemini::GeminiClient;
use crate::memory::Memory;
use crate::skills::SkillManager;
use crate::workflow::WorkflowExecutor;

/// Signal bot with memory and skills
pub struct Bot {
    config: Config,
    executor: Executor,
    gemini: GeminiClient,
    memory: Arc<Mutex<Memory>>,
    skills: SkillManager,
    workflows: WorkflowExecutor,
}

/// Signal message received
#[derive(Debug, Deserialize)]
struct SignalMessage {
    envelope: Envelope,
}

#[derive(Debug, Deserialize)]
struct Envelope {
    #[serde(rename = "source")]
    source_number: Option<String>,
    #[serde(rename = "sourceNumber")]
    source_number_alt: Option<String>,
    #[serde(rename = "dataMessage")]
    data_message: Option<DataMessage>,
}

#[derive(Debug, Deserialize)]
struct DataMessage {
    message: Option<String>,
    timestamp: Option<u64>,
}

impl Bot {
    /// Create new bot instance with memory and skills
    pub fn new(config: Config) -> Result<Self> {
        let executor = Executor::new(config.clone());
        
        let gemini = GeminiClient::new(
            config.gemini_api_key.clone(),
            config.gemini.model.clone(),
            config.gemini.temperature,
            config.gemini.max_tokens,
            config.gemini.system_prompt.clone(),
        );

        // Initialize database
        let db_path = dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".clide")
            .join("clide.db");

        let db = Database::new(&db_path)?;
        let memory = Arc::new(Mutex::new(Memory::new(db)));

        // Initialize skills
        let skills_dir = dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".clide")
            .join("skills");

        let skills = SkillManager::new(skills_dir)?;

        // Initialize workflow executor
        let workflows = WorkflowExecutor::new(executor.clone());

        Ok(Self {
            config,
            executor,
            gemini,
            memory,
            skills,
            workflows,
        })
    }

    /// Start the bot (blocking)
    pub async fn start(&self) -> Result<()> {
        info!("🛫 Starting Clide bot...");
        info!("Signal number: {}", self.config.signal_number);
        info!("Skills loaded: {}", self.skills.list_skills().len());
        info!("Listening for messages...");

        loop {
            match self.receive_messages().await {
                Ok(_) => {}
                Err(e) => {
                    error!("Error receiving messages: {}", e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
            }
        }
    }

    /// Receive messages from Signal
    async fn receive_messages(&self) -> Result<()> {
        let mut child = Command::new("signal-cli")
            .arg("-a")
            .arg(&self.config.signal_number)
            .arg("receive")
            .arg("--json")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn signal-cli")?;

        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            while let Some(line) = lines.next_line().await? {
                if line.trim().is_empty() {
                    continue;
                }

                debug!("Received line: {}", line);

                match serde_json::from_str::<SignalMessage>(&line) {
                    Ok(msg) => {
                        if let Err(e) = self.handle_message(msg).await {
                            error!("Failed to handle message: {}", e);
                        }
                    }
                    Err(e) => {
                        debug!("Failed to parse message: {}", e);
                    }
                }
            }
        }

        child.wait().await?;
        Ok(())
    }

    /// Handle incoming message with memory
    async fn handle_message(&self, msg: SignalMessage) -> Result<()> {
        let sender = msg.envelope.source_number
            .or(msg.envelope.source_number_alt)
            .unwrap_or_else(|| "unknown".to_string());

        let text = msg
            .envelope
            .data_message
            .and_then(|dm| dm.message)
            .unwrap_or_default();

        if text.is_empty() {
            return Ok(());
        }

        info!("📨 Message from {}: {}", sender, text);

        if !self.is_authorized(&sender) {
            warn!("⛔ Unauthorized sender: {}", sender);
            self.send_message(&sender, "⛔ Unauthorized").await?;
            return Ok(());
        }

        // Process command with memory context
        let response = self.process_command(&sender, &text).await?;

        // Save to memory
        let mut memory = self.memory.lock().await;
        memory
            .save_conversation(&sender, &text, &response, None, None, None)
            .await?;

        // Send response
        self.send_message(&sender, &response).await?;

        Ok(())
    }

    /// Check if sender is authorized
    fn is_authorized(&self, sender: &str) -> bool {
        if sender == self.config.signal_number {
            return true;
        }

        if self.config.authorized_numbers.is_empty() {
            return true;
        }

        self.config.authorized_numbers.contains(&sender.to_string())
    }

    /// Process command with memory and skills
    async fn process_command(&self, user: &str, text: &str) -> Result<String> {
        let text = text.trim();

        // Built-in commands
        if text == "status" || text == "/status" {
            return self.get_status().await;
        }

        if text == "help" || text == "/help" {
            return Ok(self.get_help());
        }

        if text == "memory" || text == "/memory" {
            return self.show_memory(user).await;
        }

        if text == "skills" || text == "/skills" {
            return Ok(self.list_skills());
        }

        // Skill execution: /skill <name> param1=value1 param2=value2
        if text.starts_with("/skill ") || text.starts_with("skill ") {
            return self.execute_skill(text).await;
        }

        // Learn pattern: /learn <intent> = <command>
        if text.starts_with("/learn ") {
            return self.learn_pattern(user, &text[7..]).await;
        }

        // SSH command
        if text.starts_with("ssh ") {
            return self.handle_ssh_command(&text[4..]).await;
        }

        // Check for learned patterns
        let mut memory = self.memory.lock().await;
        if let Ok(Some(learned_cmd)) = memory.get_pattern(user, text).await {
            drop(memory); // Release lock
            info!("📚 Using learned pattern: {} -> {}", text, learned_cmd);
            return self.execute_command(user, &learned_cmd).await;
        }
        drop(memory); // Release lock

        // Direct command
        if text.starts_with('/') || text.contains("&&") || text.contains("|") {
            return self.execute_command(user, text).await;
        }

        // Natural language with context
        info!("🤖 Interpreting with AI + context: {}", text);
        
        let mut memory = self.memory.lock().await;
        let context = memory.build_ai_context(user).await?;
        drop(memory);

        let prompt = format!("{}\n\nUser request: {}", context, text);

        match self.gemini.suggest_command(&prompt).await {
            Ok(suggested_command) => {
                let suggested_command = suggested_command.trim();
                
                if self.config.require_confirmation {
                    Ok(format!(
                        "💡 Suggested:\n```\n{}\n```\n\n⚠️ Reply 'yes' to execute",
                        suggested_command
                    ))
                } else {
                    self.execute_command(user, suggested_command).await
                }
            }
            Err(e) => {
                error!("Gemini error: {}", e);
                Ok(format!("❌ Failed to interpret: {}", e))
            }
        }
    }

    /// Execute shell command with memory
    async fn execute_command(&self, user: &str, command: &str) -> Result<String> {
        info!("⚡ Executing: {}", command);

        match self.executor.execute(command).await {
            Ok(result) => {
                // Save to memory
                let mut memory = self.memory.lock().await;
                memory
                    .save_conversation(
                        user,
                        command,
                        &result.output(),
                        Some(command),
                        Some(result.exit_code),
                        Some(result.duration_ms),
                    )
                    .await?;

                let output = result.output();
                let truncated = if output.len() > 2000 {
                    format!("{}... (truncated)", &output[..2000])
                } else {
                    output
                };

                if result.success() {
                    Ok(format!(
                        "✅ Completed ({} ms)\n```\n{}\n```",
                        result.duration_ms, truncated
                    ))
                } else {
                    Ok(format!(
                        "⚠️ Failed (exit: {})\n```\n{}\n```",
                        result.exit_code, truncated
                    ))
                }
            }
            Err(e) => {
                error!("Execution error: {}", e);
                Ok(format!("❌ Error: {}", e))
            }
        }
    }

    /// Execute a skill
    async fn execute_skill(&self, text: &str) -> Result<String> {
        let parts: Vec<&str> = text.split_whitespace().collect();
        if parts.len() < 2 {
            return Ok("Usage: /skill <name> param1=value1 param2=value2".to_string());
        }

        let skill_name = parts[1];
        
        // Parse parameters
        let mut params = HashMap::new();
        for part in &parts[2..] {
            if let Some((key, value)) = part.split_once('=') {
                params.insert(key.to_string(), value.to_string());
            }
        }

        info!("🎯 Executing skill: {} with params: {:?}", skill_name, params);

        match self.skills.execute_skill(skill_name, params, &self.executor).await {
            Ok(result) => {
                let output = result
                    .outputs
                    .iter()
                    .map(|r| r.output())
                    .collect::<Vec<_>>()
                    .join("\n---\n");

                let truncated = if output.len() > 2000 {
                    format!("{}... (truncated)", &output[..2000])
                } else {
                    output
                };

                if result.success {
                    Ok(format!(
                        "✅ Skill completed ({} ms)\n```\n{}\n```",
                        result.duration_ms, truncated
                    ))
                } else {
                    Ok(format!("⚠️ Skill failed\n```\n{}\n```", truncated))
                }
            }
            Err(e) => Ok(format!("❌ Skill error: {}", e)),
        }
    }

    /// Learn a pattern
    async fn learn_pattern(&self, user: &str, text: &str) -> Result<String> {
        if let Some((intent, command)) = text.split_once('=') {
            let intent = intent.trim();
            let command = command.trim();

            let mut memory = self.memory.lock().await;
            memory.learn_pattern(user, intent, command).await?;

            Ok(format!("📚 Learned: \"{}\" → {}", intent, command))
        } else {
            Ok("Usage: /learn <intent> = <command>".to_string())
        }
    }

    /// Show memory stats
    async fn show_memory(&self, user: &str) -> Result<String> {
        let memory = self.memory.lock().await;
        let stats = memory.get_stats(Some(user)).await?;

        Ok(format!(
            "💾 **Memory Stats**\n\
            • Conversations: {}\n\
            • Successful commands: {}\n\
            • Cached items: {}",
            stats.total_conversations, stats.successful_commands, stats.cached_items
        ))
    }

    /// List available skills
    fn list_skills(&self) -> String {
        let skills = self.skills.list_skills();

        if skills.is_empty() {
            return "No skills installed. Add skills to ~/.clide/skills/".to_string();
        }

        let mut output = format!("📦 **Available Skills** ({})\n\n", skills.len());

        for skill in skills {
            output.push_str(&format!(
                "• **{}** v{}\n  {}\n",
                skill.name, skill.version, skill.description
            ));
        }

        output.push_str("\nUsage: /skill <name> [params]");
        output
    }

    /// Handle SSH command
    async fn handle_ssh_command(&self, _command: &str) -> Result<String> {
        Ok("SSH functionality coming soon!".to_string())
    }

    /// Get system status
    async fn get_status(&self) -> Result<String> {
        use sysinfo::{System, SystemExt};

        let mut sys = System::new_all();
        sys.refresh_all();

        let memory = self.memory.lock().await;
        let stats = memory.get_stats(None).await?;

        let uptime = sys.uptime();
        let total_mem = sys.total_memory();
        let used_mem = sys.used_memory();

        Ok(format!(
            "🖥️ **System Status**\n\
            • Uptime: {}h {}m\n\
            • CPU cores: {}\n\
            • Memory: {:.1} GB / {:.1} GB ({:.1}%)\n\
            • Conversations: {}\n\
            • Skills: {}\n\
            • Bot: Running ✅",
            uptime / 3600,
            (uptime % 3600) / 60,
            sys.cpus().len(),
            used_mem as f64 / 1024.0 / 1024.0 / 1024.0,
            total_mem as f64 / 1024.0 / 1024.0 / 1024.0,
            (used_mem as f64 / total_mem as f64) * 100.0,
            stats.total_conversations,
            self.skills.list_skills().len()
        ))
    }

    /// Get help text
    fn get_help(&self) -> String {
        "🛫 **Clide - AI Command Assistant**\n\n\
        **Commands:**\n\
        • `status` - System status\n\
        • `help` - This help\n\
        • `memory` - Memory stats\n\
        • `skills` - List skills\n\
        • `/skill <name>` - Run skill\n\
        • `/learn <intent> = <command>` - Teach pattern\n\n\
        **Features:**\n\
        • Send commands directly\n\
        • Or use natural language\n\
        • I remember context!\n\
        • Skills for automation\n\n\
        **Examples:**\n\
        • `ls -la`\n\
        • \"show disk usage\"\n\
        • `/skill system_monitoring`\n\
        • `/learn check disk = df -h`"
            .to_string()
    }

    /// Send message via Signal
    async fn send_message(&self, recipient: &str, message: &str) -> Result<()> {
        debug!("Sending to {}: {}", recipient, message);

        let output = Command::new("signal-cli")
            .arg("-a")
            .arg(&self.config.signal_number)
            .arg("send")
            .arg("-m")
            .arg(message)
            .arg(recipient)
            .output()
            .await
            .context("Failed to send message")?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to send message: {}", error);
        }

        Ok(())
    }
}