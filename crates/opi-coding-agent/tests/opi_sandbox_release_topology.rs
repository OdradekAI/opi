//! Phase 16 task 16.15.2 — native opi-sandbox CI/release topology guard
//! (SC16-12b / design `### Repository gates` + `### Native platform contract`).
//!
//! Asserts the repository CI/release wiring matches the shipped contract:
//!   - `ci.yml` runs an opi-sandbox package job on Linux + macOS that builds the
//!     archive, packages it, and invokes the standalone smoke against the
//!     EXTRACTED binary (provenance: `extracted/bin/opi-sandbox`, never a bare
//!     workspace `target/` binary);
//!   - `ci.yml` no longer carries the stale Phase 15 `sandbox_product` job (it
//!     ran `sandbox_strict`/`sandbox_linux_backend`, which task 16.16.1 deletes
//!     from the core crate — keeping it would leave a dangling reference);
//!   - `ci.yml` retains `target_check` as the six-target opi-coding-agent
//!     compile gate — intentionally retained because it references no sandbox
//!     tests, only cross-target compilation, and preserves PR-time cross-arch
//!     compile coverage;
//!   - `release.yml` builds opi-sandbox archives for Linux + macOS only — NO
//!     Windows opi-sandbox artifact — while the ordinary six-target Opi build
//!     matrix is preserved unchanged;
//!   - the two superseded standalone sandbox workflows are gone (their coverage
//!     folded into `ci.yml`'s opi-sandbox job).
//!
//! All four supported opi-sandbox archive triples are built and smoked on
//! matching native runners. The 16.15.1 packager detects the HOST triple and
//! refuses to label a cross-built archive as native.
//!
//! These are config-contract guards over the workflow YAML (the artifact under
//! test). Execution acceptance is parsed as YAML nodes; the remaining topology
//! guards are structurally sliced by top-level job key. The packager's own
//! layout/lock/extraction contract is pinned independently by
//! `opi_sandbox_packaging.rs` (16.15.1) and is not duplicated.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

use serde_yaml::{Mapping, Value};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_repo_file(relative: &str) -> String {
    let path = repo_root().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

fn yaml_key(key: &str) -> Value {
    Value::String(key.to_owned())
}

fn yaml_field<'a>(mapping: &'a Mapping, key: &str, path: &str) -> Result<&'a Value, String> {
    mapping
        .get(yaml_key(key))
        .ok_or_else(|| format!("{path} must define `{key}`"))
}

fn yaml_mapping<'a>(value: &'a Value, path: &str) -> Result<&'a Mapping, String> {
    value
        .as_mapping()
        .ok_or_else(|| format!("{path} must be a YAML mapping"))
}

fn yaml_has_key(mapping: &Mapping, key: &str) -> bool {
    mapping.contains_key(yaml_key(key))
}

fn yaml_string_field<'a>(mapping: &'a Mapping, key: &str) -> Option<&'a str> {
    mapping.get(yaml_key(key)).and_then(Value::as_str)
}

fn validate_cargo_test_command(
    step_name: &str,
    run: &str,
    expected_target: &str,
    expect_no_run: bool,
) -> Result<(), String> {
    let tokens = run.split_whitespace().collect::<Vec<_>>();
    if tokens.get(..2) != Some(["cargo", "test"].as_slice()) {
        return Err(format!(
            "step `{step_name}` command must start with `cargo test`"
        ));
    }

    let mut package_seen = false;
    let mut feature_seen = false;
    let mut test_target_seen = false;
    let mut no_run = false;
    let mut index = 2;
    while index < tokens.len() {
        match tokens[index] {
            "-p" if !package_seen && tokens.get(index + 1) == Some(&"opi-coding-agent") => {
                package_seen = true;
                index += 2;
            }
            "--features"
                if !feature_seen
                    && tokens.get(index + 1) == Some(&"execution-backend-test-fixture") =>
            {
                feature_seen = true;
                index += 2;
            }
            "--test" if !test_target_seen && tokens.get(index + 1) == Some(&expected_target) => {
                test_target_seen = true;
                index += 2;
            }
            "--no-run" if expect_no_run && !no_run => {
                no_run = true;
                index += 1;
            }
            token => {
                return Err(format!(
                    "step `{step_name}` command contains unexpected or duplicate token `{token}`"
                ));
            }
        }
    }

    if !package_seen {
        return Err(format!(
            "step `{step_name}` command must contain `-p opi-coding-agent` exactly once"
        ));
    }
    if !feature_seen {
        return Err(format!(
            "step `{step_name}` command must contain `--features execution-backend-test-fixture` exactly once"
        ));
    }
    if !test_target_seen {
        return Err(format!(
            "step `{step_name}` command must contain `--test {expected_target}` exactly once"
        ));
    }
    if expect_no_run && !no_run {
        return Err(format!(
            "step `{step_name}` command must contain `--no-run` exactly once"
        ));
    }
    Ok(())
}

fn validate_execution_acceptance_ci(yaml: &str) -> Result<(), String> {
    let document: Value =
        serde_yaml::from_str(yaml).map_err(|error| format!("invalid workflow YAML: {error}"))?;
    let root = yaml_mapping(&document, "workflow root")?;
    let jobs = yaml_mapping(yaml_field(root, "jobs", "workflow root")?, "jobs")?;
    let job = yaml_mapping(
        yaml_field(jobs, "execution_acceptance", "jobs")?,
        "jobs.execution_acceptance",
    )?;
    if yaml_has_key(job, "if") {
        return Err("jobs.execution_acceptance has a job-level if".to_owned());
    }

    let strategy = yaml_mapping(
        yaml_field(job, "strategy", "jobs.execution_acceptance")?,
        "jobs.execution_acceptance.strategy",
    )?;
    let matrix = yaml_mapping(
        yaml_field(strategy, "matrix", "jobs.execution_acceptance.strategy")?,
        "jobs.execution_acceptance.strategy.matrix",
    )?;
    if yaml_has_key(matrix, "exclude") {
        return Err("jobs.execution_acceptance matrix exclude is forbidden".to_owned());
    }
    let os = yaml_field(matrix, "os", "jobs.execution_acceptance.strategy.matrix")?
        .as_sequence()
        .ok_or_else(|| "jobs.execution_acceptance matrix.os must be a sequence".to_owned())?;
    let actual_os = os
        .iter()
        .enumerate()
        .map(|(index, value)| {
            value.as_str().ok_or_else(|| {
                format!("jobs.execution_acceptance matrix.os[{index}] must be a string")
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let expected_os = ["ubuntu-latest", "macos-latest", "windows-latest"];
    if actual_os.as_slice() != expected_os {
        return Err(format!(
            "jobs.execution_acceptance matrix.os must equal {expected_os:?}, got {actual_os:?}"
        ));
    }

    let runs_on = yaml_field(job, "runs-on", "jobs.execution_acceptance")?
        .as_str()
        .ok_or_else(|| "jobs.execution_acceptance runs-on must be a string".to_owned())?;
    if runs_on != "${{ matrix.os }}" {
        return Err(format!(
            "jobs.execution_acceptance runs-on must equal `${{{{ matrix.os }}}}`, got `{runs_on}`"
        ));
    }

    let steps = yaml_field(job, "steps", "jobs.execution_acceptance")?
        .as_sequence()
        .ok_or_else(|| "jobs.execution_acceptance steps must be a sequence".to_owned())?;
    let step_mappings = steps
        .iter()
        .enumerate()
        .map(|(index, value)| {
            yaml_mapping(value, &format!("jobs.execution_acceptance.steps[{index}]"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let required_steps = [
        (
            "Build execution backend mock",
            "execution_backend_mock",
            true,
        ),
        (
            "Run execution product acceptance",
            "execution_product",
            false,
        ),
        (
            "Run execution protocol host acceptance",
            "execution_protocol_host",
            false,
        ),
        (
            "Run execution runtime acceptance",
            "execution_runtime",
            false,
        ),
    ];
    let mut previous_index = None;
    for (step_name, test_target, expect_no_run) in required_steps {
        let matches = step_mappings
            .iter()
            .enumerate()
            .filter_map(|(index, step)| {
                (yaml_string_field(step, "name") == Some(step_name)).then_some((index, *step))
            })
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return Err(format!(
                "jobs.execution_acceptance must define step `{step_name}` exactly once"
            ));
        }
        let (index, step) = matches[0];
        if previous_index.is_some_and(|previous| index <= previous) {
            return Err(format!(
                "jobs.execution_acceptance step `{step_name}` is out of order"
            ));
        }
        if yaml_has_key(step, "if") {
            return Err(format!(
                "jobs.execution_acceptance step `{step_name}` has a step-level if"
            ));
        }
        let run = yaml_field(
            step,
            "run",
            &format!("jobs.execution_acceptance step `{step_name}`"),
        )?
        .as_str()
        .ok_or_else(|| format!("step `{step_name}` run must be a string"))?;
        validate_cargo_test_command(step_name, run, test_target, expect_no_run)?;
        previous_index = Some(index);
    }
    Ok(())
}

fn validate_release_audit_step(yaml: &str) -> Result<(), String> {
    let document: Value =
        serde_yaml::from_str(yaml).map_err(|error| format!("invalid workflow YAML: {error}"))?;
    let root = yaml_mapping(&document, "workflow root")?;
    if let Some(defaults) = root.get(yaml_key("defaults")) {
        let defaults = yaml_mapping(defaults, "workflow root.defaults")?;
        if let Some(run) = defaults.get(yaml_key("run")) {
            let run = yaml_mapping(run, "workflow root.defaults.run")?;
            if yaml_has_key(run, "shell") {
                return Err("workflow root defaults.run.shell is forbidden".to_owned());
            }
        }
    }
    let jobs = yaml_mapping(yaml_field(root, "jobs", "workflow root")?, "jobs")?;
    let job = yaml_mapping(
        yaml_field(jobs, "sandbox_release_audit", "jobs")?,
        "jobs.sandbox_release_audit",
    )?;
    if yaml_has_key(job, "if") {
        return Err("jobs.sandbox_release_audit has a job-level if".to_owned());
    }
    if yaml_has_key(job, "shell") {
        return Err("jobs.sandbox_release_audit must not define `shell`".to_owned());
    }
    if yaml_has_key(job, "continue-on-error") {
        return Err("jobs.sandbox_release_audit must not define `continue-on-error`".to_owned());
    }
    if let Some(defaults) = job.get(yaml_key("defaults")) {
        let defaults = yaml_mapping(defaults, "jobs.sandbox_release_audit.defaults")?;
        if let Some(run) = defaults.get(yaml_key("run")) {
            let run = yaml_mapping(run, "jobs.sandbox_release_audit.defaults.run")?;
            if yaml_has_key(run, "shell") {
                return Err("jobs.sandbox_release_audit defaults.run.shell is forbidden".to_owned());
            }
        }
    }
    let audit_needs = yaml_field(job, "needs", "jobs.sandbox_release_audit")?
        .as_sequence()
        .ok_or_else(|| "jobs.sandbox_release_audit.needs must be a sequence".to_owned())?;
    let audit_needs = audit_needs
        .iter()
        .map(|value| {
            value.as_str().ok_or_else(|| {
                "jobs.sandbox_release_audit.needs entries must be strings".to_owned()
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let expected_audit_needs = ["sandbox_archive", "sandbox_windows_posture"];
    if audit_needs.as_slice() != expected_audit_needs {
        return Err(format!(
            "jobs.sandbox_release_audit.needs must equal {expected_audit_needs:?}"
        ));
    }
    for dependency in ["build", "sandbox_archive", "sandbox_windows_posture"] {
        yaml_mapping(
            yaml_field(jobs, dependency, "jobs")?,
            &format!("jobs.{dependency}"),
        )?;
    }
    let steps = yaml_field(job, "steps", "jobs.sandbox_release_audit")?
        .as_sequence()
        .ok_or_else(|| "jobs.sandbox_release_audit steps must be a sequence".to_owned())?;
    let matches = steps
        .iter()
        .enumerate()
        .filter_map(|(index, value)| {
            let step = value.as_mapping()?;
            (yaml_string_field(step, "name") == Some("Audit the complete release evidence set"))
                .then_some((index, step))
        })
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err(
            "jobs.sandbox_release_audit must define the named audit step exactly once".to_owned(),
        );
    }
    let (index, step) = matches[0];
    if yaml_has_key(step, "if") {
        return Err(format!(
            "jobs.sandbox_release_audit.steps[{index}] must not define `if`"
        ));
    }
    if yaml_has_key(step, "continue-on-error") {
        return Err(format!(
            "jobs.sandbox_release_audit.steps[{index}] must not define `continue-on-error`"
        ));
    }
    if yaml_has_key(step, "shell") {
        return Err(format!(
            "jobs.sandbox_release_audit.steps[{index}] must not define `shell`"
        ));
    }
    let run = yaml_field(
        step,
        "run",
        &format!("jobs.sandbox_release_audit.steps[{index}]"),
    )?
    .as_str()
    .ok_or_else(|| format!("jobs.sandbox_release_audit.steps[{index}].run must be a string"))?;
    let tokens = run.split_whitespace().collect::<Vec<_>>();
    if tokens.first() != Some(&"python3") {
        return Err(
            "release audit command must start with the explicit `python3` interpreter".to_owned(),
        );
    }
    let expected = [
        "python3",
        "scripts/opi-artifact-audit.py",
        "evidence",
        "--release",
    ];
    if tokens.as_slice() != expected {
        return Err(format!(
            "release audit command must equal `{}`, got `{run}`",
            expected.join(" ")
        ));
    }

    let release = yaml_mapping(yaml_field(jobs, "release", "jobs")?, "jobs.release")?;
    if yaml_has_key(release, "if") {
        return Err("jobs.release has a job-level if".to_owned());
    }
    if yaml_has_key(release, "continue-on-error") {
        return Err("jobs.release must not define `continue-on-error`".to_owned());
    }
    if yaml_has_key(release, "shell") {
        return Err("jobs.release must not define `shell`".to_owned());
    }
    if let Some(defaults) = release.get(yaml_key("defaults")) {
        let defaults = yaml_mapping(defaults, "jobs.release.defaults")?;
        if let Some(run) = defaults.get(yaml_key("run")) {
            let run = yaml_mapping(run, "jobs.release.defaults.run")?;
            if yaml_has_key(run, "shell") {
                return Err("jobs.release defaults.run.shell is forbidden".to_owned());
            }
        }
    }
    let release_needs = yaml_field(release, "needs", "jobs.release")?
        .as_sequence()
        .ok_or_else(|| "jobs.release.needs must be a sequence".to_owned())?;
    let release_needs = release_needs
        .iter()
        .map(|value| {
            value
                .as_str()
                .ok_or_else(|| "jobs.release.needs entries must be strings".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let expected_release_needs = ["build", "sandbox_archive", "sandbox_release_audit"];
    if release_needs.as_slice() != expected_release_needs {
        return Err(format!(
            "jobs.release.needs must equal {expected_release_needs:?}"
        ));
    }
    Ok(())
}

/// Slice one YAML job block: from the `<job>:` key line to the next sibling key
/// (a non-blank line at the same or lesser indentation) or EOF. Job keys in
/// GitHub Actions YAML live under `jobs:` indented two spaces, so the matcher
/// honors the key's actual indentation rather than assuming column 0.
fn job_block(yaml: &str, job_name: &str) -> String {
    let header = format!("{job_name}:");
    let leading_spaces = |line: &str| line.chars().take_while(|c| *c == ' ').count();
    let mut block = String::new();
    let mut started = false;
    let mut indent = 0usize;
    for line in yaml.split_inclusive('\n') {
        let bare = line.trim_end_matches(['\n', '\r']);
        if !started {
            if bare.trim() == header {
                indent = leading_spaces(bare);
                block.push_str(line);
                started = true;
            }
            continue;
        }
        // A non-blank line at indent <= the job key is the next sibling job or a
        // dedent to a top-level key — either ends this block.
        if !bare.trim().is_empty() && leading_spaces(bare) <= indent {
            break;
        }
        block.push_str(line);
    }
    assert!(started, "YAML has no `{header}` job");
    block
}

/// Assert every needle is a substring of haystack (whitespace-normalized so
/// YAML indentation does not fragment the match).
fn assert_present(label: &str, haystack: &str, needles: &[&str]) {
    let norm = normalize_ws(haystack);
    for needle in needles {
        assert!(
            norm.contains(&normalize_ws(needle)),
            "{label}: expected `{needle}` in the topology"
        );
    }
}

/// Assert no needle is a substring of haystack (whitespace-normalized).
fn assert_absent(label: &str, haystack: &str, needles: &[&str]) {
    let norm = normalize_ws(haystack);
    for needle in needles {
        assert!(
            !norm.contains(&normalize_ws(needle)),
            "{label}: `{needle}` must NOT appear in the topology"
        );
    }
}

fn normalize_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Slice one named step from a previously sliced job block. The step begins at
/// `- name: <step_name>` and ends at the next list item at the same indentation.
fn named_step_block(job: &str, step_name: &str) -> String {
    let header = format!("- name: {step_name}");
    let leading_spaces = |line: &str| line.chars().take_while(|c| *c == ' ').count();
    let mut block = String::new();
    let mut started = false;
    let mut indent = 0usize;
    for line in job.split_inclusive('\n') {
        let bare = line.trim_end_matches(['\n', '\r']);
        if !started {
            if bare.trim() == header {
                indent = leading_spaces(bare);
                block.push_str(line);
                started = true;
            }
            continue;
        }
        if !bare.trim().is_empty()
            && leading_spaces(bare) == indent
            && bare.trim_start().starts_with("- ")
        {
            break;
        }
        block.push_str(line);
    }
    assert!(started, "job has no `{header}` step");
    block
}

const CI: &str = ".github/workflows/ci.yml";
const RELEASE: &str = ".github/workflows/release.yml";

const VALID_SEMANTIC_EXECUTION_ACCEPTANCE: &str = r#"
jobs:
  execution_acceptance:
    strategy:
      matrix:
        os:
          - ubuntu-latest
          - macos-latest
          - windows-latest
    runs-on: ${{ matrix.os }}
    steps:
      - name: Build execution backend mock
        run: >-
          cargo test --test execution_backend_mock --no-run
          --features execution-backend-test-fixture -p opi-coding-agent
      - name: Run execution product acceptance
        run: >-
          cargo test --test execution_product
          --features execution-backend-test-fixture -p opi-coding-agent
      - name: Run execution protocol host acceptance
        run: >-
          cargo test --features execution-backend-test-fixture
          -p opi-coding-agent --test execution_protocol_host
      - name: Run execution runtime acceptance
        run: >-
          cargo test -p opi-coding-agent --test execution_runtime
          --features execution-backend-test-fixture
"#;

fn assert_execution_acceptance_error(yaml: &str, expected: &str) {
    let error = validate_execution_acceptance_ci(yaml)
        .expect_err("adversarial execution-acceptance YAML must be rejected");
    assert!(
        error.contains(expected),
        "expected validation error containing `{expected}`, got `{error}`"
    );
}

const VALID_RELEASE_AUDIT_STEP: &str = r#"
jobs:
  build: {}
  sandbox_archive: {}
  sandbox_windows_posture: {}
  sandbox_release_audit:
    needs: [sandbox_archive, sandbox_windows_posture]
    steps:
      - name: Audit the complete release evidence set
        run: >-
          python3 scripts/opi-artifact-audit.py
          evidence --release
  release:
    needs: [build, sandbox_archive, sandbox_release_audit]
    steps: []
"#;

fn assert_release_audit_error(yaml: &str, expected: &str) {
    let error = validate_release_audit_step(yaml)
        .expect_err("adversarial release-audit YAML must be rejected");
    assert!(
        error.contains(expected),
        "expected validation error containing `{expected}`, got `{error}`"
    );
}

// The six Opi release targets that MUST remain published (release.yml `build`).
const SIX_OPI_ARTIFACTS: &[&str] = &[
    "opi-linux-x64",
    "opi-linux-arm64",
    "opi-darwin-x64",
    "opi-darwin-arm64",
    "opi-windows-x64",
    "opi-windows-arm64",
];

// The six triples the `target_check` compile gate must still cover.
const SIX_TARGET_TRIPLES: &[&str] = &[
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "x86_64-pc-windows-msvc",
    "aarch64-pc-windows-msvc",
];

#[test]
fn ci_defines_opi_sandbox_package_job_with_extracted_smoke() {
    let ci = read_repo_file(CI);
    let job = job_block(&ci, "sandbox_package");
    // Builds the standalone opi-sandbox binary (release). Three independent
    // tokens so the assertion survives flag reordering.
    assert_present(
        "ci.sandbox_package",
        &job,
        &["cargo build", "-p opi-sandbox", "--bin opi-sandbox"],
    );
    // Packages via the host-neutral packager.
    assert_present("ci.sandbox_package", &job, &["package-opi-sandbox.sh"]);
    let verify = named_step_block(&job, "Verify native archive");
    assert_present(
        "ci.sandbox_package.verify",
        &verify,
        &["package-opi-sandbox.sh", "--verify"],
    );
    // Smokes the EXTRACTED binary — the provenance marker that distinguishes a
    // release archive from a workspace-only binary.
    assert_present(
        "ci.sandbox_package",
        &job,
        &["opi-sandbox-smoke.sh", "extracted/bin/opi-sandbox"],
    );
}

#[test]
fn ci_opi_sandbox_package_job_runs_linux_and_macos_only() {
    let ci = read_repo_file(CI);
    let job = job_block(&ci, "sandbox_package");
    assert_present(
        "ci.sandbox_package",
        &job,
        &["ubuntu-latest", "macos-latest"],
    );
    // No Windows runner in the opi-sandbox slice (Windows has no native
    // opi-sandbox confinement — 16.14.2 unsupported posture).
    assert_absent("ci.sandbox_package", &job, &["windows-latest"]);
}

#[test]
fn ci_no_longer_has_stale_phase15_sandbox_product_job() {
    let ci = read_repo_file(CI);
    // The whole Phase 15 sandbox_product job (and its scoped sandbox_strict /
    // sandbox_linux_backend acceptance) is removed; its tests are deleted by
    // 16.16.1, so keeping the job would dangle.
    assert_absent("ci", &ci, &["sandbox_product:"]);
}

#[test]
fn ci_retains_target_check_six_target_compile_gate() {
    let ci = read_repo_file(CI);
    let job = job_block(&ci, "target_check");
    for triple in SIX_TARGET_TRIPLES {
        assert_present("ci.target_check", &job, &[*triple]);
    }
}

#[test]
fn ci_runs_feature_gated_execution_acceptance_after_building_mock() {
    let ci = read_repo_file(CI);
    validate_execution_acceptance_ci(&ci)
        .unwrap_or_else(|error| panic!("invalid execution_acceptance topology: {error}"));
}

#[test]
fn execution_acceptance_validator_accepts_semantic_yaml_variants() {
    validate_execution_acceptance_ci(VALID_SEMANTIC_EXECUTION_ACCEPTANCE)
        .expect("block OS list, multiline commands, and reordered flags are valid");
}

#[test]
fn execution_acceptance_validator_rejects_comment_only_matrix_decoy() {
    let yaml = r#"
jobs:
  execution_acceptance:
    # strategy:
    #   matrix:
    #     os: [ubuntu-latest, macos-latest, windows-latest]
    runs-on: ubuntu-latest
    steps: []
"#;
    assert_execution_acceptance_error(yaml, "strategy");
}

#[test]
fn execution_acceptance_validator_rejects_block_scalar_node_decoy() {
    let yaml = r#"
jobs:
  execution_acceptance:
    runs-on: ubuntu-latest
    steps:
      - name: Decoy
        run: |
          strategy:
            matrix:
              os: [ubuntu-latest, macos-latest, windows-latest]
          runs-on: ${{ matrix.os }}
"#;
    assert_execution_acceptance_error(yaml, "strategy");
}

#[test]
fn execution_acceptance_validator_rejects_job_level_if_with_spaced_colon() {
    let mutated = VALID_SEMANTIC_EXECUTION_ACCEPTANCE.replacen(
        "    runs-on: ${{ matrix.os }}",
        "    if : false\n    runs-on: ${{ matrix.os }}",
        1,
    );
    assert_ne!(mutated, VALID_SEMANTIC_EXECUTION_ACCEPTANCE);
    assert_execution_acceptance_error(&mutated, "job-level if");
}

#[test]
fn execution_acceptance_validator_rejects_block_matrix_exclude() {
    let mutated = VALID_SEMANTIC_EXECUTION_ACCEPTANCE.replacen(
        "    runs-on: ${{ matrix.os }}",
        "        exclude:\n          - os: macos-latest\n    runs-on: ${{ matrix.os }}",
        1,
    );
    assert_ne!(mutated, VALID_SEMANTIC_EXECUTION_ACCEPTANCE);
    assert_execution_acceptance_error(&mutated, "matrix exclude");
}

#[test]
fn execution_acceptance_validator_rejects_flow_matrix_exclude() {
    let mutated = VALID_SEMANTIC_EXECUTION_ACCEPTANCE.replacen(
        "      matrix:\n        os:\n          - ubuntu-latest\n          - macos-latest\n          - windows-latest",
        "      matrix: { os: [ubuntu-latest, macos-latest, windows-latest], exclude: [{ os: windows-latest }] }",
        1,
    );
    assert_ne!(mutated, VALID_SEMANTIC_EXECUTION_ACCEPTANCE);
    assert_execution_acceptance_error(&mutated, "matrix exclude");
}

#[test]
fn execution_acceptance_validator_rejects_step_level_if() {
    let mutated = VALID_SEMANTIC_EXECUTION_ACCEPTANCE.replacen(
        "      - name: Run execution product acceptance",
        "      - name: Run execution product acceptance\n        if: false",
        1,
    );
    assert_ne!(mutated, VALID_SEMANTIC_EXECUTION_ACCEPTANCE);
    assert_execution_acceptance_error(&mutated, "step-level if");
}

#[test]
fn execution_acceptance_validator_rejects_extra_matrix_os() {
    let mutated = VALID_SEMANTIC_EXECUTION_ACCEPTANCE.replacen(
        "          - windows-latest",
        "          - windows-latest\n          - freebsd-latest",
        1,
    );
    assert_ne!(mutated, VALID_SEMANTIC_EXECUTION_ACCEPTANCE);
    assert_execution_acceptance_error(&mutated, "matrix.os");
}

#[test]
fn execution_acceptance_validator_rejects_missing_suite() {
    let mutated = VALID_SEMANTIC_EXECUTION_ACCEPTANCE.replacen(
        "      - name: Run execution runtime acceptance\n        run: >-\n          cargo test -p opi-coding-agent --test execution_runtime\n          --features execution-backend-test-fixture\n",
        "",
        1,
    );
    assert_ne!(mutated, VALID_SEMANTIC_EXECUTION_ACCEPTANCE);
    assert_execution_acceptance_error(&mutated, "Run execution runtime acceptance");
}

#[test]
fn execution_acceptance_validator_rejects_feature_after_shell_comment() {
    let mutated = VALID_SEMANTIC_EXECUTION_ACCEPTANCE.replacen(
        "          cargo test --test execution_product\n          --features execution-backend-test-fixture -p opi-coding-agent",
        "          cargo test --test execution_product -p opi-coding-agent # --features execution-backend-test-fixture",
        1,
    );
    assert_ne!(mutated, VALID_SEMANTIC_EXECUTION_ACCEPTANCE);
    assert_execution_acceptance_error(&mutated, "command");
}

#[test]
fn execution_acceptance_validator_rejects_cargo_harness_separator() {
    let mutated = VALID_SEMANTIC_EXECUTION_ACCEPTANCE.replacen(
        "          cargo test -p opi-coding-agent --test execution_runtime\n          --features execution-backend-test-fixture",
        "          cargo test -p opi-coding-agent --test execution_runtime\n          --features execution-backend-test-fixture -- --list",
        1,
    );
    assert_ne!(mutated, VALID_SEMANTIC_EXECUTION_ACCEPTANCE);
    assert_execution_acceptance_error(&mutated, "command");
}

#[test]
fn execution_acceptance_validator_rejects_masked_failure() {
    let mutated = VALID_SEMANTIC_EXECUTION_ACCEPTANCE.replacen(
        "          -p opi-coding-agent --test execution_protocol_host",
        "          -p opi-coding-agent --test execution_protocol_host || true",
        1,
    );
    assert_ne!(mutated, VALID_SEMANTIC_EXECUTION_ACCEPTANCE);
    assert_execution_acceptance_error(&mutated, "command");
}

#[test]
fn execution_acceptance_validator_rejects_extra_shell_command() {
    let mutated = VALID_SEMANTIC_EXECUTION_ACCEPTANCE.replacen(
        "          --features execution-backend-test-fixture -p opi-coding-agent",
        "          --features execution-backend-test-fixture -p opi-coding-agent && echo extra",
        1,
    );
    assert_ne!(mutated, VALID_SEMANTIC_EXECUTION_ACCEPTANCE);
    assert_execution_acceptance_error(&mutated, "command");
}

#[test]
fn execution_acceptance_validator_rejects_duplicate_flag() {
    let mutated = VALID_SEMANTIC_EXECUTION_ACCEPTANCE.replacen(
        "--test execution_backend_mock --no-run",
        "--test execution_backend_mock --no-run --no-run",
        1,
    );
    assert_ne!(mutated, VALID_SEMANTIC_EXECUTION_ACCEPTANCE);
    assert_execution_acceptance_error(&mutated, "command");
}

#[test]
fn execution_acceptance_validator_rejects_extra_positional_token() {
    let mutated = VALID_SEMANTIC_EXECUTION_ACCEPTANCE.replacen(
        "          cargo test -p opi-coding-agent --test execution_runtime\n          --features execution-backend-test-fixture",
        "          cargo test -p opi-coding-agent --test execution_runtime\n          --features execution-backend-test-fixture unexpected",
        1,
    );
    assert_ne!(mutated, VALID_SEMANTIC_EXECUTION_ACCEPTANCE);
    assert_execution_acceptance_error(&mutated, "command");
}

#[test]
fn release_preserves_six_target_opi_build_matrix() {
    let release = read_repo_file(RELEASE);
    let build = job_block(&release, "build");
    for artifact in SIX_OPI_ARTIFACTS {
        assert_present("release.build", &build, &[*artifact]);
    }
}

#[test]
fn release_defines_opi_sandbox_archive_job_linux_macos_only() {
    let release = read_repo_file(RELEASE);
    let job = job_block(&release, "sandbox_archive");
    assert_present("release.sandbox_archive", &job, &["package-opi-sandbox.sh"]);
    for marker in [
        "x86_64-unknown-linux-gnu",
        "ubuntu-24.04",
        "aarch64-unknown-linux-gnu",
        "ubuntu-24.04-arm",
        "x86_64-apple-darwin",
        "macos-15-intel",
        "aarch64-apple-darwin",
        "macos-15",
    ] {
        assert_present("release.sandbox_archive", &job, &[marker]);
    }
    // No Windows opi-sandbox artifact is ever produced.
    assert_absent(
        "release.sandbox_archive",
        &job,
        &["windows-latest", "pc-windows"],
    );
}

#[test]
fn release_audits_all_native_archives_and_windows_posture_before_publish() {
    let release = read_repo_file(RELEASE);
    let windows = job_block(&release, "sandbox_windows_posture");
    assert_absent(
        "release.sandbox_windows_posture",
        &windows,
        &["Add-Content", "supported = false"],
    );
    validate_release_audit_step(&release)
        .unwrap_or_else(|error| panic!("invalid sandbox_release_audit topology: {error}"));
    let audit = job_block(&release, "sandbox_release_audit");
    for target in [
        "x86_64-unknown-linux-gnu",
        "aarch64-unknown-linux-gnu",
        "x86_64-apple-darwin",
        "aarch64-apple-darwin",
    ] {
        assert_present("release.sandbox_release_audit", &audit, &[target]);
    }
    assert_present(
        "release.sandbox_release_audit",
        &audit,
        &[
            "sandbox-evidence-windows",
            "opi-artifact-audit.py",
            "--release",
        ],
    );

    let publish = job_block(&release, "release");
    assert_present("release.release", &publish, &["sandbox_release_audit"]);
}

#[test]
fn release_audit_validator_accepts_semantic_multiline_command() {
    validate_release_audit_step(VALID_RELEASE_AUDIT_STEP)
        .expect("an explicit Python interpreter in the real named step is valid");
}

#[test]
fn release_audit_validator_rejects_omitted_python_interpreter() {
    let mutated = VALID_RELEASE_AUDIT_STEP.replacen(
        "          python3 scripts/opi-artifact-audit.py",
        "          scripts/opi-artifact-audit.py",
        1,
    );
    assert_release_audit_error(&mutated, "python3");
}

#[test]
fn release_audit_validator_rejects_comment_only_interpreter_decoy() {
    let mutated = VALID_RELEASE_AUDIT_STEP.replacen(
        "          python3 scripts/opi-artifact-audit.py",
        "          scripts/opi-artifact-audit.py # python3",
        1,
    );
    assert_release_audit_error(&mutated, "command");
}

#[test]
fn release_audit_validator_rejects_block_scalar_decoy_in_another_step() {
    let yaml = r#"
jobs:
  build: {}
  sandbox_archive: {}
  sandbox_windows_posture: {}
  sandbox_release_audit:
    needs: [sandbox_archive, sandbox_windows_posture]
    steps:
      - name: Decoy
        run: |
          python3 scripts/opi-artifact-audit.py evidence --release
      - name: Audit the complete release evidence set
        run: scripts/opi-artifact-audit.py evidence --release
  release:
    needs: [build, sandbox_archive, sandbox_release_audit]
    steps: []
"#;
    assert_release_audit_error(yaml, "python3");
}

#[test]
fn release_audit_validator_rejects_shell_bypass() {
    let mutated = VALID_RELEASE_AUDIT_STEP.replacen(
        "          python3 scripts/opi-artifact-audit.py\n          evidence --release",
        "          bash -c 'python3 scripts/opi-artifact-audit.py evidence --release'",
        1,
    );
    assert_release_audit_error(&mutated, "command");
}

#[test]
fn release_audit_validator_rejects_job_continue_on_error() {
    let mutated = VALID_RELEASE_AUDIT_STEP.replacen(
        "    steps:",
        "    continue-on-error: ${{ matrix.allow_failure }}\n    steps:",
        1,
    );
    assert_ne!(mutated, VALID_RELEASE_AUDIT_STEP);
    assert_release_audit_error(&mutated, "continue-on-error");
}

#[test]
fn release_audit_validator_rejects_step_continue_on_error() {
    let mutated = VALID_RELEASE_AUDIT_STEP.replacen(
        "      - name: Audit the complete release evidence set",
        "      - name: Audit the complete release evidence set\n        continue-on-error: true",
        1,
    );
    assert_ne!(mutated, VALID_RELEASE_AUDIT_STEP);
    assert_release_audit_error(&mutated, "continue-on-error");
}

#[test]
fn release_audit_validator_rejects_audit_job_if() {
    let mutated = VALID_RELEASE_AUDIT_STEP.replacen(
        "  sandbox_release_audit:\n",
        "  sandbox_release_audit:\n    if: always()\n",
        1,
    );
    assert_release_audit_error(&mutated, "job-level if");
}

#[test]
fn release_audit_validator_rejects_job_shell_override() {
    let mutated = VALID_RELEASE_AUDIT_STEP.replacen(
        "  sandbox_release_audit:\n",
        "  sandbox_release_audit:\n    shell: bash\n",
        1,
    );
    assert_release_audit_error(&mutated, "shell");
}

#[test]
fn release_audit_validator_rejects_step_shell_override() {
    let mutated = VALID_RELEASE_AUDIT_STEP.replacen(
        "      - name: Audit the complete release evidence set",
        "      - name: Audit the complete release evidence set\n        shell: bash",
        1,
    );
    assert_release_audit_error(&mutated, "shell");
}

#[test]
fn release_audit_validator_rejects_workflow_default_shell_override() {
    let mutated = VALID_RELEASE_AUDIT_STEP.replacen(
        "jobs:\n",
        "defaults:\n  run:\n    shell: bash\njobs:\n",
        1,
    );
    assert_release_audit_error(&mutated, "defaults.run.shell");
}

#[test]
fn release_audit_validator_rejects_audit_job_default_shell_override() {
    let mutated = VALID_RELEASE_AUDIT_STEP.replacen(
        "  sandbox_release_audit:\n",
        "  sandbox_release_audit:\n    defaults:\n      run:\n        shell: bash\n",
        1,
    );
    assert_release_audit_error(&mutated, "defaults.run.shell");
}

#[test]
fn release_audit_validator_rejects_incomplete_audit_needs() {
    let mutated = VALID_RELEASE_AUDIT_STEP.replacen(
        "    needs: [sandbox_archive, sandbox_windows_posture]",
        "    needs: [sandbox_archive]",
        1,
    );
    assert_release_audit_error(&mutated, "sandbox_release_audit.needs");
}

#[test]
fn release_audit_validator_rejects_missing_dependency_job() {
    let mutated = VALID_RELEASE_AUDIT_STEP.replacen("  sandbox_windows_posture: {}\n", "", 1);
    assert_release_audit_error(&mutated, "sandbox_windows_posture");
}

#[test]
fn release_audit_validator_rejects_publish_without_audit_dependency() {
    let mutated = VALID_RELEASE_AUDIT_STEP.replacen(
        "    needs: [build, sandbox_archive, sandbox_release_audit]",
        "    needs: [build, sandbox_archive]",
        1,
    );
    assert_release_audit_error(&mutated, "jobs.release.needs");
}

#[test]
fn release_audit_validator_rejects_release_job_if() {
    let mutated =
        VALID_RELEASE_AUDIT_STEP.replacen("  release:\n", "  release:\n    if: always()\n", 1);
    assert_release_audit_error(&mutated, "jobs.release has a job-level if");
}

#[test]
fn release_audit_validator_rejects_release_job_continue_on_error() {
    let mutated = VALID_RELEASE_AUDIT_STEP.replacen(
        "  release:\n",
        "  release:\n    continue-on-error: true\n",
        1,
    );
    assert_release_audit_error(&mutated, "jobs.release");
}

#[test]
fn release_audit_validator_rejects_release_job_shell_override() {
    let mutated =
        VALID_RELEASE_AUDIT_STEP.replacen("  release:\n", "  release:\n    shell: bash\n", 1);
    assert_release_audit_error(&mutated, "jobs.release");
}

#[test]
fn release_audit_validator_rejects_release_job_default_shell_override() {
    let mutated = VALID_RELEASE_AUDIT_STEP.replacen(
        "  release:\n",
        "  release:\n    defaults:\n      run:\n        shell: bash\n",
        1,
    );
    assert_release_audit_error(&mutated, "jobs.release defaults.run.shell");
}

#[test]
fn release_no_windows_opi_sandbox_artifact_name_anywhere() {
    let release = read_repo_file(RELEASE);
    // The opi `build` job legitimately produces opi-windows-* (the main binary);
    // but no opi-sandbox-windows-* artifact name may appear anywhere.
    assert_absent("release", &release, &["opi-sandbox-windows"]);
}

#[test]
fn release_opi_sandbox_smokes_extracted_binary() {
    let release = read_repo_file(RELEASE);
    let job = job_block(&release, "sandbox_archive");
    assert_present(
        "release.sandbox_archive",
        &job,
        &["opi-sandbox-smoke.sh", "extracted/bin/opi-sandbox"],
    );
    let verify = named_step_block(&job, "Verify native archive");
    assert_present(
        "release.sandbox_archive.verify",
        &verify,
        &["package-opi-sandbox.sh", "--verify"],
    );
}

#[test]
fn unix_smoke_names_every_complete_native_acceptance_sentinel() {
    let smoke = read_repo_file("crates/opi-sandbox/scripts/opi-sandbox-smoke.sh");
    for marker in [
        "empty-cwd-smoke-result.txt",
        "setup-failure-smoke-result.txt",
        "filesystem-allow-smoke-result.txt",
        "filesystem-deny-smoke-result.txt",
        "network-deny-smoke-result.txt",
        "network-allow-smoke-result.txt",
    ] {
        assert_present("opi-sandbox-smoke", &smoke, &[marker, "archive_sha256"]);
    }
    assert_present(
        "opi-sandbox-smoke.setup-failure",
        &smoke,
        &[
            "SETUP_TMPDIR_FILE",
            r#"SETUP_NO_START="$WORKSPACE/setup-target-started.txt""#,
            r#"TMPDIR="$SETUP_TMPDIR_FILE""#,
            "/usr/bin/touch",
            "SETUP_CODE",
        ],
    );
    assert_absent(
        "opi-sandbox-smoke.setup-failure",
        &smoke,
        &[
            r#"SETUP_NO_START="$ARTIFACT_DIR/setup-target-started.txt""#,
            "MISSING_WORKSPACE",
            "definitely-not-an-executable",
        ],
    );
    assert_present(
        "opi-sandbox-smoke.network-deny",
        &smoke,
        &[
            "except OSError",
            "BIND_DENIED",
            r#"[ "$NETWORK_DENY_CODE" -eq 23 ]"#,
            r#"grep -q '^BIND_DENIED$' "$ARTIFACT_DIR/network-deny-stdout.txt""#,
            r#"! grep -q 'Traceback' "$ARTIFACT_DIR/network-deny-stderr.txt""#,
        ],
    );
}

#[test]
fn superseded_standalone_sandbox_workflows_are_removed() {
    // sandbox-macos.yml (Phase 15; triggered on sandbox_strict.rs and ran
    // --test sandbox_strict, which 16.16.1 deletes) and sandbox-macos-phase16.yml
    // (the 16.14.1 focused verifier) are both removed once ci.yml carries the
    // opi-sandbox matrix.
    for stale in [
        ".github/workflows/sandbox-macos.yml",
        ".github/workflows/sandbox-macos-phase16.yml",
    ] {
        let path = repo_root().join(stale);
        assert!(
            !path.exists(),
            "stale workflow must be removed: {}",
            path.display()
        );
    }
    // ci.yml's opi-sandbox job carries the macOS coverage they provided.
    let ci = read_repo_file(CI);
    let job = job_block(&ci, "sandbox_package");
    assert_present("ci.sandbox_package", &job, &["macos-latest"]);
}
