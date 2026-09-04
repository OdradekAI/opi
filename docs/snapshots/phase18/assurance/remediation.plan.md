# Phase 18 Remediation Plan

**Status**: READY-FOR-APPLY
**Audit index SHA-256**: `305624349f650309f374ff8556114d5572c21b02fa30c48ee1ea2eaa95d78195`
**Remediation head**: `46bddf23ebe434057d38fe2a0ba82a5dfbea9bc9`
**Disposition artifact**: `remediation.plan.dispositions.jsonl`
**Dirty-worktree baseline**: staged `[]`; unstaged `[]`; untracked `[]`
**Unresolved decisions**: none

## Current Finding Verification

| Source run / Finding ID | Verification | Source and final severity + rationale | Closure key/family | Batch | Decision |
|---|---|---|---|---|---|
| `phase18-codex-gpt56-a8bb454-20260903t021838z` / `P18-AUD-001` | **Partially confirmed.** At the remediation head the artifact verifier exposes `--matrix-output` and all 17 verifier tests pass, but `crates/opi-eval/docs/seam-evidence-matrix.md` is still absent because no current native artifact exists from which it may be derived. | Source Major; final Major. The derivation mechanism is now present, but mandatory `P18-OUT-005` and `P18-A20` remain unsatisfied until the inspectable artifact-derived result exists. | `phase18.seam-evidence-matrix` / `phase18.evidence-closure` | B2 | `fix:derive-and-retain-current-native-seam-matrix` |
| `phase18-codex-gpt56-a8bb454-20260903t021838z` / `P18-AUD-002` | **Partially confirmed.** Exact-head pull-request CI run `33794442307` succeeded for `46bddf23ebe434057d38fe2a0ba82a5dfbea9bc9`, including the three-platform runtime fixes. No native-smoke run exists for that head. GitHub workflow id `344412365` is registered at `.github/workflows/phase18-native-smoke.yml`, that path is absent on the target branch, and `gh workflow view ... --ref codex/phase18-remediation-evidence-a8bb454 --yaml` fails before dispatch. | Source Major; final Major. Current three-platform evidence now exists, but the required real Opi/pi and three-native-verifier execution remains unavailable and blocks `P18-OUT-001`, `P18-OUT-002`, `P18-A02`-`A04`, `P18-A08`-`A10`, `P18-A12`, and `P18-A22`. | `phase18.current-native-proof` / `phase18.evidence-closure` | B1 | `fix:bind-registered-dispatch-and-produce-current-native-evidence` |

## Unresolved Decisions

None. The registered GitHub workflow identity and the target-branch workflow
are both mechanically known. The minimum repair is a temporary, byte-identical
copy at the registered path. It remains manual-only and read-only, shares the
existing concurrency guard, and is deleted after the one accepted native run.
It is evidence materialization scaffolding, not a retained compatibility path
or a second producer contract.

The canonical workflow, producer, and offline verifier will bind the path that
GitHub actually executed. A non-canonical workflow path is accepted only when
its producer-commit bytes equal the canonical workflow byte-for-byte. This
preserves fail-closed provenance while allowing the already registered workflow
id to execute the current canonical producer bytes.

## Closure Batches

### Batch B1: Bind the registered workflow path and obtain current native evidence

**Closure predicate**: A producer commit makes workflow id `344412365`
resolvable on `codex/phase18-remediation-evidence-a8bb454`; the registered copy
is byte-identical to the canonical workflow; the workflow, producer, and
offline verifier agree on the actual executing path and reject path/ref or byte
drift; and one successful run yields an offline-verified `all-native` artifact
for that exact producer commit covering both Agents, all three benchmark
revisions, six paired trials, and at least two native-verifier owners.

**Dependencies**: An explicitly authorized first materialization commit and
push. The temporary registered-path copy remains present until the native run
and downloaded artifact pass verification.

**Verification union**: Native-workflow static-verifier tests and negative
paths; native-artifact verifier tests and path/ref/byte-drift negatives;
byte-identity check for the two workflow paths; GitHub target-ref resolution;
exact-commit native workflow run; downloaded receipt/artifact verification.

#### Fix B1.1: Bind the actual workflow identity without weakening the canonical producer

- **Finding source(s)**: `phase18-codex-gpt56-a8bb454-20260903t021838z` + `3193940b79737046d5b977a0ad4670d17e1e6ae5fd7d2a133f68743f37986897` + `P18-AUD-002`
- **Decision**: `fix:bind-registered-dispatch-and-produce-current-native-evidence`
- **Verification status**: Partially confirmed
- **File(s)**: `.github/workflows/opi-eval-native-smoke.yml`, `.github/workflows/phase18-native-smoke.yml` (temporary), `crates/opi-eval/scripts/verify-native-smoke-ci.py`, `crates/opi-eval/scripts/test_verify_native_smoke_ci.py`, `crates/opi-eval/scripts/verify-native-smoke-artifact.py`, `crates/opi-eval/scripts/test_verify_native_smoke_artifact.py`
- **Change kind**: behavioral
- **Change**: make the canonical workflow derive its own repository-relative path from `github.workflow_ref` and pass that path to the existing producer. Tighten the static verifier so a hard-coded or unbound workflow path is rejected. Tighten the downloaded-artifact verifier so the qualified GitHub workflow ref and recorded workflow path must agree; a path other than the canonical path is accepted only when its producer-commit blob is byte-identical to the canonical workflow blob. Add positive byte-identical-alias coverage and negative ref/path and alias-byte-drift coverage. Finally, add `.github/workflows/phase18-native-smoke.yml` as an exact copy of the updated canonical workflow for the producer commit only.
- **Closure predicate**: The two committed workflow blobs are identical; both pass the sole static producer verifier; path/ref or byte drift fails the offline verifier; and GitHub resolves workflow id `344412365` on the target ref without adding any automatic trigger or authority.
- **Red-before**: `gh workflow view 344412365 --repo OdradekAI/opi --ref codex/phase18-remediation-evidence-a8bb454 --yaml` exited nonzero with `could not find workflow file phase18-native-smoke.yml`; the contents API returned 404 for that path on the target branch and for the canonical path on `main`.
- **Green-after**: both Python verifier suites pass; both workflow paths pass `verify-native-smoke-ci.py`; their Git blob ids are equal; negative tests reject a mismatched qualified ref and a byte-different registration copy; and the same `gh workflow view` command exits zero.

#### Fix B1.2: Produce and verify the exact-commit native artifact

- **Finding source(s)**: `phase18-codex-gpt56-a8bb454-20260903t021838z` + `3193940b79737046d5b977a0ad4670d17e1e6ae5fd7d2a133f68743f37986897` + `P18-AUD-002`
- **Decision**: dispatch the sole registered workflow id against the producer commit, wait for success, download both named artifacts, and run the offline `all-native` verifier before generating any matrix or deleting the temporary registration copy.
- **Verification status**: Partially confirmed
- **File(s)**: none beyond B1.1; this step materializes external evidence
- **Change kind**: metadata
- **Change**: retain exact run, workflow, commit, upload-receipt, artifact, Agent, benchmark, trial, and native-verifier identities. Do not substitute fixture, cached-score, head-only CI, or historical native evidence.
- **Closure predicate**: The workflow concludes successfully for the exact producer SHA and the offline verifier accepts the downloaded artifact and receipt under `--criterion all-native --expected-commit <producer-sha>`.
- **Red-before**: `gh run list --repo OdradekAI/opi --workflow 344412365 --branch codex/phase18-remediation-evidence-a8bb454 ...` returned `[]` at remediation head `46bddf23ebe434057d38fe2a0ba82a5dfbea9bc9`.
- **Green-after**: the native run succeeds; the receipt and artifact download under their exact `opi-eval-native-smoke-upload-receipt` and `opi-eval-native-smoke` names; and offline `all-native` verification exits zero with the producer SHA, six sealed paired trials, all three revisions, and at least two distinct verifier owners recorded.

### Batch B2: Derive the minimum seam and remove the temporary registration copy

**Closure predicate**: The accepted B1 artifact deterministically generates the
tracked seam-evidence matrix; the matrix distinguishes proved shared behavior,
adapter-private facts, and rejected/provisional hypotheses; the temporary
registered-path copy is absent from the final tree; and ordinary pull-request
merge-ref CI succeeds for the final head without changing the canonical
workflow, producer, or verifier bytes proven by B1.

**Dependencies**: B1's successful exact-producer native artifact; an explicitly
authorized second materialization commit and push before final PR CI.

**Verification union**: Offline `all-native` verification and two-output byte
comparison; generated-matrix content checks; temporary-path absence; scoped
producer-to-final diff; documentation checks; workspace Phase-exit gates;
ordinary PR head/merge-ref identity and all required job conclusions.

#### Fix B2.1: Generate and retain the artifact-derived seam-evidence matrix

- **Finding source(s)**: `phase18-codex-gpt56-a8bb454-20260903t021838z` + `3193940b79737046d5b977a0ad4670d17e1e6ae5fd7d2a133f68743f37986897` + `P18-AUD-001`
- **Decision**: `fix:derive-and-retain-current-native-seam-matrix`
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-eval/docs/seam-evidence-matrix.md`
- **Change kind**: documentation
- **Change**: after B1 passes, invoke the already tested `--matrix-output` path against the downloaded current native artifact, write the tracked matrix, derive it again to a unique temporary path, and require byte equality. The result must name both Agents, all three benchmark revisions, observed native roles and verifier owners, proved shared fields/behaviors, adapter-private evidence, and package/type/process/trajectory/span/directory hypotheses that remain rejected or provisional.
- **Closure predicate**: The matrix exists only after `all-native` acceptance, a second derivation is byte-identical, every required real integration and at least two verifier owners are represented, and no unsupported choice is promoted to a stable seam.
- **Red-before**: `Test-Path crates/opi-eval/docs/seam-evidence-matrix.md` returned `False`; `--matrix-output` and its 17-test derivation guard are present, so the remaining failure is specifically the absent current native artifact and derived output.
- **Green-after**: run the verifier twice against the same accepted receipt/artifact and producer SHA, compare outputs byte-for-byte, inspect the required three evidence sets and provenance, and pass `python scripts/opi-doc-check.py`.

#### Fix B2.2: Remove the materialization-only registration copy

- **Finding source(s)**: supports both current findings without adding a second disposition
- **Decision**: delete `.github/workflows/phase18-native-smoke.yml` only after the B1 artifact and B2 matrix pass, retaining the updated canonical workflow and verifier guards.
- **Verification status**: Partially confirmed
- **File(s)**: `.github/workflows/phase18-native-smoke.yml`
- **Change kind**: metadata
- **Change**: remove the temporary registered path before the final evidence commit so the repository retains one canonical native producer workflow and no Phase-local compatibility entry point.
- **Closure predicate**: The final tree contains only `.github/workflows/opi-eval-native-smoke.yml`; the producer-to-final scoped diff contains only deletion of the temporary copy plus the generated matrix; canonical producer/verifier blob identities are unchanged.
- **Red-before**: not applicable before B1 because the temporary copy does not yet exist; its presence is an explicitly bounded producer-commit precondition, not an existing defect.
- **Green-after**: `Test-Path .github/workflows/phase18-native-smoke.yml` is false at the final head and the scoped producer-to-final diff is exactly that deletion plus `crates/opi-eval/docs/seam-evidence-matrix.md`.

## Final Verification

Before the first materialization boundary:

    python crates/opi-eval/scripts/test_verify_native_smoke_ci.py
    python crates/opi-eval/scripts/test_verify_native_smoke_artifact.py
    python crates/opi-eval/scripts/verify-native-smoke-ci.py --workflow .github/workflows/opi-eval-native-smoke.yml --script crates/opi-eval/scripts/native-smoke.sh --build-script crates/opi-eval/scripts/build-agent-artifacts.sh --provider crates/opi-eval/scripts/scripted-provider.py
    python crates/opi-eval/scripts/verify-native-smoke-ci.py --workflow .github/workflows/phase18-native-smoke.yml --script crates/opi-eval/scripts/native-smoke.sh --build-script crates/opi-eval/scripts/build-agent-artifacts.sh --provider crates/opi-eval/scripts/scripted-provider.py
    # Require equal Git blob ids for both workflow paths in the prospective producer commit.
    python scripts/opi-doc-check.py
    git diff --check

After the producer commit is explicitly authorized, committed, and pushed:

    PRODUCER_SHA=$(git rev-parse HEAD)
    test "$(git rev-parse "$PRODUCER_SHA:.github/workflows/opi-eval-native-smoke.yml")" = "$(git rev-parse "$PRODUCER_SHA:.github/workflows/phase18-native-smoke.yml")"
    gh workflow view 344412365 --repo OdradekAI/opi --ref codex/phase18-remediation-evidence-a8bb454 --yaml
    gh workflow run 344412365 --repo OdradekAI/opi --ref codex/phase18-remediation-evidence-a8bb454 -f candidate_sha="$PRODUCER_SHA"
    gh run watch <native-run-id> --repo OdradekAI/opi --exit-status
    gh run view <native-run-id> --repo OdradekAI/opi --json databaseId,event,headSha,status,conclusion,url,workflowName
    gh run download <native-run-id> --repo OdradekAI/opi -n opi-eval-native-smoke-upload-receipt -D <receipt-dir>
    gh run download <native-run-id> --repo OdradekAI/opi -n opi-eval-native-smoke -D <artifact-dir>
    python crates/opi-eval/scripts/verify-native-smoke-artifact.py --criterion all-native --expected-commit "$PRODUCER_SHA" --receipt <receipt-dir>/upload-receipt.json --artifact <artifact-dir>/sealed-artifact.tar --repo . --matrix-output crates/opi-eval/docs/seam-evidence-matrix.md
    python crates/opi-eval/scripts/verify-native-smoke-artifact.py --criterion all-native --expected-commit "$PRODUCER_SHA" --receipt <receipt-dir>/upload-receipt.json --artifact <artifact-dir>/sealed-artifact.tar --repo . --matrix-output <temporary-matrix>
    cmp crates/opi-eval/docs/seam-evidence-matrix.md <temporary-matrix>

After deleting the temporary workflow copy and before the second materialization boundary:

    test ! -e .github/workflows/phase18-native-smoke.yml
    python crates/opi-eval/scripts/test_verify_native_smoke_ci.py
    python crates/opi-eval/scripts/test_verify_native_smoke_artifact.py
    python scripts/opi-doc-check.py
    git diff --check
    cargo fmt --check --all
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace --all-targets
    cargo test --workspace --doc
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps

After the final evidence commit is explicitly authorized, committed, and pushed:

    FINAL_SHA=$(git rev-parse HEAD)
    git diff --name-only "$PRODUCER_SHA" "$FINAL_SHA" -- .github/workflows crates/opi-eval Cargo.toml Cargo.lock
    # Expected output: deletion of .github/workflows/phase18-native-smoke.yml
    # and addition of crates/opi-eval/docs/seam-evidence-matrix.md only.
    gh pr view 7 --repo OdradekAI/opi --json headRefOid,baseRefOid,statusCheckRollup
    gh pr checks 7 --repo OdradekAI/opi --watch --fail-fast
    gh api repos/OdradekAI/opi/pulls/7 --jq '{head:.head.sha,base:.base.sha,merge_commit_sha,mergeable_state}'
    gh api repos/OdradekAI/opi/actions/runs/<ci-run-id>/jobs --paginate
    gh run view <ci-run-id> --repo OdradekAI/opi --job <one-successful-three-platform-job-id> --log

The final evidence must show the PR head equals `FINAL_SHA`, the pull-request
run checked out the reported merge commit, and all Ubuntu/macOS/Windows `test`,
`execution_acceptance`, and retained workspace gates succeeded. Any workflow
copy that is not byte-identical, any automatic trigger or broader permission,
any path/ref/digest mismatch, failed native criterion, missing pair or verifier
owner, unexpected producer-to-final scoped difference, or non-successful final
CI conclusion keeps the finding open and requires a fresh plan.

## Exclusions

None. Both current findings remain blocking, partially confirmed, and assigned
exactly once.
