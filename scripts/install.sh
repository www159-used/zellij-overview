#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
dest="${OVERVIEW_PLUGIN_PATH:-$HOME/.config/zellij/plugins/overview.wasm}"

cd "$root"
cargo wasm
mkdir -p "$(dirname "$dest")"
cp "$root/target/wasm32-wasip1/release/overview.wasm" "$dest"
echo "installed $dest"
