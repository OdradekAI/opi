# Phase 18 Remediation Plan

**Status**: READY-FOR-APPLY
**Audit index SHA-256**: `325a4665c863139394767d3ce3454e79528b77d0301a2dad621c7330761c987c`
**Remediation head**: `a4ea8aeec414a7653559291e0a65acd73432a4ee`
**Disposition artifact**: `remediation.plan.dispositions.jsonl`
**Dirty-worktree baseline**: staged=none; unstaged=none; untracked=none
**Unresolved decisions**: none

## Current Finding Verification

| Source run / Finding ID | Verification | Source and final severity + rationale | Closure key/family | Batch | Decision |
|---|---|---|---|---|---|
| `phase18-codex-gpt56-08bc61d-20260830t201731z-b59d9710` / `P18-AUD-001` | Refuted at remediation head: `cargo test -p opi-eval insertion_rejects_ancestor_directory_alias -- --nocapture` passed in the unique committed-head archive. | Major / Major retained; the current insertion path rejects a Windows junction or Unix symlink in any bundle-artifact ancestor. | `bundle.ancestor-alias-containment` / `phase18.current-head-remediation` | none | `no-action:refuted-at-remediation-head` |
| `phase18-codex-gpt56-08bc61d-20260830t201731z-b59d9710` / `P18-AUD-002` | Refuted at remediation head: `cargo test -p opi-eval intent_publication_requires_parent_directory_durability -- --nocapture` passed in the unique committed-head archive. | Major / Major retained; durable intent proof is withheld unless the renamed entry's parent directory is synchronized. | `bundle.intent-parent-durability` / `phase18.current-head-remediation` | none | `no-action:refuted-at-remediation-head` |
| `phase18-codex-gpt56-08bc61d-20260830t201731z-b59d9710` / `P18-AUD-003` | Refuted at remediation head: `cargo test -p opi-eval importer_rejects_phase17_invalid_evidence_graphs -- --nocapture` passed in the unique committed-head archive. | Major / Major retained; mixed runs, non-increasing sequences, invalid parents, and kind/payload mismatch are rejected. | `opi.import-evidence-graph-validation` / `phase18.current-head-remediation` | none | `no-action:refuted-at-remediation-head` |
| `phase18-codex-gpt56-08bc61d-20260830t201731z-b59d9710` / `P18-AUD-004` | Refuted at remediation head: `python scripts/test_phase18_eval_smoke.py` passed all three wrapper tests on Windows in the unique committed-head archive. | Major / Major retained; the PowerShell smoke wrapper now drives the Windows-native helper branch successfully. | `runner.windows-hermetic-helper` / `phase18.current-head-remediation` | none | `no-action:refuted-at-remediation-head` |
| `phase18-codex-gpt56-08bc61d-20260830t201731z-b59d9710` / `P18-AUD-005` | Refuted at remediation head: `cargo test -p opi-eval --test report_output_containment -- --nocapture` passed all three cross-platform containment cases, including a Windows ancestor junction into the run root. | Major / Major retained; the output parent is resolved before containment and create-new checks. | `report.resolved-parent-containment` / `phase18.current-head-remediation` | none | `no-action:refuted-at-remediation-head` |
| `phase18-codex-gpt56-08bc61d-20260830t201731z-b59d9710` / `P18-AUD-006` | Confirmed. The tracked native matrix still binds run `33271354427`, the tracked CI receipt still binds `0f5a3fa152b12d7be4036b2a08ae7a195f8c2107`, and current-candidate native run `33380446360` failed `benchmark-deepswe-completed` as `native-output-invalid`. A discriminating committed-head archive test failed because legal F2P=`20.0` and P2P=`3.0` breakdowns were rejected as `bad-reward-values`. | Major / Major; P18-A22 remains unproved for the current runtime candidate until the importer accepts the native DeepSWE aggregate and fresh exact-candidate native plus three-platform receipts both verify. | `phase18.current-candidate-native-evidence` / `phase18.runtime-fidelity` | B1 | `fix:accept-deepswe-breakdowns-and-refresh-current-candidate-evidence` |
| `phase18-pi-glm53-68d74ec-20260830t200548z` / `P18-AUD-001` | Refuted at remediation head: `cargo test -p opi-coding-agent --test config_tests build_http_client_without_explicit_proxy_succeeds -- --nocapture` passed in the unique committed-head archive while preserving ambient-proxy product behavior. | Minor / Minor retained; the test asserts construction success rather than a proxy-free ambient environment. | `coding-agent.proxy-test-isolation` / `phase18.current-head-remediation` | none | `no-action:refuted-at-remediation-head` |

## Unresolved Decisions

none

## Closure Batches

### Batch B1: Restore current-candidate Phase 18 runtime evidence

**Closure predicate**: One exact candidate derived from the remediation head (1) accepts a one-trial Pier aggregate whose non-authoritative DeepSWE breakdown metrics are finite numeric values such as F2P=`20.0` and P2P=`3.0`, while only the authoritative `reward` metric remains an integer in `0..=1`; (2) passes the complete local repository gate; (3) produces one successful, unexpired, verifier-accepted Phase 18 native artifact proving both real Agents, all three pinned benchmark revisions, and the Terminal-Bench plus DeepSWE native-verifier contracts without paid-provider access; and (4) passes the existing same-repository pull-request CI with ordinary Linux/macOS/Windows merge-ref semantics and yields a new terminal receipt bound to that same runtime candidate. The derived seam matrix and CI receipt must name the new evidence. No second native dispatch is permitted by this plan.
**Dependencies**: none. External mutation begins only after the code/test/changelog candidate passes local verification and the exact plan digest is approved through apply mode.
**Verification union**: the focused parser regression; `cargo test -p opi-eval --test benchmark_integration_conformance`; native-artifact, native-CI, seam-matrix, smoke-wrapper, and documentation script tests; `python scripts/opi-doc-check.py`; `cargo fmt --check --all`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace --all-targets`; `cargo test --workspace --doc`; `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`; one successful exact-candidate `phase18-native-smoke.yml` dispatch; the P18-A22 native artifact verifier; one terminal same-candidate pull-request CI receipt; `scripts/opi-impl-smoke.ps1 full`; and clean explicit-path Git inventory.

#### Fix B1.1: Accept native DeepSWE breakdown values without weakening the aggregate reward

- **Finding source(s)**: `phase18-codex-gpt56-08bc61d-20260830t201731z-b59d9710` + `099fa8c019f1af3f6362c726895a67b45e83691b9d48b75d4a3e8466724b842b` + `P18-AUD-006`
- **Decision**: Keep the existing one-trial, non-empty-eval, one-value-per-metric, one-trial-name-per-value, numeric, finite, and required-`reward` validation. Apply the integer `0..=1` domain only when `metric == "reward"`; preserve all other native metric names and numeric values in the captured result without translating them into shared test counters. Add the exact multi-metric DeepSWE aggregate as a unit regression, retain the negative aggregate-reward cases, and record the user-visible correction under the existing `CHANGELOG.md` Unreleased section.
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/src/benchmark/process.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Move the zero-or-one integer check inside the authoritative aggregate-reward branch and add a regression with F2P=`20.0`, P2P=`3.0`, partial=`1.0`, reward=`1.0`; do not change public APIs, durable schemas, Cargo manifests, the lockfile, workflow bytes, external locks, registered specifications, authority boundaries, or the implementation ledger.
- **Closure predicate**: The multi-metric aggregate imports `Fact::Known { value: 1, origin: "pier-result" }`, while negative, above-one, fractional, missing, duplicate, non-numeric, or non-finite authoritative reward values still fail closed.
- **Red-before**: In `C:\Users\Luiz\AppData\Local\Temp\opi-remediate-plan-phase18-b4fc88f32b1741e0859c243b46a769d0`, `cargo test -p opi-eval pier_job_import_accepts_native_multi_metric_breakdowns -- --nocapture` failed: `import_pier_job_result` returned `Invalid("bad-reward-values")` for the legal F2P=`20.0` breakdown.
- **Green-after**: Run the same `cargo test -p opi-eval pier_job_import_accepts_native_multi_metric_breakdowns -- --nocapture`; expect exactly one matching test to pass, then run the complete verification union before materialization.

#### Fix B1.2: Materialize one fresh exact-candidate native and three-platform evidence cycle

- **Finding source(s)**: `phase18-codex-gpt56-08bc61d-20260830t201731z-b59d9710` + `099fa8c019f1af3f6362c726895a67b45e83691b9d48b75d4a3e8466724b842b` + `P18-AUD-006`
- **Decision**: After local green, create one candidate commit containing only B1.1, push it to the existing same-repository PR branch, and allow its ordinary pull-request CI to run. Dispatch `.github/workflows/phase18-native-smoke.yml` exactly once with that full candidate SHA. If either external run is not terminal-success, if the native artifact is missing/expired, or if any verifier rejects the evidence, do not rerun, repair, edit evidence, or substitute older artifacts; stop and record B1 as Not closed. On success, verify the downloaded artifact/receipt for P18-A22, derive the seam matrix from those exact bytes, derive the terminal CI receipt from the same candidate's actual run/job/artifact metadata, rerun the complete final gate, and materialize only the two derived tracked evidence files plus the fixed remediation result artifacts.
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/docs/seam-evidence-matrix.md`, `docs/snapshots/phase18/ci-receipt.json`
- **Change kind**: metadata
- **Change**: Replace historical runtime evidence references with script-derived, digest-bound outputs for the new candidate; never hand-edit either derived artifact and never update `.opi-impl-state.json` or its Phase snapshot.
- **Closure predicate**: The P18-A22 native verifier passes against the new candidate and its one successful native artifact, the derived matrix names that run/artifact, every required PR CI predecessor is terminal-success, the generated CI receipt names the same candidate head and actual merge-ref checkouts, and `scripts/opi-impl-smoke.ps1 full` passes afterward.
- **Red-before**: `gh run view 33380446360 --log-failed` reports `benchmark-deepswe-completed` as `native-output-invalid`; the verifier stdout includes F2P=`20.0`, P2P=`3.0`, and reward=`1.0`, while no artifact upload steps ran. The tracked matrix and CI receipt bind older candidates.
- **Green-after**: Run `python scripts/verify-phase18-native-artifact.py --criterion P18-A22 --expected-commit <candidate-sha> --receipt <new-native-receipt.json> --artifact <new-native-artifact>` and `python scripts/verify-phase18-ci.py --expected-head <candidate-sha> --run-metadata <new-pr-run.json> --jobs-metadata <new-pr-jobs.json> --artifact-metadata <new-pr-artifact.json> --inner-receipt <new-pr-inner-receipt.json> --output docs/snapshots/phase18/ci-receipt.json`; both must pass before the matrix, receipt, or result is accepted.

## Final Verification

    cargo test -p opi-eval pier_job_import_accepts_native_multi_metric_breakdowns -- --nocapture
    cargo test -p opi-eval pier_job_import_enforces_the_native_reward_domain -- --nocapture
    cargo test -p opi-eval --test benchmark_integration_conformance
    python scripts/test_verify_phase18_native_artifact.py
    python scripts/test_verify_phase18_native_ci.py
    python scripts/test_derive_phase18_seam_matrix.py
    python scripts/test_phase18_eval_smoke.py
    python scripts/test_verify_phase18_ci.py
    python scripts/opi-doc-check.py
    cargo fmt --check --all
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace --all-targets
    cargo test --workspace --doc
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
    gh workflow run phase18-native-smoke.yml --ref <published-branch> -f candidate_sha=<candidate-sha>
    gh run watch <native-run-id> --exit-status
    python scripts/verify-phase18-native-artifact.py --criterion P18-A22 --expected-commit <candidate-sha> --receipt <new-native-receipt.json> --artifact <new-native-artifact>
    python scripts/derive-phase18-seam-matrix.py --receipt <new-native-receipt.json> --artifact <new-native-artifact> --require-trajectory-spans --output crates/opi-eval/docs/seam-evidence-matrix.md --verify
    gh run watch <candidate-pr-ci-run-id> --exit-status
    python scripts/verify-phase18-ci.py --expected-head <candidate-sha> --run-metadata <new-pr-run.json> --jobs-metadata <new-pr-jobs.json> --artifact-metadata <new-pr-artifact.json> --inner-receipt <new-pr-inner-receipt.json> --output docs/snapshots/phase18/ci-receipt.json
    scripts/opi-impl-smoke.ps1 full
    python .agents/skills/_shared/scripts/validate_assurance_artifact.py dispositions docs/snapshots/phase18/assurance/remediation.result.dispositions.jsonl
    python .agents/skills/_shared/scripts/validate_assurance_artifact.py result docs/snapshots/phase18/assurance/remediation.result.md
    git status --short

The native workflow command is a hard one-dispatch ceiling for this plan. A failed, cancelled, timed-out, skipped, mismatched, or artifact-less native run is terminal for this apply. Any failing local or PR gate outside the bounded parser change also stops apply; it is reported rather than repaired without a new validated plan.

## Exclusions

| Finding ID | Disposition | Current evidence/authority |
|---|---|---|
| `phase18-codex-gpt56-08bc61d-20260830t201731z-b59d9710/P18-AUD-001` | Refuted | Current committed-head ancestor-alias regression passes on Windows. |
| `phase18-codex-gpt56-08bc61d-20260830t201731z-b59d9710/P18-AUD-002` | Refuted | Current committed-head parent-directory durability regression passes. |
| `phase18-codex-gpt56-08bc61d-20260830t201731z-b59d9710/P18-AUD-003` | Refuted | Current committed-head evidence-graph adversarial matrix passes. |
| `phase18-codex-gpt56-08bc61d-20260830t201731z-b59d9710/P18-AUD-004` | Refuted | Current committed-head Windows wrapper suite passes all three cases. |
| `phase18-codex-gpt56-08bc61d-20260830t201731z-b59d9710/P18-AUD-005` | Refuted | Current committed-head cross-platform report containment suite passes the ancestor-alias case. |
| `phase18-pi-glm53-68d74ec-20260830t200548z/P18-AUD-001` | Refuted | Current committed-head proxy construction test passes without asserting ambient proxy absence. |
