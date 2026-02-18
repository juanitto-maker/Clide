# 📖 Installation Guide

Complete guide to installing clide on Termux (Android) and other platforms.

---

## 🎯 Quick Install (Recommended)

### Termux (Android) / Linux / macOS

```bash
# One-liner installation
curl -fsSL https://raw.githubusercontent.com/yourusername/clide/main/install.sh | bash
```

This script will:
1. ✅ Detect your platform and architecture
2. ✅ Download the correct pre-compiled binary
3. ✅ Install to `~/.local/bin/clide` (Termux) or `/usr/local/bin/clide` (Linux/macOS)
4. ✅ Make it executable
5. ✅ Verify installation

**No Rust compiler or build tools needed — just download and run!**

---

## 🔧 Manual Installation

### Prerequisites

Before installing clide, ensure you have:

#### For Termux (Android)
```bash
# Update packages
pkg update && pkg upgrade

# Install tools needed for downloading
pkg install wget curl
```

#### For Linux
```bash
# wget or curl for downloading the binary
wget --version || sudo apt install wget
```

#### For macOS
```bash
# curl is pre-installed on macOS
curl --version
```

> **Note:** Rust is **not** required to run the pre-compiled binary. You only need Rust if you want to build clide from source.

---

## 📦 Step-by-Step Installation

### Step 1: Download the Binary

Choose the binary for your platform:

**Termux (Android ARM64):**
```bash
wget https://github.com/yourusername/clide/releases/latest/download/clide-aarch64-android
chmod +x clide-aarch64-android
mkdir -p ~/.local/bin
mv clide-aarch64-android ~/.local/bin/clide
```

**Linux x86_64:**
```bash
wget https://github.com/yourusername/clide/releases/latest/download/clide-x86_64-linux
chmod +x clide-x86_64-linux
sudo mv clide-x86_64-linux /usr/local/bin/clide
```

**Linux ARM64:**
```bash
wget https://github.com/yourusername/clide/releases/latest/download/clide-aarch64-linux
chmod +x clide-aarch64-linux
sudo mv clide-aarch64-linux /usr/local/bin/clide
```

**macOS (Intel):**
```bash
curl -L https://github.com/yourusername/clide/releases/latest/download/clide-x86_64-darwin -o clide
chmod +x clide
sudo mv clide /usr/local/bin/
```

**macOS (Apple Silicon):**
```bash
curl -L https://github.com/yourusername/clide/releases/latest/download/clide-aarch64-darwin -o clide
chmod +x clide
sudo mv clide /usr/local/bin/
```

### Step 2: Add to PATH (Termux only)

```bash
echo 'export PATH=$HOME/.local/bin:$PATH' >> ~/.bashrc
source ~/.bashrc
```

### Step 3: Verify Installation

```bash
clide --version
```

---

## 🔨 Build from Source (Optional)

Only needed if you want to modify the code or no pre-compiled binary is available for your platform.

### Prerequisites

```bash
# Install Rust (all platforms)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Termux only — install C linker
pkg install clang
```

### Build

```bash
git clone https://github.com/yourusername/clide.git
cd clide
cargo build --release
```

**Install the compiled binary:**
```bash
# Linux/macOS
sudo cp target/release/clide /usr/local/bin/

# Termux
cp target/release/clide ~/.local/bin/
```

---

## 📡 Step 4: Install Signal CLI

Clide relies on [signal-cli](https://github.com/AsamK/signal-cli) to send and receive Signal messages.

**Linux/macOS:**
```bash
SIGNAL_VERSION="0.13.1"
wget https://github.com/AsamK/signal-cli/releases/download/v${SIGNAL_VERSION}/signal-cli-${SIGNAL_VERSION}.tar.gz
tar xf signal-cli-${SIGNAL_VERSION}.tar.gz
sudo mv signal-cli-${SIGNAL_VERSION} /opt/signal-cli
sudo ln -sf /opt/signal-cli/bin/signal-cli /usr/local/bin/signal-cli
```

**Termux:**
```bash
pkg install openjdk-17 wget

SIGNAL_VERSION="0.13.1"
wget https://github.com/AsamK/signal-cli/releases/download/v${SIGNAL_VERSION}/signal-cli-${SIGNAL_VERSION}.tar.gz
tar xf signal-cli-${SIGNAL_VERSION}.tar.gz
mv signal-cli-${SIGNAL_VERSION} ~/.local/
echo "export PATH=\"\$HOME/.local/signal-cli-${SIGNAL_VERSION}/bin:\$PATH\"" >> ~/.bashrc
source ~/.bashrc
```

**Verify:**
```bash
signal-cli --version
```

---

## 📱 Step 5: Configure Signal

### Option A: Link as Secondary Device (Recommended)

```bash
signal-cli link -n "clide-bot"
```

A QR code will appear in your terminal. Scan it with the Signal app:
1. Open Signal on your phone
2. Settings → Linked Devices
3. Tap "+" / "Link New Device"
4. Scan the QR code

**Advantages:**
- ✅ No SMS needed
- ✅ More secure than SMS registration
- ✅ Can be revoked remotely

### Option B: Register New Number

```bash
signal-cli -a +1234567890 register
signal-cli -a +1234567890 verify <code>
```

---

## ⚙️ Step 6: Configure Clide

```bash
# Create config directory
mkdir -p ~/.clide/logs

# Copy example config
cp config.example.yaml ~/.clide/config.yaml

# Edit config
nano ~/.clide/config.yaml
```

**Minimal config.yaml:**
```yaml
# Get API key free at: https://makersuite.google.com/app/apikey
gemini_api_key: "YOUR_GEMINI_API_KEY_HERE"

# Your Signal number (format: +1234567890)
signal_number: "+1234567890"

# Basic settings
require_confirmation: false
logging:
  level: "info"
```

**Complete config reference:**
```yaml
# Gemini API key (or set GEMINI_API_KEY env var)
gemini_api_key: ""

# Gemini model to use
gemini_model: "gemini-2.5-flash"

# Your Signal phone number
signal_number: "+1234567890"

# Ask for YES/NO before replying
require_confirmation: false

# Timeout (seconds) waiting for YES/NO
confirmation_timeout: 60

# Numbers allowed to send commands (empty = anyone)
authorized_numbers: []
  # - "+1234567890"

# Shell command patterns to block
blocked_commands:
  - "rm -rf /"
  - "mkfs"
  - "dd if="

# Logging
logging:
  level: "info"   # trace | debug | info | warn | error
```

---

## 🔑 Step 7: Get Gemini API Key

1. Visit: https://makersuite.google.com/app/apikey
2. Sign in with Google account
3. Click "Create API Key"
4. Copy the key
5. Add to `~/.clide/config.yaml`

**Free tier includes:**
- 60 requests per minute
- 1,500 requests per day

---

## ✅ Step 8: Verify Everything

```bash
# Check binary version
clide --version

# Test Gemini API connection
clide test-gemini "Hello!"

# Check configuration
clide config show

# Test Signal (sends yourself a test message)
clide test-signal
```

---

## 🚀 Running Clide

### Start the Bot

```bash
clide start
```

**What happens:**
1. Loads configuration from `~/.clide/config.yaml`
2. Connects to Signal via signal-cli
3. Initialises Gemini AI
4. Starts listening for messages

**Send a test message via Signal:**
```
status
```

You should receive a response with system status.

### Run in Background (Termux)

```bash
# Install termux-services
pkg install termux-services

# Enable clide as a service
sv-enable clide

# Or use nohup
nohup clide start > ~/.clide/logs/clide.log 2>&1 &

# Keep Termux awake
termux-wake-lock
```

---

## 🔄 Updating Clide

### Automatic Update

```bash
clide update
```

### Manual Update

```bash
# Re-run the install script
curl -fsSL https://raw.githubusercontent.com/yourusername/clide/main/install.sh | bash
```

### Build from Source Update

```bash
cd clide
git pull origin main
cargo build --release
sudo cp target/release/clide /usr/local/bin/
```

---

## 🗑️ Uninstalling

### Remove Binary

```bash
# Linux/macOS
sudo rm /usr/local/bin/clide

# Termux
rm ~/.local/bin/clide
```

### Remove Configuration & Data

```bash
rm -rf ~/.clide
```

### Remove Signal CLI (Optional)

```bash
# Linux/macOS
sudo rm -rf /opt/signal-cli
sudo rm /usr/local/bin/signal-cli

# Termux
rm -rf ~/.local/signal-cli-*
```

---

## 🛠️ Troubleshooting

### "clide: command not found"

```bash
echo 'export PATH=$HOME/.local/bin:$PATH' >> ~/.bashrc
source ~/.bashrc
```

### "signal-cli: command not found"

Install signal-cli and ensure it's in your PATH (see Step 4 above).

### "Failed to connect to Gemini API"

1. Check `gemini_api_key` in `~/.clide/config.yaml`
2. Test internet connection: `ping google.com`
3. Check rate limit — wait 60 seconds and retry

### "Permission denied" running clide

```bash
chmod +x /usr/local/bin/clide
```

### Termux: "cannot execute binary file"

Wrong architecture downloaded. Re-run `install.sh` — it auto-detects the correct binary.

### Signal messages not received

```bash
signal-cli -a +1234567890 receive
```

If no messages appear, re-link or re-register your Signal account.

---

## 📱 Platform-Specific Notes

### Android / Termux
- ✅ Fully supported — primary target platform
- ⚡ Pre-compiled binary, no build needed
- 🔋 Battery-efficient, low resource footprint
- Install Termux from **F-Droid only** (Play Store version is outdated)

### Linux
- ✅ Fully supported (x86_64 and ARM64)
- 🖥️ Can run on VPS

### macOS
- ✅ Supported (Intel and Apple Silicon)

### Windows
- ⚠️ Not natively supported — use WSL2 (Windows Subsystem for Linux)

---

## 💡 Next Steps

After installation:

1. 📖 Read [WORKFLOWS.md](WORKFLOWS.md) for usage examples
2. 🔒 Review [SECURITY.md](SECURITY.md) for best practices
3. 🤝 Check [CONTRIBUTING.md](CONTRIBUTING.md) to contribute

---

## 🆘 Getting Help

- 📖 [Documentation](../README.md)
- 🐛 [Report Issues](https://github.com/yourusername/clide/issues)
- 💬 [Discussions](https://github.com/yourusername/clide/discussions)
- 📧 Email: support@yourproject.com

---

**Happy gliding!** ✈️
