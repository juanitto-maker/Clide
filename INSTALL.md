# Clide Installation Guide

Complete installation instructions for all supported platforms.

---

## Quick Install (Recommended)

### Android (Termux)

```bash
curl -fsSL https://raw.githubusercontent.com/juanitto-maker/Clide/main/install.sh | bash
```

The installer will:
1. Install the pre-built binary (or build from source if no release is available)
2. Create `~/.clide/config.yaml`
3. Interactively ask for your Gemini API key and Matrix credentials (with skip option)

---

## Platform-Specific Installation

### Linux

#### Option 1: Download Binary (Fastest)

```bash
# For x86_64 (Intel/AMD)
wget https://github.com/juanitto-maker/Clide/releases/latest/download/clide-x86_64
chmod +x clide-x86_64
sudo mv clide-x86_64 /usr/local/bin/clide

# For ARM64 (Raspberry Pi, etc.)
wget https://github.com/juanitto-maker/Clide/releases/latest/download/clide-aarch64
chmod +x clide-aarch64
sudo mv clide-aarch64 /usr/local/bin/clide
```

#### Option 2: Build from Source

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
git clone https://github.com/juanitto-maker/Clide.git
cd Clide
cargo build --release
sudo cp target/release/clide /usr/local/bin/
```

---

### macOS

```bash
# For Intel Macs
curl -L https://github.com/juanitto-maker/Clide/releases/latest/download/clide-x86_64-darwin -o clide
chmod +x clide
sudo mv clide /usr/local/bin/

# For Apple Silicon (M1/M2/M3)
curl -L https://github.com/juanitto-maker/Clide/releases/latest/download/clide-aarch64-darwin -o clide
chmod +x clide
sudo mv clide /usr/local/bin/
```

---

### Android (Termux)

**Termux is the PRIMARY target platform.** The installer handles everything automatically.

1. Install Termux from [F-Droid](https://f-droid.org/packages/com.termux/) (not Google Play)
2. Run:
```bash
pkg update && pkg install curl
curl -fsSL https://raw.githubusercontent.com/juanitto-maker/Clide/main/install.sh | bash
```
3. Follow the prompts (all steps are skippable)

---

## Post-Installation Setup

### 1. Get a Gemini API Key

1. Visit: https://aistudio.google.com/app/apikey
2. Sign in with Google
3. Click "Create API Key"
4. Copy the key

### 2. Set up Matrix/Element

You need **two Matrix accounts**: one for you (the operator) and one for the bot.

> **Why two accounts?**
> The bot listens to the room and responds to your messages. If the bot and operator share
> the same account, the bot would ignore every message you send (self-response guard).

#### Create the bot account

Go to https://app.element.io → **Create Account** → choose `matrix.org`.

> **IMPORTANT — use a password, not Google/GitHub login.**
> SSO accounts have no password, which means you cannot get their token via the API below.
> Pick a username like `@mybot:matrix.org` and set a real password during registration.

#### Get the bot's access token

```bash
curl -XPOST https://matrix.org/_matrix/client/v3/login \
  -H "Content-Type: application/json" \
  -d '{"type":"m.login.password","identifier":{"type":"m.id.user","user":"mybot"},"password":"yourpassword"}'
# Copy "access_token" from the response — it looks like "syt_..."
```

Your personal account (operator) stays in Element as normal. Only the bot account's token
goes into the Clide config. The bot never needs Element open.

#### Get your room ID
1. Open the room in Element (logged in as your personal account)
2. Settings → Advanced → Internal room ID
3. Format: `!abc123:matrix.org`

**Invite the bot account to the room** and accept the invite from any Element session logged
in as the bot.

### 3. Configure Clide

```bash
# Create config directory
mkdir -p ~/.clide

# Copy example config
cp /path/to/Clide/config.example.yaml ~/.clide/config.yaml
chmod 600 ~/.clide/config.yaml

# Edit config
nano ~/.clide/config.yaml
```

**Minimal config:**
```yaml
gemini_api_key: "YOUR_GEMINI_API_KEY"

matrix_homeserver: "https://matrix.org"
matrix_user: "@yourbot:matrix.org"
matrix_access_token: "YOUR_ACCESS_TOKEN"
matrix_room_id: "!roomid:matrix.org"
```

You can also use environment variables instead of putting secrets in the yaml:
```bash
export GEMINI_API_KEY="your-key"
export MATRIX_ACCESS_TOKEN="your-token"
```

### 4. Start the Bot

```bash
clide bot
```

Send a message in the Matrix room — you should get a Gemini-powered reply.

---

## Updating Clide

```bash
# Re-run the installer (Termux)
curl -fsSL https://raw.githubusercontent.com/juanitto-maker/Clide/main/install.sh | bash

# Or build from source
cd Clide && git pull && cargo build --release && sudo cp target/release/clide /usr/local/bin/
```

---

## Uninstallation

```bash
# Remove binary
sudo rm /usr/local/bin/clide          # Linux/macOS
rm "$PREFIX/bin/clide"                # Termux

# Remove configuration and data
rm -rf ~/.clide ~/.config/clide
```

---

## Troubleshooting

### "clide: command not found"
```bash
echo 'export PATH=$HOME/.local/bin:$PATH' >> ~/.bashrc
source ~/.bashrc
```

### "Cannot read config"
```bash
cp config.example.yaml ~/.clide/config.yaml
# Then fill in your credentials
```

### "Failed to sync with Matrix server"
- Check `matrix_homeserver` URL is correct (no trailing slash)
- Verify your access token is valid (try re-logging in to get a fresh one)
- Test connectivity: `curl https://matrix.org/_matrix/client/versions`

### "I created the bot account with Google/GitHub and now I can't get a token"

SSO accounts have no password, so the curl login approach won't work. Options:

- **Easiest:** Create a fresh bot account on https://app.element.io using username + password
  (not Google/GitHub). Use that account's token in the config.
- **Alternative:** Open https://app.element.io in a private/incognito browser window, log in
  as the bot via SSO, go to Settings → Help & About → Access Token, copy it, then **close
  the tab without clicking Log Out**. The token stays valid until explicitly logged out.

### "Failed to connect to Gemini API"
- Check `GEMINI_API_KEY` is set and valid
- Test: `curl "https://generativelanguage.googleapis.com/v1beta/models?key=YOUR_KEY"`

---

**Happy gliding!**
