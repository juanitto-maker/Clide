# ✈️ clide

> **Glide through your CLI** - Autonomous terminal operations from your pocket

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](CONTRIBUTING.md)
[![Made for Termux](https://img.shields.io/badge/Made%20for-Termux-green.svg)](https://termux.dev)

---

## 🎯 Why clide?

Tired of copying commands between chatbots and Termux? Let your terminal operations **glide** with AI.

**Before clide:**
```
You → ChatGPT: "How do I harden my VPS?"
ChatGPT → You: [Wall of commands]
You → Termux: [Copy, paste, execute, debug, repeat...]
```

**With clide:**
```
You → Signal: "Harden my VPS to Lynis 70"
clide → VPS: [Executes 8-step workflow autonomously]
clide → You: ✓ Complete! Lynis score: 42 → 71
```

---

## ✨ Features

- 🗣️ **Natural language operations** - Talk like a human, not a terminal
- 🤖 **Autonomous execution** - Trial & error handled automatically
- 🧠 **Persistent memory** - Remembers your preferences and context
- 🛡️ **Safety guardrails** - Smart confirmation for destructive operations
- 🔍 **Dry-run mode** - Preview before execution
- ↩️ **Auto-rollback** - Restores on errors
- 🎯 **Multi-VPS support** - Manage multiple servers seamlessly
- 📋 **Workflow templates** - Reusable automation recipes
- 👁️ **Proactive monitoring** - Alerts you to issues before they escalate
- 📚 **Learning system** - Adapts to your patterns over time

---

## 🚀 Quick Start

### One-Liner Install (Termux)
```bash
curl -fsSL https://raw.githubusercontent.com/juanitto-maker/Clide/main/install.sh | bash
```

### Manual Installation
```bash
# Clone the repository
git clone https://github.com/juanitto-maker/Clide
cd Clide

# Run setup
./setup.sh

# Configure
cp config.example.yaml config.yaml
nano config.yaml  # Add your API keys

# Takeoff!
python src/clide.py
```

**Detailed instructions:** [INSTALL.md](docs/INSTALL.md)

---

## 🛫 Example Flight Path

### Simple Operations
```
You: "What's my disk usage?"
clide: 
  /dev/sda1: 68% (42GB/62GB)
  Top consumers:
  - /var/log: 12GB
  - /tmp: 8GB
```

### Complex Workflows
```
You: "Setup PostgreSQL container for my app on staging VPS"

clide: 🛫 Taking off...
       ✓ Docker installed
       ✓ postgres:16-alpine pulled
       ✓ Persistent volumes configured
       ✓ Container running on port 5432
       ✓ Backups scheduled (daily 2AM UTC)
       🛬 Landed! Connection details saved to vault.

You: "Deploy it to production too"

clide: 🛫 Switching to production VPS (prod.example.com)...
       ✓ Cloned staging configuration
       ✓ Updated credentials
       ✓ Production container live
       🛬 Both environments ready!
```

### Error Recovery
```
You: "Deploy my app with SSL"

clide: 🛫 Deploying...
       ✓ Nginx configured
       ✓ Let's Encrypt SSL obtained
       ✗ Port 443 blocked by firewall
       🔄 Auto-fixing: Updating UFW rules...
       ✓ Firewall configured
       🛬 App live at https://yourapp.com (SSL: A+)
```

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────┐
│  You (Signal/Telegram)                              │
└────────────────┬────────────────────────────────────┘
                 ↓
┌─────────────────────────────────────────────────────┐
│  clide (Termux on Android)                          │
│                                                     │
│  ┌──────────────┐    ┌──────────────┐             │
│  │   Signalbot  │ ←→ │    Memory    │             │
│  │  (Messaging) │    │   (SQLite)   │             │
│  └──────┬───────┘    └──────────────┘             │
│         ↓                                          │
│  ┌──────────────┐    ┌──────────────┐             │
│  │ Gemini Flash │ ←→ │    Safety    │             │
│  │ (AI Brain)   │    │  (Guardrails)│             │
│  └──────┬───────┘    └──────────────┘             │
│         ↓                                          │
│  ┌──────────────┐    ┌──────────────┐             │
│  │  Cline CLI   │ ←→ │    Logger    │             │
│  │  (Executor)  │    │ (Flight Log) │             │
│  └──────┬───────┘    └──────────────┘             │
└─────────┼──────────────────────────────────────────┘
          ↓
┌─────────────────────────────────────────────────────┐
│  Your VPS / Local System                            │
└─────────────────────────────────────────────────────┘
```

---

## 📊 Comparison

| Feature | clide | OpenClaw | DIY Script |
|---------|-------|----------|------------|
| Lines of code | ~1,000 | 4,000+ | 500+ |
| Setup time | 10 min | 30 min | 2+ hours |
| Security audit | ✅ Easy | ⚠️ Complex | ✅ Easy |
| Termux-native | ✅ | ⚠️ | ✅ |
| Persistent memory | ✅ | ✅ | ❌ |
| Auto-retry logic | ✅ | ✅ | ❌ |
| Dry-run mode | ✅ | ❌ | ❌ |
| Multi-VPS | ✅ | ✅ | ❌ |
| Learning system | ✅ | ✅ | ❌ |
| **Flight smoothness** | ✈️ Smooth | 🦞 Complex | 🚶 Manual |

---

## 🎓 Documentation

- 📖 [Installation Guide](docs/INSTALL.md) - Termux setup & configuration
- 🔒 [Security Guidelines](docs/SECURITY.md) - Safety protocols & best practices
- 📋 [Workflow Examples](docs/WORKFLOWS.md) - Ready-to-use automation recipes
- 🤝 [Contributing Guide](CONTRIBUTING.md) - Join the crew
- 📝 [Changelog](CHANGELOG.md) - Flight log of releases

---

## 🛣️ Roadmap

### v0.1 - Alpha (Current)
- [x] Core engine (Signal + Gemini + Cline)
- [x] Memory system
- [x] Safety guardrails
- [ ] Public release

### v0.2 - Telegram Support
- [ ] Telegram bot integration
- [ ] Multi-messenger config
- [ ] Unified message handling

### v0.3 - Marketplace
- [ ] Workflow marketplace
- [ ] Community skill sharing
- [ ] One-click workflow import

### v0.4 - Multi-Model
- [ ] Claude API support
- [ ] OpenAI GPT support
- [ ] Local LLM support (Ollama)
- [ ] Model switching per conversation

### v1.0 - Production Ready
- [ ] Enterprise features
- [ ] Advanced monitoring
- [ ] Team collaboration
- [ ] Audit logs & compliance

---

## 💼 Enterprise Version

Need production-grade infrastructure for your team or business?

**HardBot** - Enterprise AI agent platform featuring:
- 🔐 Advanced security & compliance
- 👥 Multi-user environments
- 🗄️ Database & AIWB integration
- 📞 SLA & dedicated support
- 🏢 On-premise deployment options

*Built by the creator of clide with hardened architecture and security-first design.*

---

## 🤝 Contributing

We welcome contributions! Whether you're:

- 🐛 Reporting bugs
- 💡 Suggesting features
- 📝 Improving documentation
- 🔧 Submitting pull requests

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

---

## 📜 License

MIT License - Fly free! 🕊️

See [LICENSE](LICENSE) for details.

---

## 🙏 Credits

Built with love using:
- [signalbot](https://github.com/pnerg/signalbot) - Signal integration
- [Cline](https://github.com/cline/cline) - Autonomous execution
- [Gemini Flash](https://ai.google.dev/gemini-api) - AI brain
- [Termux](https://termux.dev) - Mobile Linux environment

---

## 🌟 Star History

If clide helps you glide through your terminal operations, give us a star! ⭐

---

<div align="center">

**clide** - Because your terminal operations should glide, not grind. ✈️

Made with ❤️ for the Termux community

[Report Bug](https://github.com/juanitto-maker/Clide/issues) · [Request Feature](https://github.com/juanitto-maker/Clide/issues) · [Discussions](https://github.com/juanitto-maker/Clide/discussions)

</div>
