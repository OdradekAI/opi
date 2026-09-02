# Phase 18 seam-evidence matrix

Derived by `crates/opi-eval/scripts/derive-phase18-seam-matrix.py` from the
sealed Phase 18 native artifact. Conformance-only evidence:
no score, leaderboard, or product claim is made here.

## Binding

| Field | Value |
|---|---|
| candidate_commit | `27344e3aaf03d38eaa53c7af19c777efbe9be213` |
| run_id | `33271354427` |
| artifact_digest | `12892746e012abc6a73142d8d968b39d68ef978872cb445f2a438f4771981539` |
| sealed_manifest_sha256 | `7ddadca45f155a6024ccde88c8c94414e4885fd2b95a39c389dbd5495aeff99d` |
| trials | `6` |
| matrix_schema | `phase18-seam-evidence-matrix/1` |

## Shared fields (both real Agents, all three revisions)

Frozen by shared conformance; present in every trial
receipt of both products.

| Field |
|---|
| `agent.boundary` |
| `agent.cleanup` |
| `agent.completion` |
| `agent.exit_state` |
| `agent.failure_kind` |
| `agent.product` |
| `agent.stderr_bytes` |
| `agent.stderr_truncated` |
| `agent.stdout_bytes` |
| `agent.stdout_truncated` |
| `verifier.boundary` |
| `verifier.completion` |
| `verifier.exit_state` |
| `verifier.failure_kind` |
| `verifier.reward` |

## Shared staged evidence

| Artifact path |
|---|
| `native/agent-stderr.log` |
| `native/agent-stdout.log` |
| `native/authority-ledger.json` |
| `native/verifier-stderr.log` |
| `native/verifier-stdout.log` |

## Adapter-private fields (opi)

| Field |
|---|

## Adapter-private staged evidence (opi)

| Artifact path |
|---|
| `native/evidence/manifest` |
| `native/evidence/records` |

## Adapter-private fields (pi)

| Field |
|---|

## Adapter-private staged evidence (pi)

| Artifact path |
|---|
| `native/agent-answer.txt` |
| `native/events/stdout` |

## Rejected (never unified into the shared contract)

Facts that remain per-adapter native values or fork by
native-verifier ownership; the shared contract refuses
to translate them into parity.

| Class | Product | Field |
|---|---|---|
| verifier-forked | both | `native/native/harbor-result` |
| verifier-forked | both | `native/native/pier-result` |

## Verifier ownership

| Revision | Native verifier owner |
|---|---|
| terminal-bench-2.1 | Terminal-Bench (harbor aggregate) |
| terminal-bench-3.0 | Terminal-Bench (harbor aggregate) |
| deepswe-v1.1 | DeepSWE (pier job result) |
