// ============================================
// executor.rs - Safe Command Execution
// ============================================
// Executes commands with security checks and logging

use anyhow::{Context, Result};
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{timeout, Duration};
use tracing::{debug, error, info, warn};

use crate::config::Config;

/// Command executor with security features
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

    /// Execute command with all security checks
    pub async fn execute(&self, command: &str) -> Result<ExecutionResult> {
        info!("Executing command: {}", command);

        // Security checks
        if !self.config.allow_commands {
            error!("Command execution is disabled in config");
            anyhow::bail!("Command execution is disabled");
        }

        if self.is_blocked(command) {
            error!("Command is blocked: {}", command);
            anyhow::bail!("This command is blocked for security reasons");
        }

        if self.config.deny_by_default && !self.is_allowed(command) {
            error!("Command not in whitelist: {}", command);
            anyhow::bail!("This command is not in the whitelist");
        }

        // Dry run mode
        if self.config.dry_run {
            warn!("DRY RUN: Would execute: {}", command);
            return Ok(ExecutionResult {
                stdout: format!("[DRY RUN] Would execute: {}", command),
                stderr: String::new(),
                exit_code: 0,
                duration_ms: 0,
            });
        }

        // Execute with timeout
        let start = std::time::Instant::now();
        let result = self.execute_internal(command).await?;
        let duration_ms = start.elapsed().as_millis() as u64;

        info!(
            "Command completed: exit_code={}, duration={}ms",
            result.exit_code, duration_ms
        );

        Ok(ExecutionResult {
            stdout: result.stdout,
            stderr: result.stderr,
            exit_code: result.exit_code,
            duration_ms,
        })
    }

    /// Internal execution without security checks
    async fn execute_internal(&self, command: &str) -> Result<ExecutionResult> {
        let timeout_duration = Duration::from_secs(self.config.execution.timeout);

        // Prepare command
        let mut cmd = Command::new(&self.config.execution.shell);
        
        for flag in &self.config.execution.shell_flags {
            cmd.arg(flag);
        }
        
        cmd.arg(command);
        cmd.current_dir(expand_path(&self.config.execution.working_dir));
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        debug!("Spawning process: {} {:?}", self.config.execution.shell, command);

        // Execute with timeout
        let result = timeout(timeout_duration, cmd.output()).await;

        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code().unwrap_or(-1);

                Ok(ExecutionResult {
                    stdout,
                    stderr,
                    exit_code,
                    duration_ms: 0, // Set by caller
                })
            }
            Ok(Err(e)) => {
                error!("Failed to execute command: {}", e);
                Err(e).context("Failed to execute command")
            }
            Err(_) => {
                error!("Command timed out after {}s", self.config.execution.timeout);
                anyhow::bail!("Command execution timed out")
            }
        }
    }

    /// Check if command is blocked
    fn is_blocked(&self, command: &str) -> bool {
        for pattern in &self.config.blocked_commands {
            if matches_pattern(command, pattern) {
                debug!("Command matched blocked pattern: {}", pattern);
                return true;
            }
        }
        false
    }

    /// Check if command is in whitelist
    fn is_allowed(&self, command: &str) -> bool {
        if self.config.allowed_commands.is_empty() {
            return true; // Empty whitelist means all allowed
        }

        for pattern in &self.config.allowed_commands {
            if matches_pattern(command, pattern) {
                debug!("Command matched allowed pattern: {}", pattern);
                return true;
            }
        }
        false
    }

    /// Execute multiple commands in sequence
    pub async fn execute_batch(&self, commands: Vec<String>) -> Result<Vec<ExecutionResult>> {
        let mut results = Vec::new();

        for (i, command) in commands.iter().enumerate() {
            info!("Executing batch command {}/{}", i + 1, commands.len());
            
            match self.execute(command).await {
                Ok(result) => {
                    results.push(result);
                }
                Err(e) => {
                    error!("Batch command {} failed: {}", i + 1, e);
                    return Err(e);
                }
            }
        }

        Ok(results)
    }
}

/// Check if command matches pattern (supports wildcards)
fn matches_pattern(command: &str, pattern: &str) -> bool {
    // Simple wildcard matching
    if pattern.contains('*') {
        let parts: Vec<&str> = pattern.split('*').collect();
        
        if parts.len() == 2 {
            let prefix = parts[0];
            let suffix = parts[1];
            
            return command.starts_with(prefix) && command.ends_with(suffix);
        }
    }

    // Exact match
    command == pattern || command.starts_with(&format!("{} ", pattern))
}

/// Expand ~ in path
fn expand_path(path: &str) -> String {
    if path.starts_with('~') {
        if let Some(home) = dirs::home_dir() {
            return path.replacen('~', &home.to_string_lossy(), 1);
        }
    }
    path.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_pattern() {
        assert!(matches_pattern("ls -la", "ls*"));
        assert!(matches_pattern("systemctl status nginx", "systemctl status*"));
        assert!(!matches_pattern("rm -rf /", "ls*"));
        
        assert!(matches_pattern("ls", "ls"));
        assert!(matches_pattern("ls -la", "ls"));
    }

    #[test]
    fn test_expand_path() {
        let expanded = expand_path("~/test");
        assert!(!expanded.contains('~'));
        
        let not_expanded = expand_path("/tmp/test");
        assert_eq!(not_expanded, "/tmp/test");
    }

    #[tokio::test]
    async fn test_execution_result() {
        let result = ExecutionResult {
            stdout: "Hello".to_string(),
            stderr: "".to_string(),
            exit_code: 0,
            duration_ms: 100,
        };

        assert!(result.success());
        assert_eq!(result.output(), "Hello");
    }
}