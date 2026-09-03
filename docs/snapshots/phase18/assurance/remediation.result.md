# Phase 18 Remediation Result

**Status**: COMPLETE
**Audit index SHA-256**: `480a0535bd2a90fa94b4a8ea13f0030c4b1de4a8546f2ae4e3800eea2d81e6cf`
**Plan SHA-256**: `28527db2fa4ba63221640ec2bacfb244b873cbee356abf4fe4d356e1217e06b5`
**Changed paths**: ["CHANGELOG.md","crates/opi-eval/src/cli/regrade.rs","crates/opi-eval/src/regrade.rs","crates/opi-eval/src/runner/experiment.rs","crates/opi-eval/tests/fail_closed_cli.rs"]

## Closure Batches

### B1: Fail closed when the regrade trial root cannot be enumerated

**Status**: Closed

The offline regrader now propagates trial-directory enumeration errors through
the existing `RegradeCliError::Io` boundary. The production CLI returns exit 2
with `cannot read run root` and does not publish a verified JSON report.

### B2: Reject an existing trial identity before staging

**Status**: Closed

The runner now checks the durable intent reservation immediately after deriving
the trial root. A refused retry returns exit 2 without changing any path or byte
under the existing trial tree.

## Verification

    cargo test -p opi-eval --test fail_closed_cli unreadable_regrade_root_fails_closed -- --exact
    cargo test -p opi-eval --test fail_closed_cli reused_trial_identity_is_rejected_before_staging -- --exact
    cargo fmt --check --all
    cargo clippy -p opi-eval --all-targets -- -D warnings
    cargo test -p opi-eval --all-targets
    python scripts/opi-doc-check.py
    git diff --check -- CHANGELOG.md crates/opi-eval/src/cli/regrade.rs crates/opi-eval/src/regrade.rs crates/opi-eval/src/runner/experiment.rs crates/opi-eval/tests/fail_closed_cli.rs docs/snapshots/phase18/assurance/remediation.plan.md docs/snapshots/phase18/assurance/remediation.plan.dispositions.jsonl

No incidental repairs were required.
