//! Cross-platform report-output containment regressions .

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fixtures_dir() -> PathBuf {
    manifest_dir().join("tests/fixtures")
}

fn seed_run(root: &Path) {
    let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("run")
        .arg("--config")
        .arg(fixtures_dir().join("experiment/local-paired.toml"))
        .arg("--root")
        .arg(root.canonicalize().unwrap())
        .arg("--fixtures")
        .arg(fixtures_dir().canonicalize().unwrap())
        .args(["--behavior", "happy"])
        .output()
        .expect("spawn the opi-eval run binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "seed run must succeed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn report(root: &Path, out: &Path, current_dir: Option<&Path>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_opi-eval"));
    command
        .arg("report")
        .arg("--root")
        .arg(root)
        .arg("--out")
        .arg(out);
    if let Some(current_dir) = current_dir {
        command.current_dir(current_dir);
    }
    command.output().expect("spawn opi-eval report")
}

#[cfg(windows)]
fn create_directory_alias(link: &Path, target: &Path) {
    let output = Command::new("cmd")
        .args([
            "/D",
            "/C",
            "mklink",
            "/J",
            &link.to_string_lossy(),
            &target.to_string_lossy(),
        ])
        .output()
        .expect("spawn mklink for report containment regression");
    assert!(
        output.status.success(),
        "create report-output junction: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[cfg(unix)]
fn create_directory_alias(link: &Path, target: &Path) {
    std::os::unix::fs::symlink(target, link).expect("create report-output symlink");
}

#[test]
fn output_rejects_ancestor_alias_into_run_root() {
    let root = tempfile::tempdir().unwrap();
    seed_run(root.path());
    let canonical_root = root.path().canonicalize().unwrap();
    let bundle = canonical_root.join("trials/trial-opi-1/bundle");
    let aliases = tempfile::tempdir().unwrap();
    let alias = aliases.path().join("into-run");
    create_directory_alias(&alias, &bundle);
    let out = alias.join("unmanifested-report.json");
    let in_bundle = bundle.join("unmanifested-report.json");

    let output = report(&canonical_root, &out, None);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(2),
        "ancestor alias into the run root must be rejected: stdout={} stderr={stderr}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(stderr.contains("inside the run root"), "{stderr}");
    assert!(!out.exists(), "the alias target must not be created");
    assert!(
        !in_bundle.exists(),
        "no unmanifested report may appear inside the sealed bundle"
    );
}

#[test]
fn output_rejects_relative_in_root_target() {
    let root = tempfile::tempdir().unwrap();
    seed_run(root.path());
    let canonical_root = root.path().canonicalize().unwrap();
    let parent = canonical_root.parent().unwrap();
    let relative = canonical_root
        .strip_prefix(parent)
        .unwrap()
        .join("relative-report.json");

    let output = report(&canonical_root, &relative, Some(parent));
    assert_eq!(output.status.code(), Some(2));
    assert!(!canonical_root.join("relative-report.json").exists());
}

#[test]
fn output_accepts_one_fresh_target_under_resolved_external_parent() {
    let root = tempfile::tempdir().unwrap();
    seed_run(root.path());
    let canonical_root = root.path().canonicalize().unwrap();
    let outputs = tempfile::tempdir().unwrap();
    let out = outputs.path().join("report.json");

    let first = report(&canonical_root, &out, None);
    assert_eq!(
        first.status.code(),
        Some(0),
        "fresh external target must succeed: stdout={} stderr={}",
        String::from_utf8_lossy(&first.stdout),
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(out.is_file());

    let second = report(&canonical_root, &out, None);
    assert_eq!(second.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&second.stderr).contains("already exists"));
}
