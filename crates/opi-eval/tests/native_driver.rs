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
task_name=$(basename "$task_dir")
trial="${task_name}__standin"
job="jobs/2026-09-05__12-00-00"
reward=1.0
exception=null
case "$OPI_EVAL_STANDIN_MODE" in
  ok) ;;
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
  printf '{"agent": {"name": "%s", "import_path": null}}\n' "$agent" \
    > "$job/trials/$trial/config.json"
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
printf '[{"source": "/root/answer.txt", "destination": "artifacts/answer.txt", "type": "file", "status": "ok", "service": null}]\n' \
  > "$job/trials/$trial/artifacts/manifest.json"
printf '%s\n' "$answer" > "$job/trials/$trial/artifacts/answer.txt"
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
[ -n "$PI_API_KEY" ] || {{ echo "missing dummy credential" >&2; exit 8; }}
sed -e 's/"provider":"local"/"provider":"scripted"/' \
    -e 's/"model":"scripted"/"model":"scripted\/eval"/' "{stream}"
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
      "model": "scripted:eval",
      "provider_env": {{"OPENAI_API_KEY": "<dummy-scripted-credential>"}},
      "config": {{"kind": "opi-toml", "base_url": "http://127.0.0.1:48127/v1",
                  "model_id": "eval", "api_key": "<dummy>"}},
      {}
    }},
    "pi": {{
      "executable": {{"path": {:?}, "sha256": {:?}}},
      "model": "scripted:scripted/eval",
      "provider_env": {{"PI_API_KEY": "<redacted-dummy>"}},
      "config": {{"kind": "pi-models-json", "base_url": "http://127.0.0.1:48127/v1",
                  "model_id": "scripted/eval", "api_key": "<redacted-dummy>"}},
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
        r#"schema = "opi-eval-experiment/1"
experiment_id = "native-driver-test"

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

[model_controls]
provider = "scripted"
model = "eval"
endpoint_class = "loopback"
temperature = 0.0
max_output_tokens = 4096
reasoning = "omitted"

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

#[test]
fn sibling_native_families_are_explicitly_unavailable() {
    // A TB3 experiment with native material is refused before any trial:
    // its former default-job path must not keep emitting Agent scores
    // (P19-D03/P19-D04 sequencing).
    let staged = stage("ok");
    let digest = derive_integrity(&staged);
    let config = staged.root.join("tb30.toml");
    fs::write(
        &config,
        experiment_text(&staged, &digest)
            .replace("native-driver-test", "native-tb30-test")
            .replace(
                "terminal-bench\"\nrevision = \"2.1\"",
                "terminal-bench\"\nrevision = \"3.0\"",
            ),
    )
    .unwrap();
    let run_root = staged.root.join("run-tb30");
    let (code, _, stderr) = run_native(&staged, &config, &run_root);
    assert_eq!(code, 2);
    assert!(
        stderr.contains("native scoring for terminal-bench-3.0 is unavailable"),
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
