//! Cross-platform fail-closed regressions over the production `opi-eval` binary.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fixtures_dir() -> PathBuf {
    manifest_dir().join("tests/fixtures")
}

fn run_experiment(root: &Path, behavior: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("run")
        .arg("--config")
        .arg(fixtures_dir().join("experiment/local-paired.toml"))
        .arg("--root")
        .arg(root.canonicalize().unwrap())
        .arg("--fixtures")
        .arg(fixtures_dir().canonicalize().unwrap())
        .args(["--behavior", behavior])
        .output()
        .expect("spawn the opi-eval run command")
}

fn snapshot_tree(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn visit(root: &Path, path: &Path, snapshot: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        let mut entries: Vec<_> = std::fs::read_dir(path)
            .unwrap()
            .map(|entry| entry.unwrap())
            .collect();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            let relative = path.strip_prefix(root).unwrap().to_owned();
            if entry.file_type().unwrap().is_dir() {
                snapshot.insert(relative, None);
                visit(root, &path, snapshot);
            } else {
                snapshot.insert(relative, Some(std::fs::read(&path).unwrap()));
            }
        }
    }

    let mut snapshot = BTreeMap::new();
    visit(root, root, &mut snapshot);
    snapshot
}

#[test]
fn unreadable_regrade_root_fails_closed() {
    let parent = tempfile::tempdir().unwrap();
    let missing_root = parent.path().canonicalize().unwrap().join("missing-run");
    assert!(!missing_root.exists());

    let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("regrade")
        .arg("--root")
        .arg(&missing_root)
        .output()
        .expect("spawn the opi-eval regrade command");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(
        output.status.code(),
        Some(2),
        "a missing run root must be rejected: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.trim().is_empty(),
        "no report may be published: {stdout}"
    );
    assert!(stderr.contains("cannot read run root"), "{stderr}");
}

#[test]
fn reused_trial_identity_is_rejected_before_staging() {
    let root = tempfile::tempdir().unwrap();
    let crashed = run_experiment(root.path(), "crash-after-intent");
    assert_eq!(
        crashed.status.code(),
        Some(70),
        "the seed run must stop after durable intent: stdout={} stderr={}",
        String::from_utf8_lossy(&crashed.stdout),
        String::from_utf8_lossy(&crashed.stderr)
    );

    let trial_root = root.path().join("trials/trial-opi-1");
    assert!(trial_root.join("bundle/intent.json").is_file());
    std::fs::write(trial_root.join("bench.toml"), b"sentinel-opi\n").unwrap();
    let before = snapshot_tree(&trial_root);

    let reused = run_experiment(root.path(), "happy");
    let stdout = String::from_utf8_lossy(&reused.stdout);
    let stderr = String::from_utf8_lossy(&reused.stderr);
    assert_eq!(
        reused.status.code(),
        Some(2),
        "identity reuse must be rejected: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stderr.contains("already has a durable intent reservation"),
        "{stderr}"
    );
    assert_eq!(
        snapshot_tree(&trial_root),
        before,
        "a refused retry must preserve every existing path and byte"
    );
}
