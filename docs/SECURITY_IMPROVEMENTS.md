# Security Improvements for Clide

Status of security hardening efforts and future improvements.

Last updated: 2026-03-22

---

## Implemented

These security measures are shipped and available in the current release.

### Credential Management (v0.3.0+)

| Feature | Status | Details |
|---------|--------|---------|
| Centralized secrets file | Shipped | `~/.clide/secrets.yaml` — separate from config |
| File permissions | Shipped | Installer sets `chmod 600` on secrets and config |
| Hidden input | Shipped | `clide secret set` uses hidden terminal input |
| GNU pass / GPG encryption | Shipped | Optional encrypted-at-rest layer via `clide secret pass-init` |
| Age-encrypted vault backup | Shipped | `clide vault backup/restore` to GitHub Gist |
| SSH keys in vault | Shipped | `~/.ssh/` keys included in vault archive |
| Secret scrubber | Shipped | Auto-redacts all secret values before AI prompts and chat |
| Env var priority | Shipped | `env var > secrets.yaml > config.yaml > default` |
| Secrets as env vars in skills | Shipped | `config.secrets` exported to child shell processes |

### Access Control (v0.3.0+)

| Feature | Status | Details |
|---------|--------|---------|
| User allowlist | Shipped | `authorized_users` — Matrix IDs and Telegram usernames |
| Fail-closed auth | Shipped | Unauthorized messages rejected with feedback (not silently dropped) |
| Command blocklist | Shipped | Dangerous patterns blocked before execution |
| Confirmation mode | Shipped | `require_confirmation: true` for production machines |
| AI prompt rules | Shipped | System prompt instructs AI to never leak secrets |
| Bot self-response prevention | Shipped | Bot resolves own user ID via `/whoami` to avoid loops |

### Audit & Logging (v0.3.0+)

| Feature | Status | Details |
|---------|--------|---------|
| Structured logging | Shipped | `tracing` with timestamps, log levels, file appender |
| Command output capping | Shipped | Prevents OOM on Termux from large outputs |
| YAML control-char stripping | Shipped | Invalid chars removed on config load |

---

## Future Improvements

### High Priority

| Improvement | Description | Complexity |
|-------------|-------------|------------|
| Token rotation CLI | `clide secret rotate` — auto-regenerate Matrix/Telegram tokens | Medium |
| Audit log export | Export command history as structured JSON for SIEM ingestion | Low |
| Rate limiting | Throttle commands per user per minute to prevent abuse | Low |

### Medium Priority

| Improvement | Description | Complexity |
|-------------|-------------|------------|
| Android Keystore | Use Termux API + Android hardware-backed key storage | Medium |
| Encrypted config at rest | AES-256-GCM encryption of `config.yaml` with master passphrase | Medium |
| Per-host command allowlists | Restrict which commands can run on each SSH host | Medium |
| MFA for destructive commands | Require a second-factor confirmation for `rm`, `mkfs`, etc. | High |

### Low Priority

| Improvement | Description | Complexity |
|-------------|-------------|------------|
| SELinux/AppArmor profiles | Confine Clide process with mandatory access control | High |
| Network policy | Restrict outbound connections to known endpoints only | Medium |
| Signed skill bundles | Verify skill YAML integrity before execution | Medium |

---

## Current Risk Assessment

| Risk | Severity | Mitigation |
|------|----------|------------|
| Secrets in `secrets.yaml` are plaintext at rest | Low | `chmod 600` + optional GPG via pass |
| Compromised device can read secrets from RAM | Low | Use GPG + short `gpg-agent` cache timeout |
| Stolen GitHub token could access vault Gist | Low | Vault is age-encrypted; passphrase required |
| AI could be prompt-injected to leak secrets | Low | Scrubber redacts before AI sees values; AI rules forbid leaking |
| Lost vault passphrase = lost backup | Medium | No recovery possible — store passphrase in a separate password manager |

---

## Best Practices

**Secrets storage:**
- Use `clide secret set` (hidden input) instead of editing YAML directly
- Enable GNU pass for GPG encryption on shared machines
- Back up vault before wiping devices

**Access control:**
- Keep `authorized_users` as tight as possible
- Use `require_confirmation: true` on production
- Use a dedicated bot account, not your personal one

**Rotation:**
- Rotate Gemini API keys quarterly
- Rotate Matrix access tokens after any suspected compromise
- Regenerate Telegram bot tokens via @BotFather if leaked

**File permissions:**
```bash
chmod 700 ~/.clide
chmod 600 ~/.clide/config.yaml
chmod 600 ~/.clide/secrets.yaml
chmod 600 ~/.clide/hosts.yaml
```
