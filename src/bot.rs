// ============================================
// bot.rs - Matrix Bot Core Loop
// ============================================

use anyhow::Result;
use log::{info, warn, error};
use rusqlite::Connection;

use crate::config::Config;
use crate::gemini::GeminiClient;
use crate::matrix::MatrixClient;

/// Main bot structure (exported as Bot from lib.rs)
pub struct Bot {
    config: Config,
    gemini: GeminiClient,
    matrix: MatrixClient,
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
            "You are Clide, a helpful AI assistant running inside an Element/Matrix room. \
             Be concise and direct. When asked to run shell commands, describe what \
             you would do rather than executing blindly.".to_string(),
        );

        let matrix = MatrixClient::new(
            config.matrix_homeserver.clone(),
            config.matrix_access_token.clone(),
            config.matrix_room_id.clone(),
        );

        Ok(Self {
            config,
            gemini,
            matrix,
            _db: db,
        })
    }

    /// Start the bot loop - polls Matrix room and replies via Gemini
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting Clide bot...");

        // Resolve the bot's actual Matrix user ID so the self-response guard
        // works correctly even if matrix_user in config has wrong casing or
        // a minor typo.
        match self.matrix.fetch_bot_user_id().await {
            Ok(id) => info!("Bot authenticated as: {}", id),
            Err(e) => error!(
                "Could not fetch bot user ID via /whoami ({}); \
                 falling back to config matrix_user for self-response detection. \
                 Ensure matrix_user in config matches exactly to avoid self-loops.",
                e
            ),
        }

        println!(
            "Bot running. Send a message in Matrix room {}. Ctrl+C to stop.",
            self.config.matrix_room_id
        );

        loop {
            match self.matrix.receive_messages().await {
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

    /// Handle a single incoming Matrix message
    async fn handle_message(&mut self, sender: String, text: String) -> Result<()> {
        info!("Message from {}: {}", sender, text);

        // Self-response guard: ignore messages sent by the bot itself to prevent
        // an infinite loop when running with a personal account access token.
        // Uses the user ID fetched from /whoami (case-insensitive) so a casing
        // mismatch in config cannot break this check.
        if self.matrix.is_bot_sender(&sender, &self.config.matrix_user) {
            info!("Ignoring own message from {} to prevent self-response loop.", sender);
            return Ok(());
        }

        // Authorization check (skip if no authorized users configured)
        if !self.config.authorized_users.is_empty() && !self.config.is_authorized(&sender) {
            warn!("Unauthorized sender: {}", sender);
            return Ok(());
        }

        // Optional confirmation gate
        if self.config.require_confirmation {
            if !self.confirm_execution(&sender, &text).await? {
                return Ok(());
            }
        }

        info!("Sending prompt to Gemini...");
        let response = self.gemini.generate(&text).await?;

        self.matrix.send_message(&response).await?;
        info!("Replied to {}", sender);

        Ok(())
    }

    /// Ask the room for YES/NO confirmation before proceeding
    async fn confirm_execution(&mut self, sender: &str, text: &str) -> Result<bool> {
        let confirm_msg = format!(
            "Confirm execution?\n\n{}\n\nReply with YES to proceed.",
            text
        );

        self.matrix.send_message(&confirm_msg).await?;

        let reply = self
            .matrix
            .wait_for_reply(sender, self.config.confirmation_timeout)
            .await?;

        Ok(reply.trim().eq_ignore_ascii_case("yes"))
    }

    fn db_path() -> String {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        format!("{}/.clide/memory.db", home)
    }
}
