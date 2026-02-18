# Clide - Glide through your CLI

**Autonomous terminal operations via Element/Matrix powered by AI**

A Rust-based CLI agent that executes terminal commands, manages servers, and automates
workflows through Matrix messages. Built for speed, reliability, and zero-dependency deployment.

---

## Features

- **AI-Powered** - Gemini AI understands natural language commands
- **Element/Matrix Integration** - Control via end-to-end encrypted Matrix rooms
- **Secure** - Access token auth, authorized-user allowlist, confirmation mode
- **Blazing Fast** - Native Rust performance
- **Single Binary** - No runtime dependencies (no Java, no Node.js, no signal-cli)
- **Cross-Platform** - Linux, macOS, Android (Termux)
- **SSH Support** - Remote server management
- **Rich Logging** - Structured, colorful output

---

## Quick Start

### Installation

**One-line install (Termux/Android):**
```bash
curl -fsSL https://raw.githubusercontent.com/juanitto-maker/Clide/main/install.sh | bash
```

**Manual install:**
```bash
wget https://github.com/juanitto-maker/Clide/releases/latest/download/clide-x86_64
chmod +x clide-x86_64
sudo mv clide-x86_64 /usr/local/bin/clide
```

**Build from source:**
```bash
git clone https://github.com/juanitto-maker/Clide.git
cd Clide
cargo build --release
sudo cp target/release/clide /usr/local/bin/
```

---

## Configuration

1. **Copy example config:**
```bash
mkdir -p ~/.clide
cp config.example.yaml ~/.clide/config.yaml
chmod 600 ~/.clide/config.yaml
```

2. **Edit `~/.clide/config.yaml`:**
```yaml
gemini_api_key: "YOUR_GEMINI_API_KEY"

matrix_homeserver: "https://matrix.org"
matrix_user: "@yourbot:matrix.org"
matrix_access_token: "YOUR_ACCESS_TOKEN"
matrix_room_id: "!roomid:matrix.org"

require_confirmation: false
authorized_users: []        # empty = allow anyone in the room
```

3. **Get a Matrix access token:**
   - **Via Element:** Settings → Help & About → Access Token (click to reveal)
   - **Via API:**
     ```bash
     curl -XPOST https://matrix.org/_matrix/client/v3/login \
       -H "Content-Type: application/json" \
       -d '{"type":"m.login.password","identifier":{"type":"m.id.user","user":"USERNAME"},"password":"PASSWORD"}'
     ```

4. **Find your room ID:**
   - Element → open the room → Settings → Advanced → Internal room ID
   - Format: `!abc123:matrix.org`

5. **Invite the bot account to the room** (if it isn't already a member)

---

## Usage

### Start Matrix Bot
```bash
clide bot
```

### Interactive REPL (Gemini only, no Matrix)
```bash
clide
```

### Show version
```bash
clide --version
```

---

## Matrix Commands

Send these messages in your Matrix room to control Clide:

### System Commands
```
status              # Show system status
uptime              # Show system uptime
disk                # Show disk usage
memory              # Show memory usage
processes           # List running processes
```

### File Operations
```
ls /path            # List directory
cat /path/file      # Show file contents
tail -f /var/log    # Follow log file
find /path pattern  # Search files
```

### Remote Server Management
```
ssh user@host cmd   # Execute on remote server
deploy app          # Deploy application
restart service     # Restart service
backup database     # Backup database
```

### AI-Powered Commands
```
analyze logs        # AI analyzes system logs
suggest fix         # AI suggests solutions
explain error       # AI explains error message
optimize config     # AI optimizes configuration
```

---

## Architecture

```
┌─────────────────┐
│  Element Client │  (You)
└────────┬────────┘
         │ Encrypted Messages (Matrix protocol)
         ▼
┌─────────────────┐
│  Matrix Room    │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Clide Bot      │  (This app)
│  ┌───────────┐  │
│  │ Gemini AI │  │  (Command interpretation)
│  └───────────┘  │
│  ┌───────────┐  │
│  │ Executor  │  │  (Safe command execution)
│  └───────────┘  │
│  ┌───────────┐  │
│  │ SSH Client│  │  (Remote operations)
│  └───────────┘  │
└─────────────────┘
         │
         ▼
┌─────────────────┐
│  Your System    │
└─────────────────┘
```

---

## Security

### Built-in Safety Features

- **Command Allowlist** - Block dangerous shell patterns via `blocked_commands`
- **Confirmation Mode** - Require YES before execution
- **User Allowlist** - Restrict to specific Matrix user IDs via `authorized_users`
- **Audit Logging** - All actions logged with timestamps
- **No Root Required** - Runs with user permissions
- **Access Token Auth** - Password never stored after setup

### Best Practices

1. **Never share `~/.clide/config.yaml`** - Contains API keys and access token
2. **Use a dedicated bot account** - Don't reuse your personal Matrix account
3. **Enable confirmation mode** - Review commands before execution
4. **Restrict `authorized_users`** - Limit to your own Matrix ID
5. **Rotate access tokens periodically** - Good security hygiene

---

## Development

### Prerequisites
- Rust 1.75+
- A Matrix account and room (free at [app.element.io](https://app.element.io))

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

### Run with Debug Logging
```bash
RUST_LOG=debug cargo run
```

---

## Dependencies

All dependencies compile to a single static binary — **no Java, no signal-cli, no Node.js**:

- **tokio** - Async runtime
- **reqwest** (rustls-tls) - HTTP client for Gemini and Matrix APIs (pure-Rust TLS)
- **serde / serde_json / serde_yaml** - Serialization
- **rusqlite** - Embedded SQLite database
- **tracing / tracing-subscriber** - Structured logging
- **anyhow** - Error handling
- **chrono** - Date/time utilities
- **colored** - Terminal colour output
- **dotenvy** - Environment variable loading

---

## Platform Support

| Platform | Status | Binary Name |
|----------|--------|-------------|
| Linux x64 | Tested | clide-x86_64-linux |
| Linux ARM64 | Tested | clide-aarch64-linux |
| Android (Termux) | Tested | clide-aarch64-android |
| macOS Intel | Tested | clide-x86_64-darwin |
| macOS Apple Silicon | Tested | clide-aarch64-darwin |
| Windows | WSL2 recommended | clide-x86_64-windows.exe |

---

## Documentation

- [Installation Guide](docs/INSTALL.md)
- [Security Guide](docs/SECURITY.md)
- [Workflow Examples](docs/WORKFLOWS.md)
- [Contributing Guide](docs/CONTRIBUTING.md)

---

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing`)
3. Commit your changes
4. Push to branch and open a Pull Request

---

## License

MIT License - see [LICENSE](LICENSE) file for details.

---

## Acknowledgments

- [Element](https://element.io) / [Matrix](https://matrix.org) - Open, decentralised, encrypted communication
- [Google Gemini](https://ai.google.dev) - AI capabilities
- [Rust Community](https://rust-lang.org) - Amazing ecosystem

---

## Roadmap

- [x] Core bot functionality
- [x] Gemini AI integration
- [x] Element/Matrix integration (v0.3.0)
- [x] Android/Termux support
- [ ] Web UI dashboard
- [ ] Docker support
- [ ] Multi-room support
- [ ] Scheduled commands
- [ ] Custom command aliases

---

**Built with Rust**
