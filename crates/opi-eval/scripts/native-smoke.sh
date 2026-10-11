#!/usr/bin/env bash
# opi-eval Linux x86_64 native-smoke producer.
#
# The sole committed producer contract for the opi-eval native smoke. Every
# stage writes a typed stage receipt (schema `opi-eval-native-stage-receipt/1`)
# and fails closed on drift: the dispatch stage binds github.workflow_ref,
# github.workflow_sha, the SHA-256 of the workflow bytes read from that
# workflow SHA (never the mutable working tree), the candidate_sha, every
# invoked script, the immutable action pins, and the static external lock;
# later stages bind agent, tool, provider, network, canary, trial, and
# artifact identities to it.
#
# The native-smoke producer contract commits and statically verifies this contract; it does not
# claim that the six native trials have run. The native smoke dispatches it
# against resolved material. The provider never expands authority: no
# mutable refs, no ambient credentials, no undeclared listeners, no
# default routes, no outbound fallback, no fallback grader.
#
# Usage (one stage per sequential workflow step):
#   crates/opi-eval/scripts/native-smoke.sh verify-dispatch \
#     --candidate-sha SHA --workflow-path PATH --workflow-sha SHA \
#     --workflow-ref REF --out DIR
#   crates/opi-eval/scripts/native-smoke.sh host-identity --out DIR
#   crates/opi-eval/scripts/native-smoke.sh record-tools --out DIR [--buildx-image DESC]
#   crates/opi-eval/scripts/native-smoke.sh fetch-external --external-root DIR --out DIR
#   crates/opi-eval/scripts/native-smoke.sh build-agents \
#     --build-script PATH --pi-source DIR --out DIR
#   crates/opi-eval/scripts/native-smoke.sh provider-up \
#     --provider PATH --network NAME --out DIR
#   crates/opi-eval/scripts/native-smoke.sh provider-probe --network NAME --out DIR
#   crates/opi-eval/scripts/native-smoke.sh provider-down --network NAME --out DIR
#   crates/opi-eval/scripts/native-smoke.sh preflight-canaries \
#     --external-root DIR --provider PATH --out DIR
#   crates/opi-eval/scripts/native-smoke.sh run-trials \
#     --experiment-root DIR --out DIR
#   crates/opi-eval/scripts/native-smoke.sh seal-upload --stage-root DIR --out DIR
#   crates/opi-eval/scripts/native-smoke.sh record-upload-identity \
#     --seal-out DIR --artifact-id ID --artifact-url URL \
#     --artifact-digest SHA256 --run-id ID --run-url URL \
#     --retention-days N --out DIR

set -euo pipefail

REPO_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd)
STATIC_LOCK="$REPO_ROOT/crates/opi-eval/external-locks/static/linux-x86_64.json"
SCHEMA="opi-eval-native-stage-receipt/1"
PROVIDER_NETWORK="opi-eval-provider-net"

die() {
  echo "opi-eval-native-smoke: $*" >&2
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
    "schema": "opi-eval-native-stage-receipt/1",
    "stage": stage,
    "producer": "crates/opi-eval/scripts/native-smoke.sh",
    "producer_sha256": sha(f"{repo}/crates/opi-eval/scripts/native-smoke.sh"),
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
  mkdir -p "$out"
  require_file "$workflow_path"
  require_file "$STATIC_LOCK"

  # The workflow bytes are hashed from the pinned workflow SHA and from the
  # candidate commit, then compared with the checked-out bytes: a drift in
  # either direction refuses the dispatch before anything runs.
  # Digests stream straight from git into sha256sum: command
  # substitution would strip the trailing newline and the byte-drift
  # gate would then reject every newline-terminated workflow file.
  disk_digest=$(sha256sum "$workflow_path" | cut -d' ' -f1)
  sha_digest=$(git -C "$REPO_ROOT" show "${workflow_sha}:${workflow_path}" \
    2>/dev/null | sha256sum | cut -d' ' -f1)
  [ -n "$sha_digest" ] \
    || die "verify-dispatch: cannot read workflow bytes at $workflow_sha"
  cand_digest=$(git -C "$REPO_ROOT" show "${candidate}:${workflow_path}" \
    2>/dev/null | sha256sum | cut -d' ' -f1)
  [ -n "$cand_digest" ] \
    || die "verify-dispatch: cannot read workflow bytes at $candidate"
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
            "path": "crates/opi-eval/scripts/native-smoke.sh",
            "role": "producer",
            "sha256": sha(f"{repo}/crates/opi-eval/scripts/native-smoke.sh"),
        },
        "agent-builder": {
            "path": "crates/opi-eval/scripts/build-agent-artifacts.sh",
            "role": "builder",
            "sha256": sha(f"{repo}/crates/opi-eval/scripts/build-agent-artifacts.sh"),
        },
        "provider": {
            "path": "crates/opi-eval/scripts/scripted-provider.py",
            "role": "provider",
            "sha256": sha(f"{repo}/crates/opi-eval/scripts/scripted-provider.py"),
        },
        "verifier": {
            "path": "crates/opi-eval/scripts/verify-native-smoke-ci.py",
            "role": "verifier",
            "sha256": sha(f"{repo}/crates/opi-eval/scripts/verify-native-smoke-ci.py"),
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
  mkdir -p "$out"
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
  mkdir -p "$out"
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
        "provider_invocation": "python3 -I -S crates/opi-eval/scripts/scripted-provider.py",
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

lock_path, external, repo = sys.argv[1:4]
lock = json.load(open(lock_path, encoding="utf-8"))
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
  opi_commit=$(git -C "$REPO_ROOT" rev-parse HEAD)
  bash "$build_script" --opi-source "$REPO_ROOT" --opi-commit "$opi_commit" --pi-source "$pi_source" \
    --static-lock "$STATIC_LOCK" --out "$out"
  # Adapter manifest: the exact argv, cwd, environment allowlist, and
  # configuration projection each agent phase will receive.
  python3 - "$out" "$pi_source" > "$out/adapter-manifest.json" <<'PYEOF'
import json, sys

out, pi_source = sys.argv[1:3]
opi = json.load(open(f"{out}/opi-identity.json", encoding="utf-8"))
pi = json.load(open(f"{out}/pi-identity.json", encoding="utf-8"))
manifest = {
    "schema": "opi-eval-adapter-manifest/1",
    "opi": {
        "argv": [
            opi["canonical_executable"],
            "--config", "<isolated-config>",
            "--model", "scripted:eval",
            "--json", "--json-compact", "--allow-mutating", "--no-trust",
            "--trace", "<fresh-trace-root>", "--tools", "<reviewed-list>",
            "<prompt>",
        ],
        "cwd": "<fresh-task-dir>",
        "environment_allowlist": [
            "HOME", "PATH", "XDG_CONFIG_HOME", "XDG_DATA_HOME",
            "XDG_STATE_HOME", "OPI_TRACE_ROOT",
            "OPI_EVAL_SCRIPTED_KEY=<dummy-scripted-credential>",
        ],
        "configuration_projection": {
            "selected_connection": "scripted-local",
            "api": "openai-completions",
            "base_url": "<pre-resolved-provider-endpoint>",
            "model": "scripted:eval",
            "credential": "environment reference resolved at launch",
            "ambient_fallback": "none",
        },
    },
    "pi": {
        "argv": [
            pi["node_executable"],
            pi["bundle_path"],
            "--print", "--mode", "json", "--no-session", "--no-approve",
            "--provider", "scripted", "--model", "eval",
            "--thinking", "off", "--tools", "<reviewed-list>", "<prompt>",
        ],
        "cwd": "<fresh-task-dir>",
        "environment_allowlist": [
            "HOME", "PATH", "PI_CODING_AGENT_DIR", "XDG_CONFIG_HOME",
            "XDG_DATA_HOME", "XDG_STATE_HOME",
        ],
        "configuration_projection": {
            "selected_connection": "scripted-local",
            "api": "openai-completions",
            "base_url": "<pre-resolved-provider-endpoint>",
            "model": "scripted:eval",
            "credential": "inline models.json value projected at launch",
            "ambient_fallback": "none",
        },
    },
}
print(json.dumps(manifest, indent=2, sort_keys=True))
PYEOF
  write_receipt "$out" build-agents \
    "$(python3 -c '
import json, sys
out, build_script, repo, pi_source, static_lock = sys.argv[1:6]
print(json.dumps({
    "build_script_invocation": ["bash", build_script,
        "--opi-source", repo, "--opi-commit", json.load(open(f"{out}/opi-identity.json"))["checkout_commit"], "--pi-source", pi_source,
        "--static-lock", static_lock, "--out", out],
    "opi_identity_sha256": __import__("hashlib").sha256(
        open(f"{out}/opi-identity.json", "rb").read()).hexdigest(),
    "pi_identity_sha256": __import__("hashlib").sha256(
        open(f"{out}/pi-identity.json", "rb").read()).hexdigest(),
    "adapter_manifest_sha256": __import__("hashlib").sha256(
        open(f"{out}/adapter-manifest.json", "rb").read()).hexdigest(),
}))' "$out" "$build_script" "$REPO_ROOT" "$pi_source" "$STATIC_LOCK")"
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
    raise SystemExit("opi-eval-native-smoke: no free listener endpoint")
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
    --listen "$endpoint" --request-log "$out/requests.jsonl" \
    > "$provider_log" 2>&1 &
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
    "network": "opi-eval-provider-net",
    "network_internal": True,
    "argv": ["<canonical-python3>", "-I", "-S", provider,
             "--listen", endpoint,
             "--request-log", f"{out}/requests.jsonl"],
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
  # The agent execution surface of this contract is host-side: the
  # material wrappers exec the exact built programs on the host, so the
  # provider loopback endpoint is the only admitted surface. A container
  # on the dedicated network has its own loopback namespace and must NOT
  # be able to masquerade as the agent surface.
  # Positive: the loopback endpoint answers where the agents run.
  python3 -c "import socket; socket.create_connection(('$host', $port), timeout=5).close()"
  positive=$?
  # Negative: the provider must not answer on any non-loopback host
  # interface, so no off-host or cross-bridge phase can reach it.
  # UDP connect only picks a route; no packet leaves the host.
  host_ip=$(python3 -c '
import socket
try:
    s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    s.connect(("8.8.8.8", 80))
    print(s.getsockname()[0])
except OSError:
    pass')
  if [ -n "$host_ip" ] && [ "$host_ip" != "$host" ]; then
    if python3 -c "import socket; socket.create_connection(('$host_ip', $port), timeout=5).close()"; then
      die "provider-probe: the endpoint answers on a non-loopback interface"
    fi
  fi
  [ "$positive" -eq 0 ] \
    || die "provider-probe: the agent-phase surface cannot reach the endpoint"
  # Negative: the dedicated network is internal by construction and no
  # undeclared listener exists on the host.
  network_internal=$(docker network inspect "$network" --format '{{.Internal}}')
  [ "$network_internal" = "true" ] \
    || die "provider-probe: the dedicated network is not internal"
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
        "from": "host-loopback-agent-phase-surface",
        "endpoint": endpoint, "result": "reachable"},
    "negative_probes": [
        {"check": "non-loopback-host-interface", "result": "unreachable"},
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
  mkdir -p "$out"
  # EVAL-BMK-003 canary-oracle preflight: pin the oracle and reference
  # material by digest, then probe every Agent-phase-visible surface for it
  # by whole-file digest and verbatim canary markers. A single hit stops
  # the producer before any agent dispatch.
  python3 - "$STATIC_LOCK" "$external_root" "$provider" "$REPO_ROOT" "$out" <<'PYEOF'
import hashlib, json, sys
from pathlib import Path

lock_path, external, provider, repo, out = sys.argv[1:6]
lock = json.load(open(lock_path, encoding="utf-8"))
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
    "producer": Path(f"{repo}/crates/opi-eval/scripts/native-smoke.sh").read_bytes(),
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
# The declared canary marker list the trial stage hands to the runner's
# pre-seal scan: every pinned oracle marker becomes a sealing canary.
with open(f"{out}/canary-markers.txt", "w", encoding="utf-8") as f:
    for entry in oracle:
        for marker in entry["markers"]:
            if marker:
                f.write(marker + "\n")
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
# Stage: materialize-configs
# ---------------------------------------------------------------------------
resolve_opi_eval_executable() {
  # The compiled opi-eval executable, selected from the compiler-artifact
  # stream (never an assumed target path). A cargo failure is surfaced,
  # never swallowed silently: the exit status and the stderr tail ride
  # along with the rejection.
  local build_json cargo_rc
  build_json=$(cargo build --locked --release -p opi-eval \
    --message-format=json-render-diagnostics 2>/tmp/opi-eval-opi-eval-build.err) \
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
        tail = open("/tmp/opi-eval-opi-eval-build.err", encoding="utf-8"
                    ).read().strip()[-1500:]
    except OSError:
        pass
    sys.exit(f"no opi-eval compiler-artifact executable was reported "
             f"(cargo rc={os.environ.get("CARGO_RC")}); stderr tail: {tail}")
'
}

cmd_materialize_configs() {
  external_root=""; agents_out=""; provider_out=""; out=""
  while [ $# -gt 0 ]; do
    case "$1" in
      --external-root) external_root=$2; shift 2 ;;
      --agents-out) agents_out=$2; shift 2 ;;
      --provider-out) provider_out=$2; shift 2 ;;
      --out) out=$2; shift 2 ;;
      *) die "materialize-configs: unknown argument: $1" ;;
    esac
  done
  [ -n "$external_root" ] && [ -n "$agents_out" ] && [ -n "$provider_out" ] \
    && [ -n "$out" ] \
    || die "materialize-configs: --external-root, --agents-out, --provider-out, and --out are required"
  OPI_EVAL_EXECUTABLE=$(resolve_opi_eval_executable \
    || die "materialize-configs: opi-eval executable not resolved")
  OPI_EVAL_EXECUTABLE=$(readlink -f "$OPI_EVAL_EXECUTABLE")
  # CI delegates task/image/profile/runner-material production to the
  # same public local preparation path local operators use: the already
  # verified external checkouts, built agents, and repository pins are
  # transported into the admitted pinned cache contract, and one
  # `opi-eval prepare` per benchmark family produces the prepared
  # material. CI keeps no independent transform or admission rule; any
  # shared prepare rejection stops materialization before any material
  # is published.
  python3 - "$STATIC_LOCK" "$external_root" "$agents_out" "$provider_out" \
    "$out" "$REPO_ROOT" "$OPI_EVAL_EXECUTABLE" "$PROVIDER_NETWORK" <<'MATEOF'
import hashlib, json, os, re, shutil, subprocess, sys, tomllib
from pathlib import Path

lock_path, external, agents_out, provider_out, out, repo, opi_eval, network = sys.argv[1:9]
lock = json.loads(Path(lock_path).read_text(encoding="utf-8"))
out = Path(out)
repo = Path(repo)

def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()

def write_exec(path, body):
    Path(path).write_text(body, encoding="utf-8")
    os.chmod(path, 0o755)
    return sha(path)

clone_index = {"terminal-bench-2.1": "terminal-bench-2-1",
               "terminal-bench-3.0": "terminal-bench",
               "deepswe-v1.1": "deep-swe"}
adapters = ("terminal-bench-2.1", "terminal-bench-3.0", "deepswe-v1.1")
task_ids = {}
for subject in lock["subjects"]:
    if subject["kind"] == "benchmark-source":
        task_ids[subject["id"]] = subject["task"]["id"]

endpoint = Path(provider_out, "endpoint.txt").read_text(encoding="utf-8").strip()
base_url = f"http://{endpoint}"

# 1. Transport the admitted pinned cache contract: repository pins
#    (static lock, producer scripts, benchmark profiles, integrations),
#    the fetched blob-verified corpora/runner checkouts, the pinned uv,
#    and the exact built client bytes. Nothing is transformed here; the
#    identities are exactly the stage inputs the local preparation
#    contract admits.
cache = out / "cache"
(cache / "pins" / "static").mkdir(parents=True)
shutil.copy2(lock_path, cache / "pins" / "static" / "linux-x86_64.json")
(cache / "pins" / "scripts").mkdir(parents=True)
for name in ("materialize-external-locks.sh", "verify-external-lock-ci.py",
             "build-agent-artifacts.sh", "scripted-provider.py"):
    source = repo / "crates/opi-eval/scripts" / name
    shutil.copy2(source, cache / "pins" / "scripts" / name)
    os.chmod(cache / "pins" / "scripts" / name,
             source.stat().st_mode & 0o777)
(cache / "pins" / "profiles" / "benchmarks").mkdir(parents=True)
for adapter in adapters:
    shutil.copy2(repo / "crates/opi-eval/profiles/benchmarks" / f"{adapter}.toml",
                 cache / "pins" / "profiles" / "benchmarks" / f"{adapter}.toml")
for family in ("harbor", "pier"):
    shutil.copytree(repo / "crates/opi-eval/integrations" / family,
                    cache / "pins" / "integrations" / family, symlinks=True)
for adapter in adapters:
    shutil.copytree(Path(external) / clone_index[adapter],
                    cache / "corpora" / adapter, symlinks=True)
for runner in ("harbor", "pier"):
    shutil.copytree(Path(external) / runner,
                    cache / "runners" / runner, symlinks=True)
uv = subprocess.run(["sh", "-c", "command -v uv"], capture_output=True,
                    text=True).stdout.strip()
if not uv:
    raise SystemExit("materialize-configs: uv was not resolved on PATH")
uv = str(Path(uv).resolve())
(cache / "tools").mkdir(parents=True, exist_ok=True)
shutil.copy2(uv, cache / "tools" / "uv")
os.chmod(cache / "tools" / "uv", 0o755)

# Transport and probe the same client bytes. The pi package includes its
# interpreter and bundle resources; a host-absolute launcher alone cannot
# identify the code that will run inside the task environment.
opi_identity = json.loads(Path(agents_out, "opi-identity.json").read_text("utf-8"))
pi_identity = json.loads(Path(agents_out, "pi-identity.json").read_text("utf-8"))
opi_executable = Path(opi_identity["canonical_executable"])
node_executable = Path(pi_identity["node_executable"])
pi_bundle = Path(pi_identity["bundle_path"])
for path, expected in (
        (opi_executable, opi_identity["executable_sha256"]),
        (node_executable, pi_identity["node_sha256"]),
        (pi_bundle, pi_identity["bundle_sha256"])):
    if sha(path) != expected:
        raise SystemExit("materialize-configs: built client identity drift")
opi_cache = cache / "agents" / "opi"
opi_cache.parent.mkdir(parents=True)
shutil.copy2(opi_executable, opi_cache)
os.chmod(opi_cache, opi_executable.stat().st_mode & 0o777)
pi_package = cache / "agents" / "pi"
pi_package.mkdir(parents=True)
shutil.copy2(node_executable, pi_package / "node")
os.chmod(pi_package / "node", node_executable.stat().st_mode & 0o777)
# Preserve Node's package-root lookup and dist-relative assets from the
# canonical packages/coding-agent/dist/bundle/cli.js build output.
pi_source_package = pi_bundle.parents[2]
for name in ("dist", "docs", "examples"):
    shutil.copytree(pi_source_package / name, pi_package / name, symlinks=True)
for name in ("package.json", "README.md", "CHANGELOG.md"):
    shutil.copy2(pi_source_package / name, pi_package / name)
if sha(pi_package / "node") != pi_identity["node_sha256"] \
        or sha(pi_package / "dist" / "bundle" / "cli.js") != pi_identity["bundle_sha256"]:
    raise SystemExit("materialize-configs: transported pi identity drift")
pi_executable_sha256 = write_exec(pi_package / "pi", """#!/bin/sh
agent_root=$(CDPATH= cd -- "$(dirname "$0")" && pwd)
exec "$agent_root/node" "$agent_root/dist/bundle/cli.js" "$@"
""")
for runner in ("harbor", "pier"):
    (cache / "runner-homes" / runner).mkdir(parents=True)
(cache / "uv-cache").mkdir(parents=True)

configs = {}
materials = {}
for adapter in adapters:
    benchmark = "terminal-bench" if adapter.startswith("terminal-bench") else "deepswe"
    revision = "2.1" if adapter.endswith("2.1") else "3.0" if adapter.endswith("3.0") else "v1.1"
    task_id = task_ids[adapter]
    config = out / f"{adapter}.toml"
    config.write_text(f"""# eval native experiment config (native mode)
schema = "opi-eval-experiment/2"
experiment_id = "opi-eval-native-{adapter}"
connection = "scripted-local"

[benchmark]
name = "{benchmark}"
revision = "{revision}"
dataset = "{adapter}"
integrity_digest = "pending"

[[subjects]]
id = "baseline-pi"
product = "pi"
version = "1.0.4"

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
provider = "scripted"
api = "openai-completions"
base_url = "{base_url}"
model = "eval"
credential_env = "OPI_EVAL_SCRIPTED_KEY"

[model_controls]
temperature = 0.0
top_p = "omitted"
max_output_tokens = 4096
context_window = 128000
reasoning = "omitted"
required = ["max_output_tokens"]

[execution]
mode = "native-fixture"

[storage]
cache = "{cache}"
results = "results"
capture_policy = "full"

[environment]
platform = "linux"
architecture = "x86_64"
cwd_policy = "isolated"

[[trials]]
id = "trial-pi-{adapter}"
subject = "baseline-pi"
task = "{task_id}"
group = "group-{adapter}"

[[trials]]
id = "trial-opi-{adapter}"
subject = "candidate-opi"
task = "{task_id}"
group = "group-{adapter}"
""", encoding="utf-8")
    # Shared local preparation produces this family's material. A
    # rejection here (including a shared prepare refusal) aborts the
    # whole stage before any material is published.
    prepared = out / f"prepared-{adapter}"
    # Shared preparation with the exact candidate source: prepare builds
    # and pins the musl capture helper from the same commit (P19-AUD-003).
    prepare_argv = [str(opi_eval), "prepare", "--config", str(config),
                    "--out", str(prepared)]
    if os.environ.get("OPI_EVAL_CANDIDATE_SOURCE"):
        prepare_argv += ["--opi-source", os.environ["OPI_EVAL_CANDIDATE_SOURCE"],
                         "--opi-commit", os.environ.get("OPI_EVAL_CANDIDATE_COMMIT", "")]
    prepare = subprocess.run(prepare_argv, capture_output=True, text=True)
    (out / f"prepare-{adapter}.stdout").write_text(prepare.stdout, encoding="utf-8")
    (out / f"prepare-{adapter}.stderr").write_text(prepare.stderr, encoding="utf-8")
    if prepare.returncode != 0:
        sys.stderr.write(prepare.stderr)
        raise SystemExit(f"materialize-configs: shared preparation rejected {adapter}")
    material_path = prepared / "material.json"
    if not material_path.is_file():
        raise SystemExit(
            f"materialize-configs: shared preparation published no material for {adapter}")
    manifest = json.loads(material_path.read_text(encoding="utf-8"))
    if manifest.get("schema") != "opi-eval-native-material/7":
        raise SystemExit(
            f"materialize-configs: shared preparation emitted "
            f"{manifest.get('schema')!r} for {adapter}")
    # The existing producer observes both requests from each exact client
    # against a private loopback listener. Only successful evidence is pinned;
    # validation and config freezing cannot run after a failed/incomplete probe.
    clients_path = out / f"{adapter}-output-control-clients.json"
    proof_root = out / f"{adapter}-output-control"
    clients_path.write_text(json.dumps({
        "opi": str(opi_cache), "pi": str(pi_package / "pi"),
        "pi_package": str(pi_package), "experiment": str(config),
    }, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    probe_env = {key: os.environ[key] for key in (
        "HOME", "PATH", "CARGO_HOME", "CARGO_TARGET_DIR", "CARGO_INCREMENTAL",
        "RUSTUP_HOME", "RUSTUP_TOOLCHAIN") if key in os.environ}
    probe_env.update({"OPI_EVAL_NATIVE_CLIENT_MATERIAL": str(clients_path),
                      "OPI_EVAL_OUTPUT_CONTROL_OUT": str(proof_root)})
    probe_argv = ["cargo", "test", "--locked", "-p", "opi-eval",
                  "--test", "native_driver", "native_output_control_client_matrix",
                  "--", "--ignored", "--exact"]
    (out / f"{adapter}-output-control-command.json").write_text(json.dumps({
        "argv": probe_argv, "cwd": str(repo), "env": probe_env,
    }, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    probe = subprocess.run(probe_argv, cwd=repo, env=probe_env,
                           capture_output=True, text=True)
    (out / f"{adapter}-output-control-stdout.log").write_text(probe.stdout, encoding="utf-8")
    (out / f"{adapter}-output-control-stderr.log").write_text(probe.stderr, encoding="utf-8")
    (out / f"{adapter}-output-control-exit.json").write_text(
        json.dumps({"exit_code": probe.returncode}) + "\n", encoding="utf-8")
    if probe.returncode != 0:
        raise SystemExit("materialize-configs: exact-client output-control probe failed")
    for product in ("opi", "pi"):
        proof = proof_root / product / "output-control.json"
        if not proof.is_file():
            raise SystemExit("materialize-configs: exact-client output-control proof missing")
        manifest["agents"][product]["output_control_evidence"] = {
            "path": str(proof), "sha256": sha(proof)}
    material_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n",
                             encoding="utf-8")
    # Pin the integrity digest through the production validate entry.
    summary = subprocess.run(
        [str(opi_eval), "validate", "--config", str(config),
         "--native-material", str(material_path)],
        capture_output=True, text=True)
    if summary.returncode != 0:
        raise SystemExit(f"materialize-configs: validate rejected {adapter}: "
                         f"{summary.stderr.strip()}")
    digest = next(token for token in summary.stdout.split()
                  if token.startswith("native_integrity=")).split("=", 1)[1]
    text = config.read_text(encoding="utf-8").replace(
        'integrity_digest = "pending"', f'integrity_digest = "{digest}"')
    config.write_text(text, encoding="utf-8")
    configs[adapter] = {"path": str(config), "integrity_digest": digest}
    materials[adapter] = {"path": str(material_path), "sha256": sha(material_path)}

# Shared identity fields must agree across the per-family prepared
# material: one cache, one client set, one static lock. The pinned
# exact-client output-control evidence is per-family stage proof and is
# excluded from the shared-identity comparison.
def shared_agent_identity(agents):
    return {product: {key: value for key, value in agent.items()
                      if key != "output_control_evidence"}
            for product, agent in agents.items()}


first = json.loads(Path(materials[adapters[0]]["path"]).read_text(encoding="utf-8"))
for adapter in adapters[1:]:
    other = json.loads(Path(materials[adapter]["path"]).read_text(encoding="utf-8"))
    for field in ("schema", "origin", "static_lock"):
        if other.get(field) != first.get(field):
            raise SystemExit(
                f"materialize-configs: prepared material {adapter} disagrees "
                f"on shared identity {field}")
    if shared_agent_identity(other.get("agents", {})) != shared_agent_identity(first.get("agents", {})):
        raise SystemExit(
            f"materialize-configs: prepared material {adapter} disagrees "
            f"on shared client identity")
routing = {"schema": "opi-eval-native-material-routing/1",
           "adapters": materials}
(out / "material-routing.json").write_text(
    json.dumps(routing, indent=2, sort_keys=True) + "\n", encoding="utf-8")

receipt_extra = {
    "material_routing": str(out / "material-routing.json"),
    "prepared_materials": materials,
    "cache": str(cache),
    "opi_eval_executable": str(opi_eval),
    "opi_eval_executable_sha256": sha(opi_eval),
    "configs": configs,
    "preparation": "shared-local-prepare",
}
(out / "materialize-receipt.json").write_text(
    json.dumps(receipt_extra, indent=2, sort_keys=True) + "\n", encoding="utf-8")
print(json.dumps({"prepared_materials": list(materials), "configs": list(configs)},
                 sort_keys=True), file=sys.stderr)
MATEOF
  write_receipt "$out" materialize-configs \
    "$(python3 -c 'import json,sys; print(json.dumps({"materialized": json.load(open(sys.argv[1]))["configs"], "preparation": "shared-local-prepare"}))' "$out/materialize-receipt.json")"
}

# ---------------------------------------------------------------------------
# Stage: conformance-rerun
# ---------------------------------------------------------------------------
# Resolve one adapter's prepared material from the shared routing map
# produced by materialize-configs: every caller consumes exactly the
# per-family prepared manifest, never a second synthesized material.
route_material() {
  python3 - "$1" "$2" <<'ROUEOF'
import json, sys
routing = json.load(open(sys.argv[1], encoding="utf-8"))
if routing.get("schema") != "opi-eval-native-material-routing/1":
    raise SystemExit("material routing: unexpected schema")
entry = routing.get("adapters", {}).get(sys.argv[2])
if not entry:
    raise SystemExit(f"material routing: no prepared material for {sys.argv[2]}")
print(entry["path"])
ROUEOF
}

cmd_conformance_rerun() {
  material_root=""; out=""
  while [ $# -gt 0 ]; do
    case "$1" in
      --material-root) material_root=$2; shift 2 ;;
      --out) out=$2; shift 2 ;;
      *) die "conformance-rerun: unknown argument: $1" ;;
    esac
  done
  [ -n "$material_root" ] && [ -n "$out" ] \
    || die "conformance-rerun: --material-root and --out are required"
  require_file "$material_root/material-routing.json"
  OPI_EVAL_EXECUTABLE=$(resolve_opi_eval_executable \
    || die "conformance-rerun: opi-eval executable not resolved")
  OPI_EVAL_EXECUTABLE=$(readlink -f "$OPI_EVAL_EXECUTABLE")
  # The the hermetic conformance suite shared suites rerun through the exact built Opi and
  # pi programs and all three exact native revisions: only the admitted
  # native case subset runs; failure-injection cases stay hermetic.
  mkdir -p "$out/reports"
  cases_run=0
  for case_spec in \
      "agent opi completed" "agent opi identity" \
      "agent pi completed" "agent pi identity" \
      "benchmark terminal-bench-2.1 oracle-witness" \
      "benchmark terminal-bench-2.1 nop-witness" \
      "benchmark terminal-bench-3.0 oracle-witness" \
      "benchmark terminal-bench-3.0 nop-witness" \
      "benchmark deepswe oracle-witness" \
      "benchmark deepswe nop-witness"; do
    suite=${case_spec%% *}
    rest=${case_spec#* }
    adapter=${rest%% *}
    case_id=${rest#* }
    root="$out/reports/$suite-$adapter-$case_id"
    mkdir -p "$root"
    material=$(route_material "$material_root/material-routing.json" "$adapter" \
      || die "conformance-rerun: no prepared material routed for $adapter")
    # A native case that does not meet its contract exits 1 with the
    # details only in the JSON report, so a failure surfaces the report
    # instead of a bare exit status.
    if ! "$OPI_EVAL_EXECUTABLE" conformance --suite "$suite" --adapter "$adapter" \
      --case "$case_id" --root "$root" \
      --fixtures "$REPO_ROOT/crates/opi-eval/tests/fixtures" \
      --provider "$REPO_ROOT/crates/opi-eval/scripts/scripted-provider.py" \
      --native-material "$material" > "$root/report.json"; then
      echo "conformance-rerun: case $suite-$adapter-$case_id failed; report:" >&2
      cat "$root/report.json" >&2 || true
      die "conformance-rerun: case $suite-$adapter-$case_id did not meet its contract"
    fi
    cases_run=$((cases_run + 1))
  done
  # The receipt count derives from the loop counter, so adding or removing
  # a case changes the receipt without a second hand-maintained literal.
  write_receipt "$out" conformance-rerun \
    "$(python3 -c 'import json,sys; print(json.dumps({"cases_run": int(sys.argv[1]), "mode": "native-material"}))' "$cases_run")"
}

# ---------------------------------------------------------------------------
# Stage: oracle-preflight (explicitly selected TB2.1 artifact witnesses)
# ---------------------------------------------------------------------------
cmd_oracle_preflight() {
  material_root=""; out=""
  while [ $# -gt 0 ]; do
    case "$1" in
      --material-root) material_root=$2; shift 2 ;;
      --out) out=$2; shift 2 ;;
      *) die "oracle-preflight: unknown argument: $1" ;;
    esac
  done
  [ -n "$material_root" ] && [ -n "$out" ] \
    || die "oracle-preflight: --material-root and --out are required"
  require_file "$material_root/material-routing.json"
  material=$(route_material "$material_root/material-routing.json" terminal-bench-2.1 \
    || die "oracle-preflight: no prepared material routed for terminal-bench-2.1")
  require_file "$material"
  OPI_EVAL_EXECUTABLE=$(resolve_opi_eval_executable \
    || die "oracle-preflight: opi-eval executable not resolved")
  OPI_EVAL_EXECUTABLE=$(readlink -f "$OPI_EVAL_EXECUTABLE")
  mkdir -p "$out/reports"
  # The oracle is an explicitly selected conformance diagnostic, never a
  # prerequisite silently run by `run` (P19-D01): the correct witness
  # applies the official reference solution and must grade one; the nop
  # witness delivers nothing and must grade zero. Both traverse the same
  # production job handoff and binding.
  for case_id in oracle-witness nop-witness; do
    root="$out/reports/benchmark-terminal-bench-2.1-$case_id"
    mkdir -p "$root"
    "$OPI_EVAL_EXECUTABLE" conformance \
      --suite benchmark --adapter terminal-bench-2.1 --case "$case_id" \
      --root "$root" \
      --fixtures "$REPO_ROOT/crates/opi-eval/tests/fixtures" \
      --provider "$REPO_ROOT/crates/opi-eval/scripts/scripted-provider.py" \
      --native-material "$material" > "$root/report.json"
  done
  write_receipt "$out" oracle-preflight \
    "$(python3 -c 'import json; print(json.dumps({"witnesses": ["oracle-witness", "nop-witness"], "mode": "explicitly-selected-conformance"}))')"
}


cmd_run_trials() {
  config_root=""; out=""; material_root=""; canary_out=""
  while [ $# -gt 0 ]; do
    case "$1" in
      --config-root) config_root=$2; shift 2 ;;
      --material-root) material_root=$2; shift 2 ;;
      --canary-out) canary_out=$2; shift 2 ;;
      --out) out=$2; shift 2 ;;
      *) die "run-trials: unknown argument: $1" ;;
    esac
  done
  [ -n "$config_root" ] && [ -n "$out" ] \
    || die "run-trials: --config-root and --out are required"
  mkdir -p "$out"
  # The canary markers from the preflight stage gate sealing: any oracle
  # marker found in staged agent output blocks the seal (EVAL-BMK-003).
  canary_args=()
  if [ -n "$canary_out" ] && [ -f "$canary_out/canary-markers.txt" ]; then
    canary_args=(--canaries "$canary_out/canary-markers.txt")
  fi
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
  # experiment config is supplied by the dispatch (native smoke) and must
  # cover exactly one benchmark with its paired product matrix.
  ran=0
  for config in "$config_root"/*.toml; do
    [ -e "$config" ] || die "run-trials: no experiment configs in $config_root"
    name=$(basename "$config" .toml)
    run_root="$out/$name"
    if [ -n "$material_root" ]; then
      material=$(route_material "$material_root/material-routing.json" "$name" \
        || die "run-trials: no prepared material routed for $name")
      # The declared connection credential resolves from this exported
      # dummy only for the supervised Agent launch inside the trial.
      OPI_EVAL_SCRIPTED_KEY='<dummy-scripted-credential>' \
        "$OPI_EVAL_EXECUTABLE" run --config "$config" --root "$run_root" \
        --fixtures "$REPO_ROOT/crates/opi-eval/tests/fixtures" \
        --native-material "$material" "${canary_args[@]}"
    else
      "$OPI_EVAL_EXECUTABLE" run --config "$config" --root "$run_root"
    fi
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
# Sorted SHA-256 manifest over the sealed stage content. Symbolic
# links (the runner venv's python shims) are sealed as tar link
# entries, not byte payloads: hashing their targets here would pin
# bytes the archive cannot carry, and an offline reader would see
# every link as a missing file.
manifest = {}
for path in sorted(stage.rglob("*")):
    if path.is_file() and not path.is_symlink():
        manifest[str(path.relative_to(stage))] = hashlib.sha256(
            path.read_bytes()).hexdigest()
with open(out / "artifact-manifest.json", "w", encoding="utf-8") as f:
    json.dump({"schema": "opi-eval-native-artifact-manifest/1",
               "files": manifest}, f, indent=2, sort_keys=True)
    f.write("\n")
outer = {
    "schema": "opi-eval-native-outer-receipt/1",
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

# ---------------------------------------------------------------------------
# Stage: record-upload-identity
# ---------------------------------------------------------------------------
cmd_record_upload_identity() {
  seal_out=""; artifact_id=""; artifact_url=""; artifact_digest=""
  run_id=""; run_url=""; retention_days=""; out=""
  while [ $# -gt 0 ]; do
    case "$1" in
      --seal-out) seal_out=$2; shift 2 ;;
      --artifact-id) artifact_id=$2; shift 2 ;;
      --artifact-url) artifact_url=$2; shift 2 ;;
      --artifact-digest) artifact_digest=$2; shift 2 ;;
      --run-id) run_id=$2; shift 2 ;;
      --run-url) run_url=$2; shift 2 ;;
      --retention-days) retention_days=$2; shift 2 ;;
      --out) out=$2; shift 2 ;;
      *) die "record-upload-identity: unknown argument: $1" ;;
    esac
  done
  [ -n "$seal_out" ] && [ -n "$artifact_id" ] && [ -n "$artifact_url" ] \
    && [ -n "$artifact_digest" ] && [ -n "$run_id" ] && [ -n "$run_url" ] \
    && [ -n "$retention_days" ] && [ -n "$out" ] \
    || die "record-upload-identity: all arguments are required"
  # The receipt binds the uploaded artifact identity to the sealed
  # manifest digest recorded by the seal stage; it never hashes itself
  # (no self-reference) and derives its expiry from the retention window.
  sealed_manifest_sha=$(sha256sum "$seal_out/artifact-manifest.json" | cut -d' ' -f1)
  seal_receipt_sha=$(sha256sum "$seal_out/outer-receipt.json" | cut -d' ' -f1)
  python3 - "$out" "$artifact_id" "$artifact_url" "$artifact_digest" \
    "$run_id" "$run_url" "$retention_days" "$sealed_manifest_sha" \
    "$seal_receipt_sha" <<'UPEOF'
import datetime, json, sys

(out, artifact_id, artifact_url, artifact_digest, run_id, run_url,
 retention_days, manifest_sha, seal_receipt_sha) = sys.argv[1:10]
import pathlib
pathlib.Path(out).mkdir(parents=True, exist_ok=True)
now = datetime.datetime.now(datetime.timezone.utc)
expiry = now + datetime.timedelta(days=int(retention_days))
receipt = {
    "schema": "opi-eval-upload-identity-receipt/1",
    "artifact_id": artifact_id,
    "artifact_url": artifact_url,
    "artifact_digest": artifact_digest,
    "run_id": run_id,
    "run_url": run_url,
    "sealed_manifest_sha256": manifest_sha,
    "outer_receipt_sha256": seal_receipt_sha,
    "recorded_at": now.isoformat(),
    "expires_at": expiry.isoformat(),
    "retention_days": int(retention_days),
}
with open(f"{out}/upload-receipt.json", "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2, sort_keys=True)
    f.write("\n")
UPEOF
  write_receipt "$out" record-upload-identity \
    "$(python3 -c 'import json,sys; print(json.dumps({"artifact_id": sys.argv[1], "expires": True}))' "$artifact_id")"
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
  materialize-configs) cmd_materialize_configs "$@" ;;
  conformance-rerun) cmd_conformance_rerun "$@" ;;
  oracle-preflight) cmd_oracle_preflight "$@" ;;
  run-trials) cmd_run_trials "$@" ;;
  seal-upload) cmd_seal_upload "$@" ;;
  record-upload-identity) cmd_record_upload_identity "$@" ;;
  *) die "unknown stage command: $command" ;;
esac
