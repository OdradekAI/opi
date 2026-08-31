//! Native driving-mode integration tests (task 18.14.1).
//!
//! Each test stages a resolved-material manifest whose agent, verifier,
//! and oracle entries are deterministic local stand-in executables and
//! whose task package is the checked-in synthetic fixture package, then
//! drives the production `opi-eval run --native-material` binary through
//! the full native flow: manifest validation, package verification, the
//! upstream oracle preflight, paired trials through the exact declared
//! executables with the deterministic configuration projection, native
//! verifier reports written into the trace root, sealing, and the
//! comparison report. This proves the native driver plumbing only; the
//! real built Opi/pi programs, official task environments, and unchanged
//! upstream verifiers run at the task 18.15 dispatch. No paid provider,
//! credential, network listener, or user-global resource is touched.

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
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

/// The sorted-manifest digest scheme of `runner::material`, recomputed
/// independently here so the test does not trust the implementation's own
/// helper.
fn package_manifest_digest(root: &Path) -> String {
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

struct Staged {
    /// Owns the staging directory so it outlives every test assertion.
    _guard: tempfile::TempDir,
    root: PathBuf,
    material: PathBuf,
}

/// Stages the whole native material: profile, task package, agent and
/// verifier stand-ins, provider script copy, and the manifest with
/// recomputed digests.
fn stage(oracle_body: &str) -> Staged {
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

    let trace_fixture = fixtures().join("agents/opi/trace-complete/run-0001");
    let stream_fixture = fixtures().join("agents/pi/stream-ok.jsonl");
    let ctrf_fixture = fixtures().join("benchmarks/terminal-bench-2.1/ctrf/ok-six-passed.json");

    // Opi stand-in: exact argv surface, isolated config, dummy credential,
    // completed trace child, answer file.
    write_exec(
        &mat.join("agents/opi"),
        &format!(
            r#"#!/bin/sh
# native stand-in for the exact built opi binary (test staging)
[ "$1" = "--json" ] || {{ echo "argv drift: expected --json first" >&2; exit 9; }}
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
# The stream is replayed with the native model identity projected into it,
# exactly as the real pi emits its selected model on every event.
sed -e 's/"provider":"local"/"provider":"scripted"/' \
    -e 's/"model":"scripted"/"model":"scripted\/phase18"/' "{stream}"
printf 'pi stand-in answer\n' > answer.txt
exit 0
"#,
            stream = stream_fixture.display(),
        ),
    );

    // Verifier stand-in for the pinned uv entrypoint: consumes the exact
    // `uv run --locked harbor run -p <task-dir>` argv, writes the native
    // report into its cwd (the trace root).
    write_exec(
        &mat.join("verifier-uv.sh"),
        &format!(
            r#"#!/bin/sh
# native stand-in for the pinned uv verifier entrypoint (test staging)
[ "$1" = "run" ] && [ "$2" = "--locked" ] && [ "$3" = "harbor" ] || {{ echo "argv drift" >&2; exit 9; }}
cp "{ctrf}" ./ctrf-report.json
exit 0
"#,
            ctrf = ctrf_fixture.display(),
        ),
    );

    // Oracle stand-in: same launch surface, applies the reference
    // solution, writes a passing native report.
    write_exec(&mat.join("oracle-uv.sh"), oracle_body);

    // The provider script copy is digest-pinned identity only here; the
    // stand-in agents never contact a listener in local gates.
    let provider = mat.join("phase18-scripted-provider.py");
    fs::copy(
        manifest_dir().join("../../scripts/phase18-scripted-provider.py"),
        &provider,
    )
    .unwrap();
    let static_lock = mat.join("static-lock.json");
    fs::write(&static_lock, b"{\"schema\": \"fixture-lock\"}\n").unwrap();

    let digest_of = |path: &Path| sha256_hex(&fs::read(path).unwrap());
    let material = mat.join("material.json");
    fs::write(
        &material,
        format!(
            r#"{{
  "schema": "phase18-native-material/1",
  "static_lock": {{"path": {:?}, "sha256": {:?}}},
  "provider": {{
    "script": {{"path": {:?}, "sha256": {:?}}},
    "endpoint": "http://127.0.0.1:48127/v1",
    "request_log": {:?}
  }},
  "agents": {{
    "opi": {{
      "executable": {{"path": {:?}, "sha256": {:?}}},
      "model": "scripted:phase18",
      "provider_env": {{"OPENAI_API_KEY": "<dummy-scripted-credential>"}},
      "config": {{"kind": "opi-toml", "base_url": "http://127.0.0.1:48127/v1",
                  "model_id": "phase18", "api_key": "<dummy>"}}
    }},
    "pi": {{
      "executable": {{"path": {:?}, "sha256": {:?}}},
      "model": "scripted:scripted/phase18",
      "provider_env": {{"PI_API_KEY": "<redacted-dummy>"}},
      "config": {{"kind": "pi-models-json", "base_url": "http://127.0.0.1:48127/v1",
                  "model_id": "scripted/phase18", "api_key": "<redacted-dummy>"}}
    }}
  }},
  "benchmarks": {{
    "terminal-bench-2.1": {{
      "profile": {:?},
      "task_package": {:?},
      "task_package_manifest_sha256": {:?},
      "verifier_executable": {{"path": {:?}, "sha256": {:?}}},
      "verifier_env": {{}},
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
            mat.join("agents/pi").to_string_lossy(),
            digest_of(&mat.join("agents/pi")),
            profile.to_string_lossy(),
            task.to_string_lossy(),
            package_manifest_digest(&task),
            mat.join("verifier-uv.sh").to_string_lossy(),
            digest_of(&mat.join("verifier-uv.sh")),
            mat.join("oracle-uv.sh").to_string_lossy(),
            digest_of(&mat.join("oracle-uv.sh")),
        ),
    )
    .unwrap();

    Staged {
        _guard: root,
        root: root_path,
        material,
    }
}

fn passing_oracle() -> String {
    format!(
        r#"#!/bin/sh
# native stand-in for the upstream oracle (test staging)
[ "$1" = "run" ] && [ "$2" = "--locked" ] || {{ echo "argv drift" >&2; exit 9; }}
cp "{}" ./ctrf-report.json
printf 'oracle applied\n' > ./oracle-applied.txt
exit 0
"#,
        fixtures()
            .join("benchmarks/terminal-bench-2.1/ctrf/ok-six-passed.json")
            .display()
    )
}

fn failing_oracle() -> String {
    r#"#!/bin/sh
# native stand-in for a failing upstream oracle (test staging)
[ "$1" = "run" ] && [ "$2" = "--locked" ] || { echo "argv drift" >&2; exit 9; }
exit 5
"#
    .to_owned()
}

/// Writes one Pier `jobs/<timestamp>/result.json` aggregate whose `metric`
/// awards `reward_key` to the single trial (test staging).
fn pier_oracle(metric: &str, reward_key: &str) -> String {
    format!(
        r#"#!/bin/sh
# native stand-in writing a crafted Pier job aggregate (test staging)
[ "$1" = "run" ] && [ "$2" = "--locked" ] || {{ echo "argv drift" >&2; exit 9; }}
mkdir -p ./jobs/2026-08-30__00-00-00
cat > ./jobs/2026-08-30__00-00-00/result.json <<'JSON'
{{"n_total_trials": 1, "stats": {{"evals": {{"x": {{"reward_stats": {{"{metric}": {{"{reward_key}": ["t"]}}}}}}}}}}}}
JSON
exit 0
"#
    )
}

/// Stages DeepSWE native material whose profile drives the real Pier
/// job-aggregate import (the native dispatch surface) with the fixture
/// task package and the same agent stand-ins.
fn stage_deepswe(oracle_body: &str) -> Staged {
    let root = tempfile::tempdir().unwrap();
    let root_path = root.path().to_path_buf();
    let mat = root_path.join("material");
    fs::create_dir_all(&mat).unwrap();

    let profile = mat.join("profile.toml");
    // Stage the DeepSWE profile for the native Pier job-aggregate surface,
    // with the official instruction file added to the pinned byte table
    // (the native driving contract reads it as the task prompt) and the
    // package manifest digest recomputed over the same pinned-table
    // scheme the profile parser enforces.
    let instruction = "solve the synthetic task\n";
    let pinned = [
        ("README.md", "100644"),
        ("environment/Dockerfile", "100644"),
        ("verifier/collect.sh", "100755"),
    ];
    let mut table: Vec<serde_json::Value> = pinned
        .iter()
        .map(|(path, mode)| {
            let bytes = fs::read(
                fixtures()
                    .join("benchmarks/deepswe-v1.1/task-package")
                    .join(path),
            )
            .unwrap();
            serde_json::json!({
                "mode": mode,
                "path": path,
                "sha256": sha256_hex(&bytes),
                "size": bytes.len(),
            })
        })
        .collect();
    table.push(serde_json::json!({
        "mode": "100644",
        "path": "instruction.md",
        "sha256": sha256_hex(instruction.as_bytes()),
        "size": instruction.len(),
    }));
    table.sort_by(|a, b| a["path"].as_str().cmp(&b["path"].as_str()));
    let package_digest = sha256_hex(serde_json::to_string(&table).unwrap().as_bytes());
    let profile_text = fs::read_to_string(
        fixtures().join("benchmarks/deepswe-v1.1/profile/synthetic.toml"),
    )
    .unwrap()
    .replace(
        "output_kind = \"pier-report\"",
        "output_kind = \"unpinned-pending-18-15\"",
    )
    .replace(
        "package_manifest_sha256 = \"13e94f44d464818873ba00931e66339db1a6249d94f1e1e17b9c8060726bd3ae\"",
        &format!("package_manifest_sha256 = \"{package_digest}\""),
    )
    + &format!(
        "\n[[package]]\npath = \"instruction.md\"\nmode = \"100644\"\nsize = {}\nsha256 = \"{}\"\n",
        instruction.len(),
        sha256_hex(instruction.as_bytes())
    );
    fs::write(&profile, profile_text).unwrap();
    let task = mat.join("task-package");
    copy_dir(
        &fixtures().join("benchmarks/deepswe-v1.1/task-package"),
        &task,
    );
    fs::write(task.join("instruction.md"), instruction).unwrap();

    write_exec(
        &mat.join("agents/opi"),
        r#"#!/bin/sh
# native stand-in for the exact built opi binary (test staging)
[ "$1" = "--json" ] || { echo "argv drift" >&2; exit 9; }
printf 'native stand-in answer\n' > answer.txt
exit 0
"#,
    );
    write_exec(
        &mat.join("agents/pi"),
        r#"#!/bin/sh
# native stand-in for the exact built pi bundle (test staging)
[ "$1" = "--mode" ] && [ "$2" = "json" ] || { echo "argv drift" >&2; exit 9; }
printf 'pi stand-in answer\n' > answer.txt
exit 0
"#,
    );
    write_exec(&mat.join("oracle-uv.sh"), oracle_body);
    write_exec(
        &mat.join("verifier-uv.sh"),
        r#"#!/bin/sh
# native stand-in for the pinned uv verifier entrypoint (test staging)
[ "$1" = "run" ] && [ "$2" = "--locked" ] || { echo "argv drift" >&2; exit 9; }
exit 0
"#,
    );

    let provider = mat.join("phase18-scripted-provider.py");
    fs::copy(
        manifest_dir().join("../../scripts/phase18-scripted-provider.py"),
        &provider,
    )
    .unwrap();
    let static_lock = mat.join("static-lock.json");
    fs::write(&static_lock, b"{\"schema\": \"fixture-lock\"}\n").unwrap();

    let digest_of = |path: &Path| sha256_hex(&fs::read(path).unwrap());
    let material = mat.join("material.json");
    fs::write(
        &material,
        format!(
            r#"{{
  "schema": "phase18-native-material/1",
  "static_lock": {{"path": {:?}, "sha256": {:?}}},
  "provider": {{
    "script": {{"path": {:?}, "sha256": {:?}}},
    "endpoint": "http://127.0.0.1:48127/v1",
    "request_log": {:?}
  }},
  "agents": {{
    "opi": {{
      "executable": {{"path": {:?}, "sha256": {:?}}},
      "model": "scripted:phase18",
      "provider_env": {{"OPENAI_API_KEY": "<dummy-scripted-credential>"}},
      "config": {{"kind": "opi-toml", "base_url": "http://127.0.0.1:48127/v1",
                  "model_id": "phase18", "api_key": "<dummy>"}}
    }},
    "pi": {{
      "executable": {{"path": {:?}, "sha256": {:?}}},
      "model": "scripted:scripted/phase18",
      "provider_env": {{"PI_API_KEY": "<redacted-dummy>"}},
      "config": {{"kind": "pi-models-json", "base_url": "http://127.0.0.1:48127/v1",
                  "model_id": "scripted/phase18", "api_key": "<redacted-dummy>"}}
    }}
  }},
  "benchmarks": {{
    "deepswe-v1.1": {{
      "profile": {:?},
      "task_package": {:?},
      "task_package_manifest_sha256": {:?},
      "verifier_executable": {{"path": {:?}, "sha256": {:?}}},
      "verifier_env": {{}},
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
            mat.join("agents/pi").to_string_lossy(),
            digest_of(&mat.join("agents/pi")),
            profile.to_string_lossy(),
            task.to_string_lossy(),
            package_manifest_digest(&task),
            mat.join("verifier-uv.sh").to_string_lossy(),
            digest_of(&mat.join("verifier-uv.sh")),
            mat.join("oracle-uv.sh").to_string_lossy(),
            digest_of(&mat.join("oracle-uv.sh")),
        ),
    )
    .unwrap();

    Staged {
        _guard: root,
        root: root_path,
        material,
    }
}

fn deepswe_experiment_text(integrity_digest: &str) -> String {
    format!(
        r#"schema = "phase18-experiment/1"
experiment_id = "native-deepswe-oracle-test"

[benchmark]
name = "deepswe"
revision = "v1.1"
dataset = "native-deepswe-oracle-test"
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
model = "phase18"
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
task = "synthetic-fixture-task"
group = "group-native"

[[trials]]
id = "trial-opi-native"
subject = "candidate-opi"
task = "synthetic-fixture-task"
group = "group-native"
"#,
    )
}

#[test]
fn deepswe_oracle_preflight_enforces_the_positive_native_reward_bar() {
    // A known positive native reward passes the DeepSWE oracle preflight.
    let staged = stage_deepswe(&pier_oracle("reward", "1.0"));
    let config = staged.root.join("config.toml");
    let zero = "0".repeat(64);
    fs::write(&config, deepswe_experiment_text(&zero)).unwrap();
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
        "validate rejected the deepswe material: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let digest = stdout
        .split_whitespace()
        .find(|token| token.starts_with("native_integrity="))
        .unwrap()
        .trim_start_matches("native_integrity=")
        .to_owned();
    fs::write(&config, deepswe_experiment_text(&digest)).unwrap();
    let run_root = staged.root.join("preflight-positive");
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
        .arg("--preflight-only")
        .output()
        .expect("preflight-only executes");
    assert!(
        output.status.success(),
        "positive native reward must pass: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&output.stdout).trim()).unwrap();
    assert_eq!(report["outcome"], "preflight-only");
    assert_eq!(report["preflight"]["outcome"], "passed");

    // Every non-passing shape rejects the run before any trial: a zero
    // native reward (broken or mis-collected reference solution),
    // out-of-domain rewards (negative, above one, fractional), and an
    // aggregate without any `reward` metric at all.
    for (metric, reward_key) in [
        ("reward", "0.0"),
        ("reward", "-1.0"),
        ("reward", "2.0"),
        ("reward", "0.5"),
        ("f2p", "1.0"),
    ] {
        let staged = stage_deepswe(&pier_oracle(metric, reward_key));
        fs::write(&config, deepswe_experiment_text(&digest)).unwrap();
        let run_root = staged.root.join("preflight-rejected");
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
            .arg("--preflight-only")
            .output()
            .expect("preflight-only executes");
        assert_ne!(
            output.status.code(),
            Some(0),
            "{metric}={reward_key} must not admit the task"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("oracle preflight"),
            "{metric}={reward_key}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!run_root.join("trials").exists());
    }
}

fn experiment_text(_staged: &Staged, integrity_digest: &str) -> String {
    format!(
        r#"schema = "phase18-experiment/1"
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
model = "phase18"
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

#[test]
fn native_run_drives_paired_trials_through_the_declared_executables() {
    let staged = stage(&passing_oracle());
    let config = write_config(&staged);
    let run_root = staged.root.join("run");
    let (code, stdout, stderr) = run_native(&staged, &config, &run_root);
    assert_eq!(code, 0, "stderr: {stderr}\nstdout: {stdout}");
    let report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(report["outcome"], "completed");
    assert_eq!(report["trials"].as_array().unwrap().len(), 2);
    assert_eq!(report["pairs"].as_array().unwrap().len(), 1);
    // The persisted run report carries the same contract.
    let persisted: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(run_root.join("run-report.json")).unwrap())
            .unwrap();
    assert_eq!(persisted["outcome"], "completed");

    // The oracle preflight ran and passed before any trial.
    let preflight = run_root.join("preflight/terminal-bench-2.1/preflight-receipt.json");
    let receipt: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&preflight).unwrap()).unwrap();
    assert_eq!(receipt["schema"], "phase18-oracle-preflight/1");
    assert_eq!(receipt["outcome"], "passed");

    // Both trials sealed with native agent and verifier evidence.
    for trial in ["trial-pi-native", "trial-opi-native"] {
        let bundle = run_root.join("trials").join(trial).join("bundle");
        assert!(bundle.join("manifest.json").is_file(), "{trial} sealed");
        let answer = bundle.join("artifacts/native/agent-answer.txt");
        assert!(answer.is_file(), "{trial} staged the agent answer");
        let report = bundle.join("artifacts/native/native/ctrf-report");
        assert!(
            report.is_file(),
            "{trial} staged the native verifier report"
        );
    }

    // The deterministic configuration projection reached the products.
    let opi_config =
        fs::read_to_string(run_root.join("trials/trial-opi-native/bench.toml")).unwrap();
    assert!(opi_config.contains("openai-completions"));
    assert!(opi_config.contains("http://127.0.0.1:48127/v1"));
    let models = run_root.join("trials/trial-pi-native/iso/appdata/pi-agent/models.json");
    assert!(models.is_file(), "pi isolated models.json materialized");
}

#[test]
fn native_preflight_only_stops_before_any_trial() {
    let staged = stage(&passing_oracle());
    let config = write_config(&staged);
    let run_root = staged.root.join("preflight-run");
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
        .arg("--preflight-only")
        .output()
        .expect("preflight-only executes");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(report["outcome"], "preflight-only");
    assert_eq!(report["preflight"]["outcome"], "passed");
    assert!(!run_root.join("trials").exists());
}

#[test]
fn a_failing_oracle_preflight_rejects_the_run_before_trials() {
    let staged = stage(&failing_oracle());
    let config = write_config(&staged);
    let run_root = staged.root.join("run");
    let (code, _, stderr) = run_native(&staged, &config, &run_root);
    assert_ne!(code, 0);
    assert!(stderr.contains("oracle preflight"), "stderr: {stderr}");
    assert!(!run_root.join("trials").exists());
}

#[test]
fn task_package_drift_after_materialization_is_rejected() {
    let staged = stage(&passing_oracle());
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
fn agent_executable_digest_drift_is_rejected_at_load() {
    let staged = stage(&passing_oracle());
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
        .arg(manifest_dir().join("../../scripts/phase18-scripted-provider.py"))
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
    let staged = stage(&passing_oracle());
    // Agent cases: both exact executables settle Completed with the real
    // native evidence contracts.
    for adapter in ["opi", "pi"] {
        let (code, stdout, stderr) = run_conformance_native(&staged, "agent", adapter, "completed");
        assert_eq!(code, 0, "{adapter}: {stderr}");
        let report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(report["met"], true, "{adapter}: {stdout}");
        assert_eq!(report["outcome"], "completed");
        assert!(
            report["notes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|note| note == "final-workspace-answer-verified"),
            "{adapter}: {stdout}"
        );
    }
    // Benchmark cases: the pinned verifier entrypoint grades the
    // materialized official package through the real adapter contract.
    for case in ["completed", "identity", "immutable-capture"] {
        let (code, stdout, stderr) =
            run_conformance_native(&staged, "benchmark", "terminal-bench-2.1", case);
        assert_eq!(code, 0, "{case}: {stderr}");
        let report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(report["met"], true, "{case}: {stdout}");
        assert_eq!(report["outcome"], "verified");
        if case == "immutable-capture" {
            assert!(
                stdout.contains("immutable-capture-verified"),
                "{case}: {stdout}"
            );
        }
    }
}

#[test]
fn native_agent_conformance_requires_final_workspace_answer() {
    let staged = stage(&passing_oracle());
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
    let staged = stage(&passing_oracle());
    let (code, _, stderr) = run_conformance_native(&staged, "agent", "opi", "timeout");
    assert_eq!(code, 2);
    assert!(stderr.contains("hermetic-only"), "stderr: {stderr}");
    let (code, _, stderr) =
        run_conformance_native(&staged, "benchmark", "terminal-bench-2.1", "parse-failure");
    assert_eq!(code, 2);
    assert!(stderr.contains("hermetic-only"), "stderr: {stderr}");
}
