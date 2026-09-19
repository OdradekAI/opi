#!/usr/bin/env bash
# opi-eval Linux x86_64 external-lock materializer (producer).
#
# Executes the committed static external lock at
# crates/opi-eval/external-locks/static/linux-x86_64.json on the manually
# authorized Linux runner: verifies every pinned upstream identity, pulls the
# pinned task image by digest, materializes and seals the Terminal-Bench 2.1
# verifier dependency closure, runs one official upstream oracle preflight
# through the pinned Harbor revision under the task's own upstream network
# configuration, and writes a receipt that the artifact verification stage verifies and admits as
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
usage: materialize-external-locks.sh --candidate-commit <40-hex>
                                     --static-lock <path>
                                     --output-dir <path>
       materialize-external-locks.sh prepare --cache <dir> --plan <json>

The prepare mode is the supervised local-acquisition entry opi-eval
prepare invokes on a cache miss: the plan lists missing cache-relative
identities and explicit selections; selected sources and build inputs are
verified against the static lock before installation. Opi uses only the
declared local Git commit. Seeded pins are never fetched implicitly. Only public
task/runner/build material is fetched; no model is called and no
credential is read.
USAGE
}

# Supervised local acquisition (P19-D05): acquire the plan's missing
# identities into the declared cache, fail-closed otherwise.
run_prepare_mode() {
  local cache="" plan=""
  while [ "$#" -gt 0 ]; do
    case "$1" in
      --cache) cache="$2"; shift 2 ;;
      --plan) plan="$2"; shift 2 ;;
      *) usage >&2; exit 2 ;;
    esac
  done
  [ -n "$cache" ] && [ -n "$plan" ] || { usage >&2; exit 2; }
  python3 - "$cache" "$plan" <<'PREPARE_PY'
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import urllib.request

cache, plan_path = (Path(value).resolve() for value in sys.argv[1:])
entries = json.loads(plan_path.read_text())
lock = json.loads((cache / "pins/static/linux-x86_64.json").read_text())
subjects = {row["id"]: row for row in lock["subjects"]}
tools = {row["id"]: row for row in lock["tools"]}


def fail(message):
    raise SystemExit(f"prepare: {message}")


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def run(*args, cwd=None, env=None):
    subprocess.run([str(arg) for arg in args], cwd=cwd, env=env, check=True)


def git(root, *args):
    return subprocess.check_output(["git", "-C", str(root), *args], text=True).strip()


def verify_blob(root, commit, relative, pin):
    if git(root, "rev-parse", f"{commit}:{relative}") != pin["git_blob"] or sha(root / relative) != pin["sha256"]:
        fail(f"{relative}: committed blob or working-byte drift")


def checkout(url, commit, destination):
    if not re.fullmatch(r"[0-9a-f]{40}", commit):
        fail("source requires an exact commit")
    run("git", "init", "--quiet", destination)
    run("git", "-C", destination, "-c", "protocol.file.allow=always", "fetch", "--quiet", "--depth=1", url, commit)
    run("git", "-C", destination, "-c", "core.autocrlf=false", "checkout", "--quiet", "--detach", commit)
    if git(destination, "rev-parse", "HEAD") != commit:
        fail("source commit drift")


def install(source, destination):
    # Only a previously absent destination can be claimed by this producer.
    destination.parent.mkdir(parents=True, exist_ok=True)
    if destination.exists() or destination.is_symlink():
        fail(f"refusing to replace existing cache entry {destination}")
    if source.is_dir():
        destination.mkdir()
        try:
            shutil.copytree(source, destination, dirs_exist_ok=True, symlinks=True)
        except BaseException:
            shutil.rmtree(destination)
            raise
    else:
        with destination.open("xb") as output, source.open("rb") as content:
            shutil.copyfileobj(content, output)
        destination.chmod(source.stat().st_mode & 0o777)


def archive(tool, room):
    pin = tools[tool]["archive"]
    downloaded = room / f"{tool}.archive"
    urllib.request.urlretrieve(pin["url"], downloaded)
    if sha(downloaded) != pin["sha256"]:
        fail(f"{tool}: archive digest drift")
    extracted = room / f"{tool}-archive"
    extracted.mkdir()
    with tarfile.open(downloaded) as source:
        source.extractall(extracted, filter="data")
    if tool == "uv":
        candidates = [path for path in (*extracted.glob("*/uv"), *extracted.glob("uv")) if path.is_file()]
        if len(candidates) != 1 or not candidates[0].is_file():
            fail("uv: archive must contain exactly one uv executable")
        executable = candidates[0]
        executable.chmod(0o755)
        cached = cache / "tools/uv"
        if cached.exists():
            if sha(cached) != sha(executable):
                fail("cached uv differs from its pinned archive")
        else:
            install(executable, cached)
        return cached
    candidates = list(extracted.glob("*/bin/node"))
    if len(candidates) != 1:
        fail("node: archive must preserve its complete distribution")
    distribution = candidates[0].parents[1]
    if not (distribution / "bin/npm").is_file():
        fail("node: archive is missing npm")
    return distribution / "bin/node"


def pinned_builder():
    name = "build-agent-artifacts.sh"
    matches = [row for row in lock["authority"]["producers"] if Path(row["path"]).name == name]
    path = cache / "pins/scripts" / name
    if len(matches) != 1 or not path.is_file() or hashlib.sha256(path.read_bytes().replace(b"\r\n", b"\n")).hexdigest() != matches[0]["sha256"]:
        fail("Agent builder is absent, unpinned, or changed")
    return path


def package_digest(root):
    rows = []
    for path in sorted(root.rglob("*")):
        if path.is_symlink() or not (path.is_file() or path.is_dir()):
            fail("runtime package contains a link or special file")
        relative = path.relative_to(root).as_posix()
        if path.is_file():
            rows.append((relative, sha(path)))
    return hashlib.sha256("".join(f"{name}\n{digest}\n" for name, digest in sorted(rows)).encode()).hexdigest()


if not isinstance(entries, list):
    fail("acquisition plan must be a list")
missing = set()
for entry in entries:
    identity = entry["id"]
    parts = Path(identity).parts
    if not parts or Path(identity).is_absolute() or any(part in (".", "..") for part in parts):
        fail("invalid cache-relative acquisition identity")
    if parts[0] not in ("agents", "corpora", "runners", "tools") or len(parts) < 2:
        fail(f"{identity}: required pin must be explicitly seeded")
    missing.add((parts[0], parts[1]))
if not entries:
    raise SystemExit(0)
products = entries[0]["products"]
benchmark = entries[0]["benchmark"]
if any(entry["products"] != products or entry["benchmark"] != benchmark for entry in entries):
    fail("inconsistent selected material plan")
if any(product not in ("opi", "pi") for product in products):
    fail("unsupported Agent product")
runner = "pier" if benchmark == "deepswe-v1.1" else "harbor"
allowed = {("agents", product) for product in products} | {("corpora", benchmark), ("runners", runner), ("tools", "uv")}
if not missing <= allowed:
    fail("acquisition plan exceeds its selected materials")

with tempfile.TemporaryDirectory(prefix="opi-eval-acquire-") as temporary:
    room = Path(temporary)
    if ("tools", "uv") in missing or ("runners", runner) in missing:
        archive("uv", room)
        observed = subprocess.check_output([str(cache / "tools/uv"), "--version"], text=True).strip()
        if observed != f"uv {tools['uv']['version']}":
            fail("uv version drift")

    for product in products:
        if ("agents", product) not in missing:
            continue
        builder = pinned_builder()
        output = room / f"{product}-build"
        output.mkdir()
        package = room / f"{product}-package"
        package.mkdir()
        if product == "opi":
            source, commit = entries[0].get("opi_source"), entries[0].get("opi_commit")
            if not source or not isinstance(commit, str) or not re.fullmatch(r"[0-9a-f]{40}", commit):
                fail("cold Opi build requires an explicit local source and commit")
            if git(source, "rev-parse", f"{commit}^{{commit}}") != commit:
                fail("Opi commit is unavailable locally")
            source_export = room / "opi-source"
            source_export.mkdir()
            archive_path = room / "opi-source.tar"
            with archive_path.open("wb") as stream:
                subprocess.run(["git", "-C", str(source), "archive", "--format=tar", commit], stdout=stream, check=True)
            with tarfile.open(archive_path) as source_tar:
                source_tar.extractall(source_export, filter="data")
            if sha(source_export / "Cargo.lock") != lock["workspace_input"]["cargo_lock_sha256"]:
                fail("Opi Cargo.lock differs from the declared workspace input")
            run("bash", builder, "--opi-source", source_export, "--opi-commit", commit, "--out", output)
            identity = json.loads((output / "opi-identity.json").read_text())
            executable = Path(identity["canonical_executable"])
            if identity["checkout_commit"] != commit or sha(executable) != identity["executable_sha256"]:
                fail("built Opi identity drift")
            shutil.copy2(executable, package / "opi")
        else:
            pin = subjects["pi"]
            source = room / "pi-source"
            checkout(pin["repository_url"], pin["commit"], source)
            for blob in pin["blobs"]:
                verify_blob(source, pin["commit"], blob["path"], blob)
            node = archive("node", room)
            if subprocess.check_output([str(node), "--version"], text=True).strip() != "v" + tools["node"]["version"]:
                fail("Node version drift")
            environment = dict(os.environ, PATH=str(node.parent) + os.pathsep + os.environ.get("PATH", ""))
            run("bash", builder, "--pi-source", source, "--out", output, env=environment)
            identity = json.loads((output / "pi-identity.json").read_text())
            bundle = Path(identity["bundle_path"])
            if identity["checkout_commit"] != pin["commit"] or sha(bundle) != identity["bundle_sha256"] or sha(node) != identity["node_sha256"]:
                fail("built pi identity drift")
            shutil.copy2(node, package / "node")
            source_package = source / "packages/coding-agent"
            for name in ("dist", "docs", "examples"):
                shutil.copytree(source_package / name, package / name, symlinks=True)
            for name in ("package.json", "README.md", "CHANGELOG.md"):
                shutil.copy2(source_package / name, package / name)
            (package / "pi").write_text('#!/bin/sh\nagent_root=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)\nexec "$agent_root/node" "$agent_root/dist/bundle/cli.js" "$@"\n')
            (package / "pi").chmod(0o755)
        shutil.copy2(output / f"{product}-identity.json", package / "build-identity.json")
        digest = package_digest(package)
        install(package, cache / "agents" / product)
        print(f"prepare: acquired agents/{product}; package_sha256={digest}", flush=True)

    if ("corpora", benchmark) in missing:
        pin = subjects[benchmark]
        source = room / benchmark
        checkout(pin["repository_url"], pin["commit"], source)
        if git(source, "rev-parse", f"{pin['commit']}:tasks") != pin["tasks_tree"]:
            fail("corpus tasks-tree drift")
        install(source, cache / "corpora" / benchmark)
        print(f"prepare: acquired complete corpora/{benchmark}", flush=True)

    if ("runners", runner) in missing:
        pin = tools[runner]
        target = cache / "runners" / runner
        created = False
        try:
            if not target.exists():
                source = room / runner
                urls = {"harbor": "https://github.com/harbor-framework/harbor", "pier": "https://github.com/datacurve-ai/pier"}
                checkout(urls[runner], pin["commit"], source)
                verify_blob(source, pin["commit"], "uv.lock", pin["uv_lock"])
                install(source, target)
                created = True
            if git(target, "rev-parse", "HEAD") != pin["commit"] or git(target, "status", "--porcelain", "--untracked-files=no"):
                fail("runner source drift")
            verify_blob(target, pin["commit"], "uv.lock", pin["uv_lock"])
            target = target.resolve()
            environment = dict(os.environ, UV_PROJECT=str(target), UV_PROJECT_ENVIRONMENT=str(target / ".venv"), UV_CACHE_DIR=str(cache / "uv-cache"), UV_NO_DEV="1", UV_NO_ENV_FILE="1", PYTHON_DOTENV_DISABLED="1")
            if runner == "pier":
                environment["UV_PYTHON"] = "3.12"
            # Installation at the final path keeps editable imports and shebangs valid.
            run(cache / "tools/uv", "sync", "--locked", "--no-dev", cwd=target, env=environment)
            if not (target / ".venv/bin/python").is_file() or not (target / f".venv/bin/{runner}").is_file():
                fail("runner environment lacks its interpreter or entrypoint")
            print(f"prepare: acquired runners/{runner}", flush=True)
        except BaseException:
            if created:
                shutil.rmtree(target)
            raise

print("prepare: selected acquisition settled", flush=True)
PREPARE_PY
}

if [ "${1:-}" = "prepare" ]; then
  shift
  run_prepare_mode "$@"
  exit 0
fi

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
  echo "opi-eval-materialize: FAIL: $*" >&2
  exit 1
}

info() {
  echo "opi-eval-materialize: $*"
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
[ "$LOCK_SCHEMA" = "opi-eval-external-lock/static/1" ] || fail "unsupported static lock schema: $LOCK_SCHEMA"
[ "$LOCK_ID" = "opi-eval-linux-x86_64" ] || fail "unexpected lock id: $LOCK_ID"

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
to the native-smoke profile for the native-smoke profile.
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
        trial_name="opi-eval-oracle-preflight",
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
export OPI_EVAL_CANDIDATE_COMMIT="$CANDIDATE_COMMIT"
export OPI_EVAL_WORKFLOW_REF="$WORKFLOW_REF"
export OPI_EVAL_WORKFLOW_SHA="$WORKFLOW_SHA"
export OPI_EVAL_RUN_ID="$RUN_ID"
export OPI_EVAL_RUN_ATTEMPT="$RUN_ATTEMPT"
export OPI_EVAL_RESOLVED_AT="$RESOLVED_AT"
export OPI_EVAL_EXPIRES_AT="$EXPIRES_AT"
export OPI_EVAL_CTRF_SHA="$CTRF_SHA"
export OPI_EVAL_HARBOR_EFFECTIVE_SHA="$HARBOR_EFFECTIVE_SHA"
export OPI_EVAL_NETWORK_MODE="$ACTUAL_NETWORK_MODE"
export OPI_EVAL_ORACLE_REWARD="$(python3 -c 'import json,sys; print(min(json.load(open(sys.argv[1]))["rewards"].values()))' "$ORACLE_DIR/oracle-summary.json")"
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
    "schema": "opi-eval-materialization-receipt/1",
    "lock_id": "opi-eval-linux-x86_64",
    "platform": "linux-x86_64",
    "candidate_commit": os.environ["OPI_EVAL_CANDIDATE_COMMIT"],
    "static_lock": {"path": os.path.relpath(lock_path), "sha256": static_sha},
    "workflow": {
        "ref": os.environ.get("OPI_EVAL_WORKFLOW_REF", "unknown"),
        "sha": os.environ.get("OPI_EVAL_WORKFLOW_SHA", os.environ["OPI_EVAL_CANDIDATE_COMMIT"]),
        "path": authority["workflow"]["path"],
        "bytes_sha256": authority["workflow"]["sha256"],
    },
    "producers": producers,
    "run": {
        "id": int(os.environ.get("OPI_EVAL_RUN_ID", "0")),
        "attempt": int(os.environ.get("OPI_EVAL_RUN_ATTEMPT", "1")),
    },
    "resolved_at": os.environ["OPI_EVAL_RESOLVED_AT"],
    "artifact_name": "opi-eval-linux-lock-materialization",
    "expires_at": os.environ["OPI_EVAL_EXPIRES_AT"],
    "closure": {
        "manifest_sha256": closure_manifest["manifest_sha256"],
        "file_count": len(closure_manifest["files"]),
        "total_bytes": sum(record["size"] for record in closure_manifest["files"]),
    },
    "images": [pulled],
    "oracle": {
        "status": "passed",
        "reward": float(os.environ["OPI_EVAL_ORACLE_REWARD"]),
        "ctrf_sha256": os.environ["OPI_EVAL_CTRF_SHA"],
        "harbor_results_sha256": os.environ["OPI_EVAL_HARBOR_EFFECTIVE_SHA"],
    },
    "network": {
        "mode": os.environ["OPI_EVAL_NETWORK_MODE"],
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
