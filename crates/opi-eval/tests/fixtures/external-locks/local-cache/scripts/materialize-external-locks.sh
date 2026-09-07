#!/bin/sh
# Stand-in for the pinned local materializer (test fixture).
# Prepare invokes it only in supervised acquisition mode:
#   materialize-external-locks.sh prepare --cache <dir> --plan <missing.json>
# The plan lists entries {"id": "<cache-relative path>"}; the stand-in
# writes one deterministic byte per missing entry and exits 0.
[ "$1" = "prepare" ] || { echo "unsupported mode: $1" >&2; exit 2; }
shift
cache=""
plan=""
while [ $# -gt 0 ]; do
  case "$1" in
    --cache) cache="$2"; shift 2 ;;
    --plan) plan="$2"; shift 2 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done
[ -n "$cache" ] && [ -n "$plan" ] || { echo "prepare mode requires --cache and --plan" >&2; exit 2; }
python3 - "$cache" "$plan" <<'PY'
import json, sys
cache, plan = sys.argv[1], sys.argv[2]
for entry in json.load(open(plan)):
    path = f"{cache}/{entry["id"]}"
    import os
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "wb") as handle:
        handle.write(f"acquired stand-in bytes for {entry["id"]}\n".encode())
PY
exit 0
