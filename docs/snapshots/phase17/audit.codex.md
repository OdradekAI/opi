# Phase 17 Deep Agent Core Semantic Closure -- Independent Code Audit

**Auditor**: codex (fresh-context-same-family review; no prior audit reports consulted)
**Date**: 2026-08-20
**Scope**: Phase 17 registered requirements and Tasks 17.1--17.9
**Implementation target**: `a680c5df13a08d5a2abc48b482a69d1c594f288e` (current committed implementation)
**Phase exit commit**: `a4cfa4ddc74b4dfac59b4305d4657599af866480` (provenance only)
**History use**: provenance and discovery only; no diff coverage boundary
**Method**: Read the committed Phase ledger, both registered specifications, repository standards, relevant production paths, and focused tests. Coverage was derived from the 55 registered P17 criteria, P17-A01--P17-A15, all nine task definitions of done, and their claimed evidence. Standards and Spec reviews were performed separately; uncommitted content was excluded.

---

## 1. Executive Summary

**Verdict: FAIL**

| Severity | Count |
|----------|-------|
| Blocker  | 0 |
| Major    | 4 |
| Minor    | 3 |
| Info     | 0 |

Provider preparation, complete next-turn replacement, legacy artifact preservation, and most evidence lifecycle mechanics are coherent. However, Phase 17 does not consistently fail closed after evidence failure, accepts ambiguous interactive bare model selection by guessing the active provider, retains unavailable tools in a later provider-facing projection, and exposes raw prompt/tool-result content through public event outputs.

### Per-task summary

| Task | Title | Verdict |
|------|-------|---------|
| 17.1 | Collection-owned route and authentication preparation | Pass |
| 17.2 | Durable atomic `NextTurnState` | Pass |
| 17.3 | Evidence identities, health, and lifecycle | Pass with findings |
| 17.4 | Trusted registrations and authorization | Pass with findings |
| 17.5 | Reference Product dispatchable routes | Fail |
| 17.6 | Agent evidence runtime | Fail |
| 17.7 | Product evidence, finalization, and redaction | Fail |
| 17.8 | Legacy route/session and trace migration | Pass |
| 17.9 | Cross-mode, rollback, documentation, and CI acceptance | Pass with findings |

## 2. Standards Findings

### 2.1 MAJOR: Pre-dispatch evidence failure still starts the provider request

**File:** `crates/opi-agent/src/agent_loop.rs`
**Lines:** 338--373

`emit_evidence` returns `false` and advances `EvidenceHealth` when the pre-dispatch Provider record cannot be emitted, but the result is discarded and `prepared.start_attempt()` runs unconditionally. The provider request is therefore launched after evidence is already incomplete.

**Impact:** A capture-configured run violates the fail-closed boundary for the current not-yet-launched model request. The existing test confirms only that the subsequent tool is denied; its configured provider response confirms that the provider attempt occurred.

**Fix:** Treat a failed pre-dispatch Provider emission as a terminal pre-launch error whenever complete evidence is required, and add a counter-based regression asserting zero provider starts.

```yaml
id: AUD-17-001
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex-gpt5.md
source_model: codex
independence: fresh-context-same-family
axis: standards
severity: Major
title: Pre-dispatch evidence failure still starts the provider request
claim: A failed pre-dispatch Provider evidence emission advances EvidenceHealth but does not prevent the current model request from starting.
evidence:
  - location: crates/opi-agent/src/agent_loop.rs:338-373
    detail: The return value of emit_evidence is ignored before prepared.start_attempt().
  - location: crates/opi-agent/src/agent_loop.rs:2108-2114
    detail: emit_evidence advances EvidenceHealth and returns false on emission failure.
  - location: crates/opi-agent/tests/evidence_runtime.rs:1071-1119
    detail: The regression fixture injects provider-record emission failure and only asserts later tool execution is zero.
criterion_source: P17-EVD-009; AGENTS.md fail-closed authority/evidence-boundary rule
reproduction:
  - cargo test -p opi-agent --test evidence_runtime emission_failure_advances_health_copied_into_authorization
confidence: high
status: unverified
```

### 2.2 MINOR: Closed state contracts reserve speculative variants

**Files:** `crates/opi-ai/src/auth.rs`, `crates/opi-agent/src/authority.rs`, `crates/opi-agent/src/evidence.rs`
**Lines:** `auth.rs:194,253`; `authority.rs:253`; `evidence.rs:1777`

`AuthProvenanceSource`, `AuthFallback`, `AuthorizationDecision`, and `TerminalOutcome` are documented by Phase 17 as closed semantic states but are marked `#[non_exhaustive]`. This forces downstream wildcard handling and reserves future semantic surface despite the Phase's explicit closed-state design.

**Fix:** Remove `#[non_exhaustive]` from semantic states that the design declares closed, or revise the registered specification if extensibility is intentional.

```yaml
id: AUD-17-002
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex-gpt5.md
source_model: codex
independence: fresh-context-same-family
axis: standards
severity: Minor
title: Closed state contracts reserve speculative variants
claim: Public types specified as closed state spaces are marked non_exhaustive.
evidence:
  - location: crates/opi-ai/src/auth.rs:194
    detail: AuthProvenanceSource is marked non_exhaustive.
  - location: crates/opi-agent/src/authority.rs:252-255
    detail: AuthorizationDecision is described as closed but marked non_exhaustive.
criterion_source: AGENTS.md closed-enum and no-speculative-abstraction rules; Phase 17 trusted authorization and per-call auth preparation
reproduction:
  - git show a680c5df13a08d5a2abc48b482a69d1c594f288e:crates/opi-ai/src/auth.rs
confidence: high
status: unverified
```

### 2.3 MINOR: Production comments retain task history

**File:** `crates/opi-agent/src/agent.rs`
**Line:** 572

The `clippy::too_many_arguments` justification says “Phase 17.4 adds the authorizer binding.” Equivalent Phase/task-history comments remain in other production files. Repository standards require source comments to describe the current contract and place delivery history in snapshots or Git.

**Fix:** Keep the current-contract rationale and remove Phase/task references from production comments.

```yaml
id: AUD-17-003
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex-gpt5.md
source_model: codex
independence: fresh-context-same-family
axis: standards
severity: Minor
title: Production comments retain task history
claim: Production comments contain Phase/task history rather than only current contract rationale.
evidence:
  - location: crates/opi-agent/src/agent.rs:572
    detail: The attribute comment identifies Phase 17.4 rather than only explaining the present construction seam.
criterion_source: AGENTS.md comments-and-rustdoc rule
reproduction:
  - git show a680c5df13a08d5a2abc48b482a69d1c594f288e:crates/opi-agent/src/agent.rs
confidence: high
status: unverified
```

### 2.4 MINOR: File adapter exports test-only inspection accessors

**File:** `crates/opi-coding-agent/src/evidence.rs`
**Lines:** 114--122

`FileEvidenceSink::dir` and `completed_run_dirs` are public but have no non-test repository consumer. They widen the product adapter surface solely to help tests inspect temporary output, without the two-consumer or intrinsic-state-machine evidence required for a new public seam.

**Fix:** Make the accessors crate-private or have tests inspect their supplied temporary root directly.

```yaml
id: AUD-17-004
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex-gpt5.md
source_model: codex
independence: fresh-context-same-family
axis: standards
severity: Minor
title: File adapter exports test-only inspection accessors
claim: FileEvidenceSink exposes public inspection accessors with no production repository consumer.
evidence:
  - location: crates/opi-coding-agent/src/evidence.rs:114-122
    detail: The configured root and completed directories are returned through public methods.
  - location: crates/opi-coding-agent/src/harness.rs:4331
    detail: Repository uses of completed_run_dirs are test assertions.
criterion_source: PRIN-002; AGENTS.md public-seam admission rule
reproduction:
  - git grep -n "completed_run_dirs" a680c5df13a08d5a2abc48b482a69d1c594f288e -- crates/opi-coding-agent
confidence: medium
status: unverified
```

## 3. Spec Findings

### 3.1 MAJOR: Interactive bare model selection guesses the active provider

**File:** `crates/opi-coding-agent/src/harness.rs`
**Lines:** 1988--2038

For bare input, `set_model_validated` canonicalizes with `self.agent.provider_id()`, and `try_configure_model` validates only that active-provider route. With `alpha:shared` and `beta:shared` registered, `set_model_validated("shared")` accepts and persists `alpha:shared` instead of returning typed ambiguity. The legacy route path already correctly enumerates dispatchable routes in `normalize_recorded_route`.

**Impact:** The Reference Product silently chooses a provider where Phase 17 requires unique proof or a pre-dispatch typed ambiguity failure.

**Fix:** Share the unique-route normalization rule between direct model selection and legacy resume, then add a two-provider/same-model test that asserts unchanged state and zero dispatches.

```yaml
id: AUD-17-005
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex-gpt5.md
source_model: codex
independence: fresh-context-same-family
axis: spec
severity: Major
title: Interactive bare model selection guesses the active provider
claim: Direct bare model selection uses the active provider instead of failing when multiple dispatchable routes share the model id.
evidence:
  - location: crates/opi-coding-agent/src/harness.rs:1988-2010
    detail: A bare model is persisted as canonical_model_spec(self.agent.provider_id(), model).
  - location: crates/opi-coding-agent/src/harness.rs:2021-2038
    detail: Validation first tries current_provider:bare rather than requiring one global route match.
  - location: crates/opi-coding-agent/src/harness.rs:2372-2394
    detail: Legacy normalization already implements the required unique-route scan.
  - location: crates/opi-coding-agent/tests/phase17_provider_runtime.rs:398-418
    detail: The current test intentionally asserts active-provider bare normalization rather than ambiguity rejection.
criterion_source: P17-PRV-002; P17-A02; Task 17.5 definition of done
reproduction:
  - cargo test -p opi-coding-agent --test phase17_provider_runtime phase17_coding_harness_cross_provider_switch_dispatches_both_providers
confidence: high
status: unverified
```

### 3.2 MAJOR: Incomplete evidence still projects unavailable tools to the next provider request

**Files:** `crates/opi-agent/src/agent_loop.rs`, `crates/opi-agent/src/authority.rs`, `crates/opi-coding-agent/src/tool_authority.rs`
**Lines:** `agent_loop.rs:268--276`; `authority.rs:164--169`; `tool_authority.rs:457--467`

The loop builds every request from the immutable registry's complete definition list. When explicit capture has failed, `ProductToolAuthorizer` correctly denies all tool launches, but the next provider request still advertises those now-unavailable tools. The product acceptance fixture drives this exact two-request sequence but only checks that the write does not execute.

**Fix:** Give the trusted policy path a per-request projection that excludes tools unavailable under the current evidence-health snapshot; assert that the second request omits `write` after the injected failure.

```yaml
id: AUD-17-006
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex-gpt5.md
source_model: codex
independence: fresh-context-same-family
axis: spec
severity: Major
title: Incomplete evidence still advertises unavailable tools
claim: After required-complete-evidence becomes incomplete, later provider requests retain tool definitions that the effective policy will deny.
evidence:
  - location: crates/opi-agent/src/agent_loop.rs:268-276
    detail: Every provider request receives registry.definitions().
  - location: crates/opi-agent/src/authority.rs:164-169
    detail: ToolRegistry definitions are a static unfiltered projection.
  - location: crates/opi-coding-agent/src/tool_authority.rs:457-467
    detail: Product authorization denies incomplete-evidence calls.
  - location: crates/opi-coding-agent/tests/phase17_product_evidence.rs:1634-1690
    detail: The test supplies a second provider response after injected failure but asserts only zero write side effects.
criterion_source: P17-AUT-008; P17-EVD-009
reproduction:
  - cargo test -p opi-coding-agent --test phase17_product_evidence harness_complete_evidence_mapping_denies_unlaunched_tool
confidence: high
status: unverified
```

## 4. Security and Redaction Findings

### 4.1 MAJOR: Public Agent events retain raw user and tool-result content

**Files:** `crates/opi-agent/src/event.rs`, `crates/opi-coding-agent/src/runner.rs`, `crates/opi-coding-agent/src/rpc.rs`
**Lines:** `event.rs:115--126,161--184,264--275,342--353`; `runner.rs:353--405`; `rpc.rs:480--486`

`AgentEvent::redacted_for_public` clones `Message::User` and `ToolResultMessage.content` unchanged, and `ToolExecutionEnd.result` is cloned unchanged. The Agent public fanout supplies those events directly to NDJSON and RPC serializers. A prompt or tool-result canary can consequently appear in public structured output, while the existing canary tests do not assert the absence of those terminal event shapes.

**Fix:** Classify/redact user-message and tool-result payload channels at the public event boundary and add canary coverage for `AgentEnd`, `TurnEnd`, and `ToolExecutionEnd` in NDJSON/RPC output.

```yaml
id: AUD-17-007
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex-gpt5.md
source_model: codex
independence: fresh-context-same-family
axis: security
severity: Major
title: Public Agent events retain raw user and tool-result content
claim: Public NDJSON and RPC event paths serialize raw user-message and tool-result content without redaction.
evidence:
  - location: crates/opi-agent/src/event.rs:264-275
    detail: redact_agent_message clones User messages unchanged.
  - location: crates/opi-agent/src/event.rs:161-184
    detail: ToolExecutionEnd clones result unchanged.
  - location: crates/opi-agent/src/event.rs:342-353
    detail: redact_tool_result_message clones content unchanged.
  - location: crates/opi-coding-agent/src/runner.rs:353-405
    detail: NDJSON serializes subscribed public AgentEvents.
  - location: crates/opi-coding-agent/src/rpc.rs:480-486
    detail: RPC sends subscribed AgentEvents to its JSON channel.
criterion_source: P17-FAL-004; P17-A10; Task 17.7 definition of done
reproduction:
  - cargo test -p opi-coding-agent --test json_mode phase17_canary_is_absent_from_json_and_ndjson
confidence: high
status: unverified
```

## 5. Invariant and Test Assessment

| Area | Assessment |
|------|------------|
| Route/auth immutability and retry reuse | Focused `provider_collection` and `per_request_auth` coverage passed; no deviation found. |
| Atomic next-turn state and ordering | Focused `agent_wrapper` and `hooks_queues` coverage passed; no deviation found. |
| Authorization-before-execution | Covered for missing, stale, and denied authorization; incomplete-evidence projection remains deficient (3.2). |
| Evidence lifecycle and manifests | Finalization/abandonment tests pass; pre-dispatch evidence failure is not fail-closed (2.1). |
| Redaction | Sink/file and selected output tests pass, but terminal public event shapes are uncovered and unsafe (4.1). |
| Migration and modes | Legacy byte-preservation, structural mode equivalence, and hermetic-source tests pass. Actual three-platform CI was not independently rerun during this audit. |

## 6. Minimum-change Conformance

| Task | Scenario/source | Reuse | Placement | Surface | Production slice | Ceiling/trigger | Status |
|------|-----------------|-------|-----------|---------|------------------|-----------------|--------|
| 17.1 | Provider preparation | Recorded | Core owner | Opaque prepared call | Substrate reaches later owners | No unadmitted seam observed | `conforming` |
| 17.2 | Next-turn state | Recorded | Core owner | Complete replacement | Agent/coding harness | No patch protocol observed | `conforming` |
| 17.3 | Evidence contract | Recorded | Core owner | No-op/in-memory contract | Consumed by runtime/product | Closed-state surface drift (2.2) | `drifted` |
| 17.4 | Authorization | Recorded | Core + product policy | Registration/authorizer | Product path | Projection drift (3.2) | `drifted` |
| 17.5 | Provider assembly | Recorded | Reference Product | Canonical route | Product selection | Bare-route ceiling violated (3.1) | `drifted` |
| 17.6 | Evidence runtime | Recorded | Core owner | Run lifecycle | Agent loop | Fail-closed ceiling violated (2.1) | `drifted` |
| 17.7 | Product evidence/redaction | Recorded | Reference Product | File adapter | Modes and file path | Raw public payload path and test-only public seam (4.1, 2.4) | `drifted` |
| 17.8 | Legacy migration | Recorded | Reference Product | Read-time normalization | Resume/fork | No deviation observed | `conforming` |
| 17.9 | Acceptance assurance | Recorded | Assurance | No production capability | Cross-mode tests/CI definition | Source comments retain phase history (2.3) | `drifted` |

## 7. Verification and Recommendations

The following commands were run against the pinned, clean current HEAD and passed:

```text
python scripts/opi-doc-check.py
cargo fmt --check --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p opi-ai --test provider_collection --test per_request_auth
cargo test -p opi-agent --test agent_wrapper --test hooks_queues --test evidence_runtime --test tool_authority
cargo test -p opi-coding-agent --test phase17_provider_runtime --test phase17_product_evidence --test phase17_tool_authority --test phase17_legacy_migration --test phase17_cross_mode --test phase17_failure_rollback --test phase17_api_audit
```

Priority remediation order:

1. Stop the unlaunched provider call when pre-dispatch evidence fails under required-complete-evidence policy.
2. Make direct bare selection prove exactly one dispatchable route, matching legacy normalization.
3. Project tools from current trusted policy/evidence health before every provider request.
4. Redact terminal public event payloads and prove it with NDJSON/RPC canaries.

