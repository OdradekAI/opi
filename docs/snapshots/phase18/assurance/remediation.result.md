# Phase 18 Remediation Result

**Status**: COMPLETE
**Audit index SHA-256**: `325a4665c863139394767d3ce3454e79528b77d0301a2dad621c7330761c987c`
**Plan SHA-256**: `bf5df80cfa4c974996aa569d298534aa0915e9ec468ca189f7d2edf04c364ce1`
**Changed paths**: ["CHANGELOG.md", "crates/opi-coding-agent/tests/config_tests.rs", "crates/opi-eval/docs/seam-evidence-matrix.md", "crates/opi-eval/src/agent/opi.rs", "crates/opi-eval/src/bundle/mod.rs", "crates/opi-eval/src/cli/report.rs", "crates/opi-eval/src/runner/experiment.rs", "crates/opi-eval/tests/report_output_containment.rs", "docs/snapshots/phase18/ci-receipt.json", "scripts/phase18-eval-smoke.ps1", "scripts/test_phase18_eval_smoke.py", "scripts/test_verify_phase18_native_artifact.py", "scripts/verify-phase18-native-artifact.py"]

`COMPLETE` means this apply execution and its machine dispositions are fully
recorded. It does not mean every finding is closed or that Phase 18 conforms.
The apply remained bound to remediation head
`fe6501a389031aa15252f1931b6dbcfe86c4434a`, the approved plan digest above,
and the current audit-index digest above.

## Closure outcomes

- **B1 `bundle.artifact-ancestor-containment` — Closed.** Bundle insertion now
  rejects an existing symlink, Windows junction/reparse point, or non-directory
  ancestor before any target file is created.
- **B2 `bundle.intent-directory-durability` — Closed.** Atomic publication
  syncs the containing directory after rename and propagates an injected
  directory-durability failure before returning durable proof.
- **B3 `agent.opi-phase17-evidence-graph` — Closed.** The Opi importer now
  validates the complete Phase 17 graph, including run, sequence, call,
  parent, kind/payload, and terminal correlations.
- **B4 `runner.hermetic-windows-execution` — Closed.** Windows smoke helpers
  are generated as native PowerShell/Python paths, and the happy, declared
  failure, and offline-report cases pass hermetically.
- **B5 `report.output-ancestor-containment` — Closed.** Report output parents
  are resolved before containment enforcement, so aliases into the run root
  reject before create-new publication.
- **B6 `config-test.ambient-proxy-independence` — Closed.** The no-explicit-
  proxy test no longer asserts that the host environment is proxy-free, while
  the explicit-proxy contract remains covered.
- **B7 `phase18.current-candidate-external-evidence` — Not closed.** The one
  authorized native dispatch failed before artifact upload, so the required
  same-candidate native artifact does not exist and the matrix/terminal-receipt
  pair was not installed.

## B7 external evidence

The immutable remediation candidate is
`1aadfae6589c038954ffec8639fe94e559337fa6` on
`codex/phase18-remediation` and PR #5.

- Same-repository PR CI run `33380387075` completed successfully with 27 jobs.
  Its head, merge-ref checkout, three platform attestation artifacts, downloaded
  Ubuntu artifact digest, and inner receipt passed
  `scripts/verify-phase18-ci.py --terminal` in a temporary output location.
- The sole authorized Linux native-smoke dispatch was run `33380446360`, bound
  to the same candidate. It failed in `Rerun the conformance suites through the
  exact programs` at `benchmark-deepswe-completed`; no artifacts were uploaded.
  The native process exited 0, but `import_pier_job_result` rejected the output
  as `native-output-invalid`. The fixed output exposed valid DeepSWE breakdown
  counts `20` and `3`, while the importer currently applies the aggregate
  zero-or-one reward domain to every metric before selecting the `reward`
  metric. This path was unchanged by B1-B6.
- The one-run ceiling is exhausted. No retry, production repair, matrix update,
  or terminal-receipt installation was performed. The existing matrix and
  committed CI receipt remain historical evidence and do not support a
  current-candidate claim.

Closing B7 now requires a newly approved remediation plan that owns the Pier
multi-metric import defect and separately authorizes another external evidence
cycle. This apply does not grant that authority.

## Bounded incidental repairs

- **I1 (B4)** replaced `Get-FileHash` with the .NET SHA-256 API because the
  Python-launched Windows PowerShell environment did not expose that cmdlet.
- **I2 (B4)** writes JSON with BOM-free UTF-8 so Python can parse smoke output.
- **I3 (B4)** corrected the smoke assertion to derive each bundle identity from
  its own exact manifest bytes; distinct executions retain real time facts.
- **I4 (B7)** uses `sys.executable` for native-artifact verifier subprocesses,
  avoiding a nonexistent `python3` command on Windows.
- **I5 (B7)** normalizes manifest/tar member names to POSIX-relative paths in
  the native-artifact verifier and its tests.

All five incidentals satisfy the bounded-repair guardrails and have focused
FAIL/PASS observations in the machine result dispositions.

## Verification

The approved final union was run at the candidate tree:

```text
cargo test -p opi-eval insertion_rejects_ancestor_directory_alias -- --nocapture             PASS
cargo test -p opi-eval intent_publication_requires_parent_directory_durability -- --nocapture PASS
cargo test -p opi-eval importer_rejects_phase17_invalid_evidence_graphs -- --nocapture         PASS
cargo test -p opi-eval bundle::tests                                                           PASS (9)
cargo test -p opi-eval importer                                                                PASS (6)
cargo test -p opi-eval --test agent_integration_conformance                                    PASS (1)
cargo test -p opi-eval --test report_output_containment                                        PASS (3)
cargo test -p opi-eval --test report_contract                                                  PASS
python scripts/test_phase18_eval_smoke.py                                                      PASS (3)
cargo test -p opi-coding-agent --test config_tests                                              PASS (49)
python scripts/test_verify_phase18_native_artifact.py                                           PASS (15)
python scripts/test_derive_phase18_seam_matrix.py                                               PASS (7)
python scripts/test_verify_phase18_ci.py                                                       PASS (26)
cargo fmt --check --all                                                                        PASS
cargo clippy --workspace --all-targets -- -D warnings                                          PASS
cargo test --workspace --all-targets                                                           PASS
cargo test --workspace --doc                                                                   PASS
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps                                     PASS
python scripts/opi-doc-check.py                                                                PASS
git diff --check                                                                               PASS
```

The first workspace-test attempt observed one two-second RPC receive timeout in
an unchanged test. The exact test passed immediately, the full `rpc_jsonl`
binary passed three consecutive times, and the required full workspace command
then passed; no out-of-scope change was made.

Test impact: `add` and `update`. One report-containment integration test binary
was added; existing bundle, importer, smoke, native-artifact, and configuration
tests were updated. No test uses a paid provider or live credential.

## Materialization boundary

The B1-B6 fixes and approved plan are committed and pushed as candidate
`1aadfae6589c038954ffec8639fe94e559337fa6`. This result is evidence-only and
does not change a runtime, producer, verifier, or workflow input path. The pull
request remains open; nothing was merged and no Phase PASS is claimed.
