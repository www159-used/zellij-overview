#!/usr/bin/env bash
# Load the plugin in a throwaway Zellij session. Catches host/event drift
# when the installed Zellij version moves.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

if ! command -v zellij >/dev/null 2>&1; then
  if [[ "${OVERVIEW_E2E_ZELLIJ_REQUIRED:-}" == 1 ]]; then
    echo "e2e-zellij: zellij is required" >&2
    exit 1
  fi
  echo "e2e-zellij: skip (zellij not on PATH)"
  exit 0
fi

echo "e2e-zellij: $(zellij --version)"

cargo wasm

session="ov-e2e-$$"
# Keep TMPDIR short — Zellij IPC sockets have a ~103 byte path cap.
# Do not rewrite HOME: attach --create-background then follows the wrong session.
tmp="/tmp/$session"
mkdir -p "$tmp"
wasm="$root/target/wasm32-wasip1/release/zellij-overview.wasm"
cleanup() {
  zellij delete-session --force -- "$session" >/dev/null 2>&1 || true
  rm -rf "$tmp"
}
trap cleanup EXIT

if [[ ! -f "$wasm" ]]; then
  echo "e2e-zellij: missing $wasm" >&2
  exit 1
fi

# Isolated from the developer's config, runtime dir, and enclosing session.
unset ZELLIJ ZELLIJ_SESSION_NAME
export TMPDIR="$tmp"
export ZELLIJ_SOCKET_DIR="$tmp"
export TERM="${TERM:-xterm-256color}"

cat > "$tmp/config.kdl" <<'EOF'
keybinds clear-defaults=true {}
session_serialization false
show_startup_tips false
show_release_notes false
EOF

cat > "$tmp/layout.kdl" <<EOF
layout {
    pane command="sleep" {
        args "30"
    }
    floating_panes {
        pane {
            plugin location="file:$wasm"
        }
    }
}
EOF

# --layout on the outer CLI is ignored by attach --create-background on some
# hosts (CI 0.45.0 dumped the default tab-bar layout). options --default-layout
# is the documented way to start a background session with a layout.
zellij --config "$tmp/config.kdl" attach --create-background "$session" \
  options --default-layout "$tmp/layout.kdl"

ok=0
layout=""
for _ in $(seq 1 40); do
  if layout="$(zellij --session "$session" action dump-layout 2>/dev/null)" \
    && echo "$layout" | grep -q 'zellij-overview.wasm'; then
    ok=1
    break
  fi
  sleep 0.25
done

if [[ "$ok" -ne 1 ]]; then
  echo "e2e-zellij: plugin did not appear in dump-layout" >&2
  zellij list-sessions 2>&1 || true
  zellij --session "$session" action dump-layout 2>&1 || true
  zellij --session "$session" action list-panes --json --all 2>&1 || true
  exit 1
fi

if echo "$layout" | grep -Ei 'panic|plugin crashed'; then
  echo "e2e-zellij: plugin pane looks crashed" >&2
  echo "$layout" >&2
  exit 1
fi

echo "e2e-zellij: plugin loaded"
