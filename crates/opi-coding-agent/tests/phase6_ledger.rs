use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};

/// Mirrors `phase4_ledger.rs` but targets the Phase 6 snapshot ledger.
///
/// Unlike Phase 4, no historical test pinned the Phase 6 snapshot's spec hash;
/// it was kept in sync only by convention, which let it drift silently. This
/// guard ensures `docs/snapshots/phase6/opi-impl-state.json` tracks the same
/// CRLF-normalized SHA-256 of `docs/opi-spec.md` that the Phase 4 ledger pins,
/// so any `docs/opi-spec.md` edit (version bump or content change) forces a
/// Phase 6 snapshot re-sync or this test fails.
#[test]
fn phase6_ledger_spec_hash_matches_current_spec() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let spec_path = repo_root.join("docs/opi-spec.md");
    let ledger_path = repo_root.join("docs/snapshots/phase6/opi-impl-state.json");

    let spec = fs::read_to_string(&spec_path).expect("read docs/opi-spec.md");
    let normalized_spec = spec.replace("\r\n", "\n");
    let actual = format!("{:x}", Sha256::digest(normalized_spec.as_bytes()));

    let ledger_bytes =
        fs::read(&ledger_path).expect("read docs/snapshots/phase6/opi-impl-state.json");
    let ledger: serde_json::Value =
        serde_json::from_slice(&ledger_bytes).expect("parse phase6 ledger");
    let recorded = ledger["spec_files_sha256"]["docs/opi-spec.md"]
        .as_str()
        .expect("phase6 ledger records docs/opi-spec.md hash");

    assert_eq!(
        recorded, actual,
        "phase6 ledger spec hash for docs/opi-spec.md is stale (must mirror phase4)"
    );
}
