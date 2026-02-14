# 📖 Installation Guide

Complete guide to installing clide on Termux (Android) and other platforms.

---

## 🎯 Quick Install (Recommended)

### Termux (Android)

```bash
# One-liner installation
curl -fsSL https://raw.githubusercontent.com/yourusername/clide/main/install.sh | bash
```

This script will:
1. ✅ Install Python and dependencies
2. ✅ Install signal-cli for Signal integration
3. ✅ Install Cline CLI
4. ✅ Set up clide
5. ✅ Create default configuration
6. ✅ Run first-time setup wizard

**Estimated time:** 5-10 minutes (depending on connection speed)

---

## 🔧 Manual Installation

### Prerequisites

Before installing clide, ensure you have:

#### For Termux (Android)
```bash
# Update packages
pkg update && pkg upgrade

# Install required packages
pkg install python git wget curl openssh

# Install pip
pip install --upgrade pip
```

#### For Linux/macOS
```bash
# Python 3.9 or higher
python3 --version

# Git
git --version

# If missing, install via your package manager:
# Ubuntu/Debian: sudo apt install python3 git
# macOS: brew install python3 git
```

---

## 📦 Step-by-Step Installation

### Step 1: Clone the Repository

```bash
# Clone clide
git clone https://github.com/yourusername/clide.git
cd clide
```

### Step 2: Install Python Dependencies

```bash
# Install required Python packages
pip install -r requirements.txt
```

**Required packages:**
- `signalbot` - Signal messenger integration
- `google-generativeai` - Gemini Flash API
- `pyyaml` - Configuration file handling
- `sqlite3` (built-in) - Database for memory
- `requests` - HTTP requests
- `cryptography` - Secure credential storage

### Step 3: Install Signal CLI

#### Termux
```bash
# Install Java (required for signal-cli)
pkg install openjdk-17

# Download signal-cli
wget https://github.com/AsamK/signal-cli/releases/latest/download/signal-cli-native.tar.gz

# Extract
tar xf signal-cli-native.tar.gz -C ~/.local/

# Add to PATH
echo 'export PATH="$HOME/.local/signal-cli/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

# Verify installation
signal-cli --version
```

#### Linux
```bash
# Download and install
wget https://github.com/AsamK/signal-cli/releases/latest/download/signal-cli-native.tar.gz
sudo tar xf signal-cli-native.tar.gz -C /opt
sudo ln -s /opt/signal-cli/bin/signal-cli /usr/local/bin/

# Verify
signal-cli --version
```

### Step 4: Install Cline CLI

```bash
# Install Cline globally
npm install -g @cline/cli

# Verify installation
cline --version
```

**Note:** If you don't have Node.js/npm:
```bash
# Termux
pkg install nodejs

# Ubuntu/Debian
sudo apt install nodejs npm

# macOS
brew install node
```

### Step 5: Configure Signal

#### Link Signal Account

**Option A: Link with QR Code (Recommended)**
```bash
# Start linking process
signal-cli link -n "clide-bot"

# This will display a QR code in terminal
# Scan it with your Signal app:
# Signal → Settings → Linked Devices → Add Device
```

**Option B: Register New Number**
```bash
# Register a new phone number (if you have a spare SIM)
signal-cli -a +1234567890 register

# Verify with code sent via SMS
signal-cli -a +1234567890 verify CODE
```

### Step 6: Set Up Configuration

```bash
# Copy example config
cp config.example.yaml config.yaml

# Edit configuration
nano config.yaml
```

**Edit the following:**

```yaml
# Signal Configuration
signal:
  phone_number: "+1234567890"  # Your Signal number
  
# Gemini API
gemini:
  api_key: "YOUR_GEMINI_API_KEY_HERE"
  model: "gemini-2.0-flash-exp"
  
# Cline Configuration  
cline:
  enabled: true
  safety_level: "medium"  # low, medium, high
  
# Memory
memory:
  database_path: "~/.clide/memory.db"
  max_history: 1000
  
# Safety
safety:
  dry_run_default: false
  confirm_destructive: true
  auto_backup: true
```

### Step 7: Get Gemini API Key

1. Visit [Google AI Studio](https://makersuite.google.com/app/apikey)
2. Sign in with Google account
3. Click "Create API Key"
4. Copy the key
5. Paste it in `config.yaml` under `gemini.api_key`

**Note:** Gemini Flash has a generous free tier!

### Step 8: First Run

```bash
# Start clide
python src/clide.py

# Or if you installed globally
clide
```

**First-time setup wizard will guide you through:**
1. ✅ Verifying Signal connection
2. ✅ Testing Gemini API
3. ✅ Checking Cline CLI
4. ✅ Creating memory database
5. ✅ Running test command

---

## 🚀 Running clide

### Start the Bot

```bash
# Start in foreground
python src/clide.py

# Or use the launcher script
./clide.sh
```

### Run in Background (Termux)

```bash
# Install termux-services (if not already)
pkg install termux-services

# Start clide as service
sv-enable clide

# Or use nohup
nohup python src/clide.py > clide.log 2>&1 &

# Keep Termux awake
termux-wake-lock
```

### Check Status

```bash
# Via Signal
# Send message: "status"

# Via logs
tail -f ~/.clide/logs/clide.log
```

---

## 🔧 Configuration Options

### Complete config.yaml Reference

```yaml
# ============================================
# clide Configuration File
# ============================================

# Signal Messenger
signal:
  phone_number: "+1234567890"
  receive_groups: false  # Respond in group chats?
  admin_only: true  # Only respond to your messages?
  
# AI Brain (Gemini Flash)
gemini:
  api_key: "YOUR_API_KEY"
  model: "gemini-2.0-flash-exp"
  temperature: 0.7
  max_tokens: 2048
  
# Command Executor (Cline)
cline:
  enabled: true
  max_retries: 3
  timeout: 300  # seconds
  safety_level: "medium"
  
# Memory System
memory:
  database_path: "~/.clide/memory.db"
  max_history: 1000
  auto_cleanup: true
  cleanup_days: 90
  
# Safety Settings
safety:
  dry_run_default: false
  confirm_destructive: true
  auto_backup: true
  blocked_patterns:
    - "rm -rf /"
    - "dd if=/dev/zero"
    - "mkfs"
  
# VPS Targets
vps:
  - name: "production"
    host: "prod.example.com"
    user: "admin"
    ssh_key: "~/.ssh/id_rsa"
    
  - name: "staging"
    host: "staging.example.com"
    user: "admin"
    ssh_key: "~/.ssh/id_rsa"
    
# Logging
logging:
  level: "INFO"  # DEBUG, INFO, WARNING, ERROR
  file: "~/.clide/logs/clide.log"
  max_size: "10MB"
  backup_count: 5
  
# Monitoring
monitoring:
  enabled: true
  check_interval: 300  # seconds
  alerts:
    disk_usage: 85  # percent
    memory_usage: 90
    failed_ssh: 5
```

---

## 🛠️ Troubleshooting

### Signal Issues

**Problem: "Could not connect to Signal"**
```bash
# Check signal-cli status
signal-cli -a +YOUR_NUMBER receive

# Re-link device
signal-cli link -n "clide-bot"
```

**Problem: "Account not registered"**
```bash
# Verify phone number
signal-cli -a +YOUR_NUMBER listIdentities
```

### Gemini API Issues

**Problem: "API key invalid"**
- Check key in config.yaml
- Verify at [Google AI Studio](https://makersuite.google.com/app/apikey)
- Ensure no extra spaces in the key

**Problem: "Rate limit exceeded"**
```yaml
# In config.yaml, reduce frequency
gemini:
  rate_limit: 10  # requests per minute
```

### Cline Issues

**Problem: "Cline command not found"**
```bash
# Reinstall globally
npm install -g @cline/cli

# Or add to PATH
export PATH="$HOME/.npm-global/bin:$PATH"
```

### Permission Issues

**Problem: "Permission denied" in Termux**
```bash
# Grant storage permission
termux-setup-storage

# Check permissions
ls -la ~/.clide/
```

### Python Issues

**Problem: "Module not found"**
```bash
# Reinstall dependencies
pip install -r requirements.txt --force-reinstall

# Check Python version (needs 3.9+)
python --version
```

---

## 🔄 Updating clide

### Update via Git

```bash
cd clide
git pull origin main
pip install -r requirements.txt --upgrade
```

### Update Dependencies Only

```bash
pip install -r requirements.txt --upgrade
```

### Backup Before Update

```bash
# Backup configuration
cp config.yaml config.yaml.backup

# Backup database
cp ~/.clide/memory.db ~/.clide/memory.db.backup
```

---

## 🗑️ Uninstalling

### Remove clide

```bash
# Remove files
rm -rf ~/clide
rm -rf ~/.clide

# Remove from PATH (if added)
# Edit ~/.bashrc and remove clide entries
```

### Keep Configuration & Data

```bash
# Remove only application files
rm -rf ~/clide

# Keep ~/.clide/ for later reinstall
```

---

## 📱 Platform-Specific Notes

### Android/Termux
- ✅ Fully supported
- ⚡ Optimized for mobile
- 🔋 Battery-efficient design
- 📶 Works offline (after initial setup)

### Linux
- ✅ Fully supported
- 🖥️ Can run on VPS
- 🐳 Docker support (coming soon)

### macOS
- ✅ Supported
- 🍎 Native M1/M2 support
- ⚠️ Signal CLI may need Rosetta on M1

### Windows
- ⚠️ Limited support (WSL recommended)
- 🐧 Use WSL2 for best experience

---

## 💡 Next Steps

After installation:

1. 📖 Read [WORKFLOWS.md](WORKFLOWS.md) for usage examples
2. 🔒 Review [SECURITY.md](SECURITY.md) for best practices
3. 🤝 Join our community and share feedback
4. ⭐ Star the repo if you find it useful!

---

## 🆘 Getting Help

- 📖 [Documentation](../README.md)
- 🐛 [Report Issues](https://github.com/yourusername/clide/issues)
- 💬 [Discussions](https://github.com/yourusername/clide/discussions)
- 📧 Email: support@yourproject.com

---

**Happy gliding!** ✈️
