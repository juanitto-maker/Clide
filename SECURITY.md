# 🔒 Clide Security Guide

Security best practices and threat model for Clide.

---

## 🎯 Security Philosophy

Clide is designed with **security by default** principles:

1. ✅ **End-to-end encryption** - Signal protocol
2. ✅ **Principle of least privilege** - No root required
3. ✅ **Safe defaults** - Confirmation mode enabled
4. ✅ **Audit logging** - All actions logged
5. ✅ **Secure storage** - Encrypted credentials
6. ✅ **Input validation** - All commands sanitized
7. ✅ **Sandboxed execution** - Limited permissions

---

## 🛡️ Threat Model

### What Clide Protects Against

✅ **Unauthorized access** - Signal authentication required
✅ **Command injection** - Input sanitization
✅ **Privilege escalation** - Runs as user, not root
✅ **Data exfiltration** - Logs all file access
✅ **Man-in-the-middle** - TLS for all API calls
✅ **Replay attacks** - Message timestamps validated

### What Clide Does NOT Protect Against

⚠️ **Compromised Signal account** - If attacker has your phone
⚠️ **Physical access** - If attacker has your device
⚠️ **Keyloggers** - If your system is compromised
⚠️ **Social engineering** - If you approve malicious commands
⚠️ **Zero-day exploits** - Unknown vulnerabilities

---

## 🔐 Configuration Security

### 1. Protect config.yaml

**File contains sensitive data:**
- Gemini API key
- Signal credentials
- SSH keys (if configured)

**Best practices:**

```bash
# Set restrictive permissions (owner read/write only)
chmod 600 ~/.clide/config.yaml

# Verify permissions
ls -l ~/.clide/config.yaml
# Should show: -rw------- (600)

# Never commit to git
echo "config.yaml" >> .gitignore
```

### 2. Environment Variables (Recommended)

Instead of storing secrets in config.yaml, use environment variables:

```bash
# Add to ~/.bashrc or ~/.zshrc
export CLIDE_GEMINI_API_KEY="your-api-key"
export CLIDE_SIGNAL_NUMBER="+1234567890"

# In config.yaml, reference them:
gemini_api_key: "${CLIDE_GEMINI_API_KEY}"
signal_number: "${CLIDE_SIGNAL_NUMBER}"
```

### 3. Encrypted Storage (Advanced)

Use system keyring for sensitive data:

```bash
# Store API key in system keyring
secret-tool store --label="Clide Gemini Key" service clide key gemini_api

# Configure clide to use keyring
clide config set-keyring true
```

---

## 🔑 Signal Security

### 1. Device Linking (Recommended)

**Why linking is more secure:**
- ✅ No SMS interception possible
- ✅ Requires physical access to primary device
- ✅ Can be revoked remotely
- ✅ Session-based authentication

**Setup:**
```bash
signal-cli link -n "clide-bot"
# Scan QR code from Signal app
```

**Revoke if compromised:**
1. Open Signal on phone
2. Settings → Linked Devices
3. Find "clide-bot"
4. Tap → Unlink

### 2. Number Registration (Less Secure)

**Risks:**
- ⚠️ SMS interception possible
- ⚠️ SIM swapping attacks
- ⚠️ Number recycling concerns

**Mitigation:**
- Use dedicated number for clide
- Enable 2FA on carrier account
- Monitor for unusual activity

### 3. Message Verification

Clide validates:
- ✅ Message sender (must be from authorized number)
- ✅ Message timestamp (must be recent, not replayed)
- ✅ Message format (must match expected schema)

**Configure authorized senders:**
```yaml
# In config.yaml
authorized_numbers:
  - "+1234567890"  # Your primary number
  - "+0987654321"  # Your backup number
```

---

## 🚨 Command Execution Security

### 1. Confirmation Mode (Recommended for Production)

**Enable confirmation before executing commands:**

```yaml
# In config.yaml
require_confirmation: true
confirmation_timeout: 60  # seconds
```

**How it works:**
1. User sends command via Signal
2. Clide sends confirmation request
3. User replies "yes" or "no"
4. Command executes only after "yes"
5. Times out after 60 seconds

**Example:**
```
You: restart nginx
Clide: ⚠️  Confirm: restart nginx? (yes/no)
You: yes
Clide: ✅ Executed: nginx restarted
```

### 2. Command Whitelist

**Restrict to specific commands:**

```yaml
# In config.yaml
allowed_commands:
  - "systemctl status *"
  - "docker ps"
  - "ls /var/www"
  - "tail -f /var/log/*.log"
  
# Deny everything else
deny_by_default: true
```

**Wildcard patterns:**
- `*` matches any characters
- `?` matches single character
- `[abc]` matches a, b, or c

### 3. Command Blacklist

**Block dangerous commands:**

```yaml
# In config.yaml (built-in defaults)
blocked_commands:
  - "rm -rf /*"
  - "dd if=*"
  - "mkfs.*"
  - "chmod 777 *"
  - "passwd*"
  - "userdel*"
  - "shutdown*"
  - "reboot"
```

### 4. Dry-Run Mode

**Test commands without executing:**

```yaml
# In config.yaml
dry_run: true
```

All commands will be logged but not executed. Useful for testing.

---

## 🌐 SSH Security

### 1. Host Restrictions

**Limit SSH to known servers:**

```yaml
# In config.yaml
allowed_ssh_hosts:
  - "production.example.com"
  - "staging.example.com"
  - "192.168.1.100"
```

### 2. Key-Based Authentication (Required)

**Never use password authentication:**

```bash
# Generate SSH key for clide
ssh-keygen -t ed25519 -f ~/.clide/ssh_key -N ""

# Copy to server
ssh-copy-id -i ~/.clide/ssh_key user@server

# Configure clide
clide config set-ssh-key ~/.clide/ssh_key
```

**In config.yaml:**
```yaml
ssh_key_path: "~/.clide/ssh_key"
ssh_verify_host_keys: true
```

### 3. Command Restrictions on Remote Hosts

**Use ForceCommand in SSH authorized_keys:**

```bash
# On remote server: ~/.ssh/authorized_keys
command="/usr/local/bin/clide-remote-wrapper" ssh-ed25519 AAAA...
```

**Example wrapper script:**
```bash
#!/bin/bash
# /usr/local/bin/clide-remote-wrapper

case "$SSH_ORIGINAL_COMMAND" in
  "systemctl status "*) $SSH_ORIGINAL_COMMAND ;;
  "docker ps"*) $SSH_ORIGINAL_COMMAND ;;
  "ls "*) $SSH_ORIGINAL_COMMAND ;;
  *) echo "Command not allowed"; exit 1 ;;
esac
```

---

## 🔍 Audit Logging

### 1. Enable Comprehensive Logging

```yaml
# In config.yaml
logging:
  level: "info"  # debug, info, warn, error
  file: "~/.clide/logs/clide.log"
  max_size: "100MB"
  max_backups: 10
  
  # Log all commands
  log_commands: true
  
  # Log command output
  log_output: true
  
  # Log API calls
  log_api_calls: true
```

### 2. Log Format

**Structured JSON logs for easy parsing:**

```json
{
  "timestamp": "2026-02-15T10:30:00Z",
  "level": "INFO",
  "event": "command_executed",
  "user": "+1234567890",
  "command": "systemctl status nginx",
  "exit_code": 0,
  "duration_ms": 234,
  "host": "localhost"
}
```

### 3. Monitor Logs

**Watch for suspicious activity:**

```bash
# Real-time monitoring
tail -f ~/.clide/logs/clide.log | jq .

# Filter for errors
grep ERROR ~/.clide/logs/clide.log

# Check for failed commands
jq 'select(.exit_code != 0)' ~/.clide/logs/clide.log

# List all users
jq -r '.user' ~/.clide/logs/clide.log | sort -u
```

### 4. Log Rotation

**Automatic log rotation configured by default:**

```yaml
# In config.yaml
logging:
  rotate_on_size: "100MB"
  max_files: 10
  compress_old: true
```

**Manual rotation:**
```bash
clide logs rotate
```

---

## 🔐 API Key Security

### 1. Gemini API Key Protection

**Best practices:**

✅ **Never commit to git**
```bash
echo "config.yaml" >> .gitignore
echo ".env" >> .gitignore
```

✅ **Use environment variables**
```bash
export CLIDE_GEMINI_API_KEY="your-key"
```

✅ **Rotate regularly**
```bash
# Generate new key at: https://makersuite.google.com/app/apikey
clide config set-gemini-key "new-key"
# Old key automatically revoked after 24 hours
```

✅ **Monitor usage**
```bash
# Check API usage at Google AI Studio
# Set up alerts for unusual activity
```

### 2. API Key Compromise Response

**If API key is compromised:**

1. **Immediately revoke:** Visit Google AI Studio → API Keys → Revoke
2. **Generate new key:** Create new key
3. **Update config:** `clide config set-gemini-key "new-key"`
4. **Review logs:** Check for unauthorized API calls
5. **Notify team:** If shared project

---

## 🛡️ Network Security

### 1. TLS/HTTPS

**All external communication uses TLS:**
- ✅ Signal protocol (end-to-end encrypted)
- ✅ Gemini API (HTTPS)
- ✅ SSH (encrypted by default)

**Verify certificates:**
```yaml
# In config.yaml
verify_ssl: true  # Never disable in production!
```

### 2. Firewall Rules

**Recommended firewall configuration:**

```bash
# Allow only necessary outbound connections
sudo ufw default deny outgoing
sudo ufw allow out to api.gemini.google.com port 443
sudo ufw allow out to signal.org port 443
sudo ufw allow out 22/tcp  # SSH

# Block all inbound (clide doesn't need inbound)
sudo ufw default deny incoming
```

### 3. Proxy Support

**Route through proxy if required:**

```yaml
# In config.yaml
proxy:
  http: "http://proxy.example.com:8080"
  https: "https://proxy.example.com:8443"
  no_proxy: "localhost,127.0.0.1"
```

---

## 🚨 Incident Response

### If You Suspect Compromise

**Immediate actions:**

1. **Stop clide:**
```bash
clide stop
```

2. **Revoke Signal device:**
- Open Signal → Settings → Linked Devices
- Unlink "clide-bot"

3. **Rotate API keys:**
- Gemini API key
- SSH keys

4. **Review logs:**
```bash
grep -i "error\|fail\|denied" ~/.clide/logs/clide.log
```

5. **Check system:**
```bash
# Check for unauthorized changes
sudo auditctl -l
last -f /var/log/wtmp
```

6. **Secure system:**
```bash
# Change passwords
passwd

# Update all packages
sudo apt update && sudo apt upgrade -y

# Check for rootkits
sudo rkhunter --check
```

---

## ✅ Security Checklist

**Before deploying Clide to production:**

- [ ] Config file has 600 permissions
- [ ] API keys in environment variables (not config file)
- [ ] Confirmation mode enabled
- [ ] Command whitelist configured
- [ ] Dangerous commands blacklisted
- [ ] SSH key-based auth only (no passwords)
- [ ] SSH host keys verified
- [ ] Authorized Signal numbers configured
- [ ] Comprehensive logging enabled
- [ ] Log monitoring set up
- [ ] Regular security updates scheduled
- [ ] Backup and recovery plan documented
- [ ] Incident response plan in place

---

## 📚 Additional Resources

- [Signal Security Whitepaper](https://signal.org/docs/)
- [OWASP Security Guidelines](https://owasp.org/)
- [Rust Security Best Practices](https://anssi-fr.github.io/rust-guide/)
- [SSH Hardening Guide](https://stribika.github.io/2015/01/04/secure-secure-shell.html)

---

## 🐛 Report Security Issues

**Found a security vulnerability?**

**DO NOT** open a public GitHub issue!

Instead:
1. Email: security@yourproject.com
2. Use PGP key: [link to public key]
3. We'll respond within 24 hours
4. Coordinated disclosure after fix

**Bug bounty:** Security researchers welcome!

---

## 📋 Security Updates

Subscribe to security advisories:
- GitHub: Watch → Custom → Security alerts
- RSS: https://github.com/yourusername/clide/security/advisories.atom
- Email: security-announce@yourproject.com

---

**Security is a shared responsibility. Stay vigilant!** 🛡️
