# Phase 18 Remediation Plan

**Status**: DRAFT-UNRESOLVED
**Audit index SHA-256**: `60931e9889c2ee28896758522ca2073dcb48b2fc92e15dde510d6cdf71e9e815`
**Remediation head**: `dbd8728ddeb931969fad4c6bc14d6b0104ef67d2`
**Disposition artifact**: `remediation.plan.dispositions.jsonl`
**Dirty-worktree baseline**: staged=[]; unstaged=[]; untracked=[]
**Unresolved decisions**: D2

## Current Finding Verification

| Source run / Finding ID | Verification | Source and final severity + rationale | Closure key/family | Batch | Decision |
|---|---|---|---|---|---|
| `phase18-codex-gpt56-dd7eda7-20260831t135641z` / `P18-AUD-001` | Confirmed at remediation head. The committed CI receipt still binds `0f5a3fa152b12d7be4036b2a08ae7a195f8c2107`, the seam matrix still binds native candidate `27344e3aaf03d38eaa53c7af19c777efbe9be213`, and scoped Phase 18 runtime/evidence paths changed after both candidates. Changes after the Codex audit head are assurance-only and provide no counter-evidence. | Major -> Major. Exact-current-candidate real-Agent, native-benchmark, seam-matrix, and three-platform evidence remains absent. | `phase18.exact-head-native-platform-evidence` / `phase18.assurance-evidence` | B2 | `fix:refresh-exact-candidate-native-platform-evidence` |
| `phase18-pi-glm53-25d0e68-20260831t124752z` / `P18-AUD-001` | Confirmed at remediation head. Removing the 14 blanket suppressions in the sealed archive makes `cargo clippy -p opi-eval --all-targets -- -D warnings` fail with 100 dead-code errors; `external_lock` has no in-crate consumer beyond a rustdoc link. | Minor -> Minor. The dead surface is advisory but materially masks which provisional Phase 18 contracts are executable. | `opi-eval.dead-surface-explicit-and-consumed` / `opi-eval.provisional-surface` | pending D2 | `pending:D2-decide-remove-or-integrate-dead-surface` |
| `phase18-pi-glm53-25d0e68-20260831t124752z` / `P18-AUD-002` | Confirmed at remediation head. The crate-level entry-seam sentence names only `ResolvedExperiment` and `cli::validate`, while `main.rs` consumes the other public CLI functions and integration tests consume additional public experiment types. | Minor -> Minor. The inaccurate enumeration understates the current provisional public surface. | `opi-eval.public-entry-doc-accurate` / `opi-eval.provisional-surface` | B1 | `fix:describe-current-provisional-public-surface` |

## Unresolved Decisions

| ID | Required decision | Why evidence cannot decide | Alternatives | Authority needed |
|---|---|---|---|---|
| D2 | Decide whether each currently unconsumed Phase 18 Rust surface is removed as non-production code or integrated into the real Eval execution path. | Current evidence proves the surface is unused, but the registered Phase 18 source deliberately names several failure, integrity, and supply-chain states. Deleting them may lower required meaning; wiring them in changes runtime behavior and requires a new discriminating closure design. Blanket or crate-wide suppression is not a closure alternative. | Remove the unconsumed private code and update affected tests/contracts; integrate the required states at their owning runtime boundaries; or revise the registered source before narrowing the vocabulary. | Evaluation product owner and registered Phase 18 source owner. |

## Closure Batches

### Batch B1: Make the provisional public entry-seam documentation accurate

**Closure predicate**: Crate-level documentation describes the current provisional public surface without claiming that an incomplete two-item list is exhaustive.
**Dependencies**: none
**Verification union**: `python scripts/opi-doc-check.py`; `git diff --check`; `RUSTDOCFLAGS="-D warnings" cargo doc -p opi-eval --no-deps`

#### Fix B1.1: Replace the incomplete public-seam enumeration

- **Finding source(s)**: `phase18-pi-glm53-25d0e68-20260831t124752z` + `6e88c9234110dd5341bf8b4ea37ddc3157e6db0d67129be33c519e5141f60841` + `P18-AUD-002`
- **Decision**: `fix:describe-current-provisional-public-surface`
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/src/lib.rs`
- **Change kind**: documentation
- **Change**: Replace the exclusive two-item enumeration with an accurate description of the provisional CLI and experiment-resolution exports actually consumed by the same-package binary and integration tests. Do not stabilize, rename, or broaden any Rust API.
- **Closure predicate**: A reader cannot infer that `ResolvedExperiment` and `cli::validate` are the only public entries, and generated rustdoc remains warning-free.
- **Red-before**: `rg -n 'minimum entry seam|required by the same-package CLI|pub fn (validate_native|run|regrade|report|conformance)' crates/opi-eval/src/lib.rs crates/opi-eval/src/cli crates/opi-eval/src/main.rs` -> FAIL: the documentation presents an incomplete exhaustive list while the additional public commands are consumed.
- **Green-after**: `python scripts/opi-doc-check.py && git diff --check && RUSTDOCFLAGS="-D warnings" cargo doc -p opi-eval --no-deps` -> expected PASS after source inspection confirms the enumeration is no longer incomplete.

### Batch B2: Refresh exact-candidate native and three-platform evidence

**Closure predicate**: After B1 and the D2-selected closure are committed as one exact candidate, one successful Linux native-smoke artifact and one successful three-platform CI run both bind that candidate; the native verifier passes all criteria, the seam matrix is derived from that artifact, and the terminal CI receipt is generated from that run.
**Dependencies**: B1 and the closure batch admitted by the resolved D2 decision; local verification must be green before external publication.
**Verification union**: `python scripts/test_verify_phase18_native_artifact.py`; `python scripts/test_derive_phase18_seam_matrix.py`; `python scripts/test_verify_phase18_ci.py`; exact-candidate native artifact verification; exact-candidate matrix derivation/verification; exact-candidate terminal CI receipt verification; `python scripts/opi-doc-check.py`; `git diff --check`

#### Fix B2.1: Materialize exact-candidate runtime evidence

- **Finding source(s)**: `phase18-codex-gpt56-dd7eda7-20260831t135641z` + `dfb38314eca4a05e99e10a42bbbd594a91c413caafd90226a0ab2375ca9788ef` + `P18-AUD-001`
- **Decision**: `fix:refresh-exact-candidate-native-platform-evidence`
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/docs/seam-evidence-matrix.md`, `docs/snapshots/phase18/ci-receipt.json`
- **Change kind**: metadata
- **Change**: Under a later exact-digest apply approval, publish the fully green remediation candidate, run the registered Linux native-smoke and ordinary three-platform CI paths, download their exact receipts/artifacts, verify them independently, and regenerate only the two derived tracked evidence files. Do not substitute an ancestor artifact or hand-edit derived evidence.
- **Closure predicate**: Both derived files bind the same exact candidate containing every approved remediation change, and there is no later material Phase 18 runtime/evidence delta.
- **Red-before**: `python scripts/verify-phase18-ci.py --terminal --receipt docs/snapshots/phase18/ci-receipt.json --expected-head dbd8728ddeb931969fad4c6bc14d6b0104ef67d2 --repo .` fails because the receipt binds `0f5a3fa152b12d7be4036b2a08ae7a195f8c2107`; the committed matrix independently binds native candidate `27344e3aaf03d38eaa53c7af19c777efbe9be213`.
- **Green-after**: `python scripts/verify-phase18-native-artifact.py --criterion all-native --expected-commit <candidate-sha> --receipt <native-receipt> --artifact <native-artifact> --repo . && python scripts/derive-phase18-seam-matrix.py --receipt <native-receipt> --artifact <native-artifact> --require-trajectory-spans --output crates/opi-eval/docs/seam-evidence-matrix.md --verify --repo . --expected-commit <candidate-sha> && python scripts/verify-phase18-ci.py --terminal --receipt docs/snapshots/phase18/ci-receipt.json --expected-head <candidate-sha> --repo .` -> expected PASS.

## Final Verification

    python scripts/opi-doc-check.py
    git diff --check
    RUSTDOCFLAGS="-D warnings" cargo doc -p opi-eval --no-deps
    python scripts/test_verify_phase18_native_artifact.py
    python scripts/test_derive_phase18_seam_matrix.py
    python scripts/test_verify_phase18_ci.py

No apply verification union is admitted for D2 until its owning authority resolves it and a new plan run seals the resulting exact commands, paths, closure batch, and union with B1 and B2.

## Exclusions

| Finding ID | Disposition | Current evidence/authority |
|---|---|---|
| none | none | All three current source keys are retained above; one remains pending explicit authority and two have bounded planned fixes. |
