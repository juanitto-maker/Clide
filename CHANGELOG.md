# Changelog

All notable changes to clide will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Coming Soon
- Claude API support
- Workflow marketplace
- Advanced monitoring dashboard
- Docker support

---

## [0.3.1] - 2026-03-22

### Post-release improvements

Dozens of fixes and features since the v0.3.0 release, focused on Telegram polish, credential management, VPS support, and new security skills.

### Added
- **Telegram forum topics** — bot can send messages into forum threads (`83a21bc`)
- **Telegram file/image interpretation** — uploaded files and images are forwarded to Gemini for vision-based analysis (`0eb9b77`, `30ca053`)
- **`/stop` command** — abort a running agent task from chat (`236d5e0`)
- **`/debug` command** — Telegram startup diagnostics (`80f317f`)
- **Centralized secrets management** — `~/.clide/secrets.yaml` with `clide secret` CLI (`3c222f4`)
- **GNU pass / GPG integration** — optional GPG-encrypted credential storage (`2a5a99e`)
- **SSH host registry** — `clide host add/list/remove` with `${HOST_*}` variable injection (`fb146f9`)
- **Age-encrypted vault backup & restore** — `clide vault backup/restore` to GitHub Gist (`fb146f9`)
- **Secret scrubber** — auto-redact secrets from AI prompts and chat messages (`fb146f9`)
- **SSH host injection into agent prompt** — registered hosts available to the AI (`723bac8`)
- **Secrets exported as env vars** in child shell processes (`130e3a2`)
- **SSH keys included in vault backup** (`0ab8ef1`)
- **VPS wizard in installer** — automated SSH key setup for remote servers (`3998a0f`)
- **Linux VPS support in installer and CI** — systemd service setup, x86_64 release binary (`e46ab53`)
- **Smarter message chunking** — answer and command log split into separate messages (`3e4d1d1`)
- **Graceful Ctrl+C shutdown** via `tokio::signal::ctrl_c()` (`c95a9d6`)
- **Fail-closed auth** — unauthorized messages get feedback instead of silent drop (`c891e08`, `3ded14a`)
- **YAML control-char stripping** on config load (`98a59ae`)
- **Hot-reload config** for Telegram authorized users (`5393a11`)
- **New skills shipped:** `lynis_audit`, `clide_install`, `debugger`, `aiwb_manager`, `docker_ai_sandbox`, `system_overview`, `maintenance_cron`, `intrusion_alert`, `vault_auto_backup`, `port_scanner`, `vps_manager`, `vps_hardening`

### Changed
- Default `max_agent_steps` increased from 20 to 40 (`641b39a`)
- Migrated from deprecated `gemini-2.0-flash` to `gemini-2.5-flash` (`5e83d8b`)
- `lynis_audit` skill now requires explicit SSH params instead of defaulting to local mode (`38cb8c0`)
- Consolidated SSH calls to use `echo ${SUDO_PASSWORD} | sudo -S` pattern (`4af6a61`)

### Fixed
- Telegram polling restore, file export, auth/webhook conflict (`fd60817`)
- Telegram bot token validation at startup (`80f317f`)
- Bot self-response loop prevention via `/whoami` (`df4c1ee`)
- Gemini API schema error (unsupported `additionalProperties`) (`b34754c`)
- AIWB integration: timeout, env propagation, file extraction and delivery (`e1c221d`, `413f824`, `0cedecc`)
- OOM prevention: cap command output at read time on Termux (`2fef820`)
- Installer: Gemini/VPS prompts, Telegram user storage, nickname validation (`72ffe76`, `94a662b`)

---

## [0.3.0] - 2026-02-18

### Migrate from Signal to Element/Matrix

Complete replacement of Signal/signal-cli with Element/Matrix as the messaging backend.

### Changed
- Replaced `signal.rs` with `matrix.rs` — uses Matrix Client-Server API v3 directly via `reqwest`
- `SignalClient` → `MatrixClient` (no external process dependency, no Java/JVM required)
- `signal_number` config field replaced with four Matrix fields:
  - `matrix_homeserver` — homeserver URL (e.g. `https://matrix.org`)
  - `matrix_user` — full Matrix user ID (e.g. `@user:matrix.org`)
  - `matrix_access_token` — login token (supports `MATRIX_ACCESS_TOKEN` env var)
  - `matrix_room_id` — the room the bot listens in
- `authorized_numbers` → `authorized_users` (Matrix user IDs instead of phone numbers)
- Installer (`install.sh`) completely rewritten:
  - Removed all signal-cli installation (no Java 17 required)
  - Removed libsignal ARM64 fix / patchelf / LD_PRELOAD workarounds
  - Added interactive Matrix/Element setup with skip option at every step
  - Auto-login: enter username + password to obtain an access token, or paste one manually
  - Gemini API key prompt retained with skip option

### Removed
- `fix-libsignal.sh` — no longer needed
- `signal-cli` subprocess dependency — no external processes for messaging
- Java 17 / OpenJDK 17 dependency

### Added
- `MATRIX_ACCESS_TOKEN` environment variable support (overrides config yaml)
- Initial sync skipping to avoid re-delivering history on bot startup

---

## [0.2.0] - 2025-02-XX

### Rust Rewrite

Complete rewrite from Python to Rust for performance, safety, and zero-dependency deployment.

### Changed
- Replaced Python runtime with a native Rust binary (no interpreter needed)
- Switched from `signalbot` Python library to `signal-cli` subprocess integration
- Replaced `google-generativeai` Python package with `reqwest`-based Gemini API client
- Replaced `sqlite3` Python module with `rusqlite` (bundled SQLite)
- Pure-Rust TLS via `rustls` — no OpenSSL dependency, works on Android/Termux
- Structured logging with `tracing` / `tracing-subscriber`
- Removed dependency on Cline CLI and Node.js

---

## [0.1.0-alpha] - 2025-02-XX

### Initial Release

- Signal messenger integration via signalbot
- Natural language command interpretation via Gemini Flash
- Autonomous command execution
- Persistent memory system using SQLite
- Safety guardrails (blocked commands, confirmation mode)
- Termux/Android support

---

## Version History

- **v0.3.1** - Telegram polish, credential manager, VPS support, 12 new skills
- **v0.3.0** - Element/Matrix migration: no Java, no signal-cli, pure HTTP API
- **v0.2.0** - Rust rewrite: single static binary, no Python runtime
- **v0.1.0-alpha** - Initial Python prototype with Signal

---

**clide** - Glide through your CLI
