# Phase 18 Remediation Plan

**Status**: READY-FOR-APPLY
**Audit index SHA-256**: `60931e9889c2ee28896758522ca2073dcb48b2fc92e15dde510d6cdf71e9e815`
**Remediation head**: `916153a487105cdcc98336841512fbd09897188e`
**Disposition artifact**: `remediation.plan.dispositions.jsonl`
**Dirty-worktree baseline**: staged `[]`; unstaged `[]`; untracked `[]`
**Unresolved decisions**: none

## Current Finding Verification

| Source run / Finding ID | Verification | Source and final severity + rationale | Closure key/family | Batch | Decision |
|---|---|---|---|---|---|
| `phase18-codex-gpt56-dd7eda7-20260831t135641z` / `P18-AUD-001` | Confirmed at `916153a487105cdcc98336841512fbd09897188e`. The retained Linux artifact binds candidate `27344e3aaf03d38eaa53c7af19c777efbe9be213`; the retained three-platform receipt binds candidate `0f5a3fa152b12d7be4036b2a08ae7a195f8c2107`. The native-evidence comparison to the remediation head still spans 62 scoped files, 5,885 insertions, and 742 deletions. | Major → Major. The registered Phase requires real-Agent, three-native-revision, artifact-derived seam, and three-platform evidence after the material implementation state being accepted. Local Windows tests and stored older receipts do not establish that closure. | `phase18.post-change-terminal-evidence` / `phase18.exit-evidence` | B3 | `fix:materialize-post-cleanup-candidate-and-refresh-terminal-evidence` |
| `phase18-pi-glm53-25d0e68-20260831t124752z` / `P18-AUD-001` | Confirmed at `916153a487105cdcc98336841512fbd09897188e`. Removing the fourteen blanket `#[allow(dead_code)]` attributes in the sealed archive makes affected-target clippy fail with 100 library dead-code errors and 36 library-test dead-code errors. The complete 2,238-line `external_lock.rs` module has no in-crate consumer; its only outside reference is a rustdoc link. | Minor → Minor. The behavior remains green only because broad module-level suppression hides an extensive unconsumed surface. The finding is advisory but current and reproducible. | `opi-eval.dead-code-mask` / `opi-eval.surface-discipline` | B1 | `fix:remove-blanket-mask-delete-redundant-lock-module-and-localize-intentional-exceptions` |
| `phase18-pi-glm53-25d0e68-20260831t124752z` / `P18-AUD-002` | Confirmed at `916153a487105cdcc98336841512fbd09897188e`. The crate-level documentation enumerates only `ResolvedExperiment` and `cli::validate`, while `pub mod cli`, `pub mod experiment`, the same-package binary, and integration tests consume the wider provisional module surfaces. | Minor → Minor. The inaccurate enumeration understates the current unpublished entry seam and can mislead reviewers even though it changes no runtime behavior. | `opi-eval.crate-entry-doc` / `opi-eval.surface-discipline` | B2 | `fix:document-provisional-cli-and-experiment-module-seam-without-item-enumeration` |

## Unresolved Decisions

None. The user explicitly authorized the existing Phase 18 materialization path:
commit and publish the post-B1/B2 candidate, run the Linux native-smoke and
three-platform pull-request CI against that exact candidate, verify and retain
the resulting evidence, and hand the validated result to the separately owned
implementation-ledger refresh. This plan does not revise the registered Phase
source or treat tree equivalence as a substitute for committed-candidate
evidence.

## Closure Batches

### Batch B1: Remove blanket dead-code masking and redundant Rust lock validation

**Closure predicate**: `opi-eval` contains no blanket `#[allow(dead_code)]` module suppression, the unconsumed Rust `external_lock` implementation is absent, every remaining intentionally compile-time-unused contract item has a narrow reasoned annotation at that item, and affected-target clippy plus the authoritative external-lock verifier suites pass.
**Dependencies**: none
**Verification union**: blanket-suppression/module-declaration source scan; `cargo fmt --check --all`; `cargo clippy -p opi-eval --all-targets -- -D warnings`; `cargo test -p opi-eval --all-targets`; Phase 18 materialization/native artifact verifier unit suites; `git diff --check`.

#### Fix B1.1: Expose and dispose the actual dead surface

- **Finding source(s)**: `phase18-pi-glm53-25d0e68-20260831t124752z` + `6e88c9234110dd5341bf8b4ea37ddc3157e6db0d67129be33c519e5141f60841` + `P18-AUD-001`
- **Decision**: `fix:remove-blanket-mask-delete-redundant-lock-module-and-localize-intentional-exceptions`
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/src/lib.rs`, `crates/opi-eval/src/external_lock.rs` (delete), `crates/opi-eval/src/benchmark/process.rs`, and the clippy-reported residual files `authority.rs`, `failure.rs`, `integrity.rs`, `bundle/mod.rs`, `runner/experiment.rs`, `runner/lifecycle.rs`, `runner/material.rs`, `agent/opi.rs`, `agent/process.rs`, `benchmark/deepswe.rs`, `benchmark/terminal_bench_21.rs`, `benchmark/terminal_bench_30.rs`, and `trajectory/mod.rs`.
- **Change kind**: behavioral
- **Change**: Remove the fourteen module-level suppressions. Delete the wholly unconsumed Rust `external_lock` module and replace its stale rustdoc reference with the actual external-lock ownership in `crates/opi-eval/external-locks/` and the Phase 18 verifier scripts. Delete unused private helpers/getters/constants that add no contract value. Keep specification-required closed states and schema-presence fields only with item-local `#[expect(dead_code, reason = "...")]` or underscore/serde-renamed validation-only fields, so each intentional exception names its current contract instead of masking a module. Do not add a public seam, feature flag, compatibility path, dependency, or second lock authority.
- **Closure predicate**: The source scan finds no `#[allow(dead_code)]` in `lib.rs` or `failure.rs` and no `mod external_lock`; clippy reports no warning; Rust and Python verifier tests retain the existing lock, failure, integrity, bundle, runner, adapter, and report behavior.
- **Red-before**: In the sealed archive, mechanically remove every `#[allow(dead_code)]` line from `crates/opi-eval/src/lib.rs` and `failure.rs`, then run `cargo clippy -p opi-eval --all-targets -- -D warnings`; observed FAIL with 100 library errors and 36 library-test errors, including the entire `external_lock.rs` surface.
- **Green-after**: Run the same source scan and `cargo clippy -p opi-eval --all-targets -- -D warnings`; expected no blanket mask/module declaration and PASS, followed by the focused Rust and Python verifier suites.

### Batch B2: Make the provisional library-entry documentation accurate

**Closure predicate**: The crate-level documentation describes `cli` and `experiment` as the provisional unpublished module entry seam used by the same-package binary and integration tests, without claiming that two selected items are the complete surface.
**Dependencies**: B1, because both batches edit `crates/opi-eval/src/lib.rs`
**Verification union**: source/declaration review; `RUSTDOCFLAGS="-D warnings" cargo doc -p opi-eval --no-deps`; `python scripts/opi-doc-check.py`; `git diff --check`.

#### Fix B2.1: Replace the stale item enumeration with module-level wording

- **Finding source(s)**: `phase18-pi-glm53-25d0e68-20260831t124752z` + `6e88c9234110dd5341bf8b4ea37ddc3157e6db0d67129be33c519e5141f60841` + `P18-AUD-002`
- **Decision**: `fix:document-provisional-cli-and-experiment-module-seam-without-item-enumeration`
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/src/lib.rs`
- **Change kind**: documentation
- **Change**: Replace the two-item enumeration with stable module-level wording that names the provisional `cli` and `experiment` entry modules, their same-package consumers, unpublished status, and lack of compatibility promise. Do not enumerate individual public functions or types.
- **Closure predicate**: The crate docs no longer state that `ResolvedExperiment` and `cli::validate` are the complete entry seam, and rustdoc resolves without warnings.
- **Red-before**: Compare `crates/opi-eval/src/lib.rs:11-14` with public declarations in `cli/`, `experiment.rs`, and consumers in `main.rs`/integration tests; observed FAIL because the prose lists two items while the public provisional module surface is wider.
- **Green-after**: Run `RUSTDOCFLAGS="-D warnings" cargo doc -p opi-eval --no-deps` and review the module-level wording against the public declarations; expected PASS and no exhaustive item claim.

### Batch B3: Materialize and attest the post-cleanup candidate

**Closure predicate**: One committed candidate containing B1/B2 is the exact
`expected-commit` accepted by a fresh Phase 18 `all-native` artifact
verification and the exact `expected-head` accepted by a fresh three-platform
terminal CI receipt; the seam-evidence matrix is regenerated from that same
native artifact, and neither evidence path substitutes stored output, a
fixture, tree equivalence, or an older candidate.
**Dependencies**: B1 and B2
**Verification union**: static native/CI producer checks; native and terminal
verifier unit suites; exact-candidate `all-native` artifact verification;
artifact-derived seam-matrix verification; exact-candidate terminal receipt
generation; documentation check; full workspace gates; `git diff --check`.

#### Fix B3.1: Produce fresh native and three-platform terminal evidence

- **Finding source(s)**: `phase18-codex-gpt56-dd7eda7-20260831t135641z` + `dfb38314eca4a05e99e10a42bbbd594a91c413caafd90226a0ab2375ca9788ef` + `P18-AUD-001`
- **Decision**: `fix:materialize-post-cleanup-candidate-and-refresh-terminal-evidence`
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/docs/seam-evidence-matrix.md`, `docs/snapshots/phase18/ci-receipt.json`
- **Change kind**: metadata
- **Change**: After B1/B2 and their local gates pass, create one explicit remediation commit and publish it on an authorized branch. Dispatch `.github/workflows/phase18-native-smoke.yml` with that full commit as `candidate_sha`; download the workflow's receipt and sealed artifact through one verified stream; require `verify-phase18-native-artifact.py --criterion all-native` to accept that exact commit; and regenerate `crates/opi-eval/docs/seam-evidence-matrix.md` from those bytes with `derive-phase18-seam-matrix.py --verify`. Open the authorized pull request for the same candidate, require every ordinary and attestation job to succeed, download the run/job/artifact/inner-receipt metadata, and let `verify-phase18-ci.py --terminal` write the fixed terminal receipt with that exact candidate as `--expected-head`. Keep downloaded multi-gigabyte artifacts outside the repository. After result validation, report the separately authorized `opi-implement` ledger/snapshot refresh as the materialization handoff; do not invoke that skill or edit its ledger in remediation apply.
- **Closure predicate**: The fresh `all-native` verification, regenerated seam matrix, and terminal receipt all name and validate the identical committed B1/B2 candidate, and all full repository gates pass over the resulting repository state.
- **Red-before**: `git diff --shortstat 27344e3..916153a487105cdcc98336841512fbd09897188e -- crates/opi-eval/src crates/opi-eval/tests scripts/phase18-native-smoke.sh scripts/verify-phase18-native-artifact.py .github/workflows/phase18-native-smoke.yml crates/opi-eval/external-locks` plus inspection of `docs/snapshots/phase18/ci-receipt.json`; observed FAIL because the native comparison contains 62 changed files (5,885 insertions, 742 deletions) and the terminal receipt binds `0f5a3fa152b12d7be4036b2a08ae7a195f8c2107`, not the remediation state.
- **Green-after**: With `candidate` set to the full authorized B1/B2 remediation commit and `native_receipt`, `native_artifact`, `run_metadata`, `jobs_metadata`, `artifact_metadata`, and `inner_receipt` set to the single-stream downloaded workflow outputs, run `python scripts/verify-phase18-native-artifact.py --criterion all-native --expected-commit $candidate --receipt $native_receipt --artifact $native_artifact --repo .`, `python scripts/derive-phase18-seam-matrix.py --receipt $native_receipt --artifact $native_artifact --require-trajectory-spans --output crates/opi-eval/docs/seam-evidence-matrix.md --verify --repo . --expected-commit $candidate`, and `python scripts/verify-phase18-ci.py --terminal --expected-head $candidate --run-metadata $run_metadata --jobs-metadata $jobs_metadata --artifact-metadata $artifact_metadata --inner-receipt $inner_receipt --output docs/snapshots/phase18/ci-receipt.json --repo .`; expected PASS with identical candidate identity throughout.

## Final Verification

Run the local checks before materializing B3:

    rg -n '^\s*#\[allow\(dead_code\)\]|^mod external_lock;' crates/opi-eval/src/lib.rs crates/opi-eval/src/failure.rs
    cargo fmt --check --all
    cargo clippy -p opi-eval --all-targets -- -D warnings
    cargo test -p opi-eval --all-targets
    python scripts/test_verify_phase18_native_ci.py
    python scripts/test_verify_phase18_materialization_artifact.py
    python scripts/test_verify_phase18_native_artifact.py
    python scripts/test_derive_phase18_seam_matrix.py
    python scripts/test_verify_phase18_ci.py
    RUSTDOCFLAGS="-D warnings" cargo doc -p opi-eval --no-deps
    python scripts/opi-doc-check.py
    git diff --check

The first `rg` command is expected to return no matches. After the exact B3
commands above accept the materialized candidate, run the full repository
union against the resulting state:

    python scripts/opi-doc-check.py
    cargo fmt --check --all
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace --all-targets
    cargo test --workspace --doc
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
    git diff --check

## Exclusions

None. All three current source keys are retained in closure batches. Textual ID
similarity and the other peer's `met` states do not substitute for any current
`(audit_run_id, findings_sha256, id)` identity, and no history source was
consulted.
