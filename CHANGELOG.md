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

- **v0.3.0** - Element/Matrix migration: no Java, no signal-cli, pure HTTP API
- **v0.2.0** - Rust rewrite: single static binary, no Python runtime
- **v0.1.0-alpha** - Initial Python prototype with Signal

---

**clide** - Glide through your CLI
