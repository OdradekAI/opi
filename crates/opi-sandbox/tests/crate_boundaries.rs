//! Crate-boundary invariants for the standalone `opi-sandbox` SDK (Phase 16 task 16.11.1).
//!
//! The DoD requires the crate to depend on neither `opi-agent` nor
//! `opi-coding-agent` and to read no Opi configuration, sessions, or package
//! storage. The strong structural proof is `cargo tree -p opi-sandbox --edges
//! normal` (the resolve graph has no `opi-agent`/`opi-coding-agent` edge; the
//! sole opi-internal dep is the pure-types `opi-protocol`). The secondary guard
//! asserts the library source calls no host-environment-read API, which is a
//! necessary condition for reading any `OPI_*` configuration env var.

use std::path::PathBuf;
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The resolve graph depends on `opi-protocol` and has NO `opi-agent` or
/// `opi-coding-agent` edge (runtime/normal edges; dev-deps excluded).
#[test]
fn depends_only_on_neutral_crates_not_opi_agent_or_coding_agent() {
    let output = Command::new(env!("CARGO"))
        .args(["tree", "-p", env!("CARGO_PKG_NAME"), "--edges", "normal"])
        .current_dir(manifest_dir())
        .output()
        .expect("cargo tree must run");
    assert!(
        output.status.success(),
        "cargo tree failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let tree = String::from_utf8_lossy(&output.stdout);
    assert!(
        tree.contains("opi-protocol"),
        "crate must depend on opi-protocol;\n{tree}"
    );
    for forbidden in ["opi-agent", "opi-coding-agent"] {
        assert!(
            !tree.contains(forbidden),
            "forbidden transitive dependency `{forbidden}` present:\n{tree}"
        );
    }
}

/// Static tripwire: the library source calls no host-environment-read API
/// (`std::env::*`, `dotenvy`). This is NOT the load-bearing proof — the
/// structural proof that the crate reads no Opi configuration is
/// `depends_only_on_neutral_crates_not_opi_agent_or_coding_agent` (no
/// `opi-agent`/`opi-coding-agent` dependency; the sole opi-internal dep is the
/// pure-types `opi-protocol`). This tripwire catches a future direct env read
/// in the library source; it would not catch an indirect read inside a
/// dependency (the dependency boundary does).
#[test]
fn lib_source_calls_no_host_environment_read_api() {
    let src = manifest_dir().join("src");
    let mut hits = String::new();
    for entry in std::fs::read_dir(&src).expect("read src dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        for needle in ["env::", "dotenvy"] {
            if content.contains(needle) {
                hits.push_str(&format!("{}: `{needle}`\n", path.display()));
            }
        }
    }
    assert!(
        hits.is_empty(),
        "host-environment-read API found in library source:\n{hits}"
    );
}
