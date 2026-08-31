# Phase 18 Remediation Result

**Status**: COMPLETE
**Audit index SHA-256**: `60931e9889c2ee28896758522ca2073dcb48b2fc92e15dde510d6cdf71e9e815`
**Plan SHA-256**: `e779477af9d61528ff71d0195f02478f12e1a28eaa7353ec2fe80ff85506fb49`
**Changed paths**: ["crates/opi-eval/docs/seam-evidence-matrix.md", "crates/opi-eval/src/agent/opi.rs", "crates/opi-eval/src/agent/process.rs", "crates/opi-eval/src/authority.rs", "crates/opi-eval/src/benchmark/deepswe.rs", "crates/opi-eval/src/benchmark/process.rs", "crates/opi-eval/src/benchmark/terminal_bench_21.rs", "crates/opi-eval/src/benchmark/terminal_bench_30.rs", "crates/opi-eval/src/bundle/mod.rs", "crates/opi-eval/src/external_lock.rs", "crates/opi-eval/src/failure.rs", "crates/opi-eval/src/integrity.rs", "crates/opi-eval/src/lib.rs", "crates/opi-eval/src/runner/experiment.rs", "crates/opi-eval/src/runner/lifecycle.rs", "crates/opi-eval/src/runner/material.rs", "crates/opi-eval/src/trajectory/mod.rs", "docs/snapshots/phase18/ci-receipt.json"]

## Outcome

- B1 is closed. The blanket dead-code masks and unused Rust lock module were removed, intentional retained contract fields use narrow reasoned expectations, and the focused Rust/Python checks pass.
- B2 is closed. The crate documentation now describes the provisional `cli` and `experiment` module seam without an incomplete item enumeration, and rustdoc passes with warnings denied.
- B3 is not closed. Pull request CI run `33423008524` passed all 27 jobs for candidate `e2e225fa0665c737542f71aefa27c963daf2bf73`, and the fixed terminal receipt was refreshed from that run. Native-smoke run `33423005233` failed before artifact sealing because `trial-opi-deepswe-v1.1` did not produce the required `answer.txt`; no fresh native artifact exists and the seam matrix was therefore not regenerated.

## Materialization

The approved B1/B2 candidate was published as commits `23d20837830a1062d15fb486b5aca0b7db26b0b9` and the bounded follow-up `e2e225fa0665c737542f71aefa27c963daf2bf73` on `codex/phase18-remediation-e779477`. Pull request #6 targets that exact final candidate.

The first pull-request CI run exposed a Unix library-test dead-code diagnostic for the retained benchmark cleanup evidence. Incidental repair I1 made its existing item-local expectation unconditional; replacement CI then passed on Ubuntu, macOS, and Windows.

## Scope Stop

The native failure is not caused by B1/B2: the same `expected agent output is unreadable` failure occurred in native run `33388338497` at pre-remediation commit `96f4429c094d5c69034bad1f456c55af58537cd7`, while the B1 change to `runner/experiment.rs` only deleted an unused cancellation constant. Repairing the failure would require changing native Agent/output behavior or the reserved trial-evidence contract. That is outside the approved causal scope and may change durable-format or authority semantics, so the bounded-incidental guardrails reject an in-place repair. A new `mode=plan` invocation is required before B3 can be closed.

## Verification

Passed before materialization:

- `cargo fmt --check --all`
- `cargo clippy -p opi-eval --all-targets -- -D warnings`
- `cargo test -p opi-eval --all-targets`
- Phase 18 native/CI verifier and seam-matrix Python unit suites
- `RUSTDOCFLAGS="-D warnings" cargo doc -p opi-eval --no-deps`
- `python scripts/opi-doc-check.py`
- `git diff --check`

External evidence:

- Pull request CI `33423008524`: PASS, 27/27 jobs, candidate `e2e225fa0665c737542f71aefa27c963daf2bf73`.
- Terminal receipt generation: PASS for the same candidate.
- Native smoke `33423005233`: FAIL in `Run the six paired agent trials`; no sealed artifact or upload receipt.
- Earlier native smoke `33388338497`: the same failure at pre-remediation commit `96f4429c094d5c69034bad1f456c55af58537cd7`.

The final full repository union and exact-candidate `all-native`/seam-matrix checks were not run to completion because B3 stopped at the native producer failure. No Phase PASS or implementation-ledger handoff is claimed.
