// ============================================
// bot.rs - Matrix Bot Core Loop
// ============================================

use anyhow::Result;
use log::{info, warn, error};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

use crate::agent::Agent;
use crate::config::Config;
use crate::matrix::MatrixClient;
use crate::scrubber;

/// Maximum characters for the Matrix progress status message.
/// Matrix doesn't have a hard limit like Telegram's 4096, but we keep
/// the live-updating message readable.
const MATRIX_MAX_PROGRESS_CHARS: usize = 4000;

/// Main bot structure (exported as Bot from lib.rs)
pub struct Bot {
    config: Config,
    agent: Agent,
    matrix: Arc<Mutex<MatrixClient>>,
}

impl Bot {
    /// Initialize bot from config
    pub fn new(config: Config) -> Result<Self> {
        let agent = Agent::new(&config);

        let matrix = MatrixClient::new(
            config.matrix_homeserver.clone(),
            config.matrix_access_token.clone(),
            config.matrix_room_id.clone(),
        );

        Ok(Self {
            config,
            agent,
            matrix: Arc::new(Mutex::new(matrix)),
        })
    }

    /// Start the bot loop - polls Matrix room and replies via Gemini
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting Clide bot...");

        // Resolve the bot's actual Matrix user ID so the self-response guard
        // works correctly regardless of what is written in matrix_user in config.
        {
            let mut mx = self.matrix.lock().await;
            match mx.fetch_bot_user_id().await {
                Ok(ref id) => {
                    info!(
                        "Bot authenticated as: {} — messages from this account will be \
                         ignored (self-response guard). Messages from any other account \
                         will be processed normally.",
                        id
                    );
                }
                Err(e) => error!(
                    "Could not fetch bot user ID via /whoami ({}). \
                     Self-response filtering is DISABLED — check your homeserver URL and \
                     access token.  If the bot loops, stop it and fix the config.",
                    e
                ),
            }
            mx.log_room_id();
        }

        println!(
            "Bot running. Send a message in Matrix room {}. Ctrl+C to stop.",
            self.config.matrix_room_id
        );
        println!("Send /stop in the room to abort a running task.");

        // ── Start the scheduler background task ──────────────────────────
        let _scheduler_handle = if !self.config.scheduled_tasks.is_empty() {
            let notify = crate::scheduler::NotifyChannel::Matrix {
                client: Arc::clone(&self.matrix),
            };
            let handle = crate::scheduler::spawn(self.config.clone(), notify);
            println!(
                "Scheduler: {} task(s) active",
                self.config
                    .scheduled_tasks
                    .iter()
                    .filter(|t| t.enabled)
                    .count()
            );
            Some(handle)
        } else {
            None
        };

        let ctrl_c_fut = tokio::signal::ctrl_c();
        tokio::pin!(ctrl_c_fut);

        loop {
            let messages = {
                let mut mx = self.matrix.lock().await;
                tokio::select! {
                    biased;
                    _ = &mut ctrl_c_fut => {
                        println!("\nShutting down Clide bot...");
                        break Ok(());
                    }
                    r = mx.receive_messages() => r,
                }
            };

            match messages {
                Ok(messages) => {
                    for msg in messages {
                        // ── /stop command ────────────────────────────────────
                        // The Matrix bot processes messages sequentially, so
                        // /stop received here means no agent task is running.
                        // (While an agent task runs the poll loop is blocked;
                        // runtime cancellation is not supported for Matrix.)
                        if msg.text.trim().eq_ignore_ascii_case("/stop") {
                            info!("Received /stop while idle — no task running.");
                            let _ = self
                                .matrix
                                .lock()
                                .await
                                .send_message("No task is currently running.")
                                .await;
                            continue;
                        }

                        // /stats — show usage statistics
                        if msg.text.trim().eq_ignore_ascii_case("/stats") {
                            let reply = self.build_stats_message();
                            let _ = self.matrix.lock().await.send_message(&reply).await;
                            continue;
                        }

                        // /schedule — show scheduled tasks status
                        if msg.text.trim().eq_ignore_ascii_case("/schedule") {
                            let reply = crate::scheduler::build_schedule_message(&self.config);
                            let _ = self.matrix.lock().await.send_message(&reply).await;
                            continue;
                        }

                        if let Err(e) = self.handle_message(msg.sender, msg.text).await {
                            eprintln!("Error handling message: {}", e);
                        }
                    }
                    // Brief pause between successful poll cycles. The /sync call already
                    // long-polls for up to 5 s when idle, so this only adds latency in
                    // high-traffic situations and prevents runaway tight loops if
                    // self-response filtering somehow breaks down.
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
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
        // an infinite loop.  Uses only the user ID resolved via /whoami so that
        // a wrong/old value in config.matrix_user cannot accidentally block
        // messages from the human operator.
        if self.matrix.lock().await.is_bot_sender(&sender) {
            info!("Ignoring own message from {} to prevent self-response loop.", sender);
            return Ok(());
        }

        // Authorization — fail-closed: if the allowlist is empty, nobody is
        // allowed in.  An open bot controlling a real shell is unsafe by design.
        // Add your Matrix user ID(s) to authorized_users in config.yaml.
        if self.config.authorized_users.is_empty() {
            warn!(
                "Matrix message from {} rejected: authorized_users is empty. \
                 Add your Matrix user ID to authorized_users in ~/.clide/config.yaml.",
                sender
            );
            return Ok(());
        }
        if !self.config.is_authorized(&sender) {
            warn!("Unauthorized sender: {}", sender);
            return Ok(());
        }

        // Optional confirmation gate
        if self.config.require_confirmation {
            if !self.confirm_execution(&sender, &text).await? {
                return Ok(());
            }
        }

        info!("Running agent task...");

        // Send an initial "Working..." status message and get its event ID
        // so we can edit it with live progress updates.
        let status_event_id = self
            .matrix
            .lock()
            .await
            .send_message_returning_id("Working...")
            .await
            .unwrap_or_default();

        // Progress channel (agent -> updater task)
        let (tx, mut rx) = mpsc::channel::<String>(64);

        // Spawn an updater task that edits the status message with cumulative
        // progress.  It edits in-place every 3 seconds at most to avoid
        // flooding the Matrix server with edit requests.
        let matrix_handle = Arc::clone(&self.matrix);
        let event_id = status_event_id.clone();
        let live_secrets = self.config.secrets.clone();
        let updater: tokio::task::JoinHandle<String> = tokio::spawn(async move {
            let mut log = String::new();
            let mut last_edit = tokio::time::Instant::now();
            let edit_interval = tokio::time::Duration::from_secs(3);

            while let Some(line) = rx.recv().await {
                log.push('\n');
                log.push_str(&line);

                // Only edit the message if enough time has passed since the last edit
                if last_edit.elapsed() >= edit_interval {
                    let display = if log.len() > MATRIX_MAX_PROGRESS_CHARS {
                        format!(
                            "[...]\n{}",
                            &log[log.len() - (MATRIX_MAX_PROGRESS_CHARS - 6)..]
                        )
                    } else {
                        log.clone()
                    };
                    let display = scrubber::scrub(&display, &live_secrets);
                    let _ = matrix_handle
                        .lock()
                        .await
                        .edit_message(&event_id, &format!("Working...{}", display))
                        .await;
                    last_edit = tokio::time::Instant::now();
                }
            }

            // Final flush: edit with latest progress
            if !log.is_empty() {
                let display = if log.len() > MATRIX_MAX_PROGRESS_CHARS {
                    format!(
                        "[...]\n{}",
                        &log[log.len() - (MATRIX_MAX_PROGRESS_CHARS - 6)..]
                    )
                } else {
                    log.clone()
                };
                let display = scrubber::scrub(&display, &live_secrets);
                let _ = matrix_handle
                    .lock()
                    .await
                    .edit_message(&event_id, &format!("Working...{}", display))
                    .await;
            }

            log
        });

        // Run the agentic loop (drops tx when done, closing the updater channel)
        let result = self.agent.run(&text, &sender, Some(tx), None).await;

        // Wait for the updater to flush its last edit and collect the log
        let _commands_log = updater.await.unwrap_or_default();

        // Build and send the final response
        let raw_text = match result {
            Ok(r) if !r.is_empty() => r,
            Ok(_) => "Done.".to_string(),
            Err(e) => format!("Error: {}", e),
        };
        let final_text = scrubber::scrub(&raw_text, &self.config.secrets);

        // Edit the status message with the final answer so the "Working..."
        // placeholder is replaced by the actual result.
        let mut mx = self.matrix.lock().await;
        if !status_event_id.is_empty() {
            let _ = mx.edit_message(&status_event_id, &final_text).await;
        } else {
            // Fallback: send as a new message if we couldn't get the event ID
            mx.send_message(&final_text).await?;
        }

        info!("Replied to {}", sender);

        Ok(())
    }

    /// Build a stats summary message from the database.
    fn build_stats_message(&self) -> String {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let db_path = format!("{}/.clide/memory.db", home);
        match crate::database::Database::new(&db_path) {
            Ok(db) => match db.get_stats() {
                Ok(stats) => {
                    format!(
                        "📊 Clide Stats\n\n\
                         Messages: {}\n\
                         Commands: {}\n\
                         Users: {}\n\
                         Known facts: {}\n\
                         Model: {}\n\
                         Fallback: {}\n\
                         Version: {}",
                        stats.total_messages,
                        stats.total_commands,
                        stats.total_users,
                        stats.total_facts,
                        self.config.gemini_model,
                        self.config.fallback_model,
                        crate::VERSION,
                    )
                }
                Err(e) => format!("❌ Could not read stats: {}", e),
            },
            Err(e) => format!("❌ Could not open database: {}", e),
        }
    }

    /// Ask the room for YES/NO confirmation before proceeding
    async fn confirm_execution(&mut self, sender: &str, text: &str) -> Result<bool> {
        let confirm_msg = format!(
            "Confirm execution?\n\n{}\n\nReply with YES to proceed.",
            text
        );

        self.matrix.lock().await.send_message(&confirm_msg).await?;

        let reply = self
            .matrix
            .lock()
            .await
            .wait_for_reply(sender, self.config.confirmation_timeout)
            .await?;

        Ok(reply.trim().eq_ignore_ascii_case("yes"))
    }

}
