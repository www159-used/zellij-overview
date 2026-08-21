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

python3 - "${found_roots[0]}" "${logs[@]}" <<'PY'
import json, sys
from collections import Counter

root, *paths = sys.argv[1:]
opens = 0
keys = 0
flags = Counter()
ends = Counter()
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
            opens += 1
            keys += int(row.get("keys") or 0)
            for flag in ("flash", "hjkl", "dash", "drill", "cross"):
                if row.get(flag):
                    flags[flag] += 1
            ends[str(row.get("end") or "?")] += 1

print(f"files  {len(paths)}")
for path in paths:
    print(f"  {path}")
print(f"opens  {opens}")
if opens == 0:
    raise SystemExit(0)
print(f"keys   {keys}  avg {keys / opens:.1f}")
for flag in ("flash", "hjkl", "dash", "drill", "cross"):
    n = flags[flag]
    print(f"{flag:6} {n}  {100 * n / opens:.0f}%")
print("end")
for name, n in ends.most_common():
    print(f"  {name:8} {n}  {100 * n / opens:.0f}%")
PY
