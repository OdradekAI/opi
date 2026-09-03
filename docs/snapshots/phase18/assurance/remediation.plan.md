# Phase 18 Remediation Plan

**Status**: READY-FOR-APPLY
**Audit index SHA-256**: `305624349f650309f374ff8556114d5572c21b02fa30c48ee1ea2eaa95d78195`
**Remediation head**: `102d30bd861c0ce48e79f07a723e6e14d4224cb9`
**Disposition artifact**: `remediation.plan.dispositions.jsonl`
**Dirty-worktree baseline**: staged `[]`; unstaged `[]`; untracked `[]`
**Unresolved decisions**: none

## Current Finding Verification

| Source run / Finding ID | Verification | Source and final severity + rationale | Closure key/family | Batch | Decision |
|---|---|---|---|---|---|
| `phase18-codex-gpt56-a8bb454-20260903t021838z` / `P18-AUD-001` | **Confirmed.** In the committed archive, `crates/opi-eval/docs/seam-evidence-matrix.md` is absent and `verify-native-smoke-artifact.py --help` exposes no `--matrix-output` derivation surface. | Source Major; final Major. `P18-OUT-005` and `P18-A20` require an inspectable, artifact-derived minimum-seam result and a guard that keeps unproved choices provisional. | `phase18.seam-evidence-matrix` / `phase18.evidence-closure` | B2 | `fix:derive-and-retain-native-seam-matrix` |
| `phase18-codex-gpt56-a8bb454-20260903t021838z` / `P18-AUD-002` | **Confirmed.** `gh run list --commit 102d30bd...` returns `[]` for the remediation head. The current Agent and benchmark code equals the audited code, and the only current-audit-head PR CI run, `33783810402`, is red: the Unix Agent conformance test directly executes a Git-mode `100644` provider script, and macOS runtime acceptance lets a cleanup race mask the error-mapping assertion. No native-smoke run exists for either the audit or remediation head. | Source Major; final Major. Local fixture success cannot replace passing, commit-bound Opi/pi, three-native-verifier, paired-outcome, and three-platform evidence. | `phase18.current-native-and-ci-proof` / `phase18.evidence-closure` | B1 | `fix:stabilize-gates-and-produce-current-evidence` |

## Unresolved Decisions

None. On 2026-09-04 the repository operator authorized the required sequence of
fresh plan/apply/materialization cycles, including the explicit commit/push
boundaries and manually dispatched GitHub Actions native smoke. Each new HEAD
still requires a fresh fixed plan and approval; this plan does not bypass the
remediation protocol's no-commit apply boundary.

## Closure Batches

### Batch B1: Stabilize the current evidence gates and obtain commit-bound execution

**Closure predicate**: A reviewed producer commit containing the current Eval/workflow bytes passes the pinned Linux native-smoke verifier for real Opi and pi across Terminal-Bench 2.1, Terminal-Bench 3.0, and DeepSWE v1.1 with six paired sealed trials, and the later matrix commit passes the ordinary PR CI merge-ref checks on Ubuntu, macOS, and Windows; the only scoped difference from the native producer commit is the generated matrix and remediation evidence.

**Dependencies**: Operator-authorized multi-materialization sequence; source fixes must be materialized before native dispatch; B2's generated matrix must be materialized before final PR CI.

**Verification union**: Unix permission-faithful Agent conformance; execution-runtime mapping acceptance; all `opi-eval` verifier tests; local workspace gates; exact-commit native artifact verification; final PR merge-ref CI identity and conclusions.

#### Fix B1.1: Make the scripted-provider conformance case independent of repository execute bits

- **Finding source(s)**: `phase18-codex-gpt56-a8bb454-20260903t021838z` + `3193940b79737046d5b977a0ad4670d17e1e6ae5fd7d2a133f68743f37986897` + `P18-AUD-002`
- **Decision**: invoke the explicitly supplied Python fixture through `python3` in the generated Unix helper; do not chmod or mutate the repository source and do not broaden PATH/provider fallback.
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/src/cli/conformance.rs`
- **Change kind**: behavioral
- **Change**: replace the direct execution of `$OPI_EVAL_SCRIPTED_PROVIDER` in the `provider-fixture` helper branch with an explicit `python3 "$OPI_EVAL_SCRIPTED_PROVIDER"` invocation. Retain the existing exact response-byte check and process settlement.
- **Closure predicate**: The checked-in provider remains Git mode `100644`, while the Agent conformance matrix passes from an ext4 `git archive` extraction on Unix.
- **Red-before**: `git archive --format=tar 102d30bd861c0ce48e79f07a723e6e14d4224cb9`, extract on WSL ext4, confirm `stat` reports `644`, then run `cargo test -p opi-eval --test agent_integration_conformance agent_conformance_matrix_settles_every_pinned_case -- --nocapture`; observed FAIL at `case opi provider-fixture`, conformance exit `1`.
- **Green-after**: Repeat the same permission-faithful archive command; expect Git mode `644` and PASS for both Opi and pi provider-fixture rows.

#### Fix B1.2: Make the runtime-layer mapping test exercise a typed terminal failure

- **Finding source(s)**: `phase18-codex-gpt56-a8bb454-20260903t021838z` + `3193940b79737046d5b977a0ad4670d17e1e6ae5fd7d2a133f68743f37986897` + `P18-AUD-002`
- **Decision**: keep malformed-frame teardown coverage in `execution_protocol_host`; make the runtime test use the mock peer's legal post-started `Failed{code: protocol_violation}` mode so it tests only `ProcessCommandAdapter` error lifting. Do not weaken cleanup dominance or accept multiple stable codes.
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/tests/execution_runtime.rs`
- **Change kind**: test-only
- **Change**: add the minimum canned-argument mapping for the existing `failed_post_started protocol_violation` mock mode and use it in `process_command_adapter_protocol_violation_lifts_stable_code`.
- **Closure predicate**: The focused runtime test deterministically reports `protocol_violation` on Ubuntu, macOS, and Windows, while protocol-host malformed-frame and cleanup-unconfirmed tests remain unchanged and green.
- **Red-before**: GitHub Actions run `33783810402`, job `100743619935`, command `cargo test -p opi-coding-agent --features execution-backend-test-fixture --test execution_runtime`; observed macOS FAIL with `cleanup_unconfirmed` instead of `protocol_violation`.
- **Green-after**: Run the same focused runtime command locally, then require the identically named `execution_acceptance` job to pass on all three CI platforms.

#### Fix B1.3: Materialize and prove the current native/CI execution chain

- **Finding source(s)**: `phase18-codex-gpt56-a8bb454-20260903t021838z` + `3193940b79737046d5b977a0ad4670d17e1e6ae5fd7d2a133f68743f37986897` + `P18-AUD-002`
- **Decision**: under the operator-authorized sequence, materialize B1.1/B1.2 plus B2.1, dispatch the sole pinned native-smoke workflow with that exact commit as `candidate_sha`, download both artifacts, run the offline `all-native` verifier, and retain the exact run/artifact/commit identities. After B2.2 is materialized, use ordinary PR merge-ref CI; do not substitute a head-only workflow.
- **Verification status**: Confirmed
- **File(s)**: external GitHub Actions run/artifact identities recorded in the eventual remediation result; no production path beyond B1.1/B1.2.
- **Change kind**: metadata
- **Change**: produce new evidence only; no paid provider, alternate grader, cached score, or fixture substitution.
- **Closure predicate**: The native verifier exits zero for `all-native`, the producer receipt binds the exact producer commit, and every ordinary PR CI check is successful with the PR head and checked-out merge commit recorded.
- **Red-before**: `gh run list --repo OdradekAI/opi --commit 102d30bd861c0ce48e79f07a723e6e14d4224cb9 --json databaseId,workflowName,status,conclusion,headSha` returned `[]`; the current-audit-head CI run is red and no native run exists.
- **Green-after**: Exact commands in Final Verification must return a successful native run and successful final PR CI bound to their recorded full SHAs.

### Batch B2: Derive and retain the minimum seam matrix from verified native evidence

**Closure predicate**: `crates/opi-eval/docs/seam-evidence-matrix.md` is deterministically generated only after full native-artifact acceptance, binds the producer/run/artifact identities, separates shared fields and behavior from adapter-private facts and provisional hypotheses, covers both Agents, all three benchmark revisions, and Harbor/Pier verifier ownership, and is protected by positive and fail-closed tests.

**Dependencies**: B1 source fixes and a successful exact-producer-commit native artifact; operator-authorized materialization sequence.

**Verification union**: native-artifact verifier behavioral tests; rejected-artifact no-output test; deterministic matrix comparison; documentation check; scoped native-to-final diff guard; final three-platform PR CI.

#### Fix B2.1: Add a fail-closed matrix derivation output to the native-artifact verifier

- **Finding source(s)**: `phase18-codex-gpt56-a8bb454-20260903t021838z` + `3193940b79737046d5b977a0ad4670d17e1e6ae5fd7d2a133f68743f37986897` + `P18-AUD-001`
- **Decision**: extend the existing sole native-artifact verifier rather than add another evidence parser. Permit `--matrix-output` only with `--criterion all-native`; write nothing unless every existing verification passes; derive inventories, common fields/behaviors, adapter-private evidence, verifier ownership, provisional hypotheses, and provenance from the accepted artifact plus its commit-bound profiles.
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/scripts/verify-native-smoke-artifact.py`, `crates/opi-eval/scripts/test_verify_native_smoke_artifact.py`
- **Change kind**: behavioral
- **Change**: add deterministic Markdown rendering and atomic output replacement after successful verification. Tests must prove exact stable output, dynamic inventory coverage, two verifier owners, rejection without output, and refusal outside `all-native`.
- **Closure predicate**: A valid synthetic native artifact produces a byte-stable matrix with the three required evidence sets; any invalid artifact or partial criterion leaves the output absent.
- **Red-before**: In the committed archive, `python crates/opi-eval/scripts/verify-native-smoke-artifact.py --help` contains no `--matrix-output`; `Test-Path crates/opi-eval/docs/seam-evidence-matrix.md` is `False`.
- **Green-after**: `python crates/opi-eval/scripts/test_verify_native_smoke_artifact.py` passes the new positive, deterministic, ownership, and no-output negative cases.

#### Fix B2.2: Generate the checked-in conformance result from the real artifact

- **Finding source(s)**: `phase18-codex-gpt56-a8bb454-20260903t021838z` + `3193940b79737046d5b977a0ad4670d17e1e6ae5fd7d2a133f68743f37986897` + `P18-AUD-001`
- **Decision**: generate the fixed matrix path by passing the downloaded, successfully verified native artifact to B2.1. Do not reconstruct historical conclusions, hand-author rows, or mark Rust traits, exact process envelopes, ATIF/span canonicality, directory layout, packaging, or publication stable.
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/docs/seam-evidence-matrix.md`
- **Change kind**: documentation
- **Change**: retain the generated artifact-derived matrix and its provenance; review the generated diff for only evidence-supported settlement.
- **Closure predicate**: The tracked matrix exactly matches a fresh derivation from the retained native artifact, and the only scoped native-producer-to-final-head change is the matrix plus assurance evidence.
- **Red-before**: `Test-Path -LiteralPath crates/opi-eval/docs/seam-evidence-matrix.md` returned `False` at the remediation head.
- **Green-after**: Re-run the verifier with the same receipt/artifact into a temporary path and compare bytes with the tracked matrix; expect no difference, then pass `python scripts/opi-doc-check.py`.

## Final Verification

Before the first materialization boundary:

    # From a fresh ext4 extraction of `git archive <candidate>` with the repository cache.
    stat -c '%a %n' crates/opi-eval/scripts/scripted-provider.py
    cargo test -p opi-eval --test agent_integration_conformance agent_conformance_matrix_settles_every_pinned_case -- --nocapture
    cargo test -p opi-coding-agent --features execution-backend-test-fixture --test execution_backend_mock --no-run
    cargo test -p opi-coding-agent --features execution-backend-test-fixture --test execution_runtime fixture::process_command_adapter_protocol_violation_lifts_stable_code -- --exact
    cargo test -p opi-coding-agent --features execution-backend-test-fixture --test execution_protocol_host
    python crates/opi-eval/scripts/test_verify_native_smoke_artifact.py
    cargo test -p opi-eval --all-targets
    python scripts/opi-doc-check.py
    cargo fmt --check --all
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace --all-targets
    cargo test --workspace --doc
    RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps

After the source/tooling commit is explicitly authorized, committed, and pushed:

    PRODUCER_SHA=$(git rev-parse HEAD)
    gh workflow run 344412365 --repo OdradekAI/opi --ref codex/phase18-remediation-evidence-a8bb454 -f candidate_sha="$PRODUCER_SHA"
    gh run watch <native-run-id> --repo OdradekAI/opi --exit-status
    gh run download <native-run-id> --repo OdradekAI/opi -n opi-eval-native-smoke-upload-receipt -D <receipt-dir>
    gh run download <native-run-id> --repo OdradekAI/opi -n opi-eval-native-smoke -D <artifact-dir>
    python crates/opi-eval/scripts/verify-native-smoke-artifact.py --criterion all-native --expected-commit "$PRODUCER_SHA" --receipt <receipt-dir>/upload-receipt.json --artifact <artifact-dir>/sealed-artifact.tar --repo . --matrix-output crates/opi-eval/docs/seam-evidence-matrix.md
    python crates/opi-eval/scripts/verify-native-smoke-artifact.py --criterion all-native --expected-commit "$PRODUCER_SHA" --receipt <receipt-dir>/upload-receipt.json --artifact <artifact-dir>/sealed-artifact.tar --repo . --matrix-output <temporary-matrix>
    cmp crates/opi-eval/docs/seam-evidence-matrix.md <temporary-matrix>

After the generated matrix is explicitly authorized, committed, and pushed:

    FINAL_SHA=$(git rev-parse HEAD)
    git diff --name-only "$PRODUCER_SHA" "$FINAL_SHA" -- .github/workflows crates/opi-eval Cargo.toml Cargo.lock
    # Expected scoped output: crates/opi-eval/docs/seam-evidence-matrix.md only.
    gh pr view 7 --repo OdradekAI/opi --json headRefOid,baseRefOid,statusCheckRollup
    gh pr checks 7 --repo OdradekAI/opi --watch --fail-fast
    gh api repos/OdradekAI/opi/pulls/7 --jq '{head:.head.sha,base:.base.sha,merge_commit_sha,mergeable_state}'
    gh api repos/OdradekAI/opi/actions/runs/<ci-run-id>/jobs --paginate
    gh run view <ci-run-id> --repo OdradekAI/opi --job <one-successful-three-platform-job-id> --log

The final evidence must show the PR head equals `FINAL_SHA`, the pull-request run checked out the reported merge commit, and all Ubuntu/macOS/Windows `test`, `execution_acceptance`, and retained workspace gates succeeded. Any additional scoped source/workflow/manifest difference after `PRODUCER_SHA`, failed native criterion, missing pair, missing verifier owner, or non-successful CI conclusion keeps the finding open and requires a fresh plan.

## Exclusions

None. Both current findings are blocking, confirmed, and assigned exactly once.
