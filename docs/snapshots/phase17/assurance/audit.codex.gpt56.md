# Phase 17 Audit

**Audit run ID**: `phase17-codex-gpt56-52f66ee-20260824t183354z`
**Audit head**: `52f66ee8df2b600bd6233619acada9465a7dd148`
**Reviewer ID**: `codex`
**Model ID**: `gpt56`
**Reviewer identity**: Codex
**Reviewer model ID**: `gpt-5.6`
**Model identity source**: request-config
**Independence**: fresh-context-same-family; this run used a fresh audit context and excluded prior assurance, remediation, history, and sibling conclusions.
**Baseline policy**: latest-committed-spec
**Verdict**: FAIL

## Baseline Sources

| Path | SHA-256 | Registration note |
|---|---|---|
| `.opi-impl-state.json` | `de5420ab9fc38407704fb2cc1a0fc999061d1aa817706198db6910941b974133` | Current committed implementation ledger; its supplemental-source stored hash does not match the current committed source. |
| `docs/snapshots/phase17/opi-impl-state.json` | `801ac6d69b32acaa0f6301419c397c94450fc131e765ad9895c80c7cc33dd879` | Historical delivery pointer only; historical source hashes were not treated as the current baseline. |
| `docs/opi-spec.md` | `cc7f8898f60c0d8abaa667f4b49b7affc721412e75dd3a67dcde37a783e1bc4c` | Current committed parent specification. |
| `docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md` | `72f7e2996b0fbab7fd0c56d349b9dc0d5764c8a54e4ffd6cfc48245ac2bd4917` | Current committed registered supplemental source; authoritative despite the ledger's stored-hash mismatch. |

## Requirement Conformance

| Requirement ID | Criterion | Current evidence | State | Finding IDs |
|---|---|---|---|---|
| P17-AUTH-001 | ## Status and authority; table row P17-AUTH-001 | Registry/schema/authorizer/freshness negative and product permission tests passed. | met | P17-AUD-002 |
| P17-AUTH-002 | ## Status and authority; table row P17-AUTH-002 | Registry/schema/authorizer/freshness negative and product permission tests passed. | met | — |
| P17-AUTH-003 | ## Status and authority; table row P17-AUTH-003 | Registry/schema/authorizer/freshness negative and product permission tests passed. | met | — |
| P17-OUT-001 | ## Outcome; table row P17-OUT-001 | Collection-owned prepare/dispatch and two-provider product tests passed. | met | — |
| P17-OUT-002 | ## Outcome; table row P17-OUT-002 | Complete-state validation/apply/stop/queue and compaction tests passed. | met | — |
| P17-OUT-003 | ## Outcome; table row P17-OUT-003 | Registry/schema/authorizer/freshness negative and product permission tests passed. | met | — |
| P17-OUT-004 | ## Outcome; table row P17-OUT-004 | Typed graph, redaction, lifecycle, file adapter, failure, and manifest tests passed. | met | — |
| P17-PRV-001 | ## Dispatchable provider collection; table row P17-PRV-001 | Collection-owned prepare/dispatch and two-provider product tests passed. | met | — |
| P17-PRV-002 | ## Dispatchable provider collection; table row P17-PRV-002 | Collection-owned prepare/dispatch and two-provider product tests passed. | met | — |
| P17-PRV-003 | ## Dispatchable provider collection; table row P17-PRV-003 | Collection-owned prepare/dispatch and two-provider product tests passed. | met | — |
| P17-PRV-004 | ## Dispatchable provider collection; table row P17-PRV-004 | Collection-owned prepare/dispatch and two-provider product tests passed. | met | — |
| P17-PRV-005 | ## Dispatchable provider collection; table row P17-PRV-005 | Collection-owned prepare/dispatch and two-provider product tests passed. | met | — |
| P17-PRV-006 | ## Dispatchable provider collection; table row P17-PRV-006 | Collection-owned prepare/dispatch and two-provider product tests passed. | met | — |
| P17-NXT-001 | ## Atomic next-turn state; table row P17-NXT-001 | Complete-state validation/apply/stop/queue and compaction tests passed. | met | — |
| P17-NXT-002 | ## Atomic next-turn state; table row P17-NXT-002 | Complete-state validation/apply/stop/queue and compaction tests passed. | met | — |
| P17-NXT-003 | ## Atomic next-turn state; table row P17-NXT-003 | Complete-state validation/apply/stop/queue and compaction tests passed. | met | — |
| P17-NXT-004 | ## Atomic next-turn state; table row P17-NXT-004 | Complete-state validation/apply/stop/queue and compaction tests passed. | met | — |
| P17-NXT-005 | ## Atomic next-turn state; table row P17-NXT-005 | Complete-state validation/apply/stop/queue and compaction tests passed. | met | — |
| P17-NXT-006 | ## Atomic next-turn state; table row P17-NXT-006 | Complete-state validation/apply/stop/queue and compaction tests passed. | met | — |
| P17-AUT-001 | ## Trusted tool authorization; table row P17-AUT-001 | Registry/schema/authorizer/freshness negative and product permission tests passed. | met | — |
| P17-AUT-002 | ## Trusted tool authorization; table row P17-AUT-002 | Registry/schema/authorizer/freshness negative and product permission tests passed. | met | — |
| P17-AUT-003 | ## Trusted tool authorization; table row P17-AUT-003 | Registry/schema/authorizer/freshness negative and product permission tests passed. | met | — |
| P17-AUT-004 | ## Trusted tool authorization; table row P17-AUT-004 | Registry/schema/authorizer/freshness negative and product permission tests passed. | met | — |
| P17-AUT-005 | ## Trusted tool authorization; table row P17-AUT-005 | Registry/schema/authorizer/freshness negative and product permission tests passed. | met | — |
| P17-AUT-006 | ## Trusted tool authorization; table row P17-AUT-006 | Registry/schema/authorizer/freshness negative and product permission tests passed. | met | — |
| P17-AUT-007 | ## Trusted tool authorization; table row P17-AUT-007 | Registry/schema/authorizer/freshness negative and product permission tests passed. | met | — |
| P17-AUT-008 | ## Trusted tool authorization; table row P17-AUT-008 | Registry/schema/authorizer/freshness negative and product permission tests passed. | met | — |
| P17-EVD-001 | ## Product-neutral evidence seam; table row P17-EVD-001 | Typed graph, redaction, lifecycle, file adapter, failure, and manifest tests passed. | met | — |
| P17-EVD-002 | ## Product-neutral evidence seam; table row P17-EVD-002 | Typed graph, redaction, lifecycle, file adapter, failure, and manifest tests passed. | met | — |
| P17-EVD-003 | ## Product-neutral evidence seam; table row P17-EVD-003 | Typed graph, redaction, lifecycle, file adapter, failure, and manifest tests passed. | met | — |
| P17-EVD-004 | ## Product-neutral evidence seam; table row P17-EVD-004 | Typed graph, redaction, lifecycle, file adapter, failure, and manifest tests passed. | met | — |
| P17-EVD-005 | ## Product-neutral evidence seam; table row P17-EVD-005 | Typed graph, redaction, lifecycle, file adapter, failure, and manifest tests passed. | met | — |
| P17-EVD-006 | ## Product-neutral evidence seam; table row P17-EVD-006 | Typed graph, redaction, lifecycle, file adapter, failure, and manifest tests passed. | met | — |
| P17-EVD-007 | ## Product-neutral evidence seam; table row P17-EVD-007 | Typed graph, redaction, lifecycle, file adapter, failure, and manifest tests passed. | met | — |
| P17-EVD-008 | ## Product-neutral evidence seam; table row P17-EVD-008 | Typed graph, redaction, lifecycle, file adapter, failure, and manifest tests passed. | met | — |
| P17-EVD-009 | ## Product-neutral evidence seam; table row P17-EVD-009 | Typed graph, redaction, lifecycle, file adapter, failure, and manifest tests passed. | met | — |
| P17-EVD-010 | ## Product-neutral evidence seam; table row P17-EVD-010 | Typed graph, redaction, lifecycle, file adapter, failure, and manifest tests passed. | met | — |
| P17-EVD-011 | ## Product-neutral evidence seam; table row P17-EVD-011 | Typed graph, redaction, lifecycle, file adapter, failure, and manifest tests passed. | met | — |
| P17-FAL-001 | ## Failure, cancellation, and partial-result semantics; table row P17-FAL-001 | Boundary order, typed failure, cancellation, partial outcome, and rollback tests passed. | met | — |
| P17-FAL-002 | ## Failure, cancellation, and partial-result semantics; table row P17-FAL-002 | Boundary order, typed failure, cancellation, partial outcome, and rollback tests passed. | met | — |
| P17-FAL-003 | ## Failure, cancellation, and partial-result semantics; table row P17-FAL-003 | Boundary order, typed failure, cancellation, partial outcome, and rollback tests passed. | met | — |
| P17-FAL-004 | ## Failure, cancellation, and partial-result semantics; table row P17-FAL-004 | Boundary order, typed failure, cancellation, partial outcome, and rollback tests passed. | met | — |
| P17-MIG-001 | ## Compatibility and migration; table row P17-MIG-001 | Legacy route normalization and byte-preservation tests passed. | met | — |
| P17-MIG-002 | ## Compatibility and migration; table row P17-MIG-002 | Legacy route normalization and byte-preservation tests passed. | met | — |
| P17-MIG-003 | ## Compatibility and migration; table row P17-MIG-003 | Legacy route normalization and byte-preservation tests passed. | met | — |
| P17-MIG-004 | ## Compatibility and migration; table row P17-MIG-004 | Legacy route normalization and byte-preservation tests passed. | met | — |
| P17-MIG-005 | ## Compatibility and migration; table row P17-MIG-005 | Phase 17 cross-mode acceptance passed; RPC suite has an advisory load-sensitive timeout. | met | P17-AUD-003 |
| P17-MIG-006 | ## Compatibility and migration; table row P17-MIG-006 | Legacy route normalization and byte-preservation tests passed. | met | — |
| P17-PLT-001 | ## Platform scope; table row P17-PLT-001 | Windows acceptance passed and CI matrix is configured; current-head Linux/macOS runs are absent. | not-assessable | P17-AUD-001 |
| P17-PLT-002 | ## Platform scope; table row P17-PLT-002 | Windows acceptance passed and CI matrix is configured; current-head Linux/macOS runs are absent. | met | — |
| P17-PLT-003 | ## Platform scope; table row P17-PLT-003 | Windows acceptance passed and CI matrix is configured; current-head Linux/macOS runs are absent. | met | — |
| P17-A01 | ## Acceptance scenarios and verification; table row P17-A01 | Collection-owned prepare/dispatch and two-provider product tests passed. | met | — |
| P17-A02 | ## Acceptance scenarios and verification; table row P17-A02 | Collection-owned prepare/dispatch and two-provider product tests passed. | met | — |
| P17-A03 | ## Acceptance scenarios and verification; table row P17-A03 | Collection-owned prepare/dispatch and two-provider product tests passed. | met | — |
| P17-A04 | ## Acceptance scenarios and verification; table row P17-A04 | Complete-state validation/apply/stop/queue and compaction tests passed. | met | — |
| P17-A05 | ## Acceptance scenarios and verification; table row P17-A05 | Complete-state validation/apply/stop/queue and compaction tests passed. | met | — |
| P17-A06 | ## Acceptance scenarios and verification; table row P17-A06 | Registry/schema/authorizer/freshness negative and product permission tests passed. | met | — |
| P17-A07 | ## Acceptance scenarios and verification; table row P17-A07 | Registry/schema/authorizer/freshness negative and product permission tests passed. | met | — |
| P17-A08 | ## Acceptance scenarios and verification; table row P17-A08 | Registry/schema/authorizer/freshness negative and product permission tests passed. | met | — |
| P17-A09 | ## Acceptance scenarios and verification; table row P17-A09 | Typed graph, redaction, lifecycle, file adapter, failure, and manifest tests passed. | met | — |
| P17-A10 | ## Acceptance scenarios and verification; table row P17-A10 | Typed graph, redaction, lifecycle, file adapter, failure, and manifest tests passed. | met | — |
| P17-A11 | ## Acceptance scenarios and verification; table row P17-A11 | Typed graph, redaction, lifecycle, file adapter, failure, and manifest tests passed. | met | — |
| P17-A12 | ## Acceptance scenarios and verification; table row P17-A12 | Typed graph, redaction, lifecycle, file adapter, failure, and manifest tests passed. | met | — |
| P17-A13 | ## Acceptance scenarios and verification; table row P17-A13 | Legacy route normalization and byte-preservation tests passed. | met | — |
| P17-A14 | ## Acceptance scenarios and verification; table row P17-A14 | Phase 17 cross-mode acceptance passed; RPC suite has an advisory load-sensitive timeout. | met | P17-AUD-003 |
| P17-A15 | ## Acceptance scenarios and verification; table row P17-A15 | Windows acceptance passed and CI matrix is configured; current-head Linux/macOS runs are absent. | not-assessable | P17-AUD-001 |
| P17-RBK-001 | ## Risk thresholds and rollback; table row P17-RBK-001 | Rollback keeps state/artifacts/policy coherent in focused tests. | met | — |
| P17-RBK-002 | ## Risk thresholds and rollback; table row P17-RBK-002 | Rollback keeps state/artifacts/policy coherent in focused tests. | met | — |
| P17-RBK-003 | ## Risk thresholds and rollback; table row P17-RBK-003 | Rollback keeps state/artifacts/policy coherent in focused tests. | met | — |
| P17-RBK-004 | ## Risk thresholds and rollback; table row P17-RBK-004 | Rollback keeps state/artifacts/policy coherent in focused tests. | met | — |

## Standards Review

The current head keeps routing, Agent state, trusted authority, and evidence behind the existing deep crate boundaries. No removed Phase 17 runtime owner, provider proxy, alias registry, or compatibility shim was found. `cargo fmt --check --all`, workspace Clippy with warnings denied, and the documentation contract passed. English/Chinese sandbox disclaimers remain synchronized. The active ledger's registered supplemental SHA-256 is stale (P17-AUD-002), and the concurrent RPC JSONL suite has a bounded test reliability defect (P17-AUD-003).

## Spec Review

Provider preparation freezes one canonical route and one resolved auth result across retries. NextTurnState validation and application are atomic; stop and queues observe the required order. Tool execution resolves one trusted registration, validates final arguments, authorizes against immutable policy plus current evidence health, and fails closed before launch. Evidence records carry typed run/turn/call correlations, producer-side redaction, runtime-input binding, health generation, lifecycle, and manifest validation. Legacy sessions/traces are byte-preserved and all product modes share the focused runtime semantics. 68 mandatory records are met. P17-PLT-001 and P17-A15 are not-assessable because the runtime-changing audit head has no current Linux/macOS CI execution evidence (P17-AUD-001).

## Security, Invariants, Integration, Test Quality, and Residuals

Security and invariants: negative authorization tests prove that forged model/hook/extension content, missing authorizers, stale evidence generations, invalid schema, expired scope, and incomplete required evidence produce zero new tool launches. In-flight effects retain their actual outcomes. Provider and evidence debug/public paths redact credentials and canary content before sink entry.

Integration: 91 provider tests, 218 Agent tests, and 108 focused product Phase 17 tests passed on Windows. The configured CI matrix selects the same Phase 17 acceptance binaries on Ubuntu, macOS, and Windows, but no run exists for this unpushed audit head. That missing external execution is a blocking proof gap, not evidence of a known platform behavior defect.

Test quality/residuals: the full workspace gate exposed the RPC JSONL two-second receive deadline as load-sensitive. Five isolated reruns passed; one of three concurrent-binary reruns failed in three different tests; serial 80/80 passed. The finding is advisory because dedicated Phase 17 cross-mode acceptance passed and no deterministic runtime defect was reproduced.

## Minimum-change Conformance

| Task | Status | Current evidence |
|---|---|---|
| 17.1 | conforming | Collection-owned route/auth preparation is the dispatch seam; focused provider tests passed. |
| 17.2 | conforming | Durable complete NextTurnState is the single mutable next-request state; atomicity tests passed. |
| 17.3 | conforming | Storage-neutral typed evidence identities, health, lifecycle, and no-op/in-memory sinks remain in opi-agent. |
| 17.4 | conforming | Immutable registrations and mandatory trusted authorization are enforced with zero-execution negatives. |
| 17.5 | conforming | Product construction registers dispatchable routes once and resolves each call through the collection. |
| 17.6 | conforming | Provider/retry/tool/compaction/terminal evidence reconstructs one ordered graph. |
| 17.7 | conforming | File capture/finalization/redaction remains product-owned and artifact truthfulness tests passed. |
| 17.8 | conforming | Legacy route normalization is unique-or-typed-failure and legacy bytes remain unchanged. |
| 17.9 | not-assessable | Local cross-mode/failure/rollback/docs checks pass, but current-head Linux/macOS CI evidence is absent. |

## Findings

### P17-AUD-001: Current audit head lacks three-platform acceptance evidence

- Axis: test-quality
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P17-PLT-001, P17-A15
- Claim: The runtime-changing audit head is not present on GitHub and has no Actions/check run, so local Windows success cannot prove identical Linux/macOS/Windows semantics.
- Evidence: `.github/workflows/ci.yml:92-107` defines the matrix; `gh run list` returned no run; the check-runs API returned HTTP 422 `No commit found for SHA`; all 108 focused product tests passed only on the local Windows host.
- Refutation attempted: the matrix definition, current Windows acceptance, and historical Phase 17 CI evidence were checked. The historical run predates the current runtime-changing head, and configuration plus one OS cannot refute the missing current Linux/macOS executions.
- Suggested closure: publish the exact audit head and obtain green Phase 17 acceptance jobs on Ubuntu, macOS, and Windows.

### P17-AUD-002: Registered Phase 17 supplemental-source hash is stale

- Axis: standards
- Severity: Minor
- Conformance effect: advisory
- Requirement IDs: P17-AUTH-001
- Claim: `.opi-impl-state.json` stores `9709183…`, while the exact committed supplemental blob is `72f7e299…`; the ledger therefore does not attest the current registered source bytes.
- Evidence: `.opi-impl-state.json:9` and the independently hashed Git blob for `docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md`.
- Suggested closure: update the implementation ledger through its owning workflow so the registered source hash matches the current committed blob.

### P17-AUD-003: RPC JSONL suite has a load-sensitive two-second receive timeout

- Axis: test-quality
- Severity: Minor
- Conformance effect: advisory
- Requirement IDs: P17-MIG-005, P17-A14
- Claim: concurrent `rpc_jsonl` execution can exhaust the shared fixed two-second receive deadline and make the workspace gate nondeterministic.
- Evidence: `rpc_jsonl.rs:3044-3052`; one workspace failure; one of three concurrent binary reruns failed across three different tests; five isolated reruns and serial 80/80 passed.
- Suggested closure: make the RPC test synchronization condition-based or otherwise remove dependence on a load-sensitive fixed deadline, then stress the full binary concurrently.

## Verification Commands

| Command | Result | Requirement/finding |
|---|---|---|
| `cargo test -p opi-ai --test provider_collection --test per_request_auth --test provider_trait --test auth_contracts` | PASS: 91/91 | PRV, OUT-001, A01-A03 |
| `cargo test -p opi-agent --test agent_wrapper --test hooks_queues --test phase17_prepare_call --test evidence_contract --test evidence_runtime --test tool_authority --test tool_validation --test compaction --test agent_loop_semantics --test retry_agent` | PASS: 218/218 | NXT, AUT, EVD core, FAL core, A04-A12 |
| `cargo test -p opi-coding-agent --test phase17_provider_runtime --test phase17_tool_authority --test phase17_product_evidence --test phase17_legacy_migration --test phase17_cross_mode --test phase17_failure_rollback --test phase17_api_audit --test phase17_artifact_truthfulness` | PASS: 108/108 on Windows | Product PRV/AUT/EVD/MIG/FAL/RBK/A01-A15 |
| `cargo fmt --check --all` | PASS | Standards |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS | Standards |
| `python scripts/opi-doc-check.py` | ENVIRONMENT LIMIT: Windows denied the extracted directory-symlink path | Documentation |
| `python -c '<execute opi-doc-check.py in memory with .claude/skills resolved to its committed .agents/skills symlink target>'` | PASS | Documentation, using the committed symlink target |
| `cargo test --workspace --all-targets --quiet` in an archive-local Git repository | FAIL: one load-sensitive rpc_jsonl timeout after prior suites passed | P17-AUD-003 |
| `cargo test -p opi-coding-agent --test rpc_jsonl --quiet` three times | 1 FAIL / 2 PASS; failing run timed out in three set_model tests | P17-AUD-003 |
| focused failing RPC test five times | PASS: 5/5 | P17-AUD-003 counter-evidence |
| `cargo test -p opi-coding-agent --test rpc_jsonl --quiet -- --test-threads=1` | PASS: 80/80 | P17-AUD-003 counter-evidence |
| `gh run list --repo OdradekAI/opi --commit 52f66ee8df2b600bd6233619acada9465a7dd148 --workflow ci.yml --limit 10` | NO RUNS | P17-AUD-001 |
| `gh api repos/OdradekAI/opi/commits/52f66ee8df2b600bd6233619acada9465a7dd148/check-runs` | HTTP 422: commit absent | P17-AUD-001 |

## Verdict Rationale

The verdict is mechanically FAIL because two mandatory records are not-assessable: P17-PLT-001 and P17-A15. All 68 other mandatory records are met. P17-AUD-001 is Major and blocks. P17-AUD-002 and P17-AUD-003 are advisory Minor findings and do not change the already-failing verdict.
