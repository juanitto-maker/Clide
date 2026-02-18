# 🛫 Clide - Glide through your CLI

**Autonomous terminal operations via Signal messenger powered by AI**

A Rust-based CLI agent that executes terminal commands, manages servers, and automates workflows through Signal messages. Built for speed, reliability, and zero-dependency deployment.

---

## ✨ Features

- 🤖 **AI-Powered** - Gemini AI understands natural language commands
- 📱 **Signal Integration** - Control via encrypted Signal messages
- 🔐 **Secure** - End-to-end encrypted communication
- 🚀 **Blazing Fast** - Native Rust performance
- 📦 **Single Binary** - No runtime dependencies
- 🌍 **Cross-Platform** - Linux, macOS, Android (Termux)
- 🔧 **SSH Support** - Remote server management
- 📊 **Rich Logging** - Structured, colorful output

---

## 🎯 Quick Start

### Installation

**One-line install (Linux/macOS/Termux):**
```bash
curl -fsSL https://raw.githubusercontent.com/yourusername/clide/main/install.sh | bash
```

**Manual install:**
```bash
# Download latest release
wget https://github.com/yourusername/clide/releases/latest/download/clide-linux-amd64
chmod +x clide-linux-amd64
sudo mv clide-linux-amd64 /usr/local/bin/clide
```

**Build from source:**
```bash
git clone https://github.com/yourusername/clide.git
cd clide
cargo build --release
sudo cp target/release/clide /usr/local/bin/
```

---

## ⚙️ Configuration

1. **Copy example config:**
```bash
cp config.example.yaml config.yaml
```

2. **Edit config.yaml:**
```yaml
# Gemini API Key (get from https://makersuite.google.com/app/apikey)
gemini_api_key: "YOUR_GEMINI_API_KEY_HERE"

# Signal Number (format: +1234567890)
signal_number: "+1234567890"

# Bot behaviour
require_confirmation: false
authorized_numbers: []  # Empty = allow anyone

# Logging
logging:
  level: "info"
```

3. **Set up Signal CLI:**
```bash
# Link as secondary device (recommended)
signal-cli link -n clide-bot

# Or register new number
signal-cli -a +1234567890 register
signal-cli -a +1234567890 verify <verification-code>
```

---

## 🚀 Usage

### Start Bot
```bash
clide start
```

### Test Gemini Connection
```bash
clide test-gemini "Hello, how are you?"
```

### Test SSH Connection
```bash
clide ssh user@example.com "uptime"
```

### Interactive Mode
```bash
clide
> help
> status
> config show
```

---

## 💬 Signal Commands

Send these commands via Signal to control clide:

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

## 🏗️ Architecture

```
┌─────────────────┐
│  Signal Client  │  (You)
└────────┬────────┘
         │ Encrypted Messages
         ▼
┌─────────────────┐
│  signal-cli     │  (Signal Protocol)
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

## 🔒 Security

### Built-in Safety Features

- ✅ **Command Whitelist** - Only approved commands execute
- ✅ **Confirmation Mode** - Require approval before execution
- ✅ **Host Restrictions** - Limit SSH to specific servers
- ✅ **Audit Logging** - All actions logged with timestamps
- ✅ **Encrypted Config** - Sensitive data encrypted at rest
- ✅ **No Root Required** - Runs with user permissions

### Best Practices

1. **Never share your config.yaml** - Contains API keys
2. **Use Signal device linking** - More secure than SMS registration
3. **Enable confirmation mode** - Review commands before execution
4. **Restrict SSH hosts** - Limit to known servers only
5. **Rotate API keys regularly** - Good security hygiene
6. **Review logs periodically** - Check for suspicious activity

---

## 🛠️ Development

### Prerequisites
- Rust 1.75+
- signal-cli (for Signal integration)
- SSH (for remote operations)

### Build
```bash
cargo build --release
```

### Test
```bash
cargo test
cargo test --all-features
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

## 📦 Dependencies

All dependencies are managed by Cargo and compile to a single static binary:

- **tokio** - Async runtime
- **reqwest** (rustls-tls) - HTTP client for Gemini API (pure-Rust TLS, no OpenSSL)
- **serde / serde_json / serde_yaml** - Serialization (config, JSON)
- **rusqlite** - Embedded SQLite database
- **tracing / tracing-subscriber** - Structured logging
- **ring** - Cryptography (AES-256-GCM, PBKDF2)
- **anyhow** - Error handling
- **chrono** - Date/time utilities
- **colored** - Terminal colour output
- **dotenvy** - Environment variable loading

**Zero runtime dependencies** - Just copy the binary and run!

---

## 🌍 Platform Support

| Platform | Status | Binary Name |
|----------|--------|-------------|
| Linux x64 | ✅ Tested | clide-x86_64-linux |
| Linux ARM64 | ✅ Tested | clide-aarch64-linux |
| Android (Termux) | ✅ Tested | clide-aarch64-android |
| macOS Intel | ✅ Tested | clide-x86_64-darwin |
| macOS Apple Silicon | ✅ Tested | clide-aarch64-darwin |
| Windows | ⚠️ WSL2 recommended | clide-x86_64-windows.exe |

---

## 📚 Documentation

- [Installation Guide](docs/INSTALL.md)
- [Security Guide](docs/SECURITY.md)
- [Workflow Examples](docs/WORKFLOWS.md)
- [Contributing Guide](docs/CONTRIBUTING.md)
- [Security Improvements](docs/SECURITY_IMPROVEMENTS.md)

---

## 🤝 Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing`)
3. Commit changes (`git commit -am 'Add amazing feature'`)
4. Push to branch (`git push origin feature/amazing`)
5. Open a Pull Request

See [CONTRIBUTING.md](docs/CONTRIBUTING.md) for details.

---

## 📝 License

MIT License - see [LICENSE](LICENSE) file for details.

---

## 🙏 Acknowledgments

- [Signal Messenger](https://signal.org) - Encrypted communication
- [Google Gemini](https://ai.google.dev) - AI capabilities
- [signal-cli](https://github.com/AsamK/signal-cli) - Signal protocol implementation
- [Rust Community](https://rust-lang.org) - Amazing ecosystem

---

## 📞 Support

- 🐛 [Report Issues](https://github.com/yourusername/clide/issues)
- 💬 [Discussions](https://github.com/yourusername/clide/discussions)
- 📧 Email: support@yourproject.com
- 📱 Signal: [Join Beta Group]

---

## 🗺️ Roadmap

- [x] Core bot functionality
- [x] Gemini AI integration
- [x] SSH support
- [x] Android/Termux support
- [ ] Web UI dashboard
- [ ] Plugin system
- [ ] Docker support
- [ ] Multi-user support
- [ ] Scheduled commands
- [ ] Custom command aliases

---

**Built with ❤️ in Rust** 🦀
