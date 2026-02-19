#!/usr/bin/env bash
# ============================================================
# debug.sh — Clide connectivity & config debugger
# Run this to pinpoint exactly where the bot is stuck.
# Usage: bash debug.sh
# ============================================================

set -euo pipefail

CONFIG="$HOME/.clide/config.yaml"
PASS="✅"
FAIL="❌"
WARN="⚠️ "

section() { echo; echo "──────────────────────────────────────"; echo "  $1"; echo "──────────────────────────────────────"; }
ok()   { echo "$PASS  $1"; }
fail() { echo "$FAIL  $1"; }
warn() { echo "$WARN $1"; }

# ── 1. Config file ───────────────────────────────────────────
section "1 / 5  Config file"

if [[ ! -f "$CONFIG" ]]; then
    fail "Config not found at $CONFIG"
    echo "     Run the installer or copy config.example.yaml"
    exit 1
fi
ok "Config found: $CONFIG"

# Extract values (strips quotes, handles spaces)
get_val() {
    grep -E "^$1:" "$CONFIG" \
        | head -1 \
        | sed 's/^[^:]*:[[:space:]]*//' \
        | tr -d '"' \
        | tr -d "'" \
        | tr -d '\r'
}

HS=$(get_val matrix_homeserver)
USER_ID=$(get_val matrix_user)
TOKEN=$(get_val matrix_access_token)
ROOM=$(get_val matrix_room_id)
GEMINI_KEY=$(get_val gemini_api_key)
MODEL=$(get_val gemini_model)

echo "     homeserver  : $HS"
echo "     matrix_user : $USER_ID"
echo "     room_id     : $ROOM"
echo "     gemini_model: $MODEL"

if [[ -z "$TOKEN" ]]; then
    fail "matrix_access_token is empty in config"
    exit 1
fi
# Show only first/last 4 chars of token
TSHOW="${TOKEN:0:4}...${TOKEN: -4}"
echo "     access_token: $TSHOW  (showing first/last 4 chars)"

if [[ -z "$HS" || -z "$ROOM" || -z "$GEMINI_KEY" ]]; then
    fail "One or more required config values are empty (homeserver / room_id / gemini_api_key)"
    exit 1
fi

# ── 2. Homeserver reachable ───────────────────────────────────
section "2 / 5  Homeserver connectivity"

VERSIONS=$(curl -sf --max-time 10 "${HS}/_matrix/client/versions" 2>&1) || true
if echo "$VERSIONS" | grep -q '"versions"'; then
    ok "Homeserver is reachable: $HS"
else
    fail "Cannot reach homeserver at $HS"
    echo "     Response: $VERSIONS"
    exit 1
fi

# ── 3. Access token / whoami ─────────────────────────────────
section "3 / 5  Access token (whoami)"

WHOAMI=$(curl -sf --max-time 10 \
    -H "Authorization: Bearer $TOKEN" \
    "${HS}/_matrix/client/v3/account/whoami" 2>&1) || true

if echo "$WHOAMI" | grep -q '"user_id"'; then
    AUTHED_AS=$(echo "$WHOAMI" | grep -o '"user_id":"[^"]*"' | sed 's/"user_id":"//;s/"//')
    ok "Token is valid. Bot is authenticated as: $AUTHED_AS"

    if [[ "$AUTHED_AS" == "$USER_ID" ]]; then
        ok "matrix_user in config matches authenticated identity."
    else
        warn "matrix_user in config ('$USER_ID') differs from authenticated identity ('$AUTHED_AS')."
        echo "     The bot will ignore messages FROM $AUTHED_AS (self-response guard)."
        echo "     Messages from $USER_ID will be processed normally — this is OK for two-account setup."
    fi

    if [[ "$AUTHED_AS" != *"bot"* && "$AUTHED_AS" == "$USER_ID" ]]; then
        warn "The authenticated user looks like a personal account, not a dedicated bot account."
        echo "     If you send messages from $USER_ID the bot will silently discard them"
        echo "     (because it considers them self-responses)."
    fi
else
    fail "Token is INVALID or expired."
    echo "     Matrix said: $WHOAMI"
    echo
    echo "     Fix: re-run the installer to get a fresh token, or manually:"
    echo "       curl -XPOST ${HS}/_matrix/client/v3/login \\"
    echo "         -H 'Content-Type: application/json' \\"
    echo "         -d '{\"type\":\"m.login.password\",\"identifier\":{\"type\":\"m.id.user\",\"user\":\"LOCALPART\"},\"password\":\"PASSWORD\"}'"
    echo "     Then copy the access_token into ~/.clide/config.yaml"
    exit 1
fi

# ── 4. Room membership & sync ────────────────────────────────
section "4 / 5  Room membership (/sync)"

echo "     Doing a quick /sync (may take a few seconds)…"
SYNC=$(curl -sf --max-time 30 \
    -H "Authorization: Bearer $TOKEN" \
    "${HS}/_matrix/client/v3/sync?timeout=1000" 2>&1) || true

if ! echo "$SYNC" | grep -q '"next_batch"'; then
    fail "Sync failed or returned unexpected data."
    echo "     Raw response (first 500 chars): ${SYNC:0:500}"
    exit 1
fi
ok "Sync succeeded."

# Check if configured room is present
if echo "$SYNC" | grep -qF "\"$ROOM\""; then
    ok "Configured room '$ROOM' is visible in sync."

    # Check encryption
    if echo "$SYNC" | python3 -c "
import sys, json
data = json.load(sys.stdin)
room = data.get('rooms',{}).get('join',{}).get('$ROOM',{})
events = room.get('timeline',{}).get('events',[])
enc = [e for e in events if e.get('type') == 'm.room.encrypted']
plain = [e for e in events if e.get('type') == 'm.room.message']
print('enc=' + str(len(enc)))
print('plain=' + str(len(plain)))
" 2>/dev/null; then
        :
    fi

    # Simpler encryption check via grep
    ROOM_CHUNK=$(echo "$SYNC" | grep -o "\"$ROOM\":{.*" | head -c 2000 || true)
    if echo "$ROOM_CHUNK" | grep -q '"m.room.encrypted"'; then
        fail "Room contains encrypted messages (m.room.encrypted)."
        echo "     The bot cannot decrypt these. Disable E2E encryption in the room."
    else
        ok "No encrypted messages seen in timeline."
    fi

else
    # List rooms bot IS in
    JOINED=$(echo "$SYNC" | grep -o '"rooms":{"join":{[^}]*' | grep -o '"![^"]*"' | tr -d '"' | tr '\n' ' ' || true)
    fail "Configured room '$ROOM' NOT found in sync."
    if [[ -n "$JOINED" ]]; then
        echo "     Bot is joined to these rooms instead:"
        echo "$SYNC" | python3 -c "
import sys, json
data = json.load(sys.stdin)
rooms = list(data.get('rooms',{}).get('join',{}).keys())
for r in rooms:
    print('       ' + r)
" 2>/dev/null || echo "       $JOINED"
    else
        echo "     Bot is not joined to ANY room."
    fi
    echo
    echo "     Fix: copy the 'Internal room ID' from Element → open the room"
    echo "          → Room Settings → Advanced → Internal room ID"
    echo "          and paste it as matrix_room_id in ~/.clide/config.yaml"
    exit 1
fi

# ── 5. Gemini API ────────────────────────────────────────────
section "5 / 5  Gemini API"

GEMINI_RESP=$(curl -sf --max-time 20 \
    "https://generativelanguage.googleapis.com/v1beta/models/${MODEL}:generateContent?key=${GEMINI_KEY}" \
    -H "Content-Type: application/json" \
    -d '{"contents":[{"parts":[{"text":"Say the single word PONG"}]}],"generationConfig":{"maxOutputTokens":16,"responseMimeType":"text/plain"}}' \
    2>&1) || true

if echo "$GEMINI_RESP" | grep -q '"candidates"'; then
    REPLY=$(echo "$GEMINI_RESP" | grep -o '"text":"[^"]*"' | head -1 | sed 's/"text":"//;s/"$//')
    ok "Gemini API is working. Test reply: \"$REPLY\""
elif echo "$GEMINI_RESP" | grep -q '"error"'; then
    fail "Gemini API returned an error:"
    echo "     $GEMINI_RESP" | grep -o '"message":"[^"]*"'
    echo
    echo "     Common fixes:"
    echo "       - Check gemini_api_key in config"
    echo "       - Check gemini_model is a valid model name (e.g. gemini-2.0-flash)"
    echo "       - Verify your API key at https://aistudio.google.com/app/apikey"
else
    fail "Unexpected Gemini response: ${GEMINI_RESP:0:300}"
fi

# ── Summary ──────────────────────────────────────────────────
section "All checks passed"
echo "  The bot config looks correct."
echo "  If it still does not respond:"
echo "    1. Rebuild:  cargo build --release"
echo "    2. Restart:  ./target/release/clide"
echo "    3. Check terminal for 'Message from' lines when you send a chat."
echo "    4. If you see no 'Message from' lines, run this script again"
echo "       with RUST_LOG=debug to see every sync cycle."
echo
