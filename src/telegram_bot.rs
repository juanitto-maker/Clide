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
        // and returns the full accumulated log when done.
        let tg = self.telegram.clone();
        let updater: tokio::task::JoinHandle<String> = tokio::spawn(async move {
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
            log // return full log to caller
        });

        // Run the agentic loop (drops tx when done, closing the channel)
        let result = self.agent.run(&text, &sender, Some(tx)).await;

        // Wait for the updater to flush its last edit and collect the log
        let commands_log = updater.await.unwrap_or_default();

        // Build the final HTML message: answer + optional spoiler with command log
        let final_text = match result {
            Ok(r) if !r.is_empty() => r,
            Ok(_) => "✅ Done.".to_string(),
            Err(e) => format!("❌ Error: {}", e),
        };

        let final_html = build_final_html(&final_text, &commands_log);

        // Edit the status message with the final HTML answer
        // If it's too large, fall back to split plain-text messages
        if final_html.len() <= TG_MAX_CHARS {
            if self
                .telegram
                .edit_message_html(chat_id, status_id, &final_html)
                .await
                .is_err()
            {
                // HTML edit failed (e.g. bad chars) — fall back to plain text
                let _ = self
                    .telegram
                    .edit_message(chat_id, status_id, &final_text)
                    .await;
            }
        } else {
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

/// Escape text for use inside Telegram HTML messages.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Build the final HTML message: answer prose followed by a spoiler block
/// containing the commands that were run. The user taps the spoiler to expand
/// it inline — no need to switch to Termux.
fn build_final_html(answer: &str, commands_log: &str) -> String {
    let escaped_answer = html_escape(answer);

    if commands_log.trim().is_empty() {
        return escaped_answer;
    }

    let escaped_log = html_escape(commands_log.trim());
    format!(
        "{}\n\n<tg-spoiler>🔍 Commands run:\n<pre>{}</pre></tg-spoiler>",
        escaped_answer, escaped_log
    )
}
