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
fn verifier_stand_in_script() -> String {
    r#"#!/bin/sh
# native stand-in for the pinned upstream runner entry (test staging)
set -eu
[ "$1" = "run" ] && [ "$2" = "--locked" ] && [ "$3" = "harbor" ] && [ "$4" = "run" ] \
  && [ "$5" = "-p" ] || { echo "argv drift: expected run --locked harbor run -p" >&2; exit 9; }
task_dir="$6"
agent=""
model=""
prev=""
for arg in "$@"; do
  case "$prev" in
    --agent) agent="$arg" ;;
    --model) model="$arg" ;;
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
trial="${task_name}__standin"
job="jobs/2026-09-05__12-00-00"
reward=1.0
exception=null
tb30_profile=""
case "$OPI_EVAL_STANDIN_MODE" in
  ok) ;;
  tb30-ok|tb30-undeclared|tb30-substituted-declared) tb30_profile=1 ;;
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
if [ "$agent" = "harbor.agents.nop:NopAgent" ]; then
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
  if [ -n "$tb30_profile" ]; then
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

    // The pinned integration module directory (the contract form the
    // selected import path resolves against).
    let integrations = mat.join("integrations");
    write_exec(
        &integrations.join("opi_eval_harbor.py"),
        "# native stand-in integration module (test staging)\n",
    );

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
    let integration_block = |import: &str, name: &str| {
        format!(
            r#""integration": {{"module_dir": {:?}, "module_manifest_sha256": {:?},
                        "import_path": "{}", "agent_name": "{}"}}"#,
            integrations.to_string_lossy(),
            directory_manifest_digest(&integrations),
            import,
            name,
        )
    };
    fs::write(
        &material,
        format!(
            r#"{{
  "schema": "opi-eval-native-material/2",
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
    // Tamper with the pinned integration module after materialization.
    fs::write(
        staged.root.join("material/integrations/opi_eval_harbor.py"),
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

#[test]
fn sibling_native_families_are_explicitly_unavailable() {
    // A DeepSWE experiment with native material is refused before any
    // trial: its former default-job path must not keep emitting Agent
    // scores (P19-D04 sequencing).
    let staged = stage("ok");
    let digest = derive_integrity(&staged);
    let config = staged.root.join("deepswe.toml");
    fs::write(
        &config,
        experiment_text(&staged, &digest)
            .replace("native-driver-test", "native-deepswe-test")
            .replace(
                "terminal-bench\"\nrevision = \"2.1\"",
                "deepswe\"\nrevision = \"v1.1\"",
            ),
    )
    .unwrap();
    let run_root = staged.root.join("run-deepswe");
    let (code, _, stderr) = run_native(&staged, &config, &run_root);
    assert_eq!(code, 2);
    assert!(
        stderr.contains("native scoring for deepswe-v1.1 is unavailable"),
        "stderr: {stderr}"
    );
    assert!(!run_root.join("trials").exists());
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
    for (case, agent, reward) in [("oracle-witness", "oracle", 1), ("nop-witness", "nop", 0)] {
        let (code, stdout, stderr) =
            run_conformance_native(&staged, "benchmark", "terminal-bench-2.1", case);
        assert_eq!(code, 0, "{case}: {stderr}");
        let report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(report["met"], true, "{case}: {stdout}");
        assert_eq!(report["outcome"], "verified-native", "{case}: {stdout}");
        let _ = (agent, reward);
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
