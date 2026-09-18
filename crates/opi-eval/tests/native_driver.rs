//! Native driving-mode integration tests (native mode).
//!
//! Each test stages a resolved-material manifest whose verifier entry is a
//! deterministic stand-in for the pinned upstream runner, whose task
//! package is the checked-in synthetic fixture package, and whose agents
//! carry the pinned integration identity, then drives the production
//! `opi-eval run --native-material` binary through the full native flow:
//! manifest validation, package verification, one explicitly selected
//! upstream job per trial that runs the selected Agent inside the graded
//! task environment and grades that same trial, fail-closed result
//! binding, one final classification, sealing, and the comparison report
//! (P19-D01). This proves the native driver plumbing and its identity
//! contract; the real built Opi/pi programs, official task environments,
//! and unchanged upstream runner execute only at the ignored real-TB2.1
//! witness (pinned external material) and the native smoke dispatch. No
//! paid provider, credential, network listener, or user-global resource
//! is touched.

#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fixtures() -> PathBuf {
    manifest_dir().join("tests/fixtures")
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn copy_dir(source: &Path, target: &Path) {
    fs::create_dir_all(target).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            copy_dir(&path, &target.join(entry.file_name()));
        } else {
            fs::copy(&path, target.join(entry.file_name())).unwrap();
        }
    }
}

fn write_exec(path: &Path, body: &str) {
    use std::os::unix::fs::PermissionsExt;
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, body).unwrap();
    fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

/// A private Docker daemon stand-in. It implements only the scoped lifecycle
/// exercised here; any other command fails without contacting a real engine.
fn stage_cleanup_docker(root: &Path) -> std::ffi::OsString {
    write_exec(
        &root.join("tools/docker"),
        r#"#!/usr/bin/env python3
import hashlib, json, os, pathlib, sys
root = pathlib.Path(__file__).resolve().parents[1]
state_path = root / 'docker-state.json'
state = json.loads(state_path.read_text()) if state_path.exists() else {}
args = sys.argv[1:]
with (root / 'docker-calls.jsonl').open('a') as log:
    log.write(json.dumps(args) + '\n')
if (root / 'require-dispatch-context').exists():
    assert os.environ.get('HOME') == str(root / 'runner-home')
    assert os.environ.get('DOCKER_CONTEXT') == 'fixture-job-context'
    assert os.environ.get('DOCKER_HOST') == 'unix:///fixture-job.sock'
label = 'label=com.docker.compose.project.working_dir='
if args[0] == 'info':
    print(json.dumps(dict(ID='fixture-engine', NCPU=8, MemTotal=17179869184, Runtimes={'nvidia': {}}, DockerRootDir='/unobserved-engine')))
elif args[:2] == ['image', 'inspect']:
    if (root / 'missing-image').exists(): sys.exit(1)
    print(json.dumps([args[-1]]))
elif args[0] == 'context':
    print('tcp://fixture-engine.invalid:2375')
elif args[0] == 'compose':
    working_dir = args[args.index('--project-directory') + 1]
    project = args[args.index('--project-name') + 1]
    for suffix in ('', '-neighbor'):
        directory = working_dir + suffix
        identity = hashlib.sha256(directory.encode()).hexdigest()
        state[identity] = dict(id=identity, working_dir=directory, project=project)
    state_path.write_text(json.dumps(state))
elif args[0] == 'ps':
    value = args[args.index('--filter') + 1]
    assert value.startswith(label), 'cleanup must use an exact ownership label'
    directory = value[len(label):]
    for identity, entry in state.items():
        if entry['working_dir'] == directory:
            print(identity)
elif args[0] == 'inspect':
    identity = args[-1]
    assert '--format' in args, 'never inspect container environment or secrets'
    entry = state[identity].copy()
    if (root / 'inspect-drift').exists():
        entry['working_dir'] += '-neighbor'
    print(json.dumps(entry))
elif args[0] == 'rm':
    identity = args[-1]
    assert len(identity) == 64 and identity in state
    assert not state[identity]['working_dir'].endswith('-neighbor')
    if (root / 'remove-failed').exists():
        sys.exit(1)
    if not (root / 'remove-lied').exists():
        del state[identity]
        state_path.write_text(json.dumps(state))
else:
    raise AssertionError('unexpected Docker command')
"#,
    );
    let mut paths = vec![root.join("tools")];
    paths.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
    std::env::join_paths(paths).unwrap()
}

/// The sorted-manifest digest scheme of `runner::material`, recomputed
/// independently here so the test does not trust the implementation's own
/// helper.
fn directory_manifest_digest(root: &Path) -> String {
    let mut rows: Vec<(String, String)> = Vec::new();
    fn visit(root: &Path, dir: &Path, rows: &mut Vec<(String, String)>) {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                visit(root, &path, rows);
            } else {
                let bytes = fs::read(&path).unwrap();
                rows.push((
                    path.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/"),
                    sha256_hex(&bytes),
                ));
            }
        }
    }
    visit(root, root, &mut rows);
    rows.sort();
    let mut canonical = String::new();
    for (path, digest) in rows {
        canonical.push_str(&path);
        canonical.push('\n');
        canonical.push_str(&digest);
        canonical.push('\n');
    }
    sha256_hex(canonical.as_bytes())
}

/// The deterministic stand-in for the pinned upstream runner entry
/// (`uv run --locked harbor run -p <dir> --agent <import> --model <model>`).
/// It validates the exact argv shape, reads the integration handoff, and
/// writes one complete bound job tree into its cwd (the trial's trace
/// root). The material's verifier-env mode selects the crafted native
/// shape:
///
/// - `ok` reward 1, clean terminal observation
/// - `zero` reward 0, agent failed the task
/// - `agent-nonzero` agent exited 1 (NonZeroAgentExitCodeError), reward 0
/// - `agent-timeout` agent timed out (AgentTimeoutError), reward 0
/// - `provider-error` agent died with ApiError, reward 0
/// - `oracle` the job ran the oracle agent instead
/// - `wrong-agent` a sibling product's integration ran
/// - `wrong-task` the trial reports a substituted task
/// - `substituted` collected artifact bytes differ from the delivered
///   observation
/// - `no-terminal` the terminal observation file is missing
/// - `no-job` exit 0 without writing any job tree
/// - `two-jobs` two new job trees (ambiguous admission)
/// - `tb30-ok` the TB3 separate-verifier job: declared `/app/evalbench/`
///   collection plus an undeclared delivered canary that must stay
///   outside the verifier
/// - `tb30-undeclared` the collection crossed an undeclared file
/// - `tb30-substituted-declared` the collected declared artifact lost the
///   delivered bytes
/// - `deepswe-ok` the DeepSWE committed patch crossed into the pristine
///   verifier with its base and image identity bound
/// - `deepswe-nop` the agent committed nothing: an empty patch still
///   collects (status ok) and grades zero
/// - `deepswe-undeclared` an undeclared file crossed into the verifier
/// - `deepswe-base-drift` the job recorded a different base revision
fn verifier_stand_in_script() -> String {
    r#"#!/bin/sh
# native stand-in for the pinned upstream runner entry (test staging)
set -eu
# Offline regrade dispatch (P19-AC-009): this fresh verifier environment
# grades the preserved Agent artifacts it received. No integration
# handoff, no agent dispatch, and no provider contact exist on this
# path; the argv of a graded dispatch never applies here.
if [ "${OPI_EVAL_REGRADE:-0}" = "1" ]; then
  identity="$OPI_EVAL_REGRADE_IDENTITY"
  artifacts_root="$OPI_EVAL_REGRADE_ARTIFACTS"
  [ -f "$identity" ] || { echo "missing regrade identity" >&2; exit 9; }
  [ -d "$artifacts_root" ] || { echo "missing regrade artifacts" >&2; exit 9; }
  python3 - "$identity" "$artifacts_root" "$OPI_EVAL_REGRADE_FAMILY" <<'REGRADE'
import hashlib, json, os, shutil, sys

identity = json.load(open(sys.argv[1]))
artifacts_root, family = sys.argv[2], sys.argv[3]
job = os.path.join("jobs", "2026-09-05__13-00-00")
trial_dir = os.path.join(job, identity["trial_name"])
os.makedirs(os.path.join(trial_dir, "agent"), exist_ok=True)
os.makedirs(os.path.join(trial_dir, "artifacts"), exist_ok=True)
# Deliver the preserved collected artifacts into the fresh job tree.
manifest = []
for entry in identity["collected"]:
    destination = entry["destination"]
    source_abs = os.path.join(artifacts_root, destination)
    target = os.path.join(trial_dir, "artifacts", destination)
    os.makedirs(os.path.dirname(target) or ".", exist_ok=True)
    if os.path.isdir(source_abs):
        shutil.copytree(source_abs, target)
        manifest.append({"source": entry["source"], "destination": "artifacts/" + destination, "type": "directory", "status": "ok"})
    else:
        shutil.copyfile(source_abs, target)
        manifest.append({"source": entry["source"], "destination": "artifacts/" + destination, "type": "file", "status": "ok"})
# The grading reads exactly these bytes: each family's verifier inspects
# the delivered environment, and the marker is the witness the reward
# derives from (changed witness bytes change the fresh grade).
markers = {
    "terminal-bench-2.1": ("root/answer.txt", b"native stand-in answer\n"),
    "terminal-bench-3.0": ("app/evalbench/schema.py", b"schema.py bytes"),
    "deepswe-v1.1": ("logs/artifacts/model.patch", b"fixture committed patch bytes"),
}
if family not in markers:
    print("unknown regrade family", file=sys.stderr)
    sys.exit(9)
marker_path, marker_bytes = markers[family]
observed = None
marker_abs = os.path.join(trial_dir, "artifacts", marker_path)
if os.path.isfile(marker_abs):
    with open(marker_abs, "rb") as handle:
        observed = handle.read()
reward = 1.0 if observed == marker_bytes else 0.0
terminal = {
    "schema": "opi-eval-native-agent-terminal/1",
    "product": identity["product"],
    "agent_executable_sha256": identity["agent_executable_sha256"],
    "exit_code": 0,
    "timed_out": False,
    "delivered": identity["delivered"],
}
with open(os.path.join(trial_dir, "agent", "opi-eval-terminal.json"), "w") as handle:
    json.dump(terminal, handle)
if identity["runner"] == "pier":
    eval_key = "{}__{}__adhoc".format(identity["agent_name"], identity["model"])
else:
    eval_key = identity["agent_name"] + "__adhoc"
aggregate = {
    "n_total_trials": 1,
    "stats": {"evals": {eval_key: {"reward_stats": {"reward": {str(reward): [identity["trial_name"]]}}}}},
}
with open(os.path.join(job, "result.json"), "w") as handle:
    json.dump(aggregate, handle)
config = {
    "agent": {"name": identity["agent_import_path"], "import_path": None},
    "task": {"path": os.path.abspath(os.path.join(os.path.dirname(sys.argv[1]), "task-package"))},
    "artifacts": [],
}
if identity["runner"] == "pier":
    config["agent"] = {"name": None, "import_path": identity["agent_import_path"]}
with open(os.path.join(trial_dir, "config.json"), "w") as handle:
    json.dump(config, handle)
results = {
    "task_name": identity["task_name"],
    "trial_name": identity["trial_name"],
    "agent_info": {"name": identity["agent_name"], "version": "1.0.0", "model_info": {"name": identity["model"]}},
    "verifier_result": {"rewards": {"reward": reward}},
    "exception_info": None,
}
if identity["runner"] == "harbor":
    results["verifier_environment_mode"] = identity["environment_mode"]
with open(os.path.join(trial_dir, "result.json"), "w") as handle:
    json.dump(results, handle)
with open(os.path.join(trial_dir, "artifacts", "manifest.json"), "w") as handle:
    json.dump(manifest, handle)
REGRADE
  exit $?
fi
[ "$1" = "run" ] && [ "$2" = "--locked" ] && { [ "$3" = "harbor" ] || [ "$3" = "pier" ]; } \
  && [ "$4" = "run" ] \
  || { echo "argv drift: expected run --locked harbor|pier run" >&2; exit 9; }
runner="$3"
job_config=""
if [ "$5" = "--config" ]; then
  [ "$#" = 6 ] || { echo "unexpected config override flags" >&2; exit 9; }
  job_config="$6"
  python3 - "$job_config" <<'JOBCONFIG'
import json, pathlib, sys
path = pathlib.Path(sys.argv[1])
assert path.is_absolute()
config = json.loads(path.read_text())
assert config['n_attempts'] == config['n_concurrent_trials'] == 1
assert config['retry']['max_retries'] == 0
assert len(config['agents']) == len(config['tasks']) == 1
assert config['datasets'] == []
JOBCONFIG
  task_dir=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["tasks"][0]["path"])' "$job_config")
  agent=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["agents"][0]["import_path"])' "$job_config")
  model=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["agents"][0]["model_name"])' "$job_config")
else
  [ "$5" = "-p" ] || { echo "missing task selection" >&2; exit 9; }
  task_dir="$6"
  agent=""
  model=""
  prev=""
  for arg in "$@"; do
    case "$prev" in
      --agent|--agent-import-path) agent="$arg" ;;
      --model|-m) model="$arg" ;;
    esac
    prev="$arg"
  done
fi
[ -n "$agent" ] || { echo "no explicitly selected agent" >&2; exit 9; }
[ -f "$OPI_EVAL_NATIVE_HANDOFF" ] || { echo "missing integration handoff" >&2; exit 9; }
[ -n "$OPI_EVAL_NATIVE_HANDOFF" ] && [ -n "${PYTHONPATH:-}" ] \
  || { echo "missing integration module path" >&2; exit 9; }
agent_name=$(sed -n 's/.*"integration_agent_name":"\([^"]*\)".*/\1/p' "$OPI_EVAL_NATIVE_HANDOFF")
[ -n "$agent_name" ] || { echo "handoff carries no integration identity" >&2; exit 9; }
product=$(sed -n 's/.*"product":"\([^"]*\)".*/\1/p' "$OPI_EVAL_NATIVE_HANDOFF")
diagnostic=$(sed -n 's/.*"diagnostic":"\([^"]*\)".*/\1/p' "$OPI_EVAL_NATIVE_HANDOFF")
if [ -z "$diagnostic" ]; then
  # A real agent trial carries the projected selected connection.
  [ "$model" = "scripted:eval" ] || { echo "model selector drift: $model" >&2; exit 9; }
  handoff_dir=$(dirname "$OPI_EVAL_NATIVE_HANDOFF")
  config_rel=$(sed -n 's/.*"config_path":"\([^"]*\)".*/\1/p' "$OPI_EVAL_NATIVE_HANDOFF")
  [ -n "$config_rel" ] && [ -f "$handoff_dir/$config_rel" ] \
    || { echo "handoff carries no projected configuration" >&2; exit 9; }
  grep -q "http://127.0.0.1:48127/v1" "$handoff_dir/$config_rel" \
    || { echo "projected configuration lost the selected endpoint" >&2; exit 9; }
  if [ "$product" = "opi" ]; then
    agent_key=$(sed -n 's/.*"OPI_EVAL_SCRIPTED_KEY":"\([^"]*\)".*/\1/p' "$OPI_EVAL_NATIVE_HANDOFF")
    [ -n "$agent_key" ] || { echo "handoff agent environment lost the credential" >&2; exit 9; }
  fi
fi
if [ "$OPI_EVAL_STANDIN_MODE" = "owned-container" ]; then
  # Enact the same exact-root labels used by the pinned Compose runners.
  mkdir -p witness
  sleep 300 &
  printf '%s\n' "$!" > witness/pid
  capture="$(pwd)/jobs/partial/task__partial/agent"
  mkdir -p "$capture"
  docker run -d --network none --pull never \
    --mount "type=bind,source=$capture,target=/logs/agent" \
    --label "com.docker.compose.project.working_dir=$task_dir/environment" \
    --label com.docker.compose.project=opi-eval-witness \
    "${OPI_EVAL_WITNESS_IMAGE:-busybox}" sh -c '
      umask 077
      mkdir -p /logs/agent/trace/run-1
      printf "private-root-stdout\n" > /logs/agent/stdout.log
      printf "private-root-stderr\n" > /logs/agent/stderr.log
      printf "private-root-trace\n" > /logs/agent/trace/run-1/evidence.jsonl
      touch /logs/agent/ready
      sleep 300
    ' >/dev/null
  while [ ! -f "$capture/ready" ]; do sleep 0.05; done
  printf 'ready\n' > witness/ready
  wait $!
fi
if [ "$OPI_EVAL_STANDIN_MODE" = "owned-cancel" ]; then
  # The owned-resource cancellation witness: the job owns a live child
  # process tree. It publishes the child pid under its trace root, then
  # holds the job open until the run supervisor cancels the whole tree.
  mkdir -p witness
  # Partial Agent evidence already reached the trial-owned log mount.
  capture=jobs/partial/task__partial/agent
  mkdir -p "$capture/trace/run-1"
  printf 'tool-progress\n' > "$capture/stdout.log"
  dd if=/dev/zero bs=1048576 count=2 >> "$capture/stdout.log" 2>/dev/null
  printf 'diagnostic-progress\n' > "$capture/stderr.log"
  printf 'trace-progress\n' > "$capture/trace/run-1/evidence.jsonl"
  printf 'untouched-neighbor\n' > witness/neighbor
  ln -s "$(pwd)/witness/neighbor" "$capture/trace/run-1/link"
  ln "$(pwd)/witness/neighbor" "$capture/trace/run-1/hardlink"
  mkfifo "$capture/trace/run-1/fifo"
  sleep 300 &
  printf '%s\n' "$!" > witness/pid
  printf 'ready\n' > witness/ready
  wait $!
fi
case "$OPI_EVAL_STANDIN_MODE" in
  owned-settled|owned-setup-failed)
    docker compose --project-name task-package__dqrbzu6__env \
      --project-directory "$task_dir/environment" up -d
    # The native CLI can exit zero even when trial setup and Compose down fail.
    if [ "$OPI_EVAL_STANDIN_MODE" = "owned-setup-failed" ]; then
      echo 'trial setup failed; compose down: getwd: no such file or directory' >&2
      exit 0
    fi
    OPI_EVAL_STANDIN_MODE=ok
    ;;
esac
task_name=$(basename "$task_dir")
if [ "$runner" = "pier" ]; then
  # Pier records the task's own `[task] name` as the bound trial's
  # task_name.
  task_name=$(sed -n '/^\[task\]$/,/^\[/{
    s/^name = "\(.*\)"$/\1/p
  }' "$task_dir/task.toml" | head -1)
  [ -n "$task_name" ] || task_name=$(basename "$task_dir")
fi
trial="$(printf '%s' "$task_name" | awk -F/ '{print $NF}')__standin"
job="jobs/2026-09-05__12-00-00"
reward=1.0
exception=null
tb30_profile=""
pier_profile=""
case "$OPI_EVAL_STANDIN_MODE" in
  ok) ;;
  tb30-ok|tb30-undeclared|tb30-substituted-declared) tb30_profile=1 ;;
  deepswe-ok|deepswe-nop|deepswe-undeclared|deepswe-base-drift) pier_profile=1 ;;
  zero) reward=0.0 ;;
  agent-nonzero) reward=0.0; exception='{"exception_type": "NonZeroAgentExitCodeError"}';;
  agent-timeout) reward=0.0; exception='{"exception_type": "AgentTimeoutError"}';;
  verifier-timeout) reward=0.0; exception='{"exception_type": "VerifierTimeoutError"}';;
  job-timeout) sleep 30; exit 0 ;;
  provider-error) reward=0.0; exception='{"exception_type": "ApiError"}';;
  oracle) agent_name="oracle"; agent="harbor.agents.oracle:OracleAgent" ;;
  nop-delivers-nothing) reward=0.0 ;;
  wrong-agent) agent_name="opi-eval-pi" ;;
  wrong-task) task_name="substituted-task"; trial="substituted-task__standin" ;;
  substituted) ;;
  no-terminal) ;;
  no-job) exit 0 ;;
  two-jobs) ;;
  *) echo "unknown stand-in mode" >&2; exit 9 ;;
esac
if [ "$runner" = "pier" ]; then
  # The upstream oracle and nop agents are Pier's own modules here.
  case "$OPI_EVAL_STANDIN_MODE" in oracle) agent="pier.agents.oracle:OracleAgent" ;; esac
fi
if [ "$agent" = "harbor.agents.nop:NopAgent" ] || [ "$agent" = "pier.agents.nop:NopAgent" ]; then
  # The nop agent delivers nothing: the native verifier grades zero.
  reward=0.0
fi
write_job() {
  job="$1"
  cat > "$job/result.json" <<JSON
{"n_total_trials": 1, "stats": {"evals": {"${agent_name}__adhoc": {"reward_stats": {"reward": {"$reward": ["$trial"]}}}}}}
JSON
  cat > "$job/$trial/result.json" <<JSON
{"task_name": "$task_name", "trial_name": "$trial", "agent_info": {"name": "$agent_name", "version": "1.0.0", "model_info": {"name": "$model"}}, "verifier_result": {"rewards": {"reward": $reward}}, "exception_info": $exception}
JSON
  if [ -n "$pier_profile" ]; then
    # Pier groups a modeled trial under agent__model__adhoc and records
    # the custom integration in agent.import_path.
    eval_key="${agent_name}__scripted:eval__adhoc"
    [ -n "$model" ] || eval_key="${agent_name}__adhoc"
    if [ "$OPI_EVAL_STANDIN_MODE" = "deepswe-base-drift" ]; then
      sed -i 's/1111111111111111111111111111111111111112/9999999999999999999999999999999999999999/g' "$task_dir/task.toml"
    fi
    sed -i.bak "s/\"${agent_name}__adhoc\"/\"$eval_key\"/" "$job/result.json" && rm -f "$job/result.json.bak"
    printf '{"agent": {"name": null, "import_path": "%s"}, "artifacts": [], "task": {"path": "%s"}}\n' "$agent" "$task_dir" \
      > "$job/$trial/config.json"
  elif [ -n "$tb30_profile" ]; then
    sed -i 's/"exception_info":/"verifier_environment_mode": "separate", "exception_info":/' "$job/$trial/result.json"
    printf '{"agent": {"name": "%s", "import_path": null}, "artifacts": [], "task": {"path": "%s"}}\n' "$agent" "$task_dir" \
      > "$job/$trial/config.json"
  else
    sed -i 's/"exception_info":/"verifier_environment_mode": "shared", "exception_info":/' "$job/$trial/result.json"
    printf '{"agent": {"name": "%s", "import_path": null}, "artifacts": [], "task": {"path": "%s"}}\n' "$agent" "$task_dir" \
      > "$job/$trial/config.json"
  fi
  if [ -n "$job_config" ]; then
    python3 - "$job_config" "$job/$trial/config.json" <<'TRIALCONFIG'
import json, pathlib, sys
job = json.loads(pathlib.Path(sys.argv[1]).read_text())
path = pathlib.Path(sys.argv[2])
trial = json.loads(path.read_text())
for key in ('max_timeout_sec', 'override_timeout_sec', 'model_name'):
    trial['agent'][key] = job['agents'][0].get(key)
trial['verifier'] = job['verifier']
trial['environment'] = job['environment']
for key in ('timeout_multiplier', 'agent_timeout_multiplier', 'verifier_timeout_multiplier'):
    trial[key] = job[key]
path.write_text(json.dumps(trial))
if __import__('os').environ['OPI_EVAL_STANDIN_MODE'] == 'verifier-timeout':
    result_path = path.with_name('result.json')
    result = json.loads(result_path.read_text())
    result['verifier_result'] = None
    result_path.write_text(json.dumps(result))
    aggregate_path = path.parent.parent / 'result.json'
    aggregate = json.loads(aggregate_path.read_text())
    stats = next(iter(aggregate['stats']['evals'].values()))
    stats.update(n_trials=0, n_errors=1, reward_stats={},
                 exception_stats={'VerifierTimeoutError': [result['trial_name']]})
    aggregate_path.write_text(json.dumps(aggregate))
TRIALCONFIG
  fi
}

if [ "$runner" = "pier" ]; then
  # Every pier dispatch - the deepswe run modes and the explicitly
  # selected oracle/nop witnesses - writes the pier job tree.
  pier_profile=1
fi
if [ "$OPI_EVAL_STANDIN_MODE" = "two-jobs" ]; then
  for job in jobs/2026-09-05__11-00-00 "$job"; do
    mkdir -p "$job/$trial/agent" "$job/$trial/artifacts"
    write_job "$job"
  done
  exit 0
fi
mkdir -p "$job/$trial/agent" "$job/$trial/artifacts"
write_job "$job"
if [ "$product" = "pi" ]; then
  cat > "$job/$trial/agent/stdout.log" <<'PI_SUCCESS'
OPI_EVAL_PI_SUCCESS_STREAM
PI_SUCCESS
fi
answer="native stand-in answer"
answer_sha=$(printf '%s\n' "$answer" | sha256sum | cut -d' ' -f1)
if [ -n "$tb30_profile" ]; then
  # The TB3 agent delivered a file inside the declared artifact plus an
  # unrelated canary that the task never declared: only the declared
  # artifact may cross into the separate verifier.
  schema="schema.py bytes"
  schema_sha=$(printf '%s' "$schema" | sha256sum | cut -d' ' -f1)
  canary_sha=$(printf 'canary\n' | sha256sum | cut -d' ' -f1)
  printf '{"schema": "opi-eval-native-agent-terminal/1", "product": "%s", "agent_executable_sha256": "%s", "exit_code": 0, "timed_out": false, "delivered": [{"source": "/app/evalbench/schema.py", "sha256": "%s"}, {"source": "/app/unrelated-canary.txt", "sha256": "%s"}]}\n' \
    "$product" "$(sed -n 's/.*"agent_executable_sha256":"\([^"]*\)".*/\1/p' "$OPI_EVAL_NATIVE_HANDOFF")" "$schema_sha" "$canary_sha" \
    > "$job/$trial/agent/opi-eval-terminal.json"
  mkdir -p "$job/$trial/artifacts/app/evalbench"
  printf '%s' "$schema" > "$job/$trial/artifacts/app/evalbench/schema.py"
  manifest='[{"source": "/app/evalbench/", "destination": "artifacts/app/evalbench", "type": "directory", "status": "ok", "service": null}]'
  case "$OPI_EVAL_STANDIN_MODE" in
    tb30-undeclared)
      # An undeclared file crossed the collection boundary: the binding
      # must reject it.
      manifest='[{"source": "/app/evalbench/", "destination": "artifacts/app/evalbench", "type": "directory", "status": "ok", "service": null}, {"source": "/app/unrelated-canary.txt", "destination": "artifacts/app/unrelated-canary.txt", "type": "file", "status": "ok", "service": null}]'
      ;;
    tb30-substituted-declared)
      # The collected declared artifact no longer carries the delivered
      # bytes.
      printf 'substituted schema bytes' > "$job/$trial/artifacts/app/evalbench/schema.py"
      ;;
  esac
  printf '%s\n' "$manifest" > "$job/$trial/artifacts/manifest.json"
  exit 0
fi
if [ -n "$pier_profile" ]; then
  # The DeepSWE agent committed a patch; the task's collect hook
  # extracted it and only it crossed into the pristine verifier.
  if [ -z "$diagnostic" ]; then
    printf '{"schema": "opi-eval-native-agent-terminal/1", "product": "%s", "agent_executable_sha256": "%s", "exit_code": 0, "timed_out": false, "delivered": []}\n' \
      "$product" "$(sed -n 's/.*"agent_executable_sha256":"\([^"]*\)".*/\1/p' "$OPI_EVAL_NATIVE_HANDOFF")" \
      > "$job/$trial/agent/opi-eval-terminal.json"
  fi
  mkdir -p "$job/$trial/artifacts/logs/artifacts"
  patch="fixture committed patch bytes"
  if [ "$OPI_EVAL_STANDIN_MODE" = "deepswe-nop" ] \
    || [ "$OPI_EVAL_STANDIN_MODE" = "nop-delivers-nothing" ]; then
    # The nop agent committed nothing: the hook extracted an empty
    # patch, which still collects (status ok) and grades zero.
    patch=""
  fi
  printf '%s' "$patch" > "$job/$trial/artifacts/logs/artifacts/model.patch"
  manifest='[{"source": "/logs/artifacts/model.patch", "destination": "artifacts/logs/artifacts/model.patch", "type": "file", "status": "ok"}]'
  if [ "$OPI_EVAL_STANDIN_MODE" = "deepswe-undeclared" ]; then
    # An undeclared file crossed the collection boundary into the
    # verifier: the binding must reject it.
    manifest='[{"source": "/logs/artifacts/model.patch", "destination": "artifacts/logs/artifacts/model.patch", "type": "file", "status": "ok"}, {"source": "/app/unrelated-canary.txt", "destination": "artifacts/app/unrelated-canary.txt", "type": "file", "status": "ok"}]'
  fi
  printf '%s\n' "$manifest" > "$job/$trial/artifacts/manifest.json"
  exit 0
fi
if [ "$OPI_EVAL_STANDIN_MODE" != "no-terminal" ]; then
  printf '{"schema": "opi-eval-native-agent-terminal/1", "product": "%s", "agent_executable_sha256": "%s", "exit_code": %s, "timed_out": %s, "delivered": [{"source": "/root/answer.txt", "sha256": "%s"}]}\n' \
    "$product" "$(sed -n 's/.*"agent_executable_sha256":"\([^"]*\)".*/\1/p' "$OPI_EVAL_NATIVE_HANDOFF")" \
    "$( [ "$OPI_EVAL_STANDIN_MODE" = "agent-nonzero" ] && echo 1 || echo 0 )" \
    "$( [ "$OPI_EVAL_STANDIN_MODE" = "agent-timeout" ] && echo true || echo false )" \
    "$answer_sha" > "$job/$trial/agent/opi-eval-terminal.json"
fi
if [ "$OPI_EVAL_STANDIN_MODE" = "substituted" ]; then
  answer="substituted answer"
fi
printf '[{"source": "/root/answer.txt", "destination": "artifacts/root/answer.txt", "type": "file", "status": "ok", "service": null}]\n' \
  > "$job/$trial/artifacts/manifest.json"
mkdir -p "$job/$trial/artifacts/root"
printf '%s\n' "$answer" > "$job/$trial/artifacts/root/answer.txt"
exit 0
"#
    .replace("OPI_EVAL_PI_SUCCESS_STREAM", &include_str!("fixtures/agents/pi/stream-ok.jsonl")
        .replace("\"provider\":\"local\"", "\"provider\":\"scripted\"")
        .replace("\"model\":\"scripted\"", "\"model\":\"eval\""))
}

struct Staged {
    /// Owns the staging directory so it outlives every test assertion.
    _guard: tempfile::TempDir,
    root: PathBuf,
    material: PathBuf,
}

fn pin_fixture_images(package: &Path) {
    for file in walk_files(package) {
        if file.file_name().unwrap() == "Dockerfile" {
            let text = fs::read_to_string(&file).unwrap();
            let lines = text
                .lines()
                .map(|line| {
                    if line.starts_with("FROM ") {
                        format!("FROM fixture@sha256:{}", "1".repeat(64))
                    } else {
                        line.to_owned()
                    }
                })
                .collect::<Vec<_>>();
            fs::write(file, format!("{}\n", lines.join("\n"))).unwrap();
        }
    }
}

fn fixture_git(project: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(project)
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn fixture_corpus_pin(project: &Path, adapter: &str) -> (serde_json::Value, serde_json::Value) {
    fixture_git(project, &["init", "--quiet"]);
    fixture_git(project, &["add", "-f", "--", "tasks"]);
    fixture_git(
        project,
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--allow-empty",
            "--quiet",
            "-m",
            "fixture",
        ],
    );
    let commit = fixture_git(project, &["rev-parse", "HEAD"]);
    let tree = fixture_git(project, &["rev-parse", "HEAD:tasks"]);
    (
        serde_json::json!({"root": project.join("tasks"), "source_commit": commit, "tasks_tree": tree}),
        serde_json::json!({"id": adapter, "commit": commit, "tasks_tree": tree}),
    )
}

fn fixture_task_binding(
    project: &Path,
    id: &str,
    package: &Path,
    profile: &Path,
) -> serde_json::Value {
    let mut doc: toml::Value = fs::read_to_string(profile).unwrap().parse().unwrap();
    let metadata: toml::Value = fs::read_to_string(package.join("task.toml"))
        .unwrap()
        .parse()
        .unwrap();
    let native_name = metadata["task"]
        .get("name")
        .or_else(|| metadata["task"].get("id"))
        .and_then(toml::Value::as_str)
        .unwrap_or(id);
    let mut rows = walk_files(package).into_iter().map(|file| {
        use std::os::unix::fs::PermissionsExt;
        let bytes = fs::read(&file).unwrap();
        serde_json::json!({"path": file.strip_prefix(package).unwrap().to_string_lossy(),
            "mode": if fs::metadata(file).unwrap().permissions().mode() & 0o111 != 0 {"100755"} else {"100644"},
            "size": bytes.len(), "sha256": sha256_hex(&bytes)})
    }).collect::<Vec<_>>();
    rows.sort_by(|a, b| a["path"].as_str().cmp(&b["path"].as_str()));
    for (key, value) in [
        (
            "source_commit",
            fixture_git(project, &["rev-parse", "HEAD"]),
        ),
        (
            "tasks_tree",
            fixture_git(project, &["rev-parse", "HEAD:tasks"]),
        ),
        (
            "task_tree",
            fixture_git(project, &["rev-parse", &format!("HEAD:tasks/{id}")]),
        ),
        ("task_id", id.to_owned()),
        (
            "package_manifest_sha256",
            sha256_hex(serde_json::to_string(&rows).unwrap().as_bytes()),
        ),
    ] {
        doc["identity"][key] = value.into();
    }
    doc["package"] = toml::Value::try_from(&rows).unwrap();
    let bytes = toml::to_string(&doc).unwrap();
    fs::write(profile, &bytes).unwrap();
    serde_json::json!({"native_task_name": native_name, "profile": {"path": profile, "sha256": sha256_hex(bytes.as_bytes())},
        "task_package": package, "task_package_manifest_sha256": directory_manifest_digest(package)})
}

/// Stages the whole native material: profile, task package, integration
/// module, verifier stand-in, provider script copy, and the manifest with
/// recomputed digests. The stand-in mode is projected through the
/// verifier environment (the manifest's closed projection).
fn stage(mode: &str) -> Staged {
    let root = tempfile::tempdir().unwrap();
    let root_path = root.path().to_path_buf();
    let mat = root_path.join("material");
    fs::create_dir_all(&mat).unwrap();

    let profile = mat.join("profile.toml");
    fs::copy(
        fixtures().join("benchmarks/terminal-bench-2.1/profile/synthetic.toml"),
        &profile,
    )
    .unwrap();
    let task = mat.join("task-package");
    if mode == "job-timeout" {
        let text = fs::read_to_string(&profile).unwrap();
        fs::write(
            &profile,
            text.replace("timeout_secs = 900", "timeout_secs = 1"),
        )
        .unwrap();
    }
    copy_dir(
        &fixtures().join("benchmarks/terminal-bench-2.1/task-package"),
        &task,
    );
    let tb30_task = mat.join("tb30-task-package");
    copy_dir(
        &fixtures().join("benchmarks/terminal-bench-3.0/task-package"),
        &tb30_task,
    );
    let tb30_profile = mat.join("tb30-profile.toml");
    fs::copy(
        fixtures().join("benchmarks/terminal-bench-3.0/profile/synthetic.toml"),
        &tb30_profile,
    )
    .unwrap();

    // The pinned integration module directories (the contract forms the
    // selected import paths resolve against): one Harbor module and one
    // Pier module, because the two runner surfaces are not
    // module-compatible.
    let integrations = mat.join("integrations-harbor");
    write_exec(
        &integrations.join("opi_eval_harbor.py"),
        "# native stand-in harbor integration module (test staging)\n",
    );
    let pier_integrations = mat.join("integrations-pier");
    write_exec(
        &pier_integrations.join("opi_eval_pier.py"),
        "# native stand-in pier integration module (test staging)\n",
    );

    let deepswe_task = mat.join("deepswe-task-package");
    copy_dir(
        &fixtures().join("benchmarks/deepswe-v1.1/task-package"),
        &deepswe_task,
    );
    let deepswe_profile = mat.join("deepswe-profile.toml");
    fs::copy(
        fixtures().join("benchmarks/deepswe-v1.1/profile/synthetic.toml"),
        &deepswe_profile,
    )
    .unwrap();

    let trace_fixture = fixtures().join("agents/opi/trace-complete/run-0001");
    let stream_fixture = fixtures().join("agents/pi/stream-ok.jsonl");

    // Opi stand-in: exact argv surface, isolated config, dummy credential,
    // completed trace child, answer file (the agent conformance cases
    // still execute the exact declared executables on the host).
    write_exec(
        &mat.join("agents/opi"),
        &format!(
            r#"#!/bin/sh
# native stand-in for the exact built opi binary (test staging)
[ "$1" = "--json" ] || {{ echo "argv drift: expected --json first" >&2; exit 9; }}
printf '%s\n' "$@" > opi-argv.txt
trace=""; config=""
prev=""
for arg in "$@"; do
  case "$prev" in
    --trace) trace="$arg" ;;
    --config) config="$arg" ;;
  esac
  prev="$arg"
done
[ -f "$config" ] || {{ echo "missing isolated config" >&2; exit 8; }}
grep -q "openai-completions" "$config" || {{ echo "config drift" >&2; exit 8; }}
grep -q "base_url" "$config" || {{ echo "config drift: no endpoint" >&2; exit 8; }}
[ -n "$OPENAI_API_KEY" ] || {{ echo "missing dummy credential" >&2; exit 8; }}
mkdir -p "$trace"
cp -r "{trace}" "$trace/run-0001"
printf 'native stand-in answer\n' > answer.txt
exit 0
"#,
            trace = trace_fixture.display(),
        ),
    );

    // pi stand-in: exact argv surface, isolated models.json, dummy
    // credential, native JSON stream on stdout, answer file.
    write_exec(
        &mat.join("agents/pi"),
        &format!(
            r#"#!/bin/sh
# native stand-in for the exact built pi bundle (test staging)
[ "$1" = "--mode" ] && [ "$2" = "json" ] || {{ echo "argv drift: expected --mode json" >&2; exit 9; }}
[ -f "$PI_CODING_AGENT_DIR/models.json" ] || {{ echo "missing models.json" >&2; exit 8; }}
grep -q '"apiKey":"<dummy-scripted-credential>"' "$PI_CODING_AGENT_DIR/models.json" \
  || {{ echo "models.json lost the projected credential" >&2; exit 8; }}
grep -q '"api":"openai-completions"' "$PI_CODING_AGENT_DIR/models.json" \
  || {{ echo "models.json lost the projected protocol" >&2; exit 8; }}
sed -e 's/"provider":"local"/"provider":"scripted"/' \
    -e 's/"model":"scripted"/"model":"eval"/' "{stream}"
printf 'pi stand-in answer\n' > answer.txt
exit 0
"#,
            stream = stream_fixture.display(),
        ),
    );

    // The verifier stand-in with its behavior selected by the material's
    // closed environment projection.
    let verifier = mat.join("verifier-uv.sh");
    write_exec(&verifier, &verifier_stand_in_script());

    // The provider script copy is digest-pinned identity only here; the
    // stand-in agents never contact a listener in local gates.
    let provider = mat.join("scripted-provider.py");
    fs::copy(
        manifest_dir().join("scripts/scripted-provider.py"),
        &provider,
    )
    .unwrap();
    let static_lock = mat.join("static-lock.json");
    fs::write(&static_lock, b"{\"schema\": \"fixture-lock\"}\n").unwrap();

    let pinned = |path: &Path| serde_json::json!({"path": path, "sha256": sha256_hex(&fs::read(path).unwrap())});
    let integration_block = |product: &str| {
        let class = if product == "opi" {
            "OpiAgent"
        } else {
            "PiAgent"
        };
        let entry = |dir: &Path, family: &str| {
            serde_json::json!({
            "module_dir": dir, "module_manifest_sha256": directory_manifest_digest(dir),
            "import_path": format!("opi_eval_{family}:{class}"), "agent_name": format!("opi-eval-{product}")})
        };
        serde_json::json!({"harbor": entry(&integrations, "harbor"), "pier": entry(&pier_integrations, "pier")})
    };
    let mut benchmarks = serde_json::Map::new();
    let mut subjects = Vec::new();
    for (adapter, package, template, mode) in [
        ("terminal-bench-2.1", &task, &profile, mode),
        ("terminal-bench-3.0", &tb30_task, &tb30_profile, "tb30-ok"),
        (
            "deepswe-v1.1",
            &deepswe_task,
            &deepswe_profile,
            "deepswe-ok",
        ),
    ] {
        let id = if adapter == "terminal-bench-2.1" {
            "fixture-task"
        } else {
            "synthetic-fixture-task"
        };
        // Explicit synthetic capacity needs: no machine-dependent storage
        // default and an immutable fixture image, never a pulled image.
        let metadata = package.join("task.toml");
        let mut doc: toml::Value = fs::read_to_string(&metadata).unwrap().parse().unwrap();
        if adapter == "deepswe-v1.1" {
            doc["environment"]
                .as_table_mut()
                .unwrap()
                .insert("storage_mb".to_owned(), 0.into());
            doc["environment"]["docker_image"] = format!(
                "fixture-registry/swe-bench-fixture@sha256:{}",
                "1".repeat(64)
            )
            .into();
            fs::write(
                package.join("environment/Dockerfile"),
                format!(
                    "FROM fixture-registry/swe-bench-fixture@sha256:{}\n",
                    "1".repeat(64)
                ),
            )
            .unwrap();
        }
        fs::write(&metadata, toml::to_string(&doc).unwrap()).unwrap();
        pin_fixture_images(package);
        let project = mat.join("corpora").join(adapter);
        copy_dir(package, &project.join("tasks").join(id));
        let (corpus, pin) = fixture_corpus_pin(&project, adapter);
        subjects.push(pin);
        let task_entry = fixture_task_binding(&project, id, package, template);
        benchmarks.insert(adapter.to_owned(), serde_json::json!({
            "corpus": corpus, "tasks": {id: task_entry},
            "verifier_executable": pinned(&verifier), "verifier_env": {"OPI_EVAL_STANDIN_MODE": mode},
            "oracle": pinned(&verifier), "oracle_env": {}
        }));
    }
    fs::write(
        &static_lock,
        serde_json::to_vec(&serde_json::json!({"schema": "fixture-lock", "subjects": subjects}))
            .unwrap(),
    )
    .unwrap();
    let material = mat.join("material.json");
    fs::write(&material, serde_json::to_vec(&serde_json::json!({
        "schema": "opi-eval-native-material/5", "static_lock": pinned(&static_lock),
        "provider": {"script": pinned(&provider), "endpoint": "http://127.0.0.1:48127/v1", "request_log": mat.join("requests.jsonl")},
        "agents": {
            "opi": {"executable": pinned(&mat.join("agents/opi")), "integrations": integration_block("opi")},
            "pi": {"executable": pinned(&mat.join("agents/pi")), "integrations": integration_block("pi")}
        }, "benchmarks": benchmarks
    })).unwrap()).unwrap();

    if mode != "owned-container" {
        let docker_path = stage_cleanup_docker(&root_path);
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&material).unwrap()).unwrap();
        for benchmark in value["benchmarks"].as_object_mut().unwrap().values_mut() {
            benchmark["verifier_env"]["PATH"] = serde_json::json!(docker_path.to_string_lossy());
        }
        fs::write(&material, serde_json::to_vec(&value).unwrap()).unwrap();
    }
    Staged {
        _guard: root,
        root: root_path,
        material,
    }
}

fn experiment_text(_staged: &Staged, integrity_digest: &str) -> String {
    format!(
        r#"schema = "opi-eval-experiment/2"
experiment_id = "native-driver-test"
connection = "scripted-local"

[benchmark]
name = "terminal-bench"
revision = "2.1"
dataset = "native-driver-test"
integrity_digest = "{integrity_digest}"

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
provider = "scripted"
api = "openai-completions"
base_url = "http://127.0.0.1:48127/v1"
model = "eval"
credential_env = "OPI_EVAL_SCRIPTED_KEY"

[model_controls]
temperature = 0.0
top_p = "omitted"
max_output_tokens = 4096
context_window = 8192
reasoning = "omitted"

[execution]
mode = "native-fixture"

[storage]
cache = "cache"
results = "results"
capture_policy = "full"

[environment]
platform = "linux"
architecture = "x86_64"
cwd_policy = "isolated"

[[trials]]
id = "trial-pi-native"
subject = "baseline-pi"
task = "fixture-task"
group = "group-native"

[[trials]]
id = "trial-opi-native"
subject = "candidate-opi"
task = "fixture-task"
group = "group-native"
"#,
        integrity_digest = integrity_digest,
    )
}

/// Derives the pinned integrity digest through the production validate
/// entry, exactly as the producer's config materialization does.
fn derive_integrity(staged: &Staged) -> String {
    let config = staged.root.join("config-undigested.toml");
    fs::write(&config, experiment_text(staged, "pending")).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("validate")
        .arg("--config")
        .arg(&config)
        .arg("--native-material")
        .arg(&staged.material)
        .output()
        .expect("validate runs");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    stdout
        .split_whitespace()
        .find(|token| token.starts_with("native_integrity="))
        .expect("native integrity digest in summary")
        .trim_start_matches("native_integrity=")
        .to_owned()
}

fn write_config(staged: &Staged) -> PathBuf {
    let digest = derive_integrity(staged);
    assert_eq!(digest.len(), 64);
    let config = staged.root.join("config.toml");
    fs::write(&config, experiment_text(staged, &digest)).unwrap();
    stage_output_control(staged, &config);
    config
}

/// Synthetic admission evidence for stand-in driver tests. Only the ignored
/// exact-client matrix establishes actual executable conformance.
fn stage_output_control(staged: &Staged, config: &Path) {
    use opi_eval::experiment::{
        CredentialReference, NativeClient, ResolvedCredential, ResolvedExperiment,
    };
    let experiment = ResolvedExperiment::resolve(&fs::read_to_string(config).unwrap()).unwrap();
    let mut material: serde_json::Value =
        serde_json::from_slice(&fs::read(&staged.material).unwrap()).unwrap();
    for client in [NativeClient::Opi, NativeClient::Pi] {
        let product = client.as_str();
        let root = staged.root.join("output-control-fixture").join(product);
        fs::create_dir_all(&root).unwrap();
        let mut connection = experiment.selected_connection().clone();
        connection.base_url = "http://127.0.0.1:48127".to_owned();
        connection.credential = CredentialReference::Env {
            variable: "OPI_EVAL_SCRIPTED_KEY".to_owned(),
        };
        let projection = opi_eval::experiment::project_connection(
            &connection,
            experiment.model_controls(),
            client,
            ResolvedCredential::FromEnv {
                variable: "OPI_EVAL_SCRIPTED_KEY".to_owned(),
                value: "dummy-scripted-credential".to_owned(),
            },
        )
        .unwrap();
        let config_path = root.join(projection.config_filename);
        fs::write(&config_path, &projection.config_bytes).unwrap();
        let controls = serde_json::to_value(experiment.model_controls()).unwrap();
        let (wire_path, field) = match connection.api {
            opi_eval::experiment::ConnectionApi::AnthropicMessages => {
                ("/v1/messages", "max_tokens")
            }
            opi_eval::experiment::ConnectionApi::OpenAiCompletions => {
                ("/v1/chat/completions", "max_tokens")
            }
            opi_eval::experiment::ConnectionApi::OpenAiResponses => {
                ("/v1/responses", "max_output_tokens")
            }
        };
        let requests = (1..=2).map(|seq| {
            serde_json::json!({"schema": "opi-eval-scripted-provider-log/1", "seq": seq, "status": 200, "method": "POST", "path": wire_path, "model": connection.model, "request_sha256": "d".repeat(64), "output_limits": {field: controls["max_output_tokens"]}}).to_string() + "\n"
        }).collect::<String>();
        let log = root.join("requests.jsonl");
        fs::write(&log, &requests).unwrap();
        let proof = serde_json::json!({
            "schema": "opi-eval-output-control/1", "product": product,
            "executable_sha256": material["agents"][product]["executable"]["sha256"],
            "runtime_package_manifest_sha256": material["agents"][product]["runtime_package"]["manifest_sha256"],
            "runtime_package_entrypoint": material["agents"][product]["runtime_package"]["entrypoint"],
            "connection": {"provider": connection.provider, "api": connection.api, "model": connection.model},
            "model_controls": controls, "probe_base_url": connection.base_url, "exit_code": 0,
            "config": {"path": config_path, "sha256": sha256_hex(&projection.config_bytes)},
            "requests": {"path": log, "sha256": sha256_hex(requests.as_bytes())},
        });
        let path = root.join("output-control.json");
        let bytes = serde_json::to_vec(&proof).unwrap();
        fs::write(&path, &bytes).unwrap();
        material["agents"][product]["output_control_evidence"] =
            serde_json::json!({"path": path, "sha256": sha256_hex(&bytes)});
    }
    fs::write(&staged.material, serde_json::to_vec(&material).unwrap()).unwrap();
}

fn run_native(staged: &Staged, config: &Path, run_root: &Path) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("run")
        .arg("--config")
        .arg(config)
        .arg("--root")
        .arg(run_root)
        .arg("--fixtures")
        .arg(fixtures())
        .arg("--native-material")
        .arg(&staged.material)
        .env("OPI_EVAL_SCRIPTED_KEY", "<dummy-scripted-credential>")
        .env("PATH", stage_cleanup_docker(&staged.root))
        .output()
        .expect("run executes");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn native_settlement_removes_actual_credentials() {
    for mode in ["ok", "no-job", "provider-error"] {
        let staged = stage(mode);
        let config = write_config(&staged);
        let run_root = staged.root.join("run");
        let (_, stdout, stderr) = run_native(&staged, &config, &run_root);
        let report: serde_json::Value =
            serde_json::from_str(stdout.trim()).unwrap_or_else(|_| panic!("{stderr}"));
        for trial in report["trials"].as_array().unwrap() {
            let integration = run_root
                .join("trials")
                .join(trial["id"].as_str().unwrap())
                .join("integration");
            for filename in ["config.toml", "models.json", "handoff.json"] {
                assert!(
                    !integration.join(filename).exists(),
                    "{mode}: retained {filename}"
                );
            }
            let identity = read_cleanup_json(&integration.join("handoff-identity.json"));
            assert!(identity.get("agent_env").is_none());
            assert_eq!(identity["trial"], trial["id"]);
            assert!(!identity.to_string().contains("<dummy-scripted-credential>"));
        }
    }
}

#[test]
fn native_settlement_cleans_root_bound_containers() {
    for mode in ["owned-settled", "owned-setup-failed"] {
        let staged = stage(mode);
        let config = write_config(&staged);
        let run_root = staged.root.join("run");
        let (_, stdout, stderr) = run_native(&staged, &config, &run_root);
        let report: serde_json::Value =
            serde_json::from_str(stdout.trim()).unwrap_or_else(|_| panic!("{stderr}"));
        let state = read_cleanup_json(&staged.root.join("docker-state.json"));
        assert_eq!(
            state.as_object().unwrap().len(),
            2,
            "only the two neighbors survive"
        );
        assert!(state.as_object().unwrap().values().all(|entry| {
            entry["working_dir"]
                .as_str()
                .unwrap()
                .ends_with("-neighbor")
        }));
        for trial in report["trials"].as_array().unwrap() {
            assert_eq!(trial["verifier"]["exit_state"], "exited:0");
            let evidence = read_cleanup_json(
                &run_root
                    .join("trials")
                    .join(trial["id"].as_str().unwrap())
                    .join("integration/owned-resources.json"),
            );
            assert!(evidence.get("container_prefix").is_none());
            assert_eq!(evidence["trial"], trial["id"]);
            assert_eq!(evidence["containers"][0]["status"], "removed");
        }
    }
}

#[test]
fn native_settlement_cleanup_uncertainty_blocks_success() {
    for failure in ["inspect-drift", "remove-failed", "remove-lied"] {
        let staged = stage("owned-settled");
        fs::write(staged.root.join(failure), "").unwrap();
        let config = write_config(&staged);
        let run_root = staged.root.join("run");
        let (code, stdout, stderr) = run_native(&staged, &config, &run_root);
        assert_eq!(code, 1, "{failure}: {stdout} {stderr}");
        let report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(report["outcome"], "incomplete");
        let calls = fs::read_to_string(staged.root.join("docker-calls.jsonl")).unwrap();
        if failure == "inspect-drift" {
            assert!(
                !calls.lines().any(
                    |line| serde_json::from_str::<serde_json::Value>(line).unwrap()[0] == "rm"
                )
            );
        }
        for trial in report["trials"].as_array().unwrap() {
            assert_eq!(trial["status"], "failed");
            assert_eq!(
                trial["classification"]["reason"],
                "owned-resource-cleanup-unconfirmed"
            );
            let integration = run_root
                .join("trials")
                .join(trial["id"].as_str().unwrap())
                .join("integration");
            assert!(!integration.join("handoff.json").exists());
        }
    }
}

fn read_cleanup_json(path: &Path) -> serde_json::Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

#[test]
fn native_settlement_cleanup_uses_dispatch_docker_context() {
    let staged = stage("owned-settled");
    fs::write(staged.root.join("require-dispatch-context"), "").unwrap();
    let mut material = read_cleanup_json(&staged.material);
    let env = &mut material["benchmarks"]["terminal-bench-2.1"]["verifier_env"];
    env["HOME"] = serde_json::json!(staged.root.join("runner-home").to_string_lossy());
    env["DOCKER_CONTEXT"] = serde_json::json!("fixture-job-context");
    env["DOCKER_HOST"] = serde_json::json!("unix:///fixture-job.sock");
    fs::write(&staged.material, serde_json::to_vec(&material).unwrap()).unwrap();
    let config = write_config(&staged);
    let run_root = staged.root.join("run");
    let (code, stdout, stderr) = run_native(&staged, &config, &run_root);
    assert_eq!(code, 0, "{stdout} {stderr}");
    let state = read_cleanup_json(&staged.root.join("docker-state.json"));
    assert!(state.as_object().unwrap().values().all(|entry| {
        entry["working_dir"]
            .as_str()
            .unwrap()
            .ends_with("-neighbor")
    }));
}

/// Reads one trial receipt from a completed run root.
fn receipt_of(run_root: &Path, trial: &str) -> serde_json::Value {
    let text = fs::read_to_string(run_root.join("trials").join(trial).join("receipt.json"))
        .unwrap_or_else(|error| panic!("trial {trial} receipt: {error}"));
    serde_json::from_str(&text).unwrap()
}

#[test]
fn native_run_drives_paired_trials_through_one_selected_job_each() {
    let staged = stage("ok");
    let config = write_config(&staged);
    let run_root = staged.root.join("run");
    let (code, stdout, stderr) = run_native(&staged, &config, &run_root);
    assert_eq!(code, 0, "stderr: {stderr}\nstdout: {stdout}");
    let report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(report["outcome"], "completed");
    assert_eq!(report["trials"].as_array().unwrap().len(), 2);
    assert_eq!(report["pairs"].as_array().unwrap().len(), 1);
    let persisted: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(run_root.join("run-report.json")).unwrap())
            .unwrap();
    assert_eq!(persisted["outcome"], "completed");

    // The oracle preflight no longer runs as a prerequisite of `run`:
    // no preflight directory exists (P19-D01).
    assert!(!run_root.join("preflight").exists());

    for trial in ["trial-pi-native", "trial-opi-native"] {
        // One bound job tree per trial under its own trace root.
        let jobs = run_root
            .join("trials")
            .join(trial)
            .join("verifier-trace/jobs");
        let job_names: Vec<String> = fs::read_dir(&jobs)
            .unwrap()
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            job_names.len(),
            1,
            "{trial}: exactly one job, got {job_names:?}"
        );
        // The sealed bundle covers the bound native evidence.
        let bundle = run_root.join("trials").join(trial).join("bundle");
        assert!(bundle.join("manifest.json").is_file(), "{trial} sealed");
        for key in [
            "artifacts/normalized/expected-output",
            "artifacts/native/trial-result.json",
            "artifacts/native/trial-config.json",
            "artifacts/native/job-stdout.log",
            "artifacts/native/job-stderr.log",
        ] {
            assert!(bundle.join(key).is_file(), "{trial} covers {key}");
        }
        // The receipt carries the bound identity and the classification.
        let receipt = receipt_of(&run_root, trial);
        assert_eq!(receipt["status"], "sealed");
        assert_eq!(receipt["mode"], "native");
        assert_eq!(receipt["classification"]["class"], "graded-native");
        assert_eq!(receipt["classification"]["reward"], 1);
        assert_eq!(receipt["verifier"]["completion"], "verified-native");
        assert_eq!(
            receipt["verifier"]["binding"]["agent"],
            if trial.starts_with("trial-pi") {
                "opi-eval-pi"
            } else {
                "opi-eval-opi"
            }
        );
        // No host-agent artifacts exist in native mode.
        assert!(!bundle.join("artifacts/native/agent-answer.txt").exists());
        assert!(
            !run_root
                .join("trials")
                .join(trial)
                .join("workspace")
                .exists()
        );
    }

    // Secret projections are removed; only their nonsecret identity survives.
    for trial in ["trial-pi-native", "trial-opi-native"] {
        let bundle = run_root.join("trials").join(trial).join("bundle");
        let manifest = fs::read_to_string(bundle.join("manifest.json")).unwrap();
        assert!(
            !manifest.contains("api_key"),
            "{trial}: credential leaked into manifest"
        );
        assert!(
            run_root
                .join("trials")
                .join(trial)
                .join("integration/handoff-identity.json")
                .is_file(),
            "{trial} retained the handoff identity"
        );
    }
}

#[test]
fn native_trial_outcome_classifies_every_terminal_shape() {
    // Valid zero score: the agent failed the task; the native metric is
    // preserved as a graded Agent failure.
    let staged = stage("zero");
    let config = write_config(&staged);
    let run_root = staged.root.join("run");
    let (code, stdout, stderr) = run_native(&staged, &config, &run_root);
    assert_eq!(code, 0, "stderr: {stderr}\nstdout: {stdout}");
    let report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(report["outcome"], "completed");
    let receipt = receipt_of(&run_root, "trial-opi-native");
    assert_eq!(receipt["classification"]["class"], "graded-native");
    assert_eq!(receipt["classification"]["reward"], 0);
    assert_eq!(receipt["classification"]["agent_failed"], true);
    // The pair stays comparable: a scored Agent failure remains in the
    // graded denominator.
    assert_eq!(report["pairs"][0]["comparability"], "comparable");

    // A benchmark-valid agent timeout keeps its native metric.
    let staged = stage("agent-timeout");
    let config = write_config(&staged);
    let run_root = staged.root.join("run");
    let (code, _, _) = run_native(&staged, &config, &run_root);
    assert_eq!(code, 0);
    let receipt = receipt_of(&run_root, "trial-opi-native");
    assert_eq!(receipt["classification"]["class"], "graded-native");
    assert_eq!(receipt["classification"]["timed_out"], true);
    assert_eq!(receipt["classification"]["reward"], 0);

    // The agent's own non-zero exit is a graded Agent failure.
    let staged = stage("agent-nonzero");
    let config = write_config(&staged);
    let run_root = staged.root.join("run");
    let (code, _, _) = run_native(&staged, &config, &run_root);
    assert_eq!(code, 0);
    let receipt = receipt_of(&run_root, "trial-opi-native");
    assert_eq!(receipt["classification"]["class"], "graded-native");
    assert_eq!(receipt["classification"]["agent_failed"], true);

    // A terminal provider failure preserves attempted coverage without an
    // Agent score: the pair is non-comparable with the failure class.
    let staged = stage("provider-error");
    let config = write_config(&staged);
    let run_root = staged.root.join("run");
    let (code, stdout, _) = run_native(&staged, &config, &run_root);
    assert_eq!(code, 1, "an invalid execution is a non-success outcome");
    let report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(report["outcome"], "incomplete");
    let receipt = receipt_of(&run_root, "trial-opi-native");
    assert_eq!(receipt["classification"]["class"], "invalid-execution");
    assert_eq!(receipt["classification"]["boundary"], "agent-process");
    assert!(
        receipt["classification"]["reason"]
            .as_str()
            .unwrap_or_default()
            .starts_with("agent-provider-failure"),
        "reason: {}",
        receipt["classification"]["reason"]
    );
    // The trial stays visible in the denominator as a failure pair.
    assert_eq!(
        report["pairs"][0]["comparability"],
        "infrastructure-failure:trial-pi-native"
    );
}

#[test]
fn native_job_and_verifier_timeouts_keep_distinct_settlements() {
    for (mode, reason, boundary) in [
        ("job-timeout", "native-job-timeout", "infrastructure"),
        ("verifier-timeout", "verifier-timeout", "grader"),
    ] {
        let staged = stage(mode);
        let config = write_config(&staged);
        let run_root = staged.root.join("run");
        let (code, stdout, stderr) = run_native(&staged, &config, &run_root);
        assert_eq!(code, 1, "{mode}: {stderr}\n{stdout}");
        let report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(report["outcome"], "incomplete");
        for trial in ["trial-opi-native", "trial-pi-native"] {
            let receipt = report["trials"]
                .as_array()
                .unwrap()
                .iter()
                .find(|receipt| receipt["id"] == trial)
                .unwrap_or_else(|| panic!("{trial}: {report}"));
            assert_eq!(receipt["classification"]["class"], "invalid-execution");
            assert_eq!(receipt["classification"]["reason"], reason);
            assert_eq!(receipt["classification"]["boundary"], boundary);
            assert_eq!(receipt["classification"]["timed_out"], true);
            let settlement: serde_json::Value = serde_json::from_slice(
                &fs::read(
                    run_root
                        .join("trials")
                        .join(trial)
                        .join("bundle/artifacts/control/settlement.json"),
                )
                .unwrap(),
            )
            .unwrap();
            assert_eq!(settlement["graded"], false);
            assert_eq!(settlement["timed_out"], true);
            assert_eq!(settlement["failure_kind"], reason);
            assert_eq!(settlement["boundary"], boundary);
            if mode == "job-timeout" {
                assert_eq!(receipt["verifier"]["failure_kind"], reason);
                assert_eq!(receipt["verifier"]["boundary"], boundary);
                assert!(receipt["verifier"]["binding"].is_null());
            }
        }
    }
}

#[test]
fn native_runner_timeout_keeps_partial_client_evidence_graded() {
    let staged = stage("agent-timeout");
    remediation_runner(&staged, |script| {
        // A client interrupted mid-turn has no final success event.
        let start = script.find("<<'PI_SUCCESS'\n").unwrap() + "<<'PI_SUCCESS'\n".len();
        let end = script[start..].find("\nPI_SUCCESS").unwrap() + start;
        format!("{}{}", &script[..start], &script[end..]).replace(
            "[ \"$OPI_EVAL_STANDIN_MODE\" = \"agent-timeout\" ] && echo true || echo false",
            "echo false",
        )
    });
    let config = write_config(&staged);
    let (code, stdout, stderr) = run_native(&staged, &config, &staged.root.join("run"));
    assert_eq!(code, 0, "{stdout}\n{stderr}");
    let report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    for trial in report["trials"].as_array().unwrap() {
        assert_eq!(trial["classification"]["class"], "graded-native", "{trial}");
        assert_eq!(trial["classification"]["timed_out"], true, "{trial}");
    }
}

#[test]
fn native_result_binding_rejects_every_substitution() {
    // Oracle attribution: even on job exit 0, an oracle-run job is not a
    // selected-Agent score (P19-AC-002).
    for (mode, expected_kind) in [
        ("oracle", "native-binding-agent-mismatch"),
        ("wrong-agent", "native-binding-agent-mismatch"),
        ("wrong-task", "native-binding-task-mismatch"),
        (
            "substituted",
            "native-binding-delivered-artifact-substituted",
        ),
        ("no-terminal", "native-binding-trial-invalid"),
        ("no-job", "native-binding-no-new-job"),
        ("two-jobs", "native-binding-multiple-new-jobs"),
    ] {
        let staged = stage(mode);
        let config = write_config(&staged);
        let run_root = staged.root.join("run");
        let (code, stdout, stderr) = run_native(&staged, &config, &run_root);
        assert_ne!(code, 0, "{mode}: stderr: {stderr}");
        let report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(
            report["outcome"], "incomplete",
            "{mode}: a rejected admission cannot complete"
        );
        // The receipt stays durable private diagnostics even when the
        // report transition is mechanically refused: the run report
        // carries it.
        let receipt = report["trials"]
            .as_array()
            .unwrap()
            .iter()
            .find(|trial| trial["id"] == "trial-opi-native")
            .cloned()
            .unwrap_or_else(|| panic!("{mode}: trial receipt missing from run report"));
        assert_eq!(
            receipt["status"], "failed",
            "{mode}: no seal without admission"
        );
        assert_eq!(
            receipt["classification"]["class"], "invalid-execution",
            "{mode}"
        );
        assert_eq!(
            receipt["classification"]["boundary"], "integrity",
            "{mode}: identity failures own the integrity boundary"
        );
        assert_eq!(
            receipt["verifier"]["failure_kind"], expected_kind,
            "{mode}: failure kind"
        );
        // No sealed bundle exists: scoring and export are rejected while
        // the truthful private diagnostics remain on disk.
        assert!(
            !run_root
                .join("trials/trial-opi-native/bundle/manifest.json")
                .is_file(),
            "{mode}: rejected admission must not seal"
        );
    }
}

#[test]
fn task_package_drift_after_materialization_is_rejected() {
    let staged = stage("ok");
    let config = write_config(&staged);
    // Tamper with the materialized package after the manifest pinned it.
    fs::write(
        staged.root.join("material/task-package/rogue-file.txt"),
        b"unregistered",
    )
    .unwrap();
    let run_root = staged.root.join("run");
    let (code, _, stderr) = run_native(&staged, &config, &run_root);
    assert_eq!(code, 2);
    assert!(
        stderr.contains("task package manifest drift"),
        "stderr: {stderr}"
    );
}

#[test]
fn integration_module_drift_is_rejected_at_load() {
    let staged = stage("ok");
    // Tamper with the pinned integration modules after materialization.
    fs::write(
        staged
            .root
            .join("material/integrations-harbor/opi_eval_harbor.py"),
        b"tampered",
    )
    .unwrap();
    fs::write(
        staged
            .root
            .join("material/integrations-pier/opi_eval_pier.py"),
        b"tampered",
    )
    .unwrap();
    let config = staged.root.join("config.toml");
    let zero = "0".repeat(64);
    fs::write(&config, experiment_text(&staged, &zero)).unwrap();
    let run_root = staged.root.join("run");
    let (code, _, stderr) = run_native(&staged, &config, &run_root);
    assert_eq!(code, 2);
    assert!(
        stderr.contains("integration module drift"),
        "stderr: {stderr}"
    );
}

#[test]
fn agent_executable_digest_drift_is_rejected_at_load() {
    let staged = stage("ok");
    // Tamper with the pinned agent executable after materialization.
    fs::write(staged.root.join("material/agents/opi"), b"tampered").unwrap();
    let config = staged.root.join("config.toml");
    let zero = "0".repeat(64);
    fs::write(&config, experiment_text(&staged, &zero)).unwrap();
    let run_root = staged.root.join("run");
    let (code, _, stderr) = run_native(&staged, &config, &run_root);
    assert_eq!(code, 2);
    assert!(stderr.contains("digest drift"), "stderr: {stderr}");
}

/// The TB3 experiment config: the declared separate-verifier lifecycle
/// with its `/app/evalbench/` collect declaration.
fn tb30_experiment_text(staged: &Staged, integrity_digest: &str) -> String {
    experiment_text(staged, integrity_digest)
        .replace("native-driver-test", "native-tb30-test")
        .replace("fixture-task", "synthetic-fixture-task")
        .replace(
            "terminal-bench\"\nrevision = \"2.1\"",
            "terminal-bench\"\nrevision = \"3.0\"",
        )
}

fn write_tb30_config(staged: &Staged) -> PathBuf {
    let config = staged.root.join("config-undigested-tb30.toml");
    fs::write(&config, tb30_experiment_text(staged, "pending")).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("validate")
        .arg("--config")
        .arg(&config)
        .arg("--native-material")
        .arg(&staged.material)
        .output()
        .expect("validate runs");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let digest = stdout
        .split_whitespace()
        .find(|token| token.starts_with("native_integrity="))
        .expect("native integrity digest in summary")
        .trim_start_matches("native_integrity=");
    let config = staged.root.join("tb30.toml");
    fs::write(&config, tb30_experiment_text(staged, digest)).unwrap();
    stage_output_control(staged, &config);
    config
}

/// Retarget the TB3 benchmark entry's stand-in behavior after staging.
fn set_tb30_mode(staged: &Staged, mode: &str) {
    let mut material: serde_json::Value =
        serde_json::from_slice(&fs::read(&staged.material).unwrap()).unwrap();
    material["benchmarks"]["terminal-bench-3.0"]["verifier_env"]["OPI_EVAL_STANDIN_MODE"] =
        serde_json::Value::String(mode.to_owned());
    fs::write(
        &staged.material,
        serde_json::to_vec_pretty(&material).unwrap(),
    )
    .unwrap();
}

#[test]
fn native_tb30_run_grades_declared_artifacts_across_the_separate_verifier() {
    let staged = stage("ok");
    let config = write_tb30_config(&staged);
    let run_root = staged.root.join("run-tb30");
    let (code, stdout, stderr) = run_native(&staged, &config, &run_root);
    assert_eq!(code, 0, "stderr: {stderr}\nstdout: {stdout}");
    let report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(report["outcome"], "completed");

    for trial in ["trial-pi-native", "trial-opi-native"] {
        let receipt = report["trials"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["id"] == trial)
            .cloned()
            .unwrap_or_else(|| panic!("{trial} receipt missing"));
        assert_eq!(receipt["status"], "sealed", "{trial}: {receipt}");
        // The verifier-input identity: the declared artifact crossed into
        // the separate verifier with the digests of exactly the collected
        // bytes, and the unrelated delivered canary did not.
        let collected = receipt["agent"]["collected"].as_array().unwrap();
        assert_eq!(collected.len(), 1, "{trial}: {collected:?}");
        assert_eq!(collected[0]["source"], "/app/evalbench/");
        assert!(collected[0]["sha256"].as_str().unwrap().len() == 64);
        assert!(
            !serde_json::to_string(&collected)
                .unwrap()
                .contains("canary"),
            "{trial}: the undeclared canary must stay outside the verifier"
        );
        let delivered = receipt["agent"]["terminal"]["delivered"]
            .as_array()
            .unwrap();
        assert!(
            delivered
                .iter()
                .any(|file| file["source"] == "/app/unrelated-canary.txt"),
            "{trial}: the canary was delivered by the Agent"
        );
        // The sealed bundle carries the verifier-input identity and the
        // collection manifest as bound native evidence.
        let bundle = run_root.join("trials").join(trial).join("bundle");
        assert!(
            bundle
                .join("artifacts/native/verifier-input.json")
                .is_file()
        );
        assert!(
            bundle
                .join("artifacts/native/artifacts-manifest.json")
                .is_file()
        );
    }
}

#[test]
fn native_tb30_rejects_undeclared_and_substituted_collection() {
    for (mode, kind) in [
        ("tb30-undeclared", "native-binding-undeclared-artifact"),
        (
            "tb30-substituted-declared",
            "native-binding-delivered-artifact-substituted",
        ),
    ] {
        let staged = stage("ok");
        set_tb30_mode(&staged, mode);
        let config = write_tb30_config(&staged);
        let run_root = staged.root.join("run-tb30");
        let (code, stdout, stderr) = run_native(&staged, &config, &run_root);
        assert_ne!(code, 0, "{mode}: stderr: {stderr}");
        let report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(report["outcome"], "incomplete", "{mode}");
        let receipt = report["trials"]
            .as_array()
            .unwrap()
            .iter()
            .find(|trial| trial["id"] == "trial-opi-native")
            .cloned()
            .unwrap();
        assert_eq!(receipt["status"], "failed", "{mode}");
        assert_eq!(receipt["verifier"]["failure_kind"], kind, "{mode}");
        assert!(
            !run_root
                .join("trials/trial-opi-native/bundle/manifest.json")
                .is_file(),
            "{mode}: rejected collection must not seal"
        );
    }
}

/// The DeepSWE experiment over the staged pier material with its
/// fixture task.
fn deepswe_experiment_text(staged: &Staged, integrity_digest: &str) -> String {
    experiment_text(staged, integrity_digest)
        .replace("native-driver-test", "native-deepswe-test")
        .replace("fixture-task", "synthetic-fixture-task")
        .replace(
            "terminal-bench\"\nrevision = \"2.1\"",
            "deepswe\"\nrevision = \"v1.1\"",
        )
}

fn write_deepswe_config(staged: &Staged) -> PathBuf {
    let config = staged.root.join("config-undigested-deepswe.toml");
    fs::write(&config, deepswe_experiment_text(staged, "pending")).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("validate")
        .arg("--config")
        .arg(&config)
        .arg("--native-material")
        .arg(&staged.material)
        .output()
        .expect("validate runs");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let digest = stdout
        .split_whitespace()
        .find(|token| token.starts_with("native_integrity="))
        .expect("native integrity digest in summary")
        .trim_start_matches("native_integrity=");
    let config = staged.root.join("deepswe.toml");
    fs::write(&config, deepswe_experiment_text(staged, digest)).unwrap();
    stage_output_control(staged, &config);
    config
}

/// Retarget the DeepSWE benchmark entry's stand-in behavior after
/// staging.
fn set_deepswe_mode(staged: &Staged, mode: &str) {
    let mut material: serde_json::Value =
        serde_json::from_slice(&fs::read(&staged.material).unwrap()).unwrap();
    material["benchmarks"]["deepswe-v1.1"]["verifier_env"]["OPI_EVAL_STANDIN_MODE"] =
        serde_json::Value::String(mode.to_owned());
    fs::write(
        &staged.material,
        serde_json::to_vec_pretty(&material).unwrap(),
    )
    .unwrap();
}

#[test]
fn native_deepswe_run_grades_the_committed_patch_in_the_pristine_verifier() {
    let staged = stage("ok");
    let config = write_deepswe_config(&staged);
    let run_root = staged.root.join("run-deepswe");
    let (code, stdout, stderr) = run_native(&staged, &config, &run_root);
    assert_eq!(code, 0, "stderr: {stderr}\nstdout: {stdout}");
    let report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(report["outcome"], "completed");

    for trial in ["trial-pi-native", "trial-opi-native"] {
        let receipt = report["trials"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["id"] == trial)
            .cloned()
            .unwrap_or_else(|| panic!("{trial} receipt missing"));
        assert_eq!(receipt["status"], "sealed", "{trial}: {receipt}");
        // The committed patch is the only verifier input: exactly the
        // declared collect artifact, with the digest of exactly the
        // collected bytes. The pristine verifier graded that patch - not
        // the mutable agent worktree.
        let collected = receipt["agent"]["collected"].as_array().unwrap();
        assert_eq!(collected.len(), 1, "{trial}: {collected:?}");
        assert_eq!(collected[0]["source"], "/logs/artifacts/model.patch");
        let patch_digest = collected[0]["sha256"].as_str().unwrap();
        assert_eq!(patch_digest.len(), 64);
        let job = run_root
            .join("trials")
            .join(trial)
            .join("verifier-trace/jobs/2026-09-05__12-00-00");
        let observed = sha256_hex(
            &fs::read(
                job.join("synthetic-fixture-task__standin/artifacts/logs/artifacts/model.patch"),
            )
            .unwrap(),
        );
        assert_eq!(patch_digest, &observed, "{trial}: bound digest");
        // The sealed bundle carries the verifier-input identity (mode,
        // patch digest, base revision, image) as bound native evidence.
        let bundle = run_root.join("trials").join(trial).join("bundle");
        let verifier_input: serde_json::Value = serde_json::from_slice(
            &fs::read(bundle.join("artifacts/native/verifier-input.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(verifier_input["environment_mode"], "separate");
        assert_eq!(
            verifier_input["base_revision"],
            "1111111111111111111111111111111111111112"
        );
        assert_eq!(
            verifier_input["docker_image"],
            format!(
                "fixture-registry/swe-bench-fixture@sha256:{}",
                "1".repeat(64)
            )
        );
    }
}

#[test]
fn native_deepswe_rejects_undeclared_collection_and_base_drift() {
    for (mode, kind) in [
        ("deepswe-undeclared", "native-binding-undeclared-artifact"),
        ("deepswe-base-drift", "native-binding-lifecycle-mismatch"),
    ] {
        let staged = stage("ok");
        set_deepswe_mode(&staged, mode);
        let config = write_deepswe_config(&staged);
        let run_root = staged.root.join("run-deepswe");
        let (code, stdout, stderr) = run_native(&staged, &config, &run_root);
        assert_ne!(code, 0, "{mode}: stderr: {stderr}");
        let report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(report["outcome"], "incomplete", "{mode}");
        let receipt = report["trials"]
            .as_array()
            .unwrap()
            .iter()
            .find(|trial| trial["id"] == "trial-opi-native")
            .cloned()
            .unwrap();
        assert_eq!(receipt["status"], "failed", "{mode}");
        assert_eq!(receipt["verifier"]["failure_kind"], kind, "{mode}");
        assert!(
            !run_root
                .join("trials/trial-opi-native/bundle/manifest.json")
                .is_file(),
            "{mode}: rejected collection must not seal"
        );
    }
}

#[test]
fn native_deepswe_nop_committed_nothing_grades_zero_not_rejection() {
    // The incorrect witness: the agent committed nothing, the collect
    // hook extracted an empty patch, which still collects (status ok)
    // and grades zero in the pristine verifier (P19-AC-005).
    let staged = stage("ok");
    set_deepswe_mode(&staged, "deepswe-nop");
    let config = write_deepswe_config(&staged);
    let run_root = staged.root.join("run-deepswe-nop");
    let (code, stdout, stderr) = run_native(&staged, &config, &run_root);
    assert_eq!(code, 0, "stderr: {stderr}\nstdout: {stdout}");
    let report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(report["outcome"], "completed");
    let receipt = report["trials"]
        .as_array()
        .unwrap()
        .iter()
        .find(|trial| trial["id"] == "trial-opi-native")
        .cloned()
        .unwrap();
    assert_eq!(receipt["status"], "sealed");
    let collected = receipt["agent"]["collected"].as_array().unwrap();
    assert_eq!(collected.len(), 1);
    // The empty patch digests as the sha256 of zero bytes.
    assert_eq!(
        collected[0]["sha256"],
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

fn run_conformance_native(
    staged: &Staged,
    suite: &str,
    adapter: &str,
    case: &str,
) -> (i32, String, String) {
    let root = staged.root.join(format!("conf-{suite}-{adapter}-{case}"));
    let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("conformance")
        .args(["--suite", suite])
        .args(["--adapter", adapter])
        .args(["--case", case])
        .arg("--root")
        .arg(&root)
        .arg("--fixtures")
        .arg(fixtures())
        .arg("--provider")
        .arg(manifest_dir().join("scripts/scripted-provider.py"))
        .arg("--native-material")
        .arg(&staged.material)
        .output()
        .expect("conformance executes");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn remove_agent_answer_output(staged: &Staged, agent: &str) {
    let executable = staged.root.join("material/agents").join(agent);
    let source = fs::read_to_string(&executable).unwrap();
    let answer_line = match agent {
        "opi" => "printf 'native stand-in answer\\n' > answer.txt\n",
        "pi" => "printf 'pi stand-in answer\\n' > answer.txt\n",
        other => panic!("unsupported staged agent {other}"),
    };
    let without_answer = source.replace(answer_line, "");
    assert_ne!(without_answer, source, "staged {agent} writes answer.txt");
    fs::write(&executable, without_answer).unwrap();

    let mut material: serde_json::Value =
        serde_json::from_slice(&fs::read(&staged.material).unwrap()).unwrap();
    material["agents"][agent]["executable"]["sha256"] =
        serde_json::Value::String(sha256_hex(&fs::read(&executable).unwrap()));
    fs::write(
        &staged.material,
        serde_json::to_vec_pretty(&material).unwrap(),
    )
    .unwrap();
}

#[test]
fn native_conformance_reruns_the_admitted_cases_through_the_material() {
    let staged = stage("ok");
    // Agent cases: both exact executables settle Completed with the real
    // native evidence contracts.
    for adapter in ["opi", "pi"] {
        let (code, stdout, stderr) = run_conformance_native(&staged, "agent", adapter, "completed");
        assert_eq!(code, 0, "{adapter}: {stderr}");
        let report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(report["met"], true, "{adapter}: {stdout}");
        assert_eq!(report["outcome"], "completed");
    }
    // Benchmark cases: the explicitly selected artifact witnesses through
    // the same production job handoff and binding - the oracle applies the
    // reference solution (reward one), nop delivers nothing (reward zero).
    // The former default-job conformance path no longer emits scores.
    for (adapter, cases) in [
        (
            "terminal-bench-2.1",
            [("oracle-witness", "oracle", 1), ("nop-witness", "nop", 0)],
        ),
        // The DeepSWE witnesses bind the same way through the Pier handoff:
        // the oracle's committed reference patch grades one, the nop's
        // empty patch grades zero (P19-AC-005).
        (
            "deepswe",
            [("oracle-witness", "oracle", 1), ("nop-witness", "nop", 0)],
        ),
    ] {
        for (case, agent, reward) in cases {
            let (code, stdout, stderr) =
                run_conformance_native(&staged, "benchmark", adapter, case);
            assert_eq!(code, 0, "{adapter}/{case}: {stderr}");
            let report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
            assert_eq!(report["met"], true, "{adapter}/{case}: {stdout}");
            assert_eq!(
                report["outcome"], "verified-native",
                "{adapter}/{case}: {stdout}"
            );
            let _ = (agent, reward);
        }
    }
}

#[test]
fn native_agent_conformance_requires_final_workspace_answer() {
    let staged = stage("ok");
    remove_agent_answer_output(&staged, "opi");
    let (code, stdout, stderr) = run_conformance_native(&staged, "agent", "opi", "completed");
    assert_eq!(code, 1, "stderr: {stderr}\nstdout: {stdout}");
    let report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(report["outcome"], "completed");
    assert_eq!(report["met"], false);
    assert!(
        report["notes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|note| note == "final-workspace-answer-missing-or-empty"),
        "{stdout}"
    );
}

#[test]
fn hermetic_only_conformance_cases_are_refused_in_native_mode() {
    let staged = stage("ok");
    let (code, _, stderr) = run_conformance_native(&staged, "agent", "opi", "timeout");
    assert_eq!(code, 2);
    assert!(stderr.contains("hermetic-only"), "stderr: {stderr}");
}

#[test]
fn oracle_witnesses_are_native_only_conformance_diagnostics() {
    let staged = stage("ok");
    // Without native material the witnesses are refused: they are not a
    // hermetic case.
    let root = staged.root.join("conf-hermetic-witness");
    let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("conformance")
        .args(["--suite", "benchmark"])
        .args(["--adapter", "terminal-bench-2.1"])
        .args(["--case", "oracle-witness"])
        .arg("--root")
        .arg(&root)
        .arg("--fixtures")
        .arg(fixtures())
        .arg("--provider")
        .arg(manifest_dir().join("scripts/scripted-provider.py"))
        .output()
        .expect("conformance executes");
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("native-only"),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// The real TB2.1 artifact witnesses through the pinned upstream runner
/// (P19-AC-003). Ignored by default: this filter requires the pinned
/// external material (fetched runner and task tree, materialized exact
/// built agents, a local Docker Linux engine) supplied through
/// `OPI_EVAL_NATIVE_WITNESS_MATERIAL` plus the loopback scripted
/// provider, and never a remote inference call.
#[test]
#[ignore = "requires pinned external material and a local Docker Linux engine; see OPI_EVAL_NATIVE_WITNESS_MATERIAL"]
fn native_tb21_artifact_witness() {
    let material = std::env::var("OPI_EVAL_NATIVE_WITNESS_MATERIAL")
        .expect("OPI_EVAL_NATIVE_WITNESS_MATERIAL points at the resolved material manifest");
    let material = PathBuf::from(material);
    for (case, agent, reward) in [("oracle-witness", "oracle", 1), ("nop-witness", "nop", 0)] {
        let root =
            std::env::temp_dir().join(format!("opi-eval-witness-{case}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
            .arg("conformance")
            .args(["--suite", "benchmark"])
            .args(["--adapter", "terminal-bench-2.1"])
            .args(["--case", case])
            .arg("--root")
            .arg(&root)
            .arg("--fixtures")
            .arg(fixtures())
            .arg("--provider")
            .arg(manifest_dir().join("scripts/scripted-provider.py"))
            .arg("--native-material")
            .arg(&material)
            .output()
            .expect("witness executes");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{case}: {stdout}\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(report["met"], true, "{case}: {stdout}");
        assert_eq!(report["outcome"], "verified-native", "{case}: {stdout}");
        let _ = (agent, reward);
        let _ = fs::remove_dir_all(&root);
    }
}

/// The real TB3 artifact witnesses through the pinned upstream runner
/// (P19-AC-004). Ignored by default: this filter requires the pinned
/// external material (fetched runner and task tree, materialized exact
/// built agents, a local Docker Linux engine) supplied through
/// `OPI_EVAL_NATIVE_WITNESS_MATERIAL`, and never a remote inference call.
/// The correct witness (oracle reference solution) and the incorrect
/// witness (nop agent) must change only the declared artifact: their
/// bound verifier-input digests differ, the native reward responds, and
/// nothing undeclared enters the verifier.
#[test]
#[ignore = "requires pinned external material and a local Docker Linux engine; see OPI_EVAL_NATIVE_WITNESS_MATERIAL"]
fn native_tb3_artifact_witness() {
    let material = std::env::var("OPI_EVAL_NATIVE_WITNESS_MATERIAL")
        .expect("OPI_EVAL_NATIVE_WITNESS_MATERIAL points at the resolved material manifest");
    let material = PathBuf::from(material);
    let mut collected_digests = Vec::new();
    for (case, agent, reward) in [("oracle-witness", "oracle", 1), ("nop-witness", "nop", 0)] {
        let root = std::env::temp_dir().join(format!(
            "opi-eval-tb30-witness-{case}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
            .arg("conformance")
            .args(["--suite", "benchmark"])
            .args(["--adapter", "terminal-bench-3.0"])
            .args(["--case", case])
            .arg("--root")
            .arg(&root)
            .arg("--fixtures")
            .arg(fixtures())
            .arg("--provider")
            .arg(manifest_dir().join("scripts/scripted-provider.py"))
            .arg("--native-material")
            .arg(&material)
            .output()
            .expect("witness executes");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{case}: {stdout}\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(report["met"], true, "{case}: {stdout}");
        assert_eq!(report["outcome"], "verified-native", "{case}: {stdout}");
        // The bound collected identity: exactly the declared artifact, and
        // nothing undeclared (the unrelated canary marker stays outside).
        let collected: Vec<String> = report["notes"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|note| note.as_str())
            .filter(|note| note.starts_with("native-collected: "))
            .map(str::to_owned)
            .collect();
        assert_eq!(collected.len(), 1, "{case}: {collected:?}");
        assert!(
            collected[0].starts_with("native-collected: /app/evalbench/ "),
            "{case}: {collected:?}"
        );
        assert!(!collected[0].contains("canary"), "{case}: {collected:?}");
        collected_digests.push((agent, reward, collected[0].clone()));
        let _ = fs::remove_dir_all(&root);
    }
    // The correct witness changed the declared artifact and the grader
    // responded: same declared source, different collected bytes, reward
    // one versus zero.
    assert_ne!(collected_digests[0].2, collected_digests[1].2);
}

/// The real DeepSWE committed-patch witnesses through the pinned
/// upstream Pier runner (P19-AC-005). Ignored by default: this filter
/// requires the pinned external material (fetched runner and task tree,
/// materialized exact built agents, a local Docker Linux engine) supplied
/// through `OPI_EVAL_NATIVE_WITNESS_MATERIAL`, and never a remote
/// inference call. The correct witness (oracle reference solution, reward
/// one) and the incorrect witness (nop agent, empty committed patch,
/// reward zero) must change only the collected committed patch: their
/// bound verifier-input digests differ, the native reward responds, and
/// nothing undeclared enters the pristine verifier.
#[test]
#[ignore = "requires pinned external material and a local Docker Linux engine; see OPI_EVAL_NATIVE_WITNESS_MATERIAL"]
fn native_deepswe_patch_witness() {
    let material = std::env::var("OPI_EVAL_NATIVE_WITNESS_MATERIAL")
        .expect("OPI_EVAL_NATIVE_WITNESS_MATERIAL points at the resolved material manifest");
    let material = PathBuf::from(material);
    let mut collected_digests = Vec::new();
    for (case, agent, reward) in [("oracle-witness", "oracle", 1), ("nop-witness", "nop", 0)] {
        let root = std::env::temp_dir().join(format!(
            "opi-eval-deepswe-witness-{case}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
            .arg("conformance")
            .args(["--suite", "benchmark"])
            .args(["--adapter", "deepswe"])
            .args(["--case", case])
            .arg("--root")
            .arg(&root)
            .arg("--fixtures")
            .arg(fixtures())
            .arg("--provider")
            .arg(manifest_dir().join("scripts/scripted-provider.py"))
            .arg("--native-material")
            .arg(&material)
            .output()
            .expect("witness executes");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{case}: {stdout}\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(report["met"], true, "{case}: {stdout}");
        assert_eq!(report["outcome"], "verified-native", "{case}: {stdout}");
        // The bound verifier-input identity: exactly the declared
        // committed patch, and nothing undeclared.
        let collected: Vec<String> = report["notes"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|note| note.as_str())
            .filter(|note| note.starts_with("native-collected: "))
            .map(str::to_owned)
            .collect();
        assert_eq!(collected.len(), 1, "{case}: {collected:?}");
        assert!(
            collected[0].starts_with("native-collected: /logs/artifacts/model.patch "),
            "{case}: {collected:?}"
        );
        assert!(!collected[0].contains("canary"), "{case}: {collected:?}");
        collected_digests.push((agent, reward, collected[0].clone()));
        let _ = fs::remove_dir_all(&root);
    }
    // The correct witness committed a different patch and the pristine
    // verifier's reward responded: same declared source, different
    // collected bytes, reward one versus zero.
    assert_ne!(collected_digests[0].2, collected_digests[1].2);
}

#[test]
fn native_required_output_control_rejects_unproven_binary_before_effects() {
    let staged = stage("ok");
    let config = write_config(&staged);
    let mut material: serde_json::Value =
        serde_json::from_slice(&fs::read(&staged.material).unwrap()).unwrap();
    for agent in material["agents"].as_object_mut().unwrap().values_mut() {
        agent
            .as_object_mut()
            .unwrap()
            .remove("output_control_evidence");
    }
    fs::write(&staged.material, serde_json::to_vec(&material).unwrap()).unwrap();
    fs::write(
        &config,
        fs::read_to_string(&config).unwrap().replace(
            "max_output_tokens = 4096",
            "max_output_tokens = 4096\nrequired = [\"max_output_tokens\"]",
        ),
    )
    .unwrap();
    let run_root = staged.root.join("unproven-run");
    let (code, _, stderr) = run_native(&staged, &config, &run_root);
    assert_eq!(code, 2, "{stderr}");
    assert!(stderr.contains("output-control"), "{stderr}");
    assert!(
        !run_root.join("trials").exists(),
        "no trial before control admission"
    );
    let plan = staged.root.join("unproven-plan");
    let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .args(["plan", "--config"])
        .arg(&config)
        .arg("--native-material")
        .arg(&staged.material)
        .arg("--out")
        .arg(&plan)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("output-control"));
    assert!(!plan.exists());
}

#[test]
fn native_optional_output_control_without_proof_is_not_comparable() {
    let staged = stage("ok");
    let config = write_config(&staged);
    let mut material: serde_json::Value =
        serde_json::from_slice(&fs::read(&staged.material).unwrap()).unwrap();
    for agent in material["agents"].as_object_mut().unwrap().values_mut() {
        agent
            .as_object_mut()
            .unwrap()
            .remove("output_control_evidence");
    }
    fs::write(&staged.material, serde_json::to_vec(&material).unwrap()).unwrap();
    let (code, stdout, stderr) = run_native(&staged, &config, &staged.root.join("optional-run"));
    assert_eq!(code, 1, "{stderr}");
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        report["pairs"][0]["comparability"],
        "unsupported-control:max_output_tokens"
    );
    assert!(
        report["trials"]
            .as_array()
            .unwrap()
            .iter()
            .all(|trial| trial["status"] == "sealed")
    );
}

#[test]
fn native_output_control_binds_projection_wire_and_exact_client() {
    for case in [
        "valid",
        "wire",
        "binary",
        "package",
        "entrypoint",
        "config",
        "required",
        "missing-log",
    ] {
        let staged = stage("ok");
        let config = write_config(&staged);
        fs::write(
            &config,
            fs::read_to_string(&config).unwrap().replace(
                "max_output_tokens = 4096",
                "max_output_tokens = 4096\nrequired = [\"max_output_tokens\"]",
            ),
        )
        .unwrap();
        if case == "entrypoint" {
            let mut material: serde_json::Value =
                serde_json::from_slice(&fs::read(&staged.material).unwrap()).unwrap();
            let executable = PathBuf::from(
                material["agents"]["opi"]["executable"]["path"]
                    .as_str()
                    .unwrap(),
            );
            let package = executable.parent().unwrap();
            fs::create_dir(package.join("alternate")).unwrap();
            fs::copy(&executable, package.join("alternate/opi")).unwrap();
            material["agents"]["opi"]["runtime_package"] = serde_json::json!({
                "path": package, "manifest_sha256": directory_manifest_digest(package), "entrypoint": "opi",
            });
            fs::write(&staged.material, serde_json::to_vec(&material).unwrap()).unwrap();
        }
        stage_output_control(&staged, &config);
        let mut material: serde_json::Value =
            serde_json::from_slice(&fs::read(&staged.material).unwrap()).unwrap();
        let pin = &mut material["agents"]["opi"]["output_control_evidence"];
        let path = PathBuf::from(pin["path"].as_str().unwrap());
        let mut proof: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        match case {
            "wire" | "config" => {
                let key = if case == "wire" { "requests" } else { "config" };
                let file = Path::new(proof[key]["path"].as_str().unwrap());
                let bytes = fs::read_to_string(file).unwrap().replace("4096", "8192");
                fs::write(file, &bytes).unwrap();
                proof[key]["sha256"] = sha256_hex(bytes.as_bytes()).into();
            }
            "binary" => proof["executable_sha256"] = "f".repeat(64).into(),
            "package" => proof["runtime_package_manifest_sha256"] = "f".repeat(64).into(),
            "entrypoint" => proof["runtime_package_entrypoint"] = "alternate/opi".into(),
            "required" => proof["model_controls"]["max_output_tokens"] = "unknown".into(),
            "missing-log" => fs::remove_file(proof["requests"]["path"].as_str().unwrap()).unwrap(),
            _ => {}
        }
        let bytes = serde_json::to_vec(&proof).unwrap();
        fs::write(&path, &bytes).unwrap();
        pin["sha256"] = sha256_hex(&bytes).into();
        fs::write(&staged.material, serde_json::to_vec(&material).unwrap()).unwrap();
        let run_root = staged.root.join("proven-run");
        let (code, stdout, stderr) = run_native(&staged, &config, &run_root);
        if case == "valid" {
            assert_eq!(code, 0, "{stdout} {stderr}");
            for trial in ["trial-opi-native", "trial-pi-native"] {
                assert!(
                    run_root
                        .join("trials")
                        .join(trial)
                        .join("bundle/artifacts/control/output-control.json")
                        .is_file()
                );
            }
        } else {
            assert_eq!(code, 2, "{case}: {stderr}");
            assert!(stderr.contains("output-control"), "{case}: {stderr}");
            assert!(!run_root.join("trials").exists());
        }
    }
}

#[test]
fn native_missing_credential_rejects_before_any_effect() {
    let staged = stage("ok");
    let config = write_config(&staged);
    let run_root = staged.root.join("run");
    // The declared credential reference resolves to nothing: the request
    // must be rejected before any model effect, bundle, or job exists.
    let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("run")
        .arg("--config")
        .arg(&config)
        .arg("--root")
        .arg(&run_root)
        .arg("--fixtures")
        .arg(fixtures())
        .arg("--native-material")
        .arg(&staged.material)
        .env_remove("OPI_EVAL_SCRIPTED_KEY")
        .output()
        .expect("run executes");
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("credential"), "stderr: {stderr}");
    assert!(
        !run_root
            .join("trials/trial-opi-native/bundle/intent.json")
            .is_file(),
        "no durable effect may exist before credential admission"
    );
    assert!(
        !run_root
            .join("trials/trial-opi-native/verifier-trace/jobs")
            .is_dir(),
        "no job may exist before credential admission"
    );
}

#[test]
fn native_mode_gate_rejects_inconsistent_requests() {
    let staged = stage("ok");
    let config = write_config(&staged);
    let run_root = staged.root.join("run");
    // Hermetic mode with native material: inconsistent.
    let hermetic = fs::read_to_string(&config)
        .unwrap()
        .replace("[execution]\nmode = \"native-fixture\"", "");
    let hermetic_config = staged.root.join("hermetic.toml");
    fs::write(&hermetic_config, hermetic).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("run")
        .arg("--config")
        .arg(&hermetic_config)
        .arg("--root")
        .arg(&run_root)
        .arg("--fixtures")
        .arg(fixtures())
        .arg("--native-material")
        .arg(&staged.material)
        .env("OPI_EVAL_SCRIPTED_KEY", "<dummy-scripted-credential>")
        .output()
        .expect("run executes");
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("mode"), "stderr: {stderr}");
}

// --- P19-AC-006-fixture: the real-client protocol matrix ---------------

/// Drives both actual pinned clients (the exact built Opi binary and the
/// pinned pi bundle under its resolved Node) through all three admitted
/// protocol paths against the loopback scripted provider, using the
/// production `project_connection` bytes for every configuration. Runs
/// only with locally supplied client material
/// (`OPI_EVAL_NATIVE_CLIENT_MATERIAL` = JSON with `opi`, `node`, and
/// `pi_bundle` paths) and never contacts a remote service.
#[test]
#[ignore = "requires locally supplied client material (OPI_EVAL_NATIVE_CLIENT_MATERIAL)"]
fn native_protocol_client_matrix() {
    let material = std::env::var("OPI_EVAL_NATIVE_CLIENT_MATERIAL")
        .expect("OPI_EVAL_NATIVE_CLIENT_MATERIAL points at the client material JSON");
    let clients: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(material).unwrap()).unwrap();
    let opi_bin = PathBuf::from(clients["opi"].as_str().unwrap());
    let node_bin = PathBuf::from(clients["node"].as_str().unwrap());
    let pi_bundle = PathBuf::from(clients["pi_bundle"].as_str().unwrap());
    assert!(opi_bin.is_file() && node_bin.is_file() && pi_bundle.is_file());

    let root = tempfile::tempdir().unwrap();
    // One fresh listener per client and API: each combination's full
    // matrix (tool loops, retry history, and exhausted rate limits) stays
    // inside the provider's bounded request cap, the retry script's
    // one-failure state applies per client, and the per-combination log
    // isolates the authentication evidence.
    let mut providers: Vec<std::process::Child> = Vec::new();
    let apis = [
        "anthropic-messages",
        "openai-completions",
        "openai-responses",
    ];
    let failures = [
        ("auth-fail", "provider-auth-failure"),
        ("rate-limit", "provider-rate-limit-exhausted"),
        ("protocol-error", "provider-protocol-error"),
    ];

    for api in apis {
        for client in [
            opi_eval::experiment::NativeClient::Opi,
            opi_eval::experiment::NativeClient::Pi,
        ] {
            let log = root
                .path()
                .join(format!("requests-{api}-{}.jsonl", client.as_str()));
            let (provider, base) = start_scripted_provider(&log);
            providers.push(provider);
            let base = base.unwrap();
            for model in ["eval", "auth-fail", "rate-limit", "retry", "protocol-error"] {
                let connection = opi_eval::experiment::Connection {
                    id: "matrix".to_owned(),
                    provider: "scripted".to_owned(),
                    api: match api {
                        "anthropic-messages" => {
                            opi_eval::experiment::ConnectionApi::AnthropicMessages
                        }
                        "openai-completions" => {
                            opi_eval::experiment::ConnectionApi::OpenAiCompletions
                        }
                        _ => opi_eval::experiment::ConnectionApi::OpenAiResponses,
                    },
                    base_url: base.clone(),
                    model: model.to_owned(),
                    credential: opi_eval::experiment::CredentialReference::Env {
                        variable: "OPI_EVAL_SCRIPTED_KEY".to_owned(),
                    },
                };
                let controls = matrix_controls();
                let credential = opi_eval::experiment::ResolvedCredential::FromEnv {
                    variable: "OPI_EVAL_SCRIPTED_KEY".to_owned(),
                    value: "dummy-scripted-credential".to_owned(),
                };
                let projection = opi_eval::experiment::project_connection(
                    &connection,
                    &controls,
                    client,
                    credential,
                )
                .unwrap();
                let case_root = root
                    .path()
                    .join(format!("{}-{}-{}", client.as_str(), api, model));
                let work = case_root.join("work");
                std::fs::create_dir_all(&work).unwrap();
                std::fs::create_dir_all(case_root.join("cfg")).unwrap();
                let config_path = case_root.join("cfg").join(projection.config_filename);
                std::fs::write(&config_path, &projection.config_bytes).unwrap();

                let output = match client {
                    opi_eval::experiment::NativeClient::Opi => {
                        run_opi_client(&opi_bin, &config_path, model, &case_root, &work)
                    }
                    opi_eval::experiment::NativeClient::Pi => run_pi_client(
                        &node_bin,
                        &pi_bundle,
                        &config_path,
                        model,
                        &case_root,
                        &work,
                    ),
                };
                let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
                let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
                let code = output.status.code().unwrap_or(-1);
                let name = format!("{}/{} {}", client.as_str(), api, model);

                match model {
                    "eval" | "retry" => {
                        // The deterministic tool loop completes; one
                        // intermediate 429 (retry) never becomes the final
                        // result.
                        assert_eq!(code, 0, "{name}: {stderr}");
                        if client == opi_eval::experiment::NativeClient::Pi {
                            assert!(
                                stdout.contains("\"stopReason\":\"stop\"")
                                    || stdout.contains("\"stopReason\": \"stop\""),
                                "{name}: no terminal stop: {stdout}"
                            );
                        }
                    }
                    script => {
                        let kind = failures.iter().find(|(id, _)| *id == script).unwrap().1;
                        match client {
                            opi_eval::experiment::NativeClient::Opi => {
                                // Exit 4 is the typed provider failure; the
                                // typed summary is on stderr.
                                assert_eq!(code, 4, "{name}: {stderr}");
                                let summary = match kind {
                                    "provider-auth-failure" => "provider authentication failed",
                                    "provider-rate-limit-exhausted" => "rate limited by provider",
                                    _ => "provider stream failed",
                                };
                                assert!(
                                    stderr.contains(summary),
                                    "{name}: typed summary missing: {stderr}"
                                );
                            }
                            opi_eval::experiment::NativeClient::Pi => {
                                // JSON error at process exit zero: the
                                // client's typed terminal evidence rules.
                                assert_eq!(code, 0, "{name}: {stderr}");
                                // The pinned client's own typed protocol
                                // diagnostic differs per wire.
                                let evidence = match kind {
                                    "provider-auth-failure" => "401",
                                    "provider-rate-limit-exhausted" => "429",
                                    _ => match api {
                                        "anthropic-messages" => {
                                            "Anthropic stream ended without a stop reason"
                                        }
                                        "openai-responses" => {
                                            "OpenAI Responses stream ended before a terminal response event"
                                        }
                                        _ => "Stream ended without finish_reason",
                                    },
                                };
                                assert!(
                                    stdout.contains("\"stopReason\":\"error\"")
                                        || stdout.contains("\"stopReason\": \"error\""),
                                    "{name}: no typed error stop: {stdout}"
                                );
                                assert!(
                                    stdout.contains(evidence),
                                    "{name}: evidence {evidence} missing: {stdout}"
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    // Authentication evidence: every admitted request carried the
    // projected credential through each API's native header scheme, and
    // the retry script produced exactly one failure followed by the
    // successful tool loop per client.
    for api in apis {
        for client in ["opi", "pi"] {
            let requests: Vec<serde_json::Value> =
                std::fs::read_to_string(root.path().join(format!("requests-{api}-{client}.jsonl")))
                    .unwrap()
                    .lines()
                    .map(|line| serde_json::from_str(line).unwrap())
                    .collect();
            assert!(
                requests.iter().all(|r| r["auth"].is_string()),
                "{api}/{client}: unauthenticated request admitted"
            );
            for request in &requests {
                assert_wire_output_limit(request, api, 512);
            }
            let retry: Vec<&serde_json::Value> = requests
                .iter()
                .filter(|r| r["model"].as_str() == Some("retry"))
                .collect();
            // Exactly one retryable 429, then only successes: one
            // intermediate HTTP error never becomes the final result.
            let statuses: Vec<u64> = retry
                .iter()
                .map(|r| r["status"].as_u64().unwrap())
                .collect();
            assert_eq!(
                statuses.iter().filter(|status| **status == 429).count(),
                1,
                "{api}/{client}: exactly one retryable failure: {statuses:?}"
            );
            assert!(
                statuses
                    .iter()
                    .all(|status| *status == 429 || *status == 200),
                "{api}/{client}: unexpected retry-script status: {statuses:?}"
            );
            assert_eq!(
                statuses.last(),
                Some(&200),
                "{api}/{client}: the final retry-script request succeeded: {statuses:?}"
            );
        }
    }
    for mut provider in providers {
        let _ = provider.kill();
    }
}

/// Exercises both actual clients with the same isolated long-loop provider.
/// Default and projected configurations complete the long task; a supplied
/// Opi product budget still bounds continuation.
#[test]
#[ignore = "requires locally supplied client material (OPI_EVAL_NATIVE_CLIENT_MATERIAL)"]
fn native_client_turn_budgets() {
    use opi_eval::experiment::{
        Connection, ConnectionApi, ControlValue, CredentialReference, NativeClient,
        ResolvedCredential, project_connection,
    };
    use std::io::BufRead as _;

    struct Provider(std::process::Child);
    impl Drop for Provider {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    let material = std::env::var("OPI_EVAL_NATIVE_CLIENT_MATERIAL").unwrap();
    let clients: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(material).unwrap()).unwrap();
    let bin = PathBuf::from(clients["opi"].as_str().unwrap());
    let pi = PathBuf::from(clients["pi"].as_str().unwrap());
    assert!(bin.is_absolute() && bin.is_file());
    assert!(pi.is_absolute() && pi.is_file());
    let root = tempfile::tempdir().unwrap();
    let script = root.path().join("long_loop.py");
    fs::write(
        &script,
        r#"import importlib.util, sys
spec = importlib.util.spec_from_file_location('scripted', sys.argv[1])
provider = importlib.util.module_from_spec(spec)
spec.loader.exec_module(provider)
def final_turn(body):
    results = [block for message in body.get('messages', [])
               if message.get('role') == 'user'
               and isinstance(message.get('content'), list)
               for block in message['content']
               if block.get('type') == 'tool_result']
    assert all(not result.get('is_error') for result in results)
    provider.TOOL_USE_ID = f'toolu_scripted_{len(results) + 1}'
    return len(results) >= 51
provider.wants_final_turn = final_turn
sys.argv = [sys.argv[1], '--listen', '127.0.0.1:0', '--request-log', sys.argv[2]]
sys.exit(provider.main())
"#,
    )
    .unwrap();

    for (case, expected_requests) in [
        ("default", 52),
        ("explicit-two", 2),
        ("projected", 52),
        ("pi-projected", 52),
        ("omitted-output", 0),
    ] {
        let case_root = root.path().join(case);
        let work = case_root.join("work");
        fs::create_dir_all(&work).unwrap();
        let log = case_root.join("requests.jsonl");
        let mut provider = Provider(
            Command::new("python3")
                .arg(&script)
                .arg(manifest_dir().join("scripts/scripted-provider.py"))
                .arg(&log)
                .env_clear()
                .env("PATH", "/usr/bin:/bin")
                .env("PYTHONDONTWRITEBYTECODE", "1")
                .stdout(std::process::Stdio::piped())
                .spawn()
                .unwrap(),
        );
        let mut ready = String::new();
        std::io::BufReader::new(provider.0.stdout.take().unwrap())
            .read_line(&mut ready)
            .unwrap();
        let endpoint = ready.trim().strip_prefix("listening ").unwrap();
        let connection = Connection {
            id: "long-loop".to_owned(),
            provider: "scripted".to_owned(),
            api: ConnectionApi::AnthropicMessages,
            base_url: format!("http://{endpoint}"),
            model: "eval".to_owned(),
            credential: CredentialReference::Env {
                variable: "OPI_EVAL_SCRIPTED_KEY".to_owned(),
            },
        };
        let mut controls = matrix_controls();
        controls.context_window = ControlValue::Value(1_000_000);
        controls.max_output_tokens = if case == "omitted-output" {
            ControlValue::Omitted
        } else {
            ControlValue::Value(131_072)
        };
        let projection = project_connection(
            &connection,
            &controls,
            if case == "pi-projected" {
                NativeClient::Pi
            } else {
                NativeClient::Opi
            },
            ResolvedCredential::FromEnv {
                variable: "OPI_EVAL_SCRIPTED_KEY".to_owned(),
                value: "dummy-scripted-credential".to_owned(),
            },
        )
        .unwrap();
        let config = case_root.join(projection.config_filename);
        let mut bytes = projection.config_bytes;
        if case != "pi-projected" {
            let projected: toml::Value =
                toml::from_str(std::str::from_utf8(&bytes).unwrap()).unwrap();
            assert!(
                projected
                    .get("defaults")
                    .and_then(|v| v.get("max_iterations"))
                    .is_none(),
                "evaluation projection must not supply a turn budget"
            );
        }
        if matches!(case, "default" | "explicit-two") {
            let mut value: toml::Value =
                toml::from_str(std::str::from_utf8(&bytes).unwrap()).unwrap();
            let defaults = value["defaults"].as_table_mut().unwrap();
            defaults.remove("max_iterations");
            if case == "explicit-two" {
                defaults.insert("max_iterations".to_owned(), 2.into());
            }
            bytes = toml::to_string(&value).unwrap().into_bytes();
        }
        fs::write(&config, bytes).unwrap();
        let output = if case == "pi-projected" {
            run_pi_client_for(&pi, None, &config, "scripted", "eval", &case_root, &work)
        } else {
            run_opi_client(&bin, &config, "eval", &case_root, &work)
        };
        drop(provider);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let requests: Vec<serde_json::Value> = if case == "omitted-output" {
            // A custom model needs a numeric output capability. Omitting it
            // must remain a pre-dispatch refusal, not an invented limit.
            assert!(!log.exists(), "{case}: provider was contacted");
            Vec::new()
        } else {
            fs::read_to_string(&log)
                .unwrap_or_else(|error| panic!("{case}: {error}; {stderr}"))
                .lines()
                .map(|line| serde_json::from_str(line).unwrap())
                .collect()
        };
        eprintln!(
            "{case}: requests={}, exit={:?}",
            requests.len(),
            output.status.code()
        );
        assert_eq!(requests.len(), expected_requests, "{case}: {stderr}");
        if expected_requests == 52 {
            assert!(output.status.success(), "{case}: {stderr}");
            assert!(
                String::from_utf8_lossy(&output.stdout)
                    .contains("scripted-provider: integration turn complete")
            );
        } else if case == "omitted-output" {
            assert!(!output.status.success(), "{case}: {stderr}");
            assert!(stderr.contains("max_output_tokens"), "{stderr}");
        } else {
            assert_eq!(output.status.code(), Some(1), "{case}: {stderr}");
            assert!(stderr.contains(&format!("max turns exceeded ({expected_requests})")));
        }
        if case != "omitted-output" {
            for request in requests {
                assert_wire_output_limit(&request, "anthropic-messages", 131_072);
            }
        }
    }
}

fn matrix_controls() -> opi_eval::experiment::ModelControls {
    use opi_eval::experiment::ControlValue;
    opi_eval::experiment::ModelControls {
        temperature: ControlValue::Value(0.0),
        top_p: ControlValue::Omitted,
        max_output_tokens: ControlValue::Value(512),
        context_window: ControlValue::Value(8192),
        reasoning: ControlValue::Omitted,
        required: Vec::new(),
    }
}

fn assert_wire_output_limit(request: &serde_json::Value, api: &str, expected: u64) {
    let limits = request["output_limits"]
        .as_object()
        .expect("listener observed output limits");
    assert_eq!(limits.len(), 1, "one unambiguous wire limit: {request}");
    let fields: &[&str] = match api {
        "anthropic-messages" => &["max_tokens"],
        "openai-completions" => &["max_tokens", "max_completion_tokens"],
        "openai-responses" => &["max_output_tokens"],
        _ => panic!("unsupported API"),
    };
    assert!(
        fields
            .iter()
            .any(|key| limits.get(*key).and_then(serde_json::Value::as_u64) == Some(expected)),
        "wrong wire limit: {request}"
    );
}

fn probe_package_identity(executable: &Path, package: &Path) -> Result<(String, PathBuf), String> {
    // Match NativeMaterial's package contract: an absolute regular-file tree
    // and a normal relative entrypoint bound to the executable actually run.
    if !package.is_absolute() || !package.is_dir() {
        return Err("runtime package requires an absolute directory".to_owned());
    }
    let entrypoint = executable
        .strip_prefix(package)
        .map_err(|_| "runtime package does not contain the executable")?;
    if entrypoint.as_os_str().is_empty()
        || entrypoint
            .components()
            .any(|part| !matches!(part, std::path::Component::Normal(_)))
        || entrypoint.to_str().is_none_or(|path| {
            path.chars()
                .any(|c| c.is_control() || matches!(c, '\\' | ':'))
                || path.split('/').any(|part| part.is_empty() || part == ".")
        })
    {
        return Err("runtime package entrypoint must be a normal relative path".to_owned());
    }
    fn check(path: &Path) -> Result<(), String> {
        let kind = fs::symlink_metadata(path)
            .map_err(|error| error.to_string())?
            .file_type();
        if kind.is_symlink() || !(kind.is_file() || kind.is_dir()) {
            return Err("runtime package contains a link or special file".to_owned());
        }
        if kind.is_dir() {
            for entry in fs::read_dir(path).map_err(|error| error.to_string())? {
                let entry = entry.map_err(|error| error.to_string())?;
                if entry.file_name().to_str().is_none_or(|name| {
                    name.chars()
                        .any(|c| c.is_control() || matches!(c, '\\' | ':'))
                }) {
                    return Err("runtime package contains an ambiguous path".to_owned());
                }
                check(&entry.path())?;
            }
        }
        Ok(())
    }
    for ancestor in package.ancestors() {
        if fs::symlink_metadata(ancestor)
            .map_err(|error| error.to_string())?
            .file_type()
            .is_symlink()
        {
            return Err("runtime package root traverses a link".to_owned());
        }
    }
    check(package)?;
    if !package.join(entrypoint).is_file()
        || package
            .join(entrypoint)
            .canonicalize()
            .map_err(|error| error.to_string())?
            != executable
                .canonicalize()
                .map_err(|error| error.to_string())?
    {
        return Err("runtime package entrypoint differs from executable".to_owned());
    }
    Ok((directory_manifest_digest(package), entrypoint.to_path_buf()))
}

#[test]
fn output_control_probe_rejects_unrelated_runtime_package() {
    let root = tempfile::tempdir().unwrap();
    let first = root.path().join("first");
    let second = root.path().join("second");
    for (package, resource) in [
        (&first, "first resources"),
        (&second, "different resources"),
    ] {
        fs::create_dir_all(package.join("bin")).unwrap();
        fs::write(package.join("bin/pi"), b"identical executable").unwrap();
        fs::write(package.join("resource"), resource).unwrap();
    }
    assert!(
        probe_package_identity(&first.join("bin/pi"), &second).is_err(),
        "equal executable bytes cannot bind the other package's resources"
    );
    let identity = probe_package_identity(&first.join("bin/pi"), &first).unwrap();
    assert_eq!(identity.1, PathBuf::from("bin/pi"));
    assert!(probe_package_identity(&first.join("../second/bin/pi"), &first).is_err());
    std::os::unix::fs::symlink(second.join("bin/pi"), first.join("linked-pi")).unwrap();
    assert!(probe_package_identity(&first.join("linked-pi"), &first).is_err());
}

/// Offline evidence for the exact projected output control. Supply an
/// `experiment` config path and `opi` plus native `pi` (or `node`/`pi_bundle`)
/// in OPI_EVAL_NATIVE_CLIENT_MATERIAL. Optional `opi_package`/`pi_package`
/// roots bind transported resources. OPI_EVAL_OUTPUT_CONTROL_OUT must not
/// exist. The original endpoint and credential are never used or resolved.
#[test]
#[ignore = "requires exact local clients, experiment, and a fresh OPI_EVAL_OUTPUT_CONTROL_OUT"]
fn native_output_control_client_matrix() {
    use opi_eval::experiment::{
        ControlValue, CredentialReference, NativeClient, ResolvedCredential, ResolvedExperiment,
    };
    let input = std::env::var("OPI_EVAL_NATIVE_CLIENT_MATERIAL").unwrap();
    let clients: serde_json::Value = serde_json::from_slice(&fs::read(input).unwrap()).unwrap();
    let experiment = ResolvedExperiment::resolve(
        &fs::read_to_string(clients["experiment"].as_str().unwrap()).unwrap(),
    )
    .unwrap();
    let ControlValue::Value(limit) = experiment.model_controls().max_output_tokens else {
        panic!("output-control probe requires a numeric control");
    };
    assert!(limit > 0);
    assert!(
        !matches!(
            experiment.model_controls().reasoning,
            ControlValue::Value(_)
        ),
        "this output-control probe does not attest reasoning effort"
    );
    let root = PathBuf::from(std::env::var("OPI_EVAL_OUTPUT_CONTROL_OUT").unwrap());
    assert!(root.is_absolute());
    // Validate all local client inputs before creating evidence or starting a listener.
    for key in if clients["pi"].is_string() {
        vec!["opi", "pi"]
    } else {
        vec!["opi", "node", "pi_bundle"]
    } {
        assert!(
            Path::new(clients[key].as_str().unwrap()).is_file(),
            "missing {key}"
        );
    }
    let mut package_identities = std::collections::BTreeMap::new();
    for product in ["opi", "pi"] {
        let executable_key = if product == "pi" && !clients["pi"].is_string() {
            "node"
        } else {
            product
        };
        let executable = Path::new(clients[executable_key].as_str().unwrap());
        let package = clients[format!("{product}_package")]
            .as_str()
            .map(Path::new);
        let identity = package.map(|package| probe_package_identity(executable, package).unwrap());
        if executable_key == "node" {
            // The interpreter digest alone does not identify the pi code.
            let package =
                package.expect("node/pi_bundle proof requires pi_package containing both files");
            probe_package_identity(Path::new(clients["pi_bundle"].as_str().unwrap()), package)
                .unwrap();
        }
        package_identities.insert(product, identity);
    }
    fs::create_dir(&root).expect("output-control evidence root must be fresh");
    for client in [NativeClient::Opi, NativeClient::Pi] {
        let product = client.as_str();
        let case = root.join(product);
        let work = case.join("work");
        fs::create_dir_all(&work).unwrap();
        fs::write(work.join(".env"), "").unwrap();
        let log = case.join("requests.jsonl");
        let (mut listener, endpoint) = start_scripted_provider(&log);
        let mut connection = experiment.selected_connection().clone();
        connection.base_url = endpoint.unwrap();
        connection.credential = CredentialReference::Env {
            variable: "OPI_EVAL_SCRIPTED_KEY".to_owned(),
        };
        let projection = opi_eval::experiment::project_connection(
            &connection,
            experiment.model_controls(),
            client,
            ResolvedCredential::FromEnv {
                variable: "OPI_EVAL_SCRIPTED_KEY".to_owned(),
                value: "dummy-scripted-credential".to_owned(),
            },
        )
        .unwrap();
        let config = case.join(projection.config_filename);
        fs::write(&config, &projection.config_bytes).unwrap();
        let executable = PathBuf::from(
            clients[if client == NativeClient::Pi && !clients["pi"].is_string() {
                "node"
            } else {
                product
            }]
            .as_str()
            .unwrap(),
        );
        let executable_sha256 = sha256_hex(&fs::read(&executable).unwrap());
        let package = clients[format!("{product}_package")]
            .as_str()
            .map(PathBuf::from);
        let package_identity = &package_identities[product];
        let package_digest = package_identity.as_ref().map(|identity| &identity.0);
        let package_entrypoint = package_identity.as_ref().map(|identity| &identity.1);
        let output = if client == NativeClient::Opi {
            run_opi_client_for(
                &executable,
                &config,
                &connection.provider,
                &connection.model,
                &case,
                &work,
            )
        } else {
            let bundle = if clients["pi"].is_string() {
                None
            } else {
                Some(Path::new(clients["pi_bundle"].as_str().unwrap()))
            };
            run_pi_client_for(
                &executable,
                bundle,
                &config,
                &connection.provider,
                &connection.model,
                &case,
                &work,
            )
        };
        let _ = listener.kill();
        listener.wait().unwrap();
        fs::write(case.join("stdout.jsonl"), &output.stdout).unwrap();
        fs::write(case.join("stderr.log"), &output.stderr).unwrap();
        fs::write(
            case.join("exit.json"),
            serde_json::to_vec(&serde_json::json!({"exit_code": output.status.code()})).unwrap(),
        )
        .unwrap();
        let requests = fs::read(&log).unwrap();
        let rows: Vec<serde_json::Value> = std::str::from_utf8(&requests)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(
            output.status.code(),
            Some(0),
            "{product}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(rows.len(), 2, "{product}: exact tool continuation");
        for row in &rows {
            assert_eq!(row["status"], 200);
            assert_eq!(row["model"], connection.model);
            assert_wire_output_limit(row, connection.api.as_str(), limit);
        }
        assert_eq!(
            sha256_hex(&fs::read(&executable).unwrap()),
            executable_sha256
        );
        assert_eq!(
            package
                .as_deref()
                .map(|package| probe_package_identity(&executable, package).unwrap())
                .as_ref(),
            package_identity.as_ref()
        );
        let evidence = serde_json::json!({
            "schema": "opi-eval-output-control/1", "product": product,
            "executable_sha256": executable_sha256,
            "runtime_package_manifest_sha256": package_digest,
            "runtime_package_entrypoint": package_entrypoint,
            "connection": {"provider": connection.provider, "api": connection.api, "model": connection.model},
            "model_controls": experiment.model_controls(),
            "probe_base_url": connection.base_url, "exit_code": 0,
            "config": {"path": config, "sha256": sha256_hex(&projection.config_bytes)},
            "requests": {"path": log, "sha256": sha256_hex(&requests)},
        });
        fs::write(
            case.join("output-control.json"),
            serde_json::to_vec_pretty(&evidence).unwrap(),
        )
        .unwrap();
    }
}

fn start_scripted_provider(log: &Path) -> (std::process::Child, Option<String>) {
    let port = {
        std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    };
    let mut child = Command::new("python3")
        .arg(manifest_dir().join("scripts/scripted-provider.py"))
        .arg("--listen")
        .arg(format!("127.0.0.1:{port}"))
        .arg("--request-log")
        .arg(log)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", log.parent().expect("listener log has a parent"))
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("start scripted provider");
    let mut line = String::new();
    {
        use std::io::BufRead as _;
        let stdout = child.stdout.take().unwrap();
        let mut reader = std::io::BufReader::new(stdout);
        reader
            .read_line(&mut line)
            .expect("provider readiness line");
    }
    assert!(line.starts_with("listening "), "{line}");
    let endpoint = line.trim().split(' ').nth(1).unwrap().to_owned();
    (
        child_with_stolen_stdout(child),
        Some(format!("http://{endpoint}")),
    )
}

fn child_with_stolen_stdout(mut child: std::process::Child) -> std::process::Child {
    child.stdout.take();
    child
}

fn run_opi_client(
    bin: &Path,
    config: &Path,
    model: &str,
    case_root: &Path,
    work: &Path,
) -> std::process::Output {
    run_opi_client_for(bin, config, "scripted", model, case_root, work)
}

fn run_opi_client_for(
    bin: &Path,
    config: &Path,
    provider: &str,
    model: &str,
    case_root: &Path,
    work: &Path,
) -> std::process::Output {
    let bus = sha256_hex(case_root.as_os_str().as_encoded_bytes());
    Command::new(bin)
        .arg("--config")
        .arg(config)
        .arg("--model")
        .arg(format!("{provider}:{model}"))
        .arg("--json")
        .arg("--json-compact")
        .arg("--non-interactive")
        .arg("--allow-mutating")
        .arg("--no-trust")
        .arg("--trace")
        .arg(case_root.join("trace"))
        .arg("-C")
        .arg(work)
        .arg("Use the bash tool to run the command, then finish.")
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", work)
        .env("XDG_CONFIG_HOME", work.join(".config"))
        .env("XDG_DATA_HOME", work.join(".local/share"))
        .env("XDG_CACHE_HOME", work.join(".cache"))
        .env("APPDATA", work.join(".appdata"))
        .env("no_proxy", "127.0.0.1,localhost")
        .env(
            "DBUS_SESSION_BUS_ADDRESS",
            format!("unix:abstract=opi-output-session-{}", &bus[..32]),
        )
        .env(
            "DBUS_SYSTEM_BUS_ADDRESS",
            format!("unix:abstract=opi-output-system-{}", &bus[..32]),
        )
        .env("OPI_EVAL_SCRIPTED_KEY", "dummy-scripted-credential")
        .current_dir(work)
        .output()
        .expect("run the exact built Opi binary")
}

fn run_pi_client(
    node: &Path,
    bundle: &Path,
    config: &Path,
    model: &str,
    case_root: &Path,
    work: &Path,
) -> std::process::Output {
    run_pi_client_for(
        node,
        Some(bundle),
        config,
        "scripted",
        model,
        case_root,
        work,
    )
}

fn run_pi_client_for(
    executable: &Path,
    bundle: Option<&Path>,
    config: &Path,
    provider: &str,
    model: &str,
    case_root: &Path,
    work: &Path,
) -> std::process::Output {
    let agent_dir = case_root.join("pi-agent");
    std::fs::create_dir_all(&agent_dir).unwrap();
    std::fs::copy(config, agent_dir.join("models.json")).unwrap();
    let mut command = Command::new(executable);
    if let Some(bundle) = bundle {
        command.arg(bundle);
    }
    command
        .arg("--print")
        .arg("--mode")
        .arg("json")
        .arg("--no-session")
        .arg("--no-approve")
        .arg("--no-extensions")
        .arg("--no-skills")
        .arg("--no-context-files")
        .args(["--thinking", "off"])
        .arg("--provider")
        .arg(provider)
        .arg("--model")
        .arg(model)
        .arg("--")
        .arg("Use the bash tool to run the command, then finish.")
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", work)
        .env("no_proxy", "127.0.0.1,localhost")
        .env("PI_CODING_AGENT_DIR", &agent_dir)
        .current_dir(work)
        .output()
        .expect("run the pinned pi bundle")
}

#[test]
fn native_capture_policy_controls_non_terminal_evidence() {
    let staged = stage("ok");
    let config = write_config(&staged);
    // Terminal-only: the bound artifacts and observation seal, the full
    // job logs do not.
    let terminal_only = staged.root.join("terminal-only.toml");
    std::fs::write(
        &terminal_only,
        fs::read_to_string(&config).unwrap().replace(
            "capture_policy = \"full\"",
            "capture_policy = \"terminal-only\"",
        ),
    )
    .unwrap();
    let full_root = staged.root.join("run-full");
    let terminal_root = staged.root.join("run-terminal");
    let (full_code, _, full_err) = run_native(&staged, &config, &full_root);
    let (terminal_code, _, terminal_err) = run_native(&staged, &terminal_only, &terminal_root);
    assert_eq!(full_code, 0, "{full_err}");
    assert_eq!(terminal_code, 0, "{terminal_err}");
    for trial in ["trial-pi-native", "trial-opi-native"] {
        let full_bundle = full_root.join("trials").join(trial).join("bundle");
        let terminal_bundle = terminal_root.join("trials").join(trial).join("bundle");
        assert!(
            full_bundle
                .join("artifacts/native/job-stdout.log")
                .is_file(),
            "full policy captures the job logs"
        );
        assert!(
            !terminal_bundle
                .join("artifacts/native/job-stdout.log")
                .is_file(),
            "terminal-only policy omits the job logs"
        );
        // Binding-required evidence stays on both policies.
        for role in ["native/trial-result.json", "native/trial-config.json"] {
            let file_name = role.rsplit('/').next().unwrap();
            assert!(
                full_bundle
                    .join("artifacts/native")
                    .join(file_name)
                    .is_file()
            );
            assert!(
                terminal_bundle
                    .join("artifacts/native")
                    .join(file_name)
                    .is_file()
            );
        }
    }
}

/// Seeds a complete local material cache the way an operator does from a
/// repository checkout: the pin seed (static lock, production profile,
/// integration modules, provider script, pinned materializer scripts)
/// plus the built agents, the materialized task package, the verifier
/// tool, and the oracle wrapper.
fn seed_local_cache(cache: &Path) {
    let fixture_cache = fixtures().join("external-locks/local-cache");
    copy_dir(&fixture_cache.join("scripts"), &cache.join("pins/scripts"));
    fs::create_dir_all(static_lock_in(cache).parent().unwrap()).unwrap();
    fs::copy(
        fixture_cache.join("static-lock.json"),
        static_lock_in(cache),
    )
    .unwrap();
    fs::create_dir_all(cache.join("pins/profiles/benchmarks")).unwrap();
    fs::copy(
        manifest_dir().join("profiles/benchmarks/terminal-bench-2.1.toml"),
        cache.join("pins/profiles/benchmarks/terminal-bench-2.1.toml"),
    )
    .unwrap();
    copy_dir(
        &manifest_dir().join("integrations/harbor"),
        &cache.join("pins/integrations/harbor"),
    );
    copy_dir(
        &manifest_dir().join("integrations/pier"),
        &cache.join("pins/integrations/pier"),
    );
    fs::copy(
        manifest_dir().join("scripts/scripted-provider.py"),
        cache.join("pins/scripts/scripted-provider.py"),
    )
    .unwrap();
    write_exec(
        &cache.join("agents/opi"),
        "#!/bin/sh\n# built opi stand-in (test seeding)\nexit 0\n",
    );
    write_exec(
        &cache.join("agents/pi"),
        "#!/bin/sh\n# built pi stand-in (test seeding)\nexit 0\n",
    );
    let mut subjects = Vec::new();
    for adapter in ["terminal-bench-2.1", "terminal-bench-3.0", "deepswe-v1.1"] {
        let project = cache.join("corpora").join(adapter);
        let task = project.join("tasks/openssl-selfsigned-cert");
        copy_dir(
            &fixtures()
                .join("benchmarks")
                .join(adapter)
                .join("task-package"),
            &task,
        );
        let mut doc: toml::Value = fs::read_to_string(task.join("task.toml"))
            .unwrap()
            .parse()
            .unwrap();
        if doc.get("environment").is_none() {
            doc.as_table_mut().unwrap().insert(
                "environment".to_owned(),
                toml::Value::Table(Default::default()),
            );
        }
        let environment = doc["environment"].as_table_mut().unwrap();
        environment.remove("cpus");
        environment.remove("memory_mb");
        environment.insert("storage_mb".to_owned(), 0.into());
        if environment.contains_key("docker_image") {
            environment.insert(
                "docker_image".to_owned(),
                format!("fixture@sha256:{}", "1".repeat(64)).into(),
            );
        }
        fs::write(task.join("task.toml"), toml::to_string(&doc).unwrap()).unwrap();
        if adapter == "deepswe-v1.1" {
            fs::write(
                task.join("environment/Dockerfile"),
                format!("FROM fixture@sha256:{}\n", "1".repeat(64)),
            )
            .unwrap();
        }
        for path in walk_files(&task) {
            if path.file_name().unwrap() == "Dockerfile" {
                fs::write(path, "FROM scratch\n").unwrap();
            }
        }
        let mut doc: toml::Value = fs::read_to_string(task.join("task.toml"))
            .unwrap()
            .parse()
            .unwrap();
        doc["environment"]
            .as_table_mut()
            .unwrap()
            .remove("docker_image");
        fs::write(task.join("task.toml"), toml::to_string(&doc).unwrap()).unwrap();
        subjects.push(fixture_corpus_pin(&project, adapter).1);
        fs::copy(
            manifest_dir()
                .join("profiles/benchmarks")
                .join(format!("{adapter}.toml")),
            cache
                .join("pins/profiles/benchmarks")
                .join(format!("{adapter}.toml")),
        )
        .unwrap();
    }
    write_exec(
        &cache.join("tools/uv"),
        "#!/bin/sh\n# resolved uv stand-in (test seeding)\n[ \"$1\" = --version ] && echo 'uv 0.9.7'\nexit 0\n",
    );
    let mut lock: serde_json::Value =
        serde_json::from_slice(&fs::read(static_lock_in(cache)).unwrap()).unwrap();
    let mut tools = vec![serde_json::json!({"id": "uv", "version": "0.9.7"})];
    for family in ["harbor", "pier"] {
        let project = cache.join("runners").join(family);
        fs::create_dir_all(project.join("src")).unwrap();
        fs::write(project.join(".gitignore"), ".venv/\n__pycache__/\n").unwrap();
        fs::write(
            project.join("pyproject.toml"),
            format!("[project]\nname = {family:?}\nversion = \"0.0.0\"\n"),
        )
        .unwrap();
        fs::write(project.join("uv.lock"), "version = 1\n").unwrap();
        fs::write(
            project.join("src/runner.py"),
            "# offline runner source fixture\n",
        )
        .unwrap();
        let git = |args: &[&str]| {
            let output = Command::new("git")
                .arg("-C")
                .arg(&project)
                .args(args)
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            String::from_utf8(output.stdout).unwrap().trim().to_owned()
        };
        git(&["init", "--quiet"]);
        git(&[
            "add",
            "--",
            ".gitignore",
            "pyproject.toml",
            "uv.lock",
            "src/runner.py",
        ]);
        git(&[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--quiet",
            "-m",
            "fixture",
        ]);
        tools.push(serde_json::json!({
            "id": family, "commit": git(&["rev-parse", "HEAD"]),
            "uv_lock": {"git_blob": git(&["rev-parse", "HEAD:uv.lock"]),
                "sha256": sha256_hex(&fs::read(project.join("uv.lock")).unwrap())}
        }));
        write_exec(&project.join(".venv/bin/python"), "#!/bin/sh\nexit 0\n");
        write_exec(
            &project.join(".venv/bin").join(family),
            "#!/bin/sh\nexit 0\n",
        );
    }
    lock["tools"] = tools.into();
    lock["subjects"] = subjects.into();
    fs::write(
        static_lock_in(cache),
        serde_json::to_vec_pretty(&lock).unwrap(),
    )
    .unwrap();
    write_exec(
        &cache.join("wrappers/terminal-bench-2.1-oracle.sh"),
        "#!/bin/sh\n# oracle wrapper stand-in (test seeding)\nexit 0\n",
    );
}

fn repin_cache_corpus(cache: &Path, adapter: &str) {
    let (_, pin) = fixture_corpus_pin(&cache.join("corpora").join(adapter), adapter);
    let path = static_lock_in(cache);
    let mut lock: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let entry = lock["subjects"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|entry| entry["id"] == adapter)
        .unwrap();
    *entry = pin;
    fs::write(path, serde_json::to_vec(&lock).unwrap()).unwrap();
}

/// The pinned static lock location inside a seeded cache.
fn static_lock_in(cache: &Path) -> PathBuf {
    cache.join("pins/static/linux-x86_64.json")
}

/// Copies the paired experiment fixture with `[storage].cache`
/// redirected to `cache`.
fn experiment_with_cache(dir: &Path, cache: &Path) -> PathBuf {
    let source = fs::read_to_string(fixtures().join("experiment/local-paired.toml")).unwrap();
    let redirected = source
        .lines()
        .map(|line| {
            if line.strip_prefix("cache = ").is_some() {
                format!("cache = {:?}", cache.to_string_lossy())
            } else if line.strip_prefix("task = ").is_some() {
                // The trials select the production pinned task the
                // materialized task package resolves.
                "task = \"openssl-selfsigned-cert\"".to_owned()
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let config = dir.join("experiment.toml");
    fs::write(&config, redirected).unwrap();
    config
}

/// P19-AC-001-prepare: `prepare --config C --out M` from an independent
/// working directory verifies the seeded cache and emits a local-origin
/// native-material manifest that `validate` consumes without repository
/// fixtures or GitHub run/artifact metadata.
#[test]
fn independent_directory_prepare_builds_a_local_origin_material() {
    let staged = tempfile::tempdir().unwrap();
    let cache = staged.path().join("cache");
    seed_local_cache(&cache);
    fs::remove_file(cache.join("wrappers/terminal-bench-2.1-oracle.sh")).unwrap();
    let config = experiment_with_cache(staged.path(), &cache);

    // The independent directory: neither the cache, the config, nor the
    // output tree lives inside the repository checkout.
    let independent = tempfile::tempdir().unwrap();
    let out = independent.path().join("material");
    let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("prepare")
        .arg("--config")
        .arg(&config)
        .arg("--out")
        .arg(&out)
        .current_dir(independent.path())
        .output()
        .expect("spawn the opi-eval prepare command");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(0),
        "prepare must verify and freeze the seeded cache: stdout={stdout} stderr={stderr}"
    );
    let report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(report["schema"], "opi-eval-prepare/1");
    assert_eq!(report["origin"], "local");
    assert_eq!(report["selection"]["benchmark"], "terminal-bench-2.1");
    assert_eq!(
        report["selection"]["products"],
        serde_json::json!(["opi", "pi"])
    );
    assert_eq!(
        report["images"][0]["manifest"],
        format!("sha256:{}", "2".repeat(64))
    );

    let material_path = out.join("material.json");
    assert!(material_path.is_file(), "{stdout}");
    let manifest = fs::read_to_string(&material_path).unwrap();
    let material: serde_json::Value = serde_json::from_str(&manifest).unwrap();
    assert_eq!(material["schema"], "opi-eval-native-material/5");
    assert_eq!(material["origin"], "local");
    let benchmark = &material["benchmarks"]["terminal-bench-2.1"];
    assert_eq!(benchmark["oracle"], benchmark["verifier_executable"]);
    assert_eq!(benchmark["oracle_env"], benchmark["verifier_env"]);
    // Only the selected benchmark is frozen into the manifest.
    assert_eq!(
        material["benchmarks"]
            .as_object()
            .unwrap()
            .keys()
            .collect::<Vec<_>>(),
        ["terminal-bench-2.1"]
    );
    // The provider endpoint is the experiment's selected connection.
    assert_eq!(
        material["provider"]["endpoint"],
        "http://127.0.0.1:48127/v1"
    );
    // No GitHub transport metadata is manufactured for local origin.
    assert!(
        !manifest.contains("receipt") && !manifest.contains("github"),
        "local origin must not carry transport metadata: {manifest}"
    );
    // Every materialized path stays inside the cache or output tree:
    // the runtime never needs the repository checkout.
    let repo = manifest_dir().to_string_lossy().into_owned();
    assert!(
        !manifest.contains(&repo),
        "the manifest must not point back into the repository: {manifest}"
    );
    // The frozen inventory records every verified identity for reuse.
    let inventory: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(cache.join("inventory.json")).unwrap()).unwrap();
    assert_eq!(inventory["schema"], "opi-eval-local-inventory/1");
    assert!(inventory["entries"]["agents:opi"]["sha256"].is_string());
    assert!(inventory["entries"]["corpora:terminal-bench-2.1"]["sha256"].is_string());

    // validate consumes the emitted manifest from the independent
    // directory (the production consumption call site).
    let validated = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("validate")
        .arg("--config")
        .arg(&config)
        .arg("--native-material")
        .arg(&material_path)
        .current_dir(independent.path())
        .output()
        .expect("spawn the opi-eval validate command");
    let vstdout = String::from_utf8_lossy(&validated.stdout);
    let vstderr = String::from_utf8_lossy(&validated.stderr);
    assert_eq!(
        validated.status.code(),
        Some(0),
        "validate must consume the local-origin manifest: stdout={vstdout} stderr={vstderr}"
    );
}

#[test]
fn prepare_binds_runtime_package_and_rejects_resource_drift() {
    let staged = tempfile::tempdir().unwrap();
    let cache = staged.path().join("cache");
    seed_local_cache(&cache);
    let executable = cache.join("agents/pi");
    let bytes = fs::read(&executable).unwrap();
    fs::remove_file(&executable).unwrap();
    write_exec(&executable.join("pi"), std::str::from_utf8(&bytes).unwrap());
    fs::write(executable.join("package.json"), br#"{"version":"0.85.1"}"#).unwrap();
    let config = experiment_with_cache(staged.path(), &cache);
    let out = staged.path().join("prepared");
    let prepared = run_prepare(&config, &out);
    assert!(
        prepared.status.success(),
        "{}",
        String::from_utf8_lossy(&prepared.stderr)
    );
    let path = out.join("material.json");
    let material: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let package = &material["agents"]["pi"]["runtime_package"];
    assert_eq!(package["path"], executable.to_string_lossy().as_ref());
    assert_eq!(package["entrypoint"], "pi");
    assert_eq!(
        package["manifest_sha256"],
        directory_manifest_digest(&executable)
    );
    assert_eq!(
        material["agents"]["pi"]["executable"]["path"],
        executable.join("pi").to_string_lossy().as_ref()
    );
    let validate = || {
        Command::new(env!("CARGO_BIN_EXE_opi-eval"))
            .args(["validate", "--config"])
            .arg(&config)
            .arg("--native-material")
            .arg(&path)
            .output()
            .unwrap()
    };
    let valid = validate();
    assert!(
        valid.status.success(),
        "{}",
        String::from_utf8_lossy(&valid.stderr)
    );
    for entrypoint in ["../pi", "/bin/sh", "package.json"] {
        let mut invalid = material.clone();
        invalid["agents"]["pi"]["runtime_package"]["entrypoint"] = entrypoint.into();
        fs::write(&path, serde_json::to_vec(&invalid).unwrap()).unwrap();
        let rejected = validate();
        assert!(
            !rejected.status.success(),
            "accepted entrypoint {entrypoint}"
        );
        assert!(String::from_utf8_lossy(&rejected.stderr).contains("runtime package"));
    }
    fs::write(&path, serde_json::to_vec(&material).unwrap()).unwrap();
    #[cfg(unix)]
    {
        let link = executable.join("linked-resource");
        std::os::unix::fs::symlink(executable.join("package.json"), &link).unwrap();
        let rejected = validate();
        assert!(!rejected.status.success());
        assert!(
            String::from_utf8_lossy(&rejected.stderr).contains("runtime package contains a link")
        );
        fs::remove_file(link).unwrap();
    }
    fs::write(executable.join("package.json"), br#"{"version":"0.0.0"}"#).unwrap();
    let drift = validate();
    assert!(!drift.status.success());
    assert!(String::from_utf8_lossy(&drift.stderr).contains("runtime package"));
    let refreeze = run_prepare(&config, &out);
    assert!(!refreeze.status.success());
    assert!(String::from_utf8_lossy(&refreeze.stderr).contains("material drift at runtime:pi"));
}

fn run_prepare(config: &Path, out: &Path) -> std::process::Output {
    run_prepare_env(config, out, &[])
}

#[test]
fn prepare_shares_cache_between_benchmark_profiles() {
    let staged = tempfile::tempdir().unwrap();
    let cache = staged.path().join("cache");
    seed_local_cache(&cache);
    let first = experiment_with_cache(staged.path(), &cache);
    let prepared = run_prepare(&first, &staged.path().join("tb21"));
    assert!(
        prepared.status.success(),
        "{}",
        String::from_utf8_lossy(&prepared.stderr)
    );
    fs::copy(
        manifest_dir().join("profiles/benchmarks/terminal-bench-3.0.toml"),
        cache.join("pins/profiles/benchmarks/terminal-bench-3.0.toml"),
    )
    .unwrap();

    let second = staged.path().join("tb30.toml");
    fs::write(
        &second,
        fs::read_to_string(&first)
            .unwrap()
            .replace("revision = \"2.1\"", "revision = \"3.0\""),
    )
    .unwrap();
    let prepared = run_prepare(&second, &staged.path().join("tb30"));
    assert!(
        prepared.status.success(),
        "{}",
        String::from_utf8_lossy(&prepared.stderr)
    );
    fs::write(
        cache.join("pins/profiles/benchmarks/terminal-bench-2.1.toml"),
        "drift",
    )
    .unwrap();
    let rejected = run_prepare(&first, &staged.path().join("tb21-again"));
    assert!(!rejected.status.success());
    assert!(
        String::from_utf8_lossy(&rejected.stderr)
            .contains("material drift at profiles:terminal-bench-2.1")
    );
}

#[test]
fn prepare_binds_runner_project_and_rejects_source_drift() {
    for (adapter, family) in [("terminal-bench-2.1", "harbor"), ("deepswe-v1.1", "pier")] {
        let staged = tempfile::tempdir().unwrap();
        let cache = staged.path().join("cache");
        seed_local_cache(&cache);
        let config = experiment_with_cache(staged.path(), &cache);
        if family == "pier" {
            let source = fs::read_to_string(&config)
                .unwrap()
                .replace("terminal-bench", "deepswe")
                .replace("revision = \"2.1\"", "revision = \"v1.1\"");
            fs::write(&config, source).unwrap();
            fs::copy(
                manifest_dir().join("profiles/benchmarks/deepswe-v1.1.toml"),
                cache.join("pins/profiles/benchmarks/deepswe-v1.1.toml"),
            )
            .unwrap();

            fs::copy(
                cache.join("wrappers/terminal-bench-2.1-oracle.sh"),
                cache.join("wrappers/deepswe-v1.1-oracle.sh"),
            )
            .unwrap();
        }
        let independent = tempfile::tempdir().unwrap();
        let out = independent.path().join("material");
        let prepare = || {
            Command::new(env!("CARGO_BIN_EXE_opi-eval"))
                .args(["prepare", "--config"])
                .arg(&config)
                .arg("--out")
                .arg(&out)
                .env("UNRELATED_SECRET", "must-not-be-inherited")
                .current_dir(independent.path())
                .output()
                .unwrap()
        };
        let result = prepare();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let bytes = fs::read(out.join("material.json")).unwrap();
        let material: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let project = cache.join("runners").join(family);
        let env = &material["benchmarks"][adapter]["verifier_env"];
        assert_eq!(env["UV_PROJECT"], project.to_string_lossy().as_ref());
        assert_eq!(
            env["UV_PROJECT_ENVIRONMENT"],
            project.join(".venv").to_string_lossy().as_ref()
        );
        assert_eq!(
            env["UV_PYTHON"],
            project.join(".venv/bin/python").to_string_lossy().as_ref()
        );
        assert_eq!(env["UV_OFFLINE"], "1");
        assert_eq!(env["UV_NO_DEV"], "1");
        assert_eq!(env["UV_NO_ENV_FILE"], "1");
        assert_eq!(env["PYTHON_DOTENV_DISABLED"], "1");
        assert!(env.get("UNRELATED_SECRET").is_none());
        assert!(env.get("UV_WORKING_DIRECTORY").is_none());
        let inventory = fs::read(cache.join("inventory.json")).unwrap();
        let validate = || {
            Command::new(env!("CARGO_BIN_EXE_opi-eval"))
                .args(["validate", "--config"])
                .arg(&config)
                .arg("--native-material")
                .arg(out.join("material.json"))
                .current_dir(independent.path())
                .output()
                .unwrap()
        };
        if family == "harbor" {
            let valid = validate();
            assert!(
                valid.status.success(),
                "{}",
                String::from_utf8_lossy(&valid.stderr)
            );
            let mut redirected = material.clone();
            redirected["benchmarks"][adapter]["verifier_env"]["UV_PYTHON"] =
                serde_json::json!("/unselected/python");
            fs::write(
                out.join("material.json"),
                serde_json::to_vec(&redirected).unwrap(),
            )
            .unwrap();
            let rejected = validate();
            assert!(!rejected.status.success());
            assert!(String::from_utf8_lossy(&rejected.stderr).contains("runner project"));
            fs::write(out.join("material.json"), &bytes).unwrap();
        }
        for relative in ["src/runner.py", "uv.lock"] {
            let file = project.join(relative);
            let before = fs::read(&file).unwrap();
            fs::write(&file, "# source drift\n").unwrap();
            if family == "harbor" {
                let rejected = validate();
                assert!(
                    !rejected.status.success(),
                    "validate accepted runner source drift"
                );
                assert!(String::from_utf8_lossy(&rejected.stderr).contains("runner"));
            }
            let rejected = prepare();
            assert!(
                !rejected.status.success(),
                "accepted {family} {relative} drift"
            );
            assert!(String::from_utf8_lossy(&rejected.stderr).contains("runner"));
            assert_eq!(fs::read(out.join("material.json")).unwrap(), bytes);
            assert_eq!(fs::read(cache.join("inventory.json")).unwrap(), inventory);
            fs::write(file, before).unwrap();
        }
        fs::remove_file(project.join(".venv/bin").join(family)).unwrap();
        assert!(
            !prepare().status.success(),
            "accepted missing runner entrypoint"
        );
    }
}

#[test]
fn prepare_rejects_unpinned_runner_and_uv_identities() {
    let staged = tempfile::tempdir().unwrap();
    let cache = staged.path().join("cache");
    seed_local_cache(&cache);
    let config = experiment_with_cache(staged.path(), &cache);
    let out = staged.path().join("material");
    let original: serde_json::Value =
        serde_json::from_slice(&fs::read(static_lock_in(&cache)).unwrap()).unwrap();
    for (pointer, replacement, reason) in [
        ("/tools/1/commit", "0".repeat(40), "source commit"),
        ("/tools/1/uv_lock/git_blob", "0".repeat(40), "uv.lock"),
        ("/tools/1/uv_lock/sha256", "0".repeat(64), "uv.lock"),
        ("/tools/0/version", "0.9.8".to_owned(), "uv 0.9.7"),
    ] {
        let mut lock = original.clone();
        *lock.pointer_mut(pointer).unwrap() = replacement.into();
        fs::write(static_lock_in(&cache), serde_json::to_vec(&lock).unwrap()).unwrap();
        let rejected = run_prepare(&config, &out);
        assert!(!rejected.status.success(), "accepted {pointer}");
        assert!(
            String::from_utf8_lossy(&rejected.stderr).contains(reason),
            "{}",
            String::from_utf8_lossy(&rejected.stderr)
        );
        assert!(!cache.join("inventory.json").exists());
        assert!(!out.join("material.json").exists());
    }
    fs::write(
        static_lock_in(&cache),
        serde_json::to_vec(&original).unwrap(),
    )
    .unwrap();
    let untracked = cache.join("runners/harbor/src/untracked.py");
    fs::write(&untracked, "# not part of the pinned source\n").unwrap();
    let rejected = run_prepare(&config, &out);
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("untracked source"));
    fs::remove_file(untracked).unwrap();
    write_exec(&cache.join("tools/uv"), "#!/bin/sh\necho 'uv 0.9.8'\n");
    let rejected = run_prepare(&config, &out);
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("pinned uv 0.9.7"));
}

fn run_prepare_env(config: &Path, out: &Path, env: &[(&str, &str)]) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_opi-eval"));
    command
        .arg("prepare")
        .arg("--config")
        .arg(config)
        .arg("--out")
        .arg(out);
    for (key, value) in env {
        command.env(key, value);
    }
    command
        .output()
        .expect("spawn the opi-eval prepare command")
}

/// P19-AC-007-material: local verification freezes identities, reuses
/// the verified cache, rejects drift and missing material without
/// silently fetching or replacing anything, and leaves the shared cache
/// byte-identical on rejection.
#[test]
fn local_material_integrity() {
    let staged = tempfile::tempdir().unwrap();
    let cache = staged.path().join("cache");
    seed_local_cache(&cache);
    let config = experiment_with_cache(staged.path(), &cache);
    let out = staged.path().join("material");

    // First run freezes the inventory.
    let first = run_prepare(&config, &out);
    assert_eq!(
        first.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&first.stderr)
    );
    let manifest_before = fs::read_to_string(out.join("material.json")).unwrap();
    let inventory_before = fs::read_to_string(cache.join("inventory.json")).unwrap();

    // Second run over the untouched cache reuses the frozen identities:
    // identical manifest and inventory bytes, no re-acquisition.
    let second = run_prepare(&config, &out);
    assert_eq!(second.status.code(), Some(0));
    assert_eq!(
        manifest_before,
        fs::read_to_string(out.join("material.json")).unwrap()
    );
    assert_eq!(
        inventory_before,
        fs::read_to_string(cache.join("inventory.json")).unwrap()
    );

    // Drift: tampering with a frozen identity rejects before any emit
    // and never triggers a silent re-fetch.
    write_exec(&cache.join("agents/opi"), "#!/bin/sh\n# tampered\nexit 1\n");
    let drifted = run_prepare(&config, &out);
    let stderr = String::from_utf8_lossy(&drifted.stderr);
    assert_eq!(drifted.status.code(), Some(2), "stderr: {stderr}");
    assert!(
        stderr.contains("material drift at agents:opi"),
        "drift must name the frozen identity: {stderr}"
    );
    // The previously emitted manifest survives the rejection untouched.
    assert_eq!(
        manifest_before,
        fs::read_to_string(out.join("material.json")).unwrap()
    );

    // Missing material without a declared acquirer is an actionable
    // rejection, never a silent fetch.
    let fresh = tempfile::tempdir().unwrap();
    let fresh_cache = fresh.path().join("cache");
    seed_local_cache(&fresh_cache);
    fs::remove_file(fresh_cache.join("tools/uv")).unwrap();
    fs::remove_file(fresh_cache.join("pins/scripts/materialize-external-locks.sh")).unwrap();
    fs::remove_file(fresh_cache.join("pins/scripts/verify-external-lock-ci.py")).unwrap();
    let fresh_config = experiment_with_cache(fresh.path(), &fresh_cache);
    let missing = run_prepare(&fresh_config, &fresh.path().join("material"));
    let stderr = String::from_utf8_lossy(&missing.stderr);
    assert_eq!(missing.status.code(), Some(2), "stderr: {stderr}");
    assert!(
        stderr.contains("material tools:uv missing"),
        "the missing identity must be named: {stderr}"
    );
}

/// Supervised acquisition: on a cache miss with the pinned materializer
/// seeded, prepare invokes it under supervision (plan in, log captured),
/// re-verifies the acquired bytes, and freezes them; a failing
/// materializer is a fail-closed rejection.
#[test]
fn supervised_acquisition_runs_the_pinned_materializer() {
    let staged = tempfile::tempdir().unwrap();
    let cache = staged.path().join("cache");
    seed_local_cache(&cache);
    // The pi executable has not been built into this cache yet.
    fs::remove_file(cache.join("agents/pi")).unwrap();
    let config = experiment_with_cache(staged.path(), &cache);
    let out = staged.path().join("material");

    let output = run_prepare(&config, &out);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(0),
        "supervised acquisition must complete the freeze: stdout={stdout} stderr={stderr}"
    );
    let report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(report["acquired"], serde_json::json!(["agents:pi"]));
    // The invocation is supervised and its evidence is captured.
    let log = fs::read_to_string(out.join("acquisition.log")).unwrap();
    assert!(log.contains("materialize"), "log: {log}");
    // The acquired bytes are the frozen identity the manifest pins.
    let manifest: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(out.join("material.json")).unwrap()).unwrap();
    let acquired = manifest["agents"]["pi"]["executable"]["path"]
        .as_str()
        .unwrap();
    let sha = sha256_hex(&fs::read(acquired).unwrap());
    assert_eq!(manifest["agents"]["pi"]["executable"]["sha256"], sha);
}

/// Derives a pi-only experiment document from the paired fixture by
/// dropping the opi subject, the edge, and its trial.
fn pi_only_config(dir: &Path, cache: &Path) -> PathBuf {
    let source = fs::read_to_string(fixtures().join("experiment/local-paired.toml")).unwrap();
    let mut blocks: Vec<String> = Vec::new();
    let mut current = String::new();
    for line in source.lines().chain(std::iter::once("")) {
        if line.is_empty() {
            if !current.trim().is_empty() {
                blocks.push(std::mem::take(&mut current));
            }
            continue;
        }
        current.push_str(line);
        current.push('\n');
    }
    // Keep the paired structure (edges are mandatory) but make both
    // subjects pi products: a single-product experiment.
    let filtered: Vec<String> = blocks
        .iter()
        .map(|block: &String| {
            let block = if block.contains("candidate-opi") {
                block.replace("product = \"opi\"", "product = \"pi\"")
            } else {
                block.clone()
            };
            block
                .lines()
                .map(|line| {
                    if line.strip_prefix("cache = ").is_some() {
                        format!("cache = {:?}", cache.to_string_lossy())
                    } else if line.strip_prefix("task = ").is_some() {
                        "task = \"openssl-selfsigned-cert\"".to_owned()
                    } else {
                        line.to_owned()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .collect();
    let text = filtered.join("\n");
    let config = dir.join("pi-only.toml");
    fs::write(&config, text).unwrap();
    config
}

/// P19-D05: a pi-only experiment prepares and validates without any Opi
/// material - the selected subject set drives the frozen manifest.
#[test]
fn pi_only_preparation_requires_no_opi_material() {
    let staged = tempfile::tempdir().unwrap();
    let cache = staged.path().join("cache");
    seed_local_cache(&cache);
    fs::remove_file(cache.join("agents/opi")).unwrap();
    let config = pi_only_config(staged.path(), &cache);
    let out = staged.path().join("material");

    let output = run_prepare(&config, &out);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(0),
        "pi-only preparation must not require Opi: stdout={stdout} stderr={stderr}"
    );
    let material: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(out.join("material.json")).unwrap()).unwrap();
    assert_eq!(
        material["agents"]
            .as_object()
            .unwrap()
            .keys()
            .collect::<Vec<_>>(),
        ["pi"]
    );
    let validated = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("validate")
        .arg("--config")
        .arg(&config)
        .arg("--native-material")
        .arg(out.join("material.json"))
        .output()
        .expect("spawn the opi-eval validate command");
    assert_eq!(
        validated.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&validated.stderr)
    );
}

/// P19-AC-007-material: a task resource requirement the local
/// environment cannot satisfy is an explicit rejection naming the task
/// and the requirement - the requested set never silently shrinks -
/// while a satisfied requirement freezes into the report.
#[test]
fn unmet_resource_requirement_rejects_without_silent_shrink() {
    let staged = tempfile::tempdir().unwrap();
    let cache = staged.path().join("cache");
    seed_local_cache(&cache);
    let config = experiment_with_cache(staged.path(), &cache);
    let out = staged.path().join("material");

    // The selected task declares a GPU requirement the local
    // environment cannot observe (no reachable engine): prepare must
    // reject it explicitly instead of dropping it from the set.
    let task_toml =
        cache.join("corpora/terminal-bench-2.1/tasks/openssl-selfsigned-cert/task.toml");
    fs::write(
        &task_toml,
        "artifacts = [\"/root/answer.txt\"]\n\n[task]\nid = \"openssl-selfsigned-cert\"\n\n[environment]\ncpus = 1\nmemory_mb = 256\ngpus = 1\nos = \"linux\"\n",
    )
    .unwrap();
    repin_cache_corpus(&cache, "terminal-bench-2.1");
    // A closed loopback endpoint forces the unknown-GPU observation on
    // every host, with or without a real engine: the requirement must
    // reject explicitly instead of shrinking the set.
    let rejected = run_prepare_env(&config, &out, &[("DOCKER_HOST", "tcp://127.0.0.1:1")]);
    let stderr = String::from_utf8_lossy(&rejected.stderr);
    assert_eq!(rejected.status.code(), Some(2), "stderr: {stderr}");
    assert!(
        stderr.contains("openssl-selfsigned-cert")
            && stderr.contains("gpus")
            && stderr.contains("engine"),
        "the rejection must name the task, the requirement, and the unknown observation: {stderr}"
    );
    assert!(
        !out.join("material.json").is_file(),
        "no manifest may be emitted for a rejected set"
    );

    // A satisfied requirement freezes with its selection reason. The
    // stage starts from a fresh cache: the rewritten task.toml above is
    // a different frozen identity, not drift of this one.
    let ok_staged = tempfile::tempdir().unwrap();
    let ok_cache = ok_staged.path().join("cache");
    seed_local_cache(&ok_cache);
    let ok_config = experiment_with_cache(ok_staged.path(), &ok_cache);
    let ok_out = ok_staged.path().join("material");
    fs::write(
        ok_cache.join("corpora/terminal-bench-2.1/tasks/openssl-selfsigned-cert/task.toml"),
        "artifacts = [\"/root/answer.txt\"]\n\n[task]\nid = \"openssl-selfsigned-cert\"\n\n[environment]\ncpus = 1\nmemory_mb = 256\ngpus = 0\nos = \"linux\"\n",
    )
    .unwrap();
    repin_cache_corpus(&ok_cache, "terminal-bench-2.1");
    let output = run_prepare(&ok_config, &ok_out);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(0), "stderr: {stderr}");
    let report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    let preflight = &report["preflight"]["tasks"][0];
    assert_eq!(preflight["id"], "openssl-selfsigned-cert");
    assert_eq!(preflight["status"], "ok");
    assert!(preflight["requirements"]["agent"]["gpus"].is_u64());
}

// --- Task 19.6: plan freeze, full-inventory preflight, selection (P19-D05) ---

/// Builds a three-task corpus package root (one profile-bound task, one
/// feasible extra task, one GPU-bound task) and repoints the staged
/// material's terminal-bench-2.1 benchmark at it.
fn stage_corpus(staged: &Staged) -> std::path::PathBuf {
    let project = staged.root.join("corpus-project");
    let corpus = project.join("tasks");
    let bound = corpus.join("fixture-task");
    copy_dir(&staged.root.join("material/task-package"), &bound);
    for (id, gpus) in [("alpha-notes", 0), ("gpu-train", 1)] {
        let dir = corpus.join(id);
        copy_dir(&bound, &dir);
        fs::write(
            dir.join("task.toml"),
            format!(
                "schema_version = \"1.0\"\n[task]\nid = {id:?}\n[environment]\ngpus = {gpus}\n"
            ),
        )
        .unwrap();
        let metadata: toml::Value = fs::read_to_string(dir.join("task.toml"))
            .unwrap()
            .parse()
            .unwrap();
        fs::write(dir.join("task.toml"), toml::to_string(&metadata).unwrap()).unwrap();
        fs::write(
            dir.join("instruction.md"),
            format!("Synthetic {id} instruction.\n"),
        )
        .unwrap();
    }
    let (corpus_identity, pin) = fixture_corpus_pin(&project, "terminal-bench-2.1");
    let mut value: serde_json::Value =
        serde_json::from_slice(&fs::read(&staged.material).unwrap()).unwrap();
    let mut tasks = serde_json::Map::new();
    for id in ["fixture-task", "alpha-notes"] {
        let package = staged.root.join("packages").join(id);
        copy_dir(&corpus.join(id), &package);
        let profile = staged.root.join(format!("{id}.toml"));
        fs::copy(staged.root.join("material/profile.toml"), &profile).unwrap();
        tasks.insert(
            id.to_owned(),
            fixture_task_binding(&project, id, &package, &profile),
        );
    }
    value["benchmarks"]["terminal-bench-2.1"]["corpus"] = corpus_identity;
    value["benchmarks"]["terminal-bench-2.1"]["tasks"] = tasks.into();
    let lock_path = PathBuf::from(value["static_lock"]["path"].as_str().unwrap());
    let mut lock: serde_json::Value =
        serde_json::from_slice(&fs::read(&lock_path).unwrap()).unwrap();
    *lock["subjects"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|entry| entry["id"] == "terminal-bench-2.1")
        .unwrap() = pin;
    let bytes = serde_json::to_vec(&lock).unwrap();
    fs::write(lock_path, &bytes).unwrap();
    value["static_lock"]["sha256"] = serde_json::json!(sha256_hex(&bytes));
    fs::write(&staged.material, serde_json::to_vec(&value).unwrap()).unwrap();
    corpus
}

/// A selection-mode experiment document over the staged native material.
fn selection_experiment_text(staged: &Staged, integrity_digest: &str, task_ids: &str) -> String {
    let base = experiment_text(staged, integrity_digest);
    let head = base[..base.find("[[trials]]").unwrap()].to_owned();
    format!(
        "{head}[selection]\nscope = \"subset\"\ntask_ids = {task_ids}\nreason = \"feasible subset for the deterministic plan\"\nrepetitions = 1\n"
    )
}

fn run_plan_with_closed_engine(staged: &Staged, config: &Path) -> (std::process::Output, String) {
    let out = staged.root.join("plan.json");
    let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("plan")
        .arg("--config")
        .arg(config)
        .arg("--native-material")
        .arg(&staged.material)
        .arg("--out")
        .arg(&out)
        .env("DOCKER_HOST", "tcp://127.0.0.1:1")
        .output()
        .expect("spawn the opi-eval plan command");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    (output, stdout)
}

/// P19-AC-007 (selection): the full pinned inventory enumerates from
/// metadata without models; unmet/unknown requirements stay visible in
/// the preview while a feasible explicit subset freezes.
#[test]
fn full_inventory_preflight_lists_requirements_and_freezes_feasible_subsets() {
    let staged = stage("tb21-ok");
    stage_corpus(&staged);
    let digest = derive_integrity(&staged);
    let config = staged.root.join("selection.toml");
    fs::write(
        &config,
        selection_experiment_text(&staged, &digest, "[\"fixture-task\"]"),
    )
    .unwrap();

    let (output, stdout) = run_plan_with_closed_engine(&staged, &config);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(0),
        "plan must freeze the feasible subset: stdout={stdout} stderr={stderr}"
    );
    let preview: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(preview["schema"], "opi-eval-plan-preview/2");
    let corpus = preview["corpus"].as_array().unwrap();
    assert_eq!(corpus.len(), 3, "full inventory must enumerate: {corpus:?}");
    let by_id = |id: &str| {
        corpus
            .iter()
            .find(|entry| entry["id"] == id)
            .unwrap_or_else(|| panic!("task {id} missing: {corpus:?}"))
            .clone()
    };
    assert_eq!(by_id("fixture-task")["status"], "ok");
    assert_eq!(by_id("fixture-task")["selected"], true);
    assert_eq!(by_id("alpha-notes")["status"], "ok");
    assert_eq!(by_id("alpha-notes")["selected"], false);
    let gpu = by_id("gpu-train");
    assert_eq!(gpu["status"], "unknown");
    assert_eq!(gpu["selected"], false);
    assert!(
        gpu["reason"].as_str().unwrap().contains("gpus unknown"),
        "reason: {gpu:?}"
    );
    assert_eq!(
        preview["preflight"]["unknown"],
        serde_json::json!(["gpu-train"])
    );
    assert_eq!(preview["trial_count"], 2);
    assert_eq!(preview["concurrency"], 1);
    assert_eq!(preview["selection"]["scope"], "subset");
    assert_eq!(
        preview["selection"]["task_ids"],
        serde_json::json!(["fixture-task"])
    );

    let plan: serde_json::Value = {
        let text = fs::read_to_string(staged.root.join("plan.json")).unwrap();
        serde_json::from_str(&text).unwrap()
    };
    assert_eq!(plan["schema"], "opi-eval-run-plan/2");
    assert!(plan["plan_digest"].as_str().unwrap().len() == 64);
    let trials = plan["trials"].as_array().unwrap();
    let ids: Vec<&str> = trials.iter().filter_map(|t| t["id"].as_str()).collect();
    assert_eq!(
        ids,
        [
            "trial-baseline-pi-fixture-task-r1",
            "trial-candidate-opi-fixture-task-r1"
        ]
    );
    assert_eq!(
        plan["experiment"]["manifest_digest"],
        preview["manifest_digest"]
    );
}

/// P19-AC-007 (selection): every contradiction rejects with its
/// reason - unknown ids, unmet or unknown selected requirements,
/// full-scope unmet requirements, native profile binding, and
/// max_trials. Nothing silently shrinks.
#[test]
fn plan_rejections_name_every_contradiction() {
    let staged = stage("tb21-ok");
    stage_corpus(&staged);
    let digest = derive_integrity(&staged);
    let cases: [(&str, &str, &str); 5] = [
        (
            "unknown task id",
            "[\"no-such-task\"]",
            "outside the pinned inventory",
        ),
        (
            "unobservable selected GPU requirement",
            "[\"gpu-train\"]",
            "gpus unknown",
        ),
        (
            "full scope cannot silently drop the unmet task",
            "[\"gpu-train\"]\n__scope_full",
            "gpu-train unknown",
        ),
        (
            "omitted selected material",
            "[\"gpu-train\"]",
            "has no selected byte-bound native material",
        ),
        (
            "declared max_trials below the expansion",
            "[\"fixture-task\"]\n__max_trials_1",
            "exceeds the declared max_trials",
        ),
    ];
    for (name, task_ids, expected) in cases {
        let mut text = selection_experiment_text(&staged, &digest, task_ids);
        if text.contains("__scope_full") {
            // Full scope pins no ids: the whole corpus is the selection.
            text = text
                .replace("scope = \"subset\"", "scope = \"full\"")
                .replace("\n__scope_full", "");
            let start = text.find("task_ids = ").expect("task_ids line");
            let end = text[start..]
                .find('\n')
                .map(|offset| start + offset + 1)
                .expect("task_ids line ends");
            text.replace_range(start..end, "");
        }
        text = text.replace("\n__max_trials_1", "");
        if name.contains("max_trials") {
            text = text.replace(
                "[execution]\nmode = \"native-fixture\"",
                "[execution]\nmode = \"native-fixture\"\nmax_trials = 1",
            );
        }
        let config = staged
            .root
            .join(format!("reject-{}.toml", name.replace([' ', '/'], "-")));
        fs::write(&config, text).unwrap();
        let (output, stdout) = run_plan_with_closed_engine(&staged, &config);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{name} must reject: stdout={stdout} stderr={stderr}"
        );
        assert!(stderr.contains(expected), "{name}: {stderr}");
        assert!(
            !staged.root.join("plan.json").exists(),
            "{name} must not freeze a partial plan"
        );
    }
}

/// `P19-AC-008` (native provider failures through the public
/// run-to-report path): a final provider/access failure keeps the trial
/// attempted and visible but out of the valid graded denominator - never
/// an Agent success or failure - while a valid native zero stays a graded
/// Agent outcome inside the denominator. Both facts flow from the sealed
/// settlement record, not from stderr tokens or process exits.
#[test]
fn native_provider_failure_report() {
    // Final provider/access failure: both pinned clients' trials settle
    // as terminal provider failures.
    let staged = stage("provider-error");
    let config = write_config(&staged);
    let run_root = staged.root.join("run");
    let (code, run_report, stderr) = run_native(&staged, &config, &run_root);
    assert_eq!(code, 1, "provider-failure run is incomplete: {stderr}");
    let run_report: serde_json::Value = serde_json::from_str(run_report.trim())
        .unwrap_or_else(|_error| panic!("run stdout is not one JSON report: {run_report:?}"));
    assert_eq!(run_report["outcome"], "incomplete", "{run_report}");
    let report = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("report")
        .arg("--root")
        .arg(&run_root)
        .output()
        .expect("spawn the offline report command");
    let stdout = String::from_utf8_lossy(&report.stdout).trim().to_owned();
    assert_eq!(
        report.status.code(),
        Some(0),
        "the offline report publishes incomplete coverage: {stdout} stderr={}",
        String::from_utf8_lossy(&report.stderr)
    );
    let published: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|_error| panic!("report stdout is not one JSON report: {stdout:?}"));
    let counts = &published["coverage_counts"];
    assert_eq!(counts["attempted"], 2, "{published}");
    assert_eq!(counts["valid"], 0, "{published}");
    assert_eq!(counts["invalid"], 2, "{published}");
    assert_eq!(counts["agent_failure"], 0, "{published}");
    assert_eq!(counts["unexecuted"], 0, "{published}");
    assert_eq!(published["metrics"][0]["denominator"], 0, "{published}");
    for trial in published["trials"].as_array().unwrap() {
        assert_eq!(trial["outcome"]["class"], "invalid", "{trial}");
        let reason = trial["outcome"]["reason"].as_str().unwrap_or_default();
        assert!(
            reason.starts_with("agent-provider-failure"),
            "the sealed reason names the provider failure: {trial}"
        );
    }
    assert_eq!(published["run_outcome"], "incomplete", "{published}");

    // Valid native zero: the graded zero stays a valid graded Agent
    // outcome inside the denominator, counted as an Agent failure.
    let zero_staged = stage("zero");
    let zero_config = write_config(&zero_staged);
    let zero_root = zero_staged.root.join("run");
    let (code, run_report, stderr) = run_native(&zero_staged, &zero_config, &zero_root);
    assert_eq!(code, 0, "zero-reward run completes: {stderr} {run_report}");
    let run_report: serde_json::Value = serde_json::from_str(run_report.trim())
        .unwrap_or_else(|_error| panic!("run stdout is not one JSON report: {run_report:?}"));
    assert_eq!(run_report["outcome"], "completed", "{run_report}");
    let report = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("report")
        .arg("--root")
        .arg(&zero_root)
        .output()
        .expect("spawn the offline report command");
    let stdout = String::from_utf8_lossy(&report.stdout).trim().to_owned();
    assert_eq!(report.status.code(), Some(0), "{stdout}");
    let published: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|_error| panic!("report stdout is not one JSON report: {stdout:?}"));
    let counts = &published["coverage_counts"];
    assert_eq!(counts["valid"], 2, "native zero stays graded: {published}");
    assert_eq!(counts["agent_failure"], 2, "{published}");
    assert_eq!(counts["invalid"], 0, "{published}");
    assert_eq!(published["metrics"][0]["denominator"], 2, "{published}");
    assert_eq!(published["run_outcome"], "completed", "{published}");
}

/// Runs one native family to completion and returns its root plus the
/// pre-regrade offline report bytes (the immutability baseline).
fn regrade_family_root(
    staged: &Staged,
    config: &Path,
    run_name: &str,
) -> (std::path::PathBuf, String) {
    let run_root = staged.root.join(run_name);
    let (code, stdout, stderr) = run_native(staged, config, &run_root);
    assert_eq!(code, 0, "{run_name} run: {stderr} {stdout}");
    let report = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("report")
        .arg("--root")
        .arg(&run_root)
        .output()
        .expect("spawn the offline report command");
    assert_eq!(report.status.code(), Some(0));
    (
        run_root,
        String::from_utf8_lossy(&report.stdout).trim().to_owned(),
    )
}

/// Regrades one run root offline and returns (exit, stdout report JSON).
fn regrade_run(staged: &Staged, run_root: &Path, out: &Path) -> (i32, serde_json::Value, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("regrade")
        .arg("--root")
        .arg(run_root)
        .arg("--native-material")
        .arg(&staged.material)
        .arg("--out")
        .arg(out)
        .output()
        .expect("spawn the opi-eval regrade command");
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let report = serde_json::from_str(&stdout).unwrap_or_else(|_error| {
        panic!("regrade stdout is one JSON report: {stdout:?} stderr={stderr}")
    });
    (output.status.code().unwrap_or(-1), report, stderr)
}

/// `P19-AC-009` (regrade): saved native Agent artifacts undergo fresh
/// offline native grading in fresh verifier environments - for all three
/// families - through the same binding readers, with a derived record per
/// trial, zero Agent or provider invocations, and byte-identical source
/// evidence afterwards. Byte checking alone never satisfies this path.
#[test]
fn native_offline_regrade() {
    let staged = stage("ok");
    let cases: Vec<(&str, PathBuf)> = vec![
        ("terminal-bench-2.1", write_config(&staged)),
        ("terminal-bench-3.0", write_tb30_config(&staged)),
        ("deepswe-v1.1", write_deepswe_config(&staged)),
    ];
    for (family, config) in &cases {
        let (run_root, before_report) =
            regrade_family_root(&staged, config, &format!("run-{family}"));
        let out = staged.root.join(format!("regrade-{family}"));

        let (code, report, stderr) = regrade_run(&staged, &run_root, &out);
        assert_eq!(code, 0, "{family} regrade: {stderr} {report}");
        assert_eq!(report["schema"], "opi-eval-native-regrade/1", "{report}");
        assert_eq!(report["outcome"], "regraded", "{family}: {report}");
        let trials = report["trials"].as_array().unwrap();
        assert_eq!(trials.len(), 2, "{family}: {report}");
        for trial in trials {
            assert_eq!(trial["reward"], 1, "{family}: {report}");
            assert!(trial["bundle_identity"].as_str().unwrap().len() == 64);
            let record_path = trial["record"].as_str().unwrap();
            assert!(out.join(record_path).is_file(), "{family}: {record_path}");
            let record: serde_json::Value =
                serde_json::from_str(&fs::read_to_string(out.join(record_path)).unwrap()).unwrap();
            assert_eq!(record["schema"], "opi-eval-regrade-record/1", "{record}");
            assert_eq!(record["source"]["trial"], trial["trial"], "{record}");
            for artifact in record["source"]["collected"].as_array().unwrap() {
                // Every preserved artifact digests to exactly the sealed
                // verifier-input identity the fresh grade consumed.
                assert!(artifact["sha256"].as_str().unwrap().len() == 64, "{record}");
            }
            assert!(
                record["fresh"]["verifier_executable_sha256"]
                    .as_str()
                    .unwrap()
                    .len()
                    == 64,
                "{record}"
            );
        }
        assert_eq!(
            report["rejections"].as_array().unwrap().len(),
            0,
            "{family}: {report}"
        );

        // Zero Agent or provider invocations: the stand-in agents write
        // their argv capture next to themselves; nothing of the agent
        // boundary may appear anywhere under the regrade output.
        let out_text = walk_text(&out);
        assert!(!out_text.contains("opi-argv.txt"), "{family}: agent argv");
        assert!(
            !out_text.contains("handoff.json"),
            "{family}: no integration handoff in the fresh environment"
        );
        // The fresh environment record carries no credential projection.
        for env_record in walk_files_matching(&out, "env.json") {
            let text = fs::read_to_string(env_record).unwrap();
            let projected: serde_json::Value = serde_json::from_str(&text).unwrap();
            let keys = projected["projected"].as_object().unwrap();
            assert!(
                !keys.contains_key("OPI_EVAL_SCRIPTED_KEY") && !keys.contains_key("OPENAI_API_KEY"),
                "{family}: credential material in the fresh environment: {text}"
            );
        }

        // Immutable source evidence: the offline report and the seals are
        // byte-identical after regrading.
        let after = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
            .arg("report")
            .arg("--root")
            .arg(&run_root)
            .output()
            .expect("spawn the offline report command");
        assert_eq!(
            String::from_utf8_lossy(&after.stdout).trim(),
            before_report,
            "{family}: regrade must not change the source report"
        );
        let verify = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
            .arg("verify")
            .arg("--root")
            .arg(&run_root)
            .output()
            .expect("spawn the verify command");
        assert_eq!(verify.status.code(), Some(0), "{family}: seals intact");
    }
}

fn remediation_runner(staged: &Staged, edit: impl FnOnce(String) -> String) {
    let path = staged.root.join("material/verifier-uv.sh");
    let body = edit(fs::read_to_string(&path).unwrap());
    write_exec(&path, &body);
    let mut material: serde_json::Value =
        serde_json::from_slice(&fs::read(&staged.material).unwrap()).unwrap();
    for value in material["benchmarks"].as_object_mut().unwrap().values_mut() {
        value["verifier_executable"]["sha256"] = serde_json::json!(sha256_hex(body.as_bytes()));
        value["oracle"]["sha256"] = serde_json::json!(sha256_hex(body.as_bytes()));
    }
    fs::write(&staged.material, serde_json::to_vec(&material).unwrap()).unwrap();
}

#[test]
fn remediation_004_native_provider_error_is_not_scored() {
    let mut wrongly_scored = Vec::new();
    for family in ["terminal-bench-2.1", "terminal-bench-3.0", "deepswe-v1.1"] {
        for capture in ["full", "terminal-only"] {
            for case in [
                "auth",
                "rate-limit",
                "protocol",
                "unknown",
                "nonzero-error",
                "success",
                "retry",
                "zero",
                "task-failure",
                "timeout",
            ] {
                let staged = stage("ok");
                let mutation = staged.root.join("client-terminal.py");
                fs::write(&mutation, r#"import json, pathlib, sys
case, fixtures = sys.argv[1], pathlib.Path(sys.argv[2])
failed = case in ('auth', 'rate-limit', 'protocol', 'unknown', 'nonzero-error')
reward = 0.0 if failed or case in ('zero', 'task-failure', 'timeout') else 1.0
for path in pathlib.Path('jobs').glob('*/*/agent/opi-eval-terminal.json'):
    terminal = json.loads(path.read_text())
    product = terminal['product']
    terminal['exit_code'] = 3 if case == 'auth' else 4 if failed else 1 if case == 'task-failure' else 0
    if product == 'pi' and failed:
        terminal['exit_code'] = 1 if case == 'nonzero-error' else 0
    terminal['timed_out'] = case == 'timeout'
    path.write_text(json.dumps(terminal))
    diagnostic = {'auth': 'provider authentication failed', 'rate-limit': 'rate limited by provider', 'protocol': 'provider stream failed', 'retry': 'rate limited by provider'}.get(case, '')
    path.with_name('stderr.log').write_text(diagnostic)
    fixture = {'auth': 'stream-auth-error.jsonl', 'nonzero-error': 'stream-auth-error.jsonl', 'rate-limit': 'stream-rate-limit-error.jsonl', 'protocol': 'stream-protocol-error.jsonl', 'unknown': 'stream-error.jsonl'}.get(case, 'stream-ok.jsonl')
    stream = (fixtures / fixture).read_text().replace('"provider":"local"', '"provider":"scripted"').replace('"model":"scripted"', '"model":"eval"')
    if case == 'retry':
        events = stream.splitlines()
        failed_event = next(line for line in (fixtures / 'stream-rate-limit-error.jsonl').read_text().splitlines() if json.loads(line)['type'] == 'message_end')
        events.insert(3, failed_event)
        events.insert(4, '{"type":"auto_retry_start"}')
        events.insert(5, '{"type":"auto_retry_end"}')
        stream = '\n'.join(events) + '\n'
    path.with_name('stdout.log').write_text(stream if product == 'pi' else '')
    result_path = path.parent.parent / 'result.json'
    result = json.loads(result_path.read_text())
    result['verifier_result']['rewards']['reward'] = reward
    exception = 'AgentTimeoutError' if case == 'timeout' else 'NonZeroAgentExitCodeError' if terminal['exit_code'] else None
    result['exception_info'] = {'exception_type': exception} if exception else None
    result_path.write_text(json.dumps(result))
    aggregate_path = path.parent.parent.parent / 'result.json'
    aggregate = json.loads(aggregate_path.read_text())
    for evaluation in aggregate['stats']['evals'].values():
        evaluation['reward_stats']['reward'] = {str(reward): [result['trial_name']]}
    aggregate_path.write_text(json.dumps(aggregate))
"#).unwrap();
                remediation_runner(&staged, |body| {
                    body.replacen(
                        "set -eu",
                        &format!(
                            "set -eu\ntrap 'python3 \"{}\" {case} \"{}\"' EXIT",
                            mutation.display(),
                            fixtures().join("agents/pi").display()
                        ),
                        1,
                    )
                });
                let config = match family {
                    "terminal-bench-2.1" => write_config(&staged),
                    "terminal-bench-3.0" => write_tb30_config(&staged),
                    _ => write_deepswe_config(&staged),
                };
                fs::write(
                    &config,
                    fs::read_to_string(&config).unwrap().replace(
                        "capture_policy = \"full\"",
                        &format!("capture_policy = \"{capture}\""),
                    ),
                )
                .unwrap();
                let root = staged.root.join("run");
                let (code, stdout, stderr) = run_native(&staged, &config, &root);
                let failed = matches!(
                    case,
                    "auth" | "rate-limit" | "protocol" | "unknown" | "nonzero-error"
                );
                let report = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
                    .args(["report", "--root"])
                    .arg(&root)
                    .output()
                    .unwrap();
                assert!(
                    report.status.success(),
                    "{family}/{capture}/{case}: {stderr} {stdout} {}",
                    String::from_utf8_lossy(&report.stderr)
                );
                let published: serde_json::Value = serde_json::from_slice(&report.stdout).unwrap();
                let counts = &published["coverage_counts"];
                assert_eq!(counts["attempted"], 2, "{published}");
                assert_eq!(counts["unexecuted"], 0, "{published}");
                if failed
                    && (code != 1
                        || counts["valid"] != 0
                        || counts["invalid"] != 2
                        || counts["agent_failure"] != 0)
                {
                    wrongly_scored.push(format!("{family}/{capture}/{case}: {counts}"));
                    continue;
                }
                if !failed {
                    assert_eq!(code, 0, "{family}/{capture}/{case}: {stderr} {stdout}");
                    assert_eq!(counts["valid"], 2, "{published}");
                    assert_eq!(counts["invalid"], 0, "{published}");
                    assert_eq!(
                        counts["agent_failure"],
                        if matches!(case, "zero" | "task-failure" | "timeout") {
                            2
                        } else {
                            0
                        },
                        "{published}"
                    );
                }
                for trial in published["trials"].as_array().unwrap() {
                    if failed {
                        let reason = trial["outcome"]["reason"].as_str().unwrap();
                        let expected = match case {
                            "auth" | "nonzero-error" => "provider-auth-failure",
                            "rate-limit" => "provider-rate-limit-exhausted",
                            "protocol" => "provider-protocol-error",
                            _ => "provider-failure",
                        };
                        // Opi's generic exit 4 carries no authentication diagnostic in this case.
                        assert!(
                            reason.contains(expected)
                                || (case == "nonzero-error" && reason == "provider-failure"),
                            "{family}/{case}: {trial}"
                        );
                    }
                }
                let terminals = walk_files_matching(&root.join("trials"), "agent-terminal.json");
                assert_eq!(terminals.len(), 2);
                for terminal in terminals {
                    let evidence: serde_json::Value =
                        serde_json::from_slice(&fs::read(&terminal).unwrap()).unwrap();
                    if failed {
                        assert!(
                            evidence["failure_kind"]
                                .as_str()
                                .unwrap()
                                .starts_with("provider-"),
                            "{evidence}"
                        );
                        if evidence["product"] == "pi" {
                            assert_eq!(
                                evidence["client_message"]["stopReason"], "error",
                                "{evidence}"
                            );
                        }
                    } else if case == "retry" {
                        assert!(evidence["failure_kind"].is_null(), "{evidence}");
                        if evidence["product"] == "pi" {
                            assert_eq!(
                                evidence["client_message"]["stopReason"], "stop",
                                "{evidence}"
                            );
                        }
                    }
                    assert_eq!(
                        terminal.with_file_name("agent-stdout.log").exists(),
                        capture == "full"
                    );
                }
                for settlement in walk_files_matching(&root.join("trials"), "settlement.json")
                    .into_iter()
                    .filter(|path| {
                        path.parent()
                            .and_then(Path::file_name)
                            .is_some_and(|name| name == "control")
                    })
                {
                    let value: serde_json::Value =
                        serde_json::from_slice(&fs::read(settlement).unwrap()).unwrap();
                    assert_eq!(value["graded"], !failed, "{value}");
                }
            }
        }
    }
    assert!(
        wrongly_scored.is_empty(),
        "client provider failures were scored: {wrongly_scored:#?}"
    );
}

#[test]
fn remediation_003_substituted_terminal_identity_rejects() {
    let mut admitted = Vec::new();
    for family in ["terminal-bench-2.1", "terminal-bench-3.0", "deepswe-v1.1"] {
        for case in [
            "matching",
            "product",
            "executable",
            "model",
            "missing-product",
            "missing-executable",
            "missing-model",
        ] {
            let staged = stage("ok");
            if case != "matching" {
                let mutation = staged.root.join("mutate-result.py");
                fs::write(
                    &mutation,
                    r#"import json, pathlib, sys
case = sys.argv[1]
for path in pathlib.Path("jobs").glob("*/*/agent/opi-eval-terminal.json"):
    value = json.loads(path.read_text())
    field = "product" if "product" in case else "agent_executable_sha256"
    if case in ("product", "executable"):
        value[field] = "oracle" if case == "product" else "f" * 64
    elif case in ("missing-product", "missing-executable"):
        del value[field]
    path.write_text(json.dumps(value))
for path in pathlib.Path("jobs").glob("*/*/result.json"):
    value = json.loads(path.read_text())
    if case == "model":
        value["agent_info"]["model_info"]["name"] = "unrelated/model"
    elif case == "missing-model":
        value["agent_info"].pop("model_info")
    path.write_text(json.dumps(value))
"#,
                )
                .unwrap();
                remediation_runner(&staged, |body| {
                    body.replacen(
                        "set -eu",
                        &format!(
                            "set -eu\ntrap 'python3 \"{}\" {case}' EXIT",
                            mutation.display()
                        ),
                        1,
                    )
                });
            }
            let config = match family {
                "terminal-bench-2.1" => write_config(&staged),
                "terminal-bench-3.0" => write_tb30_config(&staged),
                _ => write_deepswe_config(&staged),
            };
            let root = staged.root.join("run");
            let (code, stdout, stderr) = run_native(&staged, &config, &root);
            let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
            if case == "matching" {
                assert_eq!(code, 0, "{family}: {stderr} {stdout}");
                assert!(
                    report["trials"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .all(|trial| trial["classification"]["class"] == "graded-native")
                );
            } else {
                let invalid = report["trials"].as_array().unwrap().iter().all(|trial| {
                    trial["classification"]["class"] == "invalid-execution"
                        && trial["classification"].get("reward").is_none()
                });
                if code == 0 || !invalid {
                    admitted.push(format!("{family}/{case}"));
                }
            }
        }
    }
    assert!(
        admitted.is_empty(),
        "substituted identity fields admitted: {admitted:?}"
    );
}

#[test]
fn remediation_006_plan_material_rejects_without_panic() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("run");
    let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .args(["run", "--plan"])
        .arg(dir.path().join("missing-plan.json"))
        .arg("--native-material")
        .arg(dir.path().join("missing-material.json"))
        .arg("--root")
        .arg(&root)
        .arg("--fixtures")
        .arg(fixtures())
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(2),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!root.exists());
}

#[test]
fn native_frozen_plan_runs_without_fixtures_and_rejects_material_drift() {
    let staged = stage("ok");
    let digest = derive_integrity(&staged);
    let config = staged.root.join("selection.toml");
    fs::write(
        &config,
        selection_experiment_text(&staged, &digest, "[\"fixture-task\"]"),
    )
    .unwrap();
    stage_output_control(&staged, &config);
    let (frozen, stdout) = run_plan_with_closed_engine(&staged, &config);
    assert!(
        frozen.status.success(),
        "{stdout} {}",
        String::from_utf8_lossy(&frozen.stderr)
    );
    let plan = staged.root.join("plan.json");
    let independent = tempfile::tempdir().unwrap();
    fs::remove_file(&config).unwrap();
    for explicit in [false, true] {
        let root = staged.root.join(format!("plan-run-{explicit}"));
        let mut command = Command::new(env!("CARGO_BIN_EXE_opi-eval"));
        command
            .args(["run", "--plan"])
            .arg(&plan)
            .arg("--root")
            .arg(&root)
            .current_dir(independent.path())
            .env_clear()
            .env("PATH", stage_cleanup_docker(&staged.root))
            .env("OPI_EVAL_SCRIPTED_KEY", "<dummy-scripted-credential>");
        if explicit {
            command.arg("--native-material").arg(&staged.material);
        }
        let output = command.output().unwrap();
        assert_eq!(
            output.status.code(),
            Some(0),
            "{} {}",
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        );
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["trials"].as_array().unwrap().len(), 2);
        assert!(
            report["trials"]
                .as_array()
                .unwrap()
                .iter()
                .all(|trial| trial["status"] == "sealed")
        );
    }
    let original_plan = fs::read(&plan).unwrap();
    let original_material = fs::read(&staged.material).unwrap();
    for case in [
        "missing-material",
        "wrong-digest",
        "substituted-material",
        "plan-tamper",
        "selected-products",
    ] {
        fs::write(&plan, &original_plan).unwrap();
        fs::write(&staged.material, &original_material).unwrap();
        let mut material = staged.material.clone();
        match case {
            "missing-material" => {
                fs::remove_file(&staged.material).unwrap();
            }
            "wrong-digest" => {
                let mut bytes = original_material.clone();
                bytes.push(b' ');
                fs::write(&material, bytes).unwrap();
            }
            "substituted-material" => {
                material = staged.root.join("substitute.json");
                fs::write(&material, b"{}").unwrap();
            }
            "plan-tamper" | "selected-products" => {
                let mut value: serde_json::Value = serde_json::from_slice(&original_plan).unwrap();
                if case == "plan-tamper" {
                    value["plan_digest"] = serde_json::json!("0".repeat(64));
                } else {
                    value["experiment"]["source"] = serde_json::json!(
                        value["experiment"]["source"]
                            .as_str()
                            .unwrap()
                            .replace("product = \"opi\"", "product = \"pi\"")
                    );
                    value.as_object_mut().unwrap().remove("plan_digest");
                    value["plan_digest"] = serde_json::json!(sha256_hex(
                        serde_json::to_string(&value).unwrap().as_bytes()
                    ));
                }
                fs::write(&plan, serde_json::to_vec(&value).unwrap()).unwrap();
            }
            _ => unreachable!(),
        }
        let root = staged.root.join(case);
        let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
            .args(["run", "--plan"])
            .arg(&plan)
            .arg("--native-material")
            .arg(material)
            .arg("--root")
            .arg(&root)
            .current_dir(independent.path())
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(2),
            "{case}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!root.exists(), "{case}: rejected before run effects");
        assert!(!String::from_utf8_lossy(&output.stderr).contains("panicked"));
    }
}

#[test]
fn remediation_001_regrade_child_has_no_ambient_secret() {
    let staged = stage("ok");
    remediation_runner(&staged, |body| {
        body.replace("OPI_EVAL_SCRIPTED_KEY", "SELECTED_ACCESS")
            .replacen(
                "set -eu",
                r#"set -eu
if [ "${OPI_EVAL_REGRADE:-0}" = 1 ]; then
  python3 - <<'OBSERVE'
import json, os
names = ["ANTHROPIC_API_KEY", "OPENAI_API_KEY", "ACCESS_TOKEN", "SERVICE_SECRET",
         "DATABASE_PASSWORD", "OTHER_CREDENTIAL", "SELECTED_ACCESS"]
with open("ambient-credential-observation", "w") as output:
    json.dump({"present": [name for name in names if name in os.environ],
               "configuration": os.environ.get("VERIFIER_CONFIGURATION")}, output)
OBSERVE
  [ "${VERIFIER_FAIL:-0}" = 0 ] || exit 17
fi"#,
                1,
            )
    });
    let config = write_config(&staged);
    fs::write(
        &config,
        fs::read_to_string(&config)
            .unwrap()
            .replace("OPI_EVAL_SCRIPTED_KEY", "SELECTED_ACCESS"),
    )
    .unwrap();
    let root = staged.root.join("run");
    let run = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .args(["run", "--config"])
        .arg(&config)
        .arg("--root")
        .arg(&root)
        .arg("--native-material")
        .arg(&staged.material)
        .arg("--fixtures")
        .arg(fixtures())
        .env_clear()
        .env("PATH", stage_cleanup_docker(&staged.root))
        .env("SELECTED_ACCESS", "<dummy-scripted-credential>")
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{} {}",
        String::from_utf8_lossy(&run.stderr),
        String::from_utf8_lossy(&run.stdout)
    );
    let mut material: serde_json::Value =
        serde_json::from_slice(&fs::read(&staged.material).unwrap()).unwrap();
    for fail in [false, true] {
        let env = &mut material["benchmarks"]["terminal-bench-2.1"]["verifier_env"];
        env["VERIFIER_CONFIGURATION"] = serde_json::json!("admitted-non-secret");
        env["VERIFIER_FAIL"] = serde_json::json!(if fail { "1" } else { "0" });
        fs::write(&staged.material, serde_json::to_vec(&material).unwrap()).unwrap();
        let out = staged.root.join(format!("regrade-{fail}"));
        let mut command = Command::new(env!("CARGO_BIN_EXE_opi-eval"));
        command
            .args(["regrade", "--root"])
            .arg(&root)
            .arg("--native-material")
            .arg(&staged.material)
            .arg("--out")
            .arg(&out)
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap());
        for key in [
            "ANTHROPIC_API_KEY",
            "OPENAI_API_KEY",
            "ACCESS_TOKEN",
            "SERVICE_SECRET",
            "DATABASE_PASSWORD",
            "OTHER_CREDENTIAL",
            "SELECTED_ACCESS",
        ] {
            command.env(key, "synthetic-remediation-canary");
        }
        let output = command.output().unwrap();
        assert_eq!(
            output.status.code(),
            Some(i32::from(fail)),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
        let observations = walk_files_matching(&out, "ambient-credential-observation");
        assert_eq!(
            observations.len(),
            2,
            "both actual children must be observed"
        );
        for path in observations {
            let observed: serde_json::Value =
                serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
            assert_eq!(
                observed["present"],
                serde_json::json!([]),
                "actual verifier child inherited a credential"
            );
            assert_eq!(observed["configuration"], "admitted-non-secret");
        }
        if fail {
            let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            assert!(
                report["rejections"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|entry| entry["reason"] == "fresh-verifier-failed:17")
            );
        }
    }
    for key in [
        "API_KEY",
        "ACCESS_TOKEN",
        "SERVICE_SECRET",
        "DATABASE_PASSWORD",
        "OTHER_CREDENTIAL",
    ] {
        let mut rejected = material.clone();
        rejected["benchmarks"]["terminal-bench-2.1"]["verifier_env"][key] =
            serde_json::json!("synthetic-remediation-canary");
        fs::write(&staged.material, serde_json::to_vec(&rejected).unwrap()).unwrap();
        let out = staged.root.join(format!("rejected-{key}"));
        let (code, report, stderr) = regrade_run(&staged, &root, &out);
        assert_eq!(code, 1, "{report} {stderr}");
        assert_eq!(report["rejections"].as_array().unwrap().len(), 2);
        assert!(
            report["rejections"]
                .as_array()
                .unwrap()
                .iter()
                .all(|entry| entry["reason"] == "verifier-env-credential")
        );
        assert!(walk_files_matching(&out, "ambient-credential-observation").is_empty());
    }
}

fn walk_files(root: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(walk_files(&path));
            } else {
                files.push(path);
            }
        }
    }
    files
}

fn walk_files_matching(root: &Path, name: &str) -> Vec<std::path::PathBuf> {
    walk_files(root)
        .into_iter()
        .filter(|path| path.file_name().is_some_and(|n| n == name))
        .collect()
}

#[test]
fn remediation_012_native_unobserved_time_is_unknown() {
    let staged = stage("ok");
    let config = write_config(&staged);
    let (root, _) = regrade_family_root(&staged, &config, "run");
    let mut observed = 0;
    for path in walk_files_matching(&root, "trajectory.json") {
        let value: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        for node in value["nodes"].as_array().unwrap() {
            if node["provenance"]["rule"] == "agent-execution/1" {
                observed += 1;
                assert!(
                    node["facts"]["wall_time_ms"]["Unknown"]["reason"]
                        .as_str()
                        .is_some_and(|reason| !reason.is_empty()),
                    "native unobserved time must carry an unknown reason: {}",
                    node["facts"]["wall_time_ms"]
                );
            }
        }
    }
    assert!(observed >= 2, "inspect both native Agent execution nodes");
}

#[test]
fn remediation_007_prepare_rejects_impossible_storage() {
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join("cache");
    seed_local_cache(&cache);
    let task = cache.join("corpora/terminal-bench-2.1/tasks/openssl-selfsigned-cert/task.toml");
    let mut text = fs::read_to_string(&task).unwrap();
    if text.contains("[environment]") {
        text = text.replace("storage_mb = 0", "storage_mb = 9000000000000");
    } else {
        text.push_str("\n[environment]\nstorage_mb = 9000000000000\n");
    }
    fs::write(task, text).unwrap();
    repin_cache_corpus(&cache, "terminal-bench-2.1");
    let config = experiment_with_cache(dir.path(), &cache);
    let output = run_prepare_env(
        &config,
        &dir.path().join("prepared"),
        &[("DOCKER_HOST", "tcp://127.0.0.1:1")],
    );
    assert!(
        !output.status.success(),
        "impossible storage admitted: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(
        error.contains("storage") && (error.contains("unmet") || error.contains("unknown")),
        "{error}"
    );
    assert!(!dir.path().join("prepared/material.json").exists());
}

#[test]
fn remediation_008_feasible_second_task_can_freeze() {
    let staged = stage("ok");
    stage_corpus(&staged);
    let digest = derive_integrity(&staged);
    let config = staged.root.join("selection.toml");
    fs::write(
        &config,
        selection_experiment_text(&staged, &digest, "[\"fixture-task\", \"alpha-notes\"]"),
    )
    .unwrap();
    stage_output_control(&staged, &config);
    let (output, stdout) = run_plan_with_closed_engine(&staged, &config);
    assert!(
        output.status.success(),
        "feasible multi-task subset rejected: {stdout} {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn stage_two_task_family(staged: &Staged, adapter: &str) -> (String, String) {
    let mut material: serde_json::Value =
        serde_json::from_slice(&fs::read(&staged.material).unwrap()).unwrap();
    let original = material["benchmarks"][adapter]["tasks"]
        .as_object()
        .unwrap();
    let (first, task) = original.iter().next().unwrap();
    let first = first.clone();
    let task = task.clone();
    let second = "second-task".to_owned();
    let project = staged.root.join("two-task-corpus");
    for id in [&first, &second] {
        let source = project.join("tasks").join(id);
        copy_dir(Path::new(task["task_package"].as_str().unwrap()), &source);
        let mut doc: toml::Value = fs::read_to_string(source.join("task.toml"))
            .unwrap()
            .parse()
            .unwrap();
        let native = if adapter == "deepswe-v1.1" {
            format!("datacurve/{id}")
        } else {
            id.to_owned()
        };
        let fields = doc["task"].as_table_mut().unwrap();
        if fields.contains_key("name") {
            fields.insert("name".to_owned(), native.into());
        } else {
            fields.insert("id".to_owned(), native.into());
        }
        fs::write(source.join("task.toml"), toml::to_string(&doc).unwrap()).unwrap();
        fs::write(
            source.join("instruction.md"),
            format!("Exact instruction for {id}.\n"),
        )
        .unwrap();
    }
    let (corpus, pin) = fixture_corpus_pin(&project, adapter);
    let mut tasks = serde_json::Map::new();
    for id in [&first, &second] {
        let package = staged.root.join("two-task-packages").join(id);
        copy_dir(&project.join("tasks").join(id), &package);
        let profile = staged.root.join(format!("two-task-{id}.toml"));
        fs::copy(task["profile"]["path"].as_str().unwrap(), &profile).unwrap();
        tasks.insert(
            id.clone(),
            fixture_task_binding(&project, id, &package, &profile),
        );
    }
    material["benchmarks"][adapter]["corpus"] = corpus;
    material["benchmarks"][adapter]["tasks"] = tasks.into();
    let path = PathBuf::from(material["static_lock"]["path"].as_str().unwrap());
    let mut lock: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    *lock["subjects"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|subject| subject["id"] == adapter)
        .unwrap() = pin;
    let bytes = serde_json::to_vec(&lock).unwrap();
    fs::write(path, &bytes).unwrap();
    material["static_lock"]["sha256"] = serde_json::json!(sha256_hex(&bytes));
    fs::write(&staged.material, serde_json::to_vec(&material).unwrap()).unwrap();
    (first, second)
}

#[test]
fn material_five_dispatches_two_exact_tasks_per_family_from_both_entries() {
    for adapter in ["terminal-bench-2.1", "terminal-bench-3.0", "deepswe-v1.1"] {
        let staged = stage("ok");
        let (first, second) = stage_two_task_family(&staged, adapter);
        let config = match adapter {
            "terminal-bench-2.1" => write_config(&staged),
            "terminal-bench-3.0" => write_tb30_config(&staged),
            _ => write_deepswe_config(&staged),
        };
        let explicit = fs::read_to_string(&config).unwrap();
        let head = explicit.split("[[trials]]").next().unwrap();
        let independent = tempfile::tempdir().unwrap();
        for full in [false, true] {
            let selection = if full {
                "scope = \"full\"\n".to_owned()
            } else {
                format!("scope = \"subset\"\ntask_ids = [{first:?}, {second:?}]\n")
            };
            fs::write(
                &config,
                format!(
                    "{head}[selection]\n{selection}reason = \"two exact tasks\"\nrepetitions = 1\n"
                ),
            )
            .unwrap();
            stage_output_control(&staged, &config);
            let plan = staged.root.join(format!("plan-{full}.json"));
            let planned = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
                .args(["plan", "--config"])
                .arg(&config)
                .arg("--native-material")
                .arg(&staged.material)
                .arg("--out")
                .arg(&plan)
                .current_dir(independent.path())
                .output()
                .unwrap();
            assert!(
                planned.status.success(),
                "{adapter}: {}",
                String::from_utf8_lossy(&planned.stderr)
            );
            let preview: serde_json::Value = serde_json::from_slice(&planned.stdout).unwrap();
            assert_eq!(preview["trial_count"], 4);
            assert_eq!(preview["corpus"].as_array().unwrap().len(), 2);
            let frozen: serde_json::Value =
                serde_json::from_slice(&fs::read(&plan).unwrap()).unwrap();
            for entry in frozen["corpus"].as_array().unwrap() {
                assert_eq!(entry["material"]["status"], "bound");
                assert!(entry["requirements"]["agent"]["origins"].is_object());
                assert!(entry["requirements"]["verifier"]["origins"].is_object());
            }
            let root = staged.root.join(format!("two-run-{full}"));
            let input = if full { &plan } else { &config };
            let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
                .args(["run", if full { "--plan" } else { "--config" }])
                .arg(input)
                .arg("--native-material")
                .arg(&staged.material)
                .arg("--root")
                .arg(&root)
                .env("OPI_EVAL_SCRIPTED_KEY", "<dummy-scripted-credential>")
                .current_dir(independent.path())
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{adapter}: {} {}",
                String::from_utf8_lossy(&output.stderr),
                String::from_utf8_lossy(&output.stdout)
            );
            let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(report["trials"].as_array().unwrap().len(), 4);
            for trial in report["trials"].as_array().unwrap() {
                assert_eq!(trial["status"], "sealed");
                assert_eq!(trial["classification"]["class"], "graded-native");
                let id = trial["id"].as_str().unwrap();
                let task_id = if id.contains(&first) { &first } else { &second };
                let trial_root = root.join("trials").join(id);
                let native_config: serde_json::Value = serde_json::from_slice(
                    &fs::read(trial_root.join("bundle/artifacts/native/trial-config.json"))
                        .unwrap(),
                )
                .unwrap();
                let task_path = Path::new(native_config["task"]["path"].as_str().unwrap());
                assert_eq!(
                    fs::read_to_string(task_path.join("instruction.md")).unwrap(),
                    format!("Exact instruction for {task_id}.\n")
                );
                let job: serde_json::Value = serde_json::from_slice(
                    &fs::read(trial_root.join("verifier-trace/native-job-config.json")).unwrap(),
                )
                .unwrap();
                assert_eq!(job["tasks"][0]["path"], native_config["task"]["path"]);
            }
        }
    }
}

#[test]
fn material_five_rejects_swapped_omitted_and_stale_task_bindings() {
    let staged = stage("ok");
    let (first, second) = stage_two_task_family(&staged, "terminal-bench-2.1");
    let digest = derive_integrity(&staged);
    let config = staged.root.join("selection.toml");
    fs::write(
        &config,
        selection_experiment_text(&staged, &digest, &format!("[{first:?}, {second:?}]")),
    )
    .unwrap();
    stage_output_control(&staged, &config);
    let material: serde_json::Value =
        serde_json::from_slice(&fs::read(&staged.material).unwrap()).unwrap();
    for case in [
        "profile",
        "package",
        "name",
        "omitted",
        "tree",
        "root",
        "old-schema",
        "source",
        "missing-image",
    ] {
        let mut changed = material.clone();
        let benchmark = &mut changed["benchmarks"]["terminal-bench-2.1"];
        match case {
            "profile" => {
                benchmark["tasks"][&second]["profile"] =
                    benchmark["tasks"][&first]["profile"].clone()
            }
            "package" => {
                benchmark["tasks"][&second]["task_package"] =
                    benchmark["tasks"][&first]["task_package"].clone();
                benchmark["tasks"][&second]["task_package_manifest_sha256"] =
                    benchmark["tasks"][&first]["task_package_manifest_sha256"].clone();
            }
            "name" => benchmark["tasks"][&second]["native_task_name"] = serde_json::json!(first),
            "omitted" => {
                benchmark["tasks"].as_object_mut().unwrap().remove(&second);
            }
            "tree" => benchmark["corpus"]["tasks_tree"] = serde_json::json!("0".repeat(40)),
            "root" => {
                benchmark["corpus"]["root"] =
                    serde_json::json!(staged.root.join("two-task-corpus/tasks").join(&first))
            }
            "missing-image" => fs::write(staged.root.join("missing-image"), "absent").unwrap(),
            "old-schema" => changed["schema"] = serde_json::json!("opi-eval-native-material/4"),
            "source" => fs::write(
                staged
                    .root
                    .join("two-task-corpus/tasks")
                    .join(&first)
                    .join("rogue"),
                "untracked",
            )
            .unwrap(),
            _ => unreachable!(),
        }
        fs::write(&staged.material, serde_json::to_vec(&changed).unwrap()).unwrap();
        let out = staged.root.join(format!("reject-{case}.json"));
        let result = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
            .args(["plan", "--config"])
            .arg(&config)
            .arg("--native-material")
            .arg(&staged.material)
            .arg("--out")
            .arg(&out)
            .output()
            .unwrap();
        assert_eq!(
            result.status.code(),
            Some(2),
            "{case}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(!out.exists());
        if case == "missing-image" {
            fs::remove_file(staged.root.join("missing-image")).unwrap();
        }
        if case == "source" {
            fs::remove_file(
                staged
                    .root
                    .join("two-task-corpus/tasks")
                    .join(&first)
                    .join("rogue"),
            )
            .unwrap();
        }
    }
    fs::write(&staged.material, serde_json::to_vec(&material).unwrap()).unwrap();
    let (valid, stdout) = run_plan_with_closed_engine(&staged, &config);
    assert!(
        valid.status.success(),
        "{stdout} {}",
        String::from_utf8_lossy(&valid.stderr)
    );
    let path = staged.root.join("plan.json");
    let mut plan: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    plan["schema"] = serde_json::json!("opi-eval-run-plan/1");
    fs::write(&path, serde_json::to_vec(&plan).unwrap()).unwrap();
    let rejected_root = staged.root.join("old-plan-run");
    let rejected = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .args(["run", "--plan"])
        .arg(&path)
        .arg("--root")
        .arg(&rejected_root)
        .output()
        .unwrap();
    assert_eq!(rejected.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("regenerate"));
    assert!(!rejected_root.exists());
    let calls = fs::read_to_string(staged.root.join("docker-calls.jsonl")).unwrap();
    assert!(
        calls.lines().all(|line| line.starts_with("[\"info\"")
            || line.starts_with("[\"context\"")
            || line.starts_with("[\"image\"")),
        "preview started an image: {calls}"
    );
}

fn walk_text(root: &Path) -> String {
    walk_files(root)
        .into_iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Rewrites the staged material's deepswe task package with `edit`
/// applied to its task.toml, recomputing the manifest digest so the
/// variant material stays internally admissible.
fn deepswe_task_variant(staged: &Staged, name: &str, edit: impl FnOnce(&mut String)) -> PathBuf {
    let variant_dir = staged.root.join(name);
    copy_dir(
        &staged.root.join("material/deepswe-task-package"),
        &variant_dir,
    );
    let toml_path = variant_dir.join("task.toml");
    let mut text = fs::read_to_string(&toml_path).unwrap();
    edit(&mut text);
    fs::write(&toml_path, text).unwrap();
    let mut material: serde_json::Value =
        serde_json::from_slice(&fs::read(&staged.material).unwrap()).unwrap();
    let project = staged.root.join(format!("{name}-corpus"));
    copy_dir(&variant_dir, &project.join("tasks/synthetic-fixture-task"));
    let (corpus, pin) = fixture_corpus_pin(&project, "deepswe-v1.1");
    let profile = staged.root.join(format!("{name}.toml"));
    fs::copy(staged.root.join("material/deepswe-profile.toml"), &profile).unwrap();
    material["benchmarks"]["deepswe-v1.1"]["tasks"]["synthetic-fixture-task"] =
        fixture_task_binding(&project, "synthetic-fixture-task", &variant_dir, &profile);
    material["benchmarks"]["deepswe-v1.1"]["corpus"] = corpus;
    let mut lock: serde_json::Value = serde_json::from_slice(
        &fs::read(material["static_lock"]["path"].as_str().unwrap()).unwrap(),
    )
    .unwrap();
    *lock["subjects"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|entry| entry["id"] == "deepswe-v1.1")
        .unwrap() = pin;
    let lock_path = staged.root.join(format!("{name}-lock.json"));
    let bytes = serde_json::to_vec(&lock).unwrap();
    fs::write(&lock_path, &bytes).unwrap();
    material["static_lock"] = serde_json::json!({"path": lock_path, "sha256": sha256_hex(&bytes)});
    fs::write(
        &staged.material,
        serde_json::to_vec_pretty(&material).unwrap(),
    )
    .unwrap();
    staged.material.clone()
}

/// `P19-AC-009` (regrade fail-closed): every missing dependency or
/// identity drift rejects offline regrading with a typed reason, and the
/// fresh grade follows the preserved witness bytes rather than any
/// recorded score.
#[test]
fn native_offline_regrade_rejects_every_fail_closed_case() {
    let staged = stage("ok");
    let original_material = fs::read(&staged.material).unwrap();

    // Tampered source bundle: the seal rejects before any fresh grading.
    {
        let config = write_config(&staged);
        let (run_root, _) = regrade_family_root(&staged, &config, "run-tamper");
        let artifact =
            run_root.join("trials/trial-opi-native/bundle/artifacts/native/trial-result.json");
        let mut bytes = fs::read(&artifact).unwrap();
        bytes.push(b' ');
        fs::write(&artifact, bytes).unwrap();
        let out = staged.root.join("regrade-tamper");
        let (code, report, stderr) = regrade_run(&staged, &run_root, &out);
        assert_eq!(code, 1, "{stderr} {report}");
        assert_eq!(report["outcome"], "rejected", "{report}");
        let rejection = &report["rejections"][0];
        assert_eq!(rejection["trial"], "trial-opi-native", "{report}");
        assert!(
            rejection["reason"].as_str().unwrap().starts_with("bundle-"),
            "{report}"
        );
        assert!(
            report["trials"]
                .as_array()
                .unwrap()
                .iter()
                .all(|trial| trial["trial"] != "trial-opi-native"),
            "{report}"
        );
    }

    // Missing saved Agent artifacts: the preserved job tree is gone.
    {
        let config = write_config(&staged);
        let (run_root, _) = regrade_family_root(&staged, &config, "run-missing");
        std::fs::remove_dir_all(run_root.join("trials/trial-opi-native/verifier-trace")).unwrap();
        let out = staged.root.join("regrade-missing");
        let (code, report, _) = regrade_run(&staged, &run_root, &out);
        assert_eq!(code, 1);
        let rejection = &report["rejections"][0];
        assert_eq!(rejection["trial"], "trial-opi-native", "{report}");
        assert_eq!(rejection["reason"], "agent-artifacts-missing", "{report}");
    }

    // Tampered preserved Agent bytes: digests reject against the sealed
    // verifier-input identity before any grading.
    {
        let config = write_config(&staged);
        let (run_root, _) = regrade_family_root(&staged, &config, "run-altered");
        let jobs_root = run_root.join("trials/trial-opi-native/verifier-trace/jobs");
        let mut jobs: Vec<_> = fs::read_dir(&jobs_root)
            .unwrap()
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.join("result.json").is_file())
            .collect();
        jobs.sort();
        assert_eq!(jobs.len(), 1, "one job tree");
        let trial_dirs: Vec<_> = fs::read_dir(&jobs[0])
            .unwrap()
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect();
        assert_eq!(trial_dirs.len(), 1, "one trial inside the job");
        let answer_file = trial_dirs[0].join("artifacts/root/answer.txt");
        fs::write(&answer_file, b"substituted witness bytes\n").unwrap();
        let out = staged.root.join("regrade-altered");
        let (code, report, _) = regrade_run(&staged, &run_root, &out);
        assert_eq!(code, 1);
        let rejection = &report["rejections"][0];
        assert_eq!(rejection["trial"], "trial-opi-native", "{report}");
        assert_eq!(rejection["reason"], "agent-artifacts-tampered", "{report}");
    }

    // Wrong patch base: the fresh material's task declares a different
    // base revision than the sealed verifier-input recorded.
    {
        let config = write_deepswe_config(&staged);
        let (run_root, _) = regrade_family_root(&staged, &config, "run-base");
        deepswe_task_variant(&staged, "deepswe-base-variant", |text| {
            *text = text.replace(
                "1111111111111111111111111111111111111112",
                "9999999999999999999999999999999999999999",
            );
        });
        let out = staged.root.join("regrade-base");
        let (code, report, _) = regrade_run(&staged, &run_root, &out);
        assert_eq!(code, 1);
        assert_eq!(
            report["rejections"][0]["reason"], "patch-base-drift",
            "{report}"
        );
        fs::write(&staged.material, &original_material).unwrap();
    }

    // A verifier requiring network has no permitted offline equivalent
    // here: the task itself declares public network access.
    {
        let config = write_deepswe_config(&staged);
        let (run_root, _) = regrade_family_root(&staged, &config, "run-network");
        deepswe_task_variant(&staged, "deepswe-network-variant", |text| {
            *text = text.replace("network_mode = \"no-network\"", "network_mode = \"public\"");
        });
        let out = staged.root.join("regrade-network");
        let (code, report, _) = regrade_run(&staged, &run_root, &out);
        assert_eq!(code, 1);
        assert_eq!(
            report["rejections"][0]["reason"], "verifier-network-required",
            "{report}"
        );
        fs::write(&staged.material, &original_material).unwrap();
    }

    // Existing output root: request-level rejection, nothing overwritten.
    {
        let config = write_config(&staged);
        let (run_root, _) = regrade_family_root(&staged, &config, "run-exists");
        let out = staged.root.join("regrade-exists");
        fs::create_dir_all(&out).unwrap();
        fs::write(out.join("prior.txt"), b"prior").unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
            .arg("regrade")
            .arg("--root")
            .arg(&run_root)
            .arg("--native-material")
            .arg(&staged.material)
            .arg("--out")
            .arg(&out)
            .output()
            .expect("spawn the opi-eval regrade command");
        assert_eq!(output.status.code(), Some(2));
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("output root already exists"),
            "stderr names the request rejection"
        );
        assert_eq!(fs::read(out.join("prior.txt")).unwrap(), b"prior");
    }

    // Changed witness input: the DeepSWE nop run preserved an empty
    // committed patch; the fresh verifier grades it zero - the regrade
    // consumes the preserved bytes, not a recorded score.
    {
        set_deepswe_mode(&staged, "deepswe-nop");
        let config = write_deepswe_config(&staged);
        let (run_root, _) = regrade_family_root(&staged, &config, "run-nop");
        let out = staged.root.join("regrade-nop");
        let (code, report, stderr) = regrade_run(&staged, &run_root, &out);
        assert_eq!(code, 0, "{stderr} {report}");
        assert_eq!(report["outcome"], "regraded", "{report}");
        for trial in report["trials"].as_array().unwrap() {
            assert_eq!(trial["reward"], 0, "empty patch grades zero: {report}");
        }
    }
}

/// `P19-AC-010` (bounded native cancellation): one run-level
/// cancellation token reaches the native job supervisor. A SIGTERM at the
/// CLI stops new affected dispatch (the paired second trial never
/// starts), settles the observed job outcome as cancelled infrastructure
/// failure without a fabricated score, terminates only the owned process
/// tree (the witness child dies - no orphan execution survives the
/// successful cancellation), removes the temporary credential material,
/// and emits resource cleanup evidence. The offline report keeps both
/// the cancelled trial and the never-dispatched trial as explicit
/// missing coverage.
#[test]
fn native_signal_cancellation_bounds_dispatch() {
    use std::io::Read;
    use std::process::Stdio;

    let staged = stage("owned-cancel");
    let config = write_config(&staged);
    let run_root = staged.root.join("run");

    let mut child = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("run")
        .arg("--config")
        .arg(&config)
        .arg("--root")
        .arg(&run_root)
        .arg("--fixtures")
        .arg(fixtures())
        .arg("--native-material")
        .arg(&staged.material)
        .env("OPI_EVAL_SCRIPTED_KEY", "<dummy-scripted-credential>")
        .env("PATH", stage_cleanup_docker(&staged.root))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn the opi-eval run command");
    let cli_pid = child.id();

    // Wait until the dispatched job provably owns a live child process,
    // then cancel the whole run through the CLI signal surface.
    let mut witness = None;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(90);
    while std::time::Instant::now() < deadline {
        for trial in ["trial-opi-native", "trial-pi-native"] {
            let pid_file = run_root
                .join("trials")
                .join(trial)
                .join("verifier-trace/witness/pid");
            if pid_file.is_file() {
                witness = Some((trial.to_owned(), pid_file));
                break;
            }
        }
        if witness.is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    let (dispatched, pid_file) = witness.expect("the native job publishes its owned child witness");
    let owned_pid: i32 = fs::read_to_string(&pid_file)
        .unwrap()
        .trim()
        .parse()
        .expect("witness pid is numeric");
    assert!(
        nix_kill_alive(owned_pid),
        "the owned child is provably alive before cancellation"
    );

    unsafe { libc::kill(cli_pid as i32, libc::SIGTERM) };
    let mut stdout = String::new();
    let mut stderr = String::new();
    if let Some(mut pipe) = child.stdout.take() {
        pipe.read_to_string(&mut stdout).unwrap();
    }
    if let Some(mut pipe) = child.stderr.take() {
        pipe.read_to_string(&mut stderr).unwrap();
    }
    let status = child.wait().expect("the cancelled run exits");

    // The interrupted run still settles and renders its report: exit 1
    // with one JSON report naming the incomplete outcome.
    assert_eq!(status.code(), Some(1), "stderr: {stderr} stdout: {stdout}");
    let report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|error| {
        panic!("stdout is not one JSON report ({error}): {stdout:?} stderr: {stderr:?}")
    });
    assert_eq!(report["outcome"], "incomplete", "{report}");

    // Stop-new-dispatch: exactly one trial dispatched and settled; the
    // paired trial never started.
    let trials = report["trials"].as_array().unwrap();
    assert_eq!(
        trials.len(),
        1,
        "only the in-flight trial settled: {report}"
    );
    assert_eq!(trials[0]["id"], dispatched);
    assert_eq!(trials[0]["status"], "failed", "{:?}", trials[0]);
    assert_eq!(
        trials[0]["verifier"]["exit_state"], "cancelled",
        "{trials:?}"
    );
    // The observed cancellation is settled honestly: the reward stays an
    // explicit unknown token (never a fabricated numeric score), the
    // grade transition is mechanically refused, and cleanup evidence is
    // emitted for the terminated owned tree.
    assert!(
        trials[0]["verifier"]["reward"]
            .as_str()
            .is_some_and(|token| token.starts_with("unknown")),
        "no fabricated score after cancellation: {trials:?}"
    );
    assert_eq!(trials[0]["classification"]["class"], "no-outcome");
    assert_eq!(trials[0]["classification"]["reason"], "job-cancelled");
    assert!(
        trials[0]["verifier"]["cleanup"]
            .as_str()
            .is_some_and(|token| token.starts_with("tree-terminated")),
        "cleanup evidence for the owned tree: {trials:?}"
    );
    assert!(
        trials[0]["authority"]["grade_dispatch"]
            .as_str()
            .is_some_and(|token| token.starts_with("failed")),
        "grade admission mechanically fails after cancellation - never a silent grade: {trials:?}"
    );
    let other = if dispatched == "trial-opi-native" {
        "trial-pi-native"
    } else {
        "trial-opi-native"
    };
    assert!(
        !run_root.join("trials").join(other).exists(),
        "the paired trial is never dispatched after cancellation"
    );

    // No orphan execution survives the successful cancellation: the owned
    // child witnessed before the signal is gone.
    let mut gone = false;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while std::time::Instant::now() < deadline {
        if !nix_kill_alive(owned_pid) {
            gone = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert!(gone, "the owned child must not outlive the run");
    // Temporary credential material is removed; the owned-resource
    // identity and its cleanup result stay as durable evidence.
    let integration = run_root
        .join("trials")
        .join(&dispatched)
        .join("integration");
    assert!(
        !integration.join("handoff.json").exists(),
        "the credential handoff is removed after settlement"
    );
    let owned = serde_json::Value::Object(serde_json::Map::from_iter(std::iter::once((
        "path".to_owned(),
        serde_json::Value::String(
            integration
                .join("owned-resources.json")
                .to_string_lossy()
                .into_owned(),
        ),
    ))));
    let owned_text = fs::read_to_string(owned["path"].as_str().unwrap())
        .unwrap_or_else(|error| panic!("owned-resources evidence: {error}"));
    let owned: serde_json::Value = serde_json::from_str(&owned_text).unwrap();
    assert!(
        owned["cleanup"].is_string(),
        "cleanup evidence names the owned resources: {owned}"
    );

    // The offline report keeps both sides as explicit missing coverage:
    // the cancelled trial is invalid execution, the undistpatched trial
    // unexecuted, and neither carries a fabricated score.
    let out = staged.root.join("cancel-report");
    let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("report")
        .arg("--root")
        .arg(&run_root)
        .arg("--out")
        .arg(&out)
        .output()
        .expect("spawn the opi-eval report command");
    assert_eq!(output.status.code(), Some(0), "{:?}", output.status);
    let report: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&output.stdout).trim()).unwrap();
    assert_eq!(report["outcome"], "published", "{report}");
    assert_eq!(report["run_outcome"], "incomplete", "{report}");
    // The durable cancelled attempt remains invalid coverage without a score;
    // its undispatched pair remains unexecuted in the plan denominator.
    let counts = &report["coverage_counts"];
    assert_eq!(counts["planned"], 2, "{counts}");
    assert_eq!(counts["attempted"], 1, "{counts}");
    assert_eq!(counts["valid"], 0, "{counts}");
    assert_eq!(counts["invalid"], 1, "{counts}");
    assert_eq!(counts["unexecuted"], 1, "{counts}");
    for trial in report["trials"].as_array().unwrap() {
        assert_ne!(
            trial["outcome"]["reward"],
            serde_json::json!(0.0),
            "no fabricated zero score: {trial}"
        );
        let class = trial["outcome"]["class"].as_str().unwrap_or_default();
        assert!(
            class == "invalid" || class == "unexecuted",
            "cancelled and undistpatched trials stay explicit missing coverage: {trial}"
        );
    }
}

/// Cancellation retains a bounded private prefix view without altering raw
/// upstream bytes, following links, admitting a terminal, or resolving a model.
#[test]
fn native_cancel_preserves_bounded_partial_capture() {
    use std::process::Stdio;
    let staged = stage("owned-cancel");
    let config = write_config(&staged);
    let run_root = staged.root.join("run");
    let mut child = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .args(["run", "--config"])
        .arg(&config)
        .arg("--root")
        .arg(&run_root)
        .arg("--fixtures")
        .arg(fixtures())
        .arg("--native-material")
        .arg(&staged.material)
        .env("OPI_EVAL_SCRIPTED_KEY", "<dummy-scripted-credential>")
        .env("PATH", stage_cleanup_docker(&staged.root))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    let dispatched = loop {
        let ready = ["trial-opi-native", "trial-pi-native"]
            .into_iter()
            .find(|trial| {
                run_root
                    .join("trials")
                    .join(trial)
                    .join("verifier-trace/witness/ready")
                    .is_file()
            });
        if let Some(trial) = ready {
            break trial;
        }
        assert!(
            child.try_wait().unwrap().is_none(),
            "fixture exited before progress"
        );
        if std::time::Instant::now() >= deadline {
            unsafe { libc::kill(child.id() as i32, libc::SIGTERM) };
            child.wait().unwrap();
            panic!("fixture did not produce partial capture");
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    };
    unsafe { libc::kill(child.id() as i32, libc::SIGTERM) };
    let output = child.wait_with_output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["trials"][0]["verifier"]["exit_state"], "cancelled");
    assert_eq!(report["trials"][0]["classification"]["class"], "no-outcome");
    assert!(
        report["trials"][0]["verifier"]["reward"]
            .as_str()
            .unwrap()
            .starts_with("unknown")
    );
    let integration = run_root.join("trials").join(dispatched).join("integration");
    for name in ["handoff.json", "config.toml", "models.json"] {
        assert!(!integration.join(name).exists());
    }
    let trace = run_root
        .join("trials")
        .join(dispatched)
        .join("verifier-trace");
    let capture = integration.join("partial-capture/jobs/partial/task__partial/agent");
    assert_eq!(
        fs::metadata(trace.join("jobs/partial/task__partial/agent/stdout.log"))
            .unwrap()
            .len(),
        2 * 1024 * 1024 + 14
    );
    let stdout = fs::read(capture.join("stdout.log")).unwrap();
    assert_eq!(stdout.len(), 1024 * 1024, "partial stdout must be bounded");
    assert!(stdout.starts_with(b"tool-progress\n"));
    assert_eq!(
        fs::read(capture.join("stderr.log")).unwrap(),
        b"diagnostic-progress\n"
    );
    assert_eq!(
        fs::read(capture.join("trace/run-1/evidence.jsonl")).unwrap(),
        b"trace-progress\n"
    );
    assert_eq!(
        fs::read(trace.join("witness/neighbor")).unwrap(),
        b"untouched-neighbor\n"
    );
    for name in ["link", "hardlink", "fifo"] {
        assert!(fs::symlink_metadata(capture.join("trace/run-1").join(name)).is_err());
    }
    assert!(!capture.join("opi-eval-terminal.json").exists());
    assert!(!trace.join("jobs/partial/result.json").exists());
    let partial: serde_json::Value =
        serde_json::from_slice(&fs::read(integration.join("partial-capture.json")).unwrap())
            .unwrap();
    assert_eq!(partial["truncated_files"], 1);
    assert_eq!(partial["discarded_entries"], 3);
    let pid: i32 = fs::read_to_string(trace.join("witness/pid"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert!(!nix_kill_alive(pid));
}

/// Probes whether `pid` is still alive (signal 0 delivery).
fn nix_kill_alive(pid: i32) -> bool {
    unsafe { libc::kill(pid, 0) == 0 }
}

/// `P19-AC-010` (native no-model Docker cleanup witness): the native job
/// starts one real container with the pinned runner's exact-root labels.
/// A SIGTERM at the CLI cancels only the owned process tree and removes
/// exactly the root-bound container - the
/// successful cancellation leaves no orphan execution, the durable
/// owned-resource evidence names the removed container, and the shared
/// material stays byte-identical.
///
/// Ignored: needs the local Docker engine and the `busybox` image (pulled
/// on first use). No model, provider, or remote inference is involved.
#[test]
#[ignore = "requires the local Docker engine and the busybox image"]
fn native_owned_resource_cancellation() {
    use std::io::Read;
    use std::process::Stdio;

    let image = std::process::Command::new("docker")
        .args(["image", "inspect", "busybox"])
        .output()
        .expect("docker CLI present");
    if !image.status.success() {
        let pull = std::process::Command::new("docker")
            .args(["pull", "busybox"])
            .output()
            .expect("docker pull runs");
        assert!(
            pull.status.success(),
            "busybox image is required material: {}",
            String::from_utf8_lossy(&pull.stderr)
        );
    }

    let staged = stage("owned-container");
    let config = write_config(&staged);
    let run_root = staged.root.join("run");
    let material_before = fs::read(&staged.material).unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("run")
        .arg("--config")
        .arg(&config)
        .arg("--root")
        .arg(&run_root)
        .arg("--fixtures")
        .arg(fixtures())
        .arg("--native-material")
        .arg(&staged.material)
        .env("OPI_EVAL_SCRIPTED_KEY", "<dummy-scripted-credential>")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn the opi-eval run command");
    let cli_pid = child.id();

    // Wait until the job owns both a live child process and a running
    // container with the trial-owned root label.
    let mut witness = None;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
    while std::time::Instant::now() < deadline {
        for trial in ["trial-opi-native", "trial-pi-native"] {
            let ready = run_root
                .join("trials")
                .join(trial)
                .join("verifier-trace/witness/ready");
            if ready.is_file() {
                witness = Some(trial.to_owned());
                break;
            }
        }
        if let Some(trial) = &witness {
            let listed = std::process::Command::new("docker")
                .args([
                    "ps",
                    "--filter",
                    &format!(
                        "label=com.docker.compose.project.working_dir={}",
                        run_root
                            .join("trials")
                            .join(trial)
                            .join("task-package/environment")
                            .display()
                    ),
                    "--format",
                    "{{.Names}}",
                ])
                .output()
                .unwrap();
            if !String::from_utf8_lossy(&listed.stdout).trim().is_empty() {
                break;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let dispatched = witness.expect("the native job owns a running container witness");
    let owned_pid: i32 = fs::read_to_string(
        run_root
            .join("trials")
            .join(&dispatched)
            .join("verifier-trace/witness/pid"),
    )
    .unwrap()
    .trim()
    .parse()
    .unwrap();
    assert!(nix_kill_alive(owned_pid), "the owned child is alive");

    unsafe { libc::kill(cli_pid as i32, libc::SIGTERM) };
    let mut stdout = String::new();
    let mut stderr = String::new();
    if let Some(mut pipe) = child.stdout.take() {
        pipe.read_to_string(&mut stdout).unwrap();
    }
    if let Some(mut pipe) = child.stderr.take() {
        pipe.read_to_string(&mut stderr).unwrap();
    }
    let status = child.wait().expect("the cancelled run exits");
    assert_eq!(status.code(), Some(1), "stderr: {stderr} stdout: {stdout}");
    let report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(report["outcome"], "incomplete", "{report}");

    // The persisted run report names the removed owned container: a
    // cancelled trial never seals, so its report transition is
    // mechanically refused and the run report - not a per-trial
    // receipt - is the durable cleanup evidence.
    let persisted: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(run_root.join("run-report.json")).unwrap())
            .unwrap();
    let cancelled_trial = persisted["trials"]
        .as_array()
        .unwrap()
        .iter()
        .find(|trial| trial["id"] == dispatched)
        .expect("the cancelled trial stays in the persisted report");
    let cleanup = cancelled_trial["verifier"]["cleanup"]
        .as_str()
        .unwrap_or_default();
    assert!(
        cleanup.contains("owned-containers:containers:") && cleanup.contains(":removed"),
        "cleanup evidence names the removed container: {cleanup}"
    );

    // No orphan execution: the owned container and the owned child are
    // both gone, and exactly the private task root was addressed.
    let mut gone = false;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    while std::time::Instant::now() < deadline {
        let listed = std::process::Command::new("docker")
            .args([
                "ps",
                "-aq",
                "--filter",
                &format!(
                    "label=com.docker.compose.project.working_dir={}",
                    run_root
                        .join("trials")
                        .join(&dispatched)
                        .join("task-package/environment")
                        .display()
                ),
            ])
            .output()
            .unwrap();
        let names = String::from_utf8_lossy(&listed.stdout).trim().to_owned();
        if names.is_empty() && !nix_kill_alive(owned_pid) {
            gone = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    assert!(gone, "no owned container or child may outlive the run");

    // The durable owned-resource evidence records the exact removal.
    let owned: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(
            run_root
                .join("trials")
                .join(&dispatched)
                .join("integration/owned-resources.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let cleanup = owned["cleanup"].as_str().unwrap_or_default();
    assert!(
        cleanup.starts_with("containers:") && cleanup.ends_with(":removed"),
        "owned-resource evidence names the removed container: {owned}"
    );

    // Shared material stays byte-identical; the credential handoff is
    // gone.
    assert_eq!(fs::read(&staged.material).unwrap(), material_before);
    let trial = run_root.join("trials").join(&dispatched);
    let raw = trial.join("verifier-trace/jobs/partial/task__partial/agent");
    let partial = trial.join("integration/partial-capture/jobs/partial/task__partial/agent");
    assert_eq!(
        fs::read(raw.join("trace/run-1/evidence.jsonl")).unwrap(),
        b"private-root-trace\n"
    );
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let directory = fs::metadata(raw.join("trace/run-1")).unwrap();
    let file = fs::metadata(raw.join("trace/run-1/evidence.jsonl")).unwrap();
    assert_eq!(directory.permissions().mode() & 0o777, 0o700);
    assert_eq!(file.permissions().mode() & 0o777, 0o600);
    assert_eq!(directory.uid(), fs::metadata(&run_root).unwrap().uid());
    let helpers = owned["capture_helpers"].as_array().unwrap();
    assert_eq!(helpers.len(), 1);
    for helper in helpers {
        assert_eq!(helper["removed"], true);
        let listed = Command::new("docker")
            .args([
                "ps",
                "--all",
                "--quiet",
                "--no-trunc",
                "--filter",
                &format!("id={}", helper["id"].as_str().unwrap()),
            ])
            .output()
            .unwrap();
        assert!(listed.status.success());
        assert!(
            listed.stdout.is_empty(),
            "capture helper must not outlive cleanup"
        );
    }
    assert_eq!(
        fs::read(partial.join("trace/run-1/evidence.jsonl")).unwrap(),
        b"private-root-trace\n"
    );
    assert_eq!(
        fs::read(partial.join("stdout.log")).unwrap(),
        b"private-root-stdout\n"
    );
    assert_eq!(
        fs::read(partial.join("stderr.log")).unwrap(),
        b"private-root-stderr\n"
    );
    assert!(!raw.join("opi-eval-terminal.json").exists());
    assert!(
        !run_root
            .join("trials")
            .join(&dispatched)
            .join("integration/handoff.json")
            .exists()
    );
}

/// Builds one family experiment config with per-family trial-id suffixes so
/// the six acceptance trials stay individually attributable across the three
/// preserved family run roots.
fn acceptance_family_config(staged: &Staged, family: &str, suffix: &str) -> PathBuf {
    let base = match family {
        "terminal-bench-2.1" => experiment_text(staged, "pending"),
        "terminal-bench-3.0" => tb30_experiment_text(staged, "pending"),
        "deepswe-v1.1" => deepswe_experiment_text(staged, "pending"),
        other => panic!("unknown acceptance family {other}"),
    };
    let rewrite = |text: String| {
        text.replace("trial-pi-native", &format!("trial-pi-{suffix}"))
            .replace("trial-opi-native", &format!("trial-opi-{suffix}"))
    };
    let undigested = staged
        .root
        .join(format!("config-undigested-accept-{suffix}.toml"));
    fs::write(&undigested, rewrite(base)).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("validate")
        .arg("--config")
        .arg(&undigested)
        .arg("--native-material")
        .arg(&staged.material)
        .output()
        .expect("validate runs");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let digest = stdout
        .split_whitespace()
        .find(|token| token.starts_with("native_integrity="))
        .expect("native integrity digest in summary")
        .trim_start_matches("native_integrity=")
        .to_owned();
    let digested = match family {
        "terminal-bench-2.1" => experiment_text(staged, &digest),
        "terminal-bench-3.0" => tb30_experiment_text(staged, &digest),
        "deepswe-v1.1" => deepswe_experiment_text(staged, &digest),
        _ => unreachable!(),
    };
    let config = staged.root.join(format!("config-accept-{suffix}.toml"));
    fs::write(&config, rewrite(digested)).unwrap();
    config
}

/// Copies one directory tree byte-for-byte.
fn copy_tree(source: &Path, target: &Path) {
    fs::create_dir_all(target).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let destination = target.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &destination);
        } else {
            fs::copy(entry.path(), destination).unwrap();
        }
    }
}

/// P19-AC-011/P19-AC-013: the offline local-acceptance verifier accepts one
/// genuine three-family native evidence root end to end - six attributable
/// trials, frozen controls, digests, preserved artifacts, the Opi-only
/// scoped capture view byte-bound to the original native captures, cached
/// upstream identities, and the built Companion verify/report invoked only
/// against the supplied family roots - without launching an Agent, touching
/// credentials, or contacting a provider.
#[test]
fn local_acceptance_verifier_accepts_genuine_native_evidence() {
    let staged = stage("ok");
    let proof_config = write_config(&staged);
    stage_output_control(&staged, &proof_config);
    let frozen = opi_eval::experiment::ResolvedExperiment::resolve(
        &fs::read_to_string(&proof_config).unwrap(),
    )
    .unwrap();
    let connection = frozen.selected_connection();
    let evidence = tempfile::tempdir().unwrap();
    let root = evidence.path().join("evidence");
    let families: [(&str, &str); 3] = [
        ("terminal-bench-2.1", "tb21"),
        ("terminal-bench-3.0", "tb30"),
        ("deepswe-v1.1", "swe"),
    ];
    let material: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&staged.material).unwrap()).unwrap();

    let mut trials = Vec::new();
    let mut command_exits = Vec::new();
    let mut preserved = Vec::new();
    for (family, suffix) in families {
        let config = acceptance_family_config(&staged, family, suffix);
        let run_root = root.join("runs").join(family);
        let (code, stdout, stderr) = run_native(&staged, &config, &run_root);
        assert_eq!(code, 0, "{family} run: {stderr} {stdout}");
        let opi_id = format!("trial-opi-{suffix}");
        let pi_id = format!("trial-pi-{suffix}");
        command_exits.push(serde_json::json!({"family": family, "command": "run", "exit": code}));
        // The preserved material copy binds the frozen selection inputs.
        let preserved_material = run_root.join("material.json");
        fs::copy(&staged.material, &preserved_material).unwrap();
        preserved.push(serde_json::json!({
            "role": "material",
            "path": format!("runs/{family}/material.json"),
            "sha256": sha256_hex(&fs::read(&preserved_material).unwrap()),
        }));
        for (trial, agent, subject) in [
            (opi_id.clone(), "opi", "candidate-opi"),
            (pi_id.clone(), "pi", "baseline-pi"),
        ] {
            let bundle_manifest = run_root
                .join("trials")
                .join(&trial)
                .join("bundle/manifest.json");
            trials.push(serde_json::json!({
                "id": trial,
                "agent": agent,
                "family": family,
                "run_root": format!("runs/{family}"),
                "bundle_digest": sha256_hex(&fs::read(&bundle_manifest).unwrap()),
                "attribution": {"agent_executable_sha256": material["agents"][agent]["executable"]["sha256"]},
                "_subject": subject,
            }));
        }
        // The genuine scoped capture view: the Opi trial's preserved native
        // bytes, byte-bound by copy.
        let native = run_root
            .join("trials")
            .join(&opi_id)
            .join("bundle/artifacts/native");
        copy_tree(&native, &root.join("opi-artifact-audit").join(&opi_id));
    }

    // Observed companion exits recorded as preserved evidence.
    for (family, _) in families {
        let run_root = root.join("runs").join(family);
        for command in ["verify", "report"] {
            let mut argv = vec![command.to_owned(), "--root".to_owned()];
            argv.push(run_root.to_string_lossy().into_owned());
            if command == "report" {
                argv.push("--out".to_owned());
                argv.push(
                    evidence
                        .path()
                        .join(format!("observed-report-{family}.json"))
                        .to_string_lossy()
                        .into_owned(),
                );
            }
            let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
                .args(&argv)
                .output()
                .expect("companion invocation");
            assert_eq!(output.status.code(), Some(0), "{family} {command}");
            command_exits
                .push(serde_json::json!({"family": family, "command": command, "exit": 0}));
        }
    }

    // Trial rows carry the manifest shape only; the subject cross-check
    // stays against each family's frozen run-control.json.
    let trials: Vec<serde_json::Value> = trials
        .iter()
        .map(|trial| {
            let mut row = trial.clone();
            row.as_object_mut().unwrap().remove("_subject");
            row
        })
        .collect();
    let manifest = serde_json::json!({
        "schema": "opi-eval-local-acceptance/1",
        "operator": {"redacted": true, "label": "local-acceptance-e2e"},
        "opi_repository": {"url": "https://example.invalid/opi", "commit": "9".repeat(40)},
        "controls": {
            "execution_concurrency": 1,
            "connection_controls": {
                "connection": {"provider": connection.provider, "api": connection.api, "base_url": connection.base_url, "model": connection.model},
                "model_controls": frozen.model_controls(),
            },
        },
        "trials": trials,
        "sources": {
            "pi": {"repository_url": "https://example.invalid/pi", "commit": "a".repeat(40)},
            "harbor": {"repository_url": "https://example.invalid/harbor", "commit": "b".repeat(40)},
            "pier": {"repository_url": "https://example.invalid/pier", "commit": "c".repeat(40)},
        },
        "builds": [
            {"agent": "opi", "executable_sha256": material["agents"]["opi"]["executable"]["sha256"]},
            {"agent": "pi", "executable_sha256": material["agents"]["pi"]["executable"]["sha256"]},
        ],
        "material": [
            {"family": "terminal-bench-2.1", "material_sha256": sha256_hex(&fs::read(&staged.material).unwrap())},
            {"family": "terminal-bench-3.0", "material_sha256": sha256_hex(&fs::read(&staged.material).unwrap())},
            {"family": "deepswe-v1.1", "material_sha256": sha256_hex(&fs::read(&staged.material).unwrap())},
        ],
        "command_exits": command_exits,
        "preserved": preserved,
    });
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    fs::write(
        root.join("upstream-identity-cache.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema": "opi-eval-upstream-identity-cache/1",
            "repositories": {
                "https://example.invalid/pi": {"commits": {"a".repeat(40): {"tree": "1".repeat(40)}}},
                "https://example.invalid/harbor": {"commits": {"b".repeat(40): {"tree": "2".repeat(40)}}},
                "https://example.invalid/pier": {"commits": {"c".repeat(40): {"tree": "3".repeat(40)}}},
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let out = evidence.path().join("acceptance-report.json");
    let output = Command::new("python3")
        .arg(manifest_dir().join("scripts/verify-local-acceptance.py"))
        .arg("--root")
        .arg(&root)
        .arg("--out")
        .arg(&out)
        .arg("--companion")
        .arg(env!("CARGO_BIN_EXE_opi-eval"))
        .output()
        .expect("spawn the offline acceptance verifier");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {stderr} stdout: {stdout}"
    );
    let report: serde_json::Value =
        serde_json::from_str(stdout.trim().lines().last().unwrap()).unwrap();
    assert_eq!(report["schema"], "opi-eval-local-acceptance-verification/1");
    assert_eq!(report["accepted"], true);
    assert_eq!(report["trials"].as_array().unwrap().len(), 6);
    assert!(out.is_file(), "the durable verification report is written");

    // Rejections at the genuine seam: an empty scoped view is not
    // acceptance, and substituted capture bytes are not the original
    // evidence.
    let empty_view = root.join("opi-artifact-audit");
    for child in fs::read_dir(&empty_view).unwrap() {
        let child = child.unwrap().path();
        if child.is_dir() {
            fs::remove_dir_all(&child).unwrap();
        }
    }
    fs::write(
        empty_view.join("generic-audit-summary.json"),
        b"{\"result\": \"success\"}\n",
    )
    .unwrap();
    let out2 = evidence.path().join("acceptance-report-empty.json");
    let output = Command::new("python3")
        .arg(manifest_dir().join("scripts/verify-local-acceptance.py"))
        .arg("--root")
        .arg(&root)
        .arg("--out")
        .arg(&out2)
        .arg("--companion")
        .arg(env!("CARGO_BIN_EXE_opi-eval"))
        .output()
        .expect("spawn the offline acceptance verifier");
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("empty"),
        "an empty scoped view with a generic audit success must be rejected"
    );

    // Restore the genuine capture and tamper with its bytes.
    let capture = root.join("opi-artifact-audit").join("trial-opi-tb21");
    fs::remove_file(empty_view.join("generic-audit-summary.json")).unwrap();
    copy_tree(
        &root.join("runs/terminal-bench-2.1/trials/trial-opi-tb21/bundle/artifacts/native"),
        &capture,
    );
    let tampered = capture.join("agent-stdout.log");
    if tampered.is_file() {
        fs::write(&tampered, b"tampered capture bytes\n").unwrap();
    } else {
        fs::write(capture.join("verifier-input.json"), b"tampered\n").unwrap();
    }
    let out3 = evidence.path().join("acceptance-report-tampered.json");
    let output = Command::new("python3")
        .arg(manifest_dir().join("scripts/verify-local-acceptance.py"))
        .arg("--root")
        .arg(&root)
        .arg("--out")
        .arg(&out3)
        .arg("--companion")
        .arg(env!("CARGO_BIN_EXE_opi-eval"))
        .output()
        .expect("spawn the offline acceptance verifier");
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("byte-bound"),
        "substituted capture bytes must be rejected"
    );
}

/// P19-AC-013: a pi-only experiment runs and reports end to end without any
/// Opi product in the trial set, and the Companion package stays
/// independent - no Opi crate dependency, no bundled task corpora, grader
/// implementations, or images.
#[test]
fn pi_only_without_opi_runtime_and_packaging() {
    // Runtime: every subject is a pi product; the run completes
    // hermetically and the offline report publishes with no Opi trial.
    let staged = tempfile::tempdir().unwrap();
    let source = fs::read_to_string(fixtures().join("experiment/local-paired.toml")).unwrap();
    let config = staged.path().join("pi-only.toml");
    fs::write(
        &config,
        source.replace("product = \"opi\"", "product = \"pi\""),
    )
    .unwrap();
    let run_root = staged.path().join("pi-only-run");
    let run = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("run")
        .arg("--config")
        .arg(&config)
        .arg("--root")
        .arg(&run_root)
        .arg("--fixtures")
        .arg(fixtures())
        .args(["--behavior", "happy"])
        .output()
        .expect("spawn the pi-only run");
    let stdout = String::from_utf8_lossy(&run.stdout);
    let stderr = String::from_utf8_lossy(&run.stderr);
    // Both trials run, seal, and grade through the pi path; the pairing
    // layer refuses to fabricate a comparability verdict for two
    // identical protocols, so the run settles non-success with the
    // mismatch explicit rather than inventing an Opi baseline.
    assert_eq!(
        run.status.code(),
        Some(1),
        "stderr: {stderr} stdout: {stdout}"
    );
    let report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(report["outcome"], "incomplete", "{report}");
    assert_eq!(
        report["pairs"][0]["comparability"], "protocol-group-mismatch",
        "{report}"
    );
    for trial in report["trials"].as_array().unwrap() {
        assert_eq!(trial["status"], "sealed", "{trial}");
        assert_eq!(trial["agent"]["product"], "pi", "{trial}");
    }
    let offline = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("report")
        .arg("--root")
        .arg(&run_root)
        .arg("--out")
        .arg(staged.path().join("offline.json"))
        .output()
        .expect("spawn the offline report command");
    assert_eq!(offline.status.code(), Some(0));
    let offline_report: serde_json::Value = serde_json::from_slice(&offline.stdout).unwrap();
    assert_eq!(offline_report["outcome"], "published", "{offline_report}");

    // Packaging: the Companion manifest declares no Opi crate dependency
    // and the crate bundles no task corpora, grader implementations, or
    // images (the package listing gate covers the shipped file set).
    let manifest = fs::read_to_string(manifest_dir().join("Cargo.toml")).unwrap();
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("opi-") && trimmed.contains('=') {
            let name = trimmed.split('=').next().unwrap_or_default().trim();
            panic!("the Companion must not depend on Opi crates: {name}");
        }
    }
    for corpus in ["corpora", "graders", "images", "datasets"] {
        assert!(
            !manifest_dir().join(corpus).exists(),
            "the Companion must not bundle {corpus}"
        );
    }
}

#[test]
fn remediation_005_native_stage_deadlines_reach_runner() {
    for family in ["tb21", "tb30", "deepswe"] {
        for (bounds, agent_cap, verifier_cap) in [
            ("time_limit_seconds = 1", Some(1), None),
            ("agent_timeout_seconds = 1", Some(1), None),
            ("verifier_timeout_seconds = 1", None, Some(1)),
            (
                "time_limit_seconds = 3\nagent_timeout_seconds = 1\nverifier_timeout_seconds = 2",
                Some(1),
                Some(2),
            ),
            (
                "time_limit_seconds = 1\nagent_timeout_seconds = 3",
                Some(1),
                None,
            ),
        ] {
            let staged = stage("ok");
            remediation_runner(&staged, |body| {
                body.replacen(
                    "set -eu",
                    "set -eu\nprintf '%s\\n' \"$@\" > native-dispatch-argv.txt",
                    1,
                )
                .replace(
                    "path.write_text(json.dumps(trial))",
                    "trial.pop('timeout_multiplier', None)\npath.write_text(json.dumps(trial))",
                )
            });
            let config = match family {
                "tb21" => write_config(&staged),
                "tb30" => write_tb30_config(&staged),
                _ => write_deepswe_config(&staged),
            };
            let text = fs::read_to_string(&config)
                .unwrap()
                .replace("[execution]", &format!("[execution]\n{bounds}"));
            fs::write(&config, text).unwrap();
            let root = staged.root.join("run");
            let (code, stdout, stderr) = run_native(&staged, &config, &root);
            assert_eq!(code, 0, "{family} {bounds}: {stdout} {stderr}");
            let invocations = walk_files_matching(&root, "native-dispatch-argv.txt");
            assert_eq!(invocations.len(), 2, "inspect both actual runner spawns");
            for path in invocations {
                let args = fs::read_to_string(path).unwrap();
                let args: Vec<_> = args.lines().collect();
                assert_eq!(args.len(), 6);
                assert_eq!(args[4], "--config");
                assert!(Path::new(args[5]).is_absolute());
                let job: serde_json::Value =
                    serde_json::from_slice(&fs::read(args[5]).unwrap()).unwrap();
                assert_eq!(job["agents"][0]["max_timeout_sec"].as_u64(), agent_cap);
                assert_eq!(job["verifier"]["max_timeout_sec"].as_u64(), verifier_cap);
                assert!(job["verifier"]["override_timeout_sec"].is_null());
                let task: toml::Value = toml::from_str(
                    &fs::read_to_string(
                        Path::new(job["tasks"][0]["path"].as_str().unwrap()).join("task.toml"),
                    )
                    .unwrap(),
                )
                .unwrap();
                let has_default = task
                    .get("agent")
                    .and_then(|agent| agent.get("timeout_sec"))
                    .is_some();
                assert_eq!(
                    job["agents"][0]["override_timeout_sec"].as_u64(),
                    if has_default { None } else { agent_cap }
                );
                for key in [
                    "timeout_multiplier",
                    "agent_timeout_multiplier",
                    "verifier_timeout_multiplier",
                ] {
                    assert_eq!(job[key].as_f64(), Some(1.0));
                }
                assert_eq!(job["agents"][0]["model_name"], "scripted:eval");
                assert_eq!(job["n_attempts"], 1);
                assert_eq!(job["n_concurrent_trials"], 1);
                assert_eq!(job["retry"]["max_retries"], 0);
            }
        }
    }
}

/// Exercises the unchanged pinned runners and real Docker stages with
/// independently byte-bound, no-model delay tasks. The supplied material
/// names the controlled cases below; it is not benchmark acceptance evidence.
#[test]
#[ignore = "requires verified no-model delay material/5 and the real Docker engine"]
fn remediation_native_stage_deadlines() {
    let material = PathBuf::from(
        std::env::var("OPI_EVAL_NATIVE_WITNESS_MATERIAL")
            .expect("OPI_EVAL_NATIVE_WITNESS_MATERIAL is required"),
    );
    let input = fs::read(&material).unwrap();
    let manifest: serde_json::Value = serde_json::from_slice(&input).unwrap();
    assert_eq!(manifest["schema"], "opi-eval-native-material/5");
    let static_lock: serde_json::Value = serde_json::from_slice(
        &fs::read(manifest["static_lock"]["path"].as_str().unwrap()).unwrap(),
    )
    .unwrap();
    let expected_lock: serde_json::Value = serde_json::from_slice(
        &fs::read(manifest_dir().join("external-locks/static/linux-x86_64.json")).unwrap(),
    )
    .unwrap();
    for family in ["harbor", "pier"] {
        let tool = |lock: &serde_json::Value| {
            lock["tools"]
                .as_array()
                .unwrap()
                .iter()
                .find(|entry| entry["id"] == family)
                .unwrap()
                .clone()
        };
        assert_eq!(tool(&static_lock), tool(&expected_lock));
    }
    let guard = tempfile::tempdir().unwrap();
    let evidence = guard.keep();
    eprintln!("native deadline evidence: {}", evidence.display());
    // The neighboring container and shared immutable image must survive every
    // trial's Agent and separate-verifier cleanup, including deadline paths.
    let image = "python@sha256:272efc3f94bec901e2bc7a5c9984b3d113c317839edca2714c6a388b9476f8fc";
    let inspect_image = || {
        Command::new("/usr/bin/docker")
            .args(["image", "inspect", "--format", "{{.Id}}", image])
            .output()
            .unwrap()
    };
    let image_before = inspect_image();
    assert!(image_before.status.success());
    let neighbor = Command::new("/usr/bin/docker")
        .args([
            "run",
            "--detach",
            "--pull",
            "never",
            "--network",
            "none",
            "--label",
        ])
        .arg(format!("opi-eval-stage-neighbor={}", evidence.display()))
        .args([image, "sleep", "3600"])
        .output()
        .unwrap();
    assert!(
        neighbor.status.success(),
        "{}",
        String::from_utf8_lossy(&neighbor.stderr)
    );
    struct Neighbor(String);
    impl Drop for Neighbor {
        fn drop(&mut self) {
            let _ = Command::new("/usr/bin/docker")
                .args(["rm", "--force", &self.0])
                .output();
        }
    }
    let neighbor_id = String::from_utf8(neighbor.stdout)
        .unwrap()
        .trim()
        .to_owned();
    assert!(neighbor_id.len() == 64 && neighbor_id.bytes().all(|byte| byte.is_ascii_hexdigit()));
    let neighbor = Neighbor(neighbor_id);
    for family in ["terminal-bench-2.1", "terminal-bench-3.0", "deepswe-v1.1"] {
        for (case, bounds, exception) in [
            (
                "positive",
                "agent_timeout_seconds = 3\nverifier_timeout_seconds = 3",
                None,
            ),
            (
                "time-cap",
                "time_limit_seconds = 1",
                Some("AgentTimeoutError"),
            ),
            (
                "agent-cap",
                "agent_timeout_seconds = 1",
                Some("AgentTimeoutError"),
            ),
            (
                "verifier-cap",
                "verifier_timeout_seconds = 1",
                Some("VerifierTimeoutError"),
            ),
            (
                "both-agent",
                "agent_timeout_seconds = 1\nverifier_timeout_seconds = 1",
                Some("AgentTimeoutError"),
            ),
            (
                "both-verifier",
                "agent_timeout_seconds = 1\nverifier_timeout_seconds = 1",
                Some("VerifierTimeoutError"),
            ),
            (
                "minimum-cap",
                "time_limit_seconds = 1\nagent_timeout_seconds = 3",
                Some("AgentTimeoutError"),
            ),
            (
                "strict-task",
                "agent_timeout_seconds = 3",
                Some("AgentTimeoutError"),
            ),
            (
                "unbounded-task",
                "agent_timeout_seconds = 1",
                Some("AgentTimeoutError"),
            ),
        ] {
            let root = evidence.join(family).join(case);
            fs::create_dir_all(&root).unwrap();
            let staged = Staged {
                _guard: tempfile::tempdir().unwrap(),
                root: root.clone(),
                material: material.clone(),
            };
            let mut config = experiment_text(&staged, "pending")
                .replace("fixture-task", case)
                .replace(
                    "max_output_tokens = 4096",
                    "max_output_tokens = \"omitted\"",
                )
                .replace("context_window = 8192", "context_window = \"omitted\"")
                .replace("[execution]", &format!("[execution]\n{bounds}"));
            if family == "terminal-bench-3.0" {
                config = config.replace("revision = \"2.1\"", "revision = \"3.0\"");
            } else if family == "deepswe-v1.1" {
                config = config
                    .replace("name = \"terminal-bench\"", "name = \"deepswe\"")
                    .replace("revision = \"2.1\"", "revision = \"v1.1\"");
            }
            let path = root.join("experiment.toml");
            fs::write(&path, &config).unwrap();
            let validation = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
                .args(["validate", "--config"])
                .arg(&path)
                .arg("--native-material")
                .arg(&material)
                .env("PATH", "/usr/local/bin:/usr/bin:/bin")
                .output()
                .unwrap();
            assert!(
                validation.status.success(),
                "{family}/{case}: {}",
                String::from_utf8_lossy(&validation.stderr)
            );
            let validation = String::from_utf8(validation.stdout).unwrap();
            let digest = validation
                .split_whitespace()
                .find_map(|word| word.strip_prefix("native_integrity="))
                .unwrap();
            fs::write(&path, config.replace("pending", digest)).unwrap();
            let run = root.join("run");
            let started = std::time::Instant::now();
            let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
                .args(["run", "--config"])
                .arg(&path)
                .arg("--root")
                .arg(&run)
                .arg("--native-material")
                .arg(&material)
                .env_clear()
                .env("PATH", "/usr/local/bin:/usr/bin:/bin")
                .env("HOME", &root)
                .env("OPI_EVAL_SCRIPTED_KEY", "no-model-witness")
                .output()
                .unwrap();
            eprintln!(
                "{family}/{case}: exit {:?}, elapsed {:?}",
                output.status.code(),
                started.elapsed()
            );
            assert!(
                started.elapsed() < std::time::Duration::from_secs(240),
                "stage witness exceeded two whole-job bounds"
            );
            let publication = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
                .args(["report", "--root"])
                .arg(&run)
                .output()
                .unwrap();
            assert!(
                publication.status.success(),
                "{family}/{case}: {} {} {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
                String::from_utf8_lossy(&publication.stderr)
            );
            let report: serde_json::Value = serde_json::from_slice(&publication.stdout).unwrap();
            let verifier_timeout = exception == Some("VerifierTimeoutError");
            assert_eq!(
                report["coverage_counts"]["valid"],
                if verifier_timeout { 0 } else { 2 },
                "{family}/{case}: {report}"
            );
            assert_eq!(
                report["coverage_counts"]["invalid"],
                if verifier_timeout { 2 } else { 0 },
                "{family}/{case}: {report}"
            );
            for trial in report["trials"].as_array().unwrap() {
                if verifier_timeout {
                    assert_eq!(trial["outcome"]["reason"], "verifier-timeout", "{trial}");
                }
            }
            let results = walk_files_matching(&run, "result.json")
                .into_iter()
                .filter(|path| path.parent().unwrap().join("agent").is_dir())
                .collect::<Vec<_>>();
            assert_eq!(results.len(), 2, "{family}/{case}: {results:?}");
            for result in results {
                let native: serde_json::Value =
                    serde_json::from_slice(&fs::read(&result).unwrap()).unwrap();
                assert_eq!(
                    native["exception_info"]["exception_type"].as_str(),
                    exception,
                    "{family}/{case}: {native}"
                );
                let directory = result.parent().unwrap();
                assert!(
                    fs::read_to_string(directory.join("agent/stderr.log"))
                        .unwrap()
                        .contains("stage-witness-start"),
                    "Agent never started"
                );
                let observations =
                    fs::read_to_string(directory.join("opi-eval-commands.jsonl")).unwrap();
                let commands: Vec<serde_json::Value> = observations
                    .lines()
                    .map(|line| serde_json::from_str(line).unwrap())
                    .collect();
                assert!(
                    commands
                        .iter()
                        .all(|row| row["exit_code"].as_i64().is_some()),
                    "{family}/{case}: a command exit was not observed: {commands:?}"
                );
                if exception.is_some() {
                    let stopped: Vec<_> = commands
                        .iter()
                        .filter(|row| row["exit_code"] == 137)
                        .collect();
                    assert_eq!(stopped.len(), 1, "{family}/{case}: {commands:?}");
                    let seconds = stopped[0]["elapsed_seconds"].as_f64().unwrap();
                    // Measure only the exec, excluding native environment startup and
                    // teardown. The strict task's 0.5s cap must beat the 3s override.
                    let upper = if case == "strict-task" { 1.5 } else { 2.5 };
                    assert!(seconds < upper, "{family}/{case}: command took {seconds}s");
                }
                if verifier_timeout {
                    assert!(
                        directory.join("verifier/stage-start").exists(),
                        "verifier never started"
                    );
                    assert!(!result.parent().unwrap().join("verifier/stage-end").exists());
                } else if exception == Some("AgentTimeoutError") {
                    assert!(
                        !result
                            .parent()
                            .unwrap()
                            .join("artifacts/app/evalbench/completed")
                            .exists(),
                        "timed-out Agent reached delayed completion"
                    );
                }
            }
            for trial in ["trial-opi-native", "trial-pi-native"] {
                for component in ["environment", "tests"] {
                    let owned = run
                        .join("trials")
                        .join(trial)
                        .join("task-package")
                        .join(component);
                    let listed = Command::new("/usr/bin/docker")
                        .args(["ps", "-aq", "--filter"])
                        .arg(format!(
                            "label=com.docker.compose.project.working_dir={}",
                            owned.display()
                        ))
                        .output()
                        .unwrap();
                    assert!(listed.status.success());
                    assert!(
                        listed.stdout.is_empty(),
                        "owned containers remain after {family}/{case}"
                    );
                }
            }
            assert_eq!(fs::read(&material).unwrap(), input);
            let image_after = inspect_image();
            assert!(
                image_after.status.success(),
                "shared image removed after {family}/{case}"
            );
            assert_eq!(image_after.stdout, image_before.stdout);
            let alive = Command::new("/usr/bin/docker")
                .args(["inspect", "--format", "{{.State.Running}}", &neighbor.0])
                .output()
                .unwrap();
            assert!(alive.status.success());
            assert_eq!(String::from_utf8(alive.stdout).unwrap().trim(), "true");
        }
    }
}

#[test]
fn native_stage_configuration_drift_rejects_scores() {
    for mutation in [
        "trial['agent']['max_timeout_sec'] = 1000",
        "trial['agent']['override_timeout_sec'] = 1000",
        "trial['verifier']['max_timeout_sec'] = 1000",
        "trial['verifier']['override_timeout_sec'] = 1000",
        "trial['timeout_multiplier'] = 1000",
        "trial['timeout_multiplier'] = None",
        "trial['agent_timeout_multiplier'] = 1000",
        "trial['verifier_timeout_multiplier'] = 1000",
        "trial['agent']['max_timeout_sec'] = None",
        "trial['verifier']['override_timeout_sec'] = 'invalid'",
        "trial['environment']['import_path'] = None",
        "trial['environment']['import_path'] = 'harbor.environments.docker.docker:DockerEnvironment'",
    ] {
        let staged = stage("ok");
        remediation_runner(&staged, |body| {
            body.replace(
                "path.write_text(json.dumps(trial))",
                &format!("{mutation}\npath.write_text(json.dumps(trial))"),
            )
        });
        let config = write_config(&staged);
        let text = fs::read_to_string(&config).unwrap().replace(
            "[execution]",
            "[execution]\nagent_timeout_seconds = 1\nverifier_timeout_seconds = 1",
        );
        fs::write(&config, text).unwrap();
        let root = staged.root.join("run");
        let (code, stdout, stderr) = run_native(&staged, &config, &root);
        assert_eq!(code, 1, "{mutation}: {stdout} {stderr}");
        let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        for trial in report["trials"].as_array().unwrap() {
            assert_eq!(trial["classification"]["class"], "invalid-execution");
            assert_eq!(
                trial["classification"]["reason"],
                "native-stage-configuration-drift"
            );
        }
    }
}

#[test]
fn native_unobserved_command_exit_stops_dispatch_and_sealing() {
    let staged = stage("ok");
    remediation_runner(&staged, |body| {
        body.replace(
        "path.write_text(json.dumps(trial))",
        "path.write_text(json.dumps(trial))\n(path.parent / 'opi-eval-effect-unknown').write_text('command exit unobserved')",
    )
    });
    let config = write_config(&staged);
    let root = staged.root.join("run");
    let (code, stdout, stderr) = run_native(&staged, &config, &root);
    assert_eq!(code, 1, "{stdout} {stderr}");
    assert_eq!(
        walk_files_matching(&root, "opi-eval-effect-unknown").len(),
        1,
        "uncertain termination must stop the second dispatch"
    );
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(
        report["trials"]
            .as_array()
            .unwrap()
            .iter()
            .any(|trial| trial["classification"]["reason"]
                == "native-command-termination-unconfirmed")
    );
    assert!(
        walk_files_matching(&root, "manifest.json")
            .iter()
            .all(|path| !path.parent().unwrap().ends_with("bundle")),
        "uncertain execution cannot seal"
    );
}

#[test]
fn remediation_009_rejected_rerun_preserves_control() {
    let staged = stage("ok");
    let config = write_config(&staged);
    let original = fs::read_to_string(&config).unwrap();
    let (root, _) = regrade_family_root(&staged, &config, "run");
    let before = directory_manifest_digest(&root);
    let mut offline = Vec::new();
    for command in ["verify", "report"] {
        let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
            .args([command, "--root"])
            .arg(&root)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{command}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        offline.push((command, output.stdout));
    }
    for source in [
        original.replace(
            "experiment_id = \"native-driver-test\"",
            "experiment_id = \"changed-experiment\"",
        ),
        original,
    ] {
        fs::write(&config, source).unwrap();
        let (code, _, stderr) = run_native(&staged, &config, &root);
        assert_eq!(code, 2, "reused root must reject: {stderr}");
        assert_eq!(
            directory_manifest_digest(&root),
            before,
            "rejected run changed existing evidence bytes"
        );
        for (command, expected) in &offline {
            let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
                .args([command, "--root"])
                .arg(&root)
                .output()
                .unwrap();
            assert!(output.status.success());
            assert_eq!(&output.stdout, expected);
        }
    }
}

#[test]
fn native_simultaneous_run_root_claim_has_one_owner() {
    let staged = stage("ok");
    let config = write_config(&staged);
    let root = staged.root.join("run");
    let barrier = std::sync::Barrier::new(2);
    let results = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..2)
            .map(|_| {
                scope.spawn(|| {
                    barrier.wait();
                    run_native(&staged, &config, &root)
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>()
    });
    let mut codes: Vec<_> = results.iter().map(|result| result.0).collect();
    codes.sort();
    assert_eq!(codes, [0, 2], "{results:?}");
    let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .args(["verify", "--root"])
        .arg(&root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
