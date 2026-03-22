# Clide Security Guide

Security best practices and threat model for Clide.

---

## Security Philosophy

Clide is designed with **security by default** principles:

1. **Principle of least privilege** — no root required
2. **Fail-closed auth** — unauthorized messages are rejected, not silently dropped
3. **Safe defaults** — confirmation mode available, dangerous commands blocklisted
4. **Audit logging** — all commands logged with timestamps
5. **Secret separation** — credentials stored separately from config, auto-redacted before AI
6. **Input validation** — all commands sanitized before execution

---

## Threat Model

### What Clide Protects Against

- **Unauthorized access** — Matrix user ID / Telegram username allowlist
- **Command injection** — input sanitization, parameterized execution
- **Privilege escalation** — runs as user, not root
- **Credential leakage** — secret scrubber redacts values before AI prompts and chat
- **Man-in-the-middle** — TLS for all API calls (Matrix, Telegram, Gemini)
- **Accidental destruction** — command blocklist, confirmation mode, rollback support

### What Clide Does NOT Protect Against

- **Compromised device** — if attacker has root access to your device
- **Physical access** — if attacker has your unlocked device
- **Social engineering** — if you approve malicious commands
- **Lost vault passphrase** — no recovery mechanism
- **AI prompt injection** — while mitigated, adversarial inputs could influence AI behavior

---

## Configuration Security

### 1. Protect Your Files

```bash
# Set restrictive permissions
chmod 700 ~/.clide
chmod 600 ~/.clide/config.yaml
chmod 600 ~/.clide/secrets.yaml
chmod 600 ~/.clide/hosts.yaml

# Verify
ls -la ~/.clide/
# All files should show -rw------- (600)
```

### 2. Secrets Management

Clide separates secrets from config. API keys and tokens live in `~/.clide/secrets.yaml`, not in `config.yaml`.

**Priority (highest wins):**
```
env var  >  secrets.yaml  >  config.yaml  >  built-in default
```

**Storage options:**

| Option | At-rest protection | Best for |
|---|---|---|
| `secrets.yaml` | File permissions (`chmod 600`) | Most users |
| GNU pass layer | GPG encryption | Shared machines, high-security |
| Environment variables | Process memory only | CI/CD, Docker |

**Managing secrets:**
```bash
clide secret set MY_API_KEY       # hidden input
clide secret list                  # key names only, no values
clide secret pass-init             # set up GPG encryption
clide secret pass-set MY_API_KEY   # migrate to GPG store
```

### 3. Vault Backup

Back up secrets and SSH hosts to an encrypted GitHub Gist:

```bash
clide vault backup    # encrypts with age, uploads to Gist
clide vault restore   # restores on a new machine
```

The vault is encrypted with [age](https://age-encryption.org/) (ChaCha20-Poly1305 + Argon2). The passphrase is never stored anywhere.

---

## Messaging Security

### Matrix / Element

- Access token auth — your password is never stored
- Rotate tokens periodically: re-login via API or Element settings
- Use a dedicated bot account, not your personal one

### Telegram

- Bot token from @BotFather stored in `secrets.yaml`
- Regenerate via @BotFather > `/mybots` > API Token if compromised

### Access Control

```yaml
# config.yaml — restrict to your accounts only
authorized_users:
  - "@youraccount:matrix.org"     # Matrix user ID
  - "your_telegram_username"       # Telegram username
```

An empty list blocks everyone. Unauthorized messages are rejected with feedback.

---

## Command Execution Security

### 1. Confirmation Mode

```yaml
# config.yaml
require_confirmation: true
```

Every command requires explicit `YES` before execution.

### 2. Command Blocklist

Built-in blocklist prevents dangerous patterns:
```
rm -rf /    mkfs    dd if=    :(){ :|:& };:    chmod -R 777 /
```

### 3. Secret Scrubber

All outbound text is auto-redacted before reaching the AI or chat. The AI only sees `${KEY_NAME}` placeholders — real values are substituted by the Executor after the AI returns the command.

---

## SSH Security

### Host Registry

Register servers by nickname — IPs and keys never appear in chat or AI prompts:

```bash
clide host add prod --ip 1.2.3.4 --user root --key ~/.ssh/id_ed25519_prod
```

Skills reference hosts as `${HOST_PROD_IP}`, `${HOST_PROD_USER}`, etc.

### Key-Based Auth

```bash
# Generate a dedicated key for Clide
ssh-keygen -t ed25519 -f ~/.ssh/clide_key -C "clide-bot"
chmod 600 ~/.ssh/clide_key
```

### Restrict Remote Key Permissions

```bash
# On remote server: ~/.ssh/authorized_keys
command="~/clide-allowed-commands.sh",no-port-forwarding,no-X11-forwarding ssh-ed25519 AAAA...
```

---

## Audit Logging

Clide logs every command with timestamps via `tracing`:

```bash
# Debug mode for verbose output
RUST_LOG=debug clide bot

# Review logs
tail -f ~/.clide/logs/clide.log
```

Command output is capped to prevent OOM on resource-constrained devices (Termux).

---

## Incident Response

### If You Suspect Compromise

1. **Stop Clide immediately:**
   ```bash
   pkill -f clide
   ```

2. **Revoke tokens:**
   - Gemini: revoke at [aistudio.google.com/app/apikey](https://aistudio.google.com/app/apikey)
   - Matrix: log out all sessions in Element settings
   - Telegram: regenerate token via @BotFather

3. **Review logs:**
   ```bash
   grep "ERROR\|WARN\|DENIED" ~/.clide/logs/clide.log
   ```

4. **Rotate credentials:**
   ```bash
   clide secret set GEMINI_API_KEY
   clide secret set MATRIX_ACCESS_TOKEN
   clide secret set TELEGRAM_BOT_TOKEN
   ```

5. **Check system integrity:**
   ```bash
   last -f /var/log/wtmp
   sudo auditctl -l
   ```

---

## Security Checklist

### Initial Setup
- [ ] Dedicated bot account (Matrix and/or Telegram)
- [ ] `authorized_users` set to your accounts only
- [ ] File permissions: `chmod 600` on all files in `~/.clide/`
- [ ] SSH keys: dedicated key for Clide, `chmod 600`
- [ ] `require_confirmation: true` on production machines
- [ ] Vault backup created and passphrase stored safely

### Regular Maintenance
- [ ] Update Clide weekly
- [ ] Review logs monthly
- [ ] Rotate API keys quarterly
- [ ] Rotate Matrix/Telegram tokens after any incident
- [ ] Run `cargo audit` on dependencies quarterly

---

## Report Security Issues

**DO NOT** open a public GitHub issue for security vulnerabilities.

Instead, create a [private security advisory](https://github.com/juanitto-maker/Clide/security/advisories/new).

**Response SLA:**
- Initial response: within 48 hours
- Fix timeline: based on severity
- Public disclosure: after fix is released

---

## Additional Resources

- [Matrix Security](https://matrix.org/docs/guides/end-to-end-encryption-implementation-guide)
- [OWASP Security Guidelines](https://owasp.org/)
- [Rust Security Best Practices](https://anssi-fr.github.io/rust-guide/)
- [SSH Hardening Guide](https://stribika.github.io/2015/01/04/secure-secure-shell.html)
- [age encryption](https://age-encryption.org/)
