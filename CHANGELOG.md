# ✈️ Changelog

All notable changes to clide will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### 🛫 Coming Soon
- Telegram bot integration
- Claude API support
- Workflow marketplace
- Advanced monitoring dashboard

---

## [0.2.0] - 2025-02-XX

### 🔄 Rust Rewrite

Complete rewrite from Python to Rust for performance, safety, and zero-dependency deployment.

### ✨ Added / Changed
- Replaced Python runtime with a native Rust binary (no interpreter needed)
- Switched from `signalbot` Python library to `signal-cli` subprocess integration
- Replaced `google-generativeai` Python package with `reqwest`-based Gemini API client
- Replaced `sqlite3` Python module with `rusqlite` (bundled SQLite)
- Pure-Rust TLS via `rustls` — no OpenSSL dependency, works on Android/Termux out of the box
- Structured logging with `tracing` / `tracing-subscriber`
- Removed dependency on Cline CLI and Node.js

### 🔧 Technical Details
- Single static binary — copy and run, no runtime required
- Async throughout with `tokio`
- Environment variable support via `dotenvy` (`GEMINI_API_KEY`)

---

## [0.1.0-alpha] - 2025-02-XX

### 🎉 Initial Release - Taking Off!

The first public alpha release of clide - autonomous terminal operations from your pocket.

### ✨ Added
- **Core Features**
  - Signal messenger integration via signalbot
  - Natural language command interpretation via Gemini Flash
  - Autonomous command execution via Cline CLI
  - Persistent memory system using SQLite
  - Multi-VPS configuration and management

- **Safety Systems**
  - Smart safety guardrails for destructive operations
  - Dry-run mode for previewing commands
  - Auto-rollback on execution errors
  - Command confirmation for high-risk operations
  - Execution logging and audit trails

- **Intelligence Features**
  - Context-aware conversation handling
  - Learning from user patterns and preferences
  - Workflow template system
  - Error recovery with retry logic
  - Command history tracking

- **Termux Support**
  - Native Android/Termux compatibility
  - One-liner installation script
  - Optimized for mobile environment
  - Low resource footprint

- **Documentation**
  - Comprehensive README
  - Installation guide for Termux
  - Security guidelines
  - Workflow examples
  - Contributing guidelines

### 🔧 Technical Details
- **Dependencies**
  - Rust 1.75+
  - tokio for async runtime
  - reqwest (rustls-tls) for Gemini API HTTP calls
  - serde / serde_json / serde_yaml for config & JSON
  - rusqlite (bundled) for persistent storage
  - tracing / tracing-subscriber for structured logging
  - ring for cryptography
  - signal-cli (external, Java) for Signal integration

- **Architecture**
  - Modular Rust crate structure
  - Async/await throughout (tokio)
  - Pure-Rust TLS via rustls — no OpenSSL dependency
  - Secure credential storage

### 📝 Known Issues
- Occasional Signal connection drops (working on reconnection logic)
- Dry-run mode doesn't support all command types yet
- Memory database can grow large over time (compaction coming soon)

### 🙏 Credits
- Thanks to all early testers and contributors!
- Special thanks to the signal-cli, Rust, and Termux communities

---

## Development Log

### Week 1 (2025-02-08 to 2025-02-14)
- 🎯 Project conception and architecture design
- 🏗️ Core framework implementation
- 🔒 Security systems integration
- 📱 Termux compatibility testing
- 📖 Documentation creation

---

## Version History

- **v0.2.0** - Rust rewrite: single static binary, no runtime dependencies
- **v0.1.0-alpha** - Initial Python prototype
- *More versions coming soon as we fly higher!* ✈️

---

## How to Upgrade

### From Source
```bash
cd clide
git pull origin main
cargo build --release
sudo cp target/release/clide /usr/local/bin/
```

### Breaking Changes
None yet - this is the first release!

### Migration Notes
N/A - First release

---

## Versioning Strategy

We follow [Semantic Versioning](https://semver.org/):

- **MAJOR** version (X.0.0) - Incompatible API changes
- **MINOR** version (0.X.0) - New features, backwards compatible
- **PATCH** version (0.0.X) - Bug fixes, backwards compatible

### Pre-release Labels
- `alpha` - Early testing, may have bugs
- `beta` - Feature complete, testing phase
- `rc` - Release candidate, final testing

---

## Support

- 🐛 **Report bugs:** [GitHub Issues](https://github.com/yourusername/clide/issues)
- 💡 **Request features:** [GitHub Discussions](https://github.com/yourusername/clide/discussions)
- 📖 **Documentation:** [docs/](docs/)

---

**clide** - Glide through your CLI ✈️
