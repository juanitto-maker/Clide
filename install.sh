#!/usr/bin/env bash
# ============================================
# Clide Installer for Termux & Linux
# One-liner: curl -fsSL https://raw.githubusercontent.com/juanitto-maker/Clide/main/install.sh | bash
# ============================================

set -e

# ─── Security: prevent secrets from leaking into shell history ───────────────
unset HISTFILE 2>/dev/null || true
export HISTCONTROL=ignorespace
export HISTSIZE=0

REPO="juanitto-maker/Clide"
INSTALL_DIR="$HOME/Clide_Source"
RESTORE_MODE=false

# ─── Parse flags ──────────────────────────────────────────────────────────────
for arg in "$@"; do
    case "$arg" in
        --restore) RESTORE_MODE=true ;;
    esac
done

# ─── Platform detection ──────────────────────────────────────────────────────
# Priority: 1) Termux (Android) — PRIMARY
#           2) Linux x86_64 (Ubuntu VPS)
#           3) Linux ARM64 (Raspberry Pi etc.)
#           4) Other → unsupported

if [[ "$PREFIX" =~ "com.termux" ]]; then
    PLATFORM="termux"
    BIN_DIR="$PREFIX/bin"
    PKG_MANAGER="pkg"
    echo "✅ Termux detected"
elif [[ "$(uname -s)" == "Linux" ]]; then
    PLATFORM="linux"
    BIN_DIR="/usr/local/bin"
    PKG_MANAGER="apt-get"
    echo "✅ Linux detected ($(uname -m))"

    # ─── Bootstrap: ensure essential tools exist on a fresh VPS ───────────
    echo "   Installing bootstrap dependencies..."
    sudo apt-get update -qq 2>/dev/null
    sudo apt-get install -y -qq curl wget git build-essential 2>/dev/null
    echo "✅ Bootstrap complete"
else
    echo "❌ Unsupported platform: $(uname -s)"
    echo "   Supported: Termux (Android), Linux (x86_64, aarch64)"
    exit 1
fi

# ─── Helpers ──────────────────────────────────────────────────────────────────

# Read user input from /dev/tty so it works when piped through curl | bash
ask() {
    local prompt="$1"
    local varname="$2"
    local secret="${3:-}"
    printf "%s" "$prompt" >/dev/tty
    if [ "$secret" = "secret" ]; then
        IFS= read -rs answer </dev/tty
        echo "" >/dev/tty
    else
        IFS= read -r answer </dev/tty
    fi
    eval "$varname=\"\$answer\""
}

step() { echo ""; echo "── $1 ──────────────────────────────────────────"; }

# Replace a YAML key's value in a file without exposing the value in `ps`.
# Usage: safe_yaml_set <file> <key> <value>
# Reads file → replaces line → writes back. The secret never appears in a
# command-line argument (unlike sed -i which is visible via ps aux).
safe_yaml_set() {
    local file="$1" key="$2" value="$3"
    local tmpf
    tmpf=$(mktemp)
    if grep -q "^${key}:" "$file" 2>/dev/null; then
        while IFS= read -r line; do
            if echo "$line" | grep -q "^${key}:"; then
                printf '%s: "%s"\n' "$key" "$value"
            else
                printf '%s\n' "$line"
            fi
        done < "$file" > "$tmpf"
        mv "$tmpf" "$file"
    else
        printf '%s: "%s"\n' "$key" "$value" >> "$file"
    fi
}

# Replace a KEY=VALUE line in an env file without exposing the value in `ps`.
# Usage: safe_env_set <file> <key> <value>
safe_env_set() {
    local file="$1" key="$2" value="$3"
    local tmpf
    tmpf=$(mktemp)
    if grep -q "^${key}=" "$file" 2>/dev/null; then
        while IFS= read -r line; do
            if echo "$line" | grep -q "^${key}="; then
                printf '%s=%s\n' "$key" "$value"
            else
                printf '%s\n' "$line"
            fi
        done < "$file" > "$tmpf"
        mv "$tmpf" "$file"
    else
        printf '%s=%s\n' "$key" "$value" >> "$file"
    fi
}

# Show a simple spinner while a background PID is running
spinner() {
    local pid=$1 msg="${2:-please wait}"
    local i=0 frames='|/-\'
    while kill -0 "$pid" 2>/dev/null; do
        printf "\r   [%s] %s..." "${frames:$((i%4)):1}" "$msg" >/dev/tty
        sleep 0.3
        i=$((i+1))
    done
    printf "\r%-70s\r" "" >/dev/tty
}

# ─── 1. System packages ───────────────────────────────────────────────────────

step "Updating packages"
if [ "$PLATFORM" = "termux" ]; then
    pkg update -y 2>&1 | grep -E "^(Get:|Fetched|Reading)" | tail -5 || true
else
    sudo apt-get update -y 2>&1 | grep -E "^(Get:|Fetched|Reading)" | tail -5 || true
fi
echo "✅ Done"

step "Installing dependencies"
if [ "$PLATFORM" = "termux" ]; then
    pkg install -y git wget curl age 2>&1 | grep -E "^(Unpacking|Setting up)" | sed 's/^/   /' || true
else
    sudo apt-get install -y git wget curl age 2>&1 | grep -E "^(Unpacking|Setting up)" | sed 's/^/   /' || true
fi
echo "✅ Done"

# ─── Restore mode: decrypt vault and exit ─────────────────────────────────────
# Usage:  bash install.sh --restore
# The user needs:  (1) their GitHub token  (2) their vault Gist ID  (3) passphrase
if [ "$RESTORE_MODE" = true ]; then
    step "Vault Restore (--restore mode)"

    echo "This will decrypt your Clide vault from GitHub Gist and restore" >/dev/tty
    echo "~/.clide/secrets.yaml and ~/.clide/hosts.yaml." >/dev/tty
    echo "" >/dev/tty

    ask "GitHub personal access token (gist scope): " GITHUB_TOKEN secret
    ask "Gist ID (from your last 'vault backup' output): " GIST_ID
    if [ -z "$GITHUB_TOKEN" ] || [ -z "$GIST_ID" ]; then
        echo "❌ Token and Gist ID are required."
        exit 1
    fi

    VAULT_TMP="$HOME/.clide/vault_tmp"
    mkdir -p "$HOME/.clide" "$VAULT_TMP"
    chmod 700 "$HOME/.clide" "$VAULT_TMP"

    echo "Fetching vault from Gist $GIST_ID ..." >/dev/tty
    ENCODED=$(curl -s \
        -H "Authorization: token $GITHUB_TOKEN" \
        "https://api.github.com/gists/$GIST_ID" \
        | grep '"content"' | head -1 \
        | sed 's/.*"content": *"\(.*\)".*/\1/' \
        | sed 's/\\n/\n/g')

    if [ -z "$ENCODED" ]; then
        echo "❌ Empty response. Check your token and Gist ID."
        exit 1
    fi

    ENCRYPTED="$VAULT_TMP/vault.tar.gz.age"
    echo "$ENCODED" | base64 -d > "$ENCRYPTED"

    ARCHIVE="$VAULT_TMP/vault.tar.gz"
    echo "" >/dev/tty
    echo ">>> Enter your vault passphrase:" >/dev/tty
    age -d -o "$ARCHIVE" "$ENCRYPTED"
    rm -f "$ENCRYPTED"

    tar -xzf "$ARCHIVE" -C "$HOME/.clide"
    rm -f "$ARCHIVE"

    [ -f "$HOME/.clide/secrets.yaml" ] && chmod 600 "$HOME/.clide/secrets.yaml"
    [ -f "$HOME/.clide/hosts.yaml"   ] && chmod 600 "$HOME/.clide/hosts.yaml"

    # Restore SSH keys if they were included in the vault backup
    SSH_RESTORED=0
    if [ -d "$HOME/.clide/ssh_keys_vault" ]; then
        mkdir -p "$HOME/.ssh"
        chmod 700 "$HOME/.ssh"
        for kf in "$HOME/.clide/ssh_keys_vault"/*; do
            [ -f "$kf" ] || continue
            BASENAME=$(basename "$kf")
            cp "$kf" "$HOME/.ssh/$BASENAME"
            if echo "$BASENAME" | grep -q '\.pub$'; then
                chmod 644 "$HOME/.ssh/$BASENAME"
            else
                chmod 600 "$HOME/.ssh/$BASENAME"
            fi
            SSH_RESTORED=$((SSH_RESTORED + 1))
        done
        rm -rf "$HOME/.clide/ssh_keys_vault"
    fi

    # Save the Gist ID for future vault operations
    echo "$GIST_ID" > "$HOME/.clide/vault_gist_id"
    chmod 600 "$HOME/.clide/vault_gist_id"

    echo ""
    echo "✅ Vault restored!"
    echo "   secrets.yaml : $([ -f "$HOME/.clide/secrets.yaml" ] && echo "OK" || echo "NOT FOUND")"
    echo "   hosts.yaml   : $([ -f "$HOME/.clide/hosts.yaml"   ] && echo "OK" || echo "NOT FOUND")"
    if [ "$SSH_RESTORED" -gt 0 ]; then
        echo "   SSH keys     : $SSH_RESTORED file(s) restored to ~/.ssh/"
    fi
    echo ""
    echo "Now run the installer normally to install the binary:"
    echo "  bash install.sh   (without --restore)"
    echo ""
    echo "Or start the bot directly if already installed:"
    echo "  clide bot"
    exit 0
fi

# ─── 2. Install Clide binary ──────────────────────────────────────────────────

step "Installing Clide binary"
mkdir -p "$BIN_DIR" 2>/dev/null || sudo mkdir -p "$BIN_DIR"

CLIDE_INSTALLED=false

# Determine which binary artifact to download based on platform + arch
if [ "$PLATFORM" = "termux" ]; then
    BIN_NAME="clide-aarch64-android"
    # Fallback: also try the legacy name for older releases
    BIN_NAME_FALLBACK="clide-aarch64"
elif [ "$(uname -m)" = "x86_64" ]; then
    BIN_NAME="clide-x86_64"
    BIN_NAME_FALLBACK=""
elif [ "$(uname -m)" = "aarch64" ]; then
    BIN_NAME="clide-aarch64"
    BIN_NAME_FALLBACK=""
else
    echo "   ⚠️  Unknown architecture: $(uname -m) — will try building from source."
    BIN_NAME=""
    BIN_NAME_FALLBACK=""
fi

# 2a. Try pre-built binary from GitHub Releases (fast path, skips Rust build)
echo "   Checking for pre-built binary..."
LATEST_TAG=$(wget -qO- "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null \
    | grep '"tag_name"' | sed 's/.*"v\([^"]*\)".*/\1/' | head -1 || true)

if [ -n "$LATEST_TAG" ] && [ -n "$BIN_NAME" ]; then
    BIN_URL="https://github.com/${REPO}/releases/download/v${LATEST_TAG}/${BIN_NAME}"
    echo "   Trying pre-built binary ${BIN_NAME} for v${LATEST_TAG}..."
    TMP_BIN=$(mktemp)
    if wget -q "$BIN_URL" -O "$TMP_BIN" 2>/dev/null && [ -s "$TMP_BIN" ]; then
        if [ "$PLATFORM" = "termux" ]; then
            mv "$TMP_BIN" "$BIN_DIR/clide"
        else
            sudo mv "$TMP_BIN" "$BIN_DIR/clide"
        fi
        chmod +x "$BIN_DIR/clide" 2>/dev/null || sudo chmod +x "$BIN_DIR/clide"
        echo "✅ Pre-built binary installed (v${LATEST_TAG})"
        CLIDE_INSTALLED=true
    elif [ -n "$BIN_NAME_FALLBACK" ]; then
        # Try fallback name (e.g. clide-aarch64 for Termux on older releases)
        rm -f "$TMP_BIN"
        BIN_URL="https://github.com/${REPO}/releases/download/v${LATEST_TAG}/${BIN_NAME_FALLBACK}"
        echo "   Trying fallback binary ${BIN_NAME_FALLBACK}..."
        TMP_BIN=$(mktemp)
        if wget -q "$BIN_URL" -O "$TMP_BIN" 2>/dev/null && [ -s "$TMP_BIN" ]; then
            if [ "$PLATFORM" = "termux" ]; then
                mv "$TMP_BIN" "$BIN_DIR/clide"
            else
                sudo mv "$TMP_BIN" "$BIN_DIR/clide"
            fi
            chmod +x "$BIN_DIR/clide" 2>/dev/null || sudo chmod +x "$BIN_DIR/clide"
            echo "✅ Pre-built binary installed (v${LATEST_TAG}, fallback)"
            CLIDE_INSTALLED=true
        else
            echo "   No binary in release v${LATEST_TAG} — will build from source."
            rm -f "$TMP_BIN"
        fi
    else
        echo "   No ${BIN_NAME} binary in release v${LATEST_TAG} — will build from source."
        rm -f "$TMP_BIN"
    fi
else
    echo "   No release found — will build from source."
fi

# 2b. Build from source (fallback)
if [ "$CLIDE_INSTALLED" = false ]; then
    step "Building Clide from source"
    echo "   ⚠️  No pre-built binary — compiling from source."

    if [ "$PLATFORM" = "termux" ]; then
        echo "   This takes 10-15 min on most devices. Keep Termux open."
        echo ""

        pkg install -y rust binutils pkg-config openssl \
            >"$TMPDIR/pkg_rust.log" 2>&1 &
        spinner $! "Installing Rust toolchain"
        wait $! || { echo "❌ Rust install failed"; cat "$TMPDIR/pkg_rust.log"; exit 1; }
        grep -E "^(Unpacking|Setting up)" "$TMPDIR/pkg_rust.log" | \
            sed 's/^/   /' | tail -3 || true
    else
        echo "   Installing Rust toolchain..."
        echo ""
        BUILD_TMPDIR="${TMPDIR:-/tmp}"

        sudo apt-get install -y build-essential pkg-config libssl-dev \
            >"$BUILD_TMPDIR/pkg_rust.log" 2>&1 &
        spinner $! "Installing build dependencies"
        wait $! || { echo "❌ Build deps install failed"; cat "$BUILD_TMPDIR/pkg_rust.log"; exit 1; }

        if ! command -v cargo >/dev/null 2>&1; then
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y 2>&1 | tail -3
            . "$HOME/.cargo/env"
        fi
    fi

    if ! command -v cargo >/dev/null 2>&1; then
        echo "❌ Rust installation failed"
        exit 1
    fi
    echo "✅ $(rustc --version)"

    if [ -d "$INSTALL_DIR" ]; then
        echo "   Updating existing source..."
        git -C "$INSTALL_DIR" pull --ff-only origin main 2>/dev/null || true
    else
        echo "   Cloning repository..."
        git clone "https://github.com/${REPO}.git" "$INSTALL_DIR" 2>&1 | \
            grep -E "^(Cloning|Receiving|Resolving)" || true
    fi
    cd "$INSTALL_DIR"

    if [ "$PLATFORM" = "termux" ]; then
        # Termux build environment
        export CC="$PREFIX/bin/clang"
        export AR="$PREFIX/bin/llvm-ar"
        export OPENSSL_INCLUDE_DIR="$PREFIX/include"
        export OPENSSL_LIB_DIR="$PREFIX/lib"
    fi

    echo "   Compiling Clide... (started $(date '+%H:%M:%S'), this will take a while)"
    BUILD_LOG="${TMPDIR:-/tmp}/clide_build.log"

    cargo build --release 2>&1 | tee "$BUILD_LOG" | \
        grep -E "^(   Compiling|   Finished|  Downloaded|  Downloading|error\[)" || true

    if [ ! -f "target/release/clide" ]; then
        echo "❌ Build failed. Log: $BUILD_LOG"
        exit 1
    fi

    if [ "$PLATFORM" = "termux" ]; then
        cp target/release/clide "$BIN_DIR/clide"
    else
        sudo cp target/release/clide "$BIN_DIR/clide"
    fi
    chmod +x "$BIN_DIR/clide" 2>/dev/null || sudo chmod +x "$BIN_DIR/clide"
    echo "✅ Built and installed. Finished: $(date '+%H:%M:%S')"
    CLIDE_INSTALLED=true
fi

# ─── 3. Configuration ─────────────────────────────────────────────────────────

step "Configuration"

mkdir -p ~/.config/clide
chmod 700 ~/.config/clide

mkdir -p ~/.clide
chmod 700 ~/.clide

# Write minimal config if not already present
if [ ! -f ~/.clide/config.yaml ]; then
    if [ -f "$INSTALL_DIR/config.example.yaml" ]; then
        cp "$INSTALL_DIR/config.example.yaml" ~/.clide/config.yaml
    else
        cat >~/.clide/config.yaml <<'YAML'
# Clide configuration - edit as needed
gemini_api_key: ""
gemini_model: "gemini-2.5-flash"
matrix_homeserver: "https://matrix.org"
matrix_user: ""
matrix_access_token: ""
matrix_room_id: ""
require_confirmation: false
confirmation_timeout: 60
authorized_users: []
blocked_commands:
  - "rm -rf /"
  - "mkfs"
  - "dd if="
logging:
  level: "info"
YAML
    fi
fi
chmod 600 ~/.clide/config.yaml

# ─── 4. Install skills ────────────────────────────────────────────────────────

step "Installing skills"

# Ensure we have the source repo to copy skills from
if [ ! -d "$INSTALL_DIR/skills" ]; then
    echo "   Fetching skills from repository..."
    git clone --depth=1 "https://github.com/${REPO}.git" "$INSTALL_DIR" \
        >/dev/null 2>&1 || true
fi

if [ -d "$INSTALL_DIR/skills" ]; then
    mkdir -p ~/.clide/skills
    cp -r "$INSTALL_DIR/skills/"* ~/.clide/skills/ 2>/dev/null || true
    # Secure skill files (readable only by owner)
    find ~/.clide/skills -name "*.yaml" -exec chmod 600 {} \; 2>/dev/null || true
    SKILL_COUNT=$(find ~/.clide/skills -name "*.yaml" | wc -l)
    echo "✅ $SKILL_COUNT skill(s) installed to ~/.clide/skills/"
else
    echo "⏭  Skills directory not found — skipping. Add .yaml files to ~/.clide/skills/ manually."
fi

# ─── 5. Interactive setup ─────────────────────────────────────────────────────

echo ""
echo "═══════════════════════════════════════" >/dev/tty
echo "  Quick Setup" >/dev/tty
echo "═══════════════════════════════════════" >/dev/tty
echo "  Press Enter to skip any step." >/dev/tty
echo "  You can edit ~/.clide/config.yaml later." >/dev/tty
echo "═══════════════════════════════════════" >/dev/tty

# ── 4a. Gemini API key ────────────────────────────────────────────────────────

echo "" >/dev/tty
echo "🔑 Gemini API Key" >/dev/tty
echo "   Get one free at: https://aistudio.google.com/app/apikey" >/dev/tty
echo "" >/dev/tty
ask "Enter API key (or press Enter to skip): " GEMINI_KEY secret

if [ -n "$GEMINI_KEY" ]; then
    # Save to env file for REPL mode (safe — no secret in ps)
    safe_env_set ~/.config/clide/config.env "GEMINI_API_KEY" "$GEMINI_KEY"
    chmod 600 ~/.config/clide/config.env
    export GEMINI_API_KEY="$GEMINI_KEY"

    # Patch yaml config
    safe_yaml_set ~/.clide/config.yaml "gemini_api_key" "$GEMINI_KEY"

    # Save to secrets.yaml (primary secrets store, highest priority after env vars)
    mkdir -p ~/.clide
    safe_yaml_set ~/.clide/secrets.yaml "GEMINI_API_KEY" "$GEMINI_KEY"
    chmod 600 ~/.clide/secrets.yaml

    echo "✅ Gemini API key saved" >/dev/tty
else
    echo "" >/dev/tty
    echo "⚠️  WARNING: No Gemini API key entered!" >/dev/tty
    echo "   Clide WILL NOT WORK without an API key." >/dev/tty
    echo "   Set it later:" >/dev/tty
    echo "     export GEMINI_API_KEY=\"your-key-here\"" >/dev/tty
    echo "     # or add to ~/.clide/secrets.yaml:" >/dev/tty
    echo "     GEMINI_API_KEY: \"your-key-here\"" >/dev/tty
    echo "" >/dev/tty
fi

# ── 4b. Platform selection ────────────────────────────────────────────────────

echo "" >/dev/tty
echo "📱 Choose your messaging platform" >/dev/tty
echo "═══════════════════════════════════════" >/dev/tty
echo "   1) Telegram  – easiest, just create a bot via @BotFather" >/dev/tty
echo "   2) Element   – Matrix/Element (requires Matrix account)" >/dev/tty
echo "   3) Both      – run Telegram and Matrix simultaneously" >/dev/tty
echo "═══════════════════════════════════════" >/dev/tty
echo "" >/dev/tty
ask "Enter choice [1/2/3] (default: 1): " PLATFORM_CHOICE

case "$PLATFORM_CHOICE" in
    2)
        CLIDE_PLATFORM="matrix"
        echo "✅ Using Element/Matrix" >/dev/tty
        ;;
    3)
        CLIDE_PLATFORM="both"
        echo "✅ Using Telegram + Element/Matrix" >/dev/tty
        ;;
    *)
        CLIDE_PLATFORM="telegram"
        echo "✅ Using Telegram" >/dev/tty
        ;;
esac

# Write platform to config
safe_yaml_set ~/.clide/config.yaml "platform" "$CLIDE_PLATFORM"

# ── 4c. Telegram setup ────────────────────────────────────────────────────────

if [ "$CLIDE_PLATFORM" = "telegram" ] || [ "$CLIDE_PLATFORM" = "both" ]; then
    echo "" >/dev/tty
    echo "🤖 Telegram Bot Setup" >/dev/tty
    echo "───────────────────────────────────────" >/dev/tty
    echo "   How to get a bot token:" >/dev/tty
    echo "   1. Open Telegram → search for @BotFather" >/dev/tty
    echo "   2. Send /newbot and follow the prompts" >/dev/tty
    echo "   3. Copy the token it gives you (looks like 123456:ABC-DEF...)" >/dev/tty
    echo "" >/dev/tty

    ask "Telegram bot token (or press Enter to skip): " TG_TOKEN secret

    if [ -n "$TG_TOKEN" ]; then
        safe_yaml_set ~/.clide/config.yaml "telegram_bot_token" "$TG_TOKEN"
        safe_env_set ~/.config/clide/config.env "TELEGRAM_BOT_TOKEN" "$TG_TOKEN"
        chmod 600 ~/.config/clide/config.env
        safe_yaml_set ~/.clide/secrets.yaml "TELEGRAM_BOT_TOKEN" "$TG_TOKEN"
        chmod 600 ~/.clide/secrets.yaml

        echo "✅ Telegram bot token saved" >/dev/tty
    else
        echo "⏭  Skipped. Set later via TELEGRAM_BOT_TOKEN env var or ~/.clide/secrets.yaml" >/dev/tty
    fi
fi

# ── 4c-2. Telegram authorized users ──────────────────────────────────────────

if [ "$CLIDE_PLATFORM" = "telegram" ] || [ "$CLIDE_PLATFORM" = "both" ]; then
    echo "" >/dev/tty
    echo "👤 Authorized Telegram Users" >/dev/tty
    echo "───────────────────────────────────────" >/dev/tty
    echo "   Only users in this list can send commands to the bot." >/dev/tty
    echo "   Enter Telegram usernames WITHOUT the @ sign." >/dev/tty
    echo "   (Press Enter with no name to skip or finish the list.)" >/dev/tty
    echo "" >/dev/tty

    TG_AUTHORIZED_USERS=()
    while true; do
        if [ ${#TG_AUTHORIZED_USERS[@]} -eq 0 ]; then
            ask "Your Telegram username (or press Enter to skip): " TG_AUTH_USER
        else
            ask "Add another username (or press Enter to finish): " TG_AUTH_USER
        fi
        [ -z "$TG_AUTH_USER" ] && break
        TG_AUTH_USER="${TG_AUTH_USER#@}"   # strip leading @ if the user included it
        TG_AUTHORIZED_USERS+=("$TG_AUTH_USER")
        echo "   ✅ Added: @$TG_AUTH_USER" >/dev/tty
    done

    if [ ${#TG_AUTHORIZED_USERS[@]} -gt 0 ]; then
        # Build the new authorized_users YAML block.
        TEMP_BLOCK=$(mktemp)
        echo "authorized_users:" > "$TEMP_BLOCK"
        for u in "${TG_AUTHORIZED_USERS[@]}"; do
            echo "  - \"$u\"" >> "$TEMP_BLOCK"
        done

        # Replace the existing authorized_users section in config.yaml.
        # Handles both single-line "authorized_users: []" and multi-line formats.
        FOUND_AUTH=false
        TEMP_CFG=$(mktemp)
        SKIP_LIST=false
        while IFS= read -r line; do
            if [[ "$line" =~ ^authorized_users: ]]; then
                cat "$TEMP_BLOCK"
                SKIP_LIST=true
                FOUND_AUTH=true
            elif $SKIP_LIST && [[ "$line" =~ ^[[:space:]]+-[[:space:]] ]]; then
                : # skip old list items
            else
                SKIP_LIST=false
                echo "$line"
            fi
        done < ~/.clide/config.yaml > "$TEMP_CFG"

        # If authorized_users was not found in config, append the block
        if [ "$FOUND_AUTH" = false ]; then
            echo "" >> "$TEMP_CFG"
            cat "$TEMP_BLOCK" >> "$TEMP_CFG"
        fi

        mv "$TEMP_CFG" ~/.clide/config.yaml
        rm -f "$TEMP_BLOCK"
        chmod 600 ~/.clide/config.yaml

        # Verify the usernames were actually written
        VERIFY_OK=true
        for u in "${TG_AUTHORIZED_USERS[@]}"; do
            if ! grep -q "\"$u\"" ~/.clide/config.yaml 2>/dev/null; then
                VERIFY_OK=false
                break
            fi
        done

        if [ "$VERIFY_OK" = true ]; then
            echo "✅ Authorized users saved" >/dev/tty
        else
            # Fallback: directly write the authorized_users block at end of file
            echo "" >/dev/tty
            echo "⚠️  Re-writing authorized users (fallback)..." >/dev/tty
            # Remove any partial authorized_users lines
            sed -i '/^authorized_users:/d' ~/.clide/config.yaml
            sed -i '/^  - ".*"$/d' ~/.clide/config.yaml
            # Append fresh block
            echo "" >>~/.clide/config.yaml
            echo "authorized_users:" >>~/.clide/config.yaml
            for u in "${TG_AUTHORIZED_USERS[@]}"; do
                echo "  - \"$u\"" >>~/.clide/config.yaml
            done
            chmod 600 ~/.clide/config.yaml
            echo "✅ Authorized users saved (fallback)" >/dev/tty
        fi
    else
        echo "⏭  Skipped. Add usernames to authorized_users in ~/.clide/config.yaml later." >/dev/tty
    fi
fi

# ── 4d. Matrix/Element credentials ───────────────────────────────────────────

if [ "$CLIDE_PLATFORM" = "matrix" ] || [ "$CLIDE_PLATFORM" = "both" ]; then

echo "" >/dev/tty
echo "💬 Matrix/Element Setup" >/dev/tty
echo "───────────────────────────────────────" >/dev/tty
echo "   You need a Matrix account and a room for the bot." >/dev/tty
echo "   If you don't have one, create a free account at https://app.element.io" >/dev/tty
echo "" >/dev/tty

ask "Homeserver URL (Enter for https://matrix.org, or skip to configure later): " MATRIX_HS

if [ -z "$MATRIX_HS" ]; then
    echo "" >/dev/tty
    ask "Skip Matrix setup entirely? [Y/n]: " SKIP_MATRIX
    if [[ "$SKIP_MATRIX" =~ ^[Nn] ]]; then
        MATRIX_HS="https://matrix.org"
    else
        MATRIX_HS=""
    fi
fi

if [ -n "$MATRIX_HS" ]; then
    MATRIX_HS="${MATRIX_HS%/}"   # strip trailing slash
    safe_yaml_set ~/.clide/config.yaml "matrix_homeserver" "$MATRIX_HS"

    echo "" >/dev/tty
    echo "   Homeserver: $MATRIX_HS" >/dev/tty
    echo "" >/dev/tty

    # ── Username ──────────────────────────────────────────────────────────────
    ask "Matrix username (e.g. @yourbot:matrix.org, or press Enter to skip): " MATRIX_USER

    if [ -n "$MATRIX_USER" ]; then
        safe_yaml_set ~/.clide/config.yaml "matrix_user" "$MATRIX_USER"
        echo "✅ Matrix user saved" >/dev/tty

        # ── Password → login to get access token ─────────────────────────────
        echo "" >/dev/tty
        echo "   Option A – enter your Matrix account PASSWORD to get a token automatically." >/dev/tty
        echo "   Option B – press Enter here, then paste your access token on the next prompt." >/dev/tty
        echo "" >/dev/tty
        echo "   ⚠️  Enter your ACCOUNT PASSWORD here, NOT an access token." >/dev/tty
        echo "      If you already have an access token, press Enter to skip to Option B." >/dev/tty
        echo "   (Password is sent only to your homeserver and never stored.)" >/dev/tty
        echo "" >/dev/tty
        ask "Matrix account password (Enter to skip → enter token directly): " MATRIX_PASS secret

        if [ -n "$MATRIX_PASS" ]; then
            LOCALPART=$(echo "$MATRIX_USER" | sed 's/^@//' | cut -d: -f1)
            echo "" >/dev/tty
            echo "   Logging in as $LOCALPART..." >/dev/tty

            LOGIN_RESP=$(curl -s --max-time 15 -XPOST "${MATRIX_HS}/_matrix/client/v3/login" \
                -H "Content-Type: application/json" \
                -d "{\"type\":\"m.login.password\",\"identifier\":{\"type\":\"m.id.user\",\"user\":\"$LOCALPART\"},\"password\":\"$MATRIX_PASS\"}" \
                2>/dev/null || true)

            if [ -n "$LOGIN_RESP" ]; then
                ACCESS_TOKEN=$(echo "$LOGIN_RESP" | grep -o '"access_token":"[^"]*"' \
                    | sed 's/"access_token":"//;s/"//' || true)
                ERRCODE=$(echo "$LOGIN_RESP" | grep -o '"errcode":"[^"]*"' \
                    | sed 's/"errcode":"//;s/"//' || true)

                if [ -n "$ACCESS_TOKEN" ]; then
                    safe_env_set ~/.config/clide/config.env "MATRIX_ACCESS_TOKEN" "$ACCESS_TOKEN"
                    chmod 600 ~/.config/clide/config.env
                    safe_yaml_set ~/.clide/config.yaml "matrix_access_token" "$ACCESS_TOKEN"
                    echo "✅ Access token obtained and saved" >/dev/tty
                elif [ -n "$ERRCODE" ]; then
                    ERRMSG=$(echo "$LOGIN_RESP" | grep -o '"error":"[^"]*"' \
                        | sed 's/"error":"//;s/"//' || true)
                    echo "⚠️  Login failed: $ERRCODE - $ERRMSG" >/dev/tty
                    echo "   Enter your access token manually below." >/dev/tty
                    echo "" >/dev/tty
                    ask "Access token (or press Enter to skip): " MANUAL_TOKEN secret
                    if [ -n "$MANUAL_TOKEN" ]; then
                        safe_yaml_set ~/.clide/config.yaml "matrix_access_token" "$MANUAL_TOKEN"
                        echo "✅ Access token saved" >/dev/tty
                    fi
                else
                    echo "⚠️  Could not parse login response. Enter token manually." >/dev/tty
                    ask "Access token (or press Enter to skip): " MANUAL_TOKEN secret
                    if [ -n "$MANUAL_TOKEN" ]; then
                        safe_yaml_set ~/.clide/config.yaml "matrix_access_token" "$MANUAL_TOKEN"
                        echo "✅ Access token saved" >/dev/tty
                    fi
                fi
            else
                echo "⚠️  Could not reach $MATRIX_HS. Check your network." >/dev/tty
                echo "   You can set the token later via MATRIX_ACCESS_TOKEN env var." >/dev/tty
            fi
        else
            echo "" >/dev/tty
            echo "   To get your token manually:" >/dev/tty
            echo "   Element → Settings → Help & About → Access Token" >/dev/tty
            echo "" >/dev/tty
            ask "Access token (or press Enter to skip): " MANUAL_TOKEN secret
            if [ -n "$MANUAL_TOKEN" ]; then
                safe_yaml_set ~/.clide/config.yaml "matrix_access_token" "$MANUAL_TOKEN"
                echo "✅ Access token saved" >/dev/tty
            else
                echo "⏭  Skipped. Set later via MATRIX_ACCESS_TOKEN env var." >/dev/tty
            fi
        fi
    else
        echo "⏭  Skipped. Edit ~/.clide/config.yaml to add Matrix credentials." >/dev/tty
    fi

    # ── Room ID ───────────────────────────────────────────────────────────────
    echo "" >/dev/tty
    echo "   Room ID: the bot listens in this Matrix room." >/dev/tty
    echo "   Find it: Element → Room → Settings → Advanced → Internal room ID" >/dev/tty
    echo "   Format: !abc123:matrix.org" >/dev/tty
    echo "" >/dev/tty
    ask "Room ID (or press Enter to skip): " MATRIX_ROOM

    if [ -n "$MATRIX_ROOM" ]; then
        safe_yaml_set ~/.clide/config.yaml "matrix_room_id" "$MATRIX_ROOM"
        echo "✅ Room ID saved" >/dev/tty
    else
        echo "⏭  Skipped. Edit matrix_room_id in ~/.clide/config.yaml later." >/dev/tty
    fi
else
    echo "⏭  Matrix setup skipped. Edit ~/.clide/config.yaml to configure later." >/dev/tty
fi

fi  # end Matrix block

# ── 4e. VPS / Server credentials (optional) ─────────────────────────────

echo "" >/dev/tty
echo "🖥️  VPS / Server Setup (optional)" >/dev/tty
echo "───────────────────────────────────────" >/dev/tty
echo "   Add SSH hosts so Clide can manage your servers." >/dev/tty
echo "   The installer will set up key-based SSH so Clide" >/dev/tty
echo "   can connect without a password (you enter it once)." >/dev/tty
echo "   You can also do this later with: clide host add" >/dev/tty
echo "   (Press Enter to skip.)" >/dev/tty
echo "" >/dev/tty
ask "Add a server now? [y/N]: " ADD_HOST

if [[ "$ADD_HOST" =~ ^[Yy] ]]; then
    HOST_COUNT=0
    while true; do
        echo "" >/dev/tty
        ask "  Server nickname (e.g. prod, pi, vps): " HOST_NICK
        [ -z "$HOST_NICK" ] && break

        ask "  IP address or hostname: " HOST_IP
        if [ -z "$HOST_IP" ]; then
            echo "  ⚠️  IP is required, skipping this host." >/dev/tty
            continue
        fi

        ask "  SSH user [root]: " HOST_USER
        HOST_USER="${HOST_USER:-root}"

        ask "  SSH port [22]: " HOST_PORT
        HOST_PORT="${HOST_PORT:-22}"

        ask "  Notes (optional): " HOST_NOTES

        # ── SSH key generation & deployment ──────────────────────────────
        # Use a per-host key so revoking one server doesn't affect others.
        HOST_KEY="$HOME/.ssh/id_ed25519_${HOST_NICK}"

        mkdir -p ~/.ssh
        chmod 700 ~/.ssh

        if [ -f "$HOST_KEY" ]; then
            echo "  🔑 SSH key already exists: $HOST_KEY" >/dev/tty
        else
            echo "" >/dev/tty
            echo "  🔑 Generating SSH key for '$HOST_NICK'..." >/dev/tty
            ssh-keygen -t ed25519 -f "$HOST_KEY" -N "" -C "clide@${HOST_NICK}" </dev/null >/dev/tty 2>&1
            if [ -f "$HOST_KEY" ]; then
                chmod 600 "$HOST_KEY"
                chmod 644 "${HOST_KEY}.pub"
                echo "  ✅ Key generated: $HOST_KEY" >/dev/tty
            else
                echo "  ❌ Key generation failed. You can create it later with:" >/dev/tty
                echo "     ssh-keygen -t ed25519 -f $HOST_KEY -N \"\"" >/dev/tty
            fi
        fi

        # ── Copy key to server (password required once) ──────────────────
        if [ -f "${HOST_KEY}.pub" ]; then
            echo "" >/dev/tty
            echo "  📤 Copying key to ${HOST_USER}@${HOST_IP}..." >/dev/tty
            echo "     You'll need to enter the server password ONE LAST TIME." >/dev/tty
            echo "     After this, Clide connects without a password." >/dev/tty
            echo "" >/dev/tty

            # ssh-copy-id reads password from /dev/tty automatically
            if ssh-copy-id -i "${HOST_KEY}.pub" -p "$HOST_PORT" \
                    -o StrictHostKeyChecking=accept-new \
                    "${HOST_USER}@${HOST_IP}" </dev/tty >/dev/tty 2>&1; then

                echo "" >/dev/tty
                echo "  ✅ Key copied! Testing passwordless login..." >/dev/tty

                # Verify it works without a password
                if ssh -i "$HOST_KEY" -p "$HOST_PORT" \
                        -o BatchMode=yes -o ConnectTimeout=10 \
                        "${HOST_USER}@${HOST_IP}" "echo OK" >/dev/null 2>&1; then
                    echo "  ✅ Passwordless SSH works! Clide can now manage '$HOST_NICK'." >/dev/tty
                else
                    echo "  ⚠️  Key was copied but passwordless test failed." >/dev/tty
                    echo "     Check that the server allows key-based auth (PubkeyAuthentication yes)." >/dev/tty
                    echo "     You can test manually: ssh -i $HOST_KEY -p $HOST_PORT ${HOST_USER}@${HOST_IP}" >/dev/tty
                fi
            else
                echo "" >/dev/tty
                echo "  ⚠️  Could not copy key (wrong password or server unreachable)." >/dev/tty
                echo "     You can do it manually later:" >/dev/tty
                echo "     ssh-copy-id -i ${HOST_KEY}.pub -p $HOST_PORT ${HOST_USER}@${HOST_IP}" >/dev/tty
            fi
        fi

        # ── Save to hosts.yaml ───────────────────────────────────────────
        mkdir -p ~/.clide
        if [ ! -f ~/.clide/hosts.yaml ]; then
            : >~/.clide/hosts.yaml
        fi

        cat >>~/.clide/hosts.yaml <<HOSTEOF

$HOST_NICK:
  ip: "$HOST_IP"
  user: "$HOST_USER"
  key_path: "$HOST_KEY"
  port: $HOST_PORT
  notes: "$HOST_NOTES"
HOSTEOF
        chmod 600 ~/.clide/hosts.yaml

        HOST_COUNT=$((HOST_COUNT + 1))
        echo "  ✅ Host '$HOST_NICK' saved to ~/.clide/hosts.yaml" >/dev/tty
        echo "" >/dev/tty
        ask "  Add another server? [y/N]: " ADD_MORE
        [[ ! "$ADD_MORE" =~ ^[Yy] ]] && break
    done

    if [ "$HOST_COUNT" -gt 0 ]; then
        echo "✅ $HOST_COUNT server(s) configured with key-based SSH" >/dev/tty
    fi
else
    echo "⏭  Skipped. Add servers later with: clide host add" >/dev/tty
fi

# ─── 6. Summary ───────────────────────────────────────────────────────────────

echo ""
echo "═══════════════════════════════════════"
echo "✅ Installation Complete!"
echo "═══════════════════════════════════════"
echo ""

if command -v clide >/dev/null 2>&1; then
    echo "✅ $(clide --version)"
fi

echo ""
echo "Usage:"
echo "  clide              # Chat with Gemini (REPL)"
echo "  clide bot          # Start bot (platform set in config)"
echo "  clide --version    # Show version"
echo ""
echo "Config:   ~/.clide/config.yaml  (platform: \"$CLIDE_PLATFORM\")"
echo "Env file: ~/.config/clide/config.env"
echo ""

case "$CLIDE_PLATFORM" in
    telegram)
        echo "Next steps:"
        echo "  1. Make sure telegram_bot_token is set in ~/.clide/config.yaml"
        echo "  2. Open Telegram, start a chat with your bot, send any message"
        echo "  3. Run: clide bot"
        echo ""
        echo "Telegram quickstart:"
        echo "  - Create bot: @BotFather → /newbot"
        echo "  - Token format: 123456789:ABCdefGHI..."
        ;;
    matrix)
        echo "Next steps:"
        echo "  1. Make sure ~/.clide/config.yaml has all Matrix fields filled in"
        echo "  2. Invite your bot account to the Matrix room"
        echo "  3. Run: clide bot"
        echo ""
        echo "Element/Matrix quickstart:"
        echo "  - Free account: https://app.element.io"
        echo "  - Access token: Element → Settings → Help & About → Access Token"
        echo "  - Room ID: Room → Settings → Advanced → Internal room ID"
        ;;
    both)
        echo "Next steps:"
        echo "  1. Fill in both telegram_bot_token and Matrix fields in ~/.clide/config.yaml"
        echo "  2. Invite your Matrix bot to the room"
        echo "  3. Run: clide bot  (starts both bots simultaneously)"
        ;;
esac

echo "─── Secrets & Hosts ────────────────────────────────────────────"
echo ""
echo "  clide secret list              # show all stored secret keys"
echo "  clide secret set MY_KEY        # store a secret (hidden input)"
echo "  clide secret generate MY_KEY   # generate + store a random secret"
echo "  clide secret pass-init         # set up GNU pass (optional GPG layer)"
echo ""
echo "  clide host add                 # add an SSH host by nickname"
echo "  clide host list                # show all configured hosts"
echo ""
echo "─── Backup & Recovery ──────────────────────────────────────────"
echo ""
echo "  Via Telegram: 'backup my vault'"
echo "  To restore on a fresh device:"
echo "    bash install.sh --restore"
echo ""
echo "  Vault is age-encrypted → GitHub Gist."
echo "  Recovery needs: GitHub token + Gist ID + your passphrase."
echo ""

# ─── Platform-specific tips ──────────────────────────────────────────────────
if [ "$PLATFORM" = "termux" ]; then
    echo "─── Termux Tips ────────────────────────────────────────────────"
    echo ""
    echo "  💡 Run 'termux-wake-lock' to prevent Android from killing Clide"
    echo "     while it runs in the background."
    echo ""
    echo "  💡 If packages fail to download, try: termux-change-repo"
    echo "     to switch to a faster mirror."
    echo ""
elif [ "$PLATFORM" = "linux" ]; then
    echo "─── Linux VPS Tips ─────────────────────────────────────────────"
    echo ""
    echo "  💡 Run Clide in the background with: nohup clide bot &"
    echo "     or use a process manager like systemd or tmux."
    echo ""
fi
