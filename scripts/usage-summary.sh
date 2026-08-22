#!/usr/bin/env bash
# Summarize local overview usage logs from Zellij plugin cache.
# No titles are stored. Nothing is uploaded.
set -euo pipefail

cache_roots=()
if [[ -n "${XDG_CACHE_HOME:-}" ]]; then
  cache_roots+=("$XDG_CACHE_HOME/zellij")
fi
cache_roots+=("$HOME/.cache/zellij")
cache_roots+=("$HOME/Library/Caches/org.Zellij-Contributors.Zellij")

logs=()
found_roots=()
for root in "${cache_roots[@]}"; do
  [[ -d "$root" ]] || continue
  found_roots+=("$root")
  while IFS= read -r path; do
    logs+=("$path")
  done < <(find "$root" -name usage.jsonl -print 2>/dev/null | sort)
done

if [[ ${#found_roots[@]} -eq 0 ]]; then
  echo "no Zellij cache in:" >&2
  printf '  %s\n' "${cache_roots[@]}" >&2
  echo "macOS uses ~/Library/Caches/org.Zellij-Contributors.Zellij, not ~/.cache/zellij." >&2
  exit 1
fi

if [[ ${#logs[@]} -eq 0 ]]; then
  echo "found Zellij cache, but no usage.jsonl yet. Open overview, jump or close once, then retry." >&2
  printf '  %s\n' "${found_roots[@]}" >&2
  exit 1
fi

python3 - "${logs[@]}" <<'PY'
import json, sys
from collections import Counter

paths = sys.argv[1:]
rows = []
for path in paths:
    with open(path, encoding="utf-8") as fh:
        for raw in fh:
            raw = raw.strip()
            if not raw:
                continue
            try:
                row = json.loads(raw)
            except json.JSONDecodeError:
                continue
            if any(key in row for key in ("name", "title", "query")):
                continue
            rows.append(row)

print(f"files  {len(paths)}")
for path in paths:
    print(f"  {path}")

used = [row for row in rows if int(row.get("keys") or 0) > 0]
empty_toggle = sum(
    1
    for row in rows
    if int(row.get("keys") or 0) == 0 and row.get("end") == "toggle"
)
print(f"opens  {len(used)} used   {empty_toggle} empty Ctrl+y")
if not used:
    raise SystemExit(0)

keys = sum(int(row.get("keys") or 0) for row in used)
print(f"keys   {keys}  avg {keys / len(used):.1f}")


def path(row):
    steps = []
    if row.get("flash"):
        steps.append("flash")
    if row.get("hjkl"):
        steps.append("hjkl")
    if row.get("dash"):
        steps.append("-")
    if row.get("drill"):
        steps.append("drill")
    end = {
        "switch": "other session",
        "tab": "this tab",
        "prev": "previous tab",
        "dismiss": "q/esc",
        "toggle": "ctrl+y",
    }.get(str(row.get("end") or "?"), str(row.get("end")))
    steps.append(end)
    return " → ".join(steps)


print("path")
paths_count = Counter(path(row) for row in used)
width = max(len(name) for name in paths_count)
for name, n in paths_count.most_common():
    print(f"  {name:<{width}}  {n}  {100 * n / len(used):.0f}%")
PY
