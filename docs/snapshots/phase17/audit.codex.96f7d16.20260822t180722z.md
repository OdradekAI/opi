# Phase 17 Audit

**Audit head**: `96f7d161045c94113ec9f02f5ad3ff4c8121cea5`  
**Reviewer/model**: OpenAI Codex (GPT-5)  
**Independence**: `fresh-context-same-family`; this was a new review context, but the repository already contains same-family Phase 17 audit artifacts. Historical conclusions were opened after the first two current findings were sealed but before the full gate and boundary sweep completed. The three later recurring findings below were independently reproduced at the audit head, but their search direction may be historically correlated.  
**Run ID**: `20260822t180722z`  
**Contamination/isolation**: the main worktree was clean at audit start. Source inspection used the pinned Git object; executable checks ran in detached worktree `D:\Luiz\Odradek\opi-audit-96f7d16` with the repository-resolved external Cargo cache. Only audit-owned artifacts were added to the main worktree.  
**Verdict**: **FAIL**

The normalized source of truth is the sibling `audit.codex.96f7d16.20260822t180722z.findings.jsonl`. This report references those IDs and adds audit context.

## Executive Summary

Phase 17 retains broad, hermetic coverage and most of the registered runtime cutover is present: dispatch goes through a collection-owned prepared call, next-turn state commits atomically, trusted registrations and authorizers gate tool execution, evidence identities and manifests are typed, legacy Phase 17 fixtures remain byte-stable, and cross-mode acceptance tests pass.

The phase does not satisfy its current acceptance baseline. Two Major defects remain: the durable session reader treats every unknown entry type as ignorable instead of distinguishing required from ignorable semantics, and the Bedrock event-stream parser silently consumes integrity-invalid frames. Three Minor gaps also remain: the exact post-Allow evidence-health transition does not reauthorize, the required workspace test gate fails under the repository's mandated external cache workflow, and the recorded rollback proof explicitly omits the required pre-Phase regression profile.

| Severity | Count |
|---|---:|
| Blocker | 0 |
| Major | 2 |
| Minor | 3 |
| Info | 0 |

## Requirement Conformance

`met` means the current committed source plus executable/static evidence satisfies the criterion. `partially-met` identifies a concrete uncovered or contradicting path and links its normalized finding.

| Requirement | Criterion source | Evidence | Requirement state | Finding IDs |
|---|---|---|---|---|
| P17-OUT-001 | Phase design, Outcome | `ProviderCollection::prepare_call`; `phase17_provider_runtime` | `met` | — |
| P17-OUT-002 | Phase design, Outcome | `NextTurnState`; `hooks_queues`, `agent_wrapper` | `met` | — |
| P17-OUT-003 | Phase design, Outcome | registration → hook → schema → authorizer → execute in `agent_loop.rs` | `met` | — |
| P17-OUT-004 | Phase design, Outcome | evidence contract/runtime/product manifest suites | `met` | — |
| P17-PRV-001 | Per-call route/auth preparation | collection-owned preparation and dispatch tests | `met` | — |
| P17-PRV-002 | Per-call route/auth preparation | canonical/ambiguous route tests in `provider_collection` and `phase17_provider_runtime` | `met` | — |
| P17-PRV-003 | Per-call route/auth preparation | frozen prepared call reused across retry; `phase17_prepare_call` | `met` | — |
| P17-PRV-004 | Per-call route/auth preparation | typed route/auth failures and no-fallback tests | `met` | — |
| P17-PRV-005 | Per-call route/auth preparation | typed requested/resolved/actual/auth/fallback evidence | `met` | — |
| P17-PRV-006 | Per-call route/auth preparation | provider wire implementations remain behind `Provider`/`Request`/`EventStream` | `met` | — |
| P17-NXT-001 | Atomic next-turn state | complete replacement type and public persistence tests | `met` | — |
| P17-NXT-002 | Atomic next-turn state | intrinsic validation before one replace; rollback/cancel tests | `met` | — |
| P17-NXT-003 | Atomic next-turn state | `phase17_stop_observes_complete_next_turn_state` | `met` | — |
| P17-NXT-004 | Atomic next-turn state | stop-before-queue tests in `hooks_queues` | `met` | — |
| P17-NXT-005 | Atomic next-turn state | compaction replaces context through complete-state transition | `met` | — |
| P17-NXT-006 | Atomic next-turn state | next call routes from applied state | `met` | — |
| P17-AUTH-001 | Status and authority | current parent INV-007 is not implemented by session v1 | `partially-met` | P17-CODEX-001 |
| P17-AUTH-002 | Status and authority | status remains in implementation ledger/snapshots, not normative spec | `met` | — |
| P17-AUTH-003 | Status and authority | no silent exception added to the normative documents | `met` | — |
| P17-EVD-001 | Product-neutral evidence seam | typed IDs and monotonic allocator; `evidence_contract` | `met` | — |
| P17-EVD-002 | Product-neutral evidence seam | provider/retry/tool/compaction/terminal graph tests | `met` | — |
| P17-EVD-003 | Product-neutral evidence seam | strict manifest and direct-vs-snapshot validation | `met` | — |
| P17-EVD-004 | Product-neutral evidence seam | typed measurements and explicit unknown reasons | `met` | — |
| P17-EVD-005 | Product-neutral evidence seam | producer-boundary canary/redaction tests | `met` | — |
| P17-EVD-006 | Product-neutral evidence seam | default harness emits no evidence | `met` | — |
| P17-EVD-007 | Product-neutral evidence seam | setup failure stops before provider/tool dispatch | `met` | — |
| P17-EVD-008 | Product-neutral evidence seam | failure advances health and withholds manifest | `met` | — |
| P17-EVD-009 | Product-neutral evidence seam | launch fails closed and in-flight outcomes remain typed; exact reauthorization gap is owned by A12 | `met` | P17-CODEX-003 |
| P17-EVD-010 | Product-neutral evidence seam | core adapter guard permits only no-op/in-memory | `met` | — |
| P17-EVD-011 | Product-neutral evidence seam | no-op/in-memory/file lifecycle conformance | `met` | — |
| P17-AUT-001 | Trusted tool authorization | immutable `RegisteredTool` resolved before hook/schema/auth | `met` | — |
| P17-AUT-002 | Trusted tool authorization | final args schema-validated and forwarded unchanged | `met` | — |
| P17-AUT-003 | Trusted tool authorization | `ToolAuthorizationRequest` binds policy/scope/current health | `met` | — |
| P17-AUT-004 | Trusted tool authorization | forgery and model-content expansion tests | `met` | — |
| P17-AUT-005 | Trusted tool authorization | missing/error/deny/stale/invalid paths execute zero times | `met` | — |
| P17-AUT-006 | Trusted tool authorization | hooks expose Continue/Deny only | `met` | — |
| P17-AUT-007 | Trusted tool authorization | after-call transformation cannot mutate later authority | `met` | — |
| P17-AUT-008 | Trusted tool authorization | provider schema rebuilt from active trusted registrations each request | `met` | — |
| P17-FAL-001 | Failure semantics | Bedrock integrity failure has no typed error | `partially-met` | P17-CODEX-002 |
| P17-FAL-002 | Failure semantics | CRC-invalid frame is consumed and later frames may continue | `partially-met` | P17-CODEX-002 |
| P17-FAL-003 | Failure semantics | cancellation/evidence/partial outcomes remain typed; queue behavior covered | `met` | — |
| P17-FAL-004 | Failure semantics | provider/tool/evidence public-boundary redaction suites | `met` | — |
| P17-MIG-001 | Compatibility and migration | supported legacy fixtures readable and byte-identical | `met` | — |
| P17-MIG-002 | Compatibility and migration | legacy bare route normalizes uniquely or fails typed | `met` | — |
| P17-MIG-003 | Compatibility and migration | Reference Product file evidence adapter | `met` | — |
| P17-MIG-004 | Compatibility and migration | legacy trace coexistence/byte-preservation tests | `met` | — |
| P17-MIG-005 | Compatibility and migration | `phase17_cross_mode` | `met` | — |
| P17-MIG-006 | Compatibility and migration | API-removal source guard | `met` | — |
| P17-PLT-001 | Platform scope | platform-neutral core and three-OS CI matrix definition | `met` | — |
| P17-PLT-002 | Platform scope | hermetic endpoint/source guard and mock providers | `met` | — |
| P17-PLT-003 | Platform scope | bilingual non-sandbox boundary documentation | `met` | — |
| P17-RBK-001 | Risk/rollback | this audit blocks exit on the observed thresholds | `met` | — |
| P17-RBK-002 | Risk/rollback | snapshot explicitly records no live revert/pre-Phase regression profile | `partially-met` | P17-CODEX-005 |
| P17-RBK-003 | Risk/rollback | rollback artifact byte-preservation fixture | `met` | — |
| P17-RBK-004 | Risk/rollback | policy digest/denial unchanged across rollback fixture | `met` | — |
| P17-A01 | Acceptance scenarios | cross-provider collection/harness/evidence tests | `met` | — |
| P17-A02 | Acceptance scenarios | route/auth/wire failures dispatch zero requests | `met` | — |
| P17-A03 | Acceptance scenarios | retry retains route/parent/terminal evidence | `met` | — |
| P17-A04 | Acceptance scenarios | complete state observed at stop | `met` | — |
| P17-A05 | Acceptance scenarios | invalid/cancelled candidate preserves prior state and skips later boundaries | `met` | — |
| P17-A06 | Acceptance scenarios | model-content permission expansion denied | `met` | — |
| P17-A07 | Acceptance scenarios | extension/builtin registration laundering rejected | `met` | — |
| P17-A08 | Acceptance scenarios | expired/error authority executes zero and reports safe source | `met` | — |
| P17-A09 | Acceptance scenarios | complete graph/manifest reconstruction test | `met` | — |
| P17-A10 | Acceptance scenarios | canaries absent from sinks/files/diagnostics/metadata | `met` | — |
| P17-A11 | Acceptance scenarios | actual outcome retained, evidence incomplete, no manifest | `met` | — |
| P17-A12 | Acceptance scenarios | post-Allow sink failure returns before a second authorization | `partially-met` | P17-CODEX-003 |
| P17-A13 | Acceptance scenarios | legacy session/trace coexistence and byte identity | `met` | — |
| P17-A14 | Acceptance scenarios | one fixture across Reference Product modes | `met` | — |
| P17-A15 | Acceptance scenarios | committed three-platform acceptance job and platform-neutral guard | `met` | — |
| Phase-exit gate set | Phase design lines 785-796; AGENTS.md Verification | `cargo test --workspace --all-targets` fails under required external cache | `partially-met` | P17-CODEX-004 |

## Standards Review

The workspace topology and inward dependency direction match the repository contract. Phase 17 additions reuse the existing provider, Agent loop, hook, execution, session, and evidence seams; no new feature flag, compatibility layer, or hypothetical core adapter was observed. Public boundary errors and evidence records are generally typed and redacted.

One standards/invariant defect remains: `P17-CODEX-002`. The Bedrock parser converts CRC and structural frame failures into `None`, after already draining the complete frame. The caller receives neither a typed protocol error nor an incomplete buffer, so corrupted content can disappear and a later clean terminal frame can produce a normal completion.

## Spec Review

Three registered-contract gaps remain:

- `P17-CODEX-001`: Phase criterion P17-AUTH-001 incorporates the current parent clauses. INV-007 requires durable entries to be classified as required or explicitly ignorable and unknown required semantics to fail closed. The v1 reader instead skips every unknown `type` tag and has no durable Runtime Input Binding entry.
- `P17-CODEX-003`: P17-A12 requires a stale Allow to be reauthorized after evidence health changes. The code reauthorizes only a decision already stale against the snapshot passed into `authorize_and_verify`; a later authorization-record emission failure advances health and immediately returns `BatchInvalid`.
- `P17-CODEX-005`: P17-RBK-002 names a revert review and pre-Phase regression profile. The frozen evidence explicitly records that only structural review was performed.

## Security, Invariants, Integration, Test Quality, and Residuals

Authorization remains fail-closed in the observed A12 gap: no tool launches after the evidence-record failure, so `P17-CODEX-003` is a completeness/spec defect rather than an unsafe execution. The integrity issue in `P17-CODEX-002` is more serious because corrupted upstream content is silently lost at a protocol boundary.

The full workspace test gate is not portable to the repository's mandated external Cargo cache. `P17-CODEX-004` reproduces on Windows at the pinned head: five `session_cli` E2E tests compute `<workspace>/target/debug/opi.exe` while Cargo built the binary under the resolved external target directory. Focused Phase 17 suites, formatting, linting, doctests, and rustdoc all passed.

Historical comparison was performed only after the initial current finding seal. All five findings have same-family historical lineage and therefore are correlated persistence, not independent consensus:

| Current ID | Historical lineage | Current status |
|---|---|---|
| P17-CODEX-001 | legacy `P17-CODEX-SPEC-001`; remediation cluster C11 | independently reproduced, unresolved |
| P17-CODEX-002 | legacy `glm53-018`; remediation cluster C22 | independently reproduced, unresolved |
| P17-CODEX-003 | legacy `P17-CODEX-SPEC-006`; remediation cluster C15 | independently reproduced, unresolved |
| P17-CODEX-004 | legacy `P17-CODEX-TST-001`; remediation cluster C21 | independently reproduced, unresolved |
| P17-CODEX-005 | legacy `P17-CODEX-SPEC-009`; remediation cluster C18 | independently reproduced, unresolved |

## Minimum-change Conformance

The nine tasks contain the required reuse, placement, public-surface, production-slice, and ceiling trace. No task records a simplification trigger; the graph is pre-contract for that field, so omission is historical context rather than a finding.

| Task | Scenario/source | Reuse | Placement | Surface | Production slice | Ceiling/trigger | Status |
|---|---|---|---|---|---|---|---|
| 17.1 | OUT-001; PRV-001..006 | provider collection/auth/registry | `opi-ai` core | opaque prepared call | Agent loop and product routes | no router/alias/breadth | `conforming` |
| 17.2 | OUT-002; NXT-001..006 | Agent loop/hooks/queues/compaction | `opi-agent` core | complete `NextTurnState` | public Agent operations | no patch/facade | `conforming` |
| 17.3 | EVD-001..011 | trace/event lifecycle vocabulary | `opi-agent` core | typed IDs, health, binding, sink | Agent evidence runtime | no file/exporter | `conforming` |
| 17.4 | OUT-003; AUT-001..008 | tools/hooks/execution policy | mechanism core, policy product | registration/authorizer | product assembly and loop | no generic policy/allow-all | `conforming` |
| 17.5 | OUT-001; PRV product cutover | provider factory/harness/session | Reference Product | collection and canonical state | all product startup paths | no aliases/eager auth/second registry | `conforming` |
| 17.6 | evidence runtime expansion | loop lifecycle | `opi-agent` core | sink binding and health | Agent prompt/continue/retry | no product adapter/exporter | `conforming` |
| 17.7 | OUT-004; evidence product cutover | capture paths/runners | Reference Product | file adapter/direct binding/strict manifest | product modes | one adapter, no fabricated snapshot | `conforming` |
| 17.8 | MIG-001/002/004 | session/route/legacy trace | Reference Product | typed normalization/remediation | resume/fork/session CLI | no reader/rewrite/guess/shim | `conforming` (acceptance gap linked to P17-CODEX-001) |
| 17.9 | assurance/rollback/platform | existing CI/docs/test modes | assurance only | no new runtime seam | cross-mode/failure/API suites | no runtime/dependency expansion | `conforming` (proof gaps linked to P17-CODEX-004/005) |

## Findings

### P17-CODEX-001: Unknown session entries bypass required/ignorable classification

- Axis: `spec`
- Severity: Major
- Summary: the v1 reader classifies every unknown entry type as forward-compatible and nonfatal. It cannot reject an unknown required semantic or reconstruct a durable runtime-input binding, contrary to current INV-007 as incorporated by P17-AUTH-001.
- Refutation attempted: unsupported header versions do fail closed and malformed known entries are counted as corrupt. This does not refute the finding because unknown entry tags carry no required/ignorable classification at all.
- Suggested closure: make the durable envelope encode whether an entry is required or ignorable, fail closed on unsupported required semantics, and bind the validated committed session prefix to immutable runtime input before resume.

### P17-CODEX-002: Bedrock CRC-invalid frames are silently consumed

- Axis: `invariants`
- Severity: Major
- Summary: `parse_frames` drains a complete frame before `parse_single_frame` validates CRC/structure; invalid frames return `None` and disappear without a typed error.
- Refutation attempted: incomplete frames remain buffered and HTTP transport errors are typed. Neither covers a complete integrity-invalid frame, and the dedicated unit test explicitly asserts silent consumption.
- Suggested closure: return a typed terminal protocol failure for a malformed complete frame and stop subsequent stream production; add fixture and HTTP-loop regressions proving no later `Done` follows corruption.

### P17-CODEX-003: Post-Allow evidence failure bypasses required reauthorization

- Axis: `spec`
- Severity: Minor
- Summary: tool execution fails closed, but the exact P17-A12 transition does not call the trusted authorizer again after authorization-evidence emission changes `EvidenceHealth`.
- Suggested closure: after a post-decision health generation change, reauthorize the same immutable registration/capability/arguments against the new snapshot before deciding whether the batch may launch, and assert the second call in a focused regression.

### P17-CODEX-004: Workspace gate ignores the resolved Cargo target directory

- Axis: `test-quality`
- Severity: Minor
- Summary: five `session_cli` subprocess tests hard-code `<workspace>/target/debug/opi`, so the mandated `cargo test --workspace --all-targets` gate fails when the repository's external Cargo cache is active.
- Suggested closure: consume Cargo's integration-test binary path or the resolved target directory, then rerun the exact workspace gate with the cache lease active.

### P17-CODEX-005: Rollback proof omits the required pre-Phase regression profile

- Axis: `spec`
- Severity: Minor
- Summary: the recorded P17-RBK-002 evidence substitutes a structural scan and explicitly states that no live revert/pre-Phase regression profile was executed.
- Suggested closure: run the registered non-destructive rollback profile in an isolated checkout/worktree and preserve the exact commit range, commands, and outcomes as immutable evidence.

## Verification Commands

All Cargo commands below ran in the detached audit worktree with `CARGO_TARGET_DIR` obtained from `python scripts/opi-cargo-cache.py resolve` and an active cache lease.

| Command | Result | Obligation/finding |
|---|---|---|
| `python scripts/opi-doc-check.py` | PASS | documentation contracts |
| `cargo fmt --check --all` | PASS | formatting gate |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS | lint gate |
| `cargo test -p opi-ai --test provider_collection --test per_request_auth --test auth_contracts --test phase14_remediation` | PASS, 84 tests | provider route/auth substrate |
| `cargo test -p opi-agent --test agent_wrapper --test hooks_queues --test compaction --test phase17_prepare_call --test evidence_contract --test evidence_runtime --test tool_authority --test tool_validation --test provider_public_safety --test event_public_boundary --test run_id_process --test tool_event_redaction` | PASS, 208 passed and one subprocess helper ignored | next-turn/evidence/authority/failure |
| `cargo test -p opi-coding-agent --test phase17_provider_runtime --test phase17_tool_authority --test phase17_product_evidence --test phase17_legacy_migration --test phase17_cross_mode --test phase17_failure_rollback --test phase17_api_audit --test provider_route_diagnostics` | PASS, 107 tests | product integration/acceptance |
| `cargo test -p opi-agent --test session_storage crash_recovery_reports_unknown_future_type_separately_from_corrupt -- --exact` | PASS; confirms unknown entry is skipped | P17-CODEX-001 |
| `cargo test -p opi-agent --test evidence_runtime parallel_authorization_record_failure_on_first_or_second_launches_zero_tools -- --exact` | PASS; confirms fail-closed behavior but not reauthorization | P17-CODEX-003 |
| `cargo test -p opi-ai bedrock::event_stream::tests::parse_frames_ignores_bad_crc_without_panic -- --exact` | PASS; confirms CRC-invalid frame disappears | P17-CODEX-002 |
| `cargo test --workspace --all-targets` | FAIL; 5 `session_cli` E2E tests cannot find `<workspace>/target/debug/opi.exe` | P17-CODEX-004 |
| `cargo test --workspace --doc` | PASS, 12 doctests | documentation code examples |
| `$env:RUSTDOCFLAGS='-D warnings'; cargo doc --workspace --no-deps` | PASS | rustdoc gate |

Test impact: `none` (audit artifacts only; no source or test file changed).

## Verdict Rationale

The verdict is **FAIL** because mandatory criteria P17-AUTH-001, P17-FAL-001, P17-FAL-002, P17-RBK-002, and P17-A12 are only partially met, and the required phase-exit workspace test gate fails in the repository-prescribed cache environment. The verdict follows requirement state, independently of the severity count.
