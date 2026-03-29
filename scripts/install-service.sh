#!/usr/bin/env bash
# ─────────────────────────────────────────────
# install-service.sh — Install Clide as a systemd service
# Run once on your VPS after the binary is built/downloaded.
# Usage: sudo bash scripts/install-service.sh
# ─────────────────────────────────────────────
set -euo pipefail

# ── Defaults (override via env) ──
CLIDE_USER="${CLIDE_USER:-$(logname 2>/dev/null || echo "${SUDO_USER:-root}")}"
CLIDE_BIN="${CLIDE_BIN:-/usr/local/bin/clide}"
SERVICE_FILE="/etc/systemd/system/clide.service"

# ── Require root ──
if [ "$(id -u)" -ne 0 ]; then
    echo "Error: run this script with sudo or as root."
    exit 1
fi

# ── Find the binary ──
if [ ! -f "$CLIDE_BIN" ]; then
    # Try current directory
    if [ -f "./target/release/clide" ]; then
        echo "Copying binary to $CLIDE_BIN ..."
        cp ./target/release/clide "$CLIDE_BIN"
        chmod 755 "$CLIDE_BIN"
    elif [ -f "./clide" ]; then
        echo "Copying binary to $CLIDE_BIN ..."
        cp ./clide "$CLIDE_BIN"
        chmod 755 "$CLIDE_BIN"
    else
        echo "Error: clide binary not found at $CLIDE_BIN or in current directory."
        echo "Build first (cargo build --release) or set CLIDE_BIN=/path/to/clide"
        exit 1
    fi
else
    echo "Binary already at $CLIDE_BIN"
fi

# ── Stop existing service if running ──
if systemctl is-active clide >/dev/null 2>&1; then
    echo "Stopping existing clide service..."
    systemctl stop clide
fi

# ── Write the unit file ──
echo "Writing $SERVICE_FILE ..."
cat > "$SERVICE_FILE" <<EOF
[Unit]
Description=Clide Bot
After=network.target

[Service]
User=${CLIDE_USER}
ExecStart=${CLIDE_BIN} bot
Restart=always
RestartSec=5
Environment=HOME=/home/${CLIDE_USER}

[Install]
WantedBy=multi-user.target
EOF

# ── Enable and start ──
echo "Reloading systemd..."
systemctl daemon-reload
systemctl enable clide
systemctl start clide

echo ""
echo "Done! Clide is running as a systemd service."
echo "  Status:  sudo systemctl status clide"
echo "  Logs:    sudo journalctl -u clide -f"
echo "  Stop:    sudo systemctl stop clide"
echo "  Restart: sudo systemctl restart clide"
