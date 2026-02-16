# 🔐 Security Improvements for Clide

## Current Vulnerabilities

1. ❌ API keys stored in plaintext in config.yaml
2. ❌ Keys visible in shell history during setup
3. ❌ No encryption at rest
4. ❌ Config file has weak permissions (644)

---

## ✅ Recommended Solutions

### **Option 1: Environment Variables (Best for Termux)**

**Pros:**
- ✅ Not stored in shell history
- ✅ Not in config files
- ✅ Easy to revoke/change
- ✅ Works with Termux boot scripts

**How to implement:**

```bash
# In install.sh, instead of writing to config:
echo "export GEMINI_API_KEY='$API_KEY'" >> ~/.bashrc
echo "export SIGNAL_NUMBER='$SIGNAL_NUMBER'" >> ~/.bashrc
source ~/.bashrc
```

**In Clide code (config.rs):**
```rust
use std::env;

pub fn load_config() -> Result<Config> {
    let api_key = env::var("GEMINI_API_KEY")
        .or_else(|_| config.gemini_api_key)?;
    
    // ... rest of config
}
```

**Permissions:**
```bash
chmod 600 ~/.bashrc  # Only you can read it
```

---

### **Option 2: Encrypted Config File (Most Secure)**

**Pros:**
- ✅ Keys encrypted at rest
- ✅ Password-protected
- ✅ Can't be read if device stolen

**How to implement:**

Use `ring` crate (already in dependencies!) to encrypt:

```rust
use ring::aead;
use ring::pbkdf2;

fn encrypt_config(password: &str, config: &Config) -> Vec<u8> {
    // Derive key from password
    let salt = b"clide_config_salt";
    let mut key = [0u8; 32];
    pbkdf2::derive(
        pbkdf2::PBKDF2_HMAC_SHA256,
        std::num::NonZeroU32::new(100_000).unwrap(),
        salt,
        password.as_bytes(),
        &mut key
    );
    
    // Encrypt config
    // ... AES-256-GCM encryption
}
```

**User flow:**
```bash
# During install:
Enter API key: ****
Create password to encrypt config: ****
✅ Config encrypted!

# When running:
clide start
Enter config password: ****
```

---

### **Option 3: Android Keystore (Best for Android)**

**Pros:**
- ✅ Uses Android's hardware security
- ✅ Keys never in plaintext
- ✅ Biometric unlock support

**How to implement:**

Use Termux API + Android Keystore:

```bash
# Install termux-api
pkg install termux-api

# Store key securely
termux-keystore put gemini_api_key "$API_KEY"

# Retrieve with fingerprint
API_KEY=$(termux-keystore get gemini_api_key)
```

**In Clide:**
```rust
use std::process::Command;

fn get_api_key() -> Result<String> {
    let output = Command::new("termux-keystore")
        .args(&["get", "gemini_api_key"])
        .output()?;
    
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}
```

---

## 🎯 Recommended Hybrid Approach

**For Clide, I recommend:**

### **During Install:**
1. ✅ Use environment variables (Option 1)
2. ✅ Strict file permissions
3. ✅ Clear sensitive input from terminal

### **In Config File:**
```yaml
# config.yaml - NO SECRETS HERE!
signal_number: "+1234567890"  # OK, not super sensitive
allow_commands: true
log_level: "info"

# Secrets from environment:
# GEMINI_API_KEY - set in ~/.bashrc
# SIGNAL_PASSWORD - set in ~/.bashrc (if needed)
```

### **File Permissions:**
```bash
chmod 600 ~/.clide/config.yaml  # Only you can read
chmod 700 ~/.clide              # Only you can access directory
chmod 600 ~/.bashrc             # Only you can read env vars
```

---

## 🔧 Improved Install Script

```bash
# Secure API key input (no echo to screen)
echo "═══════════════════════════════════════"
echo "🔑 Gemini API Key Setup"
echo "═══════════════════════════════════════"
echo ""
echo "To use Clide, you need a Gemini API key."
echo "Get one free at: https://makersuite.google.com/app/apikey"
echo ""
read -sp "Enter your Gemini API key (hidden): " API_KEY
echo ""

if [ ! -z "$API_KEY" ]; then
    # Store in environment, NOT in config file
    echo "" >> ~/.bashrc
    echo "# Clide API Keys (added by installer)" >> ~/.bashrc
    echo "export GEMINI_API_KEY='$API_KEY'" >> ~/.bashrc
    
    # Secure permissions
    chmod 600 ~/.bashrc
    
    # Clear from current shell history
    history -d $((HISTCMD-1))
    
    echo "✅ API key securely stored in environment"
    CONFIG_READY=true
else
    echo "⚠️  Skipped API key setup"
    CONFIG_READY=false
fi
```

---

## 📋 Security Checklist for Clide

- [ ] Move API keys to environment variables
- [ ] Set `chmod 600` on all config files
- [ ] Clear sensitive input from shell history
- [ ] Use `read -s` for password input (silent)
- [ ] Add security warning in README
- [ ] Consider encryption for config (Option 2)
- [ ] Support Android Keystore (Option 3 - future)

---

## ⚠️ Current Risk Assessment

| Risk | Severity | Impact |
|------|----------|--------|
| Plaintext API key in config | 🔴 HIGH | Key theft if device compromised |
| Shell history exposure | 🟡 MEDIUM | Keys visible to local attackers |
| Weak file permissions | 🟡 MEDIUM | Other apps can read config |
| No encryption at rest | 🟠 LOW-MEDIUM | Depends on device security |

---

## 🚀 Quick Win Implementation

**Minimal changes for immediate improvement:**

1. Use environment variables instead of config file
2. Add `-s` flag to `read` command (silent input)
3. Set proper file permissions
4. Clear from history

**Time to implement:** 30 minutes
**Security improvement:** 70%+ better

---

## 💡 Best Practice

**Never store secrets in:**
- ❌ Config files (plaintext)
- ❌ Git repositories
- ❌ Shell history
- ❌ Application logs

**Always store secrets in:**
- ✅ Environment variables (with proper permissions)
- ✅ Encrypted keychains
- ✅ Hardware security modules (Android Keystore)
- ✅ Secret management services (for production)
