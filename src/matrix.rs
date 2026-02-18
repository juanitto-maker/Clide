// ============================================
// matrix.rs - Matrix/Element Client
// Sends/receives Matrix messages via HTTP API
// ============================================

use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::{json, Value};
use std::time::{Duration, Instant};

pub struct MatrixMessage {
    pub sender: String,
    pub text: String,
}

pub struct MatrixClient {
    homeserver: String,
    access_token: String,
    room_id: String,
    since: Option<String>,
    client: Client,
    txn_counter: u64,
    initial_sync_done: bool,
    /// Actual bot user ID fetched from /whoami; used for self-response detection.
    bot_user_id: Option<String>,
}

impl MatrixClient {
    pub fn new(homeserver: String, access_token: String, room_id: String) -> Self {
        Self {
            homeserver: homeserver.trim_end_matches('/').to_string(),
            access_token,
            room_id,
            since: None,
            client: Client::new(),
            txn_counter: 0,
            initial_sync_done: false,
            bot_user_id: None,
        }
    }

    /// Percent-encode a string for use in a URL path segment
    fn url_encode(s: &str) -> String {
        let mut out = String::with_capacity(s.len() * 3);
        for b in s.bytes() {
            match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    out.push(b as char)
                }
                _ => out.push_str(&format!("%{:02X}", b)),
            }
        }
        out
    }

    /// Fetch the authenticated user's ID from /_matrix/client/v3/account/whoami
    /// and cache it for use in self-response detection.
    pub async fn fetch_bot_user_id(&mut self) -> Result<String> {
        let url = format!("{}/_matrix/client/v3/account/whoami", self.homeserver);
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await
            .context("Failed to call /whoami")?;
        let json: Value = resp.json().await.context("Invalid /whoami response")?;
        let user_id = json["user_id"]
            .as_str()
            .context("No user_id in /whoami response")?
            .to_string();
        self.bot_user_id = Some(user_id.clone());
        Ok(user_id)
    }

    /// Returns true if `sender` matches the bot's own Matrix user ID.
    /// Compares case-insensitively and falls back to `config_user` if
    /// /whoami was never called or failed.
    pub fn is_bot_sender(&self, sender: &str, config_user: &str) -> bool {
        let id = self
            .bot_user_id
            .as_deref()
            .unwrap_or(config_user);
        sender.trim().to_lowercase() == id.trim().to_lowercase()
    }

    /// Receive new messages from the Matrix room via /sync.
    /// The first call performs an initial sync to capture the current position
    /// without replaying history; subsequent calls return only new messages.
    pub async fn receive_messages(&mut self) -> Result<Vec<MatrixMessage>> {
        let mut url = format!(
            "{}/_matrix/client/v3/sync?timeout=5000",
            self.homeserver
        );
        if let Some(since) = &self.since {
            url = format!("{}&since={}", url, since);
        }

        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await
            .context("Failed to sync with Matrix server")?;

        let json: Value = resp.json().await.context("Invalid sync response from Matrix")?;

        let was_initial = !self.initial_sync_done;

        if let Some(next_batch) = json["next_batch"].as_str() {
            self.since = Some(next_batch.to_string());
            self.initial_sync_done = true;
        }

        // Skip message processing on the first sync to avoid re-delivering history
        if was_initial {
            return Ok(Vec::new());
        }

        let mut messages = Vec::new();
        if let Some(room) = json["rooms"]["join"].get(&self.room_id) {
            if let Some(events) = room["timeline"]["events"].as_array() {
                for event in events {
                    if event["type"].as_str() != Some("m.room.message") {
                        continue;
                    }
                    if event["content"]["msgtype"].as_str() != Some("m.text") {
                        continue;
                    }
                    if let (Some(sender), Some(text)) = (
                        event["sender"].as_str(),
                        event["content"]["body"].as_str(),
                    ) {
                        // Skip messages sent by the bot itself to prevent self-response loops.
                        // Uses the user ID cached from /whoami (set via fetch_bot_user_id).
                        if let Some(ref bot_id) = self.bot_user_id {
                            if sender.trim().to_lowercase() == bot_id.trim().to_lowercase() {
                                continue;
                            }
                        }
                        let text = text.trim();
                        if !text.is_empty() {
                            messages.push(MatrixMessage {
                                sender: sender.to_string(),
                                text: text.to_string(),
                            });
                        }
                    }
                }
            }
        }

        Ok(messages)
    }

    /// Send a text message to the configured Matrix room
    pub async fn send_message(&mut self, message: &str) -> Result<()> {
        self.txn_counter += 1;
        let txn_id = format!(
            "clide-{}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
            self.txn_counter
        );

        let url = format!(
            "{}/_matrix/client/v3/rooms/{}/send/m.room.message/{}",
            self.homeserver,
            Self::url_encode(&self.room_id),
            txn_id
        );

        let body = json!({
            "msgtype": "m.text",
            "body": message
        });

        let resp = self
            .client
            .put(&url)
            .bearer_auth(&self.access_token)
            .json(&body)
            .send()
            .await
            .context("Failed to send Matrix message")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body_text = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Matrix send_message failed {}: {}",
                status,
                body_text
            ));
        }

        Ok(())
    }

    /// Poll for a reply from a specific sender within timeout_secs
    pub async fn wait_for_reply(&mut self, sender: &str, timeout_secs: u64) -> Result<String> {
        let start = Instant::now();
        let timeout = Duration::from_secs(timeout_secs);

        while start.elapsed() < timeout {
            let msgs = self.receive_messages().await?;
            for msg in msgs {
                if msg.sender == sender {
                    return Ok(msg.text);
                }
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }

        Ok(String::new())
    }
}
