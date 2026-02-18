// ============================================
// bot.rs - Signal Bot Core Loop
// ============================================

use anyhow::Result;
use log::{info, warn};
use rusqlite::Connection;

use crate::config::Config;
use crate::gemini::GeminiClient;
use crate::signal::SignalClient;

/// Main bot structure (exported as Bot from lib.rs)
pub struct Bot {
    config: Config,
    gemini: GeminiClient,
    signal: SignalClient,
    _db: Connection,
}

impl Bot {
    /// Initialize bot from config
    pub fn new(config: Config) -> Result<Self> {
        let db_path = Self::db_path();
        info!("Opening database: {}", db_path);

        // Ensure ~/.clide directory exists
        if let Some(parent) = std::path::Path::new(&db_path).parent() {
            std::fs::create_dir_all(parent)?;
        }

        let db = Connection::open(&db_path)?;

        let gemini = GeminiClient::new(
            config.gemini_api_key.clone(),
            config.get_model().to_string(),
            0.7,
            2048,
            "You are Clide, a helpful AI assistant running inside Signal messenger. \
             Be concise and direct. When asked to run shell commands, describe what \
             you would do rather than executing blindly.".to_string(),
        );

        let signal = SignalClient::new(config.signal_number.clone());

        Ok(Self {
            config,
            gemini,
            signal,
            _db: db,
        })
    }

    /// Start the bot loop - polls Signal and replies via Gemini
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting Clide bot...");
        println!("Bot running. Send a message via Signal to {}. Ctrl+C to stop.", self.config.signal_number);

        loop {
            match self.signal.receive_messages() {
                Ok(messages) => {
                    for msg in messages {
                        if let Err(e) = self.handle_message(msg.sender, msg.text).await {
                            eprintln!("Error handling message: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error receiving messages: {}", e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
            }
        }
    }

    /// Handle a single incoming Signal message
    async fn handle_message(&mut self, sender: String, text: String) -> Result<()> {
        info!("Message from {}: {}", sender, text);

        // Authorization check (skip if no authorized numbers configured)
        if !self.config.authorized_numbers.is_empty() && !self.config.is_authorized(&sender) {
            warn!("Unauthorized sender: {}", sender);
            return Ok(());
        }

        // Optional confirmation gate
        if self.config.require_confirmation {
            if !self.confirm_execution(&sender, &text)? {
                return Ok(());
            }
        }

        info!("Sending prompt to Gemini...");
        let response = self.gemini.generate(&text).await?;

        self.signal.send_message(&sender, &response)?;
        info!("Replied to {}", sender);

        Ok(())
    }

    /// Ask sender for YES/NO confirmation before proceeding
    fn confirm_execution(&self, sender: &str, text: &str) -> Result<bool> {
        let confirm_msg = format!(
            "Confirm execution?\n\n{}\n\nReply with YES to proceed.",
            text
        );

        self.signal.send_message(sender, &confirm_msg)?;

        let reply = self.signal.wait_for_reply(sender, self.config.confirmation_timeout)?;

        Ok(reply.trim().eq_ignore_ascii_case("yes"))
    }

    fn db_path() -> String {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        format!("{}/.clide/memory.db", home)
    }
}
