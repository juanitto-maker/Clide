// ============================================
// executor.rs - Safe Command Execution (CORRECTED)
// ============================================

use anyhow::{Context, Result};
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{timeout, Duration};
use tracing::{debug, error, info, warn};

use crate::config::Config;

/// Command executor with security features
#[derive(Debug, Clone)] // Added Clone here
pub struct Executor {
    config: Config,
}

/// Command execution result
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration_ms: u64,
}

impl ExecutionResult {
    /// Check if command succeeded
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }

    /// Get combined output
    pub fn output(&self) -> String {
        if self.stderr.is_empty() {
            self.stdout.clone()
        } else if self.stdout.is_empty() {
            self.stderr.clone()
        } else {
            format!("{}\n{}", self.stdout, self.stderr)
        }
    }
}

impl Executor {
    /// Create new executor
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// Execute command with safety checks
    pub async fn execute(&self, command_str: &str) -> Result<ExecutionResult> {
        let start = std::time::Instant::now();
        
        // Security Check: Blocked commands
        for blocked in &self.config.blocked_commands {
            if command_str.contains(blocked) {
                error!("Blocked command attempt: {}", command_str);
                return Err(anyhow::anyhow!("Command contains blocked pattern: {}", blocked));
            }
        }

        debug!("Executing command: {}", command_str);

        let output = Command::new("sh")
            .arg("-c")
            .arg(command_str)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn command")?
            .wait_with_output()
            .await?;

        let duration = start.elapsed().as_millis() as u64;

        Ok(ExecutionResult {
            stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            exit_code: output.status.code().unwrap_or(-1),
            duration_ms: duration,
        })
    }
}
