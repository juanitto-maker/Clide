// ============================================
// update.rs - Self-Update from GitHub Releases
// ============================================

use anyhow::{bail, Context, Result};
use colored::*;
use reqwest::Client;
use serde::Deserialize;
use std::env;
use std::io::Write;

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
pub async fn run() -> Result<()> {
    let current_version = crate::VERSION;
    println!("{}", "Checking for updates...".bright_cyan());

    let client = Client::builder()
        .user_agent(format!("clide/{}", current_version))
        .build()
        .context("Failed to create HTTP client")?;

    // Fetch latest release
    let release = fetch_latest_release(&client).await?;
    let latest_version = release.tag_name.trim_start_matches('v');

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

    // Assets are named like: clide-x86_64-unknown-linux-gnu or .tar.gz
    let binary_name = format!("clide-{}", target);
    let archive_name = format!("{}.tar.gz", binary_name);

    // Prefer the raw binary, fall back to archive
    let (asset, is_archive) = if let Some(a) = find_asset(&release.assets, &binary_name) {
        (a, false)
    } else if let Some(a) = find_asset(&release.assets, &archive_name) {
        (a, true)
    } else {
        bail!(
            "No binary for target '{}' in release v{}\nAvailable assets: {}",
            target,
            latest_version,
            release
                .assets
                .iter()
                .map(|a| a.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    };

    println!(
        "\n{} {}...",
        "Downloading".bright_cyan(),
        asset.name.bright_white()
    );

    let binary_data = download_asset(&client, &asset.browser_download_url, is_archive).await?;

    // Replace ourselves
    let exec_path = env::current_exe().context("Cannot determine executable path")?;
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

    // Write to temp file next to current binary, then atomic rename
    let tmp_path = path.with_extension("tmp");

    let mut file = std::fs::File::create(&tmp_path)
        .context("Failed to create temp file for update")?;
    file.write_all(new_binary)
        .context("Failed to write new binary")?;
    file.flush()?;
    drop(file);

    // Preserve permissions
    std::fs::set_permissions(&tmp_path, permissions)
        .context("Failed to set permissions on new binary")?;

    // Atomic rename
    std::fs::rename(&tmp_path, path).context("Failed to replace binary (rename)")?;

    Ok(())
}
