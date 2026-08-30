//! Phase 18 local acceptance suites (task 18.16).
//!
//! P18-A19 locks ordinary `opi` Minimal Runtime behavior to the 18.1
//! commit-bound pre-Phase capture: the baseline evidence must verify against
//! its bound commit, the same captured commands must behave identically at
//! the current tree, and dependency/call-graph assertions must prove the
//! Companion never activates in the product.
//!
//! P18-A20 locks the frozen experiment contract's genericity: a third
//! harness subject and a fourth, not-yet-admitted benchmark revision
//! resolve structurally while the admitted set stays un-executed and
//! provisional (P18-RDM-002).

use std::path::{Path, PathBuf};
use std::process::Command;

use opi_eval::cli;
use opi_eval::experiment::ResolvedExperiment;

/// The 18.1 start commit that owns the pre-Phase Minimal Runtime capture.
/// A baseline bound to any other commit is a late capture and must be
/// rejected.
const BASELINE_COMMIT: &str = "1ad534b73864b7894929feabd7d48104aa0b0c05";

const BASELINE_DIR: &str = "crates/opi-eval/tests/fixtures/minimal-runtime/pre-phase18";

const CAPTURE_SCRIPT: &str = "scripts/capture-phase18-minimal-runtime-baseline.py";

/// Product source anchors whose bytes are digest-bound by the baseline
/// receipt; identical bytes at the current tree prove the ordinary runtime
/// paths were untouched by the Phase.
const PRODUCT_ANCHORS: [&str; 5] = [
    "crates/opi-coding-agent/src/cli.rs",
    "crates/opi-coding-agent/src/main.rs",
    "crates/opi-coding-agent/src/runner.rs",
    "crates/opi-coding-agent/src/execution/runtime.rs",
    "crates/opi-coding-agent/src/execution/router.rs",
];

/// Product crates that must never reference the Companion.
const PRODUCT_CRATES: [&str; 6] = [
    "opi-agent",
    "opi-ai",
    "opi-coding-agent",
    "opi-protocol",
    "opi-sandbox",
    "opi-tui",
];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/opi-eval/tests -> workspace root")
        .to_path_buf()
}

fn python() -> &'static str {
    if cfg!(windows) { "python" } else { "python3" }
}

fn strip_cr(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\r' && i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
            i += 1;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    out
}

/// Remove ANSI CSI escape sequences (CI-colored cargo progress).
fn strip_ansi(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b'[' {
            i += 2;
            while i < bytes.len() && !(0x40..=0x7e).contains(&bytes[i]) {
                i += 1;
            }
            i += 1;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    out
}

fn normalize_cargo_noise(bytes: &[u8]) -> Vec<u8> {
    // Cargo build progress is environment noise (the baseline was captured
    // on a cold cache), not product behavior: strip status lines and blank
    // lines so only real diagnostics are compared. CI runners set CI=true,
    // which makes cargo emit ANSI colors even on a non-tty; strip those
    // escape sequences first so the status-line filter still matches.
    let ansi_free: Vec<u8> = strip_ansi(bytes);
    let mut out = Vec::new();
    for line in ansi_free.split(|b| *b == b'\n') {
        let text = String::from_utf8_lossy(line);
        let trimmed = text.trim_start();
        if trimmed.starts_with("Compiling")
            || trimmed.starts_with("Finished")
            || trimmed.starts_with("Running")
            || trimmed.starts_with("Downloading")
            || trimmed.starts_with("Downloaded")
            || trimmed.starts_with("Locking")
            || trimmed.starts_with("Updating")
            || trimmed.is_empty()
        {
            continue;
        }
        out.extend_from_slice(line);
        out.push(b'\n');
    }
    out
}

fn generic_fixture_path() -> PathBuf {
    workspace_root().join(
        "crates/opi-eval/tests/fixtures/experiment/generic-three-subject-fourth-benchmark.toml",
    )
}

/// P18-A20: the three-harness-subject + fourth-benchmark document resolves
/// through the production `validate` seam, keeps the unadmitted revision
/// provisional (no integrity digest), and the contract stays generic —
/// arbitrary product identities resolve without an admitted-set check.
#[test]
fn p18_a20_generic_schema_resolves_without_admitted_product_hard_coding() {
    // Production call site 1: `opi_eval::cli::validate`.
    let summary = cli::validate(&generic_fixture_path())
        .expect("generic three-subject fourth-benchmark fixture must resolve");
    assert_eq!(summary.subject_count, 3);
    assert_eq!(summary.edge_count, 2);
    assert_eq!(summary.trial_count, 4);

    // Production call site 2: `ResolvedExperiment::resolve`.
    let source = std::fs::read_to_string(generic_fixture_path()).unwrap();
    let resolved = ResolvedExperiment::resolve(&source).unwrap();

    // The fourth benchmark descriptor resolves but is not admitted: no
    // integrity digest is carried, so the revision stays provisional.
    let benchmark = resolved.benchmark();
    assert_ne!(benchmark.name, "terminal-bench");
    assert_ne!(benchmark.name, "deepswe");
    assert_eq!(
        benchmark.integrity_digest, None,
        "unadmitted revision must not claim an integrity digest"
    );

    // The third harness subject is carried with its own product identity;
    // the admitted set (opi, pi) is not hard-coded into the schema.
    let products: Vec<&str> = resolved
        .subjects()
        .iter()
        .map(|s| s.product.as_str())
        .collect();
    assert!(products.contains(&"future-agent"));
    assert_eq!(products.len(), 3);

    // Genericity: the same frozen shape accepts any product identity. A
    // schema that hard-coded two admitted products would reject this.
    let inline = format!(
        "{base}\n[[subjects]]\nid = \"solo\"\nproduct = \"totally-unknown-harness\"\nversion = \"0.0.1\"\n",
        base = r#"
schema = "phase18-experiment/1"
experiment_id = "generic-inline"

[benchmark]
name = "another-frontier"
revision = "1.0"
dataset = "another-frontier-fixture"

[[subjects]]
id = "one"
product = "first-harness"
version = "1.0.0"

[[edges]]
id = "edge"
baseline = "one"
candidate = "solo"

[model_controls]
provider = "local"
model = "scripted"
endpoint_class = "local"
temperature = 0.0
max_output_tokens = 4096
reasoning = "omitted"

[environment]
platform = "linux"
architecture = "x86_64"
cwd_policy = "isolated"

[[trials]]
id = "t-one"
subject = "one"
task = "fixture-task"
group = "g"

[[trials]]
id = "t-solo"
subject = "solo"
task = "fixture-task"
group = "g"
"#
    );
    let inline_resolved =
        ResolvedExperiment::resolve(&inline).expect("unknown product identity must resolve");
    assert_eq!(inline_resolved.subjects().len(), 2);
    assert_eq!(
        inline_resolved.benchmark().integrity_digest,
        None,
        "unadmitted inline revision stays provisional"
    );

    // The committed `validate` CLI prints the same summary through the
    // binary (production entry seam of the command).
    let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("validate")
        .arg("--config")
        .arg(generic_fixture_path())
        .output()
        .expect("run opi-eval validate");
    assert!(
        output.status.success(),
        "validate CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("subjects=3"), "summary: {stdout}");
    assert!(stdout.contains("edges=2"), "summary: {stdout}");
}

// ---------------------------------------------------------------------------
// P18-A19: ordinary opi before/after the Phase
// ---------------------------------------------------------------------------

struct BaselineCheck {
    id: String,
    family: String,
    argv: Vec<String>,
    exit: i32,
    stdout_rel: PathBuf,
    stderr_rel: PathBuf,
}

fn load_baseline_checks(dir: &Path) -> Vec<BaselineCheck> {
    let receipt: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(dir.join("receipt.json")).expect("baseline receipt"),
    )
    .expect("baseline receipt JSON");
    assert_eq!(receipt["status"], "ok", "baseline receipt is not ok");
    assert_eq!(
        receipt["commit"], BASELINE_COMMIT,
        "baseline is not bound to the 18.1 start commit (late capture)"
    );
    let mut checks = Vec::new();
    for check in receipt["checks"].as_array().expect("checks") {
        checks.push(BaselineCheck {
            id: check["id"].as_str().unwrap().to_owned(),
            family: check["family"].as_str().unwrap().to_owned(),
            argv: check["argv"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap().to_owned())
                .collect(),
            exit: check["exit"].as_i64().unwrap() as i32,
            stdout_rel: PathBuf::from(check["stdout_file"].as_str().unwrap()),
            stderr_rel: PathBuf::from(check["stderr_file"].as_str().unwrap()),
        });
    }
    assert_eq!(checks.len(), 13, "expected the 13-command baseline");
    checks
}

fn verify_baseline(dir: &Path) {
    let repo = workspace_root();
    let status = Command::new(python())
        .current_dir(&repo)
        .args([
            CAPTURE_SCRIPT,
            "--verify",
            "--expected-commit",
            BASELINE_COMMIT,
            "--input",
        ])
        .arg(dir)
        .status()
        .expect("run baseline verifier");
    assert!(
        status.success(),
        "pre-Phase baseline failed verification (hand-authored or drifted)"
    );
}

/// P18-A19: the persisted 18.1 baseline verifies against its bound commit,
/// every captured command behaves identically at the current tree, the
/// captured CLI bytes are unchanged, and dependency/call-graph assertions
/// prove the Companion never activates in the product.
#[test]
fn p18_a19_ordinary_opi_minimal_runtime_before_after() {
    let repo = workspace_root();
    let baseline_dir = repo.join(BASELINE_DIR);

    // Reject a hand-authored or drifted baseline up front: the persisted
    // receipt, digests, helper identity, and bound-commit inventory must
    // verify without rerunning the historical runtime.
    verify_baseline(&baseline_dir);
    let checks = load_baseline_checks(&baseline_dir);

    for check in &checks {
        let temp = tempfile::tempdir().expect("isolated TMPDIR per check");
        let output = Command::new(&check.argv[0])
            .args(&check.argv[1..])
            .current_dir(&repo)
            .env("TMPDIR", temp.path())
            .output()
            .expect("replay captured command");

        assert_eq!(
            output.status.code(),
            Some(check.exit),
            "{} ({}): exit drifted",
            check.id,
            check.family
        );

        // The isolated TMPDIR must be empty afterwards: no background
        // activity leaves residue behind (the baseline recorded the same).
        let residue: Vec<_> = std::fs::read_dir(temp.path())
            .expect("read isolated TMPDIR")
            .collect();
        assert!(
            residue.is_empty(),
            "{} ({}): residual files in TMPDIR: {:?}",
            check.id,
            check.family,
            residue
                .iter()
                .map(|e| e.as_ref().unwrap().file_name())
                .collect::<Vec<_>>()
        );

        if check.family == "ordinary-cli-io" {
            let want_stdout = std::fs::read(baseline_dir.join(&check.stdout_rel)).unwrap();
            let want_stderr = std::fs::read(baseline_dir.join(&check.stderr_rel)).unwrap();
            assert_eq!(
                output.stdout, want_stdout,
                "{}: --help stdout drifted from the pre-Phase capture",
                check.id
            );
            assert_eq!(
                normalize_cargo_noise(&output.stderr),
                normalize_cargo_noise(&want_stderr),
                "{}: --help stderr drifted from the pre-Phase capture",
                check.id
            );
        }
    }

    // Dependency assertion: no workspace member except opi-eval itself
    // declares a dependency on the Companion (no reverse activation edge).
    let metadata = Command::new("cargo")
        .current_dir(&repo)
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .output()
        .expect("cargo metadata");
    assert!(metadata.status.success());
    let value: serde_json::Value = serde_json::from_slice(&metadata.stdout).expect("metadata JSON");
    for package in value["packages"].as_array().unwrap() {
        let name = package["name"].as_str().unwrap();
        if name == "opi-eval" {
            continue;
        }
        for dep in package["dependencies"].as_array().unwrap() {
            assert_ne!(
                dep["name"].as_str().unwrap(),
                "opi-eval",
                "{name}: product/workspace crate must not depend on the Companion"
            );
        }
    }

    // Call-graph assertion 1: the ordinary runtime source anchors are
    // byte-identical to the baseline commit.
    for anchor in PRODUCT_ANCHORS {
        let bound = Command::new("git")
            .current_dir(&repo)
            .args(["show", &format!("{BASELINE_COMMIT}:{anchor}")])
            .output()
            .expect("git show anchor");
        assert!(
            bound.status.success(),
            "anchor missing at baseline: {anchor}"
        );
        let disk = std::fs::read(repo.join(anchor)).unwrap();
        assert_eq!(
            strip_cr(&bound.stdout),
            strip_cr(&disk),
            "product anchor drifted from the pre-Phase capture: {anchor}"
        );
    }

    // Call-graph assertion 2: no product crate source references the
    // Companion at all.
    for crate_name in PRODUCT_CRATES {
        let src = repo.join("crates").join(crate_name).join("src");
        let mut stack = vec![src];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).expect("read product src") {
                let entry = entry.expect("dir entry");
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                    continue;
                }
                let bytes = std::fs::read(&path).unwrap();
                let text = String::from_utf8_lossy(&bytes).to_lowercase();
                assert!(
                    !text.contains("opi-eval") && !text.contains("opi_eval"),
                    "{} references the Companion",
                    path.strip_prefix(&repo).unwrap().display()
                );
            }
        }
    }
}

/// P18-A19 negative: a doctored baseline must fail closed. A receipt whose
/// recorded commit is rewritten (a late capture presented as the baseline)
/// or whose raw evidence bytes are edited (hand-authored) cannot pass
/// verification.
#[test]
fn p18_a19_rejects_hand_authored_or_late_baseline() {
    let repo = workspace_root();
    let baseline_dir = repo.join(BASELINE_DIR);

    // Late baseline: same evidence, different bound commit.
    let late = tempfile::tempdir().expect("late baseline temp");
    copy_dir(&baseline_dir, late.path());
    let receipt_path = late.path().join("receipt.json");
    let mut receipt: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&receipt_path).unwrap()).unwrap();
    receipt["commit"] = serde_json::json!("deadbeefdeadbeefdeadbeefdeadbeefdeadbeef");
    std::fs::write(&receipt_path, serde_json::to_vec(&receipt).unwrap()).unwrap();
    let status = Command::new(python())
        .current_dir(&repo)
        .args([
            CAPTURE_SCRIPT,
            "--verify",
            "--expected-commit",
            BASELINE_COMMIT,
            "--input",
        ])
        .arg(late.path())
        .status()
        .expect("verify late baseline");
    assert!(!status.success(), "late baseline must be rejected");

    // Hand-authored baseline: raw evidence bytes edited after capture.
    let doctored = tempfile::tempdir().expect("doctored baseline temp");
    copy_dir(&baseline_dir, doctored.path());
    let stdout_path = doctored.path().join("checks/01-ordinary-cli-io/stdout.log");
    let mut bytes = std::fs::read(&stdout_path).unwrap();
    bytes.extend_from_slice(b"\ndoctored\n");
    std::fs::write(&stdout_path, bytes).unwrap();
    let status = Command::new(python())
        .current_dir(&repo)
        .args([
            CAPTURE_SCRIPT,
            "--verify",
            "--expected-commit",
            BASELINE_COMMIT,
            "--input",
        ])
        .arg(doctored.path())
        .status()
        .expect("verify doctored baseline");
    assert!(!status.success(), "hand-authored baseline must be rejected");
}

fn copy_dir(source: &Path, target: &Path) {
    for entry in std::fs::read_dir(source).expect("read baseline dir") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        let name = entry.file_name();
        let dest = target.join(&name);
        if path.is_dir() {
            std::fs::create_dir_all(&dest).unwrap();
            copy_dir(&path, &dest);
        } else {
            std::fs::copy(&path, &dest).unwrap();
        }
    }
}
