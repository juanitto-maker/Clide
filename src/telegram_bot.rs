// ============================================
// telegram_bot.rs - Telegram Bot Core Loop
// ============================================

use anyhow::Result;
use log::{error, info, warn};
use tokio::sync::mpsc;

use crate::agent::Agent;
use crate::config::Config;
use crate::telegram::TelegramClient;

/// Telegram messages are capped at 4096 chars; leave some headroom.
const TG_MAX_CHARS: usize = 3900;

pub struct TelegramBot {
    config: Config,
    agent: Agent,
    telegram: TelegramClient,
}

impl TelegramBot {
    pub fn new(config: Config) -> Result<Self> {
        let agent = Agent::new(&config);
        let telegram = TelegramClient::new(config.telegram_bot_token.clone());
        Ok(Self {
            config,
            agent,
            telegram,
        })
    }

    /// Start the polling loop.
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting Clide Telegram bot (agent mode)...");
        println!("Telegram bot running. Send a task to your bot. Ctrl+C to stop.");

        loop {
            match self.telegram.get_updates().await {
                Ok(messages) => {
                    for msg in messages {
                        if let Err(e) =
                            self.handle_message(msg.chat_id, msg.sender, msg.text).await
                        {
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

        // Authorization check
        if !self.config.authorized_users.is_empty() && !self.config.is_authorized(&sender) {
            warn!("Unauthorized Telegram sender: @{}", sender);
            return Ok(());
        }

        // Send initial "working" placeholder message
        let status_id = self.telegram.send_message(chat_id, "⚙️ Working...").await?;
        info!("Status message id: {}", status_id);

        // Progress channel (agent → updater task)
        let (tx, mut rx) = mpsc::channel::<String>(64);

        // Spawn a task that edits the status message with cumulative progress
        let tg = self.telegram.clone();
        let updater = tokio::spawn(async move {
            let mut log = String::new();
            while let Some(line) = rx.recv().await {
                log.push('\n');
                log.push_str(&line);

                // Keep within Telegram's limit
                let display = if log.len() > TG_MAX_CHARS {
                    format!("[…]\n{}", &log[log.len() - (TG_MAX_CHARS - 6)..])
                } else {
                    log.clone()
                };

                let _ = tg
                    .edit_message(chat_id, status_id, &format!("⚙️ Working…{}", display))
                    .await;
            }
        });

        // Run the agentic loop (drops tx when done, closing the channel)
        let result = self.agent.run(&text, Some(tx)).await;

        // Wait for the updater to flush its last edit
        let _ = updater.await;

        // Edit status message with the final answer
        let final_text = match result {
            Ok(r) if !r.is_empty() => r,
            Ok(_) => "✅ Done.".to_string(),
            Err(e) => format!("❌ Error: {}", e),
        };

        if final_text.len() <= TG_MAX_CHARS {
            self.telegram
                .edit_message(chat_id, status_id, &final_text)
                .await?;
        } else {
            // Too long — split into chunks
            self.telegram
                .edit_message(chat_id, status_id, "✅ Done. Full output below:")
                .await?;
            for chunk in final_text.as_bytes().chunks(TG_MAX_CHARS) {
                let chunk_str = String::from_utf8_lossy(chunk);
                self.telegram.send_message(chat_id, &chunk_str).await?;
            }
        }

        info!("Agent task complete for @{}", sender);
        Ok(())
    }
}
