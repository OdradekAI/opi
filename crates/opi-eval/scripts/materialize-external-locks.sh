#!/usr/bin/env bash
# opi-eval Linux x86_64 external-lock materializer (producer).
#
# Executes the committed static external lock at
# crates/opi-eval/external-locks/static/linux-x86_64.json on the manually
# authorized Linux runner: builds the candidate Companion from Cargo
# compiler-artifact output, delegates all native preparation and the explicit
# oracle-witness conformance to the shared local CLI (opi-eval prepare /
# opi-eval conformance), materializes and seals the Terminal-Bench 2.1
# verifier dependency closure as transport-only provenance, inspects the
# already prepared pinned task image graph, and writes a /2 receipt that the
# artifact verification stage verifies and admits as the resolved Linux
# lock. Offline verifier enforcement is not claimed by this run; the receipt
# records the task-declared network mode with its pinned-task-policy basis.
#
# The script is data-driven: every upstream URL, digest, Git identity, and
# environment control is read from the static lock; nothing is resolved
# dynamically and no mutable tag is ever pulled. Every stage fails closed on
# the first unverifiable byte, and a shared-CLI refusal prevents receipt
# publication entirely. Repo-tracked text pins (workflow, producer scripts,
# lock documents) hash over LF-normalized bytes; downloaded upstream
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


def pi_runtime_closure(source, package):
    # These are the pinned bundle's external imports and chord's Linux x64
    # esbuild dependency. Keep development and other-platform packages out.
    locked = json.loads((source / "package-lock.json").read_text())["packages"]
    names = ("@earendil-works/chord", "@silvia-odwyer/photon-node", "jiti", "esbuild", "@esbuild/linux-x64")
    for name in names:
        relative = "node_modules/" + name
        origin = source / relative
        pin = locked.get(relative, {})
        for ancestor in origin.parents:
            if ancestor == source:
                break
            if ancestor.is_symlink():
                fail(f"pi runtime {name}: linked ancestor")
        if name == "@earendil-works/chord":
            if pin != {"resolved": "packages/chord", "link": True} or not origin.is_symlink():
                fail("pi runtime chord: unexpected workspace link")
            target = source / "packages/chord"
            if target.is_symlink() or target.parent.is_symlink() or origin.resolve() != target:
                fail("pi runtime chord: workspace link target drift")
            origin = target
            pin = locked.get("packages/chord", {})
        elif origin.is_symlink():
            fail(f"pi runtime {name}: unexpected package link")
        if not origin.is_dir():
            fail(f"pi runtime {name}: missing locked package")
        metadata = origin / "package.json"
        if metadata.is_symlink() or not metadata.is_file():
            fail(f"pi runtime {name}: missing regular metadata")
        manifest = json.loads(metadata.read_text())
        if manifest.get("name") != name or not pin.get("version") or manifest.get("version") != pin["version"]:
            fail(f"pi runtime {name}: locked package identity drift")
        destination = package / relative
        if name == "@earendil-works/chord":
            # The workspace package contains source and tests beside its built
            # runtime. Resolve its one known link, then copy only release files.
            if (origin / "dist").is_symlink() or not (origin / "dist").is_dir():
                fail("pi runtime chord: missing regular distribution")
            package_digest(origin / "dist")
            shutil.copytree(origin / "dist", destination / "dist", symlinks=True)
            shutil.copy2(metadata, destination / "package.json")
            for item in (origin / "README.md", source / "LICENSE"):
                if item.is_symlink() or not item.is_file():
                    fail("pi runtime chord: missing regular notice")
                shutil.copy2(item, destination / item.name)
        else:
            package_digest(origin)
            shutil.copytree(origin, destination, symlinks=True)


def validate_pi_runtime(package, room):
    # No ambient provider config, Node injection or source-tree module lookup.
    home = room / "pi-smoke-home"
    home.mkdir()
    environment = {"HOME": str(home), "XDG_CONFIG_HOME": str(home / "config"), "PI_CODING_AGENT_DIR": str(home / "pi"), "PATH": "/usr/bin:/bin", "LANG": "C.UTF-8"}
    node = package / "node"
    version = json.loads((package / "package.json").read_text())["version"]
    observed = subprocess.check_output([str(node), str(package / "dist/bundle/cli.js"), "--version"], cwd=room, env=environment, text=True, timeout=30).strip()
    if observed != version:
        fail("pi runtime: standalone version drift")
    smoke = '''
import { createRequire } from "node:module";
import { pathToFileURL } from "node:url";
const root = process.argv[1];
const require = createRequire(root + "/package.json");
for (const name of ["@earendil-works/chord", "@earendil-works/chord/bundler", "@earendil-works/chord/context", "@earendil-works/chord/delta", "@earendil-works/chord/node", "@silvia-odwyer/photon-node", "jiti"]) {
    const resolved = import.meta.resolve(name);
    if (!resolved.startsWith(pathToFileURL(root + "/node_modules/").href)) throw new Error("external runtime resolution: " + name);
    await import(resolved);
}
const esbuild = require("esbuild");
if (!require.resolve("@esbuild/linux-x64/bin/esbuild").startsWith(root + "/node_modules/")) throw new Error("external esbuild binary");
const result = esbuild.transformSync("const n: number = 1", { loader: "ts" });
if (!result.code.includes("const n = 1")) throw new Error("esbuild transform failed");
'''
    subprocess.run([str(node), "--input-type=module", "--eval", smoke, str(package)], cwd=package, env=environment, check=True, timeout=30)


if not isinstance(entries, list):
    fail("acquisition plan must be a list")
missing = set()
for entry in entries:
    identity = entry["id"]
    parts = Path(identity).parts
    if not parts or Path(identity).is_absolute() or any(part in (".", "..") for part in parts):
        fail("invalid cache-relative acquisition identity")
    if parts[0] not in ("agents", "corpora", "runners", "tools", "images") or len(parts) < 2:
        fail(f"{identity}: required pin must be explicitly seeded")
    if parts[0] == "images" and (len(parts) != 2 or not re.fullmatch(r"[A-Za-z0-9_.-]+", parts[1])):
        fail("invalid image acquisition identity")
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
selected_images = {}
for kind, identity in missing:
    if kind != "images":
        continue
    matches = [image for image in lock.get("images", []) if image.get("id") == identity]
    if len(matches) != 1:
        fail(f"{identity}: image requires one exact static pin")
    image = matches[0]
    reference, manifest = image.get("reference"), image.get("manifest")
    if not isinstance(reference, str) or not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_./:@-]*", reference):
        fail(f"{identity}: invalid image reference")
    if not isinstance(manifest, str) or not re.fullmatch(r"sha256:[0-9a-f]{64}", manifest):
        fail(f"{identity}: image has no immutable manifest")
    selected_images[identity] = (f"{reference}@{manifest}", manifest)
    allowed.add((kind, identity))
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
            run("bash", builder, "--pi-source", source, "--static-lock", cache / "pins/static/linux-x86_64.json", "--out", output, env=environment)
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
            pi_runtime_closure(source, package)
            (package / "pi").write_text('#!/bin/sh\nagent_root=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)\nexec "$agent_root/node" "$agent_root/dist/bundle/cli.js" "$@"\n')
            (package / "pi").chmod(0o755)
        shutil.copy2(output / f"{product}-identity.json", package / "build-identity.json")
        digest = package_digest(package)
        if product == "pi":
            validate_pi_runtime(package, room)
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

    for identity, (reference, manifest) in sorted(selected_images.items()):
        def present():
            result = subprocess.run(["docker", "image", "inspect", "--format", "{{json .RepoDigests}}", reference], capture_output=True, text=True, timeout=15)
            if result.returncode != 0:
                return False
            try:
                digests = json.loads(result.stdout)
            except ValueError:
                return False
            return isinstance(digests, list) and any(isinstance(value, str) and value.rsplit("@", 1)[-1] == manifest for value in digests)

        if not present():
            run("docker", "pull", "--quiet", reference)
            if not present():
                fail(f"{identity}: pulled image digest is unverified")
            print(f"prepare: acquired images/{identity}; manifest={manifest}", flush=True)
        else:
            print(f"prepare: reused images/{identity}; manifest={manifest}", flush=True)

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

# The candidate Companion executable, selected from Cargo compiler-artifact
# output (never an assumed target path). A cargo failure is surfaced, never
# swallowed silently: the exit status and the stderr tail ride along with
# the rejection.
resolve_companion_executable() {
  local build_json cargo_rc
  export OPI_EVAL_BUILD_ERR="$WORK_ROOT/opi-eval-build.err"
  build_json=$(cargo build --locked --release -p opi-eval \
    --message-format=json-render-diagnostics 2>"$OPI_EVAL_BUILD_ERR") \
    && cargo_rc=0 || cargo_rc=$?
  printf '%s\n' "$build_json" | CARGO_RC="$cargo_rc" python3 -c '
import json, os, sys
for line in sys.stdin:
    line = line.strip()
    if not line.startswith("{"):
        continue
    message = json.loads(line)
    if (message.get("reason") == "compiler-artifact"
            and message.get("target", {}).get("name") == "opi-eval"
            and message.get("executable")):
        print(message["executable"])
        break
else:
    tail = ""
    try:
        tail = open(os.environ["OPI_EVAL_BUILD_ERR"], encoding="utf-8"
                    ).read().strip()[-1500:]
    except OSError:
        pass
    sys.exit(f"no opi-eval compiler-artifact executable was reported "
             f"(cargo rc={os.environ.get("CARGO_RC")}); stderr tail: {tail}")
'
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
# Stage 2: shared preparation and explicit conformance (D4).
# ---------------------------------------------------------------------------
info "stage 2: shared prepare and conformance delegation"
require_tool cargo
mkdir -p "$OUTPUT_DIR"
WORK_ROOT="$(mktemp -d "$OUTPUT_DIR/work.XXXXXX")"
SHARED_DIR="$OUTPUT_DIR/shared"
ORACLE_DIR="$OUTPUT_DIR/oracle"
mkdir -p "$SHARED_DIR" "$ORACLE_DIR"

# 2a. Locate the candidate Companion from Cargo compiler-artifact output
#     (never an assumed target path; a cargo failure is surfaced, never
#     swallowed silently).
COMPANION="$(resolve_companion_executable)"
[ -n "$COMPANION" ] && [ -x "$COMPANION" ] ||
  fail "resolved Companion executable is not runnable: $COMPANION"
COMPANION_PATH="$(command -v -- "$COMPANION" 2>/dev/null || echo "$COMPANION")"
cp -- "$COMPANION_PATH" "$SHARED_DIR/companion"
chmod 0755 "$SHARED_DIR/companion"
COMPANION_SHA="$(sha256_bytes "$SHARED_DIR/companion")"

# 2b. Seed the declared cache from this checkout: the static lock, the
#     pinned producer scripts, the benchmark profile, and the integrations.
#     Shared prepare acquires the corpora/runner/tool/agent identities
#     itself through the materializer's supervised prepare mode; the
#     default route never duplicates that acquisition.
CACHE_DIR="$WORK_ROOT/cache"
mkdir -p "$CACHE_DIR/pins/static" "$CACHE_DIR/pins/scripts" \
  "$CACHE_DIR/pins/profiles/benchmarks" "$CACHE_DIR/pins/integrations"
cp -- "$STATIC_LOCK_PATH" "$CACHE_DIR/pins/static/linux-x86_64.json"
for producer_path in $(lock_field authority.producers | python3 -c 'import json,sys; [print(p["path"]) for p in json.load(sys.stdin)]'); do
  [ -f "$producer_path" ] || fail "producer $producer_path is missing from the checkout"
  cp -- "$producer_path" "$CACHE_DIR/pins/scripts/$(basename "$producer_path")"
done
[ -f "crates/opi-eval/profiles/benchmarks/terminal-bench-2.1.toml" ] ||
  fail "benchmark profile is missing from the checkout"
cp -- "crates/opi-eval/profiles/benchmarks/terminal-bench-2.1.toml" \
  "$CACHE_DIR/pins/profiles/benchmarks/terminal-bench-2.1.toml"
[ -f "crates/opi-eval/scripts/scripted-provider.py" ] ||
  fail "scripted provider is missing from the checkout"
cp -- "crates/opi-eval/scripts/scripted-provider.py" "$CACHE_DIR/pins/scripts/scripted-provider.py"
for family in harbor pier; do
  [ -d "crates/opi-eval/integrations/$family" ] ||
    fail "integration $family is missing from the checkout"
  mkdir -p "$CACHE_DIR/pins/integrations/$family"
  find "crates/opi-eval/integrations/$family" -maxdepth 1 -type f -name '*.py' -exec cp -- {} "$CACHE_DIR/pins/integrations/$family/" \;
  [ -n "$(find "$CACHE_DIR/pins/integrations/$family" -maxdepth 1 -type f -name '*.py')" ] ||
    fail "integration $family has no python modules"
done

# 2c. One TB2.1 diagnostic config: the pinned task, both clients, an
#     unused loopback fixture connection, and no credentials. The
#     diagnostic never executes either Agent or dials a provider.
TB21_TASK_ID="$(lock_field subjects.1.task.id)"
mkdir -p "$WORK_ROOT/results"
cat > "$WORK_ROOT/diagnostic.toml" <<DIAG
# External-lock diagnostic experiment: shared prepare freezes the pinned
# material; shared conformance runs the oracle witness. No trial executes,
# the loopback fixture connection is never dialed, and no credential is
# read.

schema = "opi-eval-experiment/2"
experiment_id = "external-lock-diagnostic"
connection = "scripted-local"

[benchmark]
name = "terminal-bench"
revision = "2.1"
dataset = "terminal-bench-2.1"
# The run path alone validates this digest; the diagnostic never runs
# trials, so the placeholder stays unclaimed.
integrity_digest = "0000000000000000000000000000000000000000000000000000000000000000"

[[subjects]]
id = "baseline-pi"
product = "pi"
version = "0.84.3"

[[subjects]]
id = "candidate-opi"
product = "opi"
version = "0.1.0"

[[edges]]
id = "edge-1"
baseline = "baseline-pi"
candidate = "candidate-opi"

[[connections]]
id = "scripted-local"
provider = "local"
api = "openai-completions"
base_url = "http://127.0.0.1:48127/v1"
model = "scripted"
credential_env = "OPI_EVAL_SCRIPTED_KEY"

[model_controls]
temperature = 0.0
top_p = "omitted"
max_output_tokens = 4096
context_window = 8192
reasoning = "omitted"

[storage]
cache = "cache"
results = "results"
capture_policy = "full"

[environment]
platform = "linux"
architecture = "x86_64"
cwd_policy = "isolated"

[[trials]]
id = "trial-pi-diagnostic"
subject = "baseline-pi"
task = "$TB21_TASK_ID"
group = "group-diagnostic"

[[trials]]
id = "trial-opi-diagnostic"
subject = "candidate-opi"
task = "$TB21_TASK_ID"
group = "group-diagnostic"
DIAG
cp -- "$WORK_ROOT/diagnostic.toml" "$SHARED_DIR/diagnostic.toml"
CONFIG_SHA="$(sha256_bytes "$SHARED_DIR/diagnostic.toml")"

# 2d. Shared preparation: opi-eval prepare owns every acquisition and the
#     fresh native-material freeze. Its refusal fails this run closed.
PREPARED_DIR="$WORK_ROOT/prepared"
PREPARE_REPORT="$WORK_ROOT/prepare-report.json"
PREPARE_COMMAND="$(python3 -c 'import json,sys; print(json.dumps(sys.argv[1:]))' \
  "$COMPANION_PATH" prepare \
  --config "$WORK_ROOT/diagnostic.toml" \
  --out "$PREPARED_DIR" \
  --opi-source "$PWD" \
  --opi-commit "$CANDIDATE_COMMIT")"
if ! "$COMPANION_PATH" prepare \
    --config "$WORK_ROOT/diagnostic.toml" \
    --out "$PREPARED_DIR" \
    --opi-source "$PWD" \
    --opi-commit "$CANDIDATE_COMMIT" > "$PREPARE_REPORT"; then
  fail "shared opi-eval prepare refused the diagnostic request"
fi
python3 - "$STATIC_LOCK_PATH" "$PREPARE_REPORT" "$PREPARED_DIR/material.json" "$TB21_TASK_ID" "$STATIC_LOCK_SHA" <<'PY' || fail "shared preparation did not bind the pinned material"
import json, sys

lock_path, report_path, material_path, task_id, static_sha = sys.argv[1:6]
report = json.load(open(report_path, encoding="utf-8"))
material = json.load(open(material_path, encoding="utf-8"))
if report.get("schema") != "opi-eval-prepare/1":
    raise SystemExit(f"unexpected prepare report schema: {report.get('schema')!r}")
if report.get("origin") != "local":
    raise SystemExit("prepare report is not local-origin")
if report.get("material") != material_path:
    raise SystemExit(f"prepare report material {report.get('material')!r} is not the frozen manifest")
if material.get("schema") != "opi-eval-native-material/7":
    raise SystemExit(f"fresh material schema is not opi-eval-native-material/7: {material.get('schema')!r}")
if material.get("origin") != "local":
    raise SystemExit("frozen material is not local-origin")
if material.get("static_lock", {}).get("sha256") != static_sha:
    raise SystemExit("frozen material does not bind this run's static lock")
tasks = material.get("benchmarks", {}).get("terminal-bench-2.1", {}).get("tasks", {})
if task_id not in tasks:
    raise SystemExit(f"frozen material does not select the pinned task {task_id}")
print("shared prepare bound the pinned material")
PY
cp -- "$PREPARED_DIR/material.json" "$SHARED_DIR/material.json"
cp -- "$PREPARE_REPORT" "$SHARED_DIR/prepare-report.json"
MATERIAL_SHA="$(sha256_bytes "$SHARED_DIR/material.json")"
PREPARE_REPORT_SHA="$(sha256_bytes "$SHARED_DIR/prepare-report.json")"

# 2e. Explicit shared conformance: the official upstream oracle through the
#     prepared material, never an inline trial driver. Its refusal (or a
#     non-met witness) fails this run closed and publishes no receipt.
CONFORMANCE_ROOT="$WORK_ROOT/conformance"
CONFORMANCE_REPORT="$WORK_ROOT/conformance-report.json"
CONFORMANCE_COMMAND="$(python3 -c 'import json,sys; print(json.dumps(sys.argv[1:]))' \
  "$COMPANION_PATH" conformance \
  --suite benchmark \
  --adapter terminal-bench-2.1 \
  --case oracle-witness \
  --root "$CONFORMANCE_ROOT" \
  --fixtures "$PWD/crates/opi-eval/tests/fixtures" \
  --provider "$CACHE_DIR/pins/scripts/scripted-provider.py" \
  --native-material "$PREPARED_DIR/material.json")"
if ! "$COMPANION_PATH" conformance \
    --suite benchmark \
    --adapter terminal-bench-2.1 \
    --case oracle-witness \
    --root "$CONFORMANCE_ROOT" \
    --fixtures "$PWD/crates/opi-eval/tests/fixtures" \
    --provider "$CACHE_DIR/pins/scripts/scripted-provider.py" \
    --native-material "$PREPARED_DIR/material.json" > "$CONFORMANCE_REPORT"; then
  fail "shared opi-eval conformance refused the oracle witness"
fi
python3 - "$CONFORMANCE_REPORT" "$TB21_TASK_ID" "$STATIC_LOCK_SHA" <<'PY' || fail "shared conformance did not admit the oracle witness"
import json, sys

report = json.load(open(sys.argv[1], encoding="utf-8"))
task_id, static_sha = sys.argv[2], sys.argv[3]
if report.get("schema") != "opi-eval-conformance-report/1":
    raise SystemExit(f"unexpected conformance report schema: {report.get('schema')!r}")
expected = {"suite": "benchmark", "adapter": "terminal-bench-2.1", "case": "oracle-witness"}
for key, value in expected.items():
    if report.get(key) != value:
        raise SystemExit(f"conformance {key} is {report.get(key)!r}, expected {value!r}")
if report.get("exit_state") != "exited:0":
    raise SystemExit(f"conformance exit state is {report.get('exit_state')!r}")
if report.get("outcome") != "verified-native":
    raise SystemExit(f"conformance outcome is {report.get('outcome')!r}")
if report.get("met") is not True:
    raise SystemExit("conformance witness did not meet the pinned expectation")
identity = report.get("identity", {})
if identity.get("task_id") != task_id:
    raise SystemExit(f"conformance task {identity.get('task_id')!r} is not the pinned {task_id!r}")
if identity.get("admitted_lock_digest") != static_sha:
    raise SystemExit("conformance did not admit this run's static lock")
reward = report.get("reward", {})
if reward.get("state") != "known" or reward.get("origin") != "native-trial-result":
    raise SystemExit(f"conformance reward is not a known native result: {reward!r}")
print("shared conformance admitted the oracle witness")
PY
cp -- "$CONFORMANCE_REPORT" "$SHARED_DIR/conformance-report.json"
CONFORMANCE_REPORT_SHA="$(sha256_bytes "$SHARED_DIR/conformance-report.json")"

# 2f. Bind the raw verifier artifacts uniquely: exactly one raw result and
#     one CTRF report under the shared conformance root, zero or ambiguous
#     matches refuse before any receipt exists.
python3 - "$CONFORMANCE_ROOT" "$ORACLE_DIR" <<'PY' || fail "shared conformance raw artifacts are missing or ambiguous"
import hashlib, shutil, sys
from pathlib import Path

root, oracle_dir = Path(sys.argv[1]), Path(sys.argv[2])
roles = {"native-result.json": "raw-result.json", "ctrf-report.json": "ctrf-report.json"}
for name, target in roles.items():
    matches = sorted(path for path in root.rglob(name) if path.is_file())
    if not matches:
        raise SystemExit(f"shared conformance root carries no {name}")
    if len(matches) > 1:
        raise SystemExit(f"shared conformance root carries ambiguous {name}: {[str(p) for p in matches]}")
    shutil.copy2(matches[0], f"{oracle_dir}/{target}")
    print(f"bound {name} -> oracle/{target}")
PY
RAW_RESULT_SHA="$(sha256_bytes "$ORACLE_DIR/raw-result.json")"
CTRF_SHA="$(sha256_bytes "$ORACLE_DIR/ctrf-report.json")"

# Record the task-declared network mode from the pinned corpus bytes the
# shared preparation verified. This is a pinned-task-policy observation,
# never a claim about an effective mode this run did not observe, and
# offline enforcement is never inferred from host UV_OFFLINE.
TASK_TOML="$CACHE_DIR/corpora/terminal-bench-2.1/tasks/$TB21_TASK_ID/task.toml"
[ -f "$TASK_TOML" ] ||
  fail "pinned task package is absent from the prepared cache; cannot record the task-declared network mode"
ACTUAL_NETWORK_MODE="$(python3 - "$TASK_TOML" <<'PY'
import sys, tomllib

with open(sys.argv[1], "rb") as handle:
    doc = tomllib.load(handle)
if not doc.get("environment", {}).get("allow_internet", False):
    raise SystemExit("pinned task is not public-network; revisit the network-mode recording")
print("public")
PY
)"
[ -n "$ACTUAL_NETWORK_MODE" ] || fail "cannot determine the task-declared network mode"
{
  echo "task-declared-network-mode: $ACTUAL_NETWORK_MODE"
  echo "basis: pinned-task-policy"
  echo "source: pinned task.toml environment.allow_internet verified by the shared preparation"
  echo "offline-enforcement: not claimed by this materialization run"
} > "$ORACLE_DIR/network-inspection.txt"

# ---------------------------------------------------------------------------
# Stage 3: materialize the pinned apt closure (transport-only).
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
# Stage 4: materialize the pinned uv and Python wheel closure
# (transport-only).
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
# Stage 5: inspect the prepared pinned task image graph.
# ---------------------------------------------------------------------------
info "stage 5: task image graph"
TB21_IMAGE_REF="$(lock_field images.0.reference)"
TB21_IMAGE_MANIFEST="$(lock_field images.0.manifest)"
docker buildx imagetools inspect "$TB21_IMAGE_REF@$TB21_IMAGE_MANIFEST" --raw > "$OUTPUT_DIR/task-image-manifest.json" ||
  fail "cannot inspect the pinned task image manifest"
IMAGES_JSON="$OUTPUT_DIR/pulled-images.json"
python3 - "$STATIC_LOCK_PATH" "$OUTPUT_DIR/task-image-manifest.json" > "$IMAGES_JSON" <<'PY' || fail "inspected image graph does not match the static lock"
import hashlib, json, sys

lock_path, manifest_path = sys.argv[1], sys.argv[2]
with open(lock_path, encoding="utf-8") as handle:
    pinned = json.load(handle)["images"][0]
with open(manifest_path, "rb") as handle:
    raw = handle.read()
actual_manifest = "sha256:" + hashlib.sha256(raw).hexdigest()
if actual_manifest != pinned["manifest"]:
    raise SystemExit(f"inspected manifest {actual_manifest} != pinned {pinned['manifest']}")
manifest = json.loads(raw)
config = manifest.get("config", {}).get("digest", "")
layers = [layer.get("digest", "") for layer in manifest.get("layers", [])]
if pinned.get("config") and config != pinned["config"]:
    raise SystemExit(f"inspected config {config} != pinned {pinned['config']}")
if pinned.get("layers") and layers != pinned["layers"]:
    raise SystemExit(f"inspected layers {layers} != pinned {pinned['layers']}")
print(json.dumps({
    "id": pinned["id"],
    "reference": pinned["reference"],
    "manifest": actual_manifest,
    "config": config,
    "layers": layers,
}, sort_keys=True))
PY

# ---------------------------------------------------------------------------
# Stage 6: seal the closure directory with a canonical manifest.
# ---------------------------------------------------------------------------
info "stage 6: closure sealing"
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
# Stage 7: receipt (/2).
# ---------------------------------------------------------------------------
info "stage 7: receipt"
RESOLVED_AT="$(now_utc)"
EXPIRES_AT="$(expiry_utc)"
export OPI_EVAL_CANDIDATE_COMMIT="$CANDIDATE_COMMIT"
export OPI_EVAL_WORKFLOW_REF="$WORKFLOW_REF"
export OPI_EVAL_WORKFLOW_SHA="$WORKFLOW_SHA"
export OPI_EVAL_RUN_ID="$RUN_ID"
export OPI_EVAL_RUN_ATTEMPT="$RUN_ATTEMPT"
export OPI_EVAL_RESOLVED_AT="$RESOLVED_AT"
export OPI_EVAL_EXPIRES_AT="$EXPIRES_AT"
export OPI_EVAL_COMPANION_SHA="$COMPANION_SHA"
export OPI_EVAL_CONFIG_SHA="$CONFIG_SHA"
export OPI_EVAL_MATERIAL_SHA="$MATERIAL_SHA"
export OPI_EVAL_PREPARE_REPORT_SHA="$PREPARE_REPORT_SHA"
export OPI_EVAL_CONFORMANCE_REPORT_SHA="$CONFORMANCE_REPORT_SHA"
export OPI_EVAL_PREPARE_COMMAND="$PREPARE_COMMAND"
export OPI_EVAL_CONFORMANCE_COMMAND="$CONFORMANCE_COMMAND"
export OPI_EVAL_RAW_RESULT_SHA="$RAW_RESULT_SHA"
export OPI_EVAL_CTRF_SHA="$CTRF_SHA"
export OPI_EVAL_NETWORK_MODE="$ACTUAL_NETWORK_MODE"
export OPI_EVAL_ORACLE_REWARD="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["reward"]["value"])' "$SHARED_DIR/conformance-report.json")"
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

def shared_ref(name, digest):
    return {"path": f"shared/{name}", "sha256": digest}

receipt = {
    "schema": "opi-eval-materialization-receipt/2",
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
    "shared": {
        "companion": {
            **shared_ref("companion", os.environ["OPI_EVAL_COMPANION_SHA"]),
            "located_by": "cargo-compiler-artifact",
        },
        "config": shared_ref("diagnostic.toml", os.environ["OPI_EVAL_CONFIG_SHA"]),
        "prepare": {
            "command": json.loads(os.environ["OPI_EVAL_PREPARE_COMMAND"]),
            "report": shared_ref("prepare-report.json", os.environ["OPI_EVAL_PREPARE_REPORT_SHA"]),
            "material": {
                **shared_ref("material.json", os.environ["OPI_EVAL_MATERIAL_SHA"]),
                "schema": "opi-eval-native-material/7",
            },
        },
        "conformance": {
            "command": json.loads(os.environ["OPI_EVAL_CONFORMANCE_COMMAND"]),
            "report": shared_ref("conformance-report.json", os.environ["OPI_EVAL_CONFORMANCE_REPORT_SHA"]),
            "suite": "benchmark",
            "adapter": "terminal-bench-2.1",
            "case": "oracle-witness",
            "met": True,
            "agent_score": False,
            "reward_origin": "native-trial-result",
        },
    },
    "closure": {
        "manifest_sha256": closure_manifest["manifest_sha256"],
        "file_count": len(closure_manifest["files"]),
        "total_bytes": sum(record["size"] for record in closure_manifest["files"]),
    },
    "images": [pulled],
    "oracle": {
        "status": "passed",
        "reward": float(os.environ["OPI_EVAL_ORACLE_REWARD"]),
        "agent_score": False,
        "raw_result": {"path": "oracle/raw-result.json", "sha256": os.environ["OPI_EVAL_RAW_RESULT_SHA"]},
        "ctrf": {"path": "oracle/ctrf-report.json", "sha256": os.environ["OPI_EVAL_CTRF_SHA"]},
    },
    "network": {
        "mode": os.environ["OPI_EVAL_NETWORK_MODE"],
        "basis": "pinned-task-policy",
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
