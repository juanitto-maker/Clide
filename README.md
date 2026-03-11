<div align="center">

# Clide

### Glide through your CLI — autonomous terminal operations via AI

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange?logo=rust)](https://www.rust-lang.org)
[![Version](https://img.shields.io/badge/version-0.3.0-blue)](CHANGELOG.md)
[![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20macOS%20%7C%20Android-green)](docs/INSTALL.md)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](docs/CONTRIBUTING.md)
[![Discussions](https://img.shields.io/badge/GitHub-Discussions-purple?logo=github)](https://github.com/juanitto-maker/Clide/discussions)

</div>

---

> **Clide** is a Rust-based autonomous terminal agent. Send natural language commands from Element/Matrix or Telegram — Clide interprets them with Gemini AI, executes them on your machine or over SSH, and replies with the results. Runs as a static binary with zero runtime dependencies.

---

## Contents

- [Features](#features)
- [Installation](#installation)
  - [Fresh Linux — first steps](#fresh-linux--first-steps)
  - [One-liner install](#one-liner-install)
  - [Manual binary install](#manual-binary-install)
  - [Build from source](#build-from-source)
- [Maintenance](#maintenance)
  - [Update (code only)](#update-code-only)
  - [Full purge](#full-purge)
  - [Purge and clean reinstall](#purge-and-clean-reinstall)
- [Configuration](#configuration)
- [Credential Manager](#credential-manager)
  - [Secrets file](#secrets-file)
  - [clide secret CLI](#clide-secret-cli)
  - [GPG layer via GNU pass](#gpg-layer-via-gnu-pass)
  - [SSH host registry](#ssh-host-registry)
  - [Vault backup & restore](#vault-backup--restore)
- [Secrets & Credentials](#secrets--credentials)
- [Usage](#usage)
- [Architecture](#architecture)
- [Security](#security)
- [Platform Support](#platform-support)
- [Development](#development)
- [Contributing](#contributing)
- [Community](#community)
- [Donate](#donate)
- [License](#license)

---

## Features

| | Feature | Description |
|---|---|---|
| AI | **Gemini AI** | Natural language command interpretation |
| Chat | **Element/Matrix** | E2E-encrypted control via Matrix rooms |
| Chat | **Telegram** | Bot via @BotFather — easiest setup |
| Speed | **Single binary** | No Java, no Node.js, no signal-cli, no Python |
| SSH | **Remote ops** | Manage servers over SSH from your phone |
| Safety | **Allowlist** | Authorize users, block dangerous commands |
| Logging | **Structured** | Colorful, timestamped audit log |
| Skills | **YAML workflows** | Reusable automations with parameter injection |
| Secrets | **Credential manager** | Store API keys and tokens; optional GPG layer via GNU pass |
| Vault | **Encrypted backup** | Back up secrets and SSH hosts to GitHub Gist with age encryption |
| Hosts | **SSH host registry** | Named hosts injected into skills — IPs never sent to AI |
| Privacy | **Secret scrubber** | All outbound text auto-redacted before reaching AI or chat |

---

## Installation

### Fresh Linux — first steps

> **If this is a new or freshly imaged Linux system, always update and upgrade before running any one-liner install.** Package lists and base tools are often stale on fresh images, which can cause curl or wget to fail with certificate errors, broken dependencies, or missing tools.

#### 1. Fix your package mirrors (if needed)

On some cloud VPS images — especially from providers in mainland China (Alibaba Cloud, Tencent Cloud, Huawei Cloud) or distros that shipped with regional mirrors — your `/etc/apt/sources.list` may point to slow or unreliable mirrors. Switch to official ones **before** running apt:

**Ubuntu:**
```bash
# Check current mirrors
grep -v '^#' /etc/apt/sources.list | grep -v '^$'

# Switch to official Ubuntu mirrors (replaces cn.archive / other regionals)
sudo sed -i \
  -e 's|http://cn.archive.ubuntu.com/ubuntu|http://archive.ubuntu.com/ubuntu|g' \
  -e 's|http://[a-z]*\.archive\.ubuntu\.com/ubuntu|http://archive.ubuntu.com/ubuntu|g' \
  /etc/apt/sources.list
```

**Debian:**
```bash
# Switch to official Debian mirrors
sudo sed -i \
  -e 's|http://ftp\.[a-z]*\.debian\.org/debian|http://deb.debian.org/debian|g' \
  -e 's|http://[a-z]*\.debian\.org/debian|http://deb.debian.org/debian|g' \
  /etc/apt/sources.list
```

> **Termux users:** Run `termux-change-repo` to pick your preferred mirror interactively. Avoid TUNA or BFSU if you are outside China.

#### 2. Update and upgrade

```bash
# Debian / Ubuntu / WSL2
sudo apt update && sudo apt upgrade -y

# Then install curl if not present
sudo apt install -y curl wget

# Termux
pkg update && pkg upgrade -y && pkg install curl wget
```

> Only after these two steps should you run the one-liner installer below. Skipping them on a fresh system is the most common cause of install failures.

---

### One-liner install

**Linux / macOS:**
```bash
curl -fsSL https://raw.githubusercontent.com/juanitto-maker/Clide/main/install.sh | bash
```

**Termux (Android):**
```bash
curl -fsSL https://raw.githubusercontent.com/juanitto-maker/Clide/main/install.sh | bash
```

The installer will:
1. Download the pre-built binary for your architecture (if a release exists)
2. Fall back to building from source if no binary is available
3. Create `~/.clide/config.yaml` and `~/.config/clide/config.env`
4. Walk you through Gemini API key and messaging platform setup (all steps skippable)
5. Install bundled skills to `~/.clide/skills/`

---

### Manual binary install

**Linux x86_64 (Intel/AMD):**
```bash
wget https://github.com/juanitto-maker/Clide/releases/latest/download/clide-x86_64 -O clide
chmod +x clide
sudo mv clide /usr/local/bin/clide
```

**Linux ARM64 (Raspberry Pi, ARM servers):**
```bash
wget https://github.com/juanitto-maker/Clide/releases/latest/download/clide-aarch64 -O clide
chmod +x clide
sudo mv clide /usr/local/bin/clide
```

**macOS (Apple Silicon / M-series):**
```bash
curl -L https://github.com/juanitto-maker/Clide/releases/latest/download/clide-aarch64-darwin -o clide
chmod +x clide
sudo mv clide /usr/local/bin/clide
```

**macOS (Intel):**
```bash
curl -L https://github.com/juanitto-maker/Clide/releases/latest/download/clide-x86_64-darwin -o clide
chmod +x clide
sudo mv clide /usr/local/bin/clide
```

**Termux (Android ARM64):**
```bash
wget https://github.com/juanitto-maker/Clide/releases/latest/download/clide-aarch64 -O "$PREFIX/bin/clide"
chmod +x "$PREFIX/bin/clide"
```

---

### Build from source

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Clone and build
git clone https://github.com/juanitto-maker/Clide.git
cd Clide
cargo build --release

# Install binary
sudo cp target/release/clide /usr/local/bin/   # Linux / macOS
cp target/release/clide "$PREFIX/bin/"          # Termux
```

Verify:
```bash
clide --version
```

---

## Maintenance

### Update (code only)

Updates Clide's binary to the latest release **without** touching your config, secrets, or reinstalling Rust.

**Via installer (recommended for Termux):**
```bash
curl -fsSL https://raw.githubusercontent.com/juanitto-maker/Clide/main/install.sh | bash
```

**Via binary download (Linux/macOS):**
```bash
# x86_64
wget https://github.com/juanitto-maker/Clide/releases/latest/download/clide-x86_64 -O /tmp/clide \
  && chmod +x /tmp/clide && sudo mv /tmp/clide /usr/local/bin/clide

# ARM64
wget https://github.com/juanitto-maker/Clide/releases/latest/download/clide-aarch64 -O /tmp/clide \
  && chmod +x /tmp/clide && sudo mv /tmp/clide /usr/local/bin/clide
```

**From source (code update only, no Rust reinstall):**
```bash
cd ~/Clide_Source           # or wherever you cloned the repo
git pull origin main
cargo build --release
sudo cp target/release/clide /usr/local/bin/clide   # Linux/macOS
cp target/release/clide "$PREFIX/bin/clide"          # Termux
```

> Your `~/.clide/config.yaml`, `~/.clide/secrets.yaml`, and all skill files are **not touched** by any of the above.

---

### Full purge

Removes the binary and all Clide data. Rust itself is **not** removed.

**Linux / macOS:**
```bash
sudo rm -f /usr/local/bin/clide
rm -rf ~/.clide
rm -rf ~/.config/clide
```

**Termux:**
```bash
rm -f "$PREFIX/bin/clide"
rm -rf "$HOME/Clide_Source"
rm -rf ~/.clide
rm -rf ~/.config/clide
```

> After a full purge, re-running the one-liner installer will set everything up fresh — including a new config wizard.

---

### Purge and clean reinstall

Run the full purge above, then immediately reinstall:

**Linux / macOS:**
```bash
# 1. Purge
sudo rm -f /usr/local/bin/clide && rm -rf ~/.clide ~/.config/clide

# 2. Reinstall (update apt first if on a fresh system)
curl -fsSL https://raw.githubusercontent.com/juanitto-maker/Clide/main/install.sh | bash
```

**Termux:**
```bash
# 1. Purge
rm -f "$PREFIX/bin/clide" && rm -rf "$HOME/Clide_Source" ~/.clide ~/.config/clide

# 2. Reinstall
curl -fsSL https://raw.githubusercontent.com/juanitto-maker/Clide/main/install.sh | bash
```

> If you also want to remove Rust (only useful if you're doing a total environment reset):
> ```bash
> rustup self uninstall   # removes rustup, cargo, rustc
> ```

---

## Configuration

1. **The installer creates your config automatically.** To create it manually:
```bash
mkdir -p ~/.clide
cp /path/to/Clide/config.example.yaml ~/.clide/config.yaml
chmod 600 ~/.clide/config.yaml
```

2. **Edit the config:**
```bash
nano ~/.clide/config.yaml
```

Minimal required fields:
```yaml
gemini_api_key: "YOUR_GEMINI_API_KEY"

matrix_homeserver: "https://matrix.org"
matrix_user: "@yourbot:matrix.org"
matrix_access_token: "syt_..."
matrix_room_id: "!abc123:matrix.org"

authorized_users:
  - "@youraccount:matrix.org"
```

3. **Get a Matrix access token:**
   - Via installer: enter your account password when prompted — the token is fetched automatically
   - Via Element: Settings → Help & About → Access Token (click to reveal)
   - Via API:
     ```bash
     curl -XPOST https://matrix.org/_matrix/client/v3/login \
       -H "Content-Type: application/json" \
       -d '{"type":"m.login.password","identifier":{"type":"m.id.user","user":"mybot"},"password":"yourpassword"}'
     # Copy "access_token" from the JSON response (looks like "syt_...")
     ```

4. **Find your room ID:** Element → open the room → Settings → Advanced → Internal room ID
   Format: `!abc123:matrix.org`

5. **Get a Gemini API key:** [aistudio.google.com/app/apikey](https://aistudio.google.com/app/apikey) — free tier available.

See [`config.example.yaml`](config.example.yaml) for all available options.

---

## Credential Manager

Clide is not just a terminal bot — it is also a **credential manager** for all the secrets your automations need. Think of it like a lightweight, local alternative to tools like KeePass or `pass`: it stores your API keys, tokens, and server passwords in one place, keeps them out of chat transcripts and AI prompts, and makes them available to your skills at execution time.

There are three storage options, from simplest to most secure:

| Option | At-rest protection | Best for |
|---|---|---|
| `secrets.yaml` | File permissions (`chmod 600`) | Most users — simple and effective |
| GNU pass layer | GPG encryption | High-security setups, shared machines |
| Environment variables | Process memory only | CI/CD, Docker, ephemeral environments |

---

### Secrets file

All credentials live in `~/.clide/secrets.yaml`, separate from the main config:

```yaml
# AI providers
GEMINI_API_KEY: "AIzaSy..."
ANTHROPIC_API_KEY: "sk-ant-..."

# Messaging
MATRIX_ACCESS_TOKEN: "syt_..."
TELEGRAM_BOT_TOKEN: "123456789:ABC..."

# GitHub (for vault backups)
GITHUB_TOKEN: "ghp_..."

# Your own secrets — add anything skills need
MY_VPS_ROOT_PASSWORD: "hunter2"
MY_DISCORD_WEBHOOK: "https://discord.com/api/webhooks/..."
```

Any key defined here is automatically available as `${KEY_NAME}` in skill commands. **The AI never sees the real values** — substitution happens inside the Executor only after the AI has finished generating the command string.

---

### clide secret CLI

Clide ships a dedicated sub-command to manage the secrets file interactively:

```bash
# List all stored key names (values never printed)
clide secret list

# Print a single value (resolves pass references too)
clide secret get GEMINI_API_KEY

# Store a new secret (hidden input, choose yaml or pass storage)
clide secret set MY_WEBHOOK_URL

# Generate a random credential and store it
clide secret generate DB_PASSWORD 32   # 32 chars, default if omitted

# Set up the optional GPG encryption layer
clide secret pass-init

# Move an existing YAML secret into the GPG store
clide secret pass-set GEMINI_API_KEY
```

---

### GPG layer via GNU pass

For an extra layer of encryption, Clide integrates with [GNU pass](https://www.passwordstore.org/) — the standard UNIX password manager. Secrets stored in pass are GPG-encrypted at rest inside `~/.password-store/`.

**Setup:**
```bash
# 1. Install dependencies (Termux)
pkg install gnupg pass

# 2. Run the guided wizard
clide secret pass-init
# → lists existing GPG keys, lets you create one, then calls `pass init`

# 3. Store a secret via pass
clide secret set GEMINI_API_KEY
# → when prompted, choose option 2 (pass/GPG-encrypted)

# 4. Or migrate an existing YAML secret to pass
clide secret pass-set GEMINI_API_KEY
```

After setup, `secrets.yaml` holds a reference instead of the real value:
```yaml
GEMINI_API_KEY: "pass:clide/gemini_api_key"
```
Clide resolves this transparently at startup by calling `pass show clide/gemini_api_key`.

---

### SSH host registry

Clide maintains a local registry of named SSH hosts in `~/.clide/hosts.yaml`. Reference servers by a friendly nickname in skills, never by raw IP — connection details are injected only at execution time.

```bash
# Add a server (interactive wizard)
clide host add

# Add non-interactively (scriptable)
clide host add prod --ip 1.2.3.4 --user root --key ~/.ssh/id_ed25519_prod --notes "Hetzner VPS"

# List all registered hosts
clide host list

# Remove a host
clide host remove prod
```

After registering a host named `prod`, your skills can reference it as:

```yaml
commands:
  - "ssh -i ${HOST_PROD_KEY_PATH} -p ${HOST_PROD_PORT} ${HOST_PROD_USER}@${HOST_PROD_IP} 'systemctl restart myapp'"
```

The host IP and credentials are **never included in chat messages or AI prompts** — only the `${HOST_PROD_*}` placeholders appear in skill definitions.

---

### Vault backup & restore

Back up your entire credential set (secrets + SSH hosts) to an encrypted GitHub Gist:

```bash
# Encrypt and upload
clide vault backup
# → prompts for a passphrase, encrypts with age, uploads to GitHub Gist
# → prints the Gist ID — save it

# Restore on a new machine
clide vault restore
# → prompts for GitHub token, Gist ID, and passphrase
```

**What gets backed up:**
- `~/.clide/secrets.yaml`
- `~/.clide/hosts.yaml`

**Encryption:** [age](https://age-encryption.org/) with a passphrase. The passphrase is never stored anywhere — only you know it.

**Automated restore during install:**
```bash
curl -fsSL https://raw.githubusercontent.com/juanitto-maker/Clide/main/install.sh | bash --restore
```
The installer will prompt for your GitHub token, Gist ID, and passphrase, then decrypt and restore everything before completing setup.

--- This keeps API keys and tokens out of your config and makes it easier to share the config file without leaking credentials.

**Quick overview:**
- `~/.clide/config.yaml` — non-sensitive settings (homeserver, model, behaviour)
- `~/.clide/secrets.yaml` — API keys, tokens, passwords (never committed to git)
- `~/.config/clide/config.env` — same secrets in shell `KEY=VALUE` format (written by installer)

**Priority (highest wins):**
```
env var  >  secrets.yaml  >  config.yaml  >  built-in default
```

**Using secrets as variables in skills:**

Any key in `secrets.yaml` is available as `${KEY_NAME}` inside skill commands. The AI never sees the actual values — substitution happens at execution time:
```yaml
# In a skill YAML
commands:
  - "curl -H 'Authorization: Bearer ${MY_API_TOKEN}' https://api.example.com/data"
```

**Where to find auto-generated tokens and passwords after install:**

The installer saves everything it generates or that you provide. Here is where to look:

```bash
# Main config — contains matrix_access_token, bot token, API keys
cat ~/.clide/config.yaml

# Env file — same keys in KEY=VALUE format, auto-sourced by the installer
cat ~/.config/clide/config.env

# SSH public key (if you have one) — safe to share
cat ~/.ssh/id_ed25519.pub
# or
cat ~/.ssh/id_rsa.pub

# List all SSH keys on this system
ls -la ~/.ssh/
```

**SSH key — generate if missing:**
```bash
ssh-keygen -t ed25519 -C "your@email.com"
# Private key: ~/.ssh/id_ed25519  (never share)
# Public key:  ~/.ssh/id_ed25519.pub  (share freely)
```

**Telegram bot token:** Issued by [@BotFather](https://t.me/BotFather) at bot creation time. If you've lost it, open BotFather → `/mybots` → select your bot → API Token.

**Matrix access token:** If you've lost it, log in to Element → Settings → Help & About → scroll to Access Token → click to reveal. Or re-run the API login curl above to generate a fresh one.

For a comprehensive reference on all secrets, credential types, rotation procedures, and skill injection, see **[docs/SECRETS.md](docs/SECRETS.md)**.

---

## Usage

### Start Matrix/Telegram bot
```bash
clide bot
```

### Interactive REPL (Gemini, no bot)
```bash
clide
```

### Manage secrets
```bash
clide secret list                    # list all stored key names
clide secret get <KEY>               # print a single secret value
clide secret set <KEY>               # store a secret (hidden input)
clide secret generate <KEY> [LEN]    # generate a random credential
clide secret pass-init               # set up GPG encryption layer
clide secret pass-set <KEY>          # migrate a YAML secret into pass
```

### Manage SSH hosts
```bash
clide host list                      # list all registered hosts
clide host add [nickname] [--ip ...] # add a host (wizard or flags)
clide host remove <nickname>         # remove a host
```

### Vault backup / restore
```bash
clide vault backup                   # encrypt and upload to GitHub Gist
clide vault restore                  # restore from encrypted Gist
```

### Version
```bash
clide --version
```

### Commands you can send in the Matrix room

```
# System info
status              → system status overview
uptime              → system uptime
disk                → disk usage
memory              → memory usage
processes           → running processes

# Files
ls /path            → list directory
cat /path/file      → show file contents
find /path pattern  → search files

# Remote servers
ssh user@host cmd   → execute on remote server
deploy app          → deploy application
restart service     → restart a service
backup database     → run database backup

# AI-powered
analyze logs        → AI analysis of system logs
suggest fix         → AI suggests a solution
explain error       → AI explains an error message
```

---

## Architecture

```
┌─────────────────────┐
│   You (Element /    │
│   Telegram)         │
└──────────┬──────────┘
           │ Encrypted messages
           ▼
┌─────────────────────┐
│   Matrix Room /     │
│   Telegram Chat     │
└──────────┬──────────┘
           │
           ▼
┌──────────────────────────────────────┐
│  Clide Bot (this app)                │
│  ┌──────────────┐  ┌──────────────┐ │
│  │  Gemini AI   │  │  Executor    │ │
│  │  (interpret) │  │  (run cmds)  │ │
│  └──────────────┘  └──────────────┘ │
│  ┌──────────────┐  ┌──────────────┐ │
│  │  SSH Client  │  │  Skills      │ │
│  │  (remote)    │  │  (YAML wf)   │ │
│  └──────────────┘  └──────────────┘ │
└──────────────────────────────────────┘
           │
           ▼
┌─────────────────────┐
│   Your System /     │
│   Remote Servers    │
└─────────────────────┘
```

---

## Security

### Built-in safety features

- **User allowlist** — only Matrix IDs / Telegram usernames in `authorized_users` can issue commands
- **Command blocklist** — dangerous patterns (`rm -rf /`, `mkfs`, `dd if=`) rejected before execution
- **Confirmation mode** — set `require_confirmation: true` to require a YES reply before any command runs
- **Audit logging** — every command logged with timestamp and origin
- **No root required** — runs entirely as your user
- **Access token auth** — your Matrix password is never stored; only the session token is saved

### Best practices

1. Set `authorized_users` to your own account only
2. Use a dedicated bot account, not your personal one
3. Enable `require_confirmation: true` on sensitive machines
4. `chmod 600 ~/.clide/config.yaml ~/.clide/secrets.yaml`
5. Rotate your Matrix access token periodically
6. Review [docs/SECURITY.md](docs/SECURITY.md) before deploying on a VPS

---

## Platform Support

| Platform | Status | Binary |
|---|---|---|
| Linux x86_64 | Tested | `clide-x86_64` |
| Linux ARM64 | Tested | `clide-aarch64` |
| Android / Termux | Tested (primary) | `clide-aarch64` |
| macOS Intel | Tested | `clide-x86_64-darwin` |
| macOS Apple Silicon | Tested | `clide-aarch64-darwin` |
| Windows | WSL2 recommended | `clide-x86_64-windows.exe` |

---

## Development

### Prerequisites
- Rust 1.75+
- A Matrix account and room — free at [app.element.io](https://app.element.io)

### Build
```bash
cargo build --release
```

### Test
```bash
cargo test
```

### Lint
```bash
cargo clippy -- -D warnings
cargo fmt --check
```

### Debug mode
```bash
RUST_LOG=debug cargo run
```

### Dependencies

All compile into a single static binary — no external runtimes:

| Crate | Purpose |
|---|---|
| `tokio` | Async runtime |
| `reqwest` (rustls-tls) | HTTP client — pure-Rust TLS, no OpenSSL |
| `serde / serde_json / serde_yaml` | Serialization |
| `rusqlite` (bundled) | Embedded SQLite |
| `tracing / tracing-subscriber` | Structured logging |
| `anyhow` | Error handling |
| `chrono` | Date/time |
| `colored` | Terminal colour output |
| `dotenvy` | Env var loading |

---

## Contributing

Contributions are welcome — bug fixes, new skills, platform improvements, docs.

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/your-feature`
3. Make your changes with tests if applicable
4. Open a Pull Request with a clear description

Please read [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md) for code style, commit conventions, and the skills roadmap.

---

## Community

- **Discussions** — questions, ideas, show & tell: [GitHub Discussions](https://github.com/juanitto-maker/Clide/discussions)
- **Bug reports** — [GitHub Issues](https://github.com/juanitto-maker/Clide/issues)
- **Security issues** — please read [SECURITY.md](SECURITY.md) before reporting

---

## Donate

Clide is free and open source. If it saves you time, consider supporting development:

[![Ko-fi](https://img.shields.io/badge/Ko--fi-Support-FF5E5B?logo=ko-fi&logoColor=white)](https://ko-fi.com/juanitto_maker)
[![GitHub Sponsors](https://img.shields.io/badge/GitHub-Sponsor-EA4AAA?logo=github-sponsors)](https://github.com/sponsors/juanitto-maker)

Any amount helps keep the project active. Thank you.

---

## Documentation

| Document | Contents |
|---|---|
| [docs/USER_GUIDE.md](docs/USER_GUIDE.md) | Step-by-step user guide — start here after installing |
| [docs/INSTALL.md](docs/INSTALL.md) | Full platform-specific installation guide |
| [docs/SECRETS.md](docs/SECRETS.md) | Credential manager — secrets file, pass, host registry, vault |
| [docs/SECURITY.md](docs/SECURITY.md) | Security model and best practices |
| [docs/WORKFLOWS.md](docs/WORKFLOWS.md) | Real-world usage examples and skill templates |
| [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md) | Contribution guidelines |
| [CHANGELOG.md](CHANGELOG.md) | Version history |

---

## License

MIT — see [LICENSE](LICENSE).

---

## Acknowledgments

- [Element](https://element.io) / [Matrix](https://matrix.org) — open, decentralised, encrypted comms
- [Google Gemini](https://ai.google.dev) — AI capabilities
- [Rust Community](https://rust-lang.org) — an exceptional ecosystem

---

## Roadmap

- [x] Core bot functionality
- [x] Gemini AI integration
- [x] Element/Matrix integration (v0.3.0)
- [x] Telegram integration
- [x] Android/Termux support
- [x] YAML skills system
- [x] Credential manager (`clide secret` CLI)
- [x] SSH host registry (`clide host` CLI)
- [x] GNU pass / GPG encryption layer
- [x] Age-encrypted vault backup & restore
- [x] Secret scrubber (auto-redact in AI prompts and chat)
- [ ] Web UI dashboard
- [ ] Docker support
- [ ] Multi-room support
- [ ] Scheduled commands
- [ ] Custom command aliases
- [ ] Workflow marketplace

---

<div align="center">
Built with Rust &nbsp;|&nbsp; MIT License &nbsp;|&nbsp; <a href="https://github.com/juanitto-maker/Clide/discussions">Join the discussion</a>
</div>
