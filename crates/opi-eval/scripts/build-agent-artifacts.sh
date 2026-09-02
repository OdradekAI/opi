#!/usr/bin/env bash
# opi-eval exact agent build/locator script.
#
# Builds and locates both evaluated agents with full executable identity,
# before any agent process starts:
#
# - Opi: `cargo build --locked --release -p opi-coding-agent --bin opi
#   --message-format=json-render-diagnostics`; the executable is selected
#   from the compiler-artifact whose target name is `opi` with a non-null
#   `executable` field, never an assumed target/release path. The identity
#   records readlink -f, SHA-256, file(1), ldd, rustc -Vv, cargo -V, the
#   checkout commit and dirty state, target triple, profile, feature set,
#   and the Cargo.lock SHA-256 recomputed from the checkout.
# - pi: locked source build (`npm ci --ignore-scripts`,
#   `npm run build:offline` over the checked-in model data),
#   bundle existence check). The identity records the canonical Node
#   executable path, SHA-256, file(1), ldd, --version, the npm identity,
#   the checkout commit and dirty state, the package manifest, package
#   lock, and shrinkwrap digests, the installed node_modules tree digest,
#   and the canonical bundle path and digest.
#
# Usage:
#   crates/opi-eval/scripts/build-agent-artifacts.sh --opi-source DIR --pi-source DIR --out DIR
#
# Emits <out>/opi-identity.json and <out>/pi-identity.json
# (schema `opi-eval-agent-identity/1`). Fails closed on any drift.

set -euo pipefail

OPI_SOURCE=""
PI_SOURCE=""
OUT=""
while [ $# -gt 0 ]; do
  case "$1" in
    --opi-source) OPI_SOURCE=$2; shift 2 ;;
    --pi-source) PI_SOURCE=$2; shift 2 ;;
    --out) OUT=$2; shift 2 ;;
    *) echo "opi-eval-build-agent-artifacts: unknown argument: $1" >&2; exit 2 ;;
  esac
done
if [ -z "$OPI_SOURCE" ] || [ -z "$PI_SOURCE" ] || [ -z "$OUT" ]; then
  echo "opi-eval-build-agent-artifacts: --opi-source, --pi-source, and --out are required" >&2
  exit 2
fi
OPI_SOURCE=$(readlink -f "$OPI_SOURCE")
PI_SOURCE=$(readlink -f "$PI_SOURCE")
mkdir -p "$OUT"

git_identity() {
  # commit, dirty flag, and dirty-file count of one checkout
  dir=$1
  commit=$(git -C "$dir" rev-parse HEAD)
  if [ -n "$(git -C "$dir" status --porcelain)" ]; then dirty=true; else dirty=false; fi
  dirty_files=$(git -C "$dir" status --porcelain | wc -l | tr -d ' ')
  # One space-separated line: `read -r a b c` consumes a single line,
  # so a multi-line layout would leave every later field empty.
  printf '%s %s %s\n' "$commit" "$dirty" "$dirty_files"
}

tree_digest() {
  # sorted content digest of one directory tree (files only, relative paths)
  dir=$1
  (cd "$dir" && find . -type f -print0 | LC_ALL=C sort -z \
    | xargs -0 sha256sum | sha256sum | cut -d' ' -f1)
}

# --- Opi -----------------------------------------------------------------
echo "opi-eval-build-agent-artifacts: building opi (locked release)" >&2
build_json=$(cargo build --locked --release \
  -p opi-coding-agent --bin opi \
  --message-format=json-render-diagnostics 2>/dev/null || \
  cargo build --locked --release \
    -p opi-coding-agent --bin opi \
    --message-format=json-render-diagnostics)
OPI_EXECUTABLE=$(printf '%s\n' "$build_json" | python3 -c '
import json, sys
for line in sys.stdin:
    line = line.strip()
    if not line.startswith("{"):
        continue
    message = json.loads(line)
    if message.get("reason") != "compiler-artifact":
        continue
    package = message.get("package_id", "")
    target = message.get("target", {})
    executable = message.get("executable")
    if target.get("name") == "opi" and executable:
        print(executable)
        break
else:
    sys.exit("no opi compiler-artifact executable was reported")
')
OPI_EXECUTABLE=$(readlink -f "$OPI_EXECUTABLE")
read -r OPI_COMMIT OPI_DIRTY OPI_DIRTY_FILES <<EOF
$(git_identity "$OPI_SOURCE")
EOF
CARGO_LOCK_SHA=$(sha256sum "$OPI_SOURCE/Cargo.lock" | cut -d' ' -f1)
{
  printf 'cargo build --locked --release -p opi-coding-agent --bin opi '
  printf -- '--message-format=json-render-diagnostics\n'
} > "$OUT/opi-build-command.txt"
python3 - "$OPI_EXECUTABLE" "$OPI_COMMIT" "$OPI_DIRTY" "$OPI_DIRTY_FILES" \
  "$CARGO_LOCK_SHA" "$OPI_SOURCE" "$OUT" <<'PYEOF'
import hashlib, json, subprocess, sys

executable, commit, dirty, dirty_files, lock_sha, source, out = sys.argv[1:8]
def sha(path):
    return hashlib.sha256(open(path, "rb").read()).hexdigest()
def run(cmd):
    p = subprocess.run(cmd, capture_output=True, text=True)
    return p.stdout if p.returncode == 0 else p.stdout + p.stderr
rustc_vv = run(["rustc", "-Vv"])
cargo_v = run(["cargo", "-V"])
identity = {
    "schema": "opi-eval-agent-identity/1",
    "agent": "opi",
    "canonical_executable": executable,
    "executable_sha256": sha(executable),
    "file": run(["file", "-b", executable]).strip(),
    "ldd": run(["ldd", executable]).strip(),
    "rustc_vv": rustc_vv.strip(),
    "cargo_v": cargo_v.strip(),
    "checkout_commit": commit,
    "checkout_dirty": dirty == "true",
    "checkout_dirty_files": int(dirty_files),
    "cargo_lock_sha256": lock_sha,
    "target": next(
        (line.split(": ", 1)[1].strip() for line in rustc_vv.splitlines()
         if line.startswith("host: ")), ""),
    "profile": "release",
    "features": "workspace-default-features",
}
with open(f"{out}/opi-identity.json", "w", encoding="utf-8") as f:
    json.dump(identity, f, indent=2, sort_keys=True)
    f.write("\n")
PYEOF

# --- pi ------------------------------------------------------------------
echo "opi-eval-build-agent-artifacts: building pi (locked source build)" >&2
NODE_EXECUTABLE=$(readlink -f "$(command -v node)")
NPM_EXECUTABLE=$(readlink -f "$(command -v npm)")
npm_cache="$OUT/npm-cache"
mkdir -p "$npm_cache"
(
  cd "$PI_SOURCE"
  HOME="$OUT/npm-home" npm_config_cache="$npm_cache" \
    npm ci --ignore-scripts
  # The model-data directory is generated, never checked in: hydrate it
  # from the same-version official registry tarball of @earendil-works/
  # pi-ai so the pinned source compiles exactly as released. The default
  # build regenerates the tables from live upstream catalogs, whose drift
  # breaks this pinned commit (TS2353 on cloudflare-ai-gateway).
  pi_ai_version=$(python3 -c 'import json; print(json.load(open("packages/ai/package.json"))["version"])')
  HOME="$OUT/npm-home" npm_config_cache="$npm_cache" \
    npm pack "@earendil-works/pi-ai@${pi_ai_version}" \
    --pack-destination "$npm_cache" >&2
  pi_ai_tarball=$(ls "$npm_cache"/earendil-works-pi-ai-"${pi_ai_version}".tgz)
  pi_ai_tarball_sha=$(sha256sum "$pi_ai_tarball" | cut -d' ' -f1)
  mkdir -p "$npm_cache/pi-ai-data"
  tar -xzf "$pi_ai_tarball" -C "$npm_cache/pi-ai-data" \
    package/dist/providers/data
  rm -rf packages/ai/src/providers/data
  cp -r "$npm_cache/pi-ai-data/package/dist/providers/data" \
    packages/ai/src/providers/data
  # build:offline type-checks against the hydrated pinned-release data.
  HOME="$OUT/npm-home" npm_config_cache="$npm_cache" \
    npm run build:offline
  printf '%s\n' "$pi_ai_tarball_sha" > "$OUT/pi-ai-data-tarball-sha256.txt"
  test -f packages/coding-agent/dist/bundle/cli.js
)
read -r PI_COMMIT PI_DIRTY PI_DIRTY_FILES <<EOF
$(git_identity "$PI_SOURCE")
EOF
BUNDLE_PATH="$PI_SOURCE/packages/coding-agent/dist/bundle/cli.js"
BUNDLE_PATH=$(readlink -f "$BUNDLE_PATH")
{
  printf 'npm ci --ignore-scripts && npm run build:offline && '
  printf 'test -f packages/coding-agent/dist/bundle/cli.js\n'
} > "$OUT/pi-build-command.txt"
python3 - "$NODE_EXECUTABLE" "$NPM_EXECUTABLE" "$PI_COMMIT" "$PI_DIRTY" \
  "$PI_DIRTY_FILES" "$PI_SOURCE" "$BUNDLE_PATH" "$npm_cache" "$OUT" <<'PYEOF'
import hashlib, json, os, subprocess, sys

node, npm, commit, dirty, dirty_files, source, bundle, cache, out = sys.argv[1:10]
def sha(path):
    return hashlib.sha256(open(path, "rb").read()).hexdigest()
def run(cmd):
    p = subprocess.run(cmd, capture_output=True, text=True)
    return p.stdout if p.returncode == 0 else p.stdout + p.stderr
def tree_digest(root):
    digest = hashlib.sha256()
    for base, _, files in sorted(os.walk(root)):
        for name in sorted(files):
            path = os.path.join(base, name)
            rel = os.path.relpath(path, root)
            digest.update(rel.encode("utf-8", "surrogateescape"))
            digest.update(hashlib.sha256(open(path, "rb").read()).digest())
    return digest.hexdigest()
identity = {
    "schema": "opi-eval-agent-identity/1",
    "agent": "pi",
    "node_executable": node,
    "node_sha256": sha(node),
    "node_file": run(["file", "-b", node]).strip(),
    "node_ldd": run(["ldd", node]).strip(),
    "node_version": run([node, "--version"]).strip(),
    "npm_executable": npm,
    "npm_version": run([npm, "--version"]).strip(),
    "checkout_commit": commit,
    "checkout_dirty": dirty == "true",
    "checkout_dirty_files": int(dirty_files),
    "package_json_sha256": sha(f"{source}/package.json"),
    "package_lock_sha256": sha(f"{source}/package-lock.json"),
    "shrinkwrap_sha256": sha(
        f"{source}/packages/coding-agent/npm-shrinkwrap.json"),
    "installed_tree_sha256": tree_digest(f"{source}/node_modules"),
    "npm_cache_archive_sha256": tree_digest(cache),
    "bundle_path": bundle,
    "bundle_sha256": sha(bundle),
}
with open(f"{out}/pi-identity.json", "w", encoding="utf-8") as f:
    json.dump(identity, f, indent=2, sort_keys=True)
    f.write("\n")
PYEOF

printf '%s\n' "$OPI_EXECUTABLE" > "$OUT/opi-executable-path.txt"
printf '%s\n' "$BUNDLE_PATH" > "$OUT/pi-bundle-path.txt"
printf '%s\n' "$NODE_EXECUTABLE" > "$OUT/node-executable-path.txt"
echo "opi-eval-build-agent-artifacts: identities written to $OUT" >&2
