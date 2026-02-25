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

/// Telegram messages are capped at 4096 chars; leave some headroom.
const TG_MAX_CHARS: usize = 3900;

/// Resolve a path under $HOME, falling back to /tmp if HOME is unset.
fn home_path(subdir: &str) -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    format!("{}/{}", home, subdir)
}

/// Directory where files uploaded by Telegram users are stored.
fn upload_dir() -> String { home_path("clide_uploads") }

/// Directory scanned after every agent task — any files here are sent back
/// to the user as downloadable Telegram documents.
fn export_dir() -> String { home_path("clide_exports") }

/// File where the Telegram update offset is persisted across restarts.
fn offset_file() -> String { home_path(".clide/tg_offset") }

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

        // ── Step 2b: Pre-create required directories ──────────────────────
        // Ensure export and temp dirs exist before the agent runs any task.
        // This avoids races where the agent tries to write before the dir
        // is created, and also ensures /tmp is never needed.
        let exp = export_dir();
        let safe_tmp = home_path(".clide/tmp");
        for dir in [&exp, &safe_tmp] {
            if let Err(e) = fs::create_dir_all(dir).await {
                warn!("Could not pre-create directory {}: {}", dir, e);
            }
        }
        // Set TMPDIR so child processes (and skills) use our safe temp dir
        // instead of /tmp, which may be read-only on this platform.
        std::env::set_var("TMPDIR", &safe_tmp);

        // ── Step 3: Restore persisted offset ────────────────────────────────
        // Prevents re-delivering already-processed messages after a restart.
        self.telegram.load_offset(&offset_file());

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

        let ctrl_c_fut = tokio::signal::ctrl_c();
        tokio::pin!(ctrl_c_fut);

        loop {
            // Hot-reload config on every poll cycle so edits to config.yaml
            // (e.g. adding authorized_users) take effect immediately without
            // needing to restart the bot.
            if let Ok(refreshed) = Config::load() {
                self.config = refreshed;
            }

            let updates = tokio::select! {
                biased;
                _ = &mut ctrl_c_fut => {
                    println!("\nShutting down Clide Telegram bot...");
                    break Ok(());
                }
                r = self.telegram.get_updates() => r,
            };

            match updates {
                Ok(messages) => {
                    // Persist the offset after every successful poll so a restart
                    // doesn't cause previously-seen messages to be re-delivered.
                    self.telegram.save_offset(&offset_file());

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
                            let exp_dir = export_dir();
                            let exp_dir_exists = std::path::Path::new(&exp_dir).exists();
                            let reply = format!(
                                "🔍 Clide Debug\n\
                                 Version: {}\n\
                                 Platform: {}\n\
                                 Model: {}\n\
                                 Auth: {}\n\
                                 Sender username: @{}\n\
                                 Gemini key set: {}\n\
                                 Export dir: {} ({})",
                                crate::VERSION,
                                self.config.platform,
                                self.config.gemini_model,
                                auth_status,
                                msg.sender,
                                !self.config.gemini_api_key.is_empty(),
                                exp_dir,
                                if exp_dir_exists { "exists" } else { "not created yet" },
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
        let exp_dir = export_dir();
        let _ = fs::remove_dir_all(&exp_dir).await;
        if let Err(e) = fs::create_dir_all(&exp_dir).await {
            warn!("Could not create export dir {}: {}", exp_dir, e);
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

        // If a file was attached, save it to disk and determine whether it can
        // be fed directly to Gemini as vision inline data (images / PDFs).
        let (task, vision) = build_task_with_file(text, file).await;

        // Run the agentic loop (drops tx when done, closing the updater channel)
        let result = self.agent.run(&task, &sender, Some(tx), vision).await;

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

    /// Scan the export directory **recursively** and send every file as a
    /// Telegram document.
    ///
    /// The agent is instructed (via the system prompt) to save output files,
    /// reports, and logs to `~/clide_exports/`.  This method picks them up
    /// and forwards them to the chat so the user can download them directly.
    /// Subdirectories are traversed so files placed in nested paths (e.g. by
    /// AIWB or the agent) are not missed.
    async fn send_export_files(&self, chat_id: i64) {
        let exp_dir = export_dir();
        info!("Scanning export dir (recursive) for files to deliver: {}", exp_dir);

        let mut files: Vec<std::path::PathBuf> = Vec::new();
        if let Err(e) = collect_files_recursive(&exp_dir, &mut files).await {
            info!("Export dir '{}' not readable ({}); nothing to send.", exp_dir, e);
            return;
        }

        let mut sent = 0u32;
        for path in &files {
            let path_str = match path.to_str() {
                Some(s) => s.to_string(),
                None => continue,
            };
            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("file")
                .to_string();

            info!("Sending export file to chat {}: {}", chat_id, path_str);
            match self
                .telegram
                .send_document(chat_id, &path_str, Some(&format!("📎 {}", filename)))
                .await
            {
                Ok(msg_id) => {
                    info!("Sent '{}' as message {}", filename, msg_id);
                    sent += 1;
                }
                Err(e) => error!("Failed to send export file '{}': {}", filename, e),
            }
        }
        if sent == 0 {
            info!("No files found in export dir '{}' to send.", exp_dir);
        }
    }
}

/// Maximum file size (in bytes) to send as Gemini vision inline data.
/// Larger images fall back to the shell-based approach (cat / python3 / etc.).
const VISION_MAX_BYTES: usize = 10 * 1024 * 1024; // 10 MB

/// Return true when the MIME type is one that Gemini can process visually.
fn is_vision_mime(mime: &str) -> bool {
    matches!(
        mime,
        "image/jpeg"
            | "image/jpg"
            | "image/png"
            | "image/gif"
            | "image/webp"
            | "image/heic"
            | "image/heif"
            | "image/bmp"
            | "application/pdf"
    )
}

/// Guess from the filename extension whether this is a vision-capable file.
fn is_vision_filename(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".png")
        || lower.ends_with(".gif")
        || lower.ends_with(".webp")
        || lower.ends_with(".heic")
        || lower.ends_with(".heif")
        || lower.ends_with(".bmp")
        || lower.ends_with(".pdf")
}

/// Infer a MIME type from the filename extension (fallback when Telegram
/// doesn't supply one, e.g. for photo messages sent as documents).
fn mime_from_filename(name: &str) -> &'static str {
    let lower_name = name.to_lowercase();
    if lower_name.ends_with(".jpg") || lower_name.ends_with(".jpeg") {
        "image/jpeg"
    } else if lower_name.ends_with(".png") {
        "image/png"
    } else if lower_name.ends_with(".gif") {
        "image/gif"
    } else if lower_name.ends_with(".webp") {
        "image/webp"
    } else if lower_name.ends_with(".heic") {
        "image/heic"
    } else if lower_name.ends_with(".heif") {
        "image/heif"
    } else if lower_name.ends_with(".bmp") {
        "image/bmp"
    } else if lower_name.ends_with(".pdf") {
        "application/pdf"
    } else {
        "application/octet-stream"
    }
}

/// Save an uploaded file to `UPLOAD_DIR` and return:
///   1. A task string augmented with the file's on-disk path.
///   2. Optional `(bytes, mime_type)` for images / PDFs that Gemini can see
///      directly as `inlineData` — so the model doesn't need shell commands
///      to read image content.
///
/// If no file is attached the original `text` is returned unchanged with
/// `None` vision data.  If a write fails we still deliver the text-only task.
async fn build_task_with_file(
    text: String,
    file: Option<crate::telegram::AttachedFile>,
) -> (String, Option<(Vec<u8>, String)>) {
    let Some(attached) = file else {
        return (text, None);
    };

    // Ensure the upload directory exists.
    let up_dir = upload_dir();
    if let Err(e) = fs::create_dir_all(&up_dir).await {
        warn!("Could not create upload dir {}: {}", up_dir, e);
        return (text, None);
    }

    // Sanitise the filename to avoid path traversal.
    let safe_name: String = attached
        .filename
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '_' })
        .collect();
    let safe_name = if safe_name.is_empty() { "upload".to_string() } else { safe_name };

    let file_path = format!("{}/{}", up_dir, safe_name);

    // Determine whether this file should be sent to Gemini as vision data.
    let effective_mime = attached
        .mime_type
        .clone()
        .unwrap_or_else(|| mime_from_filename(&safe_name).to_string());
    let use_vision = (is_vision_mime(&effective_mime) || is_vision_filename(&safe_name))
        && attached.bytes.len() <= VISION_MAX_BYTES;

    match fs::write(&file_path, &attached.bytes).await {
        Ok(()) => {
            info!(
                "Saved uploaded file to {} ({} bytes, vision={})",
                file_path,
                attached.bytes.len(),
                use_vision
            );

            let user_instruction = if text.trim().is_empty() {
                if use_vision {
                    "The user sent an image/file. Describe what you see and, \
                     if it shows an error or terminal output, suggest or run \
                     the appropriate commands to address it."
                        .to_string()
                } else {
                    "The user sent a file. Read and describe it using run_command."
                        .to_string()
                }
            } else {
                text
            };

            if use_vision {
                // For images and PDFs: embed the raw bytes as Gemini inline
                // data so the model can see the content directly.  We still
                // mention the path so the agent can run shell tools on it if
                // needed (e.g. to extract text with pdftotext, resize, etc.).
                let vision_data = Some((attached.bytes, effective_mime));
                let task = format!(
                    "{}\n\n[File also saved at {} for shell access if needed]",
                    user_instruction, file_path
                );
                (task, vision_data)
            } else {
                // Non-vision files (text, binary, audio, video, etc.): tell
                // the agent where the file lives and how to access it.
                let mime_line = attached
                    .mime_type
                    .map(|m| format!("\nMIME type: {}", m))
                    .unwrap_or_default();
                let task = format!(
                    "{}\n\nThe file is stored on the local filesystem at: {}\nFilename: {}{}\n\
                    Use run_command to access it (e.g. `cat` for text, `file` to identify \
                    type, `python3` to process it). \
                    Do NOT say you cannot see the file — use the shell.",
                    user_instruction, file_path, safe_name, mime_line
                );
                (task, None)
            }
        }
        Err(e) => {
            warn!("Failed to write uploaded file to {}: {}", file_path, e);
            (text, None)
        }
    }
}

/// Recursively collect all regular files under `dir` into `out`.
async fn collect_files_recursive(
    dir: &str,
    out: &mut Vec<std::path::PathBuf>,
) -> Result<()> {
    let mut read_dir = fs::read_dir(dir).await?;
    while let Ok(Some(entry)) = read_dir.next_entry().await {
        let ft = match entry.file_type().await {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        if ft.is_file() {
            out.push(entry.path());
        } else if ft.is_dir() {
            if let Some(sub) = entry.path().to_str() {
                // Best-effort: skip unreadable sub-dirs silently
                let _ = Box::pin(collect_files_recursive(sub, out)).await;
            }
        }
    }
    Ok(())
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
