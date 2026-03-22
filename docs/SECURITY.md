# 🔒 Security Guidelines

Security is a top priority for clide. This document outlines security best practices, threat models, and how to report vulnerabilities.

---

## 🎯 Security Philosophy

clide follows these core security principles:

1. **Least Privilege** - Only request permissions actually needed
2. **Defense in Depth** - Multiple layers of security checks
3. **Fail Secure** - When in doubt, deny and ask for confirmation
4. **Transparency** - Open source code for community audit
5. **User Control** - You own your data and credentials

---

## 🛡️ Security Features

### Built-in Protections

#### 1. Command Safety System
```yaml
# In config.yaml — patterns blocked before execution
blocked_commands:
  - "rm -rf /"           # System wipe
  - "dd if="             # Disk destruction
  - "mkfs"               # Format filesystem
  - ":(){ :|:& };:"      # Fork bomb
  - "chmod -R 777 /"     # Unsafe permissions
```

#### 2. Confirmation Requirements
- Destructive operations always require explicit confirmation
- Modifying system files triggers approval prompt
- User/group changes need verification
- Firewall modifications are reviewed before execution

#### 3. Credential Management

Clide uses a layered secrets approach:
- `~/.clide/secrets.yaml` — file-permission protected (`chmod 600`)
- Optional GNU pass / GPG encryption layer for high-security setups
- Age-encrypted vault backup to GitHub Gist
- Secret scrubber auto-redacts all outbound text before reaching AI or chat

#### 4. Audit Logging
- Every command is logged with timestamp
- Execution results are recorded
- Failed attempts are tracked
- Logs are tamper-evident

#### 5. Dry-Run Mode
```bash
# Preview commands before execution
You: "Clean up old logs"
clide: [DRY-RUN] Would execute:
       find /var/log -mtime +30 -delete
       Affects: 847 files (12GB)
       Proceed? (yes/no)
```

---

## 🔐 Configuration Security

### Protecting Your config.yaml

#### File Permissions
```bash
# Ensure only you can read the config
chmod 600 config.yaml

# Verify permissions
ls -l config.yaml
# Should show: -rw------- (600)
```

#### API Key Security
```yaml
# ❌ NEVER commit config.yaml with real keys
gemini_api_key: "YOUR_KEY_HERE"  # Replace before committing

# ✅ Leave blank and set the env var instead
gemini_api_key: ""
# Then: export GEMINI_API_KEY="your-key"
```

#### Git Protection
```bash
# Add to .gitignore
echo "config.yaml" >> .gitignore
echo "*.db" >> .gitignore
echo "*.log" >> .gitignore
```

### Messaging Account Security

#### Best Practices
1. **Use a dedicated bot account** — don't use your personal Matrix or Telegram account
2. **Matrix access token** — never store your password; only the session token is saved
3. **Telegram bot** — create via @BotFather; the bot token is stored in `secrets.yaml`
4. **Rotate tokens periodically** — regenerate Matrix access tokens and Telegram bot tokens

#### Limiting Access
```yaml
# config.yaml - Restrict to your accounts only
authorized_users:
  - "@youraccount:matrix.org"   # Matrix user ID
  - "your_telegram_username"     # Telegram username
```

---

## 🚨 Threat Model

### What clide Protects Against

✅ **Command Injection**
- Input sanitization
- Parameterized execution
- No shell string concatenation

✅ **Unauthorized Access**
- Matrix user ID / Telegram username allowlist
- Fail-closed auth (unauthorized messages rejected with feedback)
- No public API endpoints

✅ **Credential Theft**
- Secrets stored in `secrets.yaml` with `chmod 600`, optional GPG encryption via pass
- Secret scrubber redacts values before sending to AI or chat
- Vault backups encrypted with age (ChaCha20-Poly1305 + Argon2)

✅ **Accidental Destruction**
- Pattern matching for dangerous commands
- Confirmation prompts
- Rollback capabilities

### What clide CANNOT Protect Against

❌ **Compromised Device**
- If your phone/Termux is rooted/jailbroken and compromised
- Malware with root access can read memory

❌ **Social Engineering**
- Someone tricking you into approving dangerous commands
- Always verify what you're approving!

❌ **Physical Access**
- Someone with physical access to unlocked device
- Use device encryption and screen lock

❌ **API Key Compromise**
- If your Gemini API key is stolen
- Monitor API usage regularly
- Rotate keys periodically

---

## 🔍 Security Best Practices

### For Users

#### 1. Strong Authentication
```yaml
# config.yaml — only allow your own accounts
authorized_users:
  - "@youraccount:matrix.org"
  - "your_telegram_username"
```

#### 2. Regular Updates
```bash
# Update clide weekly
cd clide && git pull origin main && cargo build --release
sudo cp target/release/clide /usr/local/bin/
```

#### 3. Monitor Activity
```bash
# Review logs regularly
tail -f ~/.clide/logs/clide.log

# Check for suspicious commands
grep "DENIED" ~/.clide/logs/clide.log
```

#### 4. Backup & Recovery
```bash
# Backup configuration weekly
cp config.yaml config.yaml.backup.$(date +%Y%m%d)

# Backup database
cp ~/.clide/memory.db ~/.clide/memory.db.backup.$(date +%Y%m%d)
```

#### 5. Principle of Least Privilege

Run clide as a non-root user. When using SSH, create a dedicated user on the remote host with minimal permissions, and point clide at that user's SSH key. Never run clide or SSH commands as `root`.

### For Developers

#### 1. Code Review Checklist
- [ ] No hardcoded credentials
- [ ] Input validation on all user input
- [ ] Parameterized queries for database
- [ ] Error messages don't leak sensitive info
- [ ] Secure random for cryptographic operations

#### 2. Secure Coding Patterns
```rust
// ✅ GOOD - Parameterized execution (no shell interpolation)
Command::new("ls").args(["-la", user_path]).output()?;

// ❌ BAD - Shell injection risk
Command::new("sh").args(["-c", &format!("ls -la {user_path}")]).output()?;
```

#### 3. Dependency Management
```bash
# Audit dependencies regularly
cargo audit

# Check for clippy lints (includes some security patterns)
cargo clippy -- -D warnings
```

---

## 🔒 Credential Management

### Storing Credentials Securely

#### Secrets file (`~/.clide/secrets.yaml`)
```bash
# File permissions protect at rest
chmod 600 ~/.clide/secrets.yaml

# Manage via CLI — values are hidden during input
clide secret set MY_API_KEY
clide secret list
clide secret get MY_API_KEY
```

#### Optional GPG encryption via GNU pass
```bash
# Set up GPG + pass for encrypted-at-rest secrets
clide secret pass-init

# Store a secret in pass (GPG-encrypted)
clide secret pass-set GEMINI_API_KEY
# secrets.yaml then holds: GEMINI_API_KEY: "pass:clide/gemini_api_key"
```

#### Vault backup (age encryption)
```bash
# Encrypt secrets + hosts and upload to GitHub Gist
clide vault backup
# Restore on a new machine
clide vault restore
```

### SSH Key Security

#### Generate Dedicated Keys
```bash
# Create key specifically for clide
ssh-keygen -t ed25519 -f ~/.ssh/clide_key -C "clide-bot"

# Set restrictive permissions
chmod 600 ~/.ssh/clide_key
```

#### Limit Key Permissions on VPS
```bash
# On your VPS, restrict what the key can do
# Add to ~/.ssh/authorized_keys with restrictions:
command="~/clide-allowed-commands.sh",no-port-forwarding,no-X11-forwarding,no-agent-forwarding ssh-ed25519 AAAA...
```

---

## 🚨 Incident Response

### If You Suspect a Security Issue

#### Immediate Actions
1. **Stop clide immediately**
   ```bash
   pkill -f clide
   ```

2. **Revoke API keys**
   - Google AI Studio → Revoke Gemini key
   - Generate new key

3. **Rotate messaging tokens**
   - Matrix: generate a new access token via API login
   - Telegram: revoke and regenerate via @BotFather → `/mybots` → API Token

4. **Review logs**
   ```bash
   grep "ERROR\|WARN\|DENIED" ~/.clide/logs/clide.log
   ```

5. **Check for unauthorized commands**
   ```bash
   # Review recent command history
   sqlite3 ~/.clide/memory.db "SELECT * FROM commands ORDER BY timestamp DESC LIMIT 100;"
   ```

#### Reporting Security Issues

**DO NOT** open a public GitHub issue for security vulnerabilities!

Instead:
1. Email: **create a [private security advisory on GitHub](https://github.com/juanitto-maker/Clide/security/advisories/new)**
2. Subject: "Security Issue in clide"
3. Include:
   - Description of vulnerability
   - Steps to reproduce
   - Potential impact
   - Your contact info (optional)

**Response SLA:**
- Initial response: Within 48 hours
- Fix timeline: Based on severity
- Public disclosure: After fix is released

---

## 🛠️ Security Configuration Examples

### High Security Profile
```yaml
# Paranoid mode - maximum security
require_confirmation: true
confirmation_timeout: 30
authorized_users:
  - "@youraccount:matrix.org"  # Only your account
blocked_commands:
  - "rm -rf /"
  - "mkfs"
  - "dd if="
  - "chmod 777"
logging:
  level: "debug"   # Log everything
```

### Balanced Profile (Recommended)
```yaml
# Good security without being annoying
require_confirmation: true
confirmation_timeout: 60
authorized_numbers:
  - "+1234567890"
logging:
  level: "info"
```

### Low Security Profile (Development only)
```yaml
# For local testing only - NOT for production!
require_confirmation: false
authorized_users: []  # ⚠️ Anyone can send commands
logging:
  level: "debug"
```

---

## 📋 Security Checklist

### Initial Setup
- [ ] Set up dedicated Matrix bot account or Telegram bot
- [ ] Created dedicated SSH keys
- [ ] Set restrictive file permissions (600)
- [ ] Configured `authorized_users` allowlist
- [ ] Enabled audit logging
- [ ] Backed up config.yaml securely

### Regular Maintenance
- [ ] Update clide weekly
- [ ] Review logs monthly
- [ ] Rotate API keys quarterly
- [ ] Backup database monthly
- [ ] Audit dependencies quarterly

### Before Production Use
- [ ] Tested with dry-run mode
- [ ] Reviewed all workflow templates
- [ ] Documented incident response plan
- [ ] Trained team on security practices
- [ ] Set up monitoring alerts

---

## 🔬 Security Audit

Want to audit clide's security yourself?

### Key Areas to Review
1. **Input validation** — `src/executor.rs`
2. **Credential storage** — `src/config.rs`, `src/pass_store.rs`
3. **Command execution** — `src/executor.rs`, `src/agent.rs`
4. **Secret redaction** — `src/scrubber.rs`
5. **API interactions** — `src/gemini.rs`, `src/matrix.rs`, `src/telegram.rs`

### Tools for Auditing
```bash
# Dependency vulnerability scan
cargo audit

# Static analysis / lints
cargo clippy -- -D warnings

# Check for unsafe code
grep -r "unsafe" src/
```

---

## 🌐 Network Security

### Network Permissions
```bash
# Clide only needs these network connections:
# - Matrix homeserver API (e.g. matrix.org)
# - Telegram Bot API (api.telegram.org)
# - Gemini API (generativelanguage.googleapis.com)
# - GitHub API (api.github.com) — only for vault backup/restore
# - SSH to your VPS (optional)

# No other outbound connections are made
```

### Firewall Rules (VPS)
```bash
# Only allow SSH from known IPs
ufw allow from YOUR_HOME_IP to any port 22

# Or use SSH keys only
ufw allow 22
```

---

## 📞 Contact

- 🔒 **Security Issues:** create a [private security advisory on GitHub](https://github.com/juanitto-maker/Clide/security/advisories/new)
- 🐛 **Bug Reports:** [GitHub Issues](https://github.com/juanitto-maker/Clide/issues)
- 💬 **General Questions:** [Discussions](https://github.com/juanitto-maker/Clide/discussions)

---

## 📜 Security Disclosure Policy

We follow responsible disclosure:

1. Report sent to create a [private security advisory on GitHub](https://github.com/juanitto-maker/Clide/security/advisories/new)
2. We acknowledge within 48 hours
3. We provide timeline for fix
4. We release patch
5. Public disclosure 7 days after patch

**Hall of Fame:** Security researchers who responsibly disclose vulnerabilities will be credited (with permission) in our Hall of Fame.

---

**Remember: Security is a journey, not a destination. Stay vigilant!** 🛡️
