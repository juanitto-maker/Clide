#!/data/data/com.termux/files/usr/bin/bash
# ============================================
# Clide Installer for Termux
# One-liner: curl -fsSL https://raw.githubusercontent.com/juanitto-maker/Clide/main/install.sh | bash
# ============================================

set -e

REPO="juanitto-maker/Clide"
INSTALL_DIR="$HOME/Clide_Source"
RESTORE_MODE=false

# ─── Parse flags ──────────────────────────────────────────────────────────────
for arg in "$@"; do
    case "$arg" in
        --restore) RESTORE_MODE=true ;;
    esac
done

# ─── Guards ───────────────────────────────────────────────────────────────────

if [[ ! "$PREFIX" =~ "com.termux" ]]; then
    echo "❌ This installer is for Termux on Android only."
    exit 1
fi
echo "✅ Termux detected"

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
pkg update -y 2>&1 | grep -E "^(Get:|Fetched|Reading)" | tail -5 || true
echo "✅ Done"

step "Installing dependencies"
pkg install -y git wget curl age 2>&1 | grep -E "^(Unpacking|Setting up)" | sed 's/^/   /' || true
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

    # Save the Gist ID for future vault operations
    echo "$GIST_ID" > "$HOME/.clide/vault_gist_id"
    chmod 600 "$HOME/.clide/vault_gist_id"

    echo ""
    echo "✅ Vault restored!"
    echo "   secrets.yaml : $([ -f "$HOME/.clide/secrets.yaml" ] && echo "OK" || echo "NOT FOUND")"
    echo "   hosts.yaml   : $([ -f "$HOME/.clide/hosts.yaml"   ] && echo "OK" || echo "NOT FOUND")"
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
mkdir -p "$PREFIX/bin"

CLIDE_INSTALLED=false

# 2a. Try pre-built binary from GitHub Releases (fast path, skips Rust build)
echo "   Checking for pre-built binary..."
LATEST_TAG=$(wget -qO- "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null \
    | grep '"tag_name"' | sed 's/.*"v\([^"]*\)".*/\1/' | head -1 || true)

if [ -n "$LATEST_TAG" ]; then
    BIN_URL="https://github.com/${REPO}/releases/download/v${LATEST_TAG}/clide-aarch64"
    echo "   Trying pre-built binary for v${LATEST_TAG}..."
    if wget -q "$BIN_URL" -O "$PREFIX/bin/clide" 2>/dev/null; then
        chmod +x "$PREFIX/bin/clide"
        echo "✅ Pre-built binary installed (v${LATEST_TAG})"
        CLIDE_INSTALLED=true
    else
        echo "   No aarch64 binary in release v${LATEST_TAG} — will build from source."
        rm -f "$PREFIX/bin/clide"
    fi
else
    echo "   No release found — will build from source."
fi

# 2b. Build from source (fallback)
if [ "$CLIDE_INSTALLED" = false ]; then
    step "Building Clide from source"
    echo "   ⚠️  No pre-built binary — compiling from source."
    echo "   This takes 10-15 min on most devices. Keep Termux open."
    echo ""

    pkg install -y rust binutils pkg-config openssl \
        >"$TMPDIR/pkg_rust.log" 2>&1 &
    spinner $! "Installing Rust toolchain"
    wait $! || { echo "❌ Rust install failed"; cat "$TMPDIR/pkg_rust.log"; exit 1; }
    grep -E "^(Unpacking|Setting up)" "$TMPDIR/pkg_rust.log" | \
        sed 's/^/   /' | tail -3 || true

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

    # Termux build environment
    export CC="$PREFIX/bin/clang"
    export AR="$PREFIX/bin/llvm-ar"
    export OPENSSL_INCLUDE_DIR="$PREFIX/include"
    export OPENSSL_LIB_DIR="$PREFIX/lib"

    echo "   Compiling Clide... (started $(date '+%H:%M:%S'), this will take a while)"
    BUILD_LOG="$TMPDIR/clide_build.log"

    cargo build --release 2>&1 | tee "$BUILD_LOG" | \
        grep -E "^(   Compiling|   Finished|  Downloaded|  Downloading|error\[)" || true

    if [ ! -f "target/release/clide" ]; then
        echo "❌ Build failed. Log: $BUILD_LOG"
        exit 1
    fi

    cp target/release/clide "$PREFIX/bin/clide"
    chmod +x "$PREFIX/bin/clide"
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
    # Save to env file for REPL mode
    if grep -q "GEMINI_API_KEY" ~/.config/clide/config.env 2>/dev/null; then
        sed -i "s|GEMINI_API_KEY=.*|GEMINI_API_KEY=$GEMINI_KEY|" ~/.config/clide/config.env
    else
        echo "GEMINI_API_KEY=$GEMINI_KEY" >>~/.config/clide/config.env
    fi
    chmod 600 ~/.config/clide/config.env
    export GEMINI_API_KEY="$GEMINI_KEY"

    # Patch yaml config
    sed -i "s|gemini_api_key:.*|gemini_api_key: \"$GEMINI_KEY\"|" ~/.clide/config.yaml

    # Save to secrets.yaml (primary secrets store, highest priority after env vars)
    mkdir -p ~/.clide
    if [ -f ~/.clide/secrets.yaml ]; then
        if grep -q "^GEMINI_API_KEY:" ~/.clide/secrets.yaml 2>/dev/null; then
            sed -i "s|^GEMINI_API_KEY:.*|GEMINI_API_KEY: \"$GEMINI_KEY\"|" ~/.clide/secrets.yaml
        else
            echo "GEMINI_API_KEY: \"$GEMINI_KEY\"" >>~/.clide/secrets.yaml
        fi
    else
        echo "GEMINI_API_KEY: \"$GEMINI_KEY\"" >~/.clide/secrets.yaml
    fi
    chmod 600 ~/.clide/secrets.yaml

    echo "✅ Gemini API key saved" >/dev/tty
else
    echo "⏭  Skipped. Set later via GEMINI_API_KEY env var or ~/.clide/secrets.yaml" >/dev/tty
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
if grep -q "^platform:" ~/.clide/config.yaml 2>/dev/null; then
    sed -i "s|^platform:.*|platform: \"$CLIDE_PLATFORM\"|" ~/.clide/config.yaml
else
    echo "platform: \"$CLIDE_PLATFORM\"" >>~/.clide/config.yaml
fi

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
        if grep -q "telegram_bot_token:" ~/.clide/config.yaml 2>/dev/null; then
            sed -i "s|telegram_bot_token:.*|telegram_bot_token: \"$TG_TOKEN\"|" ~/.clide/config.yaml
        else
            echo "telegram_bot_token: \"$TG_TOKEN\"" >>~/.clide/config.yaml
        fi
        if grep -q "TELEGRAM_BOT_TOKEN" ~/.config/clide/config.env 2>/dev/null; then
            sed -i "s|TELEGRAM_BOT_TOKEN=.*|TELEGRAM_BOT_TOKEN=$TG_TOKEN|" ~/.config/clide/config.env
        else
            echo "TELEGRAM_BOT_TOKEN=$TG_TOKEN" >>~/.config/clide/config.env
        fi
        chmod 600 ~/.config/clide/config.env

        # Save to secrets.yaml (primary secrets store)
        if [ -f ~/.clide/secrets.yaml ]; then
            if grep -q "^TELEGRAM_BOT_TOKEN:" ~/.clide/secrets.yaml 2>/dev/null; then
                sed -i "s|^TELEGRAM_BOT_TOKEN:.*|TELEGRAM_BOT_TOKEN: \"$TG_TOKEN\"|" ~/.clide/secrets.yaml
            else
                echo "TELEGRAM_BOT_TOKEN: \"$TG_TOKEN\"" >>~/.clide/secrets.yaml
            fi
        else
            echo "TELEGRAM_BOT_TOKEN: \"$TG_TOKEN\"" >~/.clide/secrets.yaml
        fi
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
    sed -i "s|matrix_homeserver:.*|matrix_homeserver: \"$MATRIX_HS\"|" ~/.clide/config.yaml

    echo "" >/dev/tty
    echo "   Homeserver: $MATRIX_HS" >/dev/tty
    echo "" >/dev/tty

    # ── Username ──────────────────────────────────────────────────────────────
    ask "Matrix username (e.g. @yourbot:matrix.org, or press Enter to skip): " MATRIX_USER

    if [ -n "$MATRIX_USER" ]; then
        sed -i "s|matrix_user:.*|matrix_user: \"$MATRIX_USER\"|" ~/.clide/config.yaml
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
                    if grep -q "MATRIX_ACCESS_TOKEN" ~/.config/clide/config.env 2>/dev/null; then
                        sed -i "s|MATRIX_ACCESS_TOKEN=.*|MATRIX_ACCESS_TOKEN=$ACCESS_TOKEN|" ~/.config/clide/config.env
                    else
                        echo "MATRIX_ACCESS_TOKEN=$ACCESS_TOKEN" >>~/.config/clide/config.env
                    fi
                    chmod 600 ~/.config/clide/config.env
                    sed -i "s|matrix_access_token:.*|matrix_access_token: \"$ACCESS_TOKEN\"|" ~/.clide/config.yaml
                    echo "✅ Access token obtained and saved" >/dev/tty
                elif [ -n "$ERRCODE" ]; then
                    ERRMSG=$(echo "$LOGIN_RESP" | grep -o '"error":"[^"]*"' \
                        | sed 's/"error":"//;s/"//' || true)
                    echo "⚠️  Login failed: $ERRCODE - $ERRMSG" >/dev/tty
                    echo "   Enter your access token manually below." >/dev/tty
                    echo "" >/dev/tty
                    ask "Access token (or press Enter to skip): " MANUAL_TOKEN secret
                    if [ -n "$MANUAL_TOKEN" ]; then
                        sed -i "s|matrix_access_token:.*|matrix_access_token: \"$MANUAL_TOKEN\"|" ~/.clide/config.yaml
                        echo "✅ Access token saved" >/dev/tty
                    fi
                else
                    echo "⚠️  Could not parse login response. Enter token manually." >/dev/tty
                    ask "Access token (or press Enter to skip): " MANUAL_TOKEN secret
                    if [ -n "$MANUAL_TOKEN" ]; then
                        sed -i "s|matrix_access_token:.*|matrix_access_token: \"$MANUAL_TOKEN\"|" ~/.clide/config.yaml
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
                sed -i "s|matrix_access_token:.*|matrix_access_token: \"$MANUAL_TOKEN\"|" ~/.clide/config.yaml
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
        sed -i "s|matrix_room_id:.*|matrix_room_id: \"$MATRIX_ROOM\"|" ~/.clide/config.yaml
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

        ask "  SSH key path [$HOME/.ssh/id_ed25519]: " HOST_KEY
        HOST_KEY="${HOST_KEY:-$HOME/.ssh/id_ed25519}"

        ask "  SSH port [22]: " HOST_PORT
        HOST_PORT="${HOST_PORT:-22}"

        ask "  Notes (optional): " HOST_NOTES

        # Write to hosts.yaml
        mkdir -p ~/.clide
        if [ ! -f ~/.clide/hosts.yaml ]; then
            : >~/.clide/hosts.yaml
        fi

        # Append host entry
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
        echo "  ✅ Host '$HOST_NICK' saved" >/dev/tty
        echo "" >/dev/tty
        ask "  Add another server? [y/N]: " ADD_MORE
        [[ ! "$ADD_MORE" =~ ^[Yy] ]] && break
    done

    if [ "$HOST_COUNT" -gt 0 ]; then
        echo "✅ $HOST_COUNT server(s) saved to ~/.clide/hosts.yaml" >/dev/tty
    fi

    # Ask if they want to store a VPS password in secrets
    echo "" >/dev/tty
    ask "  Store a VPS root/sudo password? [y/N]: " STORE_VPS_PASS
    if [[ "$STORE_VPS_PASS" =~ ^[Yy] ]]; then
        ask "  Secret name (e.g. VPS_ROOT_PASSWORD): " VPS_SECRET_NAME
        VPS_SECRET_NAME="${VPS_SECRET_NAME:-VPS_ROOT_PASSWORD}"
        ask "  Password (hidden): " VPS_SECRET_VAL secret
        if [ -n "$VPS_SECRET_VAL" ]; then
            if [ -f ~/.clide/secrets.yaml ]; then
                if grep -q "^${VPS_SECRET_NAME}:" ~/.clide/secrets.yaml 2>/dev/null; then
                    sed -i "s|^${VPS_SECRET_NAME}:.*|${VPS_SECRET_NAME}: \"$VPS_SECRET_VAL\"|" ~/.clide/secrets.yaml
                else
                    echo "${VPS_SECRET_NAME}: \"$VPS_SECRET_VAL\"" >>~/.clide/secrets.yaml
                fi
            else
                echo "${VPS_SECRET_NAME}: \"$VPS_SECRET_VAL\"" >~/.clide/secrets.yaml
            fi
            chmod 600 ~/.clide/secrets.yaml
            echo "  ✅ ${VPS_SECRET_NAME} saved to secrets.yaml" >/dev/tty
        fi
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
