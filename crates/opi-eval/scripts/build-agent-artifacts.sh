#!/usr/bin/env bash
# opi-eval exact agent build/locator script.
#
# Builds and locates the explicitly selected evaluated agents with full executable identity,
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
#   `npm run build:offline` over statically pinned model data),
#   bundle existence check). The identity records the canonical Node
#   executable path, SHA-256, file(1), ldd, --version, the npm identity,
#   the checkout commit and dirty state, the package manifest, package
#   lock, and shrinkwrap digests, the installed node_modules tree digest,
#   and the canonical bundle path and digest.
#
# Usage:
#   crates/opi-eval/scripts/build-agent-artifacts.sh --opi-source DIR --opi-commit COMMIT --pi-source DIR --static-lock PATH --out DIR
#
# Emits <out>/opi-identity.json and <out>/pi-identity.json
# (schema `opi-eval-agent-identity/1`). Fails closed on any drift.

set -euo pipefail

OPI_SOURCE=""
OPI_COMMIT=""
PI_SOURCE=""
STATIC_LOCK=""
OUT=""
while [ $# -gt 0 ]; do
  case "$1" in
    --opi-source) OPI_SOURCE=$2; shift 2 ;;
    --opi-commit) OPI_COMMIT=$2; shift 2 ;;
    --pi-source) PI_SOURCE=$2; shift 2 ;;
    --static-lock) STATIC_LOCK=$2; shift 2 ;;
    --out) OUT=$2; shift 2 ;;
    *) echo "opi-eval-build-agent-artifacts: unknown argument: $1" >&2; exit 2 ;;
  esac
done
if [ -z "$OUT" ] || { [ -z "$OPI_SOURCE" ] && [ -z "$PI_SOURCE" ]; }; then
  echo "opi-eval-build-agent-artifacts: --out and at least one selected source are required" >&2
  exit 2
fi
if { [ -n "$OPI_SOURCE" ] && [ -z "$OPI_COMMIT" ]; } || { [ -z "$OPI_SOURCE" ] && [ -n "$OPI_COMMIT" ]; }; then
  echo "opi-eval-build-agent-artifacts: --opi-source and --opi-commit must be paired" >&2
  exit 2
fi
if [ -n "$OPI_SOURCE" ]; then
  [[ "$OPI_COMMIT" =~ ^[0-9a-f]{40}$ ]] || { echo "invalid Opi commit" >&2; exit 2; }
  OPI_SOURCE=$(readlink -f "$OPI_SOURCE")
fi
if [ -n "$PI_SOURCE" ]; then
  PI_SOURCE=$(readlink -f "$PI_SOURCE")
  [ -n "$STATIC_LOCK" ] && [ -f "$STATIC_LOCK" ] || { echo "pi build data: --static-lock is required" >&2; exit 2; }
  STATIC_LOCK=$(readlink -f "$STATIC_LOCK")
fi
OUT=$(realpath -m "$OUT")
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
if [ -n "$OPI_SOURCE" ]; then
echo "opi-eval-build-agent-artifacts: building opi (locked release)" >&2
if [ -d "$OPI_SOURCE/.git" ]; then
  [ "$(git -C "$OPI_SOURCE" rev-parse HEAD)" = "$OPI_COMMIT" ] || { echo "Opi source commit drift" >&2; exit 2; }
fi
(
cd "$OPI_SOURCE"
build_json=$(cargo build --locked --release \
  -p opi-coding-agent --bin opi --message-format=json-render-diagnostics)
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
if [ -d "$OPI_SOURCE/.git" ]; then
  read -r observed_commit OPI_DIRTY OPI_DIRTY_FILES <<EOF
$(git_identity "$OPI_SOURCE")
EOF
  [ "$observed_commit" = "$OPI_COMMIT" ] || exit 2
else
  # The preparation owner supplies a verified git archive of this exact commit.
  OPI_DIRTY=false
  OPI_DIRTY_FILES=0
fi
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

printf '%s\n' "$OPI_EXECUTABLE" > "$OUT/opi-executable-path.txt"
)
fi

# --- pi ------------------------------------------------------------------
if [ -n "$PI_SOURCE" ]; then
echo "opi-eval-build-agent-artifacts: building pi (locked source build)" >&2
NODE_EXECUTABLE=$(readlink -f "$(command -v node)")
NPM_EXECUTABLE=$(readlink -f "$(command -v npm)")
npm_cache="$OUT/npm-cache"
mkdir -p "$npm_cache"
(
  cd "$PI_SOURCE"
  # Generated data comes only from the reviewed static archive pin, never
  # live registry resolution or the default catalog-generating build.
  pi_ai_tarball="$npm_cache/pi-ai-data.tgz"
  python3 - "$STATIC_LOCK" "$pi_ai_tarball" <<'DATA_PIN'
import hashlib, json, pathlib, re, sys, urllib.parse, urllib.request
lock_path, archive = sys.argv[1:]
def unique_fields(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise SystemExit("pi build data: duplicate static lock field")
        result[key] = value
    return result
lock = json.loads(pathlib.Path(lock_path).read_text(), object_pairs_hook=unique_fields)
subjects = [row for row in lock.get("subjects", []) if row.get("id") == "pi"]
if len(subjects) != 1 or not isinstance(subjects[0].get("model_data_archive"), dict):
    raise SystemExit("pi build data: exactly one static archive pin is required")
pin = subjects[0]["model_data_archive"]
version = json.loads(pathlib.Path("packages/ai/package.json").read_text())["version"]
if pin.get("package") != "@earendil-works/pi-ai" or pin.get("version") != version:
    raise SystemExit("pi build data: package or source version mismatch")
url = pin.get("url")
digest = pin.get("sha256")
if not isinstance(digest, str) or not re.fullmatch(r"[0-9a-f]{64}", digest):
    raise SystemExit("pi build data: invalid static archive SHA-256")
if not isinstance(url, str) or not url or any(char.isspace() for char in url):
    raise SystemExit("pi build data: invalid static archive URL")
parsed = urllib.parse.urlsplit(url)
if not ((parsed.scheme == "https" and parsed.netloc) or
        (parsed.scheme == "file" and not parsed.netloc and parsed.path.startswith("/"))):
    raise SystemExit("pi build data: invalid static archive URL")
urllib.request.urlretrieve(url, archive)
if hashlib.sha256(pathlib.Path(archive).read_bytes()).hexdigest() != digest:
    raise SystemExit("pi build data: archive digest drift")
DATA_PIN
  pi_ai_tarball_sha=$(sha256sum "$pi_ai_tarball" | cut -d' ' -f1)
  HOME="$OUT/npm-home" npm_config_cache="$npm_cache" \
    npm ci --ignore-scripts
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

printf '%s\n' "$BUNDLE_PATH" > "$OUT/pi-bundle-path.txt"
printf '%s\n' "$NODE_EXECUTABLE" > "$OUT/node-executable-path.txt"
fi
echo "opi-eval-build-agent-artifacts: identities written to $OUT" >&2
