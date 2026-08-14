//! Shared Phase 17 helpers for the task 17.9 capstone acceptance tests.
//!
//! Each task 17.9 binary pulls this in with:
//! ```text
//! #[path = "common/phase17.rs"]
//! mod phase17;
//! ```
//! The `#[path]` form compiles this file as a per-binary module without editing
//! `tests/common/mod.rs`, so no `common/mod.rs` ownership is claimed. Like
//! `common/mod.rs`, this module is compiled once per binary that includes it and
//! never participates in the published crate surface.
//!
//! Bodies are kept byte-identical to the per-binary copies in the 17.4-17.8
//! phase17 tests they replace.

#![allow(dead_code)]

use std::sync::{Arc, Mutex, MutexGuard};

/// Serialize `OPI_SESSIONS_DIR` mutation across one test binary. A `static` in
/// this per-binary module is itself per-binary, so tests in different binaries
/// (separate processes) never contend on it.
static SESSION_TEST_LOCK: Mutex<()> = Mutex::new(());

/// Acquire this binary's session-env lock for the whole body of a test that sets
/// `OPI_SESSIONS_DIR` (including across `.await` — annotate the test with
/// `#[allow(clippy::await_holding_lock)]`).
pub fn session_lock() -> MutexGuard<'static, ()> {
    SESSION_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Point `OPI_SESSIONS_DIR` at `dir`. Caller must hold `session_lock()`.
pub fn set_sessions_dir(dir: &std::path::Path) {
    // SAFETY: process-global env mutation serialized by SESSION_TEST_LOCK.
    unsafe {
        std::env::set_var("OPI_SESSIONS_DIR", dir);
    }
}

/// Remove `OPI_SESSIONS_DIR`. Caller must hold `session_lock()`.
pub fn clear_sessions_dir() {
    // SAFETY: process-global env mutation serialized by SESSION_TEST_LOCK.
    unsafe {
        std::env::remove_var("OPI_SESSIONS_DIR");
    }
}

/// A static API-key auth resolver for test routes (byte-identical to the
/// per-binary copies in 17.7/17.8).
pub fn static_resolver() -> Arc<dyn opi_ai::auth::AuthResolver> {
    Arc::new(opi_ai::auth::StaticAuthResolver::new(
        opi_ai::auth::AuthScheme::ApiKey,
        secrecy::SecretString::from("opi-test-auth"),
    ))
}

/// A `ModelInfo` whose id and display name are both `id`, on the OpenAI wire
/// with a 100k/4k capability envelope (byte-identical to the 17.7/17.8 copy).
pub fn model_info(id: &str) -> opi_ai::provider::ModelInfo {
    opi_ai::provider::ModelInfo::new(
        id,
        id,
        opi_ai::WireApi::OpenAiCompletions,
        opi_ai::ModelCapabilities::new(100_000, 4_096),
    )
}

/// Parse NDJSON output (one JSON value per non-empty line), panicking on any
/// malformed line. Byte-identical to the copy in `tests/json_mode.rs`.
pub fn parse_ndjson(output: &str) -> Vec<serde_json::Value> {
    output
        .lines()
        .filter(|l| !l.is_empty())
        .map(|line| serde_json::from_str(line).unwrap_or_else(|_| panic!("invalid JSON: {line}")))
        .collect()
}

/// Resolve the canonical artifact directory under the workspace `target/` for
/// task 17.9 (mirrors the 17.7/17.8 `artifact_dir` pattern).
pub fn artifact_dir() -> std::path::PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_owned());
    std::path::Path::new(&manifest_dir).join("../../target/opi-artifacts/phase17-task-17.9")
}

/// The most recently modified `.jsonl` file directly under `dir` (skipping
/// subdirectories — see the tests-tree read_dir guard), if any.
pub fn newest_jsonl(dir: &std::path::Path) -> Option<std::path::PathBuf> {
    let mut newest: Option<(std::time::SystemTime, std::path::PathBuf)> = None;
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let path = entry.path();
        if path.is_dir() || path.extension().is_none_or(|e| e != "jsonl") {
            continue;
        }
        if let Ok(modified) = entry.metadata().and_then(|m| m.modified())
            && newest.as_ref().is_none_or(|(t, _)| modified > *t)
        {
            newest = Some((modified, path));
        }
    }
    newest.map(|(_, p)| p)
}

/// Verify `SHA256SUMS.txt` in `dir` against a clean re-read of every listed raw
/// file, asserting each entry is present, non-empty (unless exempt), and
/// digest-matched. Mirrors the 17.7/17.8 verification block.
pub fn verify_sha256sums(dir: &std::path::Path, may_be_empty: &[&str]) {
    use sha2::{Digest, Sha256};
    let sums = std::fs::read_to_string(dir.join("SHA256SUMS.txt")).unwrap();
    for line in sums.lines() {
        let Some((digest, name)) = line.split_once("  ") else {
            panic!("malformed SHA256SUMS entry: {line}");
        };
        let path = dir.join(name);
        assert!(path.exists(), "required artifact missing: {name}");
        let bytes = std::fs::read(&path).unwrap();
        if !may_be_empty.contains(&name) {
            assert!(!bytes.is_empty(), "artifact must be non-empty: {name}");
        }
        let fresh = hex::encode(Sha256::digest(&bytes));
        assert_eq!(digest, fresh, "SHA256SUMS digest mismatch for {name}");
    }
}

/// Write `SHA256SUMS.txt` over every named raw file in `dir`, asserting each is
/// present and non-empty (unless in `may_be_empty`).
pub fn write_sha256sums(dir: &std::path::Path, names: &[&str], may_be_empty: &[&str]) {
    use sha2::{Digest, Sha256};
    use std::io::Write;
    let mut sorted: Vec<&str> = names.to_vec();
    sorted.sort_unstable();
    let mut sha_lines: Vec<String> = Vec::new();
    for name in &sorted {
        let path = dir.join(name);
        assert!(path.exists(), "required artifact missing: {name}");
        let bytes = std::fs::read(&path).unwrap();
        if !may_be_empty.contains(name) {
            assert!(!bytes.is_empty(), "artifact must be non-empty: {name}");
        }
        sha_lines.push(format!("{}  {}", hex::encode(Sha256::digest(&bytes)), name));
    }
    let mut f = std::fs::File::create(dir.join("SHA256SUMS.txt")).unwrap();
    writeln!(f, "{}", sha_lines.join("\n")).unwrap();
}
