# 🤝 Contributing to clide

Thank you for considering contributing to clide! We welcome contributions from everyone, whether you're fixing a typo, adding a feature, or improving documentation.

---

## 🛫 Quick Start for Contributors

### 1. Fork & Clone
```bash
# Fork the repo on GitHub, then:
git clone https://github.com/juanitto-maker/Clide
cd clide
```

### 2. Set Up Development Environment
```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Build the project
cargo build

# Copy config example
cp config.example.yaml config.yaml

# Add your API keys for testing
nano config.yaml
```

### 3. Create a Branch
```bash
# Use descriptive branch names
git checkout -b feature/add-telegram-support
git checkout -b fix/memory-leak-issue
git checkout -b docs/improve-install-guide
```

### 4. Make Your Changes
- Write clean, readable code
- Follow existing code style
- Add comments for complex logic
- Test your changes thoroughly

### 5. Commit with Clear Messages
```bash
# Good commit messages:
git commit -m "feat: Add Telegram bot integration"
git commit -m "fix: Resolve memory leak in SQLite connection"
git commit -m "docs: Update installation guide for Termux"

# Use conventional commits format:
# feat: New feature
# fix: Bug fix
# docs: Documentation changes
# style: Code style/formatting
# refactor: Code refactoring
# test: Adding tests
# chore: Maintenance tasks
```

### 6. Push & Create Pull Request
```bash
git push origin your-branch-name
```
Then create a PR on GitHub with a clear description of your changes.

---

## 🎯 What We're Looking For

### High Priority
- 🐛 **Bug fixes** - Squash those bugs!
- 📱 **Termux compatibility** - Ensure smooth mobile operation
- 🔒 **Security improvements** - Help keep clide safe
- 📚 **Documentation** - Make clide easier to understand
- ✅ **Tests** - Increase code coverage

### Feature Requests
- 🤖 **Additional LLM support** (Claude, GPT, local models via Ollama)
- 📋 **Workflow templates** - Share your automation recipes
- 🎨 **UI improvements** - Better terminal output formatting
- 📊 **Monitoring enhancements** - Advanced alerting systems
- 🌐 **Web UI dashboard** - Browser-based management interface

### What We Won't Accept
- ❌ Features that compromise security
- ❌ Code that breaks existing functionality without discussion
- ❌ Unnecessary dependencies that bloat the project
- ❌ Poor quality code without tests or documentation

---

## 📝 Code Guidelines

### Rust Style
- Follow the official [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Run `cargo fmt` before committing (enforced by CI)
- Address all `cargo clippy -- -D warnings` lints
- Use meaningful variable and function names
- Keep functions focused and small

**Example:**
```rust
/// Execute a shell command with safety checks.
///
/// Returns an error if the command matches a blocked pattern or
/// if the executor is in dry-run mode.
pub async fn execute_command(command: &str, dry_run: bool) -> anyhow::Result<CommandResult> {
    if is_blocked(command) {
        anyhow::bail!("Command blocked by safety rules: {command}");
    }
    if dry_run {
        tracing::info!("[DRY-RUN] Would execute: {command}");
        return Ok(CommandResult::dry_run());
    }
    // ... actual execution
}
```

### File Organization
```
src/
├── main.rs          # Entry point, CLI commands (bot, secret, host), REPL
├── agent.rs         # Core AI agent logic, command interpretation, workflow execution
├── bot.rs           # Bot orchestration layer
├── config.rs        # Configuration loading and validation (YAML)
├── database.rs      # SQLite conversation history
├── executor.rs      # Command execution — safety-first
├── gemini.rs        # Google Gemini API client
├── hosts.rs         # SSH host registry management
├── lib.rs           # Library root
├── logger.rs        # Logging setup (tracing + file appender)
├── matrix.rs        # Matrix/Element E2E messaging client
├── memory.rs        # In-memory state
├── pass_store.rs    # GNU pass / GPG integration
├── scrubber.rs      # Secret redaction before AI prompts
├── ssh.rs           # SSH client wrapper
├── telegram.rs      # Telegram client wrapper
├── telegram_bot.rs  # Telegram bot polling + message handling
├── workflow.rs      # YAML skill execution engine
└── skills/
    ├── mod.rs       # Skills module root
    └── manager.rs   # Skill discovery and loading
```

### Testing
- Write tests for new features
- Run the full test suite with `cargo test`
- Test on Termux if possible (or note in the PR that you couldn't)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dangerous_command_is_blocked() {
        assert!(is_blocked("rm -rf /"));
        assert!(is_blocked("mkfs /dev/sda"));
    }

    #[tokio::test]
    async fn test_dry_run_does_not_execute() {
        let result = execute_command("echo hello", true).await.unwrap();
        assert!(result.is_dry_run);
    }
}
```

---

## 🔒 Security Guidelines

### Reporting Security Issues
**DO NOT** open public issues for security vulnerabilities!

Instead, email: See [SECURITY.md](../SECURITY.md) (or create a private security advisory on GitHub)

We'll respond within 48 hours and work with you on a fix.

### Security Best Practices
- Never commit API keys or credentials
- Always sanitize user input
- Use parameterized queries for database operations
- Validate all file paths to prevent directory traversal
- Be paranoid about command injection

---

## 📋 Pull Request Process

### Before Submitting
- ✅ Code follows style guidelines
- ✅ All tests pass
- ✅ Documentation updated (if needed)
- ✅ Commit messages are clear
- ✅ Branch is up-to-date with main

### PR Description Template
```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Documentation update
- [ ] Performance improvement
- [ ] Code refactoring

## Testing
How did you test this?

## Screenshots (if applicable)
Add screenshots for UI changes

## Checklist
- [ ] My code follows the style guidelines
- [ ] I have tested my changes on Termux
- [ ] I have updated the documentation
- [ ] I have added tests
```

### Review Process
1. Maintainer reviews your PR (usually within 3 days)
2. Address any feedback or requested changes
3. Once approved, maintainer merges your PR
4. Your contribution is included in the next release! 🎉

---

## 🎓 First Time Contributors

New to open source? Welcome! Here's how to get started:

### Good First Issues
Look for issues labeled `good-first-issue` - these are perfect for newcomers:
- Documentation improvements
- Adding code comments
- Fixing typos
- Simple bug fixes
- Writing tests

### Need Help?
- 💬 Ask questions in [Discussions](https://github.com/juanitto-maker/Clide/discussions)
- 📧 Reach out to maintainers
- 📖 Check existing issues and PRs for examples

**Don't be shy!** Everyone was a first-time contributor once. We're here to help! 🤗

---

## 🌟 Recognition

All contributors are recognized in:
- README.md contributors section
- Release notes
- Our hearts ❤️

Significant contributors may be invited to become maintainers!

---

## 📜 Code of Conduct

### Our Pledge
We are committed to providing a welcoming and inclusive environment for everyone, regardless of:
- Age, body size, disability, ethnicity
- Gender identity and expression
- Experience level
- Nationality, personal appearance, race, religion
- Sexual identity and orientation

### Our Standards

**Positive behavior:**
- Being respectful and inclusive
- Gracefully accepting constructive criticism
- Focusing on what's best for the community
- Showing empathy towards others

**Unacceptable behavior:**
- Harassment, trolling, or insulting comments
- Public or private harassment
- Publishing others' private information
- Other conduct which could reasonably be considered inappropriate

### Enforcement
Instances of unacceptable behavior may be reported to project maintainers. All complaints will be reviewed and investigated, resulting in a response deemed necessary and appropriate.

---

## 🚀 Development Roadmap

Want to contribute but not sure where to start? Check our roadmap:

### Shipped (v0.1 – v0.3)
- Core bot functionality + Gemini AI
- Element/Matrix + Telegram integration
- YAML skills system (18 shipped skills)
- Credential manager (`clide secret` CLI)
- SSH host registry (`clide host` CLI)
- GNU pass / GPG encryption layer
- Age-encrypted vault backup & restore
- Secret scrubber (auto-redact in AI prompts)
- VPS support with systemd service

### Current Focus (v0.4+)
- Multi-model LLM support (Claude, Ollama)
- Web UI dashboard
- Docker support
- Workflow marketplace
- Scheduled commands

See [SKILLS_ROADMAP.md](SKILLS_ROADMAP.md) for the skills development roadmap.

---

## 💡 Feature Requests

Have an idea? We'd love to hear it!

1. Check [existing issues](https://github.com/juanitto-maker/Clide/issues) first
2. If it's new, open a [Feature Request](https://github.com/juanitto-maker/Clide/issues/new?template=feature_request.md)
3. Describe the problem and your proposed solution
4. Discuss with the community
5. If approved, feel free to implement it!

---

## 📞 Contact

- 🐛 **Bug Reports:** [GitHub Issues](https://github.com/juanitto-maker/Clide/issues)
- 💬 **Discussions:** [GitHub Discussions](https://github.com/juanitto-maker/Clide/discussions)
- 🔒 **Security:** See [SECURITY.md](../SECURITY.md)
- 📧 **Maintainers:** [GitHub Discussions](https://github.com/juanitto-maker/Clide/discussions)

---

## 🙏 Thank You!

Every contribution, no matter how small, makes clide better. Thank you for being part of this journey! ✈️

**Happy coding, and may your terminal operations always glide smoothly!** 🚀
