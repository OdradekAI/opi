# Phase 18 Remediation Plan

**Status**: READY-FOR-APPLY
**Audit index SHA-256**: `305624349f650309f374ff8556114d5572c21b02fa30c48ee1ea2eaa95d78195`
**Remediation head**: `a8bb45426daf960d9e60024ce34542995c4dd2d1`
**Disposition artifact**: `remediation.plan.dispositions.jsonl`
**Dirty-worktree baseline**: staged=[]; unstaged=[docs/snapshots/phase18/assurance/audit.codex.gpt56.findings.jsonl, docs/snapshots/phase18/assurance/audit.codex.gpt56.md, docs/snapshots/phase18/assurance/audit.codex.gpt56.meta.json, docs/snapshots/phase18/assurance/audit.codex.gpt56.requirements.jsonl, docs/snapshots/phase18/assurance/audit.index.json]; untracked=[docs/snapshots/phase18/assurance/history/phase18-codex-gpt56-c8d6dea-20260902t181312z/audit.codex.gpt56.findings.jsonl, docs/snapshots/phase18/assurance/history/phase18-codex-gpt56-c8d6dea-20260902t181312z/audit.codex.gpt56.md, docs/snapshots/phase18/assurance/history/phase18-codex-gpt56-c8d6dea-20260902t181312z/audit.codex.gpt56.meta.json, docs/snapshots/phase18/assurance/history/phase18-codex-gpt56-c8d6dea-20260902t181312z/audit.codex.gpt56.requirements.jsonl]; fixed remediation outputs were tracked and clean
**Unresolved decisions**: none

## Current Finding Verification

| Source run / Finding ID | Verification | Source and final severity + rationale | Closure key/family | Batch | Decision |
|---|---|---|---|---|---|
| `phase18-codex-gpt56-a8bb454-20260903t021838z` / `P18-AUD-002` | Confirmed: GitHub has no run for the remediation head, the commit is not present on the remote, and no remote branch contains it. | Major -> Major: current local tests cannot substitute for the required real Opi/pi, native-verifier, and three-platform execution at the current code identity. | `phase18.current-head-native-and-platform-evidence` / `phase18.conformance-evidence` | B1 | `fix:publish-and-execute-current-head-evidence` |
| `phase18-codex-gpt56-a8bb454-20260903t021838z` / `P18-AUD-001` | Confirmed: the clean archive has no `crates/opi-eval/docs/seam-evidence-matrix.md`; the generic N-subject experiment resolves, so the missing derived result is the discriminating gap. | Major -> Major: the registered Phase source requires an artifact-derived shared/private/provisional classification across both Agents, all three revisions, and two verifier owners. | `phase18.artifact-derived-seam-matrix` / `phase18.conformance-evidence` | B2 | `fix:derive-and-check-current-head-seam-matrix` |

## Unresolved Decisions

none

## Closure Batches

### Batch B1: Produce exact-head native and platform evidence

**Closure predicate**: Commit `a8bb45426daf960d9e60024ce34542995c4dd2d1` is remotely resolvable; one successful manual `opi-eval-native-smoke.yml` run binds that exact candidate and verifies both real Agents, all three benchmark revisions, paired conformance-only outcomes, and at least two native-verifier owners; one same-repository pull-request CI run binds the same pull-request head and passes the required Linux, macOS, and Windows jobs.
**Dependencies**: none
**Verification union**: remote commit identity, pull-request head identity, required CI checks, native-smoke run metadata, downloaded artifact/upload-receipt identities, and `verify-native-smoke-artifact.py --criterion all-native`.

#### Fix B1.1: Publish and execute the sealed remediation head

- **Finding source(s)**: `phase18-codex-gpt56-a8bb454-20260903t021838z` + `3193940b79737046d5b977a0ad4670d17e1e6ae5fd7d2a133f68743f37986897` + `P18-AUD-002`
- **Decision**: `fix:publish-and-execute-current-head-evidence`
- **Verification status**: Confirmed
- **File(s)**: none; this batch creates external execution evidence only
- **Change kind**: metadata
- **Change**: After revalidating the audit-index digest, remediation head, and non-overlapping dirty inventory, push only the existing commit to `refs/heads/codex/phase18-remediation-evidence-a8bb454`, open one same-repository pull request to `main` without merging it, wait for its required CI checks, dispatch `.github/workflows/opi-eval-native-smoke.yml` with `candidate_sha=a8bb45426daf960d9e60024ce34542995c4dd2d1`, download both native-smoke artifacts into a unique temporary directory, and retain the resulting run, PR, artifact, digest, and platform evidence in the remediation result. Approval of this fixed plan authorizes only those named external writes; it does not authorize merging the PR, updating `main`, or deleting the evidence branch.
- **Closure predicate**: The exact remediation head has successful current native-smoke and same-repository three-platform CI evidence with independently verified artifact bytes and no paid provider.
- **Red-before**: `gh run list --repo OdradekAI/opi --commit a8bb45426daf960d9e60024ce34542995c4dd2d1 --limit 20 --json databaseId,workflowName,status,conclusion,headSha,event,url,createdAt; gh api repos/OdradekAI/opi/commits/a8bb45426daf960d9e60024ce34542995c4dd2d1` -> observed `FAIL: []`; the commit lookup returned HTTP 422 and `git branch -r --contains` returned no branch.
- **Green-after**: `gh pr view codex/phase18-remediation-evidence-a8bb454 --repo OdradekAI/opi --json headRefOid,url; gh pr checks codex/phase18-remediation-evidence-a8bb454 --repo OdradekAI/opi --required; python crates/opi-eval/scripts/verify-native-smoke-artifact.py --criterion all-native --expected-commit a8bb45426daf960d9e60024ce34542995c4dd2d1 --receipt <downloaded-upload-receipt> --artifact <downloaded-native-artifact>` -> expect matching head, all required CI checks successful across the declared platforms, every all-native criterion verified, and `evidence is conformance-only`.

### Batch B2: Restore the artifact-derived seam result

**Closure predicate**: The verified current-head native artifact deterministically yields an inspectable matrix that distinguishes the minimum shared Agent and benchmark contract, adapter-private evidence, and rejected or still-provisional hypotheses, while proving two real Agents, three benchmark revisions, and at least two independently owned native-verifier contracts.
**Dependencies**: B1
**Verification union**: artifact-verifier unit and negative CLI tests, current artifact generation/check mode, generic three-subject/fourth-benchmark validation, documentation contract, and whitespace check.

#### Fix B2.1: Derive and verify the current seam matrix

- **Finding source(s)**: `phase18-codex-gpt56-a8bb454-20260903t021838z` + `3193940b79737046d5b977a0ad4670d17e1e6ae5fd7d2a133f68743f37986897` + `P18-AUD-001`
- **Decision**: `fix:derive-and-check-current-head-seam-matrix`
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/scripts/verify-native-smoke-artifact.py`, `crates/opi-eval/scripts/test_verify_native_smoke_artifact.py`, `crates/opi-eval/docs/seam-evidence-matrix.md`
- **Change kind**: behavioral
- **Change**: Extend the existing artifact verifier rather than add a parallel parser. Add deterministic `--matrix-output <path>` generation and `--check-matrix <path>` verification after all-native validation succeeds. Derive shared fields only from evidence present for both Opi and pi and all three benchmark reports; classify product-only evidence as adapter-private; preserve verifier-owner-specific native result shapes; require Terminal-Bench and DeepSWE ownership coverage; and list unproved package, Rust type, process-envelope, trajectory, span, and directory-layout choices as rejected or provisional. Add focused tests for deterministic generation, drift rejection, missing-Agent/revision/second-owner rejection, and no output on failed artifact validation. Generate the checked-in matrix only from B1's verified artifact and bind its candidate SHA, run/artifact identities, and digests. No Chinese counterpart exists for this artifact-derived conformance record, so bilingual synchronization is not applicable.
- **Closure predicate**: Generation and check mode agree byte-for-byte on a matrix whose source identities bind B1 and whose classifications meet `P18-OUT-005` and `P18-A20` without promoting an unproved seam.
- **Red-before**: `python crates/opi-eval/scripts/verify-native-smoke-artifact.py --criterion all-native --expected-commit a8bb45426daf960d9e60024ce34542995c4dd2d1 --receipt __not_used_before_parser_closure__.json --artifact __not_used_before_parser_closure__.tar --matrix-output crates/opi-eval/docs/seam-evidence-matrix.md` -> observed `FAIL: unrecognized arguments: --matrix-output ...`; `Test-Path crates/opi-eval/docs/seam-evidence-matrix.md` also returned `False` in the clean archive.
- **Green-after**: `python crates/opi-eval/scripts/test_verify_native_smoke_artifact.py; python crates/opi-eval/scripts/verify-native-smoke-artifact.py --criterion all-native --expected-commit a8bb45426daf960d9e60024ce34542995c4dd2d1 --receipt <downloaded-upload-receipt> --artifact <downloaded-native-artifact> --matrix-output crates/opi-eval/docs/seam-evidence-matrix.md; python crates/opi-eval/scripts/verify-native-smoke-artifact.py --criterion all-native --expected-commit a8bb45426daf960d9e60024ce34542995c4dd2d1 --receipt <downloaded-upload-receipt> --artifact <downloaded-native-artifact> --check-matrix crates/opi-eval/docs/seam-evidence-matrix.md` -> expect focused tests PASS, all-native verification PASS, matrix written, and byte-exact check PASS.

## Final Verification

    python crates/opi-eval/scripts/test_verify_native_smoke_artifact.py
    python crates/opi-eval/scripts/verify-native-smoke-artifact.py --criterion all-native --expected-commit a8bb45426daf960d9e60024ce34542995c4dd2d1 --receipt <downloaded-upload-receipt> --artifact <downloaded-native-artifact> --matrix-output crates/opi-eval/docs/seam-evidence-matrix.md
    python crates/opi-eval/scripts/verify-native-smoke-artifact.py --criterion all-native --expected-commit a8bb45426daf960d9e60024ce34542995c4dd2d1 --receipt <downloaded-upload-receipt> --artifact <downloaded-native-artifact> --check-matrix crates/opi-eval/docs/seam-evidence-matrix.md
    cargo run -p opi-eval -- validate --config crates/opi-eval/tests/fixtures/experiment/generic-three-subject-fourth-benchmark.toml
    python scripts/opi-doc-check.py
    git diff --check
    gh pr view codex/phase18-remediation-evidence-a8bb454 --repo OdradekAI/opi --json headRefOid,url
    gh pr checks codex/phase18-remediation-evidence-a8bb454 --repo OdradekAI/opi --required

## Exclusions

none
