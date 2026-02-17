use anyhow::Result;
use log::{info, warn};
use rusqlite::Connection;

use crate::config::Config;
use crate::gemini::GeminiClient;
use crate::signal::SignalClient;

/// Main bot structure
pub struct ClideBot {
    config: Config,
    gemini: GeminiClient,
    signal: SignalClient,
    db: Connection,
}

impl ClideBot {
    /// Initialize bot
    pub fn new(config: Config) -> Result<Self> {
        info!("Opening database: {:?}", Self::db_path());

        let db = Connection::open(Self::db_path())?;

        let gemini = GeminiClient::new(
            config.gemini_api_key.clone(),
            config.get_model().to_string(),
        );

        let signal = SignalClient::new(config.signal_number.clone());

        Ok(Self {
            config,
            gemini,
            signal,
            db,
        })
    }

    /// Start the bot loop
    pub fn start(&mut self) -> Result<()> {
        info!("Starting Clide bot...");

        loop {
            let messages = self.signal.receive_messages()?;

            for msg in messages {
                self.handle_message(msg.sender, msg.text)?;
            }
        }
    }

    /// Handle a single incoming message
    fn handle_message(&mut self, sender: String, text: String) -> Result<()> {
        info!("Message from {}: {}", sender, text);

        // Authorization check
        if !self.config.is_authorized(&sender) {
            warn!("Unauthorized sender: {}", sender);
            return Ok(());
        }

        // Optional confirmation gate
        if self.config.require_confirmation {
            if !self.confirm_execution(&sender, &text)? {
                return Ok(());
            }
        }

        // Send prompt to Gemini
        info!("Sending prompt to Gemini...");
        let response = self.gemini.generate(&text)?;

        // Send response back via Signal
        self.signal.send_message(&sender, &response)?;

        Ok(())
    }

    /// Confirmation logic (simple yes/no)
    fn confirm_execution(&self, sender: &str, text: &str) -> Result<bool> {
        let confirm_msg = format!(
            "⚠️ Confirm execution?\n\n{}\n\nReply with YES to proceed.",
            text
        );

        self.signal.send_message(sender, &confirm_msg)?;

        let reply = self.signal.wait_for_reply(
            sender,
            self.config.confirmation_timeout,
        )?;

        Ok(reply.trim().eq_ignore_ascii_case("yes"))
    }

    /// Database path
    fn db_path() -> String {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        format!("{}/.clide/memory.db", home)
    }
}
