// ============================================
// bot.rs - Matrix Bot Core Loop
// ============================================

use anyhow::Result;
use log::{info, warn, error};

use crate::agent::Agent;
use crate::config::Config;
use crate::matrix::MatrixClient;

/// Main bot structure (exported as Bot from lib.rs)
pub struct Bot {
    config: Config,
    agent: Agent,
    matrix: MatrixClient,
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
            matrix,
        })
    }

    /// Start the bot loop - polls Matrix room and replies via Gemini
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting Clide bot...");

        // Resolve the bot's actual Matrix user ID so the self-response guard
        // works correctly regardless of what is written in matrix_user in config.
        match self.matrix.fetch_bot_user_id().await {
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

        self.matrix.log_room_id();

        println!(
            "Bot running. Send a message in Matrix room {}. Ctrl+C to stop.",
            self.config.matrix_room_id
        );
        println!("Send /stop in the room to abort a running task.");

        loop {
            match self.matrix.receive_messages().await {
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
                                .send_message("No task is currently running.")
                                .await;
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
        if self.matrix.is_bot_sender(&sender) {
            info!("Ignoring own message from {} to prevent self-response loop.", sender);
            return Ok(());
        }

        // Authorization check (skip if no authorized users configured)
        if !self.config.authorized_users.is_empty() && !self.config.is_authorized(&sender) {
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
        let response = self.agent.run(&text, &sender, None).await?;

        self.matrix.send_message(&response).await?;
        info!("Replied to {}", sender);

        Ok(())
    }

    /// Ask the room for YES/NO confirmation before proceeding
    async fn confirm_execution(&mut self, sender: &str, text: &str) -> Result<bool> {
        let confirm_msg = format!(
            "Confirm execution?\n\n{}\n\nReply with YES to proceed.",
            text
        );

        self.matrix.send_message(&confirm_msg).await?;

        let reply = self
            .matrix
            .wait_for_reply(sender, self.config.confirmation_timeout)
            .await?;

        Ok(reply.trim().eq_ignore_ascii_case("yes"))
    }

}
