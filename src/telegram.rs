// ============================================
// telegram.rs - Telegram Bot API HTTP Client
// ============================================

use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;

pub struct TelegramClient {
    token: String,
    client: Client,
    /// Next offset to pass to getUpdates (prevents re-delivering old messages)
    offset: i64,
}

/// A single incoming Telegram message, simplified for the bot loop
#[derive(Debug, Clone)]
pub struct TelegramMessage {
    pub chat_id: i64,
    pub sender: String,
    pub text: String,
}

// ── Telegram API response types ────────────────────────────────────────────────

#[derive(Deserialize)]
struct GetUpdatesResponse {
    ok: bool,
    result: Vec<Update>,
}

#[derive(Deserialize)]
struct Update {
    update_id: i64,
    message: Option<Message>,
}

#[derive(Deserialize)]
struct Message {
    chat: Chat,
    from: Option<User>,
    text: Option<String>,
}

#[derive(Deserialize)]
struct Chat {
    id: i64,
}

#[derive(Deserialize)]
struct User {
    username: Option<String>,
    first_name: String,
}

// ── Client implementation ──────────────────────────────────────────────────────

impl TelegramClient {
    pub fn new(token: String) -> Self {
        Self {
            token,
            client: Client::new(),
            offset: 0,
        }
    }

    /// Long-poll Telegram for new messages (timeout=30s).
    /// Updates the internal offset so each message is delivered exactly once.
    pub async fn get_updates(&mut self) -> Result<Vec<TelegramMessage>> {
        let url = format!(
            "https://api.telegram.org/bot{}/getUpdates?offset={}&timeout=30",
            self.token, self.offset
        );

        let resp: GetUpdatesResponse = self.client.get(&url).send().await?.json().await?;

        if !resp.ok {
            return Ok(vec![]);
        }

        let mut messages = Vec::new();
        for update in resp.result {
            // Advance offset past this update so it isn't re-delivered
            self.offset = update.update_id + 1;

            if let Some(msg) = update.message {
                if let Some(text) = msg.text {
                    let sender = msg
                        .from
                        .map(|u| u.username.unwrap_or(u.first_name))
                        .unwrap_or_else(|| "unknown".to_string());
                    messages.push(TelegramMessage {
                        chat_id: msg.chat.id,
                        sender,
                        text,
                    });
                }
            }
        }
        Ok(messages)
    }

    /// Send a text reply to a chat.
    pub async fn send_message(&self, chat_id: i64, text: &str) -> Result<()> {
        let url = format!("https://api.telegram.org/bot{}/sendMessage", self.token);
        self.client
            .post(&url)
            .json(&serde_json::json!({
                "chat_id": chat_id,
                "text": text,
            }))
            .send()
            .await?;
        Ok(())
    }
}
