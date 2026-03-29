// ============================================
// update.rs - Self-Update from GitHub Releases
// ============================================

use anyhow::{bail, Context, Result};
use colored::*;
use reqwest::Client;
use serde::Deserialize;
use std::env;
use std::io::Write;
use std::process::Command;

const GITHUB_REPO: &str = "juanitto-maker/Clide";

#[derive(Debug, Deserialize)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    body: Option<String>,
    assets: Vec<ReleaseAsset>,
}

/// Detect the Rust target triple for the current platform.
fn current_target() -> &'static str {
    // This is set at compile time by build.rs or we infer from cfg
    // We match the targets from build.sh
    cfg_if_target()
}

fn cfg_if_target() -> &'static str {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    { return "x86_64-unknown-linux-gnu"; }

    #[cfg(all(target_os = "linux", target_arch = "aarch64", not(target_os = "android")))]
    { return "aarch64-unknown-linux-gnu"; }

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    { return "x86_64-apple-darwin"; }

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    { return "aarch64-apple-darwin"; }

    #[cfg(target_os = "android")]
    { return "aarch64-linux-android"; }

    #[cfg(not(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64"),
        target_os = "android",
    )))]
    { return "unknown"; }
}

/// Run the update command: check, download, replace, print changelog.
/// If `purge` is true, clean up leftover artifacts from previous versions
/// (old binaries, stale temp files, source checkouts) after updating.
pub async fn run_with_opts(purge: bool) -> Result<()> {
    let result = run_inner().await;
    if purge {
        purge_old_artifacts();
    }
    result
}

pub async fn run() -> Result<()> {
    run_inner().await
}

async fn run_inner() -> Result<()> {
    let current_version = crate::VERSION;
    println!("{}", "Checking for updates...".bright_cyan());

    let client = Client::builder()
        .user_agent(format!("clide/{}", current_version))
        .build()
        .context("Failed to create HTTP client")?;

    // Fetch latest release
    let release = fetch_latest_release(&client).await?;
    let latest_version = release.tag_name.trim_start_matches('v').trim_start_matches('.');

    println!(
        "  Current version: {}",
        format!("v{}", current_version).yellow()
    );
    println!(
        "  Latest version:  {}",
        format!("v{}", latest_version).green()
    );

    if current_version == latest_version {
        println!("\n{}", "You are already on the latest version!".bright_green());
        return Ok(());
    }

    // Find the right asset for this platform
    let target = current_target();
    if target == "unknown" {
        bail!("Unsupported platform — cannot determine target triple");
    }

    // Build candidate asset names from most specific to least specific.
    // Release assets may use full triples (clide-x86_64-unknown-linux-gnu)
    // or short names (clide-x86_64, clide-aarch64-android).
    let short_target = short_target_name(target);
    let candidates: Vec<String> = {
        let mut v = vec![format!("clide-{}", target)];
        if short_target != target {
            v.push(format!("clide-{}", short_target));
        }
        v
    };

    // Try each candidate name, then its .tar.gz variant
    let mut found: Option<(&ReleaseAsset, bool)> = None;
    for name in &candidates {
        if let Some(a) = find_asset(&release.assets, name) {
            found = Some((a, false));
            break;
        }
        let archive = format!("{}.tar.gz", name);
        if let Some(a) = find_asset(&release.assets, &archive) {
            found = Some((a, true));
            break;
        }
    }

    let (asset, is_archive) = match found {
        Some(pair) => pair,
        None => bail!(
            "No binary for target '{}' in release v{}\nAvailable assets: {}",
            target,
            latest_version,
            release
                .assets
                .iter()
                .map(|a| a.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    };

    println!(
        "\n{} {}...",
        "Downloading".bright_cyan(),
        asset.name.bright_white()
    );

    let binary_data = download_asset(&client, &asset.browser_download_url, is_archive).await?;

    // Replace ourselves
    let exec_path = env::current_exe().context("Cannot determine executable path")?;

    // On Linux (non-Android): stop systemd service before replacing if it's active
    let systemd_active = is_systemd_service_active();
    if systemd_active {
        println!("  {} stopping clide service...", "systemd:".bright_cyan());
        let _ = Command::new("sudo")
            .args(["systemctl", "stop", "clide"])
            .status();
    }

    replace_binary(&exec_path, &binary_data)?;

    println!(
        "\n{}",
        format!("Successfully updated to v{}!", latest_version).bright_green().bold()
    );

    // Print changelog
    if let Some(body) = &release.body {
        let body = body.trim();
        if !body.is_empty() {
            println!("\n{}", "Changelog:".bright_cyan().bold());
            println!("{}", body);
        }
    }

    // On Linux: restart systemd service if it exists, otherwise advise manual restart
    if systemd_active {
        println!("\n  {} restarting clide service...", "systemd:".bright_cyan());
        let status = Command::new("sudo")
            .args(["systemctl", "restart", "clide"])
            .status();
        match status {
            Ok(s) if s.success() => {
                println!("  {}", "Service restarted successfully.".bright_green());
            }
            _ => {
                println!(
                    "  {} Failed to restart service. Run: sudo systemctl restart clide",
                    "Warning:".yellow()
                );
            }
        }
    } else if has_systemd_unit() {
        // Unit file exists but service wasn't running — just let user know
        println!(
            "\n{} Service was not running. Start it with: sudo systemctl start clide",
            "Note:".bright_cyan()
        );
    } else {
        #[cfg(not(target_os = "android"))]
        println!(
            "\n{} Restart clide manually, or install the systemd service:\n  bash scripts/install-service.sh",
            "Note:".bright_cyan()
        );
    }

    Ok(())
}

async fn fetch_latest_release(client: &Client) -> Result<GitHubRelease> {
    let url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        GITHUB_REPO
    );

    let resp = client
        .get(&url)
        .send()
        .await
        .context("Failed to reach GitHub API")?;

    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        bail!("No releases found for {}", GITHUB_REPO);
    }

    if !resp.status().is_success() {
        bail!("GitHub API returned status {}", resp.status());
    }

    resp.json::<GitHubRelease>()
        .await
        .context("Failed to parse release JSON")
}

fn find_asset<'a>(assets: &'a [ReleaseAsset], name: &str) -> Option<&'a ReleaseAsset> {
    assets.iter().find(|a| a.name == name)
}

/// Convert a full Rust target triple to the short name used in release assets.
/// e.g. "x86_64-unknown-linux-gnu" → "x86_64"
///      "aarch64-unknown-linux-gnu" → "aarch64"
///      "aarch64-linux-android"     → "aarch64-android"
fn short_target_name(target: &str) -> &str {
    match target {
        "x86_64-unknown-linux-gnu" => "x86_64",
        "aarch64-unknown-linux-gnu" => "aarch64",
        "x86_64-apple-darwin" => "x86_64-darwin",
        "aarch64-apple-darwin" => "aarch64-darwin",
        "aarch64-linux-android" => "aarch64-android",
        other => other,
    }
}

async fn download_asset(client: &Client, url: &str, is_archive: bool) -> Result<Vec<u8>> {
    let resp = client
        .get(url)
        .send()
        .await
        .context("Failed to download asset")?;

    if !resp.status().is_success() {
        bail!("Download returned status {}", resp.status());
    }

    let bytes = resp.bytes().await.context("Failed to read response body")?;

    if is_archive {
        extract_from_tar_gz(&bytes)
    } else {
        Ok(bytes.to_vec())
    }
}

fn extract_from_tar_gz(data: &[u8]) -> Result<Vec<u8>> {
    use flate2::read::GzDecoder;
    use tar::Archive;

    let decoder = GzDecoder::new(data);
    let mut archive = Archive::new(decoder);

    for entry in archive.entries().context("Failed to read tar entries")? {
        let mut entry = entry.context("Failed to read tar entry")?;
        let path = entry
            .path()
            .context("Failed to read entry path")?
            .to_path_buf();

        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        // Match the clide binary inside the archive
        if file_name == "clide" || file_name.starts_with("clide-") {
            let mut buf = Vec::new();
            std::io::Read::read_to_end(&mut entry, &mut buf)
                .context("Failed to extract binary from archive")?;
            return Ok(buf);
        }
    }

    bail!("clide binary not found in archive");
}

fn replace_binary(path: &std::path::Path, new_binary: &[u8]) -> Result<()> {
    let permissions = std::fs::metadata(path)
        .context("Failed to read current binary metadata")?
        .permissions();

    // Try writing temp file next to the binary first (atomic rename possible).
    // If that fails (e.g. /usr/local/bin/ owned by root), fall back to writing
    // in a user-writable temp dir + sudo mv (atomic rename avoids "Text file busy").
    let tmp_path = path.with_extension("tmp");

    match std::fs::File::create(&tmp_path) {
        Ok(mut file) => {
            file.write_all(new_binary)
                .context("Failed to write new binary")?;
            file.flush()?;
            drop(file);
            std::fs::set_permissions(&tmp_path, permissions.clone())
                .context("Failed to set permissions on new binary")?;
            // rename() is atomic on Linux — replaces the directory entry without
            // opening the old file for writing, so it works on running binaries.
            std::fs::rename(&tmp_path, path)
                .context("Failed to replace binary (rename)")?;
        }
        Err(_) => {
            // Permission denied writing next to the binary — use sudo to create
            // a temp file in the same directory, then atomically rename it.
            // This avoids "Text file busy" which happens with `sudo cp` on a
            // running binary.
            let home = env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            let home_tmp = std::path::PathBuf::from(&home).join(".clide_update_tmp");

            let mut file = std::fs::File::create(&home_tmp)
                .context("Failed to create temp file in home directory")?;
            file.write_all(new_binary)
                .context("Failed to write new binary to temp")?;
            file.flush()?;
            drop(file);

            // Make executable
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&home_tmp, std::fs::Permissions::from_mode(0o755))?;
            }

            let path_str = path.to_string_lossy();
            let tmp_str = home_tmp.to_string_lossy();
            println!(
                "  {} (using sudo to install to {})",
                "Permission required".yellow(),
                path_str
            );

            // Step 1: sudo rm the running binary first.
            // On Linux, rm unlinks the directory entry while the kernel keeps
            // the old inode open for the running process.  This avoids
            // "Text file busy" which both `sudo cp` and `sudo mv` can trigger
            // when overwriting a running binary on some VPS kernels.
            let status = Command::new("sudo")
                .args(["rm", "-f", &path_str])
                .status()
                .context("Failed to run sudo rm")?;

            if !status.success() {
                let _ = std::fs::remove_file(&home_tmp);
                bail!(
                    "sudo rm failed. Try manually:\n  sudo rm -f {}",
                    path_str
                );
            }

            // Step 2: sudo cp the new binary into place (fresh inode).
            let status = Command::new("sudo")
                .args(["cp", &tmp_str, &path_str])
                .status()
                .context("Failed to run sudo cp")?;

            // Clean up home temp regardless of outcome
            let _ = std::fs::remove_file(&home_tmp);

            if !status.success() {
                bail!(
                    "sudo cp failed. Try manually:\n  sudo cp {} {}",
                    tmp_str,
                    path_str
                );
            }

            // Ensure the new binary is executable
            let _ = Command::new("sudo")
                .args(["chmod", "+x", &path_str])
                .status();
        }
    }

    Ok(())
}

/// Check if the clide systemd service is currently active (running).
/// Returns false on non-Linux, Android/Termux, or if systemd is not available.
fn is_systemd_service_active() -> bool {
    #[cfg(target_os = "android")]
    { return false; }

    #[cfg(not(target_os = "android"))]
    {
        Command::new("systemctl")
            .args(["is-active", "clide"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

/// Check if a clide systemd unit file exists (even if service is stopped).
fn has_systemd_unit() -> bool {
    #[cfg(target_os = "android")]
    { return false; }

    #[cfg(not(target_os = "android"))]
    {
        std::path::Path::new("/etc/systemd/system/clide.service").exists()
    }
}

/// Remove leftover artifacts from previous installs/updates.
fn purge_old_artifacts() {
    println!("\n{}", "Purging old artifacts...".bright_cyan());
    let home = env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let home = std::path::PathBuf::from(home);
    let mut removed = 0u32;

    // 1. Stale temp files from previous update attempts
    let temp_files = [
        home.join(".clide_update_tmp"),
        std::path::PathBuf::from("/usr/local/bin/clide.tmp"),
        std::path::PathBuf::from("/usr/local/bin/clide.new"),
    ];
    for path in &temp_files {
        if path.exists() {
            if std::fs::remove_file(path).is_ok() {
                println!("  Removed: {}", path.display());
                removed += 1;
            } else {
                // Try with sudo for protected paths
                let _ = Command::new("sudo")
                    .args(["rm", "-f", &path.to_string_lossy()])
                    .status();
                println!("  Removed (sudo): {}", path.display());
                removed += 1;
            }
        }
    }

    // 2. Old source checkout (Clide_Source from manual build installs)
    let source_dir = home.join("Clide_Source");
    if source_dir.is_dir() {
        println!("  Found old source dir: {}", source_dir.display());
        if std::fs::remove_dir_all(&source_dir).is_ok() {
            println!("  Removed: {}", source_dir.display());
            removed += 1;
        } else {
            println!(
                "  {} Could not remove {}. Run: rm -rf ~/Clide_Source",
                "Warning:".yellow(),
                source_dir.display()
            );
        }
    }

    // 3. Termux: stale binary copies in $PREFIX/bin/ (old names)
    #[cfg(target_os = "android")]
    {
        if let Ok(prefix) = env::var("PREFIX") {
            let old_names = ["clide.tmp", "clide.new", "clide.bak", "clide.old"];
            for name in &old_names {
                let p = std::path::PathBuf::from(&prefix).join("bin").join(name);
                if p.exists() {
                    let _ = std::fs::remove_file(&p);
                    println!("  Removed: {}", p.display());
                    removed += 1;
                }
            }
        }
    }

    // 4. Linux: stale binary copies next to /usr/local/bin/clide
    #[cfg(not(target_os = "android"))]
    {
        let stale = ["clide.bak", "clide.old"];
        for name in &stale {
            let p = std::path::PathBuf::from("/usr/local/bin").join(name);
            if p.exists() {
                let _ = Command::new("sudo")
                    .args(["rm", "-f", &p.to_string_lossy()])
                    .status();
                println!("  Removed: {}", p.display());
                removed += 1;
            }
        }
    }

    if removed == 0 {
        println!("  {}", "No stale artifacts found — all clean.".bright_green());
    } else {
        println!(
            "  {}",
            format!("Cleaned up {} artifact(s).", removed).bright_green()
        );
    }
}
