// ============================================
// executor.rs - Safe Command Execution
// ============================================

use anyhow::{Context, Result};
use std::process::Stdio;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tracing::{debug, error, warn};

use crate::config::Config;

/// Hard cap on stdout per command (2 MB). Output beyond this is drained and
/// discarded so the child process never blocks, but is not held in RAM.
const MAX_STDOUT_BYTES: usize = 2 * 1024 * 1024;
/// Stderr gets a smaller cap — it's rarely useful in bulk.
const MAX_STDERR_BYTES: usize = 512 * 1024;

#[derive(Debug, Clone)]
pub struct Executor {
    config: Config,
}

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration_ms: u64,
}

impl ExecutionResult {
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }

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
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub async fn execute(&self, command_str: &str) -> Result<ExecutionResult> {
        let start = std::time::Instant::now();

        for blocked in &self.config.blocked_commands {
            if command_str.contains(blocked) {
                error!("Blocked command attempt: {}", command_str);
                return Err(anyhow::anyhow!(
                    "Command contains blocked pattern: {}",
                    blocked
                ));
            }
        }

        debug!("Executing command: {}", command_str);

        // Always set a valid CWD so the child shell never fails with
        // "getcwd() failed: No such file or directory" (common on Termux
        // when the process was started from an inaccessible directory).
        let home_dir = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());

        let mut child = Command::new("sh")
            .arg("-c")
            .arg(command_str)
            .current_dir(&home_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn command")?;

        let mut stdout_pipe = child.stdout.take().expect("stdout piped");
        let mut stderr_pipe = child.stderr.take().expect("stderr piped");

        // Read both streams concurrently with hard size caps.
        // After the cap is reached we drain the remainder to /dev/null so the
        // child is never blocked on a full pipe — avoiding deadlock.
        let (stdout_result, stderr_result) = tokio::join!(
            read_capped(&mut stdout_pipe, MAX_STDOUT_BYTES),
            read_capped(&mut stderr_pipe, MAX_STDERR_BYTES),
        );

        let status = child.wait().await?;
        let duration = start.elapsed().as_millis() as u64;

        let (stdout_bytes, stdout_truncated) = stdout_result.unwrap_or_default();
        let (stderr_bytes, stderr_truncated) = stderr_result.unwrap_or_default();

        let mut stdout = String::from_utf8_lossy(&stdout_bytes).trim().to_string();
        if stdout_truncated {
            warn!("stdout truncated at {} MB for: {}", MAX_STDOUT_BYTES / 1024 / 1024, command_str);
            stdout.push_str(&format!(
                "\n[output truncated at {} MB]",
                MAX_STDOUT_BYTES / 1024 / 1024
            ));
        }

        let mut stderr = String::from_utf8_lossy(&stderr_bytes).trim().to_string();
        if stderr_truncated {
            stderr.push_str("\n[stderr truncated]");
        }

        Ok(ExecutionResult {
            stdout,
            stderr,
            exit_code: status.code().unwrap_or(-1),
            duration_ms: duration,
        })
    }
}

/// Read at most `limit` bytes from `reader` into a buffer, then drain the
/// rest into a sink so the writing process is never blocked.
///
/// Returns `(bytes_read, was_truncated)`.
async fn read_capped(
    reader: &mut (impl AsyncReadExt + Unpin),
    limit: usize,
) -> Result<(Vec<u8>, bool)> {
    let mut buf = Vec::new();
    reader.take(limit as u64).read_to_end(&mut buf).await?;

    let truncated = buf.len() >= limit;
    if truncated {
        // Drain remaining bytes so the child can exit cleanly
        tokio::io::copy(reader, &mut tokio::io::sink()).await?;
    }

    Ok((buf, truncated))
}
