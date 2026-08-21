#!/usr/bin/env bash
# Keep reprinting so content appears after a client attaches and the pane gets a size.
set -euo pipefail
file="${1:-}"
while true; do
  printf '\033[H\033[2J'
  if [[ -n "$file" && -f "$file" ]]; then
    cat "$file"
  elif [[ -n "$file" ]]; then
    echo "missing: $file"
  fi
  sleep 1
done
