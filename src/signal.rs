// ============================================
// signal.rs - Signal-CLI Wrapper
// Sends/receives Signal messages via signal-cli
// ============================================

use anyhow::{Context, Result};
use std::process::Command;
use std::time::{Duration, Instant};

pub struct SignalMessage {
    pub sender: String,
    pub text: String,
}

pub struct SignalClient {
    number: String,
}

impl SignalClient {
    pub fn new(number: String) -> Self {
        Self { number }
    }

    /// Receive pending messages from Signal (calls signal-cli with 5s timeout)
    pub fn receive_messages(&self) -> Result<Vec<SignalMessage>> {
        let output = Command::new("signal-cli")
            .args([
                "-u", &self.number,
                "--output=json",
                "receive",
                "--timeout", "5",
            ])
            .output()
            .context("Failed to run signal-cli. Is it installed and in PATH?")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut messages = Vec::new();

        for line in stdout.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            match serde_json::from_str::<serde_json::Value>(line) {
                Ok(val) => {
                    // signal-cli JSON envelope format
                    if let (Some(sender), Some(text)) = (
                        val["envelope"]["sourceNumber"].as_str(),
                        val["envelope"]["dataMessage"]["message"].as_str(),
                    ) {
                        let text = text.trim();
                        if !text.is_empty() {
                            messages.push(SignalMessage {
                                sender: sender.to_string(),
                                text: text.to_string(),
                            });
                        }
                    }
                }
                Err(_) => continue,
            }
        }

        Ok(messages)
    }

    /// Send a message to recipient via signal-cli
    pub fn send_message(&self, recipient: &str, message: &str) -> Result<()> {
        Command::new("signal-cli")
            .args(["-u", &self.number, "send", "-m", message, recipient])
            .output()
            .context("Failed to run signal-cli send")?;
        Ok(())
    }

    /// Poll for a reply from sender within timeout_secs
    pub fn wait_for_reply(&self, sender: &str, timeout_secs: u64) -> Result<String> {
        let start = Instant::now();
        let timeout = Duration::from_secs(timeout_secs);

        while start.elapsed() < timeout {
            let msgs = self.receive_messages()?;
            for msg in msgs {
                if msg.sender == sender {
                    return Ok(msg.text);
                }
            }
            std::thread::sleep(Duration::from_secs(2));
        }

        Ok(String::new())
    }
}
