# 🤝 Contributing to clide

Thank you for considering contributing to clide! We welcome contributions from everyone, whether you're fixing a typo, adding a feature, or improving documentation.

---

## 🛫 Quick Start for Contributors

### 1. Fork & Clone
```bash
# Fork the repo on GitHub, then:
git clone https://github.com/yourusername/clide
cd clide
```

### 2. Set Up Development Environment
```bash
# Install dependencies
pip install -r requirements.txt

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
- 💬 **New messenger integrations** (Telegram, Discord, etc.)
- 🤖 **Additional LLM support** (Claude, GPT, local models)
- 📋 **Workflow templates** - Share your automation recipes
- 🎨 **UI improvements** - Better terminal output formatting
- 📊 **Monitoring enhancements** - Advanced alerting systems

### What We Won't Accept
- ❌ Features that compromise security
- ❌ Code that breaks existing functionality without discussion
- ❌ Unnecessary dependencies that bloat the project
- ❌ Poor quality code without tests or documentation

---

## 📝 Code Guidelines

### Python Style
- Follow PEP 8 style guide
- Use meaningful variable names
- Keep functions focused and small (< 50 lines ideally)
- Add docstrings to functions and classes

**Example:**
```python
def execute_command(command: str, dry_run: bool = False) -> dict:
    """
    Execute a shell command with safety checks.
    
    Args:
        command: The shell command to execute
        dry_run: If True, only simulate execution
        
    Returns:
        dict: Execution result with stdout, stderr, and return code
        
    Raises:
        SecurityError: If command is deemed unsafe
    """
    # Implementation here
    pass
```

### File Organization
```
src/
├── clide.py         # Main entry point - keep minimal
├── bot.py           # Messenger integration - one class per messenger
├── memory.py        # Database operations - well-documented queries
├── brain.py         # LLM integration - model-agnostic design
├── executor.py      # Command execution - safety-first
├── safety.py        # Security checks - paranoid is good
└── logger.py        # Logging - structured and searchable
```

### Testing
- Write tests for new features
- Ensure existing tests pass
- Test on Termux if possible (or mention you couldn't)

```python
# Example test structure
def test_safety_check_dangerous_command():
    """Test that rm -rf / is blocked"""
    result = safety.check_command("rm -rf /")
    assert result.is_dangerous == True
    assert result.requires_confirmation == True
```

---

## 🔒 Security Guidelines

### Reporting Security Issues
**DO NOT** open public issues for security vulnerabilities!

Instead, email: security@yourproject.com (or create a private security advisory on GitHub)

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
- 💬 Ask questions in [Discussions](https://github.com/yourusername/clide/discussions)
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

### Current Focus (v0.1 - v0.2)
- Core stability improvements
- Telegram integration
- Better error handling
- Documentation expansion

### Future Plans (v0.3+)
- Workflow marketplace
- Multi-model LLM support
- Advanced monitoring features
- Team collaboration tools

See [Roadmap](https://github.com/yourusername/clide/projects) for detailed plans.

---

## 💡 Feature Requests

Have an idea? We'd love to hear it!

1. Check [existing issues](https://github.com/yourusername/clide/issues) first
2. If it's new, open a [Feature Request](https://github.com/yourusername/clide/issues/new?template=feature_request.md)
3. Describe the problem and your proposed solution
4. Discuss with the community
5. If approved, feel free to implement it!

---

## 📞 Contact

- 🐛 **Bug Reports:** [GitHub Issues](https://github.com/yourusername/clide/issues)
- 💬 **Discussions:** [GitHub Discussions](https://github.com/yourusername/clide/discussions)
- 🔒 **Security:** security@yourproject.com
- 📧 **Maintainers:** maintainer@yourproject.com

---

## 🙏 Thank You!

Every contribution, no matter how small, makes clide better. Thank you for being part of this journey! ✈️

**Happy coding, and may your terminal operations always glide smoothly!** 🚀
