// ============================================
// telegram_bot.rs - Telegram Bot Core Loop
// ============================================

use anyhow::Result;
use log::{error, info, warn};

use crate::config::Config;
use crate::gemini::GeminiClient;
use crate::telegram::TelegramClient;

pub struct TelegramBot {
    config: Config,
    gemini: GeminiClient,
    telegram: TelegramClient,
}

impl TelegramBot {
    pub fn new(config: Config) -> Result<Self> {
        let gemini = GeminiClient::new(
            config.gemini_api_key.clone(),
            config.get_model().to_string(),
            0.7,
            2048,
            "You are Clide, a helpful AI assistant running in a Telegram chat. \
             Be concise and direct. When asked to run shell commands, describe what \
             you would do rather than executing blindly."
                .to_string(),
        );

        let telegram = TelegramClient::new(config.telegram_bot_token.clone());

        Ok(Self {
            config,
            gemini,
            telegram,
        })
    }

    /// Start the bot loop - polls Telegram and replies via Gemini
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting Clide Telegram bot...");
        println!("Telegram bot running. Send a message to your bot. Ctrl+C to stop.");

        loop {
            match self.telegram.get_updates().await {
                Ok(messages) => {
                    for msg in messages {
                        if let Err(e) = self.handle_message(msg.chat_id, msg.sender, msg.text).await {
                            error!("Error handling Telegram message: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Telegram polling error: {}", e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
            }
        }
    }

    async fn handle_message(&mut self, chat_id: i64, sender: String, text: String) -> Result<()> {
        info!("Telegram message from @{}: {}", sender, text);

        // Authorization check (skip if no authorized users configured)
        if !self.config.authorized_users.is_empty() && !self.config.is_authorized(&sender) {
            warn!("Unauthorized Telegram sender: @{}", sender);
            return Ok(());
        }

        info!("Sending prompt to Gemini...");
        let response = self.gemini.generate(&text).await?;

        self.telegram.send_message(chat_id, &response).await?;
        info!("Replied to @{}", sender);

        Ok(())
    }
}
