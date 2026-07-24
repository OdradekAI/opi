//! Live-ledger spec-hash drift guard.
//!
//! `docs/snapshots/phaseN/opi-impl-state.json` are phase-exit *archives*: their
//! `spec_files_sha256` records the spec a phase was built against and must be
//! allowed to drift as the spec evolves. The previous `phase4_ledger` /
//! `phase6_ledger` guards pinned those archives to the *current* spec, which
//! turned frozen files into living fields and forced a re-sync on every
//! unrelated `docs/opi-spec.md` edit. They have been removed.
//!
//! The current spec hash is tracked in the live, git-tracked root ledger
//! `.opi-impl-state.json` instead. This guard keeps that one field honest: any
//! `docs/opi-spec.md` edit must be reflected there or this test fails. Phase
//! snapshots stay historical; the live ledger carries the present.
//!
//! The hash is CRLF-normalized (replace `\r\n` with `\n` before SHA-256) so it
//! is stable across Windows CRLF and Unix LF checkouts; the `opi-implement`
//! skill writes the same normalized convention into `spec_files_sha256`.

use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};

#[test]
fn live_ledger_spec_hash_matches_current_spec() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let spec_path = repo_root.join("docs/opi-spec.md");
    let ledger_path = repo_root.join(".opi-impl-state.json");

    let spec = fs::read_to_string(&spec_path).expect("read docs/opi-spec.md");
    let normalized_spec = spec.replace("\r\n", "\n");
    let actual = format!("{:x}", Sha256::digest(normalized_spec.as_bytes()));

    let ledger_bytes = fs::read(&ledger_path).expect("read .opi-impl-state.json");
    let ledger: serde_json::Value =
        serde_json::from_slice(&ledger_bytes).expect("parse live ledger");
    let recorded = ledger["spec_files_sha256"]["docs/opi-spec.md"]
        .as_str()
        .expect("live ledger records docs/opi-spec.md hash");

    assert_eq!(
        recorded, actual,
        "live ledger spec hash for docs/opi-spec.md is stale; re-sync \
         .opi-impl-state.json after editing docs/opi-spec.md"
    );
}
