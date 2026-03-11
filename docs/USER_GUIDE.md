# Clide User Guide

This guide walks you through everything you need to know to use Clide day-to-day — from starting the bot to managing credentials and automating your servers.

---

## Contents

- [What is Clide?](#what-is-clide)
- [First run](#first-run)
- [Sending commands](#sending-commands)
  - [Via Telegram](#via-telegram)
  - [Via Element / Matrix](#via-element--matrix)
  - [Via interactive REPL](#via-interactive-repl)
- [Understanding skills](#understanding-skills)
  - [Running a skill](#running-a-skill)
  - [Writing your own skill](#writing-your-own-skill)
- [Managing credentials](#managing-credentials)
  - [Storing a new secret](#storing-a-new-secret)
  - [Listing secrets](#listing-secrets)
  - [Retrieving a secret](#retrieving-a-secret)
  - [Generating a random credential](#generating-a-random-credential)
  - [Using secrets in skills](#using-secrets-in-skills)
- [GPG encryption with GNU pass](#gpg-encryption-with-gnu-pass)
- [Managing SSH hosts](#managing-ssh-hosts)
  - [Adding a server](#adding-a-server)
  - [Using a host in skills](#using-a-host-in-skills)
  - [Removing a host](#removing-a-host)
- [Backing up and restoring secrets](#backing-up-and-restoring-secrets)
  - [Vault backup](#vault-backup)
  - [Vault restore](#vault-restore)
- [Remote server operations](#remote-server-operations)
- [Security tips](#security-tips)
- [Troubleshooting](#troubleshooting)

---

## What is Clide?

Clide is a Rust binary that runs on your machine (Linux, macOS, Android/Termux) and acts as:

1. **A terminal AI assistant** — send natural language commands from Telegram or Element, get real shell output back.
2. **A credential manager** — store API keys, tokens, and passwords securely; use them in automations without ever exposing them to the AI.
3. **An SSH automation hub** — register your servers by nickname, then control them from your phone with plain English.
4. **A skills engine** — write reusable YAML workflows that the AI can invoke on your behalf.

Everything runs locally as a single static binary. There is no cloud component, no SaaS subscription, no telemetry.

---

## First run

After installation, start the bot:

```bash
clide bot
```

You should see coloured log output confirming which platforms are connected:

```
[INFO] Matrix client connected as @mybot:matrix.org
[INFO] Telegram bot polling started
[INFO] Clide ready. Listening for messages...
```

If you only want the interactive terminal prompt (no bot, no messaging):

```bash
clide
```

---

## Sending commands

### Via Telegram

1. Open Telegram and find the bot you created with @BotFather.
2. Send any message — Clide will interpret it with Gemini AI and run the appropriate command.

**Examples:**
```
you:   how much disk space is free?
clide: df -h output → /dev/sda1  50G  12G  38G  24% /

you:   restart nginx
clide: sudo systemctl restart nginx → [ok]

you:   run the backup skill
clide: Running backup... done. Archive saved to /backups/2024-01-15.tar.gz
```

Only users listed in `authorized_users` in your config can issue commands. Anyone else is silently ignored.

### Via Element / Matrix

Same as Telegram — type in the configured Matrix room and Clide replies. Messages are end-to-end encrypted by the Matrix protocol.

### Via interactive REPL

Run `clide` (no arguments) for a local terminal session with Gemini. No bot is started; commands run directly on your machine.

```
> disk usage on /var/log
Running: du -sh /var/log
1.2G    /var/log

> what processes are using the most CPU?
Running: ps aux --sort=-%cpu | head -10
...
```

Press `Ctrl+C` or `Ctrl+D` to exit.

---

## Understanding skills

Skills are YAML files that define reusable command workflows. Instead of asking the AI to improvise shell commands every time, you can give it a set of pre-approved, parameterised scripts to choose from.

Skills live in `~/.clide/skills/` (installed by the installer) or in the `skills/` directory of your Clide source tree.

### Running a skill

Just describe what you want in natural language — the AI picks the right skill automatically:

```
you:   harden this VPS
clide: [runs Security/vps_hardening skill]

you:   give me a system overview
clide: [runs System/system_overview skill]

you:   check uptime on prod
clide: [runs the SSH uptime check using the 'prod' host entry]
```

### Writing your own skill

Create a YAML file anywhere under `~/.clide/skills/`:

```yaml
# ~/.clide/skills/my_app/deploy.yaml
name: deploy_my_app
description: "Pull latest code and restart the app on production"
parameters:
  branch:
    description: "Git branch to deploy"
    default: "main"
commands:
  - "ssh -i ${HOST_PROD_KEY_PATH} ${HOST_PROD_USER}@${HOST_PROD_IP} 'cd /srv/myapp && git pull origin {{branch}} && systemctl restart myapp'"
timeout: 120
```

Key syntax:
- `{{parameter}}` — replaced with the value provided at call time (or the default)
- `${SECRET_NAME}` — replaced with the value from `secrets.yaml` or the environment (never visible to AI)

Restart Clide after adding skills — it loads them at startup.

---

## Managing credentials

Clide includes a built-in credential manager accessible via `clide secret`. Think of it as a local, private alternative to KeePass or 1Password, purpose-built for your automation secrets.

### Storing a new secret

```bash
clide secret set GITHUB_TOKEN
```

You will be prompted to:
1. Enter the value (hidden input — characters are not echoed to the terminal).
2. Choose where to store it: `secrets.yaml` (plain text, file-permission protected) or GNU pass (GPG-encrypted, if installed).

### Listing secrets

```bash
clide secret list
```

Prints all key names. Values are never shown. You can see at a glance which keys are set, empty, or stored in pass:

```
Stored secrets (keys only):

  GEMINI_API_KEY          [set]
  ANTHROPIC_API_KEY       [set]
  MATRIX_ACCESS_TOKEN     → pass:clide/matrix_access_token
  MY_DB_PASSWORD          (empty)
```

### Retrieving a secret

```bash
clide secret get GEMINI_API_KEY
```

Prints the value to stdout. If the secret is a `pass:` reference, it is decrypted on the fly.

> Use this sparingly — don't pipe it into logs or other commands that might record the output.

### Generating a random credential

```bash
clide secret generate DB_PASSWORD        # 32 chars (default)
clide secret generate SESSION_SECRET 64  # 64 chars
```

Generates a cryptographically random alphanumeric string, stores it in `secrets.yaml` (or pass), and prints it **once**. Copy it now — it won't be shown again in plaintext.

### Using secrets in skills

Any key in `secrets.yaml` can be used in skills with the `${KEY_NAME}` syntax:

```yaml
commands:
  - "curl -H 'Authorization: Bearer ${GITHUB_TOKEN}' https://api.github.com/user"
  - "psql -U ${DB_USER} -h ${DB_HOST} -d mydb -c 'SELECT count(*) FROM users'"
```

**The AI only ever sees the placeholder** — `${GITHUB_TOKEN}` — not the real token. Substitution happens inside the Executor, after the AI has returned the command string.

---

## GPG encryption with GNU pass

For higher-security setups (shared machines, servers you don't fully trust), you can store secrets in [GNU pass](https://www.passwordstore.org/) — a GPG-encrypted password manager that is the standard UNIX credential store.

### Setup (one-time)

```bash
# Step 1: install dependencies (Termux)
pkg install gnupg pass

# Step 1 (Debian/Ubuntu)
sudo apt install gnupg pass

# Step 2: run the guided wizard
clide secret pass-init
# → lists your GPG keys, lets you create one, initialises the pass store
```

### Moving a secret to pass

```bash
# Option A: when setting a new secret, choose option 2 at the prompt
clide secret set MY_WEBHOOK_URL

# Option B: migrate an existing YAML secret to pass
clide secret pass-set GEMINI_API_KEY
```

After migration, `secrets.yaml` looks like:
```yaml
GEMINI_API_KEY: "pass:clide/gemini_api_key"
```

Clide calls `pass show clide/gemini_api_key` at startup to decrypt the value. The plaintext value lives in RAM only while Clide is running.

### Benefits

- Secrets at rest are GPG-encrypted inside `~/.password-store/`
- You can audit keys with `pass ls` without decrypting
- The `gpg-agent` caches your passphrase so you don't need to re-enter it for every run
- Pass has its own optional git sync (`pass git push`) for multi-device access

---

## Managing SSH hosts

The SSH host registry lets you give servers friendly nicknames. Instead of remembering `root@1.2.3.4 -i ~/.ssh/id_ed25519_prod -p 2222`, you register the server once and reference it as `prod` everywhere.

### Adding a server

**Interactive wizard (recommended):**

```bash
clide host add
```

Walks you through: nickname, IP or Tailscale address, SSH user, key path, port, notes.

**Non-interactive (scriptable):**

```bash
clide host add prod \
  --ip 1.2.3.4 \
  --user root \
  --key ~/.ssh/id_ed25519_prod \
  --port 22 \
  --notes "Hetzner VPS"
```

Clide confirms the variables that are now available:

```
✅ Host 'prod' saved to ~/.clide/hosts.yaml
   Skills can reference it as: ${HOST_PROD_IP}  ${HOST_PROD_USER}  ${HOST_PROD_KEY_PATH}  ${HOST_PROD_PORT}
```

**Tailscale example:**

```bash
clide host add homepi --ip 100.64.0.5 --user pi --key ~/.ssh/id_ed25519_pi --notes "Pi via Tailscale"
```

### Using a host in skills

```yaml
commands:
  - "ssh -i ${HOST_PROD_KEY_PATH} -p ${HOST_PROD_PORT} ${HOST_PROD_USER}@${HOST_PROD_IP} 'uptime'"
  - "ssh -i ${HOST_HOMEPI_KEY_PATH} ${HOST_HOMEPI_USER}@${HOST_HOMEPI_IP} 'vcgencmd measure_temp'"
```

You can also ask the bot in plain language:

```
you:   what's the CPU temp on homepi?
clide: [uses HOST_HOMEPI_* variables, SSHs in, returns temp]
```

### Listing hosts

```bash
clide host list
```

```
Configured hosts:
  homepi        pi@100.64.0.5:22  key=~/.ssh/id_ed25519_pi  # Pi via Tailscale
  prod          root@1.2.3.4:22   key=~/.ssh/id_ed25519_prod  # Hetzner VPS
```

### Removing a host

```bash
clide host remove prod
# or
clide host rm prod
```

---

## Backing up and restoring secrets

The vault feature encrypts your `secrets.yaml` and `hosts.yaml` with [age](https://age-encryption.org/) and stores the ciphertext in a GitHub Gist. Use it when setting up a new device or recovering from a lost phone.

### Vault backup

```bash
clide vault backup
```

You will be prompted for:
- **GitHub personal access token** — needs `gist` scope. Create one at [github.com/settings/tokens](https://github.com/settings/tokens).
- **Passphrase** — used to encrypt the archive. **Not stored anywhere.** Write it down.

On success, Clide prints the Gist ID:
```
✅ Vault backed up. Gist ID: abc123def456
   Store this ID and your passphrase somewhere safe.
```

**Recommendation:** store the Gist ID and passphrase in a physical notebook or a separate password manager (e.g. Bitwarden).

### Vault restore

```bash
clide vault restore
```

Prompts for GitHub token, Gist ID, and passphrase. Restores both files to `~/.clide/` and applies `chmod 600`.

**Restore during a fresh install:**

```bash
curl -fsSL https://raw.githubusercontent.com/juanitto-maker/Clide/main/install.sh | bash --restore
```

The installer will ask for your vault credentials before completing setup, so Clide comes up fully configured on the new machine.

### Security of the vault

- The Gist content is always encrypted — even if the Gist is public, the data is unreadable without your passphrase.
- The passphrase is never stored by Clide, GitHub, or anyone else.
- age uses modern cryptography (ChaCha20-Poly1305 with Argon2 key derivation).

---

## Remote server operations

Clide's real power is managing multiple servers from your phone. Here is a typical workflow:

**1. Register your servers:**

```bash
clide host add prod --ip 203.0.113.10 --user deploy --key ~/.ssh/id_ed25519_prod
clide host add staging --ip 203.0.113.20 --user deploy --key ~/.ssh/id_ed25519_staging
clide host add db --ip 203.0.113.30 --user postgres --key ~/.ssh/id_ed25519_db --port 2222
```

**2. Write a deployment skill:**

```yaml
# ~/.clide/skills/deploy.yaml
name: deploy
description: "Deploy the app to a target environment"
parameters:
  target:
    description: "Environment to deploy to: prod or staging"
    default: "staging"
commands:
  - "ssh -i ${HOST_PROD_KEY_PATH} ${HOST_PROD_USER}@${HOST_PROD_IP} 'cd /srv/app && git pull && systemctl restart app'"
timeout: 60
```

**3. From your phone:**

```
you:   deploy to staging
clide: Pulling latest code on staging... restarting app... done ✅

you:   check disk on db server
clide: [SSHs to ${HOST_DB_*}] → /dev/sda1 80% used — consider cleaning logs

you:   run a backup on prod
clide: [runs System/backup skill on prod] → backup complete, uploaded to S3
```

---

## Security tips

1. **Keep `authorized_users` tight** — list only your own Matrix ID or Telegram username. An empty list blocks everyone.

2. **Use `require_confirmation: true`** on production machines:
   ```yaml
   require_confirmation: true
   ```
   Clide will ask for a YES reply before running any command.

3. **Rotate your tokens periodically:**
   ```bash
   # Generate a new Gemini key at aistudio.google.com, then:
   clide secret set GEMINI_API_KEY
   pkill clide && clide bot &
   ```

4. **Verify file permissions after install:**
   ```bash
   ls -la ~/.clide/
   # All files should show -rw------- (600)
   # Directory should show drwx------ (700)
   ```

5. **Use Matrix over Telegram for higher security** — Matrix provides end-to-end encryption at the protocol level.

6. **Back up your vault before wiping a device:**
   ```bash
   clide vault backup
   # Save the Gist ID and passphrase
   ```

7. **Never paste secret values into the chat** — they would be sent to the AI. Use `${KEY_NAME}` placeholders in skills instead.

---

## Troubleshooting

### Bot starts but doesn't respond to messages

- Check `authorized_users` in `~/.clide/config.yaml` — your account must be listed.
- For Matrix: verify the room ID is correct (`!abc:matrix.org` format).
- For Telegram: make sure the bot token is valid and the bot was started with `/start`.

### "GEMINI_API_KEY not set" on startup

```bash
# Check where the key is stored
grep -r GEMINI_API_KEY ~/.clide/ ~/.config/clide/ 2>/dev/null

# Set it interactively
clide secret set GEMINI_API_KEY

# Or export it for this session
export GEMINI_API_KEY="AIzaSy..."
clide bot
```

### "Skill command failed: ${MY_SECRET} not found"

- Check that the key exists: `clide secret list`
- Key names are case-sensitive — `MY_SECRET` and `my_secret` are different.
- Restart Clide after editing `secrets.yaml` — secrets are loaded at startup.

### SSH connection refused in a skill

- Verify the host entry: `clide host list`
- Test the connection manually: `ssh -i /path/to/key user@ip`
- Check that the key file exists and has `chmod 600`.
- If using Tailscale, confirm the Tailscale daemon is running on both devices.

### "pass show failed: gpg: decryption failed"

- The GPG agent may have timed out. Run: `gpg-connect-agent reloadagent /bye`
- On Termux, you may need to restart the gpg-agent: `gpgconf --kill gpg-agent && gpg-agent --daemon`

### Vault backup/restore fails

- Confirm your GitHub token has `gist` scope.
- Confirm `age` is installed: `age --version` (install with `pkg install age` on Termux, or `brew install age` on macOS).
- Check your internet connection — vault operations require network access.

---

For more detail on any topic, see:
- [docs/SECRETS.md](SECRETS.md) — complete credential manager reference
- [docs/SECURITY.md](SECURITY.md) — threat model and hardening checklist
- [docs/WORKFLOWS.md](WORKFLOWS.md) — real-world skill examples
- [docs/INSTALL.md](INSTALL.md) — platform-specific installation
