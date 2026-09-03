# Phase 18 Remediation Plan

**Status**: READY-FOR-APPLY
**Audit index SHA-256**: `480a0535bd2a90fa94b4a8ea13f0030c4b1de4a8546f2ae4e3800eea2d81e6cf`
**Remediation head**: `c8d6dea9901845d8510c15cf079cc065c7140540`
**Disposition artifact**: `remediation.plan.dispositions.jsonl`
**Dirty-worktree baseline**: staged: none; unstaged: docs/snapshots/phase18/assurance/audit.codex.gpt56.findings.jsonl, docs/snapshots/phase18/assurance/audit.codex.gpt56.md, docs/snapshots/phase18/assurance/audit.codex.gpt56.meta.json, docs/snapshots/phase18/assurance/audit.codex.gpt56.requirements.jsonl, docs/snapshots/phase18/assurance/audit.index.json; untracked: docs/snapshots/phase18/assurance/history/phase18-codex-gpt56-0ea6188-20260901t111622z-048ef0c0/audit.codex.gpt56.findings.jsonl, docs/snapshots/phase18/assurance/history/phase18-codex-gpt56-0ea6188-20260901t111622z-048ef0c0/audit.codex.gpt56.md, docs/snapshots/phase18/assurance/history/phase18-codex-gpt56-0ea6188-20260901t111622z-048ef0c0/audit.codex.gpt56.meta.json, docs/snapshots/phase18/assurance/history/phase18-codex-gpt56-0ea6188-20260901t111622z-048ef0c0/audit.codex.gpt56.requirements.jsonl
**Unresolved decisions**: none

## Current Finding Verification

| Source run / Finding ID | Verification | Source and final severity + rationale | Closure key/family | Batch | Decision |
|---|---|---|---|---|---|
| phase18-codex-gpt56-c8d6dea-20260902t181312z / P18-AUD-001 | Confirmed | Source Major; final Major. At the remediation head, read_trial_ids converts failure to read the trials directory into an empty set, and the production CLI consequently emits outcome=verified with exit 0 for a nonexistent root. | eval.regrade-root-read-fails-closed / eval.evidence-boundary-fail-closed | B1 | fix:propagate-regrade-root-read-errors |
| phase18-codex-gpt56-c8d6dea-20260902t181312z / P18-AUD-002 | Confirmed | Source Major; final Major. At the remediation head, run_trial creates staging directories and writes bench.toml before checking the durable intent reservation; the focused test proves a refused reuse replaces sentinel bytes. | eval.trial-identity-refusal-before-staging / eval.trial-evidence-immutability | B2 | fix:reject-reused-trial-before-staging |

## Unresolved Decisions

None.

## Closure Batches

### Batch B1: Fail closed when the regrade trial root cannot be enumerated

**Closure predicate**: Regrading a missing or unreadable trials root returns the existing cannot-read-run-root error at the CLI boundary, emits no verified report, and exits nonzero.
**Dependencies**: none
**Verification union**: focused cross-platform CLI regression; opi-eval all-target tests; affected-target clippy; formatting; documentation contract; diff whitespace check.

#### Fix B1.1: Propagate regrade root enumeration failures

- **Finding source(s)**: phase18-codex-gpt56-c8d6dea-20260902t181312z + 87120dade0b2d3ff79b1a2c83fd950a4d7a22df5b526af0b1344878ed84c0146 + P18-AUD-001
- **Decision**: fix:propagate-regrade-root-read-errors
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/src/regrade.rs`, `crates/opi-eval/src/cli/regrade.rs`, `crates/opi-eval/tests/fail_closed_cli.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Make trial enumeration and offline regrade return I/O failure instead of defaulting to an empty trial set; propagate that failure through the already-declared RegradeCliError::Io boundary; add a cross-platform production-binary regression and an Unreleased fix note.
- **Closure predicate**: A nonexistent run root is rejected with the cannot-read-run-root diagnostic and exit code 2, with no JSON success report.
- **Red-before**: `cargo test -p opi-eval --test fail_closed_cli unreadable_regrade_root_fails_closed -- --exact` -> FAIL (exit 101): expected exit Some(2), observed Some(0), with stdout reporting an empty verified result.
- **Green-after**: `cargo test -p opi-eval --test fail_closed_cli unreadable_regrade_root_fails_closed -- --exact` -> PASS.

### Batch B2: Reject an existing trial identity before staging

**Closure predicate**: A run request whose trial already has a durable intent reservation returns the typed identity-reuse refusal without changing any file or directory under that trial root.
**Dependencies**: none
**Verification union**: focused cross-platform CLI regression; opi-eval all-target tests; affected-target clippy; formatting; documentation contract; diff whitespace check.

#### Fix B2.1: Move the durable reservation guard before trial staging

- **Finding source(s)**: phase18-codex-gpt56-c8d6dea-20260902t181312z + 87120dade0b2d3ff79b1a2c83fd950a4d7a22df5b526af0b1344878ed84c0146 + P18-AUD-002
- **Decision**: fix:reject-reused-trial-before-staging
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/src/runner/experiment.rs`, `crates/opi-eval/tests/fail_closed_cli.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Check bundle/intent.json immediately after deriving the trial root and before creating directories or writing bench.toml; add a cross-platform crash-after-intent regression that snapshots the complete existing trial tree before the refused retry and an Unreleased fix note.
- **Closure predicate**: The second invocation exits 2 with the durable-reservation diagnostic and its before/after trial-tree snapshots are byte-for-byte and path-for-path identical.
- **Red-before**: `cargo test -p opi-eval --test fail_closed_cli reused_trial_identity_is_rejected_before_staging -- --exact` -> FAIL (exit 101): the refused retry replaced the sentinel bench.toml while returning the expected identity-reuse error.
- **Green-after**: `cargo test -p opi-eval --test fail_closed_cli reused_trial_identity_is_rejected_before_staging -- --exact` -> PASS.

## Final Verification

    cargo test -p opi-eval --test fail_closed_cli unreadable_regrade_root_fails_closed -- --exact
    cargo test -p opi-eval --test fail_closed_cli reused_trial_identity_is_rejected_before_staging -- --exact
    cargo fmt --check --all
    cargo clippy -p opi-eval --all-targets -- -D warnings
    cargo test -p opi-eval --all-targets
    python scripts/opi-doc-check.py
    git diff --check -- CHANGELOG.md crates/opi-eval/src/cli/regrade.rs crates/opi-eval/src/regrade.rs crates/opi-eval/src/runner/experiment.rs crates/opi-eval/tests/fail_closed_cli.rs docs/snapshots/phase18/assurance/remediation.plan.md docs/snapshots/phase18/assurance/remediation.plan.dispositions.jsonl

## Exclusions

None.
