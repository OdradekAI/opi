# Phase 18 Remediation Result

**Status**: COMPLETE
**Audit index SHA-256**: `305624349f650309f374ff8556114d5572c21b02fa30c48ee1ea2eaa95d78195`
**Plan SHA-256**: `cf084ff086d0edb963878973feee9e56198bdf10f6e7c34973e210823066e533`
**Changed paths**: ["crates/opi-eval/scripts/verify-native-smoke-artifact.py", "crates/opi-eval/scripts/test_verify_native_smoke_artifact.py", "crates/opi-eval/docs/seam-evidence-matrix.md"]
**Disposition artifact**: `remediation.result.dispositions.jsonl`

## Outcome

The apply attempt stopped at Batch B1 after the approved same-repository CI run failed. Both current findings remain **Not closed**. Batch B2 was not started because its B1 dependency did not close, and no incidental repair was admitted.

The `Changed paths` header is the disposition-contract union of B2's approved path attribution. None of those three production paths was modified during this stopped apply attempt.

## Batch B1: Not closed

- Pushed only commit `a8bb45426daf960d9e60024ce34542995c4dd2d1` to `refs/heads/codex/phase18-remediation-evidence-a8bb454`.
- Opened same-repository PR [#7](https://github.com/OdradekAI/opi/pull/7) against `main`; its `headRefOid` is the exact remediation head. The PR remains open and was not merged.
- CI run [33783810402](https://github.com/OdradekAI/opi/actions/runs/33783810402) completed with 21 passing and 3 failing jobs. Windows workspace tests passed.
- Ubuntu and macOS workspace tests both failed `agent_conformance_matrix_settles_every_pinned_case`: the Opi `provider-fixture` case exited `1` instead of `0` at `crates/opi-eval/tests/agent_integration_conformance.rs:175`.
- macOS execution acceptance failed `fixture::process_command_adapter_protocol_violation_lifts_stable_code`: it observed `bash backend error: cleanup_unconfirmed` instead of `bash backend error: protocol_violation` at `crates/opi-coding-agent/tests/execution_runtime.rs:817`.
- `gh pr checks --required` reported that the branch has no configured required-check subset. The complete declared CI job set was therefore inspected directly.
- The native-smoke workflow was not dispatched because the prerequisite CI evidence was not green.

The failed jobs block B1. Repairing either code path would change the exact candidate head; the `opi-coding-agent` failure is also outside B1's empty production-path set. The bounded incidental-repair guardrails therefore do not permit a repair under this approved plan.

## Batch B2: Not closed

B2 depends on a verified native artifact from B1. Because B1 stopped before native-smoke dispatch, the verifier extension, focused tests, matrix generation, and matrix check were not run. The three approved B2 production paths remain unchanged.

## Verification

- `python .agents/skills/_shared/scripts/validate_assurance_artifact.py audit-set docs/snapshots/phase18/assurance` -> `PASS`
- `python .agents/skills/_shared/scripts/validate_assurance_artifact.py plan docs/snapshots/phase18/assurance/remediation.plan.md` -> `PASS plan_sha256=cf084ff086d0edb963878973feee9e56198bdf10f6e7c34973e210823066e533`
- `gh pr view codex/phase18-remediation-evidence-a8bb454 --repo OdradekAI/opi --json number,state,headRefOid,headRefName,baseRefName,url` -> PR `#7`, open, exact head, base `main`
- `gh pr checks codex/phase18-remediation-evidence-a8bb454 --repo OdradekAI/opi` -> 21 pass, 3 fail
- `gh run list --repo OdradekAI/opi --commit a8bb45426daf960d9e60024ce34542995c4dd2d1 --limit 20 --json databaseId,workflowName,status,conclusion,headSha,event,url,createdAt` -> only failed CI run `33783810402`; no native-smoke run

## Materialization Boundary

The evidence branch and PR are external writes retained by the approved plan; the PR is unmerged. Locally, this apply attempt replaced only the two fixed remediation result artifacts. It preserved the carried-in live audit set, plan, and history inventory and made no production-code, test, documentation, manifest, lockfile, schema, specification, authority, or implementation-ledger edit.

A new remediation plan is required before any CI repair, candidate-head change, native-smoke dispatch for a different head, or B2 implementation. This result does not grant Phase 18 conformance.
