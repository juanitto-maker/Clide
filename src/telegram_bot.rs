// ============================================
// telegram_bot.rs - Telegram Bot Core Loop
// ============================================

use anyhow::Result;
use log::{error, info, warn};
use std::sync::atomic::Ordering;
use tokio::fs;
use tokio::sync::mpsc;

use crate::agent::Agent;
use crate::config::Config;
use crate::telegram::TelegramClient;

/// Directory where files uploaded by Telegram users are stored.
const UPLOAD_DIR: &str = "/tmp/clide_uploads";

/// Directory scanned after every agent task — any files here are sent back
/// to the user as downloadable Telegram documents.
const EXPORT_DIR: &str = "/tmp/clide_exports";

/// Telegram messages are capped at 4096 chars; leave some headroom.
const TG_MAX_CHARS: usize = 3900;

/// File where the Telegram update offset is persisted across restarts.
const OFFSET_FILE: &str = "/tmp/clide_tg_offset";

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

        // ── Step 1: Validate the token with getMe ────────────────────────────
        // This fails fast with a clear error instead of silently polling forever
        // with a bad token.
        print!("Connecting to Telegram... ");
        match self.telegram.get_me().await {
            Ok(username) => {
                println!("✅ Connected as @{}", username);
                println!("  → Open Telegram and send a message to @{}", username);
            }
            Err(e) => {
                println!("❌ FAILED");
                eprintln!();
                eprintln!("ERROR: Could not connect to Telegram API: {}", e);
                eprintln!();
                eprintln!("Most likely causes:");
                eprintln!("  1. TELEGRAM_BOT_TOKEN is wrong or missing");
                eprintln!("     Check: ~/.clide/config.yaml  (telegram_bot_token: ...)");
                eprintln!("         or ~/.clide/secrets.yaml (TELEGRAM_BOT_TOKEN: ...)");
                eprintln!("  2. No internet connection");
                eprintln!("  3. Token was revoked — create a new one via @BotFather");
                return Err(e);
            }
        }

        // ── Step 2: Show config summary ──────────────────────────────────────
        if self.config.authorized_users.is_empty() {
            println!("  ⚠️  authorized_users is EMPTY — nobody can send commands.");
            println!("     Add your Telegram username to ~/.clide/config.yaml:");
            println!("     authorized_users:");
            println!("       - \"your_username\"");
        } else {
            println!(
                "  Authorized users: {}",
                self.config.authorized_users.join(", ")
            );
        }
        println!("  Gemini model: {}", self.config.gemini_model);
        println!();
        println!("Send /stop in the chat to abort a running task.");
        println!("Send /ping to confirm the bot sees your messages.");
        println!("Press Ctrl+C here to shut the bot down.");
        println!();

        // ── Step 3: Restore persisted offset ────────────────────────────────
        // Prevents re-delivering already-processed messages after a restart.
        self.telegram.load_offset(OFFSET_FILE);

        // ── Step 4: Clear any active webhook ────────────────────────────────
        // A leftover webhook causes Telegram to return 409 Conflict on every
        // getUpdates call, making the bot appear completely unresponsive.
        print!("Clearing any active webhook... ");
        match self.telegram.delete_webhook().await {
            Ok(()) => println!("done."),
            Err(e) => println!("skipped ({}).", e),
        }

        println!("Polling for messages (long-poll 30s)…");
        println!();

        loop {
            // Hot-reload config on every poll cycle so edits to config.yaml
            // (e.g. adding authorized_users) take effect immediately without
            // needing to restart the bot.
            if let Ok(refreshed) = Config::load() {
                self.config = refreshed;
            }

            match self.telegram.get_updates().await {
                Ok(messages) => {
                    // Persist the offset after every successful poll so a restart
                    // doesn't cause previously-seen messages to be re-delivered.
                    self.telegram.save_offset(OFFSET_FILE);

                    for msg in messages {
                        // ── Built-in commands ────────────────────────────────

                        // /ping or /start — health-check, useful to confirm the
                        // bot is alive without running a full agent task.
                        if msg.text.trim().eq_ignore_ascii_case("/ping")
                            || msg.text.trim().eq_ignore_ascii_case("/start")
                        {
                            let _ = self
                                .telegram
                                .send_message(
                                    msg.chat_id,
                                    "🟢 Clide is online and ready!\n\
                                     Send me a task to execute.\n\
                                     Use /stop to cancel a running task.",
                                )
                                .await;
                            continue;
                        }

                        // /stop command ─────────────────────────────────────
                        // When the bot is idle (here in the poll loop) there is
                        // no task running, so just inform the user.
                        // While a task IS running the stop-watcher spawned inside
                        // handle_message intercepts /stop and cancels the agent.
                        if msg.text.trim().eq_ignore_ascii_case("/stop") {
                            info!("Received /stop while idle — no task running.");
                            let _ = self
                                .telegram
                                .send_message(msg.chat_id, "No task is currently running.")
                                .await;
                            continue;
                        }

                        // /debug — show live config and bot status without
                        // running a full agent task.  Useful for diagnosing
                        // authorization or config problems quickly.
                        if msg.text.trim().eq_ignore_ascii_case("/debug") {
                            let auth_status = if self.config.authorized_users.is_empty() {
                                "⚠️ authorized_users is EMPTY — add your username to config.yaml".to_string()
                            } else if self.config.is_authorized(&msg.sender) {
                                format!("✅ @{} is authorized", msg.sender)
                            } else {
                                format!(
                                    "🚫 @{} is NOT in authorized_users.\n\
                                     Add it to ~/.clide/config.yaml:\n\
                                     authorized_users:\n  - \"{}\"",
                                    msg.sender, msg.sender
                                )
                            };
                            let reply = format!(
                                "🔍 Clide Debug\n\
                                 Version: {}\n\
                                 Platform: {}\n\
                                 Model: {}\n\
                                 Auth: {}\n\
                                 Sender username: @{}\n\
                                 Gemini key set: {}",
                                crate::VERSION,
                                self.config.platform,
                                self.config.gemini_model,
                                auth_status,
                                msg.sender,
                                !self.config.gemini_api_key.is_empty(),
                            );
                            let _ = self.telegram.send_message(msg.chat_id, &reply).await;
                            continue;
                        }

                        if let Err(e) = self
                            .handle_message(msg.chat_id, msg.sender, msg.text, msg.file)
                            .await
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

    async fn handle_message(
        &mut self,
        chat_id: i64,
        sender: String,
        text: String,
        file: Option<crate::telegram::AttachedFile>,
    ) -> Result<()> {
        info!("Telegram message from @{}: {}", sender, text);

        // Authorization — fail-closed: if the allowlist is empty, nobody is
        // allowed in.  An open bot controlling a real shell is unsafe by design.
        // Add your Telegram username(s) to authorized_users in config.yaml.
        if self.config.authorized_users.is_empty() {
            warn!(
                "Telegram message from @{} rejected: authorized_users is empty. \
                 Add your username to authorized_users in ~/.clide/config.yaml.",
                sender
            );
            let _ = self
                .telegram
                .send_message(
                    chat_id,
                    &format!(
                        "⚠️ Bot not configured yet.\n\
                         Add the following line to ~/.clide/config.yaml:\n\n\
                         authorized_users:\n  - \"{}\"\n\n\
                         Then restart the bot.",
                        sender
                    ),
                )
                .await;
            return Ok(());
        }
        if !self.config.is_authorized(&sender) {
            warn!("Unauthorized Telegram sender: @{}", sender);
            let _ = self
                .telegram
                .send_message(
                    chat_id,
                    &format!(
                        "🚫 Access denied.\n\
                         Your Telegram username \"{}\" is not in authorized_users.\n\
                         Add it to ~/.clide/config.yaml and restart the bot.",
                        sender
                    ),
                )
                .await;
            return Ok(());
        }

        // ── Prepare the export directory ──────────────────────────────────────
        // Clear previous task's exports so we don't re-send stale files.
        // The agent is told to save output files here; we forward them after the task.
        let _ = fs::remove_dir_all(EXPORT_DIR).await;
        if let Err(e) = fs::create_dir_all(EXPORT_DIR).await {
            warn!("Could not create export dir {}: {}", EXPORT_DIR, e);
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

        // ── Stop-watcher task ─────────────────────────────────────────────────
        // Cloning TelegramClient shares the Arc<AtomicI64> offset counter, so
        // messages consumed by the watcher won't be re-delivered to the main loop.
        let cancel = self.agent.cancel_token();
        let tg_watcher = self.telegram.clone();
        let cancel_watcher = cancel.clone();
        let stop_task: tokio::task::JoinHandle<()> = tokio::spawn(async move {
            loop {
                match tg_watcher.get_updates_short().await {
                    Ok(updates) => {
                        for update in updates {
                            if update.text.trim().eq_ignore_ascii_case("/stop") {
                                info!("Stop-watcher received /stop — cancelling agent.");
                                cancel_watcher.store(true, Ordering::SeqCst);
                                let _ = tg_watcher
                                    .send_message(
                                        update.chat_id,
                                        "🛑 Stopping current task...",
                                    )
                                    .await;
                                return; // watcher's job is done
                            }
                            // Non-stop messages received while the agent runs are
                            // acknowledged (offset advanced) and silently dropped.
                            // They will NOT be re-delivered to the main loop.
                            info!(
                                "Stop-watcher dropped message while agent busy: {}",
                                update.text
                            );
                        }
                    }
                    Err(e) => {
                        warn!("Stop-watcher poll error: {}", e);
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    }
                }
            }
        });

        // If a file was attached, save it to disk and prepend its path to the task.
        let task = build_task_with_file(text, file).await;

        // Run the agentic loop (drops tx when done, closing the updater channel)
        let result = self.agent.run(&task, &sender, Some(tx)).await;

        // Agent finished — abort the stop-watcher (it's no longer needed)
        stop_task.abort();

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

        // ── Send export files ─────────────────────────────────────────────────
        // Any files the agent saved to EXPORT_DIR are forwarded to the user as
        // downloadable document attachments.
        self.send_export_files(chat_id).await;

        info!("Agent task complete for @{}", sender);
        Ok(())
    }

    /// Scan the export directory and send every file as a Telegram document.
    ///
    /// The agent is instructed (via the system prompt) to save output files,
    /// reports, and logs to `/tmp/clide_exports/`.  This method picks them up
    /// and forwards them to the chat so the user can download them directly.
    async fn send_export_files(&self, chat_id: i64) {
        let mut read_dir = match fs::read_dir(EXPORT_DIR).await {
            Ok(rd) => rd,
            Err(_) => return, // Export dir doesn't exist — nothing to send.
        };

        while let Ok(Some(entry)) = read_dir.next_entry().await {
            // Skip directories and symlinks — only send plain files.
            let file_type = match entry.file_type().await {
                Ok(ft) => ft,
                Err(_) => continue,
            };
            if !file_type.is_file() {
                continue;
            }

            let path = entry.path();
            let path_str = match path.to_str() {
                Some(s) => s.to_string(),
                None => continue,
            };
            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("file")
                .to_string();

            info!("Sending export file to chat: {}", path_str);
            match self
                .telegram
                .send_document(chat_id, &path_str, Some(&format!("📎 {}", filename)))
                .await
            {
                Ok(msg_id) => info!("Sent '{}' as message {}", filename, msg_id),
                Err(e) => warn!("Failed to send export file '{}': {}", filename, e),
            }
        }
    }
}

/// Save an uploaded file to `UPLOAD_DIR` and return a task string that
/// includes the file's on-disk path so the agent can read / process it.
///
/// If no file is attached the original `text` is returned unchanged.
/// If the write fails we still deliver the text-only task and log a warning.
async fn build_task_with_file(
    text: String,
    file: Option<crate::telegram::AttachedFile>,
) -> String {
    let Some(attached) = file else {
        return text;
    };

    // Ensure the upload directory exists.
    if let Err(e) = fs::create_dir_all(UPLOAD_DIR).await {
        warn!("Could not create upload dir {}: {}", UPLOAD_DIR, e);
        return text;
    }

    // Sanitise the filename to avoid path traversal.
    let safe_name: String = attached
        .filename
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '_' })
        .collect();
    let safe_name = if safe_name.is_empty() { "upload".to_string() } else { safe_name };

    let file_path = format!("{}/{}", UPLOAD_DIR, safe_name);

    match fs::write(&file_path, &attached.bytes).await {
        Ok(()) => {
            info!("Saved uploaded file to {}", file_path);
            let user_instruction = if text.trim().is_empty() {
                "The user sent a file. Read and describe it using run_command.".to_string()
            } else {
                text
            };
            let mime_line = attached
                .mime_type
                .map(|m| format!("\nMIME type: {}", m))
                .unwrap_or_default();
            format!(
                "{}\n\nThe file is stored on the local filesystem at: {}\nFilename: {}{}\n\
                Use run_command to access it (e.g. `cat` for text, `file` to identify type, \
                `python3` to process it). Do NOT say you cannot see the file — use the shell.",
                user_instruction, file_path, safe_name, mime_line
            )
        }
        Err(e) => {
            warn!("Failed to write uploaded file to {}: {}", file_path, e);
            text
        }
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
