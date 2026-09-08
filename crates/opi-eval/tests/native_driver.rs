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
/// - `agent-nonzero` agent exited 3 (NonZeroAgentExitCodeError), reward 0
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
trial_dir = os.path.join(job, "trials", identity["trial_name"])
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
        manifest.append({"source": entry["source"], "destination": destination, "type": "directory", "status": "ok"})
    else:
        shutil.copyfile(source_abs, target)
        manifest.append({"source": entry["source"], "destination": destination, "type": "file", "status": "ok"})
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
    "task": {"verifier": {"environment_mode": identity["environment_mode"]}},
}
if identity["runner"] == "pier":
    config["artifacts"] = []
    config["task"]["artifacts"] = identity["declared"]
    config["task"]["verifier"]["network_mode"] = identity["verifier_network"]
    config["task"]["metadata"] = {"base_commit_hash": identity["base_revision"]}
    config["task"]["environment"] = {"docker_image": identity["docker_image"]}
else:
    config["artifacts"] = identity["declared"]
with open(os.path.join(trial_dir, "config.json"), "w") as handle:
    json.dump(config, handle)
results = {
    "task_name": identity["task_name"],
    "trial_name": identity["trial_name"],
    "agent_info": {"name": identity["agent_name"], "version": "1.0.0", "model_info": {"name": "scripted/eval"}},
    "verifier_result": {"rewards": {"reward": reward}},
    "exception_info": None,
}
with open(os.path.join(trial_dir, "results.json"), "w") as handle:
    json.dump(results, handle)
with open(os.path.join(trial_dir, "artifacts", "manifest.json"), "w") as handle:
    json.dump(manifest, handle)
REGRADE
  exit $?
fi
[ "$1" = "run" ] && [ "$2" = "--locked" ] && { [ "$3" = "harbor" ] || [ "$3" = "pier" ]; } \
  && [ "$4" = "run" ] && [ "$5" = "-p" ] \
  || { echo "argv drift: expected run --locked harbor|pier run -p" >&2; exit 9; }
runner="$3"
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
  cat > "$job/trials/$trial/results.json" <<JSON
{"task_name": "$task_name", "trial_name": "$trial", "agent_info": {"name": "$agent_name", "version": "1.0.0", "model_info": {"name": "scripted/eval"}}, "verifier_result": {"rewards": {"reward": $reward}}, "exception_info": $exception}
JSON
  if [ -n "$pier_profile" ]; then
    # The Pier job records the task's own declarations and identity: the
    # separate no-network verifier, the model.patch collect declaration,
    # and the base revision and image the pristine verifier rebuilds
    # from. Pier groups a modeled trial under agent__model__adhoc.
    eval_key="${agent_name}__scripted:eval__adhoc"
    [ -n "$model" ] || eval_key="${agent_name}__adhoc"
    base="1111111111111111111111111111111111111112"
    if [ "$OPI_EVAL_STANDIN_MODE" = "deepswe-base-drift" ]; then
      base="9999999999999999999999999999999999999999"
    fi
    sed -i.bak "s/\"${agent_name}__adhoc\"/\"$eval_key\"/" "$job/result.json" && rm -f "$job/result.json.bak"
    printf '{"agent": {"name": "%s", "import_path": null}, "artifacts": [], "task": {"verifier": {"environment_mode": "separate", "network_mode": "no-network"}, "artifacts": ["/logs/artifacts/model.patch"], "metadata": {"base_commit_hash": "%s"}, "environment": {"docker_image": "fixture-registry/swe-bench-fixture:synthetic"}}}\n' "$agent" "$base" \
      > "$job/trials/$trial/config.json"
  elif [ -n "$tb30_profile" ]; then
    # The TB3 job records the task's declared separate lifecycle and its
    # /app/evalbench/ collect declaration exactly as the pinned runner
    # would.
    printf '{"agent": {"name": "%s", "import_path": null}, "artifacts": ["/app/evalbench/"], "task": {"verifier": {"environment_mode": "separate"}}}\n' "$agent" \
      > "$job/trials/$trial/config.json"
  else
    printf '{"agent": {"name": "%s", "import_path": null}, "artifacts": ["/root/answer.txt"], "task": {"verifier": {"environment_mode": "shared"}}}\n' "$agent" \
      > "$job/trials/$trial/config.json"
  fi
}

if [ "$runner" = "pier" ]; then
  # Every pier dispatch - the deepswe run modes and the explicitly
  # selected oracle/nop witnesses - writes the pier job tree.
  pier_profile=1
fi
if [ "$OPI_EVAL_STANDIN_MODE" = "two-jobs" ]; then
  for job in jobs/2026-09-05__11-00-00 "$job"; do
    mkdir -p "$job/trials/$trial/agent" "$job/trials/$trial/artifacts"
    write_job "$job"
  done
  exit 0
fi
mkdir -p "$job/trials/$trial/agent" "$job/trials/$trial/artifacts"
write_job "$job"
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
    "$product" "$(printf 'a%.0s' $(seq 1 64))" "$schema_sha" "$canary_sha" \
    > "$job/trials/$trial/agent/opi-eval-terminal.json"
  mkdir -p "$job/trials/$trial/artifacts/app/evalbench"
  printf '%s' "$schema" > "$job/trials/$trial/artifacts/app/evalbench/schema.py"
  manifest='[{"source": "/app/evalbench/", "destination": "app/evalbench", "type": "directory", "status": "ok", "service": null}]'
  case "$OPI_EVAL_STANDIN_MODE" in
    tb30-undeclared)
      # An undeclared file crossed the collection boundary: the binding
      # must reject it.
      manifest='[{"source": "/app/evalbench/", "destination": "app/evalbench", "type": "directory", "status": "ok", "service": null}, {"source": "/app/unrelated-canary.txt", "destination": "app/unrelated-canary.txt", "type": "file", "status": "ok", "service": null}]'
      ;;
    tb30-substituted-declared)
      # The collected declared artifact no longer carries the delivered
      # bytes.
      printf 'substituted schema bytes' > "$job/trials/$trial/artifacts/app/evalbench/schema.py"
      ;;
  esac
  printf '%s\n' "$manifest" > "$job/trials/$trial/artifacts/manifest.json"
  exit 0
fi
if [ -n "$pier_profile" ]; then
  # The DeepSWE agent committed a patch; the task's collect hook
  # extracted it and only it crossed into the pristine verifier.
  if [ -z "$diagnostic" ]; then
    printf '{"schema": "opi-eval-native-agent-terminal/1", "product": "%s", "agent_executable_sha256": "%s", "exit_code": 0, "timed_out": false, "delivered": []}\n' \
      "$product" "$(printf 'a%.0s' $(seq 1 64))" \
      > "$job/trials/$trial/agent/opi-eval-terminal.json"
  fi
  mkdir -p "$job/trials/$trial/artifacts/logs/artifacts"
  patch="fixture committed patch bytes"
  if [ "$OPI_EVAL_STANDIN_MODE" = "deepswe-nop" ] \
    || [ "$OPI_EVAL_STANDIN_MODE" = "nop-delivers-nothing" ]; then
    # The nop agent committed nothing: the hook extracted an empty
    # patch, which still collects (status ok) and grades zero.
    patch=""
  fi
  printf '%s' "$patch" > "$job/trials/$trial/artifacts/logs/artifacts/model.patch"
  manifest='[{"source": "/logs/artifacts/model.patch", "destination": "logs/artifacts/model.patch", "type": "file", "status": "ok"}]'
  if [ "$OPI_EVAL_STANDIN_MODE" = "deepswe-undeclared" ]; then
    # An undeclared file crossed the collection boundary into the
    # verifier: the binding must reject it.
    manifest='[{"source": "/logs/artifacts/model.patch", "destination": "logs/artifacts/model.patch", "type": "file", "status": "ok"}, {"source": "/app/unrelated-canary.txt", "destination": "app/unrelated-canary.txt", "type": "file", "status": "ok"}]'
  fi
  printf '%s\n' "$manifest" > "$job/trials/$trial/artifacts/manifest.json"
  exit 0
fi
if [ "$OPI_EVAL_STANDIN_MODE" != "no-terminal" ]; then
  printf '{"schema": "opi-eval-native-agent-terminal/1", "product": "%s", "agent_executable_sha256": "%s", "exit_code": %s, "timed_out": %s, "delivered": [{"source": "/root/answer.txt", "sha256": "%s"}]}\n' \
    "$product" "$(printf 'a%.0s' $(seq 1 64))" \
    "$( [ "$OPI_EVAL_STANDIN_MODE" = "agent-nonzero" ] && echo 3 || echo 0 )" \
    "$( [ "$OPI_EVAL_STANDIN_MODE" = "agent-timeout" ] && echo true || echo false )" \
    "$answer_sha" > "$job/trials/$trial/agent/opi-eval-terminal.json"
fi
if [ "$OPI_EVAL_STANDIN_MODE" = "substituted" ]; then
  answer="substituted answer"
fi
printf '[{"source": "/root/answer.txt", "destination": "root/answer.txt", "type": "file", "status": "ok", "service": null}]\n' \
  > "$job/trials/$trial/artifacts/manifest.json"
mkdir -p "$job/trials/$trial/artifacts/root"
printf '%s\n' "$answer" > "$job/trials/$trial/artifacts/root/answer.txt"
exit 0
"#
    .to_owned()
}

struct Staged {
    /// Owns the staging directory so it outlives every test assertion.
    _guard: tempfile::TempDir,
    root: PathBuf,
    material: PathBuf,
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

    let digest_of = |path: &Path| sha256_hex(&fs::read(path).unwrap());
    let material = mat.join("material.json");
    let integration_entry = |dir: &Path, import: &str, name: &str| {
        format!(
            r#"{{"module_dir": {:?}, "module_manifest_sha256": {:?},
                        "import_path": "{}", "agent_name": "{}"}}"#,
            dir.to_string_lossy(),
            directory_manifest_digest(dir),
            import,
            name,
        )
    };
    let integration_block = |import: &str, name: &str| {
        format!(
            r#""integrations": {{
        "harbor": {},
        "pier": {}
      }}"#,
            integration_entry(&integrations, import, name),
            integration_entry(&pier_integrations, &import.replace("harbor", "pier"), name),
        )
    };
    fs::write(
        &material,
        format!(
            r#"{{
  "schema": "opi-eval-native-material/3",
  "static_lock": {{"path": {:?}, "sha256": {:?}}},
  "provider": {{
    "script": {{"path": {:?}, "sha256": {:?}}},
    "endpoint": "http://127.0.0.1:48127/v1",
    "request_log": {:?}
  }},
  "agents": {{
    "opi": {{
      "executable": {{"path": {:?}, "sha256": {:?}}},
      {}
    }},
    "pi": {{
      "executable": {{"path": {:?}, "sha256": {:?}}},
      {}
    }}
  }},
  "benchmarks": {{
    "terminal-bench-2.1": {{
      "profile": {:?},
      "task_package": {:?},
      "task_package_manifest_sha256": {:?},
      "verifier_executable": {{"path": {:?}, "sha256": {:?}}},
      "verifier_env": {{"OPI_EVAL_STANDIN_MODE": "{}"}},
      "oracle": {{"path": {:?}, "sha256": {:?}}},
      "oracle_env": {{}}
    }},
    "terminal-bench-3.0": {{
      "profile": {:?},
      "task_package": {:?},
      "task_package_manifest_sha256": {:?},
      "verifier_executable": {{"path": {:?}, "sha256": {:?}}},
      "verifier_env": {{"OPI_EVAL_STANDIN_MODE": "tb30-ok"}},
      "oracle": {{"path": {:?}, "sha256": {:?}}},
      "oracle_env": {{}}
    }},
    "deepswe-v1.1": {{
      "profile": {:?},
      "task_package": {:?},
      "task_package_manifest_sha256": {:?},
      "verifier_executable": {{"path": {:?}, "sha256": {:?}}},
      "verifier_env": {{"OPI_EVAL_STANDIN_MODE": "deepswe-ok"}},
      "oracle": {{"path": {:?}, "sha256": {:?}}},
      "oracle_env": {{}}
    }}
  }}
}}"#,
            static_lock.to_string_lossy(),
            digest_of(&static_lock),
            provider.to_string_lossy(),
            digest_of(&provider),
            mat.join("requests.jsonl").to_string_lossy(),
            mat.join("agents/opi").to_string_lossy(),
            digest_of(&mat.join("agents/opi")),
            integration_block("opi_eval_harbor:OpiAgent", "opi-eval-opi"),
            mat.join("agents/pi").to_string_lossy(),
            digest_of(&mat.join("agents/pi")),
            integration_block("opi_eval_harbor:PiAgent", "opi-eval-pi"),
            profile.to_string_lossy(),
            task.to_string_lossy(),
            directory_manifest_digest(&task),
            verifier.to_string_lossy(),
            digest_of(&verifier),
            mode,
            verifier.to_string_lossy(),
            digest_of(&verifier),
            tb30_profile.to_string_lossy(),
            tb30_task.to_string_lossy(),
            directory_manifest_digest(&tb30_task),
            verifier.to_string_lossy(),
            digest_of(&verifier),
            verifier.to_string_lossy(),
            digest_of(&verifier),
            deepswe_profile.to_string_lossy(),
            deepswe_task.to_string_lossy(),
            directory_manifest_digest(&deepswe_task),
            verifier.to_string_lossy(),
            digest_of(&verifier),
            verifier.to_string_lossy(),
            digest_of(&verifier),
        ),
    )
    .unwrap();

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
    config
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
        .output()
        .expect("run executes");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
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

    // The integration handoff (with credential material) stays outside the
    // sealed bundle.
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
                .join("integration/handoff.json")
                .is_file(),
            "{trial} staged the handoff"
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
            &fs::read(job.join(
                "trials/synthetic-fixture-task__standin/artifacts/logs/artifacts/model.patch",
            ))
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
            "fixture-registry/swe-bench-fixture:synthetic"
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
    Command::new(bin)
        .arg("--config")
        .arg(config)
        .arg("--model")
        .arg(format!("scripted:{model}"))
        .arg("--json")
        .arg("--json-compact")
        .arg("--allow-mutating")
        .arg("--no-trust")
        .arg("--trace")
        .arg(case_root.join("trace"))
        .arg("-C")
        .arg(work)
        .arg("Use the bash tool to run the command, then finish.")
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
    let agent_dir = case_root.join("pi-agent");
    std::fs::create_dir_all(&agent_dir).unwrap();
    std::fs::copy(config, agent_dir.join("models.json")).unwrap();
    Command::new(node)
        .arg(bundle)
        .arg("--print")
        .arg("--mode")
        .arg("json")
        .arg("--no-session")
        .arg("--no-approve")
        .arg("--no-extensions")
        .arg("--no-skills")
        .arg("--no-context-files")
        .arg("--provider")
        .arg("scripted")
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
    copy_dir(
        &fixtures().join("benchmarks/terminal-bench-2.1/task-package"),
        &cache.join("packages/terminal-bench-2.1"),
    );
    write_exec(
        &cache.join("tools/uv"),
        "#!/bin/sh\n# resolved uv stand-in (test seeding)\nexit 0\n",
    );
    write_exec(
        &cache.join("wrappers/terminal-bench-2.1-oracle.sh"),
        "#!/bin/sh\n# oracle wrapper stand-in (test seeding)\nexit 0\n",
    );
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
    assert_eq!(material["schema"], "opi-eval-native-material/3");
    assert_eq!(material["origin"], "local");
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
    assert!(inventory["entries"]["packages:terminal-bench-2.1"]["sha256"].is_string());

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

fn run_prepare(config: &Path, out: &Path) -> std::process::Output {
    run_prepare_env(config, out, &[])
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
    let task_toml = cache.join("packages/terminal-bench-2.1/task.toml");
    fs::write(
        &task_toml,
        "artifacts = [\"/root/answer.txt\"]\n\n[task]\nid = \"openssl-selfsigned-cert\"\n\n[environment]\ncpus = 1\nmemory_mb = 256\ngpus = 1\nos = \"linux\"\n",
    )
    .unwrap();
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
        ok_cache.join("packages/terminal-bench-2.1/task.toml"),
        "artifacts = [\"/root/answer.txt\"]\n\n[task]\nid = \"openssl-selfsigned-cert\"\n\n[environment]\ncpus = 1\nmemory_mb = 256\ngpus = 0\nos = \"linux\"\n",
    )
    .unwrap();
    let output = run_prepare(&ok_config, &ok_out);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(0), "stderr: {stderr}");
    let report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    let preflight = &report["preflight"]["tasks"][0];
    assert_eq!(preflight["id"], "openssl-selfsigned-cert");
    assert_eq!(preflight["status"], "ok");
    assert!(preflight["requirements"]["gpus"].is_u64());
}

// --- Task 19.6: plan freeze, full-inventory preflight, selection (P19-D05) ---

/// Builds a three-task corpus package root (one profile-bound task, one
/// feasible extra task, one GPU-bound task) and repoints the staged
/// material's terminal-bench-2.1 benchmark at it.
fn stage_corpus(staged: &Staged) -> std::path::PathBuf {
    let corpus = staged.root.join("corpus-package");
    let bound = corpus.join("fixture-task");
    copy_dir(&staged.root.join("material/task-package"), &bound);
    for (id, gpus) in [("alpha-notes", None), ("gpu-train", Some(1))] {
        let dir = corpus.join(id);
        fs::create_dir_all(dir.join("environment")).unwrap();
        fs::create_dir_all(dir.join("tests")).unwrap();
        fs::write(
            dir.join("task.toml"),
            format!(
                "schema_version = \"1.0\"\n\n[task]\nid = \"{id}\"\n\n[verifier]\nenvironment_mode = \"separate\"\n{}",
                match gpus {
                    Some(count) => format!("\n[environment]\ncpus = 2\nmemory_mb = 2048\ngpus = {count}\n"),
                    None => String::new(),
                }
            ),
        )
        .unwrap();
        fs::write(
            dir.join("instruction.md"),
            format!("TB3 synthetic corpus task {id} instruction (test staging only).\n"),
        )
        .unwrap();
    }
    let material: serde_json::Value = {
        let text = fs::read_to_string(&staged.material).unwrap();
        let mut value: serde_json::Value = serde_json::from_str(&text).unwrap();
        value["benchmarks"]["terminal-bench-2.1"]["task_package"] = json_path(&corpus);
        value["benchmarks"]["terminal-bench-2.1"]["task_package_manifest_sha256"] =
            serde_json::json!(directory_manifest_digest(&corpus));
        value
    };
    fs::write(
        &staged.material,
        serde_json::to_string_pretty(&material).unwrap(),
    )
    .unwrap();
    corpus
}

fn json_path(path: &Path) -> serde_json::Value {
    serde_json::json!(path.to_string_lossy().to_string())
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
    assert_eq!(preview["schema"], "opi-eval-plan-preview/1");
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
        gpu["reason"].as_str().unwrap().contains("GPU observation"),
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
    assert_eq!(plan["schema"], "opi-eval-run-plan/1");
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
            "GPU observation is unavailable",
        ),
        (
            "full scope cannot silently drop the unmet task",
            "[\"gpu-train\"]\n__scope_full",
            "gpu-train unknown",
        ),
        (
            "native profile binding",
            "[\"alpha-notes\"]",
            "not bound by the pinned native profile",
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
    material["benchmarks"]["deepswe-v1.1"]["task_package"] =
        serde_json::Value::String(variant_dir.to_string_lossy().into_owned());
    material["benchmarks"]["deepswe-v1.1"]["task_package_manifest_sha256"] =
        serde_json::Value::String(directory_manifest_digest(&variant_dir));
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
        let trial_dirs: Vec<_> = fs::read_dir(jobs[0].join("trials"))
            .unwrap()
            .flatten()
            .map(|entry| entry.path())
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
