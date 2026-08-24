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

tmp="$(mktemp -d)"
session="ov-e2e-$$"
wasm="$root/target/wasm32-wasip1/release/zellij-overview.wasm"
cleanup() {
  zellij delete-session --force -- "$session" >/dev/null 2>&1 || true
  rm -rf "$tmp"
}
trap cleanup EXIT

# Isolated from the developer's config and from an enclosing Zellij session.
unset ZELLIJ ZELLIJ_SESSION_NAME

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

zellij --config "$tmp/config.kdl" --layout "$tmp/layout.kdl" attach --create-background "$session"

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
  zellij --session "$session" action dump-layout 2>&1 || true
  exit 1
fi

if echo "$layout" | grep -Ei 'panic|plugin crashed'; then
  echo "e2e-zellij: plugin pane looks crashed" >&2
  echo "$layout" >&2
  exit 1
fi

echo "e2e-zellij: plugin loaded"
