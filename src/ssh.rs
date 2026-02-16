// ============================================
// ssh.rs - SSH Operations (CORRECTED)
// ============================================

use anyhow::{Context, Result};
use russh::client;
use russh_keys::key;
use std::path::Path;
use std::sync::Arc;

pub struct SshClient {
    timeout: u64,
    #[allow(dead_code)]
    verify_host_keys: bool,
}

struct ClientHandler;

#[async_trait::async_trait]
impl client::Handler for ClientHandler {
    type Error = anyhow::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &key::PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true) // Trust all hosts for now; configure for production
    }
}

#[derive(Debug, Clone)]
pub struct SshOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: u32,
}

impl SshClient {
    pub fn new(timeout: u64, verify_host_keys: bool) -> Self {
        Self {
            timeout,
            verify_host_keys,
        }
    }

    pub async fn execute(
        &self,
        host: &str,
        user: &str,
        command: &str,
        key_path: Option<&Path>,
    ) -> Result<SshOutput> {
        let (hostname, port) = parse_host(host);
        
        // FIXED: connection_timeout is no longer a field in russh::client::Config
        let config = russh::client::Config {
            ..Default::default()
        };
        let config = Arc::new(config);
        let sh = ClientHandler;
        
        // Wrap connection in a timeout
        let mut session = tokio::time::timeout(
            std::time::Duration::from_secs(self.timeout),
            russh::client::connect(config, (hostname, port), sh)
        ).await.context("SSH connection timed out")??;

        let auth_res = if let Some(path) = key_path {
            let key_pair = load_private_key(path)?;
            session.authenticate_publickey(user, Arc::new(key_pair)).await?
        } else {
            false
        };

        if !auth_res {
            return Err(anyhow::anyhow!("SSH Authentication failed"));
        }

        let mut channel = session.channel_open_session().await?;
        
        // FIXED: .exec requires &[u8] for the command
        channel.exec(true, command.as_bytes()).await?;

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut exit_code = 0;

        while let Some(msg) = channel.wait().await {
            match msg {
                russh::ChannelMsg::Data { data } => stdout.extend_from_slice(&data),
                russh::ChannelMsg::ExtendedData { data, .. } => stderr.extend_from_slice(&data),
                russh::ChannelMsg::ExitStatus { exit_status } => exit_code = exit_status,
                _ => {}
            }
        }

        Ok(SshOutput {
            stdout: String::from_utf8_lossy(&stdout).to_string(),
            stderr: String::from_utf8_lossy(&stderr).to_string(),
            exit_code,
        })
    }
}

fn parse_host(host: &str) -> (String, u16) {
    if let Some((h, p)) = host.split_once(':') {
        (h.to_string(), p.parse().unwrap_or(22))
    } else {
        (host.to_string(), 22)
    }
}

fn load_private_key(path: &Path) -> Result<key::KeyPair> {
    let key_data = std::fs::read_to_string(path)?;
    let key_pair = russh_keys::decode_secret_key(&key_data, None)
        .context("Failed to decode SSH key")?;
    Ok(key_pair)
}
