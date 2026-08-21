#!/usr/bin/env bash
# Start two generic recording sessions: `dev` (4 tabs) and `ops` (2 tabs).
# Creates them in the background so the current session is left alone.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
chmod +x demo/fixtures/hold.sh

session_names() {
    zellij list-sessions -n 2>/dev/null | awk '{print $1}'
}

session_exists() {
    session_names | grep -qx "$1"
}

start_session() {
    local name="$1"
    local layout="$2"
    if session_exists "$name"; then
        echo "session already running: $name"
        return
    fi
    # new-tab into a background session suspends every command pane.
    # Start from the layout instead, then drop the temporary client.
    script -q /dev/null \
        zellij --session "$name" --new-session-with-layout "$layout" \
        >/dev/null 2>&1 &
    local pid=$!
    local i
    for i in $(seq 1 25); do
        if session_exists "$name"; then
            break
        fi
        sleep 0.2
    done
    sleep 0.5
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
    echo "started $name"
}

start_session dev "$root/layouts/demo-dev.kdl"
start_session ops "$root/layouts/demo-ops.kdl"
echo
zellij list-sessions
echo
echo "attach with:  zellij attach dev"
echo "stop with:    ./scripts/stop-demo-sessions.sh"
