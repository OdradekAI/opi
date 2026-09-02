#!/usr/bin/env bash
# Phase 18 Linux x86_64 external-lock materializer (producer).
#
# Executes the committed static external lock at
# crates/opi-eval/external-locks/static/linux-x86_64.json on the manually
# authorized Linux runner: verifies every pinned upstream identity, pulls the
# pinned task image by digest, materializes and seals the Terminal-Bench 2.1
# verifier dependency closure, runs one official upstream oracle preflight
# through the pinned Harbor revision under the task's own upstream network
# configuration, and writes a receipt that task 18.3 verifies and admits as
# the resolved Linux lock. Offline verifier enforcement is not claimed by
# this run; the receipt records the actual effective network mode.
#
# The script is data-driven: every upstream URL, digest, Git identity, and
# environment control is read from the static lock; nothing is resolved
# dynamically and no mutable tag is ever pulled. Every stage fails closed on
# the first unverifiable byte. Repo-tracked text pins (workflow, producer
# scripts, lock documents) hash over LF-normalized bytes; downloaded upstream
# artifacts hash over their exact bytes.

set -euo pipefail

usage() {
  cat <<'USAGE'
usage: phase18-materialize-locks.sh --candidate-commit <40-hex>
                                     --static-lock <path>
                                     --output-dir <path>
USAGE
}

CANDIDATE_COMMIT=""
STATIC_LOCK_PATH=""
OUTPUT_DIR=""

while [ "$#" -gt 0 ]; do
  case "$1" in
    --candidate-commit) CANDIDATE_COMMIT="$2"; shift 2 ;;
    --static-lock) STATIC_LOCK_PATH="$2"; shift 2 ;;
    --output-dir) OUTPUT_DIR="$2"; shift 2 ;;
    *) usage >&2; exit 2 ;;
  esac
done

if [ -z "$CANDIDATE_COMMIT" ] || [ -z "$STATIC_LOCK_PATH" ] || [ -z "$OUTPUT_DIR" ]; then
  usage >&2
  exit 2
fi

fail() {
  echo "phase18-materialize: FAIL: $*" >&2
  exit 1
}

info() {
  echo "phase18-materialize: $*"
}

now_utc() {
  date -u +%Y-%m-%dT%H:%M:%SZ
}

expiry_utc() {
  date -u -d "+30 days" +%Y-%m-%dT%H:%M:%SZ
}

sha256_bytes() {
  sha256sum "$1" | awk '{print $1}'
}

lf_sha256_bytes() {
  python3 - "$1" <<'PY'
import hashlib, sys
with open(sys.argv[1], "rb") as handle:
    print(hashlib.sha256(handle.read().replace(b"\r\n", b"\n")).hexdigest())
PY
}

require_tool() {
  command -v "$1" >/dev/null 2>&1 || fail "required tool not found: $1"
}

lock_field() {
  python3 - "$STATIC_LOCK_PATH" "$1" <<'PY'
import json, sys
with open(sys.argv[1], encoding="utf-8") as handle:
    lock = json.load(handle)
value = lock
for segment in sys.argv[2].split("."):
    if isinstance(value, list):
        value = value[int(segment)]
    else:
        value = value[segment]
if isinstance(value, (dict, list)):
    print(json.dumps(value, sort_keys=True))
else:
    print(value)
PY
}

# ---------------------------------------------------------------------------
# Stage 0: environment and tool identities.
# ---------------------------------------------------------------------------
require_tool git
require_tool python3
require_tool curl
require_tool docker

UNAME="$(uname -srm)"
[ "$(uname -s)" = "Linux" ] || fail "materialization requires Linux, found $(uname -s)"
[ "$(uname -m)" = "x86_64" ] || fail "materialization requires x86_64, found $(uname -m)"

info "stage 0: host and tool identities"
TOOLCHAIN_JSON="$(mktemp)"
trap 'rm -f "$TOOLCHAIN_JSON"' EXIT
{
  printf '{\n'
  printf '  "uname": %s,\n' "$(python3 -c 'import json,sys; print(json.dumps(sys.argv[1]))' "$UNAME")"
  printf '  "image_os": %s,\n' "$(python3 -c 'import json,os; print(json.dumps(os.environ.get("ImageOS","")))' 2>/dev/null || echo '""')"
  printf '  "image_version": %s,\n' "$(python3 -c 'import json,os; print(json.dumps(os.environ.get("ImageVersion","")))' 2>/dev/null || echo '""')"
  printf '  "bash": %s,\n' "$(python3 -c 'import json,subprocess; print(json.dumps(subprocess.run(["bash","--version"],capture_output=True,text=True).stdout.splitlines()[0]))')"
  printf '  "git": %s,\n' "$(python3 -c 'import json,subprocess; print(json.dumps(subprocess.run(["git","--version"],capture_output=True,text=True).stdout.strip()))')"
  printf '  "python3": %s,\n' "$(python3 -c 'import json,sys; print(json.dumps(sys.version.split()[0]))')"
  printf '  "docker_client": %s,\n' "$(docker version --format '{{.Client.Version}}' 2>/dev/null || echo unknown)"
  printf '  "docker_server": %s\n' "$(docker version --format '{{.Server.Version}}' 2>/dev/null || echo unknown)"
  printf '}\n'
} > "$TOOLCHAIN_JSON"

# ---------------------------------------------------------------------------
# Stage 1: checkout binding and static-lock self-verification.
# ---------------------------------------------------------------------------
info "stage 1: candidate-commit binding"
HEAD_COMMIT="$(git rev-parse HEAD)"
[ "$HEAD_COMMIT" = "$CANDIDATE_COMMIT" ] ||
  fail "HEAD $HEAD_COMMIT is not the candidate commit $CANDIDATE_COMMIT"

case "$CANDIDATE_COMMIT" in
  [0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f]) ;;
  *) fail "candidate commit is not 40 hex characters: $CANDIDATE_COMMIT" ;;
esac

[ -f "$STATIC_LOCK_PATH" ] || fail "static lock not found: $STATIC_LOCK_PATH"
STATIC_LOCK_SHA="$(lf_sha256_bytes "$STATIC_LOCK_PATH")"
LOCK_SCHEMA="$(lock_field schema)"
LOCK_ID="$(lock_field lock_id)"
[ "$LOCK_SCHEMA" = "phase18-external-lock/static/1" ] || fail "unsupported static lock schema: $LOCK_SCHEMA"
[ "$LOCK_ID" = "phase18-linux-x86_64" ] || fail "unexpected lock id: $LOCK_ID"

WORKFLOW_PATH="$(lock_field authority.workflow.path)"
WORKFLOW_PIN_SHA="$(lock_field authority.workflow.sha256)"
WORKFLOW_OBSERVED_SHA="$(lf_sha256_bytes "$WORKFLOW_PATH")"
[ "$WORKFLOW_OBSERVED_SHA" = "$WORKFLOW_PIN_SHA" ] ||
  fail "workflow bytes drifted: $WORKFLOW_OBSERVED_SHA != pinned $WORKFLOW_PIN_SHA"

for producer in $(lock_field authority.producers | python3 -c 'import json,sys; [print(p["path"]) for p in json.load(sys.stdin)]'); do
  PINNED_SHA="$(lock_field "authority.producers" | python3 -c 'import json,sys; print(next(p["sha256"] for p in json.load(sys.stdin) if p["path"] == sys.argv[1]))' "$producer")"
  OBSERVED_SHA="$(lf_sha256_bytes "$producer")"
  [ "$OBSERVED_SHA" = "$PINNED_SHA" ] ||
    fail "producer $producer drifted: $OBSERVED_SHA != pinned $PINNED_SHA"
done
info "stage 1: workflow and producer pins verified against the checkout"

WORKFLOW_REF="${GITHUB_WORKFLOW_REF:-unknown}"
WORKFLOW_SHA="${GITHUB_WORKFLOW_SHA:-$CANDIDATE_COMMIT}"
RUN_ID="${GITHUB_RUN_ID:-0}"
RUN_ATTEMPT="${GITHUB_RUN_ATTEMPT:-1}"

# ---------------------------------------------------------------------------
# Stage 2: fetch the Terminal-Bench 2.1 source and verify the task package.
# ---------------------------------------------------------------------------
info "stage 2: benchmark source identities"
mkdir -p "$OUTPUT_DIR"
WORK_ROOT="$(mktemp -d "$OUTPUT_DIR/work.XXXXXX")"
TB21_COMMIT="$(lock_field subjects.1.commit)"
TB21_URL="$(lock_field subjects.1.repository_url)"
TB21_TASKS_TREE="$(lock_field subjects.1.tasks_tree)"
TB21_TASK_ID="$(lock_field subjects.1.task.id)"
TB21_TASK_TREE="$(lock_field subjects.1.task.tree)"

git clone --quiet --no-checkout "$TB21_URL" "$WORK_ROOT/terminal-bench-2-1"
git -C "$WORK_ROOT/terminal-bench-2-1" fetch --quiet origin "$TB21_COMMIT" --depth 1
git -C "$WORK_ROOT/terminal-bench-2-1" checkout --quiet --detach "$TB21_COMMIT"
[ "$(git -C "$WORK_ROOT/terminal-bench-2-1" rev-parse HEAD)" = "$TB21_COMMIT" ] ||
  fail "Terminal-Bench 2.1 checkout is not $TB21_COMMIT"
[ "$(git -C "$WORK_ROOT/terminal-bench-2-1" rev-parse "HEAD:tasks")" = "$TB21_TASKS_TREE" ] ||
  fail "Terminal-Bench 2.1 tasks tree drifted from $TB21_TASKS_TREE"
[ "$(git -C "$WORK_ROOT/terminal-bench-2-1" rev-parse "HEAD:tasks/$TB21_TASK_ID")" = "$TB21_TASK_TREE" ] ||
  fail "task tree for $TB21_TASK_ID drifted from $TB21_TASK_TREE"

python3 - "$STATIC_LOCK_PATH" "$WORK_ROOT/terminal-bench-2-1" "$TB21_TASK_ID" <<'PY' || fail "task package files do not match the static lock"
import hashlib, json, subprocess, sys

lock_path, repo, task_id = sys.argv[1], sys.argv[2], sys.argv[3]
with open(lock_path, encoding="utf-8") as handle:
    subject = json.load(handle)["subjects"][1]
files = subject["task"]["files"]
assert subject["task"]["id"] == task_id
for pinned in files:
    blob = subprocess.run(
        ["git", "-C", repo, "rev-parse", f"HEAD:tasks/{task_id}/{pinned['path']}"],
        capture_output=True, text=True, check=True,
    ).stdout.strip()
    if blob != pinned["git_blob"]:
        raise SystemExit(f"blob mismatch for {pinned['path']}: {blob}")
    content = subprocess.run(
        ["git", "-C", repo, "cat-file", "blob", blob],
        capture_output=True, check=True,
    ).stdout
    digest = hashlib.sha256(content).hexdigest()
    if digest != pinned["sha256"]:
        raise SystemExit(f"content digest mismatch for {pinned['path']}")
    if len(content) != pinned["size"]:
        raise SystemExit(f"size mismatch for {pinned['path']}")
print(f"task package verified: {len(files)} files")
PY

# ---------------------------------------------------------------------------
# Stage 3: materialize the pinned apt closure.
# ---------------------------------------------------------------------------
info "stage 3: apt closure"
SNAPSHOT_BASE="https://snapshot.debian.org"
CLOSURE_DIR="$OUTPUT_DIR/closure"
mkdir -p "$CLOSURE_DIR/apt/indexes" "$CLOSURE_DIR/apt/pool" "$CLOSURE_DIR/uv" "$CLOSURE_DIR/wheels"

python3 - "$STATIC_LOCK_PATH" "$SNAPSHOT_BASE" "$CLOSURE_DIR" <<'PY' || fail "apt closure did not verify against the static lock"
import hashlib, json, sys, urllib.request

lock_path, snapshot_base, closure_dir = sys.argv[1], sys.argv[2], sys.argv[3]
with open(lock_path, encoding="utf-8") as handle:
    closure = json.load(handle)["closures"][0]
epoch = closure["apt_epoch"]
archives = {"debian": "archive/debian", "debian-security": "archive/debian-security"}

def fetch(url: str, target: str, expected: str) -> None:
    urllib.request.urlretrieve(url, target)  # noqa: S310 - pinned https snapshot URLs
    with open(target, "rb") as handle:
        actual = hashlib.sha256(handle.read()).hexdigest()
    if actual != expected:
        raise SystemExit(f"digest mismatch for {url}: {actual} != {expected}")

for index in closure["indexes"]:
    path = f"{archives[index['archive']]}/{epoch}/{index['path']}"
    fetch(
        f"{snapshot_base}/{path}",
        f"{closure_dir}/apt/indexes/{epoch}_{index['suite']}_{index['path'].split('/')[-1]}",
        index["sha256"],
    )
for package in closure["packages"]:
    fetch(
        f"{snapshot_base}/archive/debian/{epoch}/{package['path']}",
        f"{closure_dir}/apt/pool/{package['path'].split('/')[-1]}",
        package["sha256"],
    )
print(f"apt closure verified: {len(closure['indexes'])} indexes, {len(closure['packages'])} packages")
PY

# ---------------------------------------------------------------------------
# Stage 4: materialize the pinned uv and Python wheel closure.
# ---------------------------------------------------------------------------
info "stage 4: uv and wheel closure"
python3 - "$STATIC_LOCK_PATH" "$CLOSURE_DIR" <<'PY' || fail "uv/wheel closure did not verify against the static lock"
import hashlib, json, sys, urllib.request

lock_path, closure_dir = sys.argv[1], sys.argv[2]
with open(lock_path, encoding="utf-8") as handle:
    closure = json.load(handle)["closures"][0]
uv = closure["uv"]

def fetch(url: str, target: str, expected: str) -> None:
    urllib.request.urlretrieve(url, target)  # noqa: S310 - pinned first-party URLs
    with open(target, "rb") as handle:
        actual = hashlib.sha256(handle.read()).hexdigest()
    if actual != expected:
        raise SystemExit(f"digest mismatch for {url}: {actual} != {expected}")

fetch(uv["installer"]["url"], f"{closure_dir}/uv/uv-installer.sh", uv["installer"]["sha256"])
fetch(uv["archive"]["url"], f"{closure_dir}/uv/uv-x86_64-unknown-linux-gnu.tar.gz", uv["archive"]["sha256"])
with open(f"{closure_dir}/uv/uv-installer.sha256", "w", encoding="utf-8") as handle:
    handle.write(f"{uv['uv_sha256']}  uv\n{uv['uvx_sha256']}  uvx\n")
for wheel in closure["wheels"]:
    fetch(wheel["url"], f"{closure_dir}/wheels/{wheel['url'].split('/')[-1]}", wheel["sha256"])
print(f"uv closure verified: installer, archive, {len(closure['wheels'])} wheels")
PY

# ---------------------------------------------------------------------------
# Stage 5: pull the pinned task image by digest and record its OCI graph.
# ---------------------------------------------------------------------------
info "stage 5: task image pull"
TB21_IMAGE_REF="$(lock_field images.0.reference)"
TB21_IMAGE_MANIFEST="$(lock_field images.0.manifest)"
docker pull --quiet "$TB21_IMAGE_REF@$TB21_IMAGE_MANIFEST" ||
  fail "cannot pull $TB21_IMAGE_REF by digest $TB21_IMAGE_MANIFEST"
docker buildx imagetools inspect "$TB21_IMAGE_REF@$TB21_IMAGE_MANIFEST" --raw > "$OUTPUT_DIR/task-image-manifest.json" ||
  fail "cannot inspect the pulled task image manifest"
# A digest pull leaves no local tag; bind the task tag to the pulled digest so
# the Harbor environment uses exactly the pinned image bytes instead of
# re-resolving the mutable tag.
docker tag "$TB21_IMAGE_REF@$TB21_IMAGE_MANIFEST" "$TB21_IMAGE_REF" ||
  fail "cannot bind the local task tag to the pulled digest"
IMAGES_JSON="$OUTPUT_DIR/pulled-images.json"
python3 - "$STATIC_LOCK_PATH" "$OUTPUT_DIR/task-image-manifest.json" > "$IMAGES_JSON" <<'PY' || fail "pulled image graph does not match the static lock"
import hashlib, json, sys

lock_path, manifest_path = sys.argv[1], sys.argv[2]
with open(lock_path, encoding="utf-8") as handle:
    pinned = json.load(handle)["images"][0]
with open(manifest_path, "rb") as handle:
    raw = handle.read()
actual_manifest = "sha256:" + hashlib.sha256(raw).hexdigest()
if actual_manifest != pinned["manifest"]:
    raise SystemExit(f"pulled manifest {actual_manifest} != pinned {pinned['manifest']}")
manifest = json.loads(raw)
config = manifest.get("config", {}).get("digest", "")
layers = [layer.get("digest", "") for layer in manifest.get("layers", [])]
if pinned.get("config") and config != pinned["config"]:
    raise SystemExit(f"pulled config {config} != pinned {pinned['config']}")
if pinned.get("layers") and layers != pinned["layers"]:
    raise SystemExit(f"pulled layers {layers} != pinned {pinned['layers']}")
print(json.dumps({
    "id": pinned["id"],
    "reference": pinned["reference"],
    "manifest": actual_manifest,
    "config": config,
    "layers": layers,
}, sort_keys=True))
PY

# ---------------------------------------------------------------------------
# Stage 6: runner sources and the pinned uv executable.
# ---------------------------------------------------------------------------
info "stage 6: runner toolchain"
HARBOR_COMMIT="$(lock_field tools.2.commit)"
HARBOR_URL="https://github.com/harbor-framework/harbor"
PIER_COMMIT="$(lock_field tools.3.commit)"
PIER_URL="https://github.com/datacurve-ai/pier"

for pin in 2 3; do
  URL="$([ "$pin" = 2 ] && echo "$HARBOR_URL" || echo "$PIER_URL")"
  COMMIT="$([ "$pin" = 2 ] && echo "$HARBOR_COMMIT" || echo "$PIER_COMMIT")"
  NAME="$([ "$pin" = 2 ] && echo harbor || echo pier)"
  git clone --quiet "$URL" "$WORK_ROOT/$NAME"
  git -C "$WORK_ROOT/$NAME" checkout --quiet --detach "$COMMIT"
  [ "$(git -C "$WORK_ROOT/$NAME" rev-parse HEAD)" = "$COMMIT" ] ||
    fail "$NAME checkout is not $COMMIT"
  python3 - "$STATIC_LOCK_PATH" "$pin" "$WORK_ROOT/$NAME" <<'PY' || fail "$NAME uv.lock does not match the static lock"
import hashlib, json, subprocess, sys

lock_path, pin, repo = sys.argv[1], int(sys.argv[2]), sys.argv[3]
with open(lock_path, encoding="utf-8") as handle:
    tool = json.load(handle)["tools"][pin]
blob = subprocess.run(
    ["git", "-C", repo, "rev-parse", "HEAD:uv.lock"],
    capture_output=True, text=True, check=True,
).stdout.strip()
if blob != tool["uv_lock"]["git_blob"]:
    raise SystemExit(f"{tool['id']} uv.lock blob {blob} != pinned")
content = subprocess.run(
    ["git", "-C", repo, "cat-file", "blob", blob],
    capture_output=True, check=True,
).stdout
digest = hashlib.sha256(content).hexdigest()
if digest != tool["uv_lock"]["sha256"]:
    raise SystemExit(f"{tool['id']} uv.lock digest {digest} != pinned")
PY
done

UV_RUNNER_URL="$(lock_field tools.1.archive.url)"
UV_RUNNER_SHA="$(lock_field tools.1.archive.sha256)"
curl --fail --silent --show-error --location "$UV_RUNNER_URL" -o "$WORK_ROOT/uv-runner.tar.gz"
[ "$(sha256_bytes "$WORK_ROOT/uv-runner.tar.gz")" = "$UV_RUNNER_SHA" ] ||
  fail "runner uv archive digest mismatch"
mkdir -p "$WORK_ROOT/uv-runner"
tar -xzf "$WORK_ROOT/uv-runner.tar.gz" -C "$WORK_ROOT/uv-runner"
UV_BIN="$(find "$WORK_ROOT/uv-runner" -maxdepth 2 -name uv -type f | head -n 1)"
[ -n "$UV_BIN" ] || fail "runner uv binary not found in archive"
UV_BIN_SHA="$(sha256_bytes "$UV_BIN")"
"$UV_BIN" --version >/dev/null || fail "runner uv binary does not execute"

# ---------------------------------------------------------------------------
# Stage 7: seal the closure directory with a canonical manifest.
# ---------------------------------------------------------------------------
info "stage 7: closure sealing"
mkdir -p "$CLOSURE_DIR/opt"
cat > "$CLOSURE_DIR/opt/curl-shim.sh" <<'SHIM'
#!/usr/bin/env bash
# Adapter policy shim: serve only the pinned uv installer for the exact
# upstream argument vector; delegate everything else to the image curl.
set -euo pipefail
if [ "${1:-}" = "-LsSf" ] && [ "${2:-}" = "https://astral.sh/uv/0.9.5/install.sh" ]; then
  cat /opt/uv/uv-installer.sh
  exit 0
fi
exec /usr/bin/curl "$@"
SHIM
chmod 0755 "$CLOSURE_DIR/opt/curl-shim.sh"

cat > "$CLOSURE_DIR/opt/apt-closure.conf" <<'APT'
# Adapter policy: direct apt at the sealed snapshot closure only.
Acquire::Check-Valid-Until "false";
APT::Get::AllowUnauthenticated "false";
APT::Get::Download-Only "false";
APT::Get::Fix-Broken "false";
Dir::Etc::sourcelist "/opt/apt/sources.list";
Dir::State::lists "/opt/apt/lists";
APT_CONF_CLOSED=1
APT

python3 - "$CLOSURE_DIR" > "$OUTPUT_DIR/closure-manifest.json" <<'PY'
import hashlib, json, os, sys

closure_dir = sys.argv[1]
records = []
for root, _, names in os.walk(closure_dir):
    for name in names:
        path = os.path.join(root, name)
        rel = os.path.relpath(path, closure_dir)
        role = (
            "apt-index" if rel.startswith("apt/indexes/") else
            "apt-package" if rel.startswith("apt/pool/") else
            "uv-asset" if rel.startswith("uv/") else
            "wheel" if rel.startswith("wheels/") else
            "adapter-policy"
        )
        with open(path, "rb") as handle:
            data = handle.read()
        records.append({
            "path": rel,
            "role": role,
            "size": len(data),
            "sha256": hashlib.sha256(data).hexdigest(),
        })
records.sort(key=lambda record: record["path"])
manifest = hashlib.sha256(
    "".join(f"{r['path']}\t{r['role']}\t{r['size']}\t{r['sha256']}\n" for r in records).encode()
).hexdigest()
print(json.dumps({"manifest_sha256": manifest, "files": records}, sort_keys=True, indent=2))
PY

CLOSURE_MANIFEST_SHA="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["manifest_sha256"])' "$OUTPUT_DIR/closure-manifest.json")"
CLOSURE_FILE_COUNT="$(python3 -c 'import json,sys; print(len(json.load(open(sys.argv[1]))["files"]))' "$OUTPUT_DIR/closure-manifest.json")"
info "closure sealed: $CLOSURE_FILE_COUNT files, manifest $CLOSURE_MANIFEST_SHA"

# ---------------------------------------------------------------------------
# Stage 8: upstream oracle preflight under the pinned upstream configuration.
# ---------------------------------------------------------------------------
info "stage 8: oracle preflight"
ORACLE_DIR="$OUTPUT_DIR/oracle"
mkdir -p "$ORACLE_DIR"
cat > "$WORK_ROOT/oracle-driver.py" <<'DRIVER'
"""Upstream oracle preflight driver (harbor v0.22.0 API).

Runs the official Terminal-Bench 2.1 solution for the pinned task through the
pinned Harbor revision and requires the native verifier to report a passing
reward. The trial uses the task package's own upstream configuration
verbatim: its declared docker_image, its declared network baseline, and the
upstream oracle agent. The sealed closure stays a pinned provenance artifact
and is not mounted into this run; offline verifier enforcement is deferred
to the native-smoke profile per the phase18 option-A amendment.
"""

import asyncio
import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(sys.argv[2]).resolve()))

from harbor.models.trial.config import (  # noqa: E402
    AgentConfig,
    TaskConfig,
    TrialConfig,
)
from harbor.trial.trial import Trial  # noqa: E402


async def main() -> int:
    task_dir = pathlib.Path(sys.argv[1]).resolve()
    work_dir = pathlib.Path(sys.argv[3]).resolve()
    config = TrialConfig(
        task=TaskConfig(path=task_dir),
        agent=AgentConfig(name="oracle"),
        trials_dir=work_dir,
        trial_name="phase18-oracle-preflight",
    )
    trial = await Trial.create(config)
    result = await trial.run()
    verifier = result.verifier_result
    rewards = dict(verifier.rewards) if verifier is not None and verifier.rewards else {}
    if not rewards or any(float(value) <= 0.0 for value in rewards.values()):
        print(f"oracle preflight failed: rewards={rewards}", file=sys.stderr)
        return 1
    report = trial.paths.result_path.read_text(encoding="utf-8")
    (work_dir / "harbor-results.json").write_text(report, encoding="utf-8")
    print(json.dumps({"rewards": rewards}, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(asyncio.run(main()))
DRIVER

if ! "$UV_BIN" run --locked --project "$WORK_ROOT/harbor" python "$WORK_ROOT/oracle-driver.py" \
    "$WORK_ROOT/terminal-bench-2-1/tasks/$TB21_TASK_ID" \
    "$WORK_ROOT/harbor" \
    "$ORACLE_DIR" > "$ORACLE_DIR/oracle-summary.json"; then
  fail "upstream oracle preflight did not pass"
fi

CTRF_PATH="$(find "$ORACLE_DIR" \( -name '*.ctrf' -o -name 'ctrf*.json' \) | head -n 1)"
[ -n "$CTRF_PATH" ] || fail "oracle preflight produced no CTRF report"
CTRF_SHA="$(sha256_bytes "$CTRF_PATH")"
HARBOR_EFFECTIVE_SHA="$(lf_sha256_bytes "$ORACLE_DIR/harbor-results.json")"

# Record the verifier phase's actual effective network mode. The pinned task
# declares environment.allow_internet = true and harbor v0.22.0 keeps the
# shared verifier on that public baseline; this run applies no offline
# overlay, so the receipt must not claim one.
ACTUAL_NETWORK_MODE="$(python3 - "$WORK_ROOT/terminal-bench-2-1/tasks/$TB21_TASK_ID/task.toml" <<'PY'
import sys, tomllib

with open(sys.argv[1], "rb") as handle:
    doc = tomllib.load(handle)
if not doc.get("environment", {}).get("allow_internet", False):
    raise SystemExit("pinned task is not public-network; revisit the network-mode recording")
print("public")
PY
)"
[ -n "$ACTUAL_NETWORK_MODE" ] || fail "cannot determine the effective verifier network mode"
{
  echo "network-mode: $ACTUAL_NETWORK_MODE"
  echo "source: pinned task.toml environment.allow_internet and harbor ${HARBOR_COMMIT} shared-verifier network baseline"
  echo "offline-enforcement: not applied by this materialization run"
} > "$ORACLE_DIR/network-inspection.txt"

# ---------------------------------------------------------------------------
# Stage 9: receipt.
# ---------------------------------------------------------------------------
info "stage 9: receipt"
RESOLVED_AT="$(now_utc)"
EXPIRES_AT="$(expiry_utc)"
export PHASE18_CANDIDATE_COMMIT="$CANDIDATE_COMMIT"
export PHASE18_WORKFLOW_REF="$WORKFLOW_REF"
export PHASE18_WORKFLOW_SHA="$WORKFLOW_SHA"
export PHASE18_RUN_ID="$RUN_ID"
export PHASE18_RUN_ATTEMPT="$RUN_ATTEMPT"
export PHASE18_RESOLVED_AT="$RESOLVED_AT"
export PHASE18_EXPIRES_AT="$EXPIRES_AT"
export PHASE18_CTRF_SHA="$CTRF_SHA"
export PHASE18_HARBOR_EFFECTIVE_SHA="$HARBOR_EFFECTIVE_SHA"
export PHASE18_NETWORK_MODE="$ACTUAL_NETWORK_MODE"
export PHASE18_ORACLE_REWARD="$(python3 -c 'import json,sys; print(min(json.load(open(sys.argv[1]))["rewards"].values()))' "$ORACLE_DIR/oracle-summary.json")"
python3 - "$STATIC_LOCK_PATH" "$OUTPUT_DIR" > "$OUTPUT_DIR/receipt.json" <<'PY'
import hashlib, json, os, sys

lock_path, output_dir = sys.argv[1], sys.argv[2]
with open(lock_path, encoding="utf-8") as handle:
    lock_bytes = handle.read().encode()
static_sha = hashlib.sha256(lock_bytes.replace(b"\r\n", b"\n")).hexdigest()
with open(f"{output_dir}/closure-manifest.json", encoding="utf-8") as handle:
    closure_manifest = json.load(handle)
with open(f"{output_dir}/pulled-images.json", encoding="utf-8") as handle:
    pulled = json.load(handle)

producers = []
authority = json.loads(lock_bytes)["authority"]
for producer in authority["producers"]:
    with open(producer["path"], "rb") as handle:
        observed = hashlib.sha256(handle.read().replace(b"\r\n", b"\n")).hexdigest()
    producers.append({"path": producer["path"], "sha256": observed})

receipt = {
    "schema": "phase18-materialization-receipt/1",
    "lock_id": "phase18-linux-x86_64",
    "platform": "linux-x86_64",
    "candidate_commit": os.environ["PHASE18_CANDIDATE_COMMIT"],
    "static_lock": {"path": os.path.relpath(lock_path), "sha256": static_sha},
    "workflow": {
        "ref": os.environ.get("PHASE18_WORKFLOW_REF", "unknown"),
        "sha": os.environ.get("PHASE18_WORKFLOW_SHA", os.environ["PHASE18_CANDIDATE_COMMIT"]),
        "path": authority["workflow"]["path"],
        "bytes_sha256": authority["workflow"]["sha256"],
    },
    "producers": producers,
    "run": {
        "id": int(os.environ.get("PHASE18_RUN_ID", "0")),
        "attempt": int(os.environ.get("PHASE18_RUN_ATTEMPT", "1")),
    },
    "resolved_at": os.environ["PHASE18_RESOLVED_AT"],
    "artifact_name": "phase18-linux-lock-materialization",
    "expires_at": os.environ["PHASE18_EXPIRES_AT"],
    "closure": {
        "manifest_sha256": closure_manifest["manifest_sha256"],
        "file_count": len(closure_manifest["files"]),
        "total_bytes": sum(record["size"] for record in closure_manifest["files"]),
    },
    "images": [pulled],
    "oracle": {
        "status": "passed",
        "reward": float(os.environ["PHASE18_ORACLE_REWARD"]),
        "ctrf_sha256": os.environ["PHASE18_CTRF_SHA"],
        "harbor_results_sha256": os.environ["PHASE18_HARBOR_EFFECTIVE_SHA"],
    },
    "network": {
        "mode": os.environ["PHASE18_NETWORK_MODE"],
        "evidence": "oracle/network-inspection.txt",
    },
}
print(json.dumps(receipt, sort_keys=True, indent=2))
PY

RECEIPT_SHA="$(sha256_bytes "$OUTPUT_DIR/receipt.json")"
info "materialization complete"
info "  closure manifest : $CLOSURE_MANIFEST_SHA ($CLOSURE_FILE_COUNT files)"
info "  receipt          : $OUTPUT_DIR/receipt.json ($RECEIPT_SHA)"
info "  expires_at       : $EXPIRES_AT"
info "  staging          : $OUTPUT_DIR"
