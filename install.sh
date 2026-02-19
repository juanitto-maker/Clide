#!/data/data/com.termux/files/usr/bin/bash
# ============================================
# Clide Installer for Termux
# One-liner: curl -fsSL https://raw.githubusercontent.com/juanitto-maker/Clide/main/install.sh | bash
# ============================================

set -e

REPO="juanitto-maker/Clide"
INSTALL_DIR="$HOME/Clide_Source"

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
pkg install -y git wget curl 2>&1 | grep -E "^(Unpacking|Setting up)" | sed 's/^/   /' || true
echo "✅ Done"

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

# ─── 4. Interactive setup ─────────────────────────────────────────────────────

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
    echo "✅ Gemini API key saved" >/dev/tty
else
    echo "⏭  Skipped. Set later via GEMINI_API_KEY env var or ~/.clide/config.yaml" >/dev/tty
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
        echo "✅ Telegram bot token saved" >/dev/tty
    else
        echo "⏭  Skipped. Set later via TELEGRAM_BOT_TOKEN env var or ~/.clide/config.yaml" >/dev/tty
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

# ─── 5. Summary ───────────────────────────────────────────────────────────────

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
echo ""
