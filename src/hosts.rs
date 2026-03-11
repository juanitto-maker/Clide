// ============================================
// hosts.rs - Named host registry
// ============================================
// Stores SSH host configurations under nicknames in ~/.clide/hosts.yaml.
// The bot resolves @nickname or "on prod" references to actual connection
// details at execution time — nicknames never leave the device.
//
// File format (~/.clide/hosts.yaml):
//   prod:
//     ip: "1.2.3.4"
//     user: "root"
//     key_path: "/data/data/com.termux/files/home/.ssh/id_ed25519_prod"
//     port: 22
//     notes: "Main VPS - Hetzner DE"
//   pi:
//     ip: "100.x.y.z"       ← can be Tailscale IP
//     user: "pi"
//     key_path: "~/.ssh/id_ed25519_pi"
//     port: 22
//     notes: "Raspberry Pi home lab"
//
// The file is chmod 600. It is included in vault backups (encrypted).

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostEntry {
    pub ip: String,
    pub user: String,
    pub key_path: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub notes: String,
}

fn default_port() -> u16 { 22 }

pub type HostMap = HashMap<String, HostEntry>;

pub fn hosts_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join(".clide/hosts.yaml")
}

/// Load the hosts file. Returns an empty map if the file does not exist.
pub fn load() -> Result<HostMap> {
    let path = hosts_path();
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("Cannot read {:?}", path))?;
    let map: HostMap = serde_yaml::from_str(&raw)
        .with_context(|| format!("Cannot parse {:?}", path))?;
    Ok(map)
}

/// Save the hosts map back to disk (overwrites) and tighten permissions.
pub fn save(map: &HostMap) -> Result<()> {
    let path = hosts_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let yaml = serde_yaml::to_string(map)?;
    std::fs::write(&path, yaml)?;
    // Restrict to owner-read-write only.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

/// Add or replace a host entry.
pub fn add(nickname: &str, entry: HostEntry) -> Result<()> {
    let mut map = load()?;
    map.insert(nickname.to_lowercase(), entry);
    save(&map)
}

/// Remove a host by nickname. Errors if it doesn't exist.
pub fn remove(nickname: &str) -> Result<()> {
    let mut map = load()?;
    if map.remove(&nickname.to_lowercase()).is_none() {
        bail!("Host '{}' not found in hosts.yaml", nickname);
    }
    save(&map)
}

/// Inject all host fields into the secrets map so skills can reference them as
/// ${HOST_PROD_IP}, ${HOST_PROD_USER}, ${HOST_PROD_KEY_PATH}, ${HOST_PROD_PORT}.
///
/// This is called once during Config::load (through the host manager) so every
/// skill automatically has access to connection details without exposing
/// nicknames or IPs in the chat transcript.
pub fn inject_into_secrets(map: &HostMap, secrets: &mut HashMap<String, String>) {
    for (nick, entry) in map {
        let prefix = format!("HOST_{}", nick.to_uppercase());
        secrets.insert(format!("{}_IP", prefix), entry.ip.clone());
        secrets.insert(format!("{}_USER", prefix), entry.user.clone());
        secrets.insert(format!("{}_KEY_PATH", prefix), entry.key_path.clone());
        secrets.insert(format!("{}_PORT", prefix), entry.port.to_string());
    }
}

/// Pretty-print the host list for display (IPs are NOT shown in chat —
/// this is only used in the local CLI `clide host list` command).
pub fn format_list(map: &HostMap) -> String {
    if map.is_empty() {
        return "No hosts configured. Run: clide host add <nickname> [options]".to_string();
    }
    let mut lines = vec!["Configured hosts:".to_string()];
    let mut names: Vec<&String> = map.keys().collect();
    names.sort();
    for name in names {
        let h = &map[name];
        let notes = if h.notes.is_empty() { String::new() } else { format!("  # {}", h.notes) };
        lines.push(format!(
            "  {:12}  {}@{}:{}  key={}{}",
            name, h.user, h.ip, h.port, h.key_path, notes
        ));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inject_into_secrets() {
        let mut map = HostMap::new();
        map.insert("prod".to_string(), HostEntry {
            ip: "1.2.3.4".to_string(),
            user: "root".to_string(),
            key_path: "~/.ssh/id_prod".to_string(),
            port: 22,
            notes: String::new(),
        });
        let mut secrets = HashMap::new();
        inject_into_secrets(&map, &mut secrets);
        assert_eq!(secrets.get("HOST_PROD_IP").map(|s| s.as_str()), Some("1.2.3.4"));
        assert_eq!(secrets.get("HOST_PROD_USER").map(|s| s.as_str()), Some("root"));
        assert_eq!(secrets.get("HOST_PROD_PORT").map(|s| s.as_str()), Some("22"));
    }
}
