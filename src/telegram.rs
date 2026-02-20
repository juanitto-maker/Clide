// ============================================
// telegram.rs - Telegram Bot API HTTP Client
// ============================================

use anyhow::Result;
use log::warn;
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

/// A file attached to an incoming Telegram message.
#[derive(Debug, Clone)]
pub struct AttachedFile {
    /// Original filename (may be synthesised, e.g. "photo.jpg" for photos).
    pub filename: String,
    /// MIME type when provided by Telegram.
    pub mime_type: Option<String>,
    /// Raw file bytes downloaded from Telegram's CDN.
    pub bytes: Vec<u8>,
}

/// A single incoming Telegram message, simplified for the bot loop
#[derive(Debug, Clone)]
pub struct TelegramMessage {
    pub chat_id: i64,
    pub sender: String,
    /// Text body of the message, or caption for media messages.
    pub text: String,
    /// Optional file attached to the message (document, photo, audio, video, voice).
    pub file: Option<AttachedFile>,
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
    /// Set for plain text messages.
    text: Option<String>,
    /// Set for media messages (photo, document, …) when the user adds a caption.
    caption: Option<String>,
    // ── Media attachment types ──────────────────────────────────────────────
    document: Option<Document>,
    /// Telegram sends photos as an array of sizes; we pick the largest one.
    photo: Option<Vec<PhotoSize>>,
    audio: Option<Document>,
    video: Option<Document>,
    voice: Option<Document>,
    video_note: Option<VideoNote>,
}

/// Shared shape for document / audio / video / voice attachments.
#[derive(Deserialize)]
struct Document {
    file_id: String,
    file_name: Option<String>,
    mime_type: Option<String>,
}

#[derive(Deserialize)]
struct PhotoSize {
    file_id: String,
}

#[derive(Deserialize)]
struct VideoNote {
    file_id: String,
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

/// Response from getFile — gives us the CDN path to download a file.
#[derive(Deserialize)]
struct GetFileResponse {
    ok: bool,
    result: Option<FileInfo>,
}

#[derive(Deserialize)]
struct FileInfo {
    file_path: Option<String>,
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
                // Use text for plain messages, caption for media messages.
                let text = msg.text.or(msg.caption).unwrap_or_default();

                let sender = msg
                    .from
                    .map(|u| u.username.unwrap_or(u.first_name))
                    .unwrap_or_else(|| "unknown".to_string());

                // Detect an attached file and try to download it.
                let file = self.extract_file(&msg.document, &msg.photo, &msg.audio, &msg.video, &msg.voice, &msg.video_note).await;

                // Only deliver the message if it has text, a caption, or a file.
                if !text.is_empty() || file.is_some() {
                    messages.push(TelegramMessage {
                        chat_id: msg.chat.id,
                        sender,
                        text,
                        file,
                    });
                }
            }
        }
        Ok(messages)
    }

    /// Inspect the message's media fields and download the first attachment found.
    /// Returns `None` if there is no attachment or if downloading fails.
    async fn extract_file(
        &self,
        document: &Option<Document>,
        photo: &Option<Vec<PhotoSize>>,
        audio: &Option<Document>,
        video: &Option<Document>,
        voice: &Option<Document>,
        video_note: &Option<VideoNote>,
    ) -> Option<AttachedFile> {
        // Determine (file_id, filename, mime_type) from whichever field is present.
        let (file_id, filename, mime_type): (&str, String, Option<String>) =
            if let Some(doc) = document {
                (
                    &doc.file_id,
                    doc.file_name.clone().unwrap_or_else(|| "document".to_string()),
                    doc.mime_type.clone(),
                )
            } else if let Some(photos) = photo {
                // Telegram sends multiple sizes; the last entry is the largest.
                let p = photos.last()?;
                (&p.file_id, "photo.jpg".to_string(), Some("image/jpeg".to_string()))
            } else if let Some(aud) = audio {
                (
                    &aud.file_id,
                    aud.file_name.clone().unwrap_or_else(|| "audio".to_string()),
                    aud.mime_type.clone(),
                )
            } else if let Some(vid) = video {
                (
                    &vid.file_id,
                    vid.file_name.clone().unwrap_or_else(|| "video".to_string()),
                    vid.mime_type.clone(),
                )
            } else if let Some(v) = voice {
                (
                    &v.file_id,
                    "voice.ogg".to_string(),
                    v.mime_type.clone().or_else(|| Some("audio/ogg".to_string())),
                )
            } else if let Some(vn) = video_note {
                (&vn.file_id, "video_note.mp4".to_string(), Some("video/mp4".to_string()))
            } else {
                return None;
            };

        // Resolve file_id → CDN path via getFile.
        let cdn_path = match self.get_file_path(file_id).await {
            Ok(p) => p,
            Err(e) => {
                warn!("getFile failed for file_id={}: {}", file_id, e);
                return None;
            }
        };

        // Download the raw bytes.
        let bytes = match self.download_file(&cdn_path).await {
            Ok(b) => b,
            Err(e) => {
                warn!("File download failed ({}): {}", cdn_path, e);
                return None;
            }
        };

        Some(AttachedFile { filename, mime_type, bytes })
    }

    /// Call Telegram's getFile API to resolve a file_id into a CDN path.
    async fn get_file_path(&self, file_id: &str) -> Result<String> {
        let url = format!(
            "https://api.telegram.org/bot{}/getFile?file_id={}",
            self.token, file_id
        );
        let resp: GetFileResponse = self.client.get(&url).send().await?.json().await?;
        if !resp.ok {
            return Err(anyhow::anyhow!("getFile returned ok=false for file_id={}", file_id));
        }
        resp.result
            .and_then(|f| f.file_path)
            .ok_or_else(|| anyhow::anyhow!("No file_path in getFile response"))
    }

    /// Download a file from Telegram's CDN using the path returned by getFile.
    async fn download_file(&self, file_path: &str) -> Result<Vec<u8>> {
        let url = format!(
            "https://api.telegram.org/file/bot{}/{}",
            self.token, file_path
        );
        let bytes = self.client.get(&url).send().await?.bytes().await?;
        Ok(bytes.to_vec())
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
