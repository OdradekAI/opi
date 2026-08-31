# Phase 18 Remediation Result

**Status**: COMPLETE
**Audit index SHA-256**: `60931e9889c2ee28896758522ca2073dcb48b2fc92e15dde510d6cdf71e9e815`
**Plan SHA-256**: `dc7779cbb7d531672fb661f43f89d82956180d7f501c8eeefc60fef45606f36f`
**Changed paths**: ["crates/opi-eval/src/cli/conformance.rs", "crates/opi-eval/tests/native_driver.rs", "docs/snapshots/phase18/ci-receipt.json", "scripts/phase18-scripted-provider.py", "scripts/test_phase18_scripted_provider.py"]

## Outcome

- B1 is closed. The scripted provider now emits the terminal SSE finish separately, its bash arguments are valid JSON, and native Opi conformance enables the mutating tool required to create the promised final-workspace output.
- Native conformance refuses a successful process that omits or empties `answer.txt`; the focused negative and positive native-driver cases pass.
- The two current pi/GLM findings remain Refuted exactly as planned: focused clippy and rustdoc checks pass without restoring either cited condition.
- Incidental repair I1 is closed. The first repaired native run exposed malformed scripted bash arguments inside the already-owned provider surface; a focused JSON-decoding test reproduced the failure before the one-character escaping repair and passed afterward.

## Materialization and Remote Evidence

The implementation candidate is `3b4a39d92338f8cf159296f43c4b8e60809aacc7` on `codex/phase18-remediation-e779477` and pull request #6.

- Pull-request CI run `33441533936` completed successfully with 27/27 jobs. Terminal receipt generation verified the single-stream Ubuntu attestation download and wrote `docs/snapshots/phase18/ci-receipt.json` for the same candidate.
- Linux native-smoke run `33441557309` completed successfully. The downloaded 11.26 GB sealed artifact and upload-identity receipt passed `all-native` verification for P18-A02, P18-A03, P18-A04, P18-A08, P18-A09, P18-A10, P18-A12, and BMK-003, with the evidence classified as conformance-only.
- Relative to the implementation candidate, the evidence commit changes only `docs/snapshots/phase18/ci-receipt.json`, `docs/snapshots/phase18/assurance/remediation.result.md`, and `docs/snapshots/phase18/assurance/remediation.result.dispositions.jsonl`. It contains no Phase 18 runtime, provider, adapter, verifier, workflow, or test change.

## Verification

Focused and producer/verifier checks passed:

- `python -m unittest scripts.test_phase18_scripted_provider.ScriptedProviderListenerTest.test_streaming_tool_call_turn_is_deterministic`
- `python scripts/test_phase18_scripted_provider.py` (15 tests)
- `cargo test -p opi-eval --test native_driver native_conformance_reruns_the_admitted_cases_through_the_material -- --exact` under WSL (1 passed)
- `cargo test -p opi-eval --test native_driver` under WSL (10 passed)
- `cargo test -p opi-eval --test agent_integration_conformance` (1 passed)
- `cargo clippy -p opi-eval --all-targets -- -D warnings`
- `RUSTDOCFLAGS="-D warnings" cargo doc -p opi-eval --no-deps`
- `python scripts/test_verify_phase18_native_ci.py` (34 tests)
- `python scripts/test_verify_phase18_native_artifact.py` (15 tests)
- `python scripts/test_verify_phase18_ci.py` (26 tests)

The workspace verification union passed:

- `cargo fmt --check --all`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace --all-targets`
- `cargo test --workspace --doc`
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`
- `python scripts/opi-doc-check.py`
- `git diff --check`

The first all-targets attempt encountered one unrelated timing-test timeout; that exact test then passed five consecutive focused reruns, and the complete all-targets command passed on rerun. The Windows native-artifact invocation could not unpack one deeply nested Linux virtual-environment path; the same repository verifier and downloaded artifact then passed under WSL/Linux path semantics.

No Phase PASS or implementation-ledger update is claimed. A fresh audit or owning-workflow return remains a separate explicit action.
