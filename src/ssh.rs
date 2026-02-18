// ============================================
// ssh.rs - SSH stub (russh removed: fails to compile on Android ARM64)
// SSH support can be added back later as an optional feature.
// ============================================

use anyhow::Result;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct SshClient {
    #[allow(dead_code)]
    timeout: u64,
    #[allow(dead_code)]
    verify_host_keys: bool,
}

#[derive(Debug, Clone)]
pub struct SshOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: u32,
}

impl SshClient {
    pub fn new(timeout: u64, verify_host_keys: bool) -> Self {
        Self { timeout, verify_host_keys }
    }

    pub async fn execute(
        &self,
        _host: &str,
        _user: &str,
        _command: &str,
        _key_path: Option<&Path>,
    ) -> Result<SshOutput> {
        Err(anyhow::anyhow!("SSH support not compiled in (russh removed for Android ARM64 compatibility)"))
    }
}
