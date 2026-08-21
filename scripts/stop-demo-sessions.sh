#!/usr/bin/env bash
# Kill only the recording sessions created by start-demo-sessions.sh.
set -euo pipefail

for name in dev ops; do
    if zellij list-sessions -n 2>/dev/null | awk '{print $1}' | grep -qx "$name"; then
        zellij kill-session "$name"
        echo "killed $name"
    else
        echo "not running: $name"
    fi
done
