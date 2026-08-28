#!/usr/bin/env bash
# Phase 18 Linux x86_64 native-smoke producer (task 18.14).
#
# The sole committed producer contract for the Phase 18 native smoke. Every
# stage writes a typed stage receipt (schema `phase18-native-stage-receipt/1`)
# and fails closed on drift: the dispatch stage binds github.workflow_ref,
# github.workflow_sha, the SHA-256 of the workflow bytes read from that
# workflow SHA (never the mutable working tree), the candidate_sha, every
# invoked script, the immutable action pins, and the static external lock;
# later stages bind agent, tool, provider, network, canary, trial, and
# artifact identities to it.
#
# Task 18.14 commits and statically verifies this contract; it does not
# claim that the six native trials have run. Task 18.15 dispatches it
# against resolved material. The provider never expands authority: no
# mutable refs, no ambient credentials, no undeclared listeners, no
# default routes, no outbound fallback, no fallback grader.
#
# Usage (one stage per sequential workflow step):
#   scripts/phase18-native-smoke.sh verify-dispatch \
#     --candidate-sha SHA --workflow-path PATH --workflow-sha SHA \
#     --workflow-ref REF --out DIR
#   scripts/phase18-native-smoke.sh host-identity --out DIR
#   scripts/phase18-native-smoke.sh record-tools --out DIR [--buildx-image DESC]
#   scripts/phase18-native-smoke.sh fetch-external --external-root DIR --out DIR
#   scripts/phase18-native-smoke.sh build-agents \
#     --build-script PATH --pi-source DIR --out DIR
#   scripts/phase18-native-smoke.sh provider-up \
#     --provider PATH --network NAME --out DIR
#   scripts/phase18-native-smoke.sh provider-probe --network NAME --out DIR
#   scripts/phase18-native-smoke.sh provider-down --network NAME --out DIR
#   scripts/phase18-native-smoke.sh preflight-canaries \
#     --external-root DIR --provider PATH --out DIR
#   scripts/phase18-native-smoke.sh run-trials \
#     --experiment-root DIR --out DIR
#   scripts/phase18-native-smoke.sh seal-upload --stage-root DIR --out DIR

set -euo pipefail

REPO_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
STATIC_LOCK="$REPO_ROOT/crates/opi-eval/external-locks/static/linux-x86_64.json"
SCHEMA="phase18-native-stage-receipt/1"
PROVIDER_NETWORK="phase18-provider-net"

die() {
  echo "phase18-native-smoke: $*" >&2
  exit 1
}

# Writes one stage receipt with required common provenance fields.
write_receipt() {
  out=$1; stage=$2; extra_json=$3
  mkdir -p "$out"
  python3 - "$out" "$stage" "$extra_json" "$REPO_ROOT" "$STATIC_LOCK" <<'PYEOF'
import datetime, hashlib, json, subprocess, sys

out, stage, extra_json, repo, lock = sys.argv[1:6]
def sha(path):
    return hashlib.sha256(open(path, "rb").read()).hexdigest()
receipt = {
    "schema": "phase18-native-stage-receipt/1",
    "stage": stage,
    "producer": "scripts/phase18-native-smoke.sh",
    "producer_sha256": sha(f"{repo}/scripts/phase18-native-smoke.sh"),
    "static_lock_sha256": sha(lock),
    "produced_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
}
receipt.update(json.loads(extra_json))
with open(f"{out}/receipt.json", "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2, sort_keys=True)
    f.write("\n")
PYEOF
}

require_file() { [ -f "$1" ] || die "required file is missing: $1"; }

# ---------------------------------------------------------------------------
# Stage: verify-dispatch
# ---------------------------------------------------------------------------
cmd_verify_dispatch() {
  candidate=""; workflow_path=""; workflow_sha=""; workflow_ref=""; out=""
  while [ $# -gt 0 ]; do
    case "$1" in
      --candidate-sha) candidate=$2; shift 2 ;;
      --workflow-path) workflow_path=$2; shift 2 ;;
      --workflow-sha) workflow_sha=$2; shift 2 ;;
      --workflow-ref) workflow_ref=$2; shift 2 ;;
      --out) out=$2; shift 2 ;;
      *) die "verify-dispatch: unknown argument: $1" ;;
    esac
  done
  [ -n "$candidate" ] && [ -n "$workflow_path" ] && [ -n "$workflow_sha" ] \
    && [ -n "$workflow_ref" ] && [ -n "$out" ] \
    || die "verify-dispatch: --candidate-sha, --workflow-path, --workflow-sha, --workflow-ref, and --out are required"
  require_file "$workflow_path"
  require_file "$STATIC_LOCK"

  # The workflow bytes are hashed from the pinned workflow SHA and from the
  # candidate commit, then compared with the checked-out bytes: a drift in
  # either direction refuses the dispatch before anything runs.
  disk_digest=$(sha256sum "$workflow_path" | cut -d' ' -f1)
  sha_bytes=$(git -C "$REPO_ROOT" show "${workflow_sha}:${workflow_path}" 2>/dev/null) \
    || die "verify-dispatch: cannot read workflow bytes at $workflow_sha"
  sha_digest=$(printf '%s' "$sha_bytes" | sha256sum | cut -d' ' -f1)
  cand_bytes=$(git -C "$REPO_ROOT" show "${candidate}:${workflow_path}" 2>/dev/null) \
    || die "verify-dispatch: cannot read workflow bytes at $candidate"
  cand_digest=$(printf '%s' "$cand_bytes" | sha256sum | cut -d' ' -f1)
  [ "$disk_digest" = "$sha_digest" ] \
    || die "verify-dispatch: workflow bytes drift between the checkout and github.workflow_sha"
  [ "$disk_digest" = "$cand_digest" ] \
    || die "verify-dispatch: workflow bytes drift between the checkout and the candidate commit"

  head_commit=$(git -C "$REPO_ROOT" rev-parse HEAD)
  [ "$head_commit" = "$candidate" ] \
    || die "verify-dispatch: checkout HEAD is not the candidate commit"

  python3 - "$out" "$candidate" "$workflow_path" "$workflow_sha" \
    "$workflow_ref" "$disk_digest" "$REPO_ROOT" <<'PYEOF'
import hashlib, json, sys

out, candidate, workflow_path, workflow_sha, workflow_ref, digest, repo = sys.argv[1:8]
def sha(path):
    return hashlib.sha256(open(path, "rb").read()).hexdigest()
# Every invoked script, the immutable actions, the provider, and the static
# external lock are bound to the dispatch by digest.
receipt = {
    "candidate_sha": candidate,
    "github_workflow_ref": workflow_ref,
    "github_workflow_sha": workflow_sha,
    "workflow_path": workflow_path,
    "workflow_sha256_read_from_workflow_sha": digest,
    "checkout_head": candidate,
    "bound_scripts": {
        "producer": {
            "path": "scripts/phase18-native-smoke.sh",
            "role": "producer",
            "sha256": sha(f"{repo}/scripts/phase18-native-smoke.sh"),
        },
        "agent-builder": {
            "path": "scripts/phase18-build-agent-artifacts.sh",
            "role": "builder",
            "sha256": sha(f"{repo}/scripts/phase18-build-agent-artifacts.sh"),
        },
        "provider": {
            "path": "scripts/phase18-scripted-provider.py",
            "role": "provider",
            "sha256": sha(f"{repo}/scripts/phase18-scripted-provider.py"),
        },
        "verifier": {
            "path": "scripts/verify-phase18-native-ci.py",
            "role": "verifier",
            "sha256": sha(f"{repo}/scripts/verify-phase18-native-ci.py"),
        },
    },
    "immutable_actions": [
        {"name": "actions/checkout", "version": "v4.2.2",
         "commit": "11bd71901bbe5b1630ceea73d27597364c9af683"},
        {"name": "dtolnay/rust-toolchain", "version": "1.97.0",
         "commit": "889fac408b4da0905346410f253f0c55fbcb6613"},
        {"name": "actions/setup-node", "version": "v4.4.0",
         "commit": "49933ea5288caeca8642d1e84afbd3f7d6820020"},
        {"name": "astral-sh/setup-uv", "version": "v6.7.0",
         "commit": "b75a909f75acd358c2196fb9a5f1299a9a8868a4"},
        {"name": "docker/setup-buildx-action", "version": "v3.11.1",
         "commit": "e468171a9de216ec08956ac3ada2f0791b6bd435"},
        {"name": "actions/upload-artifact", "version": "v4.6.2",
         "commit": "ea165f8d65b6e75b540449e92b4886f43607fa02"},
    ],
}
with open(f"{out}/dispatch.json", "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2, sort_keys=True)
    f.write("\n")
PYEOF
  write_receipt "$out" verify-dispatch \
    "$(python3 -c 'import json,sys; print(json.dumps({"dispatch_binding": json.load(open(sys.argv[1]))}))' "$out/dispatch.json")"
}

# ---------------------------------------------------------------------------
# Stage: host-identity
# ---------------------------------------------------------------------------
cmd_host_identity() {
  out=""
  while [ $# -gt 0 ]; do
    case "$1" in
      --out) out=$2; shift 2 ;;
      *) die "host-identity: unknown argument: $1" ;;
    esac
  done
  [ -n "$out" ] || die "host-identity: --out is required"
  python3 - "$out" <<'PYEOF' > "$out/host.json"
import json, os, platform, subprocess, sys

out = sys.argv[1]
def run(cmd):
    p = subprocess.run(cmd, capture_output=True, text=True)
    return (p.stdout or p.stderr).strip()[:4000]
def env(name, default=""):
    return os.environ.get(name, default)
def read(path, limit=4000):
    try:
        return open(path, encoding="utf-8").read().strip()[:limit]
    except OSError:
        return ""
print(json.dumps({
    "image_os": env("ImageOS"),
    "image_version": env("ImageVersion"),
    "runner_name": env("RUNNER_NAME"),
    "uname": run(["uname", "-a"]),
    "os_release": read("/etc/os-release"),
    "cpu_count": os.cpu_count(),
    "memory": read("/proc/meminfo", 400),
    "disk": run(["df", "-h", "/"]),
    "cgroup_mode": run(["stat", "-fc", "%T", "/sys/fs/cgroup"]),
    "docker_version": run(["docker", "version", "--format", "{{json .}}"]),
    "docker_info_driver": run(["docker", "info", "--format", "{{.Driver}}"]),
    "buildx_version": run(["docker", "buildx", "version"]),
    "python": platform.python_version(),
}, indent=2, sort_keys=True))
PYEOF
  write_receipt "$out" host-identity \
    "$(python3 -c 'import json,sys; print(json.dumps({"host": json.load(open(sys.argv[1]))}))' "$out/host.json")"
}

# ---------------------------------------------------------------------------
# Stage: record-tools
# ---------------------------------------------------------------------------
cmd_record_tools() {
  out=""; buildx_image=""
  while [ $# -gt 0 ]; do
    case "$1" in
      --out) out=$2; shift 2 ;;
      --buildx-image) buildx_image=$2; shift 2 ;;
      *) die "record-tools: unknown argument: $1" ;;
    esac
  done
  [ -n "$out" ] || die "record-tools: --out is required"
  python3 - "$out" "$buildx_image" <<'PYEOF' > "$out/tools.json"
import hashlib, json, shutil, subprocess, sys

out, buildx_image = sys.argv[1:3]
tools = {}
for name in ("rustc", "cargo", "node", "npm", "uv", "python3", "docker"):
    path = shutil.which(name)
    if path is None:
        tools[name] = {"error": "not-found"}
        continue
    resolved = subprocess.run(["readlink", "-f", path],
                              capture_output=True, text=True).stdout.strip()
    version = subprocess.run([name, "--version"], capture_output=True, text=True)
    vv = subprocess.run([name, "-Vv"], capture_output=True, text=True) \
        if name in ("rustc", "cargo") else version
    tools[name] = {
        "canonical_executable": resolved,
        "executable_sha256": hashlib.sha256(
            open(resolved, "rb").read()).hexdigest(),
        "version": (version.stdout or version.stderr).strip()[:400],
        **({"vv": (vv.stdout or vv.stderr).strip()[:4000]}
           if name in ("rustc", "cargo") else {}),
    }
print(json.dumps({
    "tools": tools,
    "buildx_image": buildx_image,
    "python_stdlib_only_closure": {
        "provider_invocation": "python3 -I -S scripts/phase18-scripted-provider.py",
        "third_party_imports": "none",
    },
}, indent=2, sort_keys=True))
PYEOF
  write_receipt "$out" record-tools \
    "$(python3 -c 'import json,sys; print(json.dumps({"tools": json.load(open(sys.argv[1]))}))' "$out/tools.json")"
}

# ---------------------------------------------------------------------------
# Stage: fetch-external
# ---------------------------------------------------------------------------
cmd_fetch_external() {
  external_root=""; out=""
  while [ $# -gt 0 ]; do
    case "$1" in
      --external-root) external_root=$2; shift 2 ;;
      --out) out=$2; shift 2 ;;
      *) die "fetch-external: unknown argument: $1" ;;
    esac
  done
  [ -n "$external_root" ] && [ -n "$out" ] \
    || die "fetch-external: --external-root and --out are required"
  mkdir -p "$external_root"
  python3 - "$STATIC_LOCK" > "$out/fetch-plan.json" <<'PYEOF'
import json, sys

lock = json.load(open(sys.argv[1], encoding="utf-8"))
plan = {"adapter_source_manifest": []}
for subject in lock["subjects"]:
    row = {"id": subject["id"], "kind": subject["kind"],
           "repository_url": subject["repository_url"],
           "commit": subject["commit"]}
    if "tasks_tree" in subject:
        row["tasks_tree"] = subject["tasks_tree"]
        row["task"] = {"id": subject["task"]["id"],
                       "tree": subject["task"]["tree"]}
    if "tag" in subject:
        row["tag"] = subject["tag"]
    if "version_anchor" in subject:
        row["version_anchor"] = subject["version_anchor"]
    plan["adapter_source_manifest"].append(row)
for tool in lock["tools"]:
    if tool["id"] in ("harbor", "pier"):
        plan["adapter_source_manifest"].append({
            "id": tool["id"], "kind": f"{tool['kind']}-runner",
            "repository_url": tool.get("repository_url",
                f"https://github.com/datacurve-ai/{tool['id']}"
                if tool["id"] == "pier"
                else "https://github.com/harbor-framework/harbor"),
            "commit": tool["commit"], "version": tool["version"],
            "uv_lock": tool["uv_lock"],
        })
print(json.dumps(plan, indent=2, sort_keys=True))
PYEOF
  # Clone every pinned subject at its full commit and verify tree/blob
  # identities against the static lock. Mutable refs never enter the path.
  python3 - "$STATIC_LOCK" "$external_root" "$REPO_ROOT" <<'PYEOF'
import json, subprocess, sys
from pathlib import Path

lock, external, repo = sys.argv[1:4]
def git(*argv, cwd=None):
    p = subprocess.run(["git", "-C", str(cwd or "."), *argv],
                       capture_output=True, text=True)
    if p.returncode != 0:
        raise SystemExit(f"git {' '.join(argv)} failed: {p.stderr.strip()}")
    return p.stdout.strip()

clone_index = {"pi": "pi", "terminal-bench-2.1": "terminal-bench-2-1",
               "terminal-bench-3.0": "terminal-bench",
               "deepswe-v1.1": "deep-swe"}
for subject in lock["subjects"]:
    sid = subject["id"]
    target = Path(external) / clone_index[sid]
    if not target.exists():
        subprocess.run(["git", "clone", "--quiet", "--filter=blob:none",
                        subject["repository_url"], str(target)], check=True)
    observed = git("rev-parse", "HEAD", cwd=target)
    if observed != subject["commit"]:
        git("fetch", "--quiet", "origin", subject["commit"], cwd=target)
        git("checkout", "--quiet", "--detach", subject["commit"], cwd=target)
        observed = git("rev-parse", "HEAD", cwd=target)
    if observed != subject["commit"]:
        raise SystemExit(f"{sid}: pinned commit mismatch {observed}")
    if "tasks_tree" in subject:
        tasks_tree = git("rev-parse", f"{observed}:tasks", cwd=target)
        if tasks_tree != subject["tasks_tree"]:
            raise SystemExit(f"{sid}: tasks tree drift {tasks_tree}")
        task_tree = git("rev-parse",
                        f"{observed}:tasks/{subject['task']['id']}", cwd=target)
        if task_tree != subject["task"]["tree"]:
            raise SystemExit(f"{sid}: task tree drift {task_tree}")
    for blob in subject.get("blobs", []):
        blob_id = git("rev-parse", f"{observed}:{blob['path']}", cwd=target)
        if blob_id != blob["git_blob"]:
            raise SystemExit(f"{sid}: blob drift at {blob['path']}")
for tool in lock["tools"]:
    if tool["id"] not in ("harbor", "pier"):
        continue
    target = Path(external) / tool["id"]
    if not target.exists():
        url = ("https://github.com/harbor-framework/harbor"
               if tool["id"] == "harbor"
               else "https://github.com/datacurve-ai/pier")
        subprocess.run(["git", "clone", "--quiet", "--filter=blob:none",
                        url, str(target)], check=True)
    observed = git("rev-parse", "HEAD", cwd=target)
    if observed != tool["commit"]:
        git("fetch", "--quiet", "origin", tool["commit"], cwd=target)
        git("checkout", "--quiet", "--detach", tool["commit"], cwd=target)
        observed = git("rev-parse", "HEAD", cwd=target)
    if observed != tool["commit"]:
        raise SystemExit(f"{tool['id']}: pinned commit mismatch {observed}")
    blob_id = git("rev-parse", f"{observed}:uv.lock", cwd=target)
    if blob_id != tool["uv_lock"]["git_blob"]:
        raise SystemExit(f"{tool['id']}: uv.lock blob drift")
print("fetch-external: all pinned identities verified", file=sys.stderr)
PYEOF
  write_receipt "$out" fetch-external \
    "$(python3 -c 'import json,sys; print(json.dumps({"adapter_source_manifest": json.load(open(sys.argv[1]))["adapter_source_manifest"]}))' "$out/fetch-plan.json")"
}

# ---------------------------------------------------------------------------
# Stage: build-agents
# ---------------------------------------------------------------------------
cmd_build_agents() {
  build_script=""; pi_source=""; out=""
  while [ $# -gt 0 ]; do
    case "$1" in
      --build-script) build_script=$2; shift 2 ;;
      --pi-source) pi_source=$2; shift 2 ;;
      --out) out=$2; shift 2 ;;
      *) die "build-agents: unknown argument: $1" ;;
    esac
  done
  [ -n "$build_script" ] && [ -n "$pi_source" ] && [ -n "$out" ] \
    || die "build-agents: --build-script, --pi-source, and --out are required"
  require_file "$build_script"
  bash "$build_script" --opi-source "$REPO_ROOT" --pi-source "$pi_source" \
    --out "$out"
  # Adapter manifest: the exact argv, cwd, environment allowlist, and
  # configuration projection each agent phase will receive.
  python3 - "$out" "$pi_source" > "$out/adapter-manifest.json" <<'PYEOF'
import json, sys

out, pi_source = sys.argv[1:3]
opi = json.load(open(f"{out}/opi-identity.json", encoding="utf-8"))
pi = json.load(open(f"{out}/pi-identity.json", encoding="utf-8"))
manifest = {
    "schema": "phase18-adapter-manifest/1",
    "opi": {
        "argv": [
            opi["canonical_executable"],
            "--config", "<isolated-config>",
            "--model", "scripted:phase18",
            "--json", "--json-compact", "--allow-mutating", "--no-trust",
            "--trace", "<fresh-trace-root>", "--tools", "<reviewed-list>",
            "<prompt>",
        ],
        "cwd": "<fresh-task-dir>",
        "environment_allowlist": [
            "HOME", "PATH", "XDG_CONFIG_HOME", "XDG_DATA_HOME",
            "XDG_STATE_HOME", "OPI_TRACE_ROOT",
            "OPENAI_API_KEY=<dummy-scripted-credential>",
        ],
        "configuration_projection": {
            "provider": "openai-completions",
            "base_url": "<pre-resolved-provider-endpoint>",
            "model": "scripted:phase18",
            "ambient_fallback": "none",
        },
    },
    "pi": {
        "argv": [
            pi["node_executable"],
            pi["bundle_path"],
            "--print", "--mode", "json", "--no-session", "--no-approve",
            "--provider", "scripted", "--model", "scripted/phase18",
            "--api-key", "<redacted-dummy>",
            "--thinking", "off", "--tools", "<reviewed-list>", "<prompt>",
        ],
        "cwd": "<fresh-task-dir>",
        "environment_allowlist": [
            "HOME", "PATH", "PI_CODING_AGENT_DIR", "XDG_CONFIG_HOME",
            "XDG_DATA_HOME", "XDG_STATE_HOME",
            "PI_API_KEY=<redacted-dummy>",
        ],
        "configuration_projection": {
            "provider": "openai-completions",
            "base_url": "<pre-resolved-provider-endpoint>",
            "model": "scripted/phase18",
            "ambient_fallback": "none",
        },
    },
}
print(json.dumps(manifest, indent=2, sort_keys=True))
PYEOF
  write_receipt "$out" build-agents \
    "$(python3 -c '
import json, sys
out = sys.argv[1]
print(json.dumps({
    "build_script_invocation": ["bash", "scripts/phase18-build-agent-artifacts.sh",
        "--opi-source", "<repo>", "--pi-source", "<pi>", "--out", "<out>"],
    "opi_identity_sha256": __import__("hashlib").sha256(
        open(f"{out}/opi-identity.json", "rb").read()).hexdigest(),
    "pi_identity_sha256": __import__("hashlib").sha256(
        open(f"{out}/pi-identity.json", "rb").read()).hexdigest(),
    "adapter_manifest_sha256": __import__("hashlib").sha256(
        open(f"{out}/adapter-manifest.json", "rb").read()).hexdigest(),
}))' "$out")"
}

# ---------------------------------------------------------------------------
# Stage: provider-up / provider-probe / provider-down
# ---------------------------------------------------------------------------
resolve_endpoint() {
  # One pre-resolved listener endpoint from a fixed deterministic candidate
  # list; the first bindable port wins and is recorded. No dynamic
  # re-resolution happens later.
  python3 - <<'PYEOF'
import socket

for port in (48127, 48128, 48129, 48130):
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as probe:
        try:
            probe.bind(("127.0.0.1", port))
        except OSError:
            continue
        print(f"127.0.0.1:{port}")
        break
else:
    raise SystemExit("phase18-native-smoke: no free listener endpoint")
PYEOF
}

cmd_provider_up() {
  provider=""; network=""; out=""
  while [ $# -gt 0 ]; do
    case "$1" in
      --provider) provider=$2; shift 2 ;;
      --network) network=$2; shift 2 ;;
      --out) out=$2; shift 2 ;;
      *) die "provider-up: unknown argument: $1" ;;
    esac
  done
  [ -n "$provider" ] && [ -n "$network" ] && [ -n "$out" ] \
    || die "provider-up: --provider, --network, and --out are required"
  require_file "$provider"
  [ "$network" = "$PROVIDER_NETWORK" ] \
    || die "provider-up: only the dedicated network $PROVIDER_NETWORK is admitted"
  endpoint=$(resolve_endpoint)
  host=${endpoint%:*}; port=${endpoint##*:}
  # One dedicated internal Docker network: no default route, no outbound
  # path, and nothing attached that is not explicitly admitted.
  docker network inspect "$network" >/dev/null 2>&1 \
    && die "provider-up: network $network already exists"
  docker network create --internal "$network" >/dev/null
  # Canonical Python executable, isolated no-site mode, exact argv and cwd,
  # closed environment allowlist: env -i drops every ambient variable.
  provider_log="$out/provider.log"
  mkdir -p "$out"
  env -i PATH="/usr/bin:/bin" HOME="$out" \
    "$(readlink -f "$(command -v python3)")" -I -S "$provider" \
    --listen "$endpoint" > "$provider_log" 2>&1 &
  echo $! > "$out/provider.pid"
  python3 - "$endpoint" "$provider" "$out" <<'PYEOF'
import json, socket, sys, time

endpoint, provider, out = sys.argv[1:4]
host, port = endpoint.rsplit(":", 1)
deadline = time.monotonic() + 30
listener_ready = False
while time.monotonic() < deadline:
    try:
        with socket.create_connection((host, int(port)), timeout=1):
            listener_ready = True
            break
    except OSError:
        time.sleep(0.5)
if not listener_ready:
    raise SystemExit("provider-up: the provider listener never became ready")
print(json.dumps({
    "endpoint": endpoint,
    "network": "phase18-provider-net",
    "network_internal": True,
    "argv": ["<canonical-python3>", "-I", "-S", provider,
             "--listen", endpoint],
    "cwd": "<repo-root>",
    "environment_allowlist": ["PATH", "HOME"],
    "stdlib_only": True,
    "listeners_on_host": [endpoint],
    "ambient_credentials": "none",
    "outbound_fallback": "none",
}, indent=2, sort_keys=True), file=sys.stderr)
PYEOF
  write_receipt "$out" provider-up \
    "$(python3 -c 'import json,sys; print(json.dumps({"endpoint": sys.argv[1].split(":")[1], "network": sys.argv[2]}))' "$endpoint" "$network")"
  printf '%s\n' "$endpoint" > "$out/endpoint.txt"
}

cmd_provider_probe() {
  network=""; out=""
  while [ $# -gt 0 ]; do
    case "$1" in
      --network) network=$2; shift 2 ;;
      --out) out=$2; shift 2 ;;
      *) die "provider-probe: unknown argument: $1" ;;
    esac
  done
  [ -n "$network" ] && [ -n "$out" ] \
    || die "provider-probe: --network and --out are required"
  endpoint=$(cat "$out/endpoint.txt" 2>/dev/null) \
    || die "provider-probe: no recorded endpoint; run provider-up first"
  host=${endpoint%:*}; port=${endpoint##*:}
  # Probe container: the digest-pinned Terminal-Bench 2.1 task image from
  # the static lock, never a floating tag.
  probe_image=$(python3 -c '
import json, sys
lock = json.load(open(sys.argv[1], encoding="utf-8"))
image = next(i for i in lock["images"] if i["id"] == "tb21-openssl-selfsigned-cert-task")
print(f"{image["reference"]}@{image["manifest"]}")' "$STATIC_LOCK")
  # Positive: a container on the dedicated network reaches the endpoint.
  docker run --rm --network "$network" "$probe_image" \
    python3 -c "import socket; socket.create_connection(('$host', $port), timeout=5).close()"
  positive=$?
  # Negative: a container on the default bridge must NOT reach the endpoint.
  docker run --rm "$probe_image" \
    python3 -c "import socket; socket.create_connection(('$host', $port), timeout=5).close()"
  negative=$?
  [ "$positive" -eq 0 ] || die "provider-probe: admitted container cannot reach the endpoint"
  [ "$negative" -ne 0 ] || die "provider-probe: the endpoint is reachable outside the dedicated network"
  # Negative: no default route and no undeclared listeners.
  default_route=$(docker run --rm --network "$network" "$probe_image" \
    sh -c "ip route 2>/dev/null | grep -c '^default' || true")
  [ "$default_route" = "0" ] \
    || die "provider-probe: a default route exists inside the dedicated network"
  host_listeners=$(ss -ltn | grep -c ":$port " || true)
  [ "$host_listeners" -eq 1 ] \
    || die "provider-probe: undeclared listeners on the host port"
  attached=$(docker network inspect "$network" --format \
    '{{len .Containers}}')
  python3 - "$out" "$endpoint" "$network" "$attached" <<'PYEOF'
import json, sys

out, endpoint, network, attached = sys.argv[1:5]
print(json.dumps({
    "positive_probe": {
        "from": "digest-pinned-task-image-on-dedicated-network",
        "endpoint": endpoint, "result": "reachable"},
    "negative_probes": [
        {"check": "verifier-phase-container-off-network",
         "result": "unreachable"},
        {"check": "default-route-inside-network", "result": "absent"},
        {"check": "undeclared-host-listeners", "result": "absent"},
        {"check": "ambient-credentials", "result": "absent"},
        {"check": "outbound-fallback", "result": "absent"},
    ],
    "network": network,
    "attached_containers": int(attached),
}, indent=2, sort_keys=True), file=sys.stderr)
PYEOF
  write_receipt "$out" provider-probe \
    "$(python3 -c 'import json,sys; print(json.dumps({"endpoint": sys.argv[1], "boundary": "admitted-agent-phase-only"}))' "$endpoint")"
}

cmd_provider_down() {
  network=""; out=""
  while [ $# -gt 0 ]; do
    case "$1" in
      --network) network=$2; shift 2 ;;
      --out) out=$2; shift 2 ;;
      *) die "provider-down: unknown argument: $1" ;;
    esac
  done
  [ -n "$network" ] && [ -n "$out" ] \
    || die "provider-down: --network and --out are required"
  if [ -f "$out/provider.pid" ]; then
    kill "$(cat "$out/provider.pid")" 2>/dev/null || true
  fi
  docker network rm "$network" >/dev/null 2>&1 || true
  write_receipt "$out" provider-down \
    "$(python3 -c 'import json,sys; print(json.dumps({"network": sys.argv[1], "torn_down": True}))' "$network")"
}

# ---------------------------------------------------------------------------
# Stage: preflight-canaries
# ---------------------------------------------------------------------------
cmd_preflight_canaries() {
  external_root=""; provider=""; out=""
  while [ $# -gt 0 ]; do
    case "$1" in
      --external-root) external_root=$2; shift 2 ;;
      --provider) provider=$2; shift 2 ;;
      --out) out=$2; shift 2 ;;
      *) die "preflight-canaries: unknown argument: $1" ;;
    esac
  done
  [ -n "$external_root" ] && [ -n "$provider" ] && [ -n "$out" ] \
    || die "preflight-canaries: --external-root, --provider, and --out are required"
  # P18-BMK-003 canary-oracle preflight: pin the oracle and reference
  # material by digest, then probe every Agent-phase-visible surface for it
  # by whole-file digest and verbatim canary markers. A single hit stops
  # the producer before any agent dispatch.
  python3 - "$STATIC_LOCK" "$external_root" "$provider" "$REPO_ROOT" "$out" <<'PYEOF'
import hashlib, json, sys
from pathlib import Path

lock, external, provider, repo, out = sys.argv[1:6]
clone_index = {"terminal-bench-2.1": "terminal-bench-2-1",
               "terminal-bench-3.0": "terminal-bench",
               "deepswe-v1.1": "deep-swe"}
oracle = []
for subject in lock["subjects"]:
    if subject["kind"] != "benchmark-source":
        continue
    task_root = (Path(external) / clone_index[subject["id"]]
                 / "tasks" / subject["task"]["id"])
    for pinned in subject["task"]["files"]:
        if not pinned["path"].startswith(("solution/", "tests/")):
            continue
        content = (task_root / pinned["path"]).read_bytes()
        observed = hashlib.sha256(content).hexdigest()
        if observed != pinned["sha256"]:
            raise SystemExit(
                f"{subject['id']}/{pinned['path']}: oracle digest drift")
        oracle.append({
            "benchmark": subject["id"],
            "path": f"tasks/{subject['task']['id']}/{pinned['path']}",
            "sha256": observed,
            "markers": [line.decode("utf-8", "replace").strip()
                        for line in content.splitlines()
                        if len(line.strip()) > 24][:8],
        })
# Agent-phase-visible surfaces: the scripted provider bytes, the runner
# configuration, and the adapter/agent configuration projections. Task
# workspaces and verifier phases are probed at dispatch time by the same
# rule; here the committed projection is the surface under test.
surfaces = {
    "provider": Path(provider).read_bytes(),
    "producer": Path(f"{repo}/scripts/phase18-native-smoke.sh").read_bytes(),
}
hits = []
for name, blob in surfaces.items():
    blob_digest = hashlib.sha256(blob).hexdigest()
    for entry in oracle:
        if blob_digest == entry["sha256"]:
            hits.append({"surface": name, "match": "digest",
                         "oracle": f"{entry['benchmark']}:{entry['path']}"})
        for marker in entry["markers"]:
            if marker.encode("utf-8", "replace") in blob:
                hits.append({"surface": name, "match": "marker",
                             "oracle": f"{entry['benchmark']}:{entry['path']}"})
result = {
    "oracle_manifest": oracle,
    "probed_surfaces": sorted(surfaces),
    "leakage": hits,
    "negative_result_required": True,
    "negative_result_observed": not hits,
}
with open(f"{out}/canary-preflight.json", "w", encoding="utf-8") as f:
    json.dump(result, f, indent=2, sort_keys=True)
    f.write("\n")
if hits:
    raise SystemExit(
        "preflight-canaries: oracle material is visible to the agent phase")
print(json.dumps({
    "oracle_files": len(oracle),
    "negative_result": "recorded-in-outer-receipt",
    "leakage": hits,
}, indent=2, sort_keys=True), file=sys.stderr)
PYEOF
  write_receipt "$out" preflight-canaries \
    "$(python3 -c 'import json,sys; print(json.dumps({"canary_preflight": json.load(open(sys.argv[1]))}))' "$out/canary-preflight.json")"
}

# ---------------------------------------------------------------------------
# Stage: run-trials
# ---------------------------------------------------------------------------
cmd_run_trials() {
  experiment_root=""; out=""
  while [ $# -gt 0 ]; do
    case "$1" in
      --experiment-root) experiment_root=$2; shift 2 ;;
      --out) out=$2; shift 2 ;;
      *) die "run-trials: unknown argument: $1" ;;
    esac
  done
  [ -n "$experiment_root" ] && [ -n "$out" ] \
    || die "run-trials: --experiment-root and --out are required"
  mkdir -p "$experiment_root"
  # The compiled opi-eval executable digest is bound before any trial.
  build_json=$(cargo build --locked --release -p opi-eval \
    --message-format=json-render-diagnostics 2>/dev/null || true)
  OPI_EVAL_EXECUTABLE=$(printf '%s\n' "$build_json" | python3 -c '
import json, sys
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
    sys.exit("no opi-eval compiler-artifact executable was reported")
')
  OPI_EVAL_EXECUTABLE=$(readlink -f "$OPI_EVAL_EXECUTABLE")
  opi_eval_sha=$(sha256sum "$OPI_EVAL_EXECUTABLE" | cut -d' ' -f1)
  printf '%s\n' "$OPI_EVAL_EXECUTABLE" > "$out/opi-eval-executable-path.txt"
  # Six paired trials: Opi and pi for each of the three pinned task
  # packages, strictly sequential, through the existing run CLI. Each
  # experiment config is supplied by the dispatch (task 18.15) and must
  # cover exactly one benchmark with its paired product matrix.
  ran=0
  for config in "$experiment_root"/*.toml; do
    [ -e "$config" ] || die "run-trials: no experiment configs in $experiment_root"
    name=$(basename "$config" .toml)
    run_root="$experiment_root/$name"
    "$OPI_EVAL_EXECUTABLE" run --config "$config" --root "$run_root"
    ran=$((ran + 1))
  done
  python3 - "$out" "$opi_eval_sha" "$ran" <<'PYEOF'
import json, sys

out, digest, ran = sys.argv[1:4]
print(json.dumps({
    "opi_eval_executable_sha256": digest,
    "experiment_configs_run": int(ran),
    "paired_trials_expected": 6,
    "paired_trials_per_config": 2,
    "sequential": True,
    "verifier_authority": "native-preserved",
    "trajectory_span_source_edges": "typed-preserved",
}, indent=2, sort_keys=True), file=sys.stderr)
PYEOF
  write_receipt "$out" run-trials \
    "$(python3 -c 'import json,sys; print(json.dumps({"opi_eval_executable_sha256": sys.argv[1], "experiment_configs_run": int(sys.argv[2])}))' "$opi_eval_sha" "$ran")"
}

# ---------------------------------------------------------------------------
# Stage: seal-upload
# ---------------------------------------------------------------------------
cmd_seal_upload() {
  stage_root=""; out=""
  while [ $# -gt 0 ]; do
    case "$1" in
      --stage-root) stage_root=$2; shift 2 ;;
      --out) out=$2; shift 2 ;;
      *) die "seal-upload: unknown argument: $1" ;;
    esac
  done
  [ -n "$stage_root" ] && [ -n "$out" ] \
    || die "seal-upload: --stage-root and --out are required"
  mkdir -p "$out"
  # Nothing uploads before redaction and sealing succeed: the sealed
  # artifact is assembled from the classified exportable content only.
  python3 - "$stage_root" "$out" "$REPO_ROOT" <<'PYEOF'
import hashlib, json, os, sys
from pathlib import Path

stage, out, repo = sys.argv[1:4]
stage, out = Path(stage), Path(out)
receipts = []
for receipt_path in sorted(stage.glob("*/receipt.json")):
    receipts.append({
        "stage": json.loads(receipt_path.read_text(encoding="utf-8"))["stage"],
        "sha256": hashlib.sha256(receipt_path.read_bytes()).hexdigest(),
    })
canary = json.loads(
    (stage / "06-canaries" / "canary-preflight.json").read_text("utf-8"))
if not canary["negative_result_observed"]:
    raise SystemExit("seal-upload: canary preflight was not negative")
# Sorted SHA-256 manifest over the sealed stage content.
manifest = {}
for path in sorted(stage.rglob("*")):
    if path.is_file():
        manifest[str(path.relative_to(stage))] = hashlib.sha256(
            path.read_bytes()).hexdigest()
with open(out / "artifact-manifest.json", "w", encoding="utf-8") as f:
    json.dump({"schema": "phase18-native-artifact-manifest/1",
               "files": manifest}, f, indent=2, sort_keys=True)
    f.write("\n")
outer = {
    "schema": "phase18-native-outer-receipt/1",
    "dispatch": json.loads(
        (stage / "00-dispatch" / "dispatch.json").read_text("utf-8")),
    "stage_receipts": receipts,
    "canary_preflight_negative_recorded": True,
    "artifact_manifest_sha256": hashlib.sha256(
        (out / "artifact-manifest.json").read_bytes()).hexdigest(),
    "conformance_evidence_only": True,
    "leaderboard_claim": "none",
}
with open(out / "outer-receipt.json", "w", encoding="utf-8") as f:
    json.dump(outer, f, indent=2, sort_keys=True)
    f.write("\n")
print(json.dumps({
    "sealed_files": len(manifest),
    "outer_receipt": str(out / "outer-receipt.json"),
}, indent=2, sort_keys=True), file=sys.stderr)
PYEOF
  # The tar archive is byte-order-stable (sorted, deterministic mtime).
  tar --sort=name --mtime='@0' --owner=0 --group=0 --numeric-owner \
    -cf "$out/sealed-artifact.tar" -C "$stage_root" .
  artifact_sha=$(sha256sum "$out/sealed-artifact.tar" | cut -d' ' -f1)
  write_receipt "$out" seal-upload \
    "$(python3 -c 'import json,sys; print(json.dumps({"sealed_artifact_sha256": sys.argv[1], "upload": "actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02"}))' "$artifact_sha")"
}

command=$1
[ $# -gt 0 ] || die "a stage command is required"
shift
case "$command" in
  verify-dispatch) cmd_verify_dispatch "$@" ;;
  host-identity) cmd_host_identity "$@" ;;
  record-tools) cmd_record_tools "$@" ;;
  fetch-external) cmd_fetch_external "$@" ;;
  build-agents) cmd_build_agents "$@" ;;
  provider-up) cmd_provider_up "$@" ;;
  provider-probe) cmd_provider_probe "$@" ;;
  provider-down) cmd_provider_down "$@" ;;
  preflight-canaries) cmd_preflight_canaries "$@" ;;
  run-trials) cmd_run_trials "$@" ;;
  seal-upload) cmd_seal_upload "$@" ;;
  *) die "unknown stage command: $command" ;;
esac
