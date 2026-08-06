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
//! test), structurally sliced by top-level job key; they are not source-text
//! tautologies. The packager's own layout/lock/extraction contract is pinned
//! independently by `opi_sandbox_packaging.rs` (16.15.1) and is not duplicated.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_repo_file(relative: &str) -> String {
    let path = repo_root().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
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

fn assert_feature_gated_test_step(step: &str, target: &str, must_not_run: bool) {
    assert_present(
        target,
        step,
        &[
            "cargo test",
            "-p opi-coding-agent",
            "--features execution-backend-test-fixture",
            &format!("--test {target}"),
        ],
    );
    if must_not_run {
        assert_present(target, step, &["--no-run"]);
    } else {
        assert_absent(target, step, &["--no-run"]);
    }
}

const CI: &str = ".github/workflows/ci.yml";
const RELEASE: &str = ".github/workflows/release.yml";

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
    let job = job_block(&ci, "execution_acceptance");
    assert_present("ci.execution_acceptance", &job, &["ubuntu-latest"]);

    let build_name = "Build execution backend mock";
    let build = named_step_block(&job, build_name);
    assert_feature_gated_test_step(&build, "execution_backend_mock", true);

    let acceptance_steps = [
        ("Run execution product acceptance", "execution_product"),
        (
            "Run execution protocol host acceptance",
            "execution_protocol_host",
        ),
        ("Run execution runtime acceptance", "execution_runtime"),
    ];
    let build_position = job
        .find(&format!("- name: {build_name}"))
        .expect("build step is present");
    for (step_name, target) in acceptance_steps {
        let step = named_step_block(&job, step_name);
        assert_feature_gated_test_step(&step, target, false);
        let run_position = job
            .find(&format!("- name: {step_name}"))
            .expect("acceptance step is present");
        assert!(
            build_position < run_position,
            "execution_backend_mock must be built before `{target}` runs"
        );
    }
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
    let smoke = read_repo_file("scripts/opi-sandbox-smoke.sh");
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
