# Rust CLI Project — Termux + VPS Deployment Guide

**For Claude Code: Read this FIRST before writing any code.**

This guide documents battle-tested patterns for Rust CLI projects that must run on multiple platforms. Every recommendation here comes from a real bug or failed deploy encountered during the development of [Clide](https://github.com/juanitto-maker/Clide).

## Target Platforms

| Target | Platform | Asset name |
|--------|----------|------------|
| `x86_64-unknown-linux-gnu` | Linux VPS (Ubuntu) | `appname-x86_64` |
| `aarch64-unknown-linux-gnu` | ARM Linux VPS | `appname-aarch64` |
| `aarch64-linux-android` | Termux (Android) | `appname-aarch64-android` |

---

## 1. VERSION MANAGEMENT — SINGLE SOURCE OF TRUTH

**Rule: Git tags are the ONLY source of truth for versions.**

NEVER rely on `Cargo.toml` as the authoritative version. CI must derive the version from the git tag and inject it into `Cargo.toml` before building.

### Why this matters

We shipped a release where `Cargo.toml` said `0.2.1` while git tags had reached `v0.4.3`. The auto-release CI read `Cargo.toml`, bumped to `0.2.2`, and published it as "latest" — overriding v0.4.3. Users running `appname update` downgraded themselves.

### Version injection in CI

Before every build step, overwrite `Cargo.toml` from the tag:

```yaml
- name: Set version in Cargo.toml
  run: |
    TAG_VERSION="${GITHUB_REF_NAME#v}"  # strips "v" prefix → "0.4.3"
    sed -i "s/^version = .*/version = \"${TAG_VERSION}\"/" Cargo.toml
    # CRITICAL: strip Windows-style line endings — they silently corrupt
    # the version string (e.g. "0.2.1\r" instead of "0.2.1") and cause
    # downstream regex validation to fail.
    sed -i 's/\r//' Cargo.toml
```

After building, commit `Cargo.toml` back to main so it stays in sync:

```yaml
- name: Update Cargo.toml on main
  run: |
    git config user.name "github-actions[bot]"
    git config user.email "github-actions[bot]@users.noreply.github.com"
    git add Cargo.toml
    git diff --staged --quiet || {
      git commit -m "[release] update Cargo.toml to ${TAG_VERSION}"
      git push origin main
    }
```

### Tag format: handle legacy prefixes

Some repos accumulate tags in mixed formats (`v0.4.3`, `v.0.4.3`). When reading tags in CI, normalize both:

```bash
# Strips "v" or "v." prefix, filters to valid semver, takes highest
CURRENT=$(git tag -l 'v*' | sed 's/^v\.*//;' | grep -E '^[0-9]+\.[0-9]+\.[0-9]+$' | sort -V | tail -1)
```

In Rust update code, strip the same way:

```rust
let latest = tag.trim_start_matches('v').trim_start_matches('.');
```

### Release trigger strategies

**Option A: Manual tags only (recommended for small teams)**

```yaml
on:
  push:
    tags:
      - 'v[0-9]+.[0-9]+.[0-9]+'
```

Release flow: commit → push → go to GitHub Releases → create tag → publish → CI triggers.

**Option B: Auto-release on every push to main**

Viable once the version-from-tags foundation is solid. Requires two safeguards:

1. **Skip mechanism** — commits containing `[skip release]` don't trigger a build.
2. **Concurrency guard** — prevents overlapping runs from clobbering each other:

```yaml
on:
  push:
    branches: [main]

concurrency:
  group: auto-release
  cancel-in-progress: false
```

The CI job reads the highest git tag, bumps the patch, creates the new tag, builds, and publishes. The `[release] update Cargo.toml` commit from step above must also be excluded from triggering another release (infinite loop).

```yaml
- id: check
  run: |
    MSG="${{ github.event.head_commit.message }}"
    if echo "$MSG" | grep -qiE '\[skip release\]|\[release\] update Cargo\.toml'; then
      echo "should_release=false" >> "$GITHUB_OUTPUT"
    else
      echo "should_release=true" >> "$GITHUB_OUTPUT"
    fi
```

---

## 2. GITHUB ACTIONS — CROSS-COMPILATION

### The key lesson: don't use `cross` where you don't need it

`cross` (the Docker-based cross-compiler) is fragile. Installing from git HEAD (`cargo install cross --git ...`) broke our builds with metadata corruption. Use it **only** for targets that genuinely need it (Android). For everything else, use native toolchains.

### Build matrix

```yaml
jobs:
  build:
    runs-on: ubuntu-latest
    strategy:
      fail-fast: false    # Don't cancel other targets if one fails
      matrix:
        include:
          - target: x86_64-unknown-linux-gnu
            artifact: appname-x86_64
            use_cross: false
          - target: aarch64-unknown-linux-gnu
            artifact: appname-aarch64
            use_cross: false          # Native toolchain, NOT cross
          - target: aarch64-linux-android
            artifact: appname-aarch64-android
            use_cross: true           # Only Android needs cross
```

### Build steps per target

**x86_64 (native) — just `cargo build`:**

```yaml
- name: Build (native x86_64)
  if: matrix.target == 'x86_64-unknown-linux-gnu'
  run: cargo build --release --target ${{ matrix.target }}
```

**aarch64-gnu — native cross-compiler, NOT Docker `cross`:**

```yaml
- name: Install cross-compilation tools (aarch64-gnu)
  if: matrix.target == 'aarch64-unknown-linux-gnu'
  run: |
    sudo apt-get update
    sudo apt-get install -y gcc-aarch64-linux-gnu g++-aarch64-linux-gnu

- name: Build (aarch64-gnu)
  if: matrix.target == 'aarch64-unknown-linux-gnu'
  env:
    CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER: aarch64-linux-gnu-gcc
  run: cargo build --release --target ${{ matrix.target }}
```

**Android — `cross` with pinned version + edge Docker image:**

```yaml
- name: Install cross (Android only)
  if: matrix.target == 'aarch64-linux-android'
  run: cargo install cross --version 0.2.5   # Pin version, never install from git HEAD

- name: Build (Android via cross)
  if: matrix.target == 'aarch64-linux-android'
  run: cross build --release --target ${{ matrix.target }}
```

### Cross.toml — fix Android NDK missing `libunwind`

The default `cross` Docker image ships an old Android NDK that lacks `libunwind`, causing `ld: cannot find -lunwind`. Fix by pointing to the edge image:

```toml
# Cross.toml (repo root)
[target.aarch64-linux-android]
image = "ghcr.io/cross-rs/aarch64-linux-android:edge"
```

### Release job

```yaml
release:
  needs: [bump-version, build]
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
      with:
        fetch-depth: 0

    - uses: actions/download-artifact@v4
      with:
        path: artifacts
        merge-multiple: true

    - name: Create tag and release
      run: |
        git config user.name "github-actions[bot]"
        git config user.email "github-actions[bot]@users.noreply.github.com"
        git tag "${{ needs.bump-version.outputs.new_tag }}"
        git push origin "${{ needs.bump-version.outputs.new_tag }}"

    - uses: softprops/action-gh-release@v2
      with:
        tag_name: ${{ needs.bump-version.outputs.new_tag }}
        files: artifacts/*
        generate_release_notes: true
```

---

## 3. SELF-UPDATE COMMAND — `appname update`

### Architecture detection

```rust
fn get_asset_name() -> &'static str {
    #[cfg(target_os = "android")]
    return "appname-aarch64-android";

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return "appname-x86_64";

    #[cfg(all(target_os = "linux", target_arch = "aarch64", not(target_os = "android")))]
    return "appname-aarch64";
}
```

### Asset name resolution with fallback

Release assets may use full triples (`appname-x86_64-unknown-linux-gnu`) or short names (`appname-x86_64`). If asset names ever drift, the update silently fails. Build a candidate list:

```rust
fn short_target_name(target: &str) -> &str {
    match target {
        "x86_64-unknown-linux-gnu"  => "x86_64",
        "aarch64-unknown-linux-gnu" => "aarch64",
        "aarch64-linux-android"     => "aarch64-android",
        other => other,
    }
}

// Try: full triple → short name → each with .tar.gz
let candidates = vec![
    format!("appname-{}", full_target),
    format!("appname-{}", short_target_name(full_target)),
];
for name in &candidates {
    if let Some(asset) = find_asset(&release.assets, name) { return Ok(asset); }
    let archive = format!("{}.tar.gz", name);
    if let Some(asset) = find_asset(&release.assets, &archive) { return Ok(asset); }
}

// On failure, list available assets so the user can diagnose:
bail!(
    "No binary for '{}' in release. Available: {}",
    target,
    release.assets.iter().map(|a| &a.name).collect::<Vec<_>>().join(", ")
);
```

### Version comparison

```rust
let latest = tag.trim_start_matches('v').trim_start_matches('.');
let current = env!("CARGO_PKG_VERSION");
if latest == current {
    println!("Already on latest version!");
    return Ok(());
}
```

### Binary replacement — THE CRITICAL PART

**NEVER use `cp` to overwrite a running binary.** On Linux this causes `ETXTBSY` ("Text file busy"). Even `mv` can trigger it on some VPS kernels.

The two-tier strategy that actually works:

```rust
fn replace_binary(exec_path: &Path, new_binary: &[u8]) -> Result<()> {
    let tmp_path = exec_path.with_extension("tmp");

    // TIER 1: Write temp file next to binary, then atomic rename.
    // Works when you own the directory (Termux, local installs).
    // rename() replaces the directory entry without opening the old file
    // for writing, so it works on running binaries.
    match std::fs::File::create(&tmp_path) {
        Ok(mut file) => {
            file.write_all(new_binary)?;
            file.flush()?;
            drop(file);
            set_executable(&tmp_path)?;
            std::fs::rename(&tmp_path, exec_path)?;
            return Ok(());
        }
        Err(_) => { /* Permission denied — fall through to tier 2 */ }
    }

    // TIER 2: Binary is in a root-owned path (e.g. /usr/local/bin/).
    // Write to $HOME temp dir, then sudo rm + sudo cp.
    let home = std::env::var("HOME").unwrap_or("/tmp".into());
    let home_tmp = PathBuf::from(&home).join(".appname_update_tmp");

    let mut file = std::fs::File::create(&home_tmp)?;
    file.write_all(new_binary)?;
    file.flush()?;
    drop(file);
    set_executable(&home_tmp)?;

    // Step 1: sudo rm the running binary.
    // rm unlinks the directory entry while the kernel keeps the old inode
    // open for the running process. This is the key to avoiding ETXTBSY.
    let status = Command::new("sudo")
        .args(["rm", "-f", &exec_path.to_string_lossy()])
        .status()?;
    if !status.success() {
        let _ = std::fs::remove_file(&home_tmp);
        bail!("sudo rm failed. Try: sudo rm -f {}", exec_path.display());
    }

    // Step 2: sudo cp into the now-vacant path (fresh inode).
    let status = Command::new("sudo")
        .args(["cp", &home_tmp.to_string_lossy(), &exec_path.to_string_lossy()])
        .status()?;
    let _ = std::fs::remove_file(&home_tmp); // always clean up
    if !status.success() {
        bail!("sudo cp failed");
    }

    let _ = Command::new("sudo").args(["chmod", "+x", &exec_path.to_string_lossy()]).status();
    Ok(())
}

#[cfg(unix)]
fn set_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))?;
    Ok(())
}
```

### systemd integration: stop BEFORE replace, restart AFTER

Order matters. If you replace while the service is running, the new binary might not load cleanly.

```rust
// BEFORE downloading/replacing:
let systemd_active = is_systemd_service_active();
if systemd_active {
    let _ = Command::new("sudo").args(["systemctl", "stop", "appname"]).status();
}

// ... replace binary ...

// AFTER replacing:
if systemd_active {
    let _ = Command::new("sudo").args(["systemctl", "restart", "appname"]).status();
}
```

```rust
fn is_systemd_service_active() -> bool {
    #[cfg(target_os = "android")]
    { return false; }   // Termux has no systemd

    #[cfg(not(target_os = "android"))]
    {
        Command::new("systemctl")
            .args(["is-active", "appname"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}
```

### Error handling for GitHub API

Handle common failure modes that will happen in production:

```rust
async fn fetch_latest_release(client: &Client) -> Result<GitHubRelease> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", REPO);
    let resp = client.get(&url).send().await?;

    match resp.status() {
        s if s == StatusCode::NOT_FOUND => bail!("No releases found for {}", REPO),
        s if s == StatusCode::FORBIDDEN  => bail!("GitHub API rate limit exceeded. Wait a few minutes and retry."),
        s if !s.is_success()             => bail!("GitHub API returned status {}", s),
        _ => {}
    }

    resp.json::<GitHubRelease>().await.context("Failed to parse release JSON")
}
```

### Purge stale artifacts

Update mechanisms that download temp files should have a cleanup command (`appname update --purge`):

```rust
fn purge_old_artifacts() {
    let home = std::env::var("HOME").unwrap_or("/tmp".into());
    let temps = [
        PathBuf::from(&home).join(".appname_update_tmp"),
        PathBuf::from("/usr/local/bin/appname.tmp"),
        PathBuf::from("/usr/local/bin/appname.new"),
    ];
    for path in &temps {
        if path.exists() {
            let _ = std::fs::remove_file(path)
                .or_else(|_| Command::new("sudo").args(["rm", "-f", &path.to_string_lossy()]).status().map(|_| ()));
        }
    }
}
```

---

## 4. FIRST-TIME INSTALLATION — `install.sh`

The guide is incomplete without covering how users get the binary in the first place. The install script must handle the same platform detection challenges as the update code.

### Platform detection

```bash
if [[ "$PREFIX" =~ "com.termux" ]]; then
    PLATFORM="termux"
    BIN_DIR="$PREFIX/bin"
elif [[ "$(uname -s)" == "Linux" ]]; then
    PLATFORM="linux"
    BIN_DIR="/usr/local/bin"
else
    echo "Unsupported platform: $(uname -s)"
    exit 1
fi
```

### Bootstrap dependencies on fresh VPS

A fresh Ubuntu VPS often lacks `curl`, `git`, even `wget`. The install script must handle this:

```bash
if [ "$PLATFORM" = "linux" ]; then
    echo "Installing bootstrap dependencies..."
    sudo apt-get update -qq 2>/dev/null
    sudo apt-get install -y -qq curl wget git build-essential 2>/dev/null
fi
```

### Download correct binary per architecture

```bash
ARCH=$(uname -m)
case "$PLATFORM-$ARCH" in
    termux-aarch64)  ASSET="appname-aarch64-android" ;;
    linux-x86_64)    ASSET="appname-x86_64" ;;
    linux-aarch64)   ASSET="appname-aarch64" ;;
    *)
        echo "Unsupported: $PLATFORM $ARCH"
        exit 1
        ;;
esac

curl -fSL "https://github.com/USER/REPO/releases/latest/download/$ASSET" \
    -o "$BIN_DIR/appname"
chmod +x "$BIN_DIR/appname"
```

### One-liner install

```bash
curl -fsSL https://raw.githubusercontent.com/USER/REPO/main/install.sh | bash
```

When piped through `curl | bash`, interactive prompts must read from `/dev/tty`:

```bash
ask() {
    local prompt="$1" varname="$2"
    printf "%s" "$prompt" >/dev/tty
    IFS= read -r answer </dev/tty
    eval "$varname=\"\$answer\""
}
```

---

## 5. SYSTEMD SERVICE — VPS SETUP

Create `scripts/install-service.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail

APP_NAME="appname"
APP_USER="${CLIDE_USER:-$(logname 2>/dev/null || echo "${SUDO_USER:-root}")}"
BINARY_PATH="/usr/local/bin/$APP_NAME"
SERVICE_FILE="/etc/systemd/system/$APP_NAME.service"

# Require root
if [ "$(id -u)" -ne 0 ]; then
    echo "Error: run with sudo"
    exit 1
fi

# Stop existing service if running
if systemctl is-active "$APP_NAME" >/dev/null 2>&1; then
    echo "Stopping existing service..."
    systemctl stop "$APP_NAME"
fi

# Write unit file
cat > "$SERVICE_FILE" <<EOF
[Unit]
Description=$APP_NAME Bot
After=network.target

[Service]
User=$APP_USER
ExecStart=$BINARY_PATH bot
Restart=always
RestartSec=5
Environment=HOME=/home/$APP_USER

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable "$APP_NAME"
systemctl start "$APP_NAME"

echo "Done! Commands:"
echo "  sudo systemctl status $APP_NAME"
echo "  sudo systemctl restart $APP_NAME"
echo "  sudo journalctl -u $APP_NAME -f"
```

---

## 6. TERMUX SPECIFICS

- **No systemd** — update is simpler: replace binary, kill old process, start fresh.
- **No sudo** — binary is user-owned, so direct write/rename works (Tier 1 in update code).
- **Binary location**: `/data/data/com.termux/files/usr/bin/appname`

Kill and restart pattern:

```bash
pkill appname && appname bot &
```

Manual install from release:

```bash
curl -L https://github.com/USER/REPO/releases/latest/download/appname-aarch64-android \
  -o $PREFIX/bin/appname \
  && chmod +x $PREFIX/bin/appname
```

Detect Termux in Rust:

```rust
// Compile-time: #[cfg(target_os = "android")]
// Runtime: std::env::var("TERMUX_VERSION").is_ok()
```

---

## 7. COMMON PITFALLS & SOLUTIONS

Every row below is a real bug we hit and fixed.

| Problem | Root Cause | Fix |
|---------|-----------|-----|
| `Text file busy` (ETXTBSY) | Overwriting a running binary with `cp` | `rm` first to unlink inode, then `cp` to fresh inode |
| `appname update` says "already latest" | Tag format `v.0.4.x` not fully stripped | `.trim_start_matches('v').trim_start_matches('.')` |
| Binary reports wrong version | `Cargo.toml` not updated before build | `sed -i` Cargo.toml from tag in CI, before `cargo build` |
| CI creates wrong version (0.2.x instead of 0.4.x) | Auto-bump reads Cargo.toml instead of git tags | Read highest semver tag: `git tag -l 'v*' \| sed ... \| sort -V \| tail -1` |
| Version string is `"0.2.1\r"` — regex fails | Windows line endings in Cargo.toml | `sed -i 's/\r//' Cargo.toml` after version injection |
| `cross` metadata corruption | Installed from git HEAD | Pin version: `cargo install cross --version 0.2.5` |
| `ld: cannot find -lunwind` (Android) | Old NDK in default cross Docker image | `Cross.toml` → `ghcr.io/cross-rs/aarch64-linux-android:edge` |
| aarch64-gnu build fails | Docker `cross` broken for GNU target | Use native `gcc-aarch64-linux-gnu` + `cargo build` instead |
| Asset not found in release | Short name vs full triple mismatch | Try multiple candidate names with fallback |
| `systemctl` not found | On Termux, not VPS | Guard with `#[cfg(target_os = "android")]` |
| `sudo` not found | On Termux | Same guard — Termux needs no sudo |
| Permission denied writing binary | Binary at `/usr/local/bin/` owned by root | Two-tier: try direct write → fall back to `$HOME` tmp + sudo |
| Overlapping CI releases clobber each other | No concurrency control | `concurrency: { group: auto-release, cancel-in-progress: false }` |
| Install script fails on fresh VPS | Missing `curl`, `git`, `build-essential` | Bootstrap `apt-get install` before anything else |
| GitHub API 403 during update | Rate limiting | Check status code, show meaningful error message |
| Stale temp files after failed updates | No cleanup mechanism | `appname update --purge` command |

---

## 8. CHECKLIST FOR NEW RUST CLI PROJECT

```
[ ] Cargo.toml version starts at 0.1.0 (will be overwritten by CI)
[ ] .github/workflows/release.yml with tag trigger (or auto-release with guards)
[ ] Concurrency group in workflow to prevent overlapping runs
[ ] Version injected from git tag via sed, with \r stripping
[ ] Cross.toml for Android NDK edge image
[ ] aarch64-gnu uses native gcc toolchain, NOT Docker cross
[ ] cross pinned to specific version for Android builds only
[ ] fail-fast: false in build matrix
[ ] src/update.rs: two-tier binary replacement (rename or rm+cp)
[ ] src/update.rs: stop systemd BEFORE replace, restart AFTER
[ ] src/update.rs: asset name fallback (full triple → short name)
[ ] src/update.rs: strip both 'v' and '.' from version tags
[ ] src/update.rs: handle GitHub API rate limiting
[ ] src/update.rs: --purge flag for stale artifact cleanup
[ ] install.sh: platform detection (Termux → Linux → unsupported)
[ ] install.sh: bootstrap apt packages on fresh VPS
[ ] install.sh: correct binary asset per architecture
[ ] scripts/install-service.sh for VPS systemd setup
[ ] README documents: how to release, how to install, how to update
[ ] First release tested: appname update on both Termux and VPS
```

---

## 9. DEVELOPER WORKFLOW SUMMARY

```bash
# Daily development (no CI triggered):
edit code → git add → git commit → git push

# When ready to release:
#   Option A (manual): GitHub.com → Releases → New → tag v0.5.0 → Publish
#   Option B (auto): just push to main — CI auto-bumps patch version
#     Skip with: git commit -m "docs: update readme [skip release]"

# Wait ~4 minutes for CI to build all 3 targets

# On VPS:
appname update
# → stops systemd service, downloads binary, replaces via rm+cp, restarts service

# On Termux:
appname update
# → downloads binary, replaces via atomic rename, done

# Verify:
appname --version
```

---

## 10. FILE STRUCTURE REFERENCE

```
project/
├── Cargo.toml                    # Version overwritten by CI
├── Cross.toml                    # Android NDK edge image
├── install.sh                    # First-time installer (curl | bash)
├── scripts/
│   └── install-service.sh        # One-time systemd setup for VPS
├── src/
│   ├── main.rs
│   └── update.rs                 # Self-update logic
└── .github/
    └── workflows/
        └── auto-release.yml      # Build + release pipeline
```
