// ============================================
// telegram.rs - Telegram Bot API HTTP Client
// ============================================

use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;
use std::sync::{
    atomic::{AtomicI64, Ordering},
    Arc,
};

#[derive(Clone)]
pub struct TelegramClient {
    token: String,
    client: Client,
    /// Next offset to pass to getUpdates (prevents re-delivering old messages).
    /// Wrapped in Arc so clones share the same counter — critical for the
    /// stop-watcher task that runs concurrently with the main polling loop.
    offset: Arc<AtomicI64>,
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

/// Minimal wrapper around a Telegram API response that carries a result.message_id
#[derive(Deserialize)]
struct SentMessage {
    message_id: i64,
}

#[derive(Deserialize)]
struct SendResponse {
    #[allow(dead_code)]
    ok: bool,
    result: Option<SentMessage>,
}

// ── Client implementation ──────────────────────────────────────────────────────

impl TelegramClient {
    pub fn new(token: String) -> Self {
        Self {
            token,
            client: Client::new(),
            offset: Arc::new(AtomicI64::new(0)),
        }
    }

    /// Long-poll Telegram for new messages (timeout=30s).
    /// Updates the shared offset so each message is delivered exactly once.
    /// Takes `&self` because the offset is behind an Arc — clones share it.
    pub async fn get_updates(&self) -> Result<Vec<TelegramMessage>> {
        self.fetch_updates(30).await
    }

    /// Short-poll Telegram for new messages (timeout=5s).
    /// Used by the stop-watcher task so it can react quickly without blocking
    /// the 30-second long-poll window.
    pub async fn get_updates_short(&self) -> Result<Vec<TelegramMessage>> {
        self.fetch_updates(5).await
    }

    /// Inner helper: call getUpdates with a configurable timeout.
    async fn fetch_updates(&self, timeout_secs: u32) -> Result<Vec<TelegramMessage>> {
        let offset = self.offset.load(Ordering::SeqCst);
        let url = format!(
            "https://api.telegram.org/bot{}/getUpdates?offset={}&timeout={}",
            self.token, offset, timeout_secs
        );

        let resp: GetUpdatesResponse = self.client.get(&url).send().await?.json().await?;

        if !resp.ok {
            return Ok(vec![]);
        }

        let mut messages = Vec::new();
        for update in resp.result {
            // Advance the shared offset past this update so it isn't re-delivered.
            // fetch_and_update is not strictly atomic but this function is never
            // called concurrently from two tasks on the same client.
            self.offset.store(update.update_id + 1, Ordering::SeqCst);

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

    /// Send a text reply to a chat. Returns the new message's message_id.
    pub async fn send_message(&self, chat_id: i64, text: &str) -> Result<i64> {
        let url = format!("https://api.telegram.org/bot{}/sendMessage", self.token);
        let resp: SendResponse = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "chat_id": chat_id,
                "text": text,
            }))
            .send()
            .await?
            .json()
            .await?;

        Ok(resp.result.map(|m| m.message_id).unwrap_or(0))
    }

    /// Edit an existing message in a chat (best-effort; ignores "not modified" errors).
    pub async fn edit_message(&self, chat_id: i64, message_id: i64, text: &str) -> Result<()> {
        let url = format!(
            "https://api.telegram.org/bot{}/editMessageText",
            self.token
        );
        self.client
            .post(&url)
            .json(&serde_json::json!({
                "chat_id": chat_id,
                "message_id": message_id,
                "text": text,
            }))
            .send()
            .await?;
        Ok(())
    }

    /// Send an HTML-formatted message. Returns the new message's message_id.
    pub async fn send_message_html(&self, chat_id: i64, html: &str) -> Result<i64> {
        let url = format!("https://api.telegram.org/bot{}/sendMessage", self.token);
        let resp: SendResponse = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "chat_id": chat_id,
                "text": html,
                "parse_mode": "HTML",
            }))
            .send()
            .await?
            .json()
            .await?;
        Ok(resp.result.map(|m| m.message_id).unwrap_or(0))
    }

    /// Edit an existing message with HTML formatting (best-effort).
    pub async fn edit_message_html(
        &self,
        chat_id: i64,
        message_id: i64,
        html: &str,
    ) -> Result<()> {
        let url = format!(
            "https://api.telegram.org/bot{}/editMessageText",
            self.token
        );
        self.client
            .post(&url)
            .json(&serde_json::json!({
                "chat_id": chat_id,
                "message_id": message_id,
                "text": html,
                "parse_mode": "HTML",
            }))
            .send()
            .await?;
        Ok(())
    }
}
