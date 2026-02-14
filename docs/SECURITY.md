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
```python
# Automatic blocking of dangerous patterns
BLOCKED_PATTERNS = [
    "rm -rf /",           # System wipe
    "dd if=/dev/zero",    # Disk destruction
    "mkfs",               # Format filesystem
    ":(){ :|:& };:",      # Fork bomb
    "chmod -R 777 /",     # Unsafe permissions
]
```

#### 2. Confirmation Requirements
- Destructive operations always require explicit confirmation
- Modifying system files triggers approval prompt
- User/group changes need verification
- Firewall modifications are reviewed before execution

#### 3. Credential Encryption
```yaml
# All sensitive data is encrypted at rest
credentials:
  encryption: "AES-256-GCM"
  key_derivation: "PBKDF2-SHA256"
  iterations: 100000
```

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
gemini:
  api_key: "YOUR_KEY_HERE"  # Replace before committing

# ✅ Use environment variables instead
gemini:
  api_key: "${GEMINI_API_KEY}"
```

#### Git Protection
```bash
# Add to .gitignore
echo "config.yaml" >> .gitignore
echo "*.db" >> .gitignore
echo "*.log" >> .gitignore
```

### Signal Account Security

#### Best Practices
1. **Use a dedicated number** - Don't use your primary Signal account
2. **Enable registration lock** - Prevents unauthorized re-registration
3. **Link, don't register** - Link as secondary device when possible
4. **PIN protection** - Set a strong Signal PIN

#### Limiting Access
```yaml
# config.yaml - Restrict to your messages only
signal:
  admin_only: true
  allowed_numbers:
    - "+1234567890"  # Your number only
```

---

## 🚨 Threat Model

### What clide Protects Against

✅ **Command Injection**
- Input sanitization
- Parameterized execution
- No shell string concatenation

✅ **Unauthorized Access**
- Phone number whitelist
- Admin-only mode
- No public API endpoints

✅ **Credential Theft**
- Encrypted storage
- Never logged in plaintext
- Memory cleared after use

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
```bash
# Use strong Signal PIN
Signal → Settings → Account → Signal PIN
# Use 6+ digits, not birthday/phone number
```

#### 2. Regular Updates
```bash
# Update clide weekly
cd clide && git pull origin main

# Update dependencies
pip install -r requirements.txt --upgrade
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
```yaml
# Don't give clide more access than needed
vps:
  - name: "production"
    user: "clide-bot"  # Not root!
    ssh_key: "~/.ssh/clide_readonly"  # Limited permissions
```

### For Developers

#### 1. Code Review Checklist
- [ ] No hardcoded credentials
- [ ] Input validation on all user input
- [ ] Parameterized queries for database
- [ ] Error messages don't leak sensitive info
- [ ] Secure random for cryptographic operations

#### 2. Secure Coding Patterns
```python
# ✅ GOOD - Parameterized execution
subprocess.run(["ls", "-la", user_path], check=True)

# ❌ BAD - Shell injection risk
subprocess.run(f"ls -la {user_path}", shell=True)
```

#### 3. Dependency Management
```bash
# Audit dependencies regularly
pip-audit

# Check for known vulnerabilities
safety check
```

---

## 🔒 Credential Management

### Storing Credentials Securely

#### Encryption Implementation
```python
# clide uses industry-standard encryption
from cryptography.fernet import Fernet

# Key derived from user passphrase
key = derive_key(passphrase, salt, iterations=100000)

# Encrypt sensitive data
encrypted = Fernet(key).encrypt(credential.encode())
```

#### User Passphrase
```bash
# Set during first run
clide setup

# You'll be prompted for a master passphrase
# This encrypts all stored credentials
# NEVER forget this passphrase!
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

3. **Change Signal PIN**
   - Signal → Settings → Account → Signal PIN

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
1. Email: **security@yourproject.com**
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
safety:
  dry_run_default: true  # Always preview first
  confirm_destructive: true
  confirm_all: true  # Confirm every command
  auto_backup: true
  max_retries: 1  # No automatic retries
  
logging:
  level: "DEBUG"  # Log everything
  
signal:
  admin_only: true
  allowed_numbers:
    - "+1234567890"  # Only your number
```

### Balanced Profile (Recommended)
```yaml
# Good security without being annoying
safety:
  dry_run_default: false
  confirm_destructive: true
  auto_backup: true
  max_retries: 3
  
signal:
  admin_only: true
```

### Low Security Profile (Development)
```yaml
# For testing only - NOT for production!
safety:
  dry_run_default: false
  confirm_destructive: false  # ⚠️ Dangerous!
  auto_backup: false
  
# DO NOT use this in production!
```

---

## 📋 Security Checklist

### Initial Setup
- [ ] Generated strong Signal PIN
- [ ] Created dedicated SSH keys
- [ ] Set restrictive file permissions (600)
- [ ] Configured admin_only mode
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
1. **Input validation** - `src/safety.py`
2. **Credential storage** - `src/memory.py`
3. **Command execution** - `src/executor.py`
4. **API interactions** - `src/brain.py`

### Tools for Auditing
```bash
# Static analysis
bandit -r src/

# Dependency vulnerabilities
safety check

# Code quality
pylint src/
```

---

## 🌐 Network Security

### Termux Network Permissions
```bash
# clide only needs these network permissions:
# - Signal API (signal-cli)
# - Gemini API (ai.google.dev)
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

- 🔒 **Security Issues:** security@yourproject.com
- 🐛 **Bug Reports:** [GitHub Issues](https://github.com/yourusername/clide/issues)
- 💬 **General Questions:** [Discussions](https://github.com/yourusername/clide/discussions)

---

## 📜 Security Disclosure Policy

We follow responsible disclosure:

1. Report sent to security@yourproject.com
2. We acknowledge within 48 hours
3. We provide timeline for fix
4. We release patch
5. Public disclosure 7 days after patch

**Hall of Fame:** Security researchers who responsibly disclose vulnerabilities will be credited (with permission) in our Hall of Fame.

---

**Remember: Security is a journey, not a destination. Stay vigilant!** 🛡️
