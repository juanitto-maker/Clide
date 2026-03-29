// ============================================
// executor.rs - Safe Command Execution
// ============================================

use anyhow::{Context, Result};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc::Sender;
use tracing::{debug, error, warn};

use regex::Regex;

use crate::config::Config;

/// Maximum length for a single line sent to the live output channel.
/// Lines longer than this are truncated for display (full output still captured).
const MAX_LINE_DISPLAY_BYTES: usize = 4096;

/// Hard cap on stdout per command (2 MB). Output beyond this is drained and
/// discarded so the child process never blocks, but is not held in RAM.
const MAX_STDOUT_BYTES: usize = 2 * 1024 * 1024;
/// Stderr gets a smaller cap — it's rarely useful in bulk.
const MAX_STDERR_BYTES: usize = 512 * 1024;

#[derive(Debug, Clone)]
pub struct Executor {
    config: Config,
    /// Compiled regex patterns for blocked commands.
    blocked_regexes: Vec<Regex>,
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
        let blocked_regexes: Vec<Regex> = config
            .blocked_patterns
            .iter()
            .filter_map(|p| match Regex::new(p) {
                Ok(r) => Some(r),
                Err(e) => {
                    warn!("Invalid blocked_pattern regex '{}': {}", p, e);
                    None
                }
            })
            .collect();
        debug!("Compiled {} blocked command regex patterns", blocked_regexes.len());
        Self { config, blocked_regexes }
    }

    pub async fn execute(&self, command_str: &str) -> Result<ExecutionResult> {
        let start = std::time::Instant::now();

        for blocked in &self.config.blocked_commands {
            if command_str.contains(blocked) {
                error!("Blocked command attempt (substring): {}", command_str);
                return Err(anyhow::anyhow!(
                    "Command contains blocked pattern: {}",
                    blocked
                ));
            }
        }

        for re in &self.blocked_regexes {
            if re.is_match(command_str) {
                error!("Blocked command attempt (regex): {}", command_str);
                return Err(anyhow::anyhow!(
                    "Command matches blocked regex pattern: {}",
                    re.as_str()
                ));
            }
        }

        debug!("Executing command: {}", command_str);

        // Always set a valid CWD so the child shell never fails with
        // "getcwd() failed: No such file or directory" (common on Termux
        // when the process was started from an inaccessible directory).
        let home_dir = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());

        // Propagate HOME and TMPDIR explicitly so child processes (including
        // tools like AIWB) inherit correct values even if the parent env is
        // incomplete.  TMPDIR is set by telegram_bot.rs to $HOME/.clide/tmp
        // to avoid /tmp which is read-only on Termux/Android.
        let safe_tmp = std::env::var("TMPDIR")
            .unwrap_or_else(|_| format!("{}/.clide/tmp", home_dir));

        let mut child = Command::new("sh")
            .arg("-c")
            .arg(command_str)
            .current_dir(&home_dir)
            .env("HOME", &home_dir)
            .env("TMPDIR", &safe_tmp)
            .env("TEMPDIR", &safe_tmp)
            .env("TMP", &safe_tmp)
            .env("TEMP", &safe_tmp)
            .envs(self.config.secrets.iter())
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

    /// Execute a command with streaming output.
    ///
    /// Behaves like `execute()` but reads stdout line-by-line, sending each
    /// batch of lines to `live_output` approximately every 500 ms.  This lets
    /// callers relay real-time progress for long-running commands.
    ///
    /// The final `ExecutionResult` contains the full captured output (subject
    /// to `MAX_STDOUT_BYTES` / `MAX_STDERR_BYTES` caps), identical to what
    /// `execute()` would return.
    pub async fn execute_streaming(
        &self,
        command_str: &str,
        live_output: Option<Sender<String>>,
    ) -> Result<ExecutionResult> {
        // If no live_output channel, just fall back to the regular method.
        let live_tx = match live_output {
            Some(tx) => tx,
            None => return self.execute(command_str).await,
        };

        let start = std::time::Instant::now();

        // ── Blocked command checks (same as execute()) ───────────────────
        for blocked in &self.config.blocked_commands {
            if command_str.contains(blocked) {
                error!("Blocked command attempt (substring): {}", command_str);
                return Err(anyhow::anyhow!(
                    "Command contains blocked pattern: {}",
                    blocked
                ));
            }
        }
        for re in &self.blocked_regexes {
            if re.is_match(command_str) {
                error!("Blocked command attempt (regex): {}", command_str);
                return Err(anyhow::anyhow!(
                    "Command matches blocked regex pattern: {}",
                    re.as_str()
                ));
            }
        }

        debug!("Executing command (streaming): {}", command_str);

        let home_dir = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
        let safe_tmp = std::env::var("TMPDIR")
            .unwrap_or_else(|_| format!("{}/.clide/tmp", home_dir));

        let mut child = Command::new("sh")
            .arg("-c")
            .arg(command_str)
            .current_dir(&home_dir)
            .env("HOME", &home_dir)
            .env("TMPDIR", &safe_tmp)
            .env("TEMPDIR", &safe_tmp)
            .env("TMP", &safe_tmp)
            .env("TEMP", &safe_tmp)
            .envs(self.config.secrets.iter())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn command")?;

        let stdout_pipe = child.stdout.take().expect("stdout piped");
        let mut stderr_pipe = child.stderr.take().expect("stderr piped");

        // ── Stream stdout line-by-line with batching ─────────────────────
        // We read lines from stdout, accumulate them into a batch buffer,
        // and flush the batch to `live_tx` every ~500ms or when the buffer
        // gets large.  The full output is also accumulated (up to the cap).
        let batch_tx = live_tx;
        let stdout_handle: tokio::task::JoinHandle<(Vec<u8>, bool)> = tokio::spawn(async move {
            let mut reader = BufReader::new(stdout_pipe);
            let mut full_buf: Vec<u8> = Vec::new();
            let mut truncated = false;
            let mut batch_lines: Vec<String> = Vec::new();
            let mut last_flush = tokio::time::Instant::now();
            let flush_interval = tokio::time::Duration::from_millis(500);

            loop {
                let mut line_buf = String::new();
                let read_result = tokio::time::timeout(
                    flush_interval,
                    reader.read_line(&mut line_buf),
                )
                .await;

                match read_result {
                    Ok(Ok(0)) => {
                        // EOF — flush remaining batch and break
                        if !batch_lines.is_empty() {
                            let batch = batch_lines.join("");
                            let _ = batch_tx.send(batch).await;
                        }
                        break;
                    }
                    Ok(Ok(_n)) => {
                        // Got a line — accumulate into full buffer (up to cap)
                        if full_buf.len() < MAX_STDOUT_BYTES {
                            let remaining = MAX_STDOUT_BYTES - full_buf.len();
                            if line_buf.len() <= remaining {
                                full_buf.extend_from_slice(line_buf.as_bytes());
                            } else {
                                full_buf.extend_from_slice(&line_buf.as_bytes()[..remaining]);
                                truncated = true;
                            }
                        } else if !truncated {
                            truncated = true;
                        }

                        // Truncate the line for display if too long
                        let display_line = if line_buf.len() > MAX_LINE_DISPLAY_BYTES {
                            let safe = crate::truncate_utf8(&line_buf, MAX_LINE_DISPLAY_BYTES);
                            format!("{}…\n", safe.trim_end_matches('\n'))
                        } else {
                            line_buf
                        };
                        batch_lines.push(display_line);

                        // Flush if enough time has passed or batch is large
                        if last_flush.elapsed() >= flush_interval || batch_lines.len() >= 50 {
                            let batch = batch_lines.join("");
                            let _ = batch_tx.send(batch).await;
                            batch_lines = Vec::new();
                            last_flush = tokio::time::Instant::now();
                        }
                    }
                    Ok(Err(_e)) => {
                        // Read error — flush and break
                        if !batch_lines.is_empty() {
                            let batch = batch_lines.join("");
                            let _ = batch_tx.send(batch).await;
                        }
                        break;
                    }
                    Err(_) => {
                        // Timeout — flush accumulated batch even though no new line arrived
                        if !batch_lines.is_empty() {
                            let batch = batch_lines.join("");
                            let _ = batch_tx.send(batch).await;
                            batch_lines = Vec::new();
                            last_flush = tokio::time::Instant::now();
                        }
                    }
                }
            }

            // If we hit the cap, drain remaining stdout so child doesn't block
            if truncated {
                let mut remaining_reader = reader.into_inner();
                let _ = tokio::io::copy(&mut remaining_reader, &mut tokio::io::sink()).await;
            }

            (full_buf, truncated)
        });

        // Read stderr fully with the existing capped approach
        let stderr_handle = tokio::spawn(async move {
            read_capped(&mut stderr_pipe, MAX_STDERR_BYTES).await.unwrap_or_default()
        });

        // Wait for both streams to finish, then wait for child exit
        let (stdout_result, stderr_result) = tokio::join!(stdout_handle, stderr_handle);

        let status = child.wait().await?;
        let duration = start.elapsed().as_millis() as u64;

        let (stdout_bytes, stdout_truncated) = stdout_result.unwrap_or_default();
        let (stderr_bytes, stderr_truncated) = stderr_result.unwrap_or_default();

        let mut stdout = String::from_utf8_lossy(&stdout_bytes).trim().to_string();
        if stdout_truncated {
            warn!(
                "stdout truncated at {} MB for: {}",
                MAX_STDOUT_BYTES / 1024 / 1024,
                command_str
            );
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
