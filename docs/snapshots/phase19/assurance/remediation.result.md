# Phase 19 Remediation Result

**Status**: COMPLETE
**Audit index SHA-256**: `7ae158dc8cfbdf0d4ec23497f2d5d34883878156f5eb1f6b83ae07d0ce98dd57`
**Plan SHA-256**: `8c658a0aff8dd0a17e2e971a13abe030ae795e4cd0b14a684dda946fcc0fca20`
**Disposition artifact**: `remediation.result.dispositions.jsonl`
**Changed paths**: [".github/workflows/opi-eval-external-lock-materialization.yml", "CHANGELOG.md", "crates/opi-eval/README.md", "crates/opi-eval/README.zh.md", "crates/opi-eval/external-locks/static/linux-x86_64.json", "crates/opi-eval/integrations/harbor/opi_eval_harbor.py", "crates/opi-eval/integrations/pier/opi_eval_pier.py", "crates/opi-eval/scripts/build-agent-artifacts.sh", "crates/opi-eval/scripts/materialize-external-locks.sh", "crates/opi-eval/scripts/native-smoke.sh", "crates/opi-eval/scripts/test_native_integrations.py", "crates/opi-eval/scripts/test_native_smoke_output_control.py", "crates/opi-eval/scripts/test_verify_external_lock_artifact.py", "crates/opi-eval/scripts/test_verify_external_lock_ci.py", "crates/opi-eval/scripts/test_verify_local_acceptance.py", "crates/opi-eval/scripts/test_verify_native_smoke_artifact.py", "crates/opi-eval/scripts/test_verify_native_smoke_ci.py", "crates/opi-eval/scripts/verify-external-lock-artifact.py", "crates/opi-eval/scripts/verify-external-lock-ci.py", "crates/opi-eval/scripts/verify-local-acceptance.py", "crates/opi-eval/scripts/verify-native-smoke-artifact.py", "crates/opi-eval/src/benchmark/process.rs", "crates/opi-eval/src/cli/conformance.rs", "crates/opi-eval/src/cli/run.rs", "crates/opi-eval/src/experiment.rs", "crates/opi-eval/src/main.rs", "crates/opi-eval/src/native_capture.rs", "crates/opi-eval/src/regrade.rs", "crates/opi-eval/src/runner/experiment.rs", "crates/opi-eval/src/runner/material.rs", "crates/opi-eval/src/runner/plan.rs", "crates/opi-eval/src/runner/prepare.rs", "crates/opi-eval/tests/experiment_contract.rs", "crates/opi-eval/tests/fixtures/agents/opi/capture-thinking.jsonl", "crates/opi-eval/tests/fixtures/agents/pi/capture-thinking.jsonl", "crates/opi-eval/tests/native_driver.rs", "crates/opi-eval/tests/support/native_witness_material.rs"]

All nine dependency-ordered batches were applied under the approved plan
digest at remediation head `31b3ae11095a41f1cbec46c6886b8513de2a292c`, with
the six workflow-maintenance carry-in files preserved unchanged. The eleven
fix records are Closed with observed red-before and green-after evidence;
C005 and C006 remain explicit Cannot confirm exclusions with no production
edit, and the pi registry observation remains Info/No action. Nothing here
closes C005/C006, changes the audit verdict or admits Phase exit.

## Batch outcomes

- **B1** (C/P19-AUD-001): positive `max_output_bytes` now rejects at
  resolution in all three modes through `UnenforceableExecutionBound`;
  contract fixtures synchronized; bilingual READMEs and CHANGELOG updated.
- **B2** (C/P19-AUD-002): the native handoff stage uses an exclusive 0700
  directory and 0600 files for credentials; promoted host-privacy probe and
  lib unit test.
- **B3** (C/P19-AUD-004): the local-acceptance verifier admits trial ids and
  one pair-agreed contained family root; every consumer reuses the selected
  root and the read seam admits only bounded relative paths.
- **B4** (C/P19-AUD-009 + P/P19-AUD-001): the output-control smoke fixture
  drives a real synthetic uv executable, mocked current prepare, validated
  `--native-material` argv and the repository cwd.
- **B7** (C/P19-AUD-011): optional high reasoning projects Unsupported with
  the native-execution rationale instead of Effective.
- **B8** (C/P19-AUD-003, D1): fresh native dispatch filters private thinking
  at the first capture sink through the pinned musl capture helper
  (material/7, handoff/4, terminal/3, capture receipts, partial-capture/2),
  with strict dual-grammar admission, refusal without a native score on
  unknown/duplicate/invalid/over-bound input, receipt-bound re-verification
  before completion/sealing/partial copies, and legacy material/6 plus
  terminal/2 preserved read-only for sealed offline verification.
- **B5** (C/P19-AUD-008 + P/P19-AUD-002): rustfmt-clean assertion.
- **B6** (C/P19-AUD-010): runner and CLI Rustdoc state the explicit
  diagnostic oracle and the hermetic/native-material split truthfully.
- **B9** (C/P19-AUD-007, D4): the default external-lock materializer
  delegates preparation and the explicit oracle-witness conformance to the
  shared CLI and publishes receipt/resolved /2 only after both succeed;
  transport-only closure provenance, the inspected image graph, and the
  pinned-toolchain workflow byte pins are retained.

## Verification union observed at the final boundary

`cargo fmt --check --all`; `cargo clippy --workspace --all-targets -- -D
warnings`; every opi-eval test target (lib 172, native_driver 112, contract
and integration suites); full script discovery `272 tests OK` (one optional
docker skip); `python3 scripts/opi-doc-check.py`; `git diff --check`; both
retained B8 probes (thinking-persistence marker_paths=0; first-sink probe
8/8 cases, 0 leaking); the B9 delegation/refusal probe exit 0; the B3
acceptance probes 3/3; static-lock producer and workflow pins re-verified
in sync. Two ambient, remediation-independent flakes were observed and
reported, not fixed: the known scripted-provider loopback listener under
full workspace discovery, and `mutation_lock_times_out_under_contention`
in the untouched opi-coding-agent crate under heavy load; both pass
repeatedly standalone. Sustained full-workspace test runs crashed the WSL
host twice (SIGBUS/E_FAIL), so the consolidated workspace run was completed
crate-by-crate with identical binary coverage. `cargo test --workspace
--doc` and the workspace rustdoc gate were not re-observed after the final
B9 bytes and remain the only unexecuted union members; no Rust source
changed in B9 after the B8-boundary runs that included them.

## Scope notes

Three B8-planned paths needed no change and are attributed unchanged:
`crates/opi-eval/src/regrade.rs` (the legacy read-only boundary needs no
regrade-seam edit), `crates/opi-eval/scripts/build-agent-artifacts.sh`
(helper acquisition lives in prepare's cold build), and
`crates/opi-eval/tests/support/native_witness_material.rs` (the ignored
opt-in witness filters compile unchanged against the legacy admission
path). Conversely the B8 implementation added
`crates/opi-eval/src/native_capture_runtime.rs` (the hidden capture-helper
mode, isolated from the lib's no-unsafe boundary) and registered the parser
module via `crates/opi-eval/src/lib.rs`; both lie inside B8's causal
surface and are carried in the working tree. The machine-checked `Changed
paths` array above equals the disposition-record union exactly as the
contract requires.

No commit, branch, push or release was made; authorization for any of those
remains with the user. C005/C006 follow-up evidence routes remain subject
to their actual material/privacy/count/deadline admission.
