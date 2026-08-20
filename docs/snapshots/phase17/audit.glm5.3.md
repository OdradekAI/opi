# Phase 17 Deep Agent Core Semantic Closure -- Independent Code Audit

**Auditor**: glm5.2 (independent; no prior audit reports consulted)
**Date**: 2026-08-15
**Scope**: Phase 17 registered requirements (55 `P17-*` + `P17-A01`--`A15`) and Tasks 17.1--17.9
**Implementation target**: `eb5e3166834f804c9b47f5d17f8131652931c601` (current committed implementation)
**Phase exit commit**: `a4cfa4ddc74b4dfac59b4305d4657599af866480` (last task `verified_at_commit`; provenance only)
**History use**: provenance and discovery only; no diff coverage boundary
**Method**: Pinned-HEAD whole-implementation audit. Requirements were built from the
phase17 ledger snapshot (9 task DoDs, evidence claims, inference notes), the registered
design spec, and the parent `docs/opi-spec.md` INV/PRIN/CTRL rows. Coverage came from 11
parallel deep-read/dimension auditor agents (opi-ai substrate + adapters, opi-agent
state/authority/evidence, product harness/modes, the Matt Standards and Spec axes,
invariants, test quality, security/redaction, integration + minimum-change overlay), each
reading its assigned files in full at `eb5e316`; four agent groups (`agent-state-loop`,
`agent-authority`, `product-provider`, `product-sessions-legacy`) died on provider-side
rate limits after retries and were completed by the lead auditor via direct full-file reads
(agent.rs, loop_types.rs, hooks.rs, compaction.rs, authority.rs, tool.rs, the agent_loop
ordering/authorization regions, session_coordinator.rs, session_cli.rs, and the
provider_factory.rs bundle/guard/bedrock regions). 62 raw findings were clustered into 48
distinct findings; each cluster was adversarially verified against the cited code by an
independent verifier agent (42 CONFIRMED unmodified, 6 CONFIRMED after claim/severity
correction, 0 REFUTED), and the lead auditor re-verified the highest-severity findings and
the four manually covered groups directly. No `audit.*.md`, evaluator transcript, or prior
review record was read (contamination isolation). A concurrent worktree modification of
`docs/snapshots/phase17/audit.codex.md` by another session was observed mid-audit and
excluded from the evidence set; `git diff` confirms no file under `crates/`, `scripts/`,
`docs/superpowers/`, `docs/opi-spec.md`, `CHANGELOG.md`, the READMEs, or `.github/` changed
during the audit. `git rev-parse HEAD` still equaled the pinned `eb5e316` immediately
before this verdict.

---

## 1. Executive Summary

**Verdict: PASS-WITH-FINDINGS**

| Severity | Count |
|----------|-------|
| Blocker | 0 |
| Major | 4 |
| Minor | 29 |
| Info | 16 |

Phase 17 is a substantially faithful and unusually well-tested implementation of a demanding
semantic-closure specification: collection-owned dispatch with frozen prepared calls, atomic
NextTurnState with the fixed prepare/apply/stop/queue ordering, a genuinely fail-closed
trusted authorization boundary (zero-execution counter tests hold on every enumerated
failure class), and a product-neutral evidence seam with producer-side redaction. All 35
focused test binaries this audit ran at HEAD passed with zero failures. The four Majors are
not runtime safety defects: one is a missing spec-required typed validation on the public
`prepare_call` seam (3.1; the in-repo product path always passes matching values), and
three are evidence-currency or evidence-accuracy gaps created chiefly by the post-exit
remediation commit `211aba8` -- unpushed, so no CI exists for HEAD (3.2); an exit
criterion proof inverted by `assert_ne` (5.1); an outcome-retention test leg dropped (5.2)
-- which the archived exit-ledger text no longer describes accurately. The Minors cluster
around evidence-fact fidelity (fallback axis, trigger fact, Bedrock provenance),
capture-configured failure-path ergonomics (file-sink wedge 7.1, zero-record error
re-typing 6.3, `turn_offset` divergence 7.2), and standards hygiene (pervasive phase-history
comments 2.1).

### Per-task summary

| Task | Title | Verdict | Key findings |
|------|-------|---------|--------------|
| 17.1 | Add collection-owned route and authentication preparation | PASS-WITH-FINDINGS | Major 3.1 (request-route mismatch class absent); Minor 5.10, 2.9; Info 4.5 |
| 17.2 | Cut over Agent to durable atomic NextTurnState | PASS-WITH-FINDINGS | Minor 2.6; Info 6.4, 3.6 |
| 17.3 | Define evidence identities, health, and storage-neutral lifecycle | PASS-WITH-FINDINGS | Minor 3.3, 3.4, 2.8; Info 8.1, 8.2 |
| 17.4 | Cut over trusted tool registrations and mandatory authorization | PASS-WITH-FINDINGS | Minor 2.12, 2.7 |
| 17.5 | Wire the Reference Product to dispatchable provider routes | PASS-WITH-FINDINGS | Minor 5.11, 2.10, 2.11; Info 8.4 |
| 17.6 | Expand Agent evidence runtime over stable identities | PASS-WITH-FINDINGS | Minor 2.3, 2.4; Info 4.4 |
| 17.7 | Cut over Reference Product evidence, finalization, and redaction | PASS-WITH-FINDINGS | Major 5.1, 5.2; Minor 6.2, 6.3, 7.1, 7.2, 3.5, 5.5, 4.2; Info 8.1, 6.5, 6.6, 8.3 |
| 17.8 | Migrate legacy session routes and preserve opaque trace artifacts | PASS-WITH-FINDINGS | Minor 2.5, 2.14 |
| 17.9 | Close local cross-mode, failure, rollback, documentation, and CI acceptance | PASS-WITH-FINDINGS | Major 3.2; Minor 5.3, 5.4, 5.6, 5.7; Info 5.8, 5.9, 5.12 |

### Local verification at `audit_head` (Windows host, all green)

`python scripts/opi-doc-check.py` -> PASS. Focused `cargo test` runs (per-target, external
cargo cache, `CARGO_INCREMENTAL=0`): opi-ai `provider_collection` 48, `auth_contracts` 7,
`per_request_auth` 8, `provider_trait` ok; opi-agent `agent_wrapper` 21, `hooks_queues` 8,
`agent_loop_semantics` 20, `phase17_prepare_call` 18, `retry_agent` 2, `compaction` 11,
`evidence_contract` 35, `evidence_runtime` 28, `tool_authority` 6, `tool_validation` 30,
`extensions` 8, `diagnostics_runtime` 17; opi-coding-agent `phase17_api_audit` 5,
`phase17_artifact_truthfulness` 1, `phase17_legacy_migration` 7, `phase17_provider_runtime` 6,
`phase17_tool_authority` 13, `phase17_product_evidence` 15, `phase17_cross_mode` 2,
`phase17_failure_rollback` 6, `non_interactive` 9, `json_mode` 29, `rpc_jsonl` 15,
`interactive_mock` 80, `session_runtime` 40, `session_cli` 12, `provider_factory` 53,
`provider_identity` 7, `tool_selection` 44, `credential_store` 60, `list_models` 30 -- all
0 failed. Note: the 17.3/17.7 task-level ledger `verification` blocks cite
`crates/opi-agent/tests/trace_envelope.rs`, which was deleted with the legacy trace contract
in `32c79e7` (17.7); that citation was true at its `verified_at_commit` and is historical
provenance, not a HEAD defect.

---

## 2. Standards (Matt code-review axis) Findings

### 2.1 MINOR: Phase/task/DoD history preserved in source comments across all Phase 17 files, contrary to CLAUDE.md

**File:** `crates/opi-agent/src/evidence.rs (representative; all 13 assigned files affected)`
**Lines:** workspace-wide
**Claim:** Phase/task/history references pervade rustdoc and inline comments in every assigned Phase 17 file (153 grep matches over the 13 files), violating the documented repo standard that source comments describe only current contracts.
**Adversarial verification:** CONFIRMED -- The governing standard exists verbatim in CLAUDE.md/AGENTS.md Working principles: 'Do not preserve Phase, task, PR, or review history in source comments.' Every cited line was opened and verified to carry phase/task/DoD/D.2 history: opi-agent/src/evidence.rs:3 '(Phase 17 task 17.3)', 5-7 'authorization (17.4), the Agent runtime (17.6), ... cutover (17.7)', 767 '(17.7) treats ... fail-closed', 891 'required-complete-evidence policy (17.7)'; agent_

```yaml
id: standards-01
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: standards
severity: Minor
title: "Phase/task/DoD history preserved in source comments across all Phase 17 files, contrary to CLAUDE.md"
claim: "Phase/task/history references pervade rustdoc and inline comments in every assigned Phase 17 file (153 grep matches over the 13 files), violating the documented repo standard that source comments describe only current contracts."
evidence:
  - location: "crates/opi-agent/src/evidence.rs:3,5-7,767,891"
    detail: "Module doc cites tasks 17.3/17.4/17.6/17.7; lines 767 and 891 carry (17.7) and required-complete-evidence policy (17.7)."
  - location: "crates/opi-agent/src/agent_loop.rs:236,863,1273,1429,1639-1640"
    detail: "Comments cite Phase 12 task 12.7 DoD clause 5, Phase 11.8 (S2), Phase 17.4 invocation order, Phase 11.8 (S1), Phase 7 Diagnostic."
  - location: "crates/opi-coding-agent/src/harness.rs:2,985,2509,3453,3829"
    detail: "Comments cite Phase 16.10 D.2 must-fix, Phase 10 Workstream 10.2, Phase 7 task 7.5, C5, T6 gate (task 15.7)."
  - location: "crates/opi-ai/src/auth.rs:1,102 + opi-coding-agent/src/evidence.rs:1 + agent.rs:1 + hooks.rs:52-124 + tool_authority.rs:1,40,137,143"
    detail: "Module docs cite Phase 14.2, Phase 17, task 17.7, Workstream 10.1, S8.2 Phase 17.2, S6.1, Phase 17.2, Phase 17.4; tool_authority.rs:40,137,143 cite volatile spec line numbers. Per-file grep counts total 153 matches across the 13 assigned files (harness.rs 63, agent_loop.rs 16, provider_factory.rs 16, session_coordinator.rs 11, loop_types.rs 10, others 1-7 each)."
criterion_source: "CLAUDE.md (AGENTS.md) Working principles: comments and rustdoc describe current contracts; do not preserve Phase/task/PR/review history in source comments"
reproduction:
  - "grep -cE 'Phase 1[0-9]|task 1[0-9]|Phase 7|Phase 14|Phase 17|task 17|17x' per assigned file — 153 matches total"
confidence: high
status: unverified
```

### 2.2 MINOR: Duplicated run-entry skeleton in harness has already diverged: C5 failed-turn rewind missing from continue_

**File:** `crates/opi-coding-agent/src/harness.rs`
**Lines:** 2509-2629
**Claim:** CodingHarness::prompt, prompt_with_content, retry_last_prompt, and continue_ duplicate the identical setup/result/finalize skeleton, and the C5 discard-unpersisted-failed-turn-user-message rewind is applied in only two of the four entries (absent from continue_), so a failed prompt followed by continue_ persists the stale failed-turn user message into the new turn's persistence slice — exactly the absorption the C5 comment says must not happen.
**Adversarial verification:** CONFIRMED -- All textual claims verified at HEAD. The four entries share the identical setup/result/finalize skeleton (prompt 2505-2535, prompt_with_content 2539-2571, retry_last_prompt 2577-2602, continue_ 2605-2629). rewind_agent_context(self.turn_offset) appears in prompt (2509-2514, under the C5 comment) and prompt_with_content (2546-2549, 'see prompt'), and in no other entry; retry_last_prompt's omission is explicitly documented as intentional in prompt'

```yaml
id: standards-02
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: standards
severity: Minor
title: "Duplicated run-entry skeleton in harness has already diverged: C5 failed-turn rewind missing from continue_"
claim: "CodingHarness::prompt, prompt_with_content, retry_last_prompt, and continue_ duplicate the identical setup/result/finalize skeleton, and the C5 discard-unpersisted-failed-turn-user-message rewind is applied in only two of the four entries (absent from continue_), so a failed prompt followed by continue_ persists the stale failed-turn user message into the new turn's persistence slice — exactly the absorption the C5 comment says must not happen."
evidence:
  - location: "crates/opi-coding-agent/src/harness.rs:2509-2514"
    detail: "prompt() calls rewind_agent_context(self.turn_offset) under the C5 comment about discarding unpersisted failed-turn user messages."
  - location: "crates/opi-coding-agent/src/harness.rs:2546-2549"
    detail: "prompt_with_content repeats the rewind with a see-prompt comment."
  - location: "crates/opi-coding-agent/src/harness.rs:2605-2629"
    detail: "continue_ contains the same skeleton (session resume, extension state, setup_evidence_run, result match, persist_turn, finalize, turn_offset update) but NO rewind_agent_context call and no comment explaining an intentional skip."
  - location: "crates/opi-coding-agent/src/harness.rs:2510-2512"
    detail: "retry_last_prompt documents its intentional divergence (reuses the failed-turn user message after interactive login), showing omitted-vs-intentional is distinguished elsewhere. Scenario: prompt fails after pushing the user message (turn_offset stays N, context N+1), continue_ pushes a new message, persist_turn then persists messages[offset..] including the stale failed-turn user message."
criterion_source: "CLAUDE.md Working principles (minimum change; comments describe current contracts); Fowler baseline: Duplicated Code"
reproduction:
  - "Make a prompt fail after the user message is pushed (turn_offset unchanged), then call continue_ with a new message and inspect what persist_turn writes: the stale failed-turn user message is included in the persisted slice."
confidence: medium
status: unverified
```

### 2.3 MINOR: Tool evidence emission is duplicated and structurally divergent between sequential and parallel batch paths: sequential emits authorization+outcome records, parallel emits one merged record with different keys and no phase discriminator

**File:** `crates/opi-agent/src/agent_loop.rs`
**Lines:** 301-309, 328-404, 458, 519-568, 1079-1142, 1219-1223, 1378-1381
**Claim:** The same conceptual tool call is emitted through two hand-maintained paths selected by batch composition (any Sequential tool in the batch, not the tool itself): sequential batches emit an authorization-phase record plus an outcome-phase record sharing one CallId with payloads phase:authorization{tool,registration_id,capability,decision} and phase:outcome{tool,registration_id,is_error}, while all-parallel batches emit one merged post-hoc Tool record per call with keys {tool,is_error,registration_id,capability,decision} and no phase discriminator, the authorization label re-derived by an inline match duplicating the standalone authorization_label() helper. Evidence consumers cannot rely on a uniform Tool-record shape and a parallel tool's representation changes when a Sequential tool (e.g. bash) shares the batch; the duplicate authorization_stale stable-code + reason construction is also independently maintained twice.
**Adversarial verification:** CONFIRMED -- Every evidence item verified against the code at HEAD. (1) batch_is_sequential (agent_loop.rs:301-309) is computed from batch composition via iter().any(execution_mode() == Sequential) with unwrap_or(true) for unknown tools — not from the tool being emitted. (2) The sequential branch passes Some(&mut tool_evidence) into execute_tool (line 347), which emits the phase:authorization record inside (1352-1354 via emit_tool_authorization_evidence), the

```yaml
id: standards-03
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: standards
severity: Minor
title: "Tool evidence emission is duplicated and structurally divergent between sequential and parallel batch paths: sequential emits authorization+outcome records, parallel emits one merged record with different keys and no phase discriminator"
claim: "The same conceptual tool call is emitted through two hand-maintained paths selected by batch composition (any Sequential tool in the batch, not the tool itself): sequential batches emit an authorization-phase record plus an outcome-phase record sharing one CallId with payloads phase:authorization{tool,registration_id,capability,decision} and phase:outcome{tool,registration_id,is_error}, while all-parallel batches emit one merged post-hoc Tool record per call with keys {tool,is_error,registration_id,capability,decision} and no phase discriminator, the authorization label re-derived by an inline match duplicating the standalone authorization_label() helper. Evidence consumers cannot rely on a uniform Tool-record shape and a parallel tool's representation changes when a Sequential tool (e.g. bash) shares the batch; the duplicate authorization_stale stable-code + reason construction is also independently maintained twice."
evidence:
  - location: "crates/opi-agent/src/agent_loop.rs:301-309"
    detail: "batch_is_sequential = tool_calls.iter().any(... execution_mode() == Sequential) — batch composition, not the tool, selects the emission branch."
  - location: "crates/opi-agent/src/agent_loop.rs:328-404"
    detail: "Sequential path passes Some(&mut tool_evidence) into execute_tool (authorization record emitted inside) then emit_tool_outcome_evidence producing phase:outcome payload."
  - location: "crates/opi-agent/src/agent_loop.rs:458,519-568"
    detail: "Parallel path passes None, then after join_all emits one record per call with {tool,is_error,registration_id,capability,decision}; the authorization label is re-derived by an inline match duplicating authorization_label() defined at 1079-1084."
  - location: "crates/opi-agent/src/agent_loop.rs:1086-1142"
    detail: "emit_tool_authorization_evidence (phase:authorization with capability, decision) and emit_tool_outcome_evidence (phase:outcome with is_error) share the same CallId in the sequential branch."
criterion_source: "P17-EVD-002 (explicit run/turn/call correlation and kind); Phase 17 spec 'Core vocabulary and adapters' (one storage-neutral lifecycle contract); Fowler baseline: Duplicated Code; CLAUDE.md Design boundaries"
reproduction:
  - "Execute a batch containing only parallel tools, then the same parallel tool alongside a Sequential tool (e.g. bash); diff the emitted Tool evidence records — one merged record with no phase key vs two records (phase:authorization + phase:outcome) sharing a CallId."
confidence: high
status: unverified
```

### 2.4 MINOR: Auth-source/fallback token vocabulary duplicated across producer and consumer crates with silent-default mappings

**File:** `crates/opi-coding-agent/src/evidence.rs`
**Lines:** 549-565, 604-646 (with agent_loop.rs:1586-1604)
**Claim:** The provider evidence record serializes auth_source/fallback as hand-rolled string tokens in opi-agent, and the product manifest re-parses those tokens with silent defaults (unrecognized token becomes Static or None), so the vocabulary lives in two crates whose default arms can silently normalize an unrecognized classification instead of failing visibly; the last-Provider-record extraction is also performed twice on the same payload.
**Adversarial verification:** CONFIRMED -- All cited code verified. Producer side (agent_loop.rs:1586-1604): auth_source_token maps Static/Environment/CredentialStore/OAuth to 'static'/'environment'/'credential_store'/'oauth' with a `_ => "static"` catch-all at 1592; auth_fallback_token maps NotAttempted/Used with `_ => "not_attempted"` at 1602. Consumer side (opi-coding-agent/src/evidence.rs:631-646): auth_source_from_token maps the tokens back with `_ => AuthProvenanceSource::Static` (6

```yaml
id: standards-08
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: standards
severity: Minor
title: "Auth-source/fallback token vocabulary duplicated across producer and consumer crates with silent-default mappings"
claim: "The provider evidence record serializes auth_source/fallback as hand-rolled string tokens in opi-agent, and the product manifest re-parses those tokens with silent defaults (unrecognized token becomes Static or None), so the vocabulary lives in two crates whose default arms can silently normalize an unrecognized classification instead of failing visibly; the last-Provider-record extraction is also performed twice on the same payload."
evidence:
  - location: "crates/opi-agent/src/agent_loop.rs:1586-1604"
    detail: "auth_source_token maps AuthProvenanceSource to static/environment/credential_store/oauth with a default-to-static arm; auth_fallback_token maps to not_attempted/used with a default-to-not_attempted arm."
  - location: "crates/opi-coding-agent/src/evidence.rs:631-646"
    detail: "auth_source_from_token maps the tokens back with a default to AuthProvenanceSource::Static; fallback_allowed_from_token maps used/not_attempted with a default to None — an unrecognized token is silently normalized rather than a typed error."
  - location: "crates/opi-coding-agent/src/evidence.rs:549-565 vs 604-618"
    detail: "extract_route_facts and extract_provenance_facts each independently reverse-find the last Provider record and serde-parse the same JSON payload twice."
criterion_source: "Fowler baseline: Duplicated Code; CLAUDE.md Design boundaries (fail-closed validation at boundaries)"
reproduction:
  - "Emit a Provider record whose auth_source token is an unrecognized string; the manifest silently records auth_source=static instead of failing visibly."
confidence: high
status: unverified
```

### 2.5 MINOR: Bare-spec normalization idiom duplicated across three harness sites plus a lenient splitter in the product evidence module

**File:** `crates/opi-coding-agent/src/harness.rs`
**Lines:** 1859-1872, 1896-1919, 1929-1941 (with evidence.rs:658-668)
**Claim:** harness.rs re-implements the contains-(:)-else-prefix-current-provider bare-spec normalization three times with slightly different fallback rules (apply_agent_model prefixes unconditionally; try_configure_model prefixes only when model_info resolves, else passes the bare spec to the strict parser; set_model_validated prefixes for the persisted canonical fact), and opi-coding-agent evidence.rs split_spec re-splits provider:model via split_once without validation when parsing the Provider evidence payload. Canonical parsing itself stays single-sourced in opi_ai::registry::parse_model_spec, so this is a divergence risk, not a current wrong behavior.
**Adversarial verification:** PARTIALLY-CONFIRMED -- All code facts verified exactly, but the severity understates what the repo's own finding contract prescribes for this category. Verified: apply_agent_model (harness.rs:1859-1872) prefixes the current provider unconditionally when !model.contains(':'); set_model_validated (1896-1919) repeats the contains-(':') check at 1903-1910 to canonicalize the persisted model_change fact; try_configure_model (1929-1941) repeats it again but prefixes only whe

```yaml
id: P17-STD-006
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: standards
severity: Minor
title: "Bare-spec normalization idiom duplicated across three harness sites plus a lenient splitter in the product evidence module"
claim: "harness.rs re-implements the contains-(:)-else-prefix-current-provider bare-spec normalization three times with slightly different fallback rules (apply_agent_model prefixes unconditionally; try_configure_model prefixes only when model_info resolves, else passes the bare spec to the strict parser; set_model_validated prefixes for the persisted canonical fact), and opi-coding-agent evidence.rs split_spec re-splits provider:model via split_once without validation when parsing the Provider evidence payload. Canonical parsing itself stays single-sourced in opi_ai::registry::parse_model_spec, so this is a divergence risk, not a current wrong behavior."
evidence:
  - location: "crates/opi-coding-agent/src/harness.rs:1859-1872,1896-1919,1929-1941"
    detail: "Three separate contains-(:)-based normalization blocks with differing fallback semantics in one file."
  - location: "crates/opi-coding-agent/src/evidence.rs:658-668"
    detail: "split_spec uses spec.split_once(':') with an unwrap_or_else fallback to resolved — a second, lenient spec splitter used on evidence payloads."
criterion_source: "CLAUDE.md working principles (no duplicated logic that could diverge); Phase 17 spec 'Dispatchable provider collection' (one canonical selection path)"
reproduction:
  - "n/a (static finding; cited tests runnable per Section 1)"
confidence: high
status: unverified
```

### 2.6 MINOR: Dead terminal-assistant handling in the loop plus a comment describing behavior the code does not implement

**File:** `crates/opi-agent/src/agent_loop.rs`
**Lines:** 762-807,932,1035
**Claim:** The turn's terminal assistant message is bound, moved into terminal_msg, and then explicitly discarded via a let-underscore binding while the preceding comment claims the loop checks it (if the stream ended without one, preserve state and stop) — no such check exists; related dead parameters (_turn_id) are retained with underscore names.
**Adversarial verification:** CONFIRMED -- Every cited fact holds at HEAD. terminal_assistant is declared uninitialized at agent_loop.rs:214 and assigned only at 583/595 (both followed by break 'stream). The comment at 762-764 claims 'A turn must produce a terminal assistant message to finalize. If the stream ended without one ... preserve state and stop', yet no check on the terminal message exists anywhere: the value is moved into terminal_msg at 765 and explicitly discarded by `let _ =

```yaml
id: standards-06
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: standards
severity: Minor
title: "Dead terminal-assistant handling in the loop plus a comment describing behavior the code does not implement"
claim: "The turn's terminal assistant message is bound, moved into terminal_msg, and then explicitly discarded via a let-underscore binding while the preceding comment claims the loop checks it (if the stream ended without one, preserve state and stop) — no such check exists; related dead parameters (_turn_id) are retained with underscore names."
evidence:
  - location: "crates/opi-agent/src/agent_loop.rs:762-765"
    detail: "Comment describes a terminal-message check that must run before finalizing, followed by binding terminal_msg = terminal_assistant."
  - location: "crates/opi-agent/src/agent_loop.rs:807"
    detail: "The value is never read; the loop performs no terminal-message check (a stream ending without Done/Error already returns AgentError::ProviderProtocol at 730-735)."
  - location: "crates/opi-agent/src/agent_loop.rs:932,1035"
    detail: "observe_provider_failure(_turn_id) and malformed_tool_arguments_result(_turn_id) keep unused parameters at all call sites."
criterion_source: "CLAUDE.md Working principles (minimum change; comments describe current contracts)"
reproduction:
  - "Read agent_loop.rs:762-807 — the comment's described check does not exist and terminal_msg is discarded."
confidence: high
status: unverified
```

### 2.7 MINOR: register_product_tools builds extension registrations only for a tautological debug_assert and drops them; build() ends with a no-op discard of active_tool_names

**File:** `crates/opi-coding-agent/src/tool_authority.rs`
**Lines:** 110-118, 206-211
**Claim:** register_product_tools converts every extension tool into a full RegisteredTool solely to run a debug_assert that is true by construction of register_extension_tools, then drops the registrations; EffectiveUserPolicy::build likewise ends with a no-op let-underscore discard of active_tool_names — dead computation retained in the fail-closed authorization path.
**Adversarial verification:** CONFIRMED -- Verified in full. register_product_tools (tool_authority.rs:106-119) builds extension_registrations via register_extension_tools, whose map body (lines 87-96) constructs ToolOrigin::Extension for every item, so the debug_assert at 112-116 is true by construction and cannot fail in any build; the registrations are then dropped at 117 without any other use. In release builds the conversion (definition clones, RegistrationId formatting, Arc::from) i

```yaml
id: standards-07
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: standards
severity: Minor
title: "register_product_tools builds extension registrations only for a tautological debug_assert and drops them; build() ends with a no-op discard of active_tool_names"
claim: "register_product_tools converts every extension tool into a full RegisteredTool solely to run a debug_assert that is true by construction of register_extension_tools, then drops the registrations; EffectiveUserPolicy::build likewise ends with a no-op let-underscore discard of active_tool_names — dead computation retained in the fail-closed authorization path."
evidence:
  - location: "crates/opi-coding-agent/src/tool_authority.rs:110-118"
    detail: "debug_assert that all registrations have ToolOrigin::Extension followed by drop(extension_registrations); register_extension_tools (80-99) always constructs ToolOrigin::Extension, so the assertion cannot fail."
  - location: "crates/opi-coding-agent/src/tool_authority.rs:206-211"
    detail: "After the digest loop consumes active_tool_names (sorted at 173, iterated at 181), the binding is discarded as a statement with an explanatory comment — a no-op."
criterion_source: "CLAUDE.md Working principles (no speculative code; minimum change)"
reproduction:
  - "Read tool_authority.rs:80-118 and 173-211 — the assert is tautological and the final statement discards the binding."
confidence: high
status: unverified
```

### 2.8 MINOR: require_complete guards against empty digests that the type system makes unrepresentable (dead impossible-state error handling)

**File:** `crates/opi-agent/src/evidence.rs`
**Lines:** 1132,1137,1144-1155,1172 (with 328-339)
**Claim:** The empty-string ContentDigest guard arms in FinalizedManifest::require_complete can never fire: ContentDigest's inner field is private, the sole constructor from_hex validates exactly-64 lowercase hex characters, and the type derives no Deserialize, so no code in the workspace can construct an empty digest — error handling added for a state the design makes impossible.
**Adversarial verification:** CONFIRMED -- Fully verified. ContentDigest is 'pub struct ContentDigest(String)' (evidence.rs:314) with a private tuple field; its derives are Debug, Clone, PartialEq, Eq, Hash, Serialize — no Deserialize (line 312) — and evidence.rs contains no submodules and no Deserialize derive anywhere, so there is no serde or cross-module construction path. The impl block exposes exactly two functions: from_hex (sole constructor) and as_hex. from_hex (328-339) rejects a

```yaml
id: standards-04
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: standards
severity: Minor
title: "require_complete guards against empty digests that the type system makes unrepresentable (dead impossible-state error handling)"
claim: "The empty-string ContentDigest guard arms in FinalizedManifest::require_complete can never fire: ContentDigest's inner field is private, the sole constructor from_hex validates exactly-64 lowercase hex characters, and the type derives no Deserialize, so no code in the workspace can construct an empty digest — error handling added for a state the design makes impossible."
evidence:
  - location: "crates/opi-agent/src/evidence.rs:328-339"
    detail: "from_hex rejects any input that is not exactly 64 lowercase hex characters, including the empty string."
  - location: "crates/opi-agent/src/evidence.rs:1132,1137,1144-1155,1172"
    detail: "Four unreachable arms matching an empty inner digest; workspace grep shows the only ContentDigest tuple-pattern uses are these dead guards."
  - location: "crates/opi-coding-agent/tests/phase17_product_evidence.rs:1035-1038"
    detail: "The test asserts from_hex of the empty string errors (constructor boundary), not the require_complete gate."
criterion_source: "CLAUDE.md Working principles: do not add error handling for states the design makes impossible"
reproduction:
  - "grep -rn 'ContentDigest(' crates/ — the only tuple-pattern uses are the four dead empty-digest guards; from_hex is the sole constructor and validates length."
confidence: high
status: unverified
```

### 2.9 MINOR: CollectionError::AuthNotConfigured is a dead variant never constructed anywhere

**File:** `crates/opi-ai/src/provider_collection.rs`
**Lines:** 243-252
**Claim:** The typed error variant CollectionError::AuthNotConfigured is defined but no code path in the workspace constructs it; the missing-auth class actually surfaces as CollectionError::Provider(ProviderError::CredentialNeeded), leaving the variant and its downstream match arms unreachable residue of the pre-Phase-17 auth-status gate.
**Adversarial verification:** CONFIRMED -- Verified exhaustively. CollectionError::AuthNotConfigured is defined at provider_collection.rs:246-252 (the finding's 243-252 includes the preceding doc line — accurate). A grep of every .rs file in crates/ finds the variant name at exactly six sites: the enum definition (provider_collection.rs:247) plus five consumer arms — the CollectionError-to-AgentError mapping (agent_loop.rs:911-912), the AgentError variant definition (loop_types.rs:67), it

```yaml
id: OPAI-17_1-003
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: standards
severity: Minor
title: "CollectionError::AuthNotConfigured is a dead variant never constructed anywhere"
claim: "The typed error variant CollectionError::AuthNotConfigured is defined but no code path in the workspace constructs it; the missing-auth class actually surfaces as CollectionError::Provider(ProviderError::CredentialNeeded), leaving the variant and its downstream match arms unreachable residue of the pre-Phase-17 auth-status gate."
evidence:
  - location: "crates/opi-ai/src/provider_collection.rs:247-252"
    detail: "Variant definition; grep across all crates finds zero construction sites — only consumer match/map arms in opi-agent (agent_loop.rs:911-912, diagnostic.rs:925, loop_types.rs:67) and an exit-code mapping in opi-coding-agent/src/runner.rs:937."
  - location: "crates/opi-ai/src/provider_collection.rs:425-429"
    detail: "Actual missing-auth path: resolver.resolve() errors propagate as CollectionError::Provider(ProviderError::CredentialNeeded) via #[from], tested at tests/provider_collection.rs:320-350 and tests/per_request_auth.rs:87-120."
criterion_source: "CLAUDE.md working principles (minimum change, no unrequested abstractions; 17.5 contract step removes obsolete seams)"
reproduction:
  - "grep -rn AuthNotConfigured --include=*.rs crates/ — the only producer-side occurrence is the enum definition; every other hit is a consumer match arm."
confidence: high
status: unverified
```

### 2.10 MINOR: azure/gemini/vertex/bedrock silently drop documented Request.timeout and Request.extra_headers

**File:** `crates/opi-ai/src/azure_openai.rs`
**Lines:** 225-249
**Claim:** azure_openai, gemini, vertex, and bedrock stream_prepared implementations never read request.timeout or request.extra_headers, silently dropping both documented Request fields, while anthropic/openai_chat/openai_responses/openai_codex_responses honor both; production impact is nil today because the Agent always builds Request with timeout None and empty extra_headers, so the gap is embedder-facing.
**Adversarial verification:** CONFIRMED -- Verified across all eight adapters. azure_openai.rs stream_prepared (225-249), gemini.rs (842-866), vertex.rs (168-193), and bedrock/mod.rs (207-264) capture only url/body/cancel/client (plus model parsing); request.timeout and request.extra_headers are never referenced, and none of them rejects reserved header names — they are silently dropped, including the documented auth-header rejection contract. The contrast family honors both: anthropic.rs

```yaml
id: OPAI-ADPT-004
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: standards
severity: Minor
title: "azure/gemini/vertex/bedrock silently drop documented Request.timeout and Request.extra_headers"
claim: "azure_openai, gemini, vertex, and bedrock stream_prepared implementations never read request.timeout or request.extra_headers, silently dropping both documented Request fields, while anthropic/openai_chat/openai_responses/openai_codex_responses honor both; production impact is nil today because the Agent always builds Request with timeout None and empty extra_headers, so the gap is embedder-facing."
evidence:
  - location: "crates/opi-ai/src/azure_openai.rs:225-249"
    detail: "stream_prepared clones url/body/cancel/client only; request.timeout and request.extra_headers never referenced; same in gemini.rs:842-866, vertex.rs:168-193, bedrock/mod.rs:207-264 (extra headers not even rejected)."
  - location: "crates/opi-ai/src/anthropic.rs:1299-1302"
    detail: "Contrast: these adapters capture request.timeout and merge request.extra_headers through ProviderHeaders (anthropic.rs:1286,1299-1302; openai_chat.rs:1495-1502)."
  - location: "crates/opi-ai/src/provider.rs:115-124"
    detail: "Request documents per-request HTTP timeout semantics and extra headers appended to the outbound request — a contract the four adapters do not implement."
criterion_source: "Request field contracts at provider.rs:115-124; P17-PRV-006 (provider-neutral request interface semantics preserved across adapters)"
reproduction:
  - "Set Request{timeout: Some(1ms), extra_headers: vec![(X-Test,1)]} on GeminiProvider/AzureOpenAIProvider and observe via wiremock that the header is absent and a stalled server is not timed out."
confidence: high
status: unverified
```

### 2.11 MINOR: Route auth-source classification is hardcoded to AuthProvenanceSource::Static for all production routes; correctness rests on resolver self-reporting, and the Bedrock route mislabels env/profile/file AWS credentials as static

**File:** `crates/opi-coding-agent/src/provider_factory.rs`
**Lines:** 185-194, 1956, 1968-1975
**Claim:** build_harness_collection registers every dispatch route with AuthProvenanceSource::Static regardless of the resolver kind built one call earlier; the registered source applies only as a default when prepare_call sees default provenance, so accuracy depends on each resolver self-reporting non-default provenance — which the env/keychain CredentialAuthResolver does. The Bedrock route breaks this: build_bedrock registers a placeholder StaticAuthResolver with a sentinel secret and the CredentialSource computed by resolve_credentials is discarded, so evidence records auth_source=static for credentials actually resolved from AWS env vars, shared profile files, or config; the ledger residual wording is itself inaccurate in the pessimistic direction (claims accuracy only in tests/OAuth registrations while the env/store production resolvers are accurate).
**Adversarial verification:** CONFIRMED -- All code claims verified. build_harness_collection (provider_factory.rs:185-194) registers every route with the constant AuthProvenanceSource::Static regardless of the resolver built by build_runtime_route/route_auth_resolver. The registered source applies only when prepare_call sees default provenance (provider_collection.rs:425-439: `if auth.provenance == AuthProvenance::default()` overwrites with the route source), so accuracy rests on resolve

```yaml
id: standards-05
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: standards
severity: Minor
title: "Route auth-source classification is hardcoded to AuthProvenanceSource::Static for all production routes; correctness rests on resolver self-reporting, and the Bedrock route mislabels env/profile/file AWS credentials as static"
claim: "build_harness_collection registers every dispatch route with AuthProvenanceSource::Static regardless of the resolver kind built one call earlier; the registered source applies only as a default when prepare_call sees default provenance, so accuracy depends on each resolver self-reporting non-default provenance — which the env/keychain CredentialAuthResolver does. The Bedrock route breaks this: build_bedrock registers a placeholder StaticAuthResolver with a sentinel secret and the CredentialSource computed by resolve_credentials is discarded, so evidence records auth_source=static for credentials actually resolved from AWS env vars, shared profile files, or config; the ledger residual wording is itself inaccurate in the pessimistic direction (claims accuracy only in tests/OAuth registrations while the env/store production resolvers are accurate)."
evidence:
  - location: "crates/opi-coding-agent/src/provider_factory.rs:185-194"
    detail: "register_route is called with AuthProvenanceSource::Static as a constant for all routes, while route_credentials()/route_auth_resolver() in the same file know each route's real credential shape."
  - location: "crates/opi-coding-agent/src/provider_factory.rs:1956,1968-1975"
    detail: "The CredentialSource returned by resolve_credentials is discarded; a placeholder StaticAuthResolver with SecretString 'bedrock-compound-credential' is registered (stream_prepared never reads the secret — documented compound-credential exemption at bedrock/mod.rs:195-206)."
  - location: "crates/opi-ai/src/provider_collection.rs:433-439"
    detail: "The registered source only applies as a default overwrite when the resolver returned AuthProvenance::default() — correctness rests on resolver cooperation."
  - location: "crates/opi-ai/src/bedrock/credentials.rs:93-186"
    detail: "resolve_credentials returns (BedrockCredentials, CredentialSource) with the accurate classification already available."
criterion_source: "P17-PRV-005 (auth source facts MUST remain distinguishable/accurate in evidence; .audit-criteria.txt recorded residual); CLAUDE.md Design boundaries (fail-closed validation, naming honesty); Fowler duplicated knowledge that can diverge"
reproduction:
  - "Configure bedrock via AWS_SHARED_CREDENTIALS_FILE profile, run with --trace, and read the manifest auth_source: it reports static while the credential came from ProfileFile."
confidence: high
status: unverified
```

### 2.12 MINOR: Misplaced rustdoc: InvocationContext carries the doc comment describing ToolAuthorizationRequest

**File:** `crates/opi-agent/src/authority.rs`
**Lines:** 223-242
**Claim:** The enum InvocationContext (NoSession/Session) carries the rustdoc intended for ToolAuthorizationRequest (core-confirmed facts supplied to the trusted authorizer, full arguments inspected inside the trusted boundary, run/turn/call typed evidence identities minted for this exact call), misdocumenting the type it annotates.
**Adversarial verification:** CONFIRMED -- Verified at authority.rs:223-245. The doc comment at 223-226 on `pub enum InvocationContext` reads: 'Core-confirmed facts supplied to the trusted authorizer. Full arguments are inspected inside the trusted boundary; the emitted authorization outcome carries only the classified or redacted representation. Run/turn/call are the typed evidence identities minted for this exact call.' Sentences two and three describe request-level concerns (arguments;

```yaml
id: standards-09
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: standards
severity: Minor
title: "Misplaced rustdoc: InvocationContext carries the doc comment describing ToolAuthorizationRequest"
claim: "The enum InvocationContext (NoSession/Session) carries the rustdoc intended for ToolAuthorizationRequest (core-confirmed facts supplied to the trusted authorizer, full arguments inspected inside the trusted boundary, run/turn/call typed evidence identities minted for this exact call), misdocumenting the type it annotates."
evidence:
  - location: "crates/opi-agent/src/authority.rs:223-233"
    detail: "Doc lines describe request-level facts (arguments, run/turn/call identities) that belong to ToolAuthorizationRequest, whose own doc at 244-245 is the correct one-sentence contract."
  - location: "crates/opi-agent/src/authority.rs:228-242"
    detail: "InvocationContext's actual contract (opaque trusted session context vs NoSession) is documented only on its variants, not the type."
criterion_source: "CLAUDE.md Working principles: comments and rustdoc describe current contracts"
reproduction:
  - "Read authority.rs:223-245 — the InvocationContext doc restates ToolAuthorizationRequest's contract."
confidence: high
status: unverified
```

### 2.13 INFO: Stale references to the removed stream()/api_key seam survive in test-file docs and comments

**File:** `crates/opi-ai/tests/provider_lifecycle.rs`
**Lines:** 1 (with openai_compat_lifecycle.rs:250, test_support.rs:85-291)
**Claim:** Test substrate still names the removed pre-Phase-17 entry point and a removed constructor signature: provider_lifecycle.rs's module doc targets AnthropicProvider::stream(), openai_compat_lifecycle.rs documents new_for_profile with a leading api_key parameter that no longer exists, and test_support.rs MockProvider docs repeatedly describe stream() calls; the phase17_api_audit removed-symbol scan covers only crates/*/src, so this vocabulary drift in tests is unchecked.
**Adversarial verification:** CONFIRMED -- All three citation clusters hold at HEAD. provider_lifecycle.rs:1 documents 'Contract tests for AnthropicProvider::stream()' — a method that no longer exists anywhere: the Provider trait (provider.rs) exposes only stream_prepared, anthropic.rs implements only stream_prepared (line 1259), and all 8 call sites in the test file use stream_prepared. openai_compat_lifecycle.rs:250 comments 'Real signature is (api_key, base_url, provider_id, compat, ex

```yaml
id: OPAI-ADPT-007
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: standards
severity: Info
title: "Stale references to the removed stream()/api_key seam survive in test-file docs and comments"
claim: "Test substrate still names the removed pre-Phase-17 entry point and a removed constructor signature: provider_lifecycle.rs's module doc targets AnthropicProvider::stream(), openai_compat_lifecycle.rs documents new_for_profile with a leading api_key parameter that no longer exists, and test_support.rs MockProvider docs repeatedly describe stream() calls; the phase17_api_audit removed-symbol scan covers only crates/*/src, so this vocabulary drift in tests is unchecked."
evidence:
  - location: "crates/opi-ai/tests/provider_lifecycle.rs:1"
    detail: "Module doc targets AnthropicProvider::stream() — the method removed by 17.5; all tests in the file actually call stream_prepared."
  - location: "crates/opi-ai/tests/openai_compat_lifecycle.rs:250"
    detail: "Comment claims the real signature is (api_key, base_url, provider_id, compat, extra_headers, models) — new_for_profile's real signature (openai_chat.rs:911-917) has no api_key parameter."
  - location: "crates/opi-ai/src/test_support.rs:85-88,106-107,162,285-291"
    detail: "MockProvider docs and the stream_prepared doc refer to stream() calls; the implementation is stream_prepared."
criterion_source: "CLAUDE.md comment standards (comments describe current contracts); P17-MIG-006 removal cleanliness (scope of the api-audit scan)"
reproduction:
  - "grep for AnthropicProvider::stream and 'api_key, base_url' in crates/opi-ai/tests — stale vocabulary only in docs/comments."
confidence: high
status: unverified
```

### 2.14 INFO: Test-only public surface retained in production modules (on_turn_end_simple, compaction_entries, doc-hidden pub validate_single_wire_models)

**File:** `crates/opi-coding-agent/src/session_coordinator.rs`
**Lines:** 364-378,480-484 (with provider_factory.rs:1989-2018)
**Claim:** session_coordinator exposes two members documented as test-only (on_turn_end_simple at 364-378, compaction_entries at 480-484) and provider_factory marks validate_single_wire_models pub plus doc(hidden) (1989-2018) although nothing outside the module calls it — retained wider-than-necessary surface in published library code. It pre-dates Phase 17 (on_turn_end_simple introduced in ad7492c, validate_single_wire_models in 0589a18); session_coordinator.rs was modified by task 17.5 commits (a4136a1, 6600cd2) and provider_factory.rs by the 17.5 commits plus the phase-17 remediation commit 211aba8, without narrowing; task 17.8 itself (4893014) touched harness.rs/diagnostic.rs/tests only.
**Adversarial verification:** PARTIALLY-CONFIRMED -- The core standards claim is fully accurate: on_turn_end_simple (session_coordinator.rs:364-378, doc 'Backwards-compatible variant used by tests that don't track Agent indices') and compaction_entries (480-484, doc 'Exposed for tests that need to assert resume correctness') are pub on the published harness type with callers only in tests (rpc_jsonl.rs, session_runtime.rs); validate_single_wire_models (provider_factory.rs:1989-2018) is #[doc(hidden

```yaml
id: standards-10
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: standards
severity: Info
title: "Test-only public surface retained in production modules (on_turn_end_simple, compaction_entries, doc-hidden pub validate_single_wire_models)"
claim: "session_coordinator exposes two members documented as test-only (on_turn_end_simple at 364-378, compaction_entries at 480-484) and provider_factory marks validate_single_wire_models pub plus doc(hidden) (1989-2018) although nothing outside the module calls it — retained wider-than-necessary surface in published library code. It pre-dates Phase 17 (on_turn_end_simple introduced in ad7492c, validate_single_wire_models in 0589a18); session_coordinator.rs was modified by task 17.5 commits (a4136a1, 6600cd2) and provider_factory.rs by the 17.5 commits plus the phase-17 remediation commit 211aba8, without narrowing; task 17.8 itself (4893014) touched harness.rs/diagnostic.rs/tests only."
evidence:
  - location: "crates/opi-coding-agent/src/session_coordinator.rs:364-378,480-484"
    detail: "on_turn_end_simple documented as a backwards-compatible variant used by tests that don't track Agent indices; compaction_entries exposed for tests that assert resume correctness — both pub on the published harness type."
  - location: "crates/opi-coding-agent/src/provider_factory.rs:1989-2018 + tests/provider_factory.rs:1687"
    detail: "validate_single_wire_models is doc(hidden) pub but only called by validate_single_wire_provider (line 1985) and nothing external; the integration test imports only validate_single_wire_provider."
criterion_source: "CLAUDE.md Design boundaries (no abstraction/surface for hypothetical use)"
reproduction:
  - "grep for external callers of on_turn_end_simple, compaction_entries, and validate_single_wire_models — only tests/internal call sites."
confidence: medium
status: unverified
```

## 3. Spec (Matt code-review axis) Findings

### 3.1 MAJOR: prepare_call never cross-checks request.model against the resolved route; wire model can silently diverge from the validated route

**File:** `crates/opi-ai/src/provider_collection.rs`
**Lines:** 398-457
**Claim:** ProviderCollection::prepare_call validates request content against the spec-resolved ModelInfo but never checks that the Request identifies that same canonical provider:model, and no typed request-route-mismatch error exists at the provider-route boundary (the only occurrence of the term is the design doc's failure table). A Request whose model differs from the spec therefore silently dispatches on the wire with a model other than the frozen/validated PreparedRoute on gemini, azure_openai, vertex, and bedrock (their stream_prepared derives the wire model from request.model with no capability/thinking/wire validation), and on every adapter when the wire model exists only via registry dynamic catalogs or extension overrides. anthropic, openai_chat, openai_responses, openai_codex_responses, and api_mapped re-validate the request.model-derived model at dispatch time against the provider's construction-time catalog, so the finding's image-content reproduction is rejected there (as a dispatch-time stream error, not a preparation-boundary typed mismatch). The production caller (agent_loop.rs) builds request.model from the same ModelSelection whose to_spec() is the spec, so the gap is reachable only through the public publishable opi-ai seam, where evidence/frozen-route vs wire-model divergence is real on the non-validating adapters.
**Adversarial verification:** PARTIALLY-CONFIRMED -- The core structural claim is accurate: prepare_call (provider_collection.rs:398-458) resolves the route from the spec string, validates via validate_request_for_model(&provider_id, Some(model), &request), and freezes request.model unmodified into PreparedProviderCall; request.model is never compared to provider_id/model.id anywhere in the function, and validate_request_for_model (provider.rs:285-325) checks only image content vs model capabilitie

```yaml
id: OPAI-17_1-001
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: spec
severity: Major
title: "prepare_call never cross-checks request.model against the resolved route; wire model can silently diverge from the validated route"
claim: "ProviderCollection::prepare_call validates request content against the spec-resolved ModelInfo but never checks that the Request identifies that same canonical provider:model, and no typed request-route-mismatch error exists at the provider-route boundary (the only occurrence of the term is the design doc's failure table). A Request whose model differs from the spec therefore silently dispatches on the wire with a model other than the frozen/validated PreparedRoute on gemini, azure_openai, vertex, and bedrock (their stream_prepared derives the wire model from request.model with no capability/thinking/wire validation), and on every adapter when the wire model exists only via registry dynamic catalogs or extension overrides. anthropic, openai_chat, openai_responses, openai_codex_responses, and api_mapped re-validate the request.model-derived model at dispatch time against the provider's construction-time catalog, so the finding's image-content reproduction is rejected there (as a dispatch-time stream error, not a preparation-boundary typed mismatch). The production caller (agent_loop.rs) builds request.model from the same ModelSelection whose to_spec() is the spec, so the gap is reachable only through the public publishable opi-ai seam, where evidence/frozen-route vs wire-model divergence is real on the non-validating adapters."
evidence:
  - location: "crates/opi-ai/src/provider_collection.rs:403-407"
    detail: "prepare_call resolves the route from spec and validates against the spec-resolved model; request.model is never compared to provider_id/model.id anywhere in the function."
  - location: "crates/opi-ai/src/provider.rs:285-325"
    detail: "validate_request_for_model inspects image content, model.validate() (wire compat), and thinking resolution only; it never reads request.model."
  - location: "crates/opi-ai/src/anthropic.rs:795-798,828-829"
    detail: "Adapters derive wire model_id from request.model (suffix-or-bare split) and send it in the JSON body; same pattern in gemini.rs:506, openai_chat.rs:1021, openai_responses.rs:262, azure_openai.rs:231, vertex.rs:174, bedrock/mod.rs:216, openai_codex_responses.rs:277-279."
  - location: "docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md:672-675"
    detail: "Failure table Provider-route row requires a distinguishable request-route-mismatch typed error; grep over opi-ai finds no such check or test (only construction-time WireCompatMismatch and api_mapped route-id tests)."
criterion_source: "Phase 17 design 'Failure, cancellation, and partial-result semantics' Provider-route row (request-route mismatch) and P17-FAL-001; task 17.1 DoD ('It validates one canonical provider:model Request'; 'request-route mismatch ... return typed errors')"
reproduction:
  - "Register a route whose catalog has model-a (image-capable) and model-b (text-only); call prepare_call(alpha:model-a) with request.model=alpha:model-b and image content; validation runs against model-a and start_attempt dispatches text-only model-b on the wire while PreparedRoute reports model-a."
confidence: high
status: unverified
```

### 3.2 MAJOR: P17-PLT-001/A15: three-platform CI evidence covers 40f2e6e, not the audited HEAD eb5e316 — post-exit commit 211aba8 changed 34 production source files with no CI coverage possible

**File:** `.audit-criteria.txt (PLT-001/A15 claims) vs git history`
**Lines:** criteria 193-199, 277-279
**Claim:** The only three-platform CI acceptance evidence cited for PLT-001 and A15 (run 31798070731, head 40f2e6e) predates the audited tree: commit 211aba8 (2026-08-15) subsequently changed 34 production source files across opi-agent, opi-ai, and opi-coding-agent (agent_loop.rs ~427 lines, provider_collection.rs ~128, opi-coding-agent evidence.rs ~229, credential_store.rs, authority.rs, evidence.rs, loop_types.rs, registry.rs, auth.rs) plus every phase17 test file, and is unpushed (origin/main at 877c41f), so no CI run can exist for eb5e316; the MUST 'identical on Linux, macOS, and Windows' is mechanically unverified at HEAD.
**Adversarial verification:** CONFIRMED -- Every factual element verified at HEAD. target/opi-artifacts/phase17-exit/ci-three-platform-evidence.json records run_id 31798070731 with head_sha 40f2e6ee4866f1cd44eefb952b8f40afcbb029ac, and criteria-trace-final.json cites exactly this run (and no other) as the evidence for both P17-PLT-001 and P17-A15 'at the FINAL exit SHA'. git log 40f2e6e..eb5e316 shows 7 commits; git diff 40f2e6e..eb5e316 shows 34 changed production source files under crat

```yaml
id: SP-001
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: spec
severity: Major
title: "P17-PLT-001/A15: three-platform CI evidence covers 40f2e6e, not the audited HEAD eb5e316 — post-exit commit 211aba8 changed 34 production source files with no CI coverage possible"
claim: "The only three-platform CI acceptance evidence cited for PLT-001 and A15 (run 31798070731, head 40f2e6e) predates the audited tree: commit 211aba8 (2026-08-15) subsequently changed 34 production source files across opi-agent, opi-ai, and opi-coding-agent (agent_loop.rs ~427 lines, provider_collection.rs ~128, opi-coding-agent evidence.rs ~229, credential_store.rs, authority.rs, evidence.rs, loop_types.rs, registry.rs, auth.rs) plus every phase17 test file, and is unpushed (origin/main at 877c41f), so no CI run can exist for eb5e316; the MUST 'identical on Linux, macOS, and Windows' is mechanically unverified at HEAD."
evidence:
  - location: "target/opi-artifacts/phase17-exit/ci-three-platform-evidence.json"
    detail: "head_sha recorded as 40f2e6ee4866f1cd44eefb952b8f40afcbb029ac, run 31798070731 — the SHA the PLT-001/A15 met-claims cite."
  - location: "git history 40f2e6e..eb5e316"
    detail: "7 commits; 211aba8 alone changes 34 production src files plus all phase17 test files (e.g. the AUT-008 per-request re-projection fix moving registry.definitions() inside the turn loop landed only in 211aba8)."
  - location: "git branch -r --contains eb5e316 (empty); git log origin/main -1 = 877c41f"
    detail: "211aba8 and eb5e316 are unpushed, so no CI run can cover the audited tree; the recorded matrix cannot cover HEAD."
criterion_source: "P17-PLT-001 / P17-A15 (design.md lines 761, 783); Phase-exit gates clause (design.md lines 802-805)"
reproduction:
  - "git diff 40f2e6e..eb5e316 -- crates (34 changed production src files after the recorded exit SHA)"
confidence: high
status: unverified
```

### 3.3 MINOR: EnvironmentFacts cannot represent the spec-listed trigger fact; its doc comment claims it does

**File:** `crates/opi-agent/src/evidence.rs`
**Lines:** 700-710
**Claim:** The Phase 17 manifest retention list (budget, trigger, time, platform/environment identity, measurement origin) cannot be represented: EnvironmentFacts has only budget/time/platform, its doc comment nevertheless claims trigger coverage, and no production path populates a trigger fact.
**Adversarial verification:** CONFIRMED -- Verified exactly. opi-agent/src/evidence.rs:700-710: EnvironmentFacts' doc comment reads 'Budget, trigger, time, and platform/environment identity plus measurement origin' but the struct fields are only budget, time, platform — no trigger field exists. opi-coding-agent/src/evidence.rs:462-468: build_finalized_manifest fills environment with { budget: capture.budget, time: Measurement::Unknown{NotReported}, platform: current_platform() }. A repo-w

```yaml
id: EVD-ENV-TRIGGER
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: spec
severity: Minor
title: "EnvironmentFacts cannot represent the spec-listed trigger fact; its doc comment claims it does"
claim: "The Phase 17 manifest retention list (budget, trigger, time, platform/environment identity, measurement origin) cannot be represented: EnvironmentFacts has only budget/time/platform, its doc comment nevertheless claims trigger coverage, and no production path populates a trigger fact."
evidence:
  - location: "crates/opi-agent/src/evidence.rs:700-710"
    detail: "Doc comment reads 'Budget, trigger, time, and platform/environment identity plus measurement origin' but the struct fields are only budget, time, platform."
  - location: "crates/opi-coding-agent/src/evidence.rs:462-468"
    detail: "Product manifest assembly fills EnvironmentFacts { budget, time: Unknown(NotReported), platform } — no trigger fact exists anywhere in the manifest contract."
  - location: "docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md:618-619"
    detail: "Spec bullet lists trigger among retained facts; parent CTRL-002 also lists it."
criterion_source: "Phase 17 design line 618-619 (Resolved-execution manifest retention list); docs/opi-spec.md CTRL-002 (budget, trigger, time, environment)"
reproduction:
  - "grep -n trigger crates/opi-agent/src/evidence.rs — only the EnvironmentFacts doc comment; no field. Read build_finalized_manifest environment construction: budget/time/platform only."
confidence: high
status: unverified
```

### 3.4 MINOR: ProvenanceFacts.fallback_allowed conflates fallback-was-used with fallback-was-allowed and drops the spec's stable fallback reason

**File:** `crates/opi-agent/src/evidence.rs`
**Lines:** 629-635
**Claim:** ProvenanceFacts.fallback_allowed is documented as whether an authentication fallback was allowed, when known, but the product feeds it the attempt-state (used/not_attempted), so every ordinary run records fallback_allowed=Some(false) (asserting fallback was not permitted) and a used fallback's from/to/stable_reason are dropped from the finalized manifest.
**Adversarial verification:** CONFIRMED -- Every citation checks out. ProvenanceFacts.fallback_allowed is an Option<bool> documented as the allowed-axis (evidence.rs:633-634), but the product feeds it the attempt axis: the loop writes only a "used"/"not_attempted" token into the Provider record (agent_loop.rs:1598-1602 via :204), and the product extractor maps used->Some(true), not_attempted->Some(false) (opi-coding-agent/src/evidence.rs:640-646). AuthProvenance's closed NotAttempted | Us

```yaml
id: EVD-FALLBACK-AXIS
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: spec
severity: Minor
title: "ProvenanceFacts.fallback_allowed conflates fallback-was-used with fallback-was-allowed and drops the spec's stable fallback reason"
claim: "ProvenanceFacts.fallback_allowed is documented as whether an authentication fallback was allowed, when known, but the product feeds it the attempt-state (used/not_attempted), so every ordinary run records fallback_allowed=Some(false) (asserting fallback was not permitted) and a used fallback's from/to/stable_reason are dropped from the finalized manifest."
evidence:
  - location: "crates/opi-agent/src/evidence.rs:629-635"
    detail: "Field declared as whether fallback was allowed, only an Option<bool>; the spec's closed not-attempted | used{from,to,stable_reason} classification is not representable."
  - location: "crates/opi-coding-agent/src/evidence.rs:640-646"
    detail: "fallback_allowed_from_token maps used to Some(true), not_attempted to Some(false) — writing the attempt-state under the allowed-axis."
  - location: "crates/opi-ai/src/auth.rs:144-160"
    detail: "Closed classification: NotAttempted means no fallback attempted (not not-allowed); Used{from,to,reason} carries a stable non-secret reason that never reaches the manifest."
  - location: "crates/opi-coding-agent/tests/phase17_product_evidence.rs:712 (per .audit-criteria.txt P17-A01)"
    detail: "P17-A01 test asserts fallback_allowed=Some(false) for a normal no-fallback run, pinning the mislabeled semantics as expected output."
criterion_source: "P17-PRV-005 / P17-EVD-003 provenance retention; design doc lines 279-289 (AuthProvenance closed fallback classification) and 612-613"
reproduction:
  - "Run any normal capture-enabled run and inspect manifest.json: provenance.fallback_allowed == false although env fallback may be permitted by policy — the allowed/used axes are indistinguishable; from/to/reason are absent."
confidence: high
status: unverified
```

### 3.5 MINOR: Partial-side-effect and cleanup-unknown terminal outcomes are defined but never produced in evidence

**File:** `crates/opi-coding-agent/src/harness.rs`
**Lines:** 90-98 (with crates/opi-agent/src/evidence.rs:761-763)
**Claim:** The spec requires the finalized manifest to retain terminal outcomes including partial failure and cleanup uncertainty, and defines TerminalOutcome::PartialSideEffect/CleanupUnknown for that purpose, but the harness maps only Ok to Success, Cancelled to Cancelled, and other to Failed, and a workspace-wide grep shows PartialSideEffect/CleanupUnknown are constructed nowhere, so a run whose tool ended cleanup-unknown finalizes a manifest whose outcome is Success and whose Tool evidence record carries only the boolean is_error.
**Adversarial verification:** CONFIRMED -- All citations verified. evidence_outcome maps Ok->Success, Err(Cancelled)->Cancelled, everything else->Failed (harness.rs:92-98), and this is the only source of FinalizedManifest.outcome (RunDynamicFacts.outcome -> build_finalized_manifest, opi-coding-agent/src/evidence.rs:410,448). A workspace-wide grep for PartialSideEffect|CleanupUnknown over all .rs sources returns only the enum definitions at opi-agent/src/evidence.rs:761,763 — no producer, 

```yaml
id: P17PROD-04
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: spec
severity: Minor
title: "Partial-side-effect and cleanup-unknown terminal outcomes are defined but never produced in evidence"
claim: "The spec requires the finalized manifest to retain terminal outcomes including partial failure and cleanup uncertainty, and defines TerminalOutcome::PartialSideEffect/CleanupUnknown for that purpose, but the harness maps only Ok to Success, Cancelled to Cancelled, and other to Failed, and a workspace-wide grep shows PartialSideEffect/CleanupUnknown are constructed nowhere, so a run whose tool ended cleanup-unknown finalizes a manifest whose outcome is Success and whose Tool evidence record carries only the boolean is_error."
evidence:
  - location: "crates/opi-coding-agent/src/harness.rs:92-98"
    detail: "evidence_outcome maps Ok to Success, Err(Cancelled) to Cancelled, Err(_) to Failed — no partial/cleanup-unknown classification ever reaches the manifest."
  - location: "crates/opi-agent/src/evidence.rs:761,763"
    detail: "TerminalOutcome::PartialSideEffect and CleanupUnknown are defined and referenced nowhere else (no producer, no test)."
  - location: "crates/opi-agent/src/agent_loop.rs:1114-1142"
    detail: "The tool outcome evidence record payload carries only phase/tool/registration_id/is_error; the cleanup/partial classification stays in ToolResult.diagnostics, which never crosses into evidence."
criterion_source: "Phase 17 design 'Resolved-execution manifest' (retains terminal outcomes including cancellation, retry, compaction, partial failure, and cleanup uncertainty); P17-EVD-002; CTRL-002"
reproduction:
  - "Run a harness prompt whose bash tool ends with FailureCode::CleanupUnconfirmed; the finalized manifest outcome is Success and no evidence value records cleanup uncertainty."
confidence: high
status: unverified
```

### 3.6 INFO: P17-NXT-005: product compaction applies post-loop through Agent::replace_state rather than the in-loop prepare_next_turn candidate transition named by one design sentence

**File:** `crates/opi-coding-agent/src/harness.rs`
**Lines:** 2706-2729 (threshold), 1874-1880 (replace_agent_context)
**Claim:** Threshold compaction runs in persist_turn after the loop returns and swaps the complete context via Agent::replace_state — the validated idle-state complete replacement the design's State-boundary section mandates — rather than entering through the loop's prepare_next_turn candidate as the Fixed-ordering prose (product compaction must enter the same complete-state transition rather than call a direct message replacement setter after the loop) can be read to require. The observable NXT-005 outcomes (complete replacement, no append-only residue, bounded provider-visible context) are implemented and tested, so this is an interpretation note on the design's internal tension, not an observable deviation.
**Adversarial verification:** CONFIRMED -- Implementation and design citations all verified. Threshold compaction runs in persist_turn after the loop returns (harness.rs:2706-2729) and swaps context via state_snapshot()/replace_state — the validated complete idle-state replacement — not through the loop's prepare_next_turn candidate transition; replace_agent_context (1874-1880, used by manual compaction at 3202) is the same validated seam. The behavioral closure is proven by phase17_compa

```yaml
id: SP-005
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: spec
severity: Info
title: "P17-NXT-005: product compaction applies post-loop through Agent::replace_state rather than the in-loop prepare_next_turn candidate transition named by one design sentence"
claim: "Threshold compaction runs in persist_turn after the loop returns and swaps the complete context via Agent::replace_state — the validated idle-state complete replacement the design's State-boundary section mandates — rather than entering through the loop's prepare_next_turn candidate as the Fixed-ordering prose (product compaction must enter the same complete-state transition rather than call a direct message replacement setter after the loop) can be read to require. The observable NXT-005 outcomes (complete replacement, no append-only residue, bounded provider-visible context) are implemented and tested, so this is an interpretation note on the design's internal tension, not an observable deviation."
evidence:
  - location: "crates/opi-coding-agent/src/harness.rs:2715-2723"
    detail: "Post-loop complete-state replacement via state_snapshot/replace_state through the validated idle seam, not the in-loop prepare_next_turn transition."
  - location: "crates/opi-coding-agent/tests/session_runtime.rs:3994-4115"
    detail: "Behavioral closure: post-compaction provider request is [summary, kept tail, current turn] with no standalone superseded message object — the append-only prohibition is proven."
  - location: "design.md lines 336-338 vs 374-376"
    detail: "The two design passages: the State boundary mandates the one validated idle-state replacement (which the implementation uses); the Fixed-ordering prose asks compaction to enter the same complete-state transition in-loop."
criterion_source: "P17-NXT-005 (design.md line 384); design.md lines 373-376 vs 336-338"
reproduction:
  - "n/a (static finding; cited tests runnable per Section 1)"
confidence: medium
status: unverified
```

## 4. Security / Redaction Findings

### 4.1 MINOR: 401/403 auth-error bodies embed only shape-redacted excerpts in azure/gemini/vertex/bedrock, with no canary test on this path

**File:** `crates/opi-ai/src/azure_openai.rs`
**Lines:** 358-371
**Claim:** The four adapters that embed provider error-body excerpts in their 401/403 auth-failure errors rely solely on shape-based redaction (http.rs safe_excerpt matches sk-/gh-/github_pat/AIza/JWT/bearer/credentialed-URL/query-secret patterns), so a proxy or gateway that echoes a submitted credential of another shape (an Azure api-key, an AWS session token, a Bedrock x-amz-security-token) into a 401/403 body would surface that raw value in AuthFailed diagnostics; the Anthropic/OpenAI families drop the body entirely for 401/403 citing exactly this threat, and the existing secret-canary matrix only exercises the 5xx ProviderSide path with an sk-shaped canary, never the 401/403 AuthFailed path.
**Adversarial verification:** CONFIRMED -- Every evidence citation holds at HEAD. azure_openai.rs map_azure_status (arms at 363-371), gemini.rs map_gemini_error (804-839 incl. the body-code 401/403 fallback), vertex.rs map_vertex_status (280-315 incl. fallback), and bedrock/mod.rs map_bedrock_status (985-1004) all embed crate::http::safe_excerpt(body) in AuthFailed for 401/403. http.rs:253-282 defines exactly the cited shape-based regexes (sk-/gh[pousr]_/github_pat_/AIza/JWT, bearer synta

```yaml
id: OPAI-ADPT-001
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: security
severity: Minor
title: "401/403 auth-error bodies embed only shape-redacted excerpts in azure/gemini/vertex/bedrock, with no canary test on this path"
claim: "The four adapters that embed provider error-body excerpts in their 401/403 auth-failure errors rely solely on shape-based redaction (http.rs safe_excerpt matches sk-/gh-/github_pat/AIza/JWT/bearer/credentialed-URL/query-secret patterns), so a proxy or gateway that echoes a submitted credential of another shape (an Azure api-key, an AWS session token, a Bedrock x-amz-security-token) into a 401/403 body would surface that raw value in AuthFailed diagnostics; the Anthropic/OpenAI families drop the body entirely for 401/403 citing exactly this threat, and the existing secret-canary matrix only exercises the 5xx ProviderSide path with an sk-shaped canary, never the 401/403 AuthFailed path."
evidence:
  - location: "crates/opi-ai/src/azure_openai.rs:358-371"
    detail: "map_azure_status 401/403 to AuthFailed with safe_excerpt(body) included."
  - location: "crates/opi-ai/src/gemini.rs:804-831 and crates/opi-ai/src/vertex.rs:280-307"
    detail: "Both map 401/403 (plus body-code fallback) to AuthFailed with safe_excerpt(body) embedded."
  - location: "crates/opi-ai/src/bedrock/mod.rs:985-994"
    detail: "map_bedrock_status 401/403 to AuthFailed with safe_excerpt(body)."
  - location: "crates/opi-ai/src/http.rs:253-282"
    detail: "Redaction regexes cover only known token shapes, bearer syntax, credentialed URLs, and query-string secret names — arbitrary api-key/session-token values do not match. In-repo threat-model precedent drops the body for 401/403 in anthropic.rs:1085-1088, openai_chat.rs:1287-1290, openai_responses.rs:463-464, openai_codex_responses.rs:379-387."
criterion_source: "P17-FAL-004 (error diagnostics MUST NOT expose credentials); Phase 17 design 'Risk thresholds' bullet 5; P17-A10 provider-error canaries"
reproduction:
  - "Register an azure route against wiremock returning 401 with body containing the literal submitted api-key (e.g. key=0123456789abcdef0123456789abcdef), call stream_prepared, and observe the AuthFailed message retains the key verbatim."
confidence: high
status: unverified
```

### 4.2 MINOR: Agent::emit_event bypasses the public redaction seam for harness lifecycle events (SessionPersistError.message, CompactionEnd.error_message/result.summary reach mode stdout unredacted)

**File:** `crates/opi-agent/src/agent.rs`
**Lines:** 305-310
**Claim:** Harness-emitted lifecycle events reach RPC/NDJSON subscribers without the redaction the same event variants receive on the agent-loop path: Agent::emit_event forwards raw events, so SessionPersistError.message and CompactionEnd.error_message (both explicitly redacted by AgentEvent::redacted_for_public) and CompactionEnd.result.summary (built verbatim from compacted user/tool text via generate_core_summary) are serialized unredacted to mode stdout.
**Adversarial verification:** CONFIRMED -- Tried to refute on four fronts and all held. (1) Bypass: Agent::emit_event (agent.rs:305-310) does a raw fan-out to the same subscriber list that the loop reaches only through emit_public_event, which applies event.redacted_for_public() first (agent_loop.rs:1616-1617; shared sink built at agent.rs:396-404). (2) Emission sites: harness emits SessionPersistError{message} and CompactionEnd{result,error_message} via agent.emit_event exactly in the ci

```yaml
id: SEC-001
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: security
severity: Minor
title: "Agent::emit_event bypasses the public redaction seam for harness lifecycle events (SessionPersistError.message, CompactionEnd.error_message/result.summary reach mode stdout unredacted)"
claim: "Harness-emitted lifecycle events reach RPC/NDJSON subscribers without the redaction the same event variants receive on the agent-loop path: Agent::emit_event forwards raw events, so SessionPersistError.message and CompactionEnd.error_message (both explicitly redacted by AgentEvent::redacted_for_public) and CompactionEnd.result.summary (built verbatim from compacted user/tool text via generate_core_summary) are serialized unredacted to mode stdout."
evidence:
  - location: "crates/opi-agent/src/agent.rs:305-310"
    detail: "Agent::emit_event forwards the raw AgentEvent to subscribers with no redacted_for_public() call, unlike the loop's emit_public_event which applies event.redacted_for_public() before the same fan-out."
  - location: "crates/opi-coding-agent/src/harness.rs:2699-2786"
    detail: "CodingHarness emits CompactionEnd{result,error_message} and SessionPersistError{message} through agent.emit_event on the persist_turn/persist_extension_state paths; these events reach mode subscribers unredacted."
  - location: "crates/opi-agent/src/event.rs:198-213"
    detail: "redacted_for_public explicitly redacts CompactionEnd.error_message and SessionPersistError.message, proving these variants are inside the intended redaction contract — but that arm is unreachable for harness-emitted lifecycle events because no caller applies it (RPC serializes as-is via agent_event_to_value; NDJSON copies error_message directly)."
  - location: "crates/opi-agent/src/compaction.rs:263-280"
    detail: "generate_core_summary embeds the first ~500 bytes of extracted message text (user prompts and tool-result output) verbatim into the compaction summary placed in CompactionEnd.result.summary and passed through unredacted; the product does treat this class as sensitive on the RPC session_info surface, where branch summaries are redacted."
criterion_source: "Phase 17 design P17-FAL-004 (error and evidence diagnostics MUST NOT expose credentials, raw environment values, or unclassified content; secret-canary matrix across text, JSON/NDJSON, RPC, trace, and manifest surfaces) and P17-A10"
reproduction:
  - "Drive a --json (or RPC) run whose session crosses the compaction threshold with a prompt or tool result containing a marker string; inspect stdout for a CompactionEnd line whose result.summary contains the marker verbatim."
  - "Force a session-persist io failure (read-only session dir) in --json mode; the SessionPersistError line carries the raw io error including the absolute session path."
confidence: high
status: unverified
```

### 4.3 INFO: Network error strings embed the request URL without safe_excerpt; SecretRedactor query regex omits bare key

**File:** `crates/opi-ai/src/anthropic.rs`
**Lines:** 962-967
**Claim:** Adapter network-error strings embed the full request URL without safe_excerpt (reqwest error Display includes the URL), and the opi-agent SecretRedactor query-parameter regex omits the bare key parameter name that opi-ai's own safe_excerpt covers, so a hypothetical URL query-string secret in a Network error would not be scrubbed at either layer; no reviewed wire currently places a secret in a request URL, so this is pattern-coverage asymmetry without a live leak.
**Adversarial verification:** CONFIRMED -- Every cited code fact verified. (1) ProviderError::Network(e.to_string()) is built without safe_excerpt at anthropic.rs:962-967, openai_chat.rs:1166-1172, gemini.rs:705-707, and bedrock/mod.rs:330 (which wraps the error in a format! string but still embeds the reqwest Display) — and I found two additional uncited sites with the same pattern (azure_openai.rs:270, vertex.rs:214), strengthening the claim. safe_excerpt is applied only to provider err

```yaml
id: SEC-002
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: security
severity: Info
title: "Network error strings embed the request URL without safe_excerpt; SecretRedactor query regex omits bare key"
claim: "Adapter network-error strings embed the full request URL without safe_excerpt (reqwest error Display includes the URL), and the opi-agent SecretRedactor query-parameter regex omits the bare key parameter name that opi-ai's own safe_excerpt covers, so a hypothetical URL query-string secret in a Network error would not be scrubbed at either layer; no reviewed wire currently places a secret in a request URL, so this is pattern-coverage asymmetry without a live leak."
evidence:
  - location: "crates/opi-ai/src/anthropic.rs:962-967"
    detail: "ProviderError::Network(e.to_string()) is built without safe_excerpt; only ProviderSide/ProviderError bodies pass through it (map_http_status). Same in openai_chat.rs:1166-1171, gemini.rs:705-707, bedrock/mod.rs:330."
  - location: "crates/opi-agent/src/streaming_proxy.rs:354-358"
    detail: "The SecretRedactor credential-bearing query-parameter pattern covers api[_-]?key|token|access_token|refresh_token|authorization but omits the bare key parameter name, while opi-ai's safe_excerpt query pattern (http.rs:275-282) does cover key — a URL of the form ?key=<secret> inside a Network error string would survive both layers."
  - location: "crates/opi-ai/src/gemini.rs:699-703"
    detail: "Gemini attaches its secret via the x-goog-api-key header, not the URL; no live leak exists."
criterion_source: "Phase 17 design P17-FAL-004 (defense-in-depth scope)"
reproduction:
  - "n/a (static finding; cited tests runnable per Section 1)"
confidence: high
status: unverified
```

### 4.4 INFO: ToolExecutionEnd.result is exempt from public redaction while partial_result/details/diagnostics are redacted

**File:** `crates/opi-agent/src/event.rs`
**Lines:** 152-175
**Claim:** redacted_for_public redacts args, partial_result, details, and diagnostics messages/context, but ToolExecutionEnd.result is cloned unchanged; the loop builds it from the tool result content, i.e. the tool output (e.g. bash stdout) crosses to RPC/NDJSON unredacted — consistent with the tested user-owned-data design (the canary test asserts the command is scrubbed while tool output text stays visible) but inconsistent at field level with the sibling fields on the same seam.
**Adversarial verification:** CONFIRMED -- Field-level facts verified exactly. In AgentEvent::redacted_for_public, ToolExecutionEnd (event.rs:152-175) clones `result` unchanged (line 163: `result: result.clone()`) while the sibling fields on the same seam are redacted: ToolExecutionStart/Update args via redact_public_value (139, 149), Update partial_result via redact_public_value (150), End details via redact_public_value (164), and diagnostics message via redact_text + context via redact

```yaml
id: SEC-004
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: security
severity: Info
title: "ToolExecutionEnd.result is exempt from public redaction while partial_result/details/diagnostics are redacted"
claim: "redacted_for_public redacts args, partial_result, details, and diagnostics messages/context, but ToolExecutionEnd.result is cloned unchanged; the loop builds it from the tool result content, i.e. the tool output (e.g. bash stdout) crosses to RPC/NDJSON unredacted — consistent with the tested user-owned-data design (the canary test asserts the command is scrubbed while tool output text stays visible) but inconsistent at field level with the sibling fields on the same seam."
evidence:
  - location: "crates/opi-agent/src/event.rs:152-175"
    detail: "redacted_for_public redacts args, partial_result, details, and diagnostics, but ToolExecutionEnd.result is cloned unchanged; the loop builds it as json of result.content, so tool output crosses to public events unredacted."
  - location: "crates/opi-agent/tests/tool_event_redaction.rs:103-191"
    detail: "tool_events_redact_command_context_and_provider_content_stays_unchanged asserts the command canary is scrubbed while the tool output text stays visible — documented-by-test design (user-owned data to the user's own client); noting only the field-level incoherence with partial_result handling."
criterion_source: "Phase 17 design Redaction boundary (scope is the EvidenceSink boundary, which is enforced)"
reproduction:
  - "n/a (static finding; cited tests runnable per Section 1)"
confidence: high
status: unverified
```

### 4.5 INFO: Adapters materialize secrets into non-zeroizing plain Strings: azure/gemini/vertex expose_secret().to_string() before the spawned HTTP task, and Bedrock/AWS credential fields plus SecretKey copies persist in memory after drop

**File:** `crates/opi-ai/src/azure_openai.rs`
**Lines:** 226 (with gemini.rs:842-858, vertex.rs:169, bedrock/credentials.rs:13-18, bedrock/sigv4.rs:15-20, provider_collection.rs:56-74)
**Claim:** azure, gemini, and vertex call expose_secret().to_string() in the stream_prepared body before spawning the task, creating a plain-String copy of the secret in task state, deviating from the documented Provider::stream_prepared contract of attaching the secret immediately before the HTTP request; these copies — like BedrockCredentials/AwsCredentials secret fields and provider_collection's SecretKey(String) — are plain Strings with redacted Debug/Display but no zeroization, unlike the SecretString/zeroize contract the credential module documents and opi-coding-agent implements for envelope buffers. No secret enters diagnostics, events, evidence, or errors in these adapters, so this is a contract-consistency and zeroization-gap deviation without an observed leakage channel.
**Adversarial verification:** CONFIRMED -- Every citation verified exactly. azure_openai.rs:226, gemini.rs:843, and vertex.rs:169 each run auth.secret.expose_secret().to_string() at stream_prepared entry and move the plain String into the spawned HTTP task, while the documented Provider::stream_prepared contract (provider.rs:29-48, esp. 36-38: 'attaching the secret via secrecy::ExposeSecret immediately before the HTTP request') is matched by the contrast adapters that keep ResolvedAuth an

```yaml
id: OPAI-ADPT-005
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: security
severity: Info
title: "Adapters materialize secrets into non-zeroizing plain Strings: azure/gemini/vertex expose_secret().to_string() before the spawned HTTP task, and Bedrock/AWS credential fields plus SecretKey copies persist in memory after drop"
claim: "azure, gemini, and vertex call expose_secret().to_string() in the stream_prepared body before spawning the task, creating a plain-String copy of the secret in task state, deviating from the documented Provider::stream_prepared contract of attaching the secret immediately before the HTTP request; these copies — like BedrockCredentials/AwsCredentials secret fields and provider_collection's SecretKey(String) — are plain Strings with redacted Debug/Display but no zeroization, unlike the SecretString/zeroize contract the credential module documents and opi-coding-agent implements for envelope buffers. No secret enters diagnostics, events, evidence, or errors in these adapters, so this is a contract-consistency and zeroization-gap deviation without an observed leakage channel."
evidence:
  - location: "crates/opi-ai/src/azure_openai.rs:226"
    detail: "let api_key = auth.secret.expose_secret().to_string(); at stream_prepared entry, moved into the spawned task; same eager pattern at gemini.rs:843 and vertex.rs:169 — the SecretString original zeroizes but the copy does not."
  - location: "crates/opi-ai/src/anthropic.rs:939 and crates/opi-ai/src/openai_chat.rs:1150-1153"
    detail: "Contrast: these keep SecretString-bearing ResolvedAuth and call expose_secret only inside stream_http at header-build time, matching the documented contract."
  - location: "crates/opi-ai/src/bedrock/credentials.rs:13-18; crates/opi-ai/src/bedrock/sigv4.rs:15-20"
    detail: "BedrockCredentials/AwsCredentials hold secret_access_key and session_token as plain String (Debug redacts, heap copies persist after drop)."
  - location: "crates/opi-ai/src/provider_collection.rs:56-74"
    detail: "SecretKey(String) stores raw key material with redacted Debug/Display but no zeroization; the documented contract is implemented in opi-coding-agent (EnvelopeFields/ApiKeyEnvelopeV1 zeroize, credential_store.rs:448-508, 563-588) but not for these opi-ai copies."
criterion_source: "Provider::stream_prepared contract documentation (crates/opi-ai/src/provider.rs:29-48); crates/opi-ai/src/credential.rs:39-48 (documented zeroization contract for credential material)"
reproduction:
  - "Read azure_openai.rs:226 vs anthropic.rs:939 and the trait doc at provider.rs:36-38; read bedrock/credentials.rs:13-18 and provider_collection.rs:56-74 — no zeroize on drop for the plain-String copies."
confidence: high
status: unverified
```

## 5. Test Quality Findings

### 5.1 MAJOR: P17-A09 four-kind one-run evidence graph is not demonstrated at HEAD: the cited test asserts the Compaction record does NOT share the prompt run identity, and no test reconstructs Provider+Retry+Tool+Compaction under one run identity

**File:** `crates/opi-coding-agent/tests/phase17_product_evidence.rs`
**Lines:** 1141-1175
**Claim:** The exit-criteria claim that P17-A09 is closed by Provider+Retry+Tool+Compaction all sharing one run identity under one strict complete manifest is false at HEAD: phase17_one_run_graph_includes_tool_execution_record proves Provider+Retry+Tool in the prompt run and then explicitly asserts the manual-compaction record has a different run identity plus a second separate finalized manifest; commit 211aba8 inverted the assertion from assert_eq (share ONE run identity) to assert_ne after phase exit. Structurally manual compaction is a separate evidence run (setup_evidence_run resets the sink; emit_manual_compaction_evidence mints a fresh IdentityAllocator); only the in-prompt threshold-compaction path (persist_turn to Agent::emit_compaction_evidence over the persisted allocator) emits a same-run Compaction record, and that correlation is proven only at the substrate level by driving the Agent method directly — never together with retry+tool through the product harness.
**Adversarial verification:** CONFIRMED -- Verified in full. At HEAD, phase17_one_run_graph_includes_tool_execution_record (phase17_product_evidence.rs:1106-1175) proves Provider+Retry+Tool share one run identity (assert_eq run ids, retry parented to provider call, tool parented into the graph) and then, after harness.compact(Manual), asserts the recorder contains ONLY Compaction-kind records and assert_ne!(compaction_rec.run, prompt_run) at line 1163, plus a second separate finalized man

```yaml
id: TQ-01
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: test-quality
severity: Major
title: "P17-A09 four-kind one-run evidence graph is not demonstrated at HEAD: the cited test asserts the Compaction record does NOT share the prompt run identity, and no test reconstructs Provider+Retry+Tool+Compaction under one run identity"
claim: "The exit-criteria claim that P17-A09 is closed by Provider+Retry+Tool+Compaction all sharing one run identity under one strict complete manifest is false at HEAD: phase17_one_run_graph_includes_tool_execution_record proves Provider+Retry+Tool in the prompt run and then explicitly asserts the manual-compaction record has a different run identity plus a second separate finalized manifest; commit 211aba8 inverted the assertion from assert_eq (share ONE run identity) to assert_ne after phase exit. Structurally manual compaction is a separate evidence run (setup_evidence_run resets the sink; emit_manual_compaction_evidence mints a fresh IdentityAllocator); only the in-prompt threshold-compaction path (persist_turn to Agent::emit_compaction_evidence over the persisted allocator) emits a same-run Compaction record, and that correlation is proven only at the substrate level by driving the Agent method directly — never together with retry+tool through the product harness."
evidence:
  - location: "crates/opi-coding-agent/tests/phase17_product_evidence.rs:1146-1163"
    detail: "Asserts no Provider record in the manual-compaction run and assert_ne!(compaction_rec.run, prompt_run) — the compaction record must NOT share the prompt run."
  - location: "git diff 40f2e6e..eb5e316 (commit 211aba8)"
    detail: "At the phase-exit SHA the same test asserted assert_eq of the run ids with a share-ONE-run-identity message; HEAD flipped it to assert_ne."
  - location: ".audit-criteria.txt P17-A09 row (253-255)"
    detail: "Claims the FOUR-KIND conjunction closed in ONE run, no deferral remains — describes no test at HEAD."
  - location: "crates/opi-coding-agent/src/harness.rs:3175,3230-3262"
    detail: "compact_with_diagnostic calls setup_evidence_run (resets sink to a new run); emit_manual_compaction_evidence mints a fresh IdentityAllocator, so manual compaction is structurally a separate run; only the in-prompt auto-compaction path (harness.rs:2711-2723) emits a same-run Compaction record and no test combines that path with retry+tool."
criterion_source: "P17-A09 and P17-EVD-002 (.audit-criteria.txt:253-255); design.md line 777; Phase 17 exit-audit audit_notes round3; PRIN-005"
reproduction:
  - "git diff 40f2e6e eb5e316 -- crates/opi-coding-agent/tests/phase17_product_evidence.rs (compaction run-identity assertion flip)"
  - "grep -rn compaction_rec.run crates/opi-coding-agent/tests/"
confidence: high
status: unverified
```

### 5.2 MAJOR: P17-A11 outcome-retention is untested at HEAD and the pinned exit evidence misstates test polarity (claims harness.prompt Ok with non-empty messages; tests assert Err(EvidenceFinalization))

**File:** `crates/opi-coding-agent/tests/phase17_product_evidence.rs`
**Lines:** 148-178, 230-265 (with .audit-criteria.txt:263)
**Claim:** After commit 211aba8 both the emission-failure and finalization-failure product tests assert harness.prompt returns Err(AgentError::EvidenceFinalization) and dropped their former assertions that the run's assistant output survives: the test named preserves_outcome checks only the Err shape, sink.has_failure(), and completed_manifest().is_none(), so no HEAD test proves the executed messages/session content survive an evidence failure (production persists the turn before finalize_evidence_run, but a regression discarding the turn on evidence failure would pass the current suite). The pinned exit-criteria EVID text (harness.prompt Ok with non-empty messages) therefore no longer matches HEAD in polarity or coverage; the criterion's MUSTs are met by the real assertions, making this an evidence-record inaccuracy plus an untested outcome-retention leg rather than a runtime defect.
**Adversarial verification:** CONFIRMED -- All load-bearing claims verified. At HEAD, evidence_emission_failure_withholds_manifest_and_preserves_outcome (phase17_product_evidence.rs:149-178) asserts only Err(AgentError::EvidenceFinalization) (167-171), sink.has_failure() (173), and completed_manifest().is_none() (174-177) — no assertion on messages or session content; finalization_failure_withholds_manifest_through_harness (231-265) likewise asserts Err + has_failure + no manifest. The gi

```yaml
id: TQ-02
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: test-quality
severity: Major
title: "P17-A11 outcome-retention is untested at HEAD and the pinned exit evidence misstates test polarity (claims harness.prompt Ok with non-empty messages; tests assert Err(EvidenceFinalization))"
claim: "After commit 211aba8 both the emission-failure and finalization-failure product tests assert harness.prompt returns Err(AgentError::EvidenceFinalization) and dropped their former assertions that the run's assistant output survives: the test named preserves_outcome checks only the Err shape, sink.has_failure(), and completed_manifest().is_none(), so no HEAD test proves the executed messages/session content survive an evidence failure (production persists the turn before finalize_evidence_run, but a regression discarding the turn on evidence failure would pass the current suite). The pinned exit-criteria EVID text (harness.prompt Ok with non-empty messages) therefore no longer matches HEAD in polarity or coverage; the criterion's MUSTs are met by the real assertions, making this an evidence-record inaccuracy plus an untested outcome-retention leg rather than a runtime defect."
evidence:
  - location: "crates/opi-coding-agent/tests/phase17_product_evidence.rs:149-177"
    detail: "At the exit SHA the test asserted prompt Ok with non-empty messages; at HEAD it asserts Err(EvidenceFinalization) plus only has_failure()/completed_manifest().is_none(); the name still promises outcome preservation but no assertion verifies it."
  - location: "git diff 40f2e6e..eb5e316, hunks ~141-162 and 228-250"
    detail: "The finalization-failure test likewise changed from expect run-completes + non-empty messages to assert Err(EvidenceFinalization)."
  - location: ".audit-criteria.txt:263 (P17-A11 row) and :107 (P17-EVD-008 row)"
    detail: "Claims harness.prompt Ok with non-empty messages — false at HEAD."
  - location: "crates/opi-coding-agent/tests/phase17_failure_rollback.rs:392-400"
    detail: "Recomposed leg asserts the same Err + no-manifest; no outcome-retention assertion."
criterion_source: "P17-A11 ('Actual execution outcome is retained, evidence is incomplete, and no finalized manifest exists'); P17-EVD-008; PRIN-005"
reproduction:
  - "git diff 40f2e6e eb5e316 -- crates/opi-coding-agent/tests/phase17_product_evidence.rs"
  - "grep -n preserves_outcome -A 30 crates/opi-coding-agent/tests/phase17_product_evidence.rs"
confidence: high
status: unverified
```

### 5.3 MINOR: Exit-ledger evidence text misdescribes cited assertions at HEAD: A01 actual-route flipped to mock-stamped metadata (typed not_reported actual never exercised on the product path), the AUT-005 caveat is stale, and AUT-008 across-requests rests on a single-request assertion

**File:** `.audit-criteria.txt`
**Lines:** 139, 151, 223 (with phase17_product_evidence.rs:850-854, phase17_tool_authority.rs:558-603, json_mode.rs:180-190)
**Claim:** After post-archive remediation commit 211aba8, several Phase 17 met-claim rows in the archived exit ledger (docs/snapshots/phase17/opi-impl-state.json criteria_trace; the live .opi-impl-state.json no longer carries these rows, and no .audit-criteria.txt exists in the worktree) describe cited evidence inaccurately at HEAD eb5e316: (a) the P17-A01/P17-PRV-005/P17-EVD-004 text claims the manifest asserts actual='' with a typed not_reported reason, while the product test now asserts a mock-stamped actual (provider mock, model mock-model, actual_reason None) because MockProvider's base_assistant hardcodes that metadata — the product-level unknown-actual-with-typed-reason path is pinned only at the substrate level; (b) the P17-AUT-005 caveat ('no dedicated execution-counter test drives an authorizer returning Err(AuthorizationError)') is stale — the FailingAuthorizer leg added in 211aba8 drives a real AuthorizationError::Failed through Agent::prompt asserting zero executions and stable_code authorization_unavailable; (c) the P17-AUT-008 'absent across requests' claim rests on single-request calls[0].tools assertions, with per-turn projection recomputation structural (agent_loop.rs:112) but not multi-request tested. The requirements themselves remain satisfied; only the ledger claim text mismatches HEAD tests.
**Adversarial verification:** PARTIALLY-CONFIRMED -- All three sub-claims verified against HEAD eb5e316, but the cited artifact does not exist: there is no .audit-criteria.txt anywhere in the worktree (tracked or untracked; git ls-files and filesystem search both empty), and rows 139/151/223 are unverifiable. The met-claims actually live in the archived exit ledger docs/snapshots/phase17/opi-impl-state.json criteria_trace (the live .opi-impl-state.json no longer carries the P17 rows), so the findin

```yaml
id: TQ-03
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: test-quality
severity: Minor
title: "Exit-ledger evidence text misdescribes cited assertions at HEAD: A01 actual-route flipped to mock-stamped metadata (typed not_reported actual never exercised on the product path), the AUT-005 caveat is stale, and AUT-008 across-requests rests on a single-request assertion"
claim: "After post-archive remediation commit 211aba8, several Phase 17 met-claim rows in the archived exit ledger (docs/snapshots/phase17/opi-impl-state.json criteria_trace; the live .opi-impl-state.json no longer carries these rows, and no .audit-criteria.txt exists in the worktree) describe cited evidence inaccurately at HEAD eb5e316: (a) the P17-A01/P17-PRV-005/P17-EVD-004 text claims the manifest asserts actual='' with a typed not_reported reason, while the product test now asserts a mock-stamped actual (provider mock, model mock-model, actual_reason None) because MockProvider's base_assistant hardcodes that metadata — the product-level unknown-actual-with-typed-reason path is pinned only at the substrate level; (b) the P17-AUT-005 caveat ('no dedicated execution-counter test drives an authorizer returning Err(AuthorizationError)') is stale — the FailingAuthorizer leg added in 211aba8 drives a real AuthorizationError::Failed through Agent::prompt asserting zero executions and stable_code authorization_unavailable; (c) the P17-AUT-008 'absent across requests' claim rests on single-request calls[0].tools assertions, with per-turn projection recomputation structural (agent_loop.rs:112) but not multi-request tested. The requirements themselves remain satisfied; only the ledger claim text mismatches HEAD tests."
evidence:
  - location: "crates/opi-coding-agent/tests/phase17_product_evidence.rs:851-854"
    detail: "A01 test asserts manifest.route.actual.provider_id == mock, model_id mock-model, actual_reason None for the beta run — not the claimed actual='' with typed not_reported reason."
  - location: "git diff 40f2e6e..eb5e316"
    detail: "The exit-SHA version asserted actual.provider_id == '' with Some(UnknownReason::NotReported) — inverted at HEAD."
  - location: "crates/opi-ai/src/test_support.rs:187-193"
    detail: "base_assistant() hardcodes provider mock / model mock-model, so every product test's provider-reported actual is mock regardless of the dispatching provider; no product test exercises actual_route_from_messages returning None (harness.rs:2897-2920); the typed not_reported actual is pinned only at the substrate level (evidence_runtime.rs:288-293)."
  - location: ".audit-criteria.txt P17-AUT-005 row (139)"
    detail: "Caveat text claims no dedicated execution-counter test drives an authorizer returning Err(AuthorizationError)."
criterion_source: "P17-A01, P17-PRV-005, P17-EVD-004; P17-AUT-005; P17-AUT-008; PRIN-005 (claims follow immutable, reproducible evidence)"
reproduction:
  - "git diff 40f2e6e eb5e316 -- crates/opi-coding-agent/tests/phase17_product_evidence.rs (A01 actual assertions)"
  - "compare .audit-criteria.txt rows 139/151/223 against phase17_tool_authority.rs:558-603 and json_mode.rs:180-190 at HEAD eb5e316"
confidence: high
status: unverified
```

### 5.4 MINOR: P17-RBK-004 before/after policy snapshot is near-tautological: the after policy is rebuilt from identical literals, never read from the harness

**File:** `crates/opi-coding-agent/tests/phase17_failure_rollback.rs`
**Lines:** 579-663
**Claim:** phase17_rollback_does_not_widen_user_policy compares build_policy() output before and after a harness prompt, but build_policy is a pure function over the same literal inputs, so digest equality cannot detect run-driven widening of the harness's actual effective policy; only global-state corruption of the literal-derived policy would flip it.
**Adversarial verification:** CONFIRMED -- Verified at phase17_failure_rollback.rs:577-663. build_policy is a capture-less closure (580-591) that calls EffectiveUserPolicy::build over the same literals (Interactive, ["read"], mutating=false, PermissionPolicy::empty(), complete-evidence=false, "project"/"package"/"workspace") both times; `before` at 592 and `after` at 627. Nothing from the harness run (621-624) flows into the compared value — the digest is a pure sha256 over those literals

```yaml
id: TQ-04
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: test-quality
severity: Minor
title: "P17-RBK-004 before/after policy snapshot is near-tautological: the after policy is rebuilt from identical literals, never read from the harness"
claim: "phase17_rollback_does_not_widen_user_policy compares build_policy() output before and after a harness prompt, but build_policy is a pure function over the same literal inputs, so digest equality cannot detect run-driven widening of the harness's actual effective policy; only global-state corruption of the literal-derived policy would flip it."
evidence:
  - location: "crates/opi-coding-agent/tests/phase17_failure_rollback.rs:580-592,627-632"
    detail: "Both before and after construct EffectiveUserPolicy from the same literals (Interactive, read-only capabilities, empty PermissionPolicy, same scope strings); the harness's own EffectiveUserPolicy is never inspected."
  - location: "crates/opi-coding-agent/tests/phase17_failure_rollback.rs:621-624"
    detail: "The prompt content (try to escalate permissions) has no data-flow path into build_policy(), so the run cannot influence the compared value."
criterion_source: "P17-RBK-004"
reproduction:
  - "Read phase17_failure_rollback.rs:579-663 — both compared values derive from the same pure function over identical literals."
confidence: high
status: unverified
```

### 5.5 MINOR: EVD-011 lifecycle conformance covers only no-op and in-memory sinks; file adapter is a separate hand-written test and the product-level strict-gate negative was downgraded to a constructor check

**File:** `crates/opi-agent/tests/evidence_contract.rs`
**Lines:** 700-717 (with phase17_product_evidence.rs:1034-1039)
**Claim:** The generic exercise<S: EvidenceSink> lifecycle conformance harness in opi-agent runs only NoopEvidenceSink and InMemoryEvidenceSink; the Reference Product FileEvidenceSink (structurally unable to join this opi-agent test because it lives in the downstream crate) is covered by a separate hand-rolled lifecycle test plus harness-driven end-to-end tests, but no single shared conformance contract exercises all three adapters, per-phase failure injection exists only for the in-memory sink, and the file adapter's lifecycle test omits the finalize_artifact step. Additionally, the product-level strict-gate missing-config-identity negative present at the phase-exit SHA (mutation of manifest.config.harness_digest to empty + require_complete().is_err()) was replaced at HEAD by a ContentDigest::from_hex("").is_err() constructor check. That swap reflects a real hardening — from_hex now rejects non-canonical SHA-256 text, making an empty config digest unrepresentable through the public API (ContentDigest has no Deserialize/Default and a private field), so no constructible strict-gate scenario went unexercised — but the retained empty-digest branches of require_complete (evidence.rs:1132-1176) are now untestable dead-at-HEAD defensive code, and the EVD-011 'one applicable lifecycle/failure conformance contract' across all three adapters is not literally realized as one shared contract.
**Adversarial verification:** PARTIALLY-CONFIRMED -- The factual evidence all holds at HEAD: (1) noop_and_in_memory_satisfy_one_lifecycle_conformance_contract (evidence_contract.rs:700-717) runs the generic exercise only for the no-op and in-memory sinks; (2) FileEvidenceSink's lifecycle-shaped test is the separate file_evidence_sink_writes_records_and_manifest (phase17_product_evidence.rs:272-416), which never calls finalize_artifact — a step the shared contract includes (evidence_contract.rs:712)

```yaml
id: TQ-06
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: test-quality
severity: Minor
title: "EVD-011 lifecycle conformance covers only no-op and in-memory sinks; file adapter is a separate hand-written test and the product-level strict-gate negative was downgraded to a constructor check"
claim: "The generic exercise<S: EvidenceSink> lifecycle conformance harness in opi-agent runs only NoopEvidenceSink and InMemoryEvidenceSink; the Reference Product FileEvidenceSink (structurally unable to join this opi-agent test because it lives in the downstream crate) is covered by a separate hand-rolled lifecycle test plus harness-driven end-to-end tests, but no single shared conformance contract exercises all three adapters, per-phase failure injection exists only for the in-memory sink, and the file adapter's lifecycle test omits the finalize_artifact step. Additionally, the product-level strict-gate missing-config-identity negative present at the phase-exit SHA (mutation of manifest.config.harness_digest to empty + require_complete().is_err()) was replaced at HEAD by a ContentDigest::from_hex(\"\").is_err() constructor check. That swap reflects a real hardening — from_hex now rejects non-canonical SHA-256 text, making an empty config digest unrepresentable through the public API (ContentDigest has no Deserialize/Default and a private field), so no constructible strict-gate scenario went unexercised — but the retained empty-digest branches of require_complete (evidence.rs:1132-1176) are now untestable dead-at-HEAD defensive code, and the EVD-011 'one applicable lifecycle/failure conformance contract' across all three adapters is not literally realized as one shared contract."
evidence:
  - location: "crates/opi-agent/tests/evidence_contract.rs:701-717"
    detail: "noop_and_in_memory_satisfy_one_lifecycle_conformance_contract calls exercise only for Noop and InMemory sinks; FileEvidenceSink (opi-coding-agent) cannot claim membership in this one contract and is exercised only by the hand-written file_evidence_sink_writes_records_and_manifest."
  - location: "crates/opi-coding-agent/tests/phase17_product_evidence.rs:1034-1039"
    detail: "The exit-SHA test mutated manifest.config.harness_digest to empty and asserted require_complete().is_err(); at HEAD this became assert of from_hex of the empty string erroring — a constructor check that no longer proves the strict gate rejects a missing config identity (criteria still claims missing harness-config digest rejected)."
  - location: "git diff 40f2e6e..eb5e316, hunk ~956-1031"
    detail: "git diff shows the require_complete missing-config assertion being replaced by the constructor assertion."
criterion_source: "P17-EVD-011, P17-EVD-003"
reproduction:
  - "read crates/opi-agent/tests/evidence_contract.rs:700-717 and crates/opi-coding-agent/tests/phase17_product_evidence.rs:1034-1039"
confidence: high
status: unverified
```

### 5.6 MINOR: P17-A14/MIG-005 cross-mode equivalence rests on construction and citation: NonInteractiveRunner::cancel is uncalled and untested, the cross-mode fixture is tool-free, and cancellation is behaviorally proven only on the interactive-token and RPC-abort seams

**File:** `crates/opi-coding-agent/src/runner.rs`
**Lines:** 729-732
**Claim:** The print/JSON-mode cancellation entry NonInteractiveRunner::cancel (a one-line delegation to harness.cancel(), runner.rs:729-732) has no production caller and no test anywhere in the workspace (the print/JSON entry in main.rs never calls it and installs no signal handler; zero cancel occurrences in non_interactive.rs/json_mode.rs), so P17-A14 cancellation-equivalence rests on behavioral proof for only the interactive-token path and the RPC-abort path. The cross-mode golden test drives no cancellation in any of the five modes and argues convergence by citation of the harness token; it is also tool-free in outcome (the fixture emits no tool_call_response; the four runner constructions use ToolSelection::Disabled while the interactive/harness library-seam modes use the builder default ToolSelection::Default, which projects the coding default toolset), so per-mode authority semantics are exercised in none of the five modes, with authority equivalence delegated to the 17.4 A06-A08 matrix, which runs against direct ToolAuthorizer::authorize calls and opi-agent Agent::prompt rather than the print/JSON/RPC entry points. The delegation is honestly recorded in the test header, making this a scoped-coverage gap plus a dead public API rather than a hidden claim.
**Adversarial verification:** PARTIALLY-CONFIRMED -- Core claim verified at HEAD eb5e316: runner.rs:729-732 is exactly the one-line cancel delegation, and a workspace-wide grep for .cancel(), runner.cancel, and ::cancel( finds zero production or test callers (main.rs print/JSON path at ~800-940 never calls it and installs no signal handler; grep of non_interactive.rs/non_interactive_policy.rs/json_mode.rs for 'cancel' returns nothing; other crates' cancel hits are CancellationToken/agent internals)

```yaml
id: P17-XMODE-001
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: test-quality
severity: Minor
title: "P17-A14/MIG-005 cross-mode equivalence rests on construction and citation: NonInteractiveRunner::cancel is uncalled and untested, the cross-mode fixture is tool-free, and cancellation is behaviorally proven only on the interactive-token and RPC-abort seams"
claim: "The print/JSON-mode cancellation entry NonInteractiveRunner::cancel (a one-line delegation to harness.cancel(), runner.rs:729-732) has no production caller and no test anywhere in the workspace (the print/JSON entry in main.rs never calls it and installs no signal handler; zero cancel occurrences in non_interactive.rs/json_mode.rs), so P17-A14 cancellation-equivalence rests on behavioral proof for only the interactive-token path and the RPC-abort path. The cross-mode golden test drives no cancellation in any of the five modes and argues convergence by citation of the harness token; it is also tool-free in outcome (the fixture emits no tool_call_response; the four runner constructions use ToolSelection::Disabled while the interactive/harness library-seam modes use the builder default ToolSelection::Default, which projects the coding default toolset), so per-mode authority semantics are exercised in none of the five modes, with authority equivalence delegated to the 17.4 A06-A08 matrix, which runs against direct ToolAuthorizer::authorize calls and opi-agent Agent::prompt rather than the print/JSON/RPC entry points. The delegation is honestly recorded in the test header, making this a scoped-coverage gap plus a dead public API rather than a hidden claim."
evidence:
  - location: "crates/opi-coding-agent/src/runner.rs:729-732"
    detail: "pub fn cancel(&self) { self.harness.cancel(); } — grep over opi-coding-agent src and tests finds zero call sites."
  - location: "crates/opi-coding-agent/tests/phase17_cross_mode.rs:16-25,408-437"
    detail: "The golden test drives no cancellation in any of the five modes; its header argues cancellation converges on the harness CancellationToken proven (RPC path only) in phase17_failure_rollback."
  - location: "crates/opi-coding-agent/tests/phase17_cross_mode.rs:246-258,288-292,352-366"
    detail: "All runner modes constructed with ToolSelection::Disabled and the fixture emits no tool_call_response; equivalence asserted over dispatch counts, model strings, evidence kinds, manifest presence, session_summary route, RPC AgentEnd — zero authority surface."
  - location: "crates/opi-coding-agent/tests/phase17_cross_mode.rs:476-485"
    detail: "tool-execution-counts.json hardcodes 0 for every mode; authority equivalence delegated to the 17.4 matrix, which runs only against CodingHarness/Agent::prompt, not the print/JSON/RPC entry points."
criterion_source: "P17-A14 / P17-MIG-005 (design.md Acceptance scenarios and Compatibility-and-migration tables; cancellation-equivalence and mode-consistency claims)"
reproduction:
  - "grep -rn .cancel() crates/opi-coding-agent/src crates/opi-coding-agent/tests — the only NonInteractiveRunner hit is the definition at runner.rs:730"
  - "cargo test -p opi-coding-agent --test non_interactive (no cancellation test present)"
confidence: high
status: unverified
```

### 5.7 MINOR: strip_comments safety claim is false: string literals containing // or /* can hide removed-interface tokens from the MIG-006 scan

**File:** `crates/opi-coding-agent/tests/phase17_api_audit.rs`
**Lines:** 67-102
**Claim:** The MIG-006 removal audit's comment stripper drops source text triggered by comment markers inside string literals (a string containing // hides the rest of its line; a string containing /* enters block-comment depth and can suppress scanning of the remainder of the file), so the documented invariant that over-stripping can only make the audit fail louder, never pass a retained symbol silently, is false, leaving a latent blind spot in the only mechanical scan pinning removed-interface absence.
**Adversarial verification:** CONFIRMED -- Verified by reading strip_comments (phase17_api_audit.rs:70-102) in full: the scanner tracks only comment state, never string-literal state. A '//' inside a string literal hits the line-comment branch (lines 89-92) and skips to the next newline, removing the remainder of that line from the scanned output; a '/*' inside a string literal sets depth=1 (lines 93-95) and suppresses everything until the next '*/' or EOF. Since the removed-interface ass

```yaml
id: P17-XMODE-002
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: test-quality
severity: Minor
title: "strip_comments safety claim is false: string literals containing // or /* can hide removed-interface tokens from the MIG-006 scan"
claim: "The MIG-006 removal audit's comment stripper drops source text triggered by comment markers inside string literals (a string containing // hides the rest of its line; a string containing /* enters block-comment depth and can suppress scanning of the remainder of the file), so the documented invariant that over-stripping can only make the audit fail louder, never pass a retained symbol silently, is false, leaving a latent blind spot in the only mechanical scan pinning removed-interface absence."
evidence:
  - location: "crates/opi-coding-agent/tests/phase17_api_audit.rs:67-102"
    detail: "Doc comment asserts over-stripping can only fail louder; mechanically, format! with a URL-bearing literal strips tokens later on the same line, and a string containing /* sets block-comment depth and strips everything until */ or EOF, excluding a retained symbol's definition from the scan."
  - location: "crates/opi-ai/src/http.rs:223; crates/opi-coding-agent/src/tool/find.rs:15; crates/opi-coding-agent/src/tool/glob.rs:14"
    detail: "Currently latent: only three /* occurrences exist in the six crates' src, each inside /// doc comments; definition-site reintroduction is also caught because definitions contain the bare token, and the Allow-variant case is pinned by the same-crate exhaustive match in opi-agent/src/agent_loop.rs:1281-1291 (no wildcard)."
criterion_source: "P17-MIG-006 (Phase 17 design, Compatibility and migration)"
reproduction:
  - "Apply strip_comments to the line: let s = 'a/*b'.to_owned(); let p = SharedProvider {}; — the SharedProvider token is absent from the stripped output, so phase17_removed_interfaces_are_absent_from_production_source passes despite the retained symbol."
confidence: high
status: unverified
```

### 5.8 INFO: Canary/absence guards have scope or vacuity gaps: platform scan covers 3 files only, hermetic scan misses non-phase17-named tests, and one redaction canary is vacuous

**File:** `crates/opi-coding-agent/tests/phase17_cross_mode.rs`
**Lines:** 550-571 (with phase17_api_audit.rs:282-307, auth_contracts.rs:199-241)
**Claim:** Three absence-scanning tests are weaker than their names suggest: the platform-neutrality scan covers only the three 17.9 binaries, the hermetic-network scan matches only files named phase17* in two crates (missing the opi-ai phase17 module and per_request_auth.rs), and the auth-provenance redaction canary never plants the secret in any input so its absence assertion cannot fail.
**Adversarial verification:** CONFIRMED -- All three sub-claims verified. (a) phase17_platform_contract_is_platform_neutral include_str!s exactly phase17_cross_mode.rs, phase17_failure_rollback.rs, phase17_api_audit.rs; nine phase17 test sources exist in opi-coding-agent/tests (including phase17_provider_runtime/product_evidence/tool_authority/legacy_migration/artifact_truthfulness and common/phase17.rs) and greps for cfg(target/cfg(unix/cfg(windows)/#[ignore] return zero hits in all of t

```yaml
id: TQ-08
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: test-quality
severity: Info
title: "Canary/absence guards have scope or vacuity gaps: platform scan covers 3 files only, hermetic scan misses non-phase17-named tests, and one redaction canary is vacuous"
claim: "Three absence-scanning tests are weaker than their names suggest: the platform-neutrality scan covers only the three 17.9 binaries, the hermetic-network scan matches only files named phase17* in two crates (missing the opi-ai phase17 module and per_request_auth.rs), and the auth-provenance redaction canary never plants the secret in any input so its absence assertion cannot fail."
evidence:
  - location: "crates/opi-coding-agent/tests/phase17_cross_mode.rs:551-556"
    detail: "phase17_platform_contract_is_platform_neutral include_str!s only phase17_cross_mode/failure_rollback/api_audit; provider_runtime, product_evidence, tool_authority, legacy_migration, and common/phase17.rs are outside the scan (verified today to contain no cfg target attributes and no ignore, so the gap is latent)."
  - location: "crates/opi-coding-agent/tests/phase17_api_audit.rs:282-307"
    detail: "The hermetic scan filters files starting with phase17 under opi-coding-agent/tests and opi-agent/tests only; crates/opi-ai/tests/provider_collection.rs mod phase17 and per_request_auth.rs are not scanned (both are network-free today; per_request_auth uses local wiremock so the scan proxy would not detect it anyway)."
  - location: "crates/opi-ai/tests/auth_contracts.rs:199-241"
    detail: "auth_provenance_debug_carries_no_secret asserts the debug output does not contain sk-super-secret where no input ever contains that value — a vacuous canary (contrast provider_collection.rs:1960-1984 where a canary flows through the real resolver)."
criterion_source: "P17-PLT-002, P17-A15 (task-local precondition)"
reproduction:
  - "grep -rn cfg(target_os / cfg(unix / cfg(windows across crates/opi-coding-agent/tests/phase17*.rs and common/phase17.rs (zero hits at HEAD — gaps are latent)"
confidence: high
status: unverified
```

### 5.9 INFO: Platform-neutral acceptance scan misses #[cfg_attr(...)] and #[ignore = ...] attribute forms

**File:** `crates/opi-coding-agent/tests/phase17_cross_mode.rs`
**Lines:** 550-571
**Claim:** The task-local P17-A15 platform-neutrality guard matches only attributes whose trimmed line starts with exactly #[cfg( or #[ignore], so #[cfg_attr(windows, ignore)] (or #[ignore = reason]) would evade both checks and let a test silently skip on one OS while CI stays green.
**Adversarial verification:** CONFIRMED -- Verified exactly. phase17_platform_contract_is_platform_neutral (phase17_cross_mode.rs:550-571) asserts on each trimmed line: !(t.starts_with("#[cfg(") && (contains target_os|unix|windows)) and !t.starts_with("#[ignore]"). "#[cfg_attr(windows, ignore)]" does not start with "#[cfg(" (6th char is '_' not '(') and does not start with "#[ignore]", so it evades both checks while expanding to #[ignore] on Windows; "#[ignore = \"...\"]" likewise fails t

```yaml
id: P17-XMODE-003
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: test-quality
severity: Info
title: "Platform-neutral acceptance scan misses #[cfg_attr(...)] and #[ignore = ...] attribute forms"
claim: "The task-local P17-A15 platform-neutrality guard matches only attributes whose trimmed line starts with exactly #[cfg( or #[ignore], so #[cfg_attr(windows, ignore)] (or #[ignore = reason]) would evade both checks and let a test silently skip on one OS while CI stays green."
evidence:
  - location: "crates/opi-coding-agent/tests/phase17_cross_mode.rs:550-571"
    detail: "Assertions require the trimmed line to start with #[cfg( plus a target specifier, or #[ignore]; the token #[cfg_attr( does not start with #[cfg( and #[ignore = ... does not start with #[ignore]."
  - location: "crates/opi-coding-agent/tests/phase17_cross_mode.rs (whole-file grep)"
    detail: "Latent only: the three acceptance sources contain no cfg_attr and no ignore attributes today; the CI acceptance job itself is verified OS-neutral with identical steps on the three OSes."
criterion_source: "P17-A15 task-local precondition / P17-PLT-001 (Phase 17 design, Platform scope)"
reproduction:
  - "Add #[cfg_attr(windows, ignore)] above any test in phase17_failure_rollback.rs: the platform-neutral scan still passes and the Windows CI job stays green while the test silently skips."
confidence: high
status: unverified
```

### 5.10 MINOR: Mid-attempt credential rejection/expiry termination is untested and unlatched at the substrate

**File:** `crates/opi-ai/tests/provider_collection.rs`
**Lines:** 1913-1957
**Claim:** The opi-ai substrate provides no mechanism and no test ensuring that a credential rejection or expiry observed during an attempt terminates the logical call: after an attempt stream ends with Err(CredentialRevoked/CredentialNeeded) the active slot is released and PreparedProviderCall::start_attempt will dispatch again with the same frozen auth; termination is delegated entirely to the caller honoring ProviderError::is_retryable().
**Adversarial verification:** CONFIRMED -- Verified from the full source. AttemptStream::poll_next (provider_collection.rs:689-705) releases the active slot on Poll::Ready(None), on Ready(Some(Err(_))) — including CredentialRevoked/CredentialNeeded — and on terminal events; Drop also releases. start_attempt (756-771) gates only on request.cancel.is_cancelled() and the active flag; nothing latches a terminal credential-rejection state, so after a stream ends with Err(CredentialRevoked) a s

```yaml
id: OPAI-17_1-002
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: test-quality
severity: Minor
title: "Mid-attempt credential rejection/expiry termination is untested and unlatched at the substrate"
claim: "The opi-ai substrate provides no mechanism and no test ensuring that a credential rejection or expiry observed during an attempt terminates the logical call: after an attempt stream ends with Err(CredentialRevoked/CredentialNeeded) the active slot is released and PreparedProviderCall::start_attempt will dispatch again with the same frozen auth; termination is delegated entirely to the caller honoring ProviderError::is_retryable()."
evidence:
  - location: "crates/opi-ai/src/provider_collection.rs:689-712,756-771"
    detail: "AttemptStream releases the active slot on any terminal event or error (including CredentialRevoked) and start_attempt only gates on cancellation and the active flag — nothing latches a terminal credential-rejection state."
  - location: "crates/opi-ai/tests/provider_collection.rs:1913-1957"
    detail: "rejected_or_expired_credential_terminates_the_call_without_dispatch covers only preparation-time resolver failures; no opi-ai test drives a stream that errors with CredentialRevoked and then asserts no further attempt."
  - location: "crates/opi-ai/src/provider.rs:400-405"
    detail: "The only enforcement is the taxonomy: is_retryable() is false for CredentialRevoked/CredentialNeeded, honored by the in-repo caller (agent_loop.rs:600,678); an embedder driving start_attempt directly on the public seam is not constrained."
criterion_source: ".audit-digest.txt task 17.1 DoD ('Credential rejection/expiry terminates the logical call'; public opi-ai tests cover rejection/expiry termination); design doc 'A credential rejection or expiry ends the logical call rather than refreshing inside a retry'"
reproduction:
  - "prepare_call on a route whose resolver succeeds, then have the attempt stream yield Err(ProviderError::CredentialRevoked); after draining, start_attempt returns Ok and re-dispatches with the frozen auth."
confidence: high
status: unverified
```

### 5.11 MINOR: api_mapped_provider.rs test double still stores and invokes an AuthResolver inside stream_prepared and asserts per-stream re-resolution

**File:** `crates/opi-ai/tests/api_mapped_provider.rs`
**Lines:** 40-95, 241-277
**Claim:** Task 17.5 required migrating Provider implementations/tests to the prepared seam and removing provider-owned AuthResolver invocation/state; the api_mapped_provider.rs test double (RecordingRoute) stores Arc<dyn AuthResolver> as state and invokes it inside stream_prepared, and two tests assert provider-side per-stream re-resolution (calls==2 and calls==3), preserving the removed provider-owned-auth-resolution semantics as expected behavior even though all production adapters are clean.
**Adversarial verification:** CONFIRMED -- Verified exactly as claimed. RecordingRoute (api_mapped_provider.rs:40-45) holds an `auth: Arc<dyn AuthResolver>` field, and its stream_prepared (72-94) calls `auth.resolve().await?` inside the provider stream, ignoring the passed-in `_auth`. mapped_routes_share_one_lazy_auth_resolver (242-263) asserts calls==2 after two direct stream_prepared calls, and mapped_provider_re_resolves_auth_for_every_stream (266-277) asserts calls==3 — the per-stream

```yaml
id: OPAI-ADPT-003
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: test-quality
severity: Minor
title: "api_mapped_provider.rs test double still stores and invokes an AuthResolver inside stream_prepared and asserts per-stream re-resolution"
claim: "Task 17.5 required migrating Provider implementations/tests to the prepared seam and removing provider-owned AuthResolver invocation/state; the api_mapped_provider.rs test double (RecordingRoute) stores Arc<dyn AuthResolver> as state and invokes it inside stream_prepared, and two tests assert provider-side per-stream re-resolution (calls==2 and calls==3), preserving the removed provider-owned-auth-resolution semantics as expected behavior even though all production adapters are clean."
evidence:
  - location: "crates/opi-ai/tests/api_mapped_provider.rs:40-45,72-77"
    detail: "RecordingRoute struct holds field auth: Arc<dyn AuthResolver> and stream_prepared calls auth.resolve().await? inside the provider stream."
  - location: "crates/opi-ai/tests/api_mapped_provider.rs:241-277"
    detail: "mapped_routes_share_one_lazy_auth_resolver asserts calls==2 after two direct stream_prepared calls; mapped_provider_re_resolves_auth_for_every_stream asserts calls==3 — the exact per-stream provider-side resolution the collection now owns."
  - location: "grep over crates/opi-ai/src"
    detail: "Production is clean: AuthResolver storage/invocation exists only in auth.rs, provider_collection.rs, and test_support.rs; no adapter owns a resolver."
criterion_source: "Phase 17 design 'Dispatchable provider collection' (concrete adapters no longer store or call an AuthResolver); task 17.5 DoD (.audit-digest.txt: migrates all remaining Provider implementations/tests, removes provider-owned AuthResolver invocation/state)"
reproduction:
  - "cargo test -p opi-ai --test api_mapped_provider mapped_provider_re_resolves_auth_for_every_stream (passes, asserting removed semantics)"
confidence: high
status: unverified
```

### 5.12 INFO: Two timing patterns carry residual flake/hang risk on loaded CI (fixed 120ms sleep before cancel; unbounded yield-loop with no deadline)

**File:** `crates/opi-agent/tests/agent_loop_semantics.rs`
**Lines:** 1848-1861 (with opi-ai provider_collection.rs:1758-1761)
**Claim:** One test relies on a fixed 120ms sleep before cancelling and then asserts exactly one provider call (can fail if the spawned loop is not scheduled within the window), and another spins unboundedly on a counter that a regression could leave at zero (hang instead of clean failure).
**Adversarial verification:** CONFIRMED -- Both cited sites verified verbatim. agent_loop_semantics.rs:1849-1850 sleeps a fixed Duration::from_millis(120) then calls cancel.cancel(), and lines 1857-1861 assert call_count == 1 with the message 'provider was called once before the cancel was observed' — if an extreme scheduler stall prevented the spawned agent_loop task from reaching provider dispatch within 120ms, the cancel lands first, the loop returns Cancelled with zero provider calls,

```yaml
id: TQ-07
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: test-quality
severity: Info
title: "Two timing patterns carry residual flake/hang risk on loaded CI (fixed 120ms sleep before cancel; unbounded yield-loop with no deadline)"
claim: "One test relies on a fixed 120ms sleep before cancelling and then asserts exactly one provider call (can fail if the spawned loop is not scheduled within the window), and another spins unboundedly on a counter that a regression could leave at zero (hang instead of clean failure)."
evidence:
  - location: "crates/opi-agent/tests/agent_loop_semantics.rs:1849-1861"
    detail: "sleep(120ms) then cancel, then assert call_count == 1 — under an extreme scheduler stall the cancel can land before the provider dispatch, failing the assertion (other cancellation tests in the same suites use event-driven waits correctly, e.g. retry_agent.rs:561-574)."
  - location: "crates/opi-ai/tests/provider_collection.rs:1758-1761"
    detail: "cancelling_during_auth_resolution_stops_preparation spins while resolve_hits == 0 with no deadline — if a regression made prepare_call fail before consulting the resolver, the test hangs until the cargo timeout instead of failing; the subsequent 1s timeout only bounds the spawned task, not this loop."
criterion_source: ""
reproduction:
  - "read agent_loop_semantics.rs:1820-1888 and provider_collection.rs:1731-1768"
confidence: medium
status: unverified
```

## 6. Invariants Findings

### 6.1 MINOR: Cancellation produces ProviderError::Cancelled on four adapters but a silent stream end on azure/gemini/vertex/bedrock

**File:** `crates/opi-ai/src/gemini.rs`
**Lines:** 720-724 (with azure_openai.rs:284-288, vertex.rs:228-232, bedrock/mod.rs:343-345)
**Claim:** On cancellation, azure/gemini/vertex/bedrock return Ok(()) from their HTTP loop so the attempt stream simply ends with None and no ProviderError::Cancelled, while anthropic/openai_chat/openai_responses/openai_codex_responses return Err(ProviderError::Cancelled); the Request contract documents that a cancelled request produces ProviderError::Cancelled, so direct EventStream consumers (e.g. provider_collection::drain_to_completion) misclassify cancellation of the four quiet adapters as StreamError (stream ended without a terminal event). The production Agent path is unaffected because agent_loop races cancel.cancelled() first with a biased select.
**Adversarial verification:** CONFIRMED -- All eight adapter cancel arms verified. azure_openai.rs (select arm at 286-288), gemini.rs (722-724), vertex.rs (230-232), and bedrock/mod.rs (345) return Ok(()) on cancellation, so stream_prepared's spawn sends nothing and the channel closes: the stream ends with None and no ProviderError::Cancelled. anthropic.rs (989-991), openai_chat.rs (1193-1195), openai_responses.rs (409), and openai_codex_responses.rs (203) return Err(ProviderError::Cancel

```yaml
id: OPAI-ADPT-002
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: invariants
severity: Minor
title: "Cancellation produces ProviderError::Cancelled on four adapters but a silent stream end on azure/gemini/vertex/bedrock"
claim: "On cancellation, azure/gemini/vertex/bedrock return Ok(()) from their HTTP loop so the attempt stream simply ends with None and no ProviderError::Cancelled, while anthropic/openai_chat/openai_responses/openai_codex_responses return Err(ProviderError::Cancelled); the Request contract documents that a cancelled request produces ProviderError::Cancelled, so direct EventStream consumers (e.g. provider_collection::drain_to_completion) misclassify cancellation of the four quiet adapters as StreamError (stream ended without a terminal event). The production Agent path is unaffected because agent_loop races cancel.cancelled() first with a biased select."
evidence:
  - location: "crates/opi-ai/src/azure_openai.rs:284-288"
    detail: "Cancel arm returns Ok(()) — pinned by tests/azure_openai_fixtures.rs:589-621 asserting next().is_none() after cancel; same quiet arm in gemini.rs:720-724, vertex.rs:228-232, bedrock/mod.rs:343-345."
  - location: "crates/opi-ai/src/anthropic.rs:987-991"
    detail: "Contrast: these return Err(ProviderError::Cancelled) at anthropic.rs:987-991, openai_chat.rs:1191-1195, openai_responses.rs:407-409, openai_codex_responses.rs:201-203."
  - location: "crates/opi-ai/src/provider_collection.rs:619-637"
    detail: "drain_to_completion converts a stream that ends without a terminal event into Err(StreamError), so a cancelled azure/gemini/vertex/bedrock stream is reported as a stream/protocol error, not cancellation."
  - location: "crates/opi-agent/src/agent_loop.rs:239-251"
    detail: "Biased select polls cancel.cancelled() before stream.next(), so the Agent always surfaces AgentError::Cancelled regardless of adapter behavior."
criterion_source: "docs/opi-spec.md INV-006 (cancellation MUST be observable); P17-FAL-001/FAL-003; Request contract at provider.rs:115-119"
reproduction:
  - "Build a ResolvedAuth + Request with a cancelled CancellationToken for GeminiProvider::stream_prepared, then drain_to_completion(stream) — returns Err(StreamError) instead of Err(ProviderError::Cancelled)."
confidence: high
status: unverified
```

### 6.2 MINOR: Manual-compaction evidence emission failure is typed as AgentError::EvidenceFinalization, collapsing the emission/finalization failure classes

**File:** `crates/opi-coding-agent/src/harness.rs`
**Lines:** 3256-3259
**Claim:** AgentError has only EvidenceSetup and EvidenceFinalization variants (defensible because mid-run emission failure is intentionally fail-open), but the manual-compaction path maps an EvidenceError::Emission from recorder.emit onto AgentError::EvidenceFinalization, so a caller cannot distinguish an emission failure from a finalization failure on that path and the surfaced diagnostic code is wrong.
**Adversarial verification:** CONFIRMED -- All citations exact. AgentError carries only EvidenceSetup(String) and EvidenceFinalization(String) (loop_types.rs:44-54); no emission variant exists, which the finding itself calls defensible. But emit_manual_compaction_evidence maps recorder.emit failures — which the sink contract types as EvidenceError::Emission (FileEvidenceSink returns Emission on IO write failure, opi-coding-agent/src/evidence.rs:165-185) — onto AgentError::EvidenceFinaliza

```yaml
id: EVD-COMPACT-EMISSION-CLASS
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: invariants
severity: Minor
title: "Manual-compaction evidence emission failure is typed as AgentError::EvidenceFinalization, collapsing the emission/finalization failure classes"
claim: "AgentError has only EvidenceSetup and EvidenceFinalization variants (defensible because mid-run emission failure is intentionally fail-open), but the manual-compaction path maps an EvidenceError::Emission from recorder.emit onto AgentError::EvidenceFinalization, so a caller cannot distinguish an emission failure from a finalization failure on that path and the surfaced diagnostic code is wrong."
evidence:
  - location: "crates/opi-agent/src/loop_types.rs:44-53"
    detail: "AgentError variants: EvidenceSetup(String) and EvidenceFinalization(String); no emission variant exists (mid-run emission failure is by design fail-open via EvidenceHealth::Incomplete)."
  - location: "crates/opi-coding-agent/src/harness.rs:3256-3259"
    detail: "emit_manual_compaction_evidence maps recorder.emit errors to AgentError::EvidenceFinalization — an Emission-phase error (e.g. FileEvidenceSink marks EvidenceError::Emission on IO write failure) crosses as a finalization failure."
  - location: "crates/opi-agent/src/diagnostic.rs:882-889"
    detail: "From<&AgentError> maps EvidenceFinalization to the evidence_finalization_failed diagnostic ('could not be durably finalized') — a wrong diagnostic class for an emission failure."
criterion_source: "P17-FAL-001 (Evidence boundary must expose distinguishable setup/emission/finalization/incomplete classes to its caller)"
reproduction:
  - "Configure capture; call CodingHarness::compact(...) with a sink whose emit fails (e.g. read-only evidence dir); the returned error and diagnostic carry the finalization class for what is an emission-phase failure."
confidence: high
status: unverified
```

### 6.3 MINOR: With capture configured, a run error before the first evidence record (pre-turn cancellation, hook error) is re-typed as AgentError::EvidenceFinalization at the CodingHarness boundary

**File:** `crates/opi-coding-agent/src/harness.rs`
**Lines:** 2518-2527, 2859-2865 (with crates/opi-agent/src/agent_loop.rs:90-102)
**Claim:** When capture is configured and agent.prompt fails before any evidence record is emitted — cancellation observed at turn start or a transform_context/convert_to_llm hook error, both before the first emit_evidence (unlike prepare_call failures, which emit a diagnostic record first) — finalize_evidence_run hits its records.is_empty() guard and the prompt error arm returns Err(AgentError::EvidenceFinalization) with the original error only stringified inside, so the typed AgentError::Cancelled / AgentError::Hook class is lost at the public product boundary; without capture the original typed error is returned unchanged, so the conflation is specific to the capture-configured path.
**Adversarial verification:** CONFIRMED -- Mechanism fully re-derived from code. With capture configured, CodingHarness::prompt calls finalize_evidence_run on the run-error arm (harness.rs:2518-2527); a failure before the first evidence record means recorder.records() is empty, and finalize_evidence_run's records.is_empty() guard returns Err(AgentError::EvidenceFinalization("evidence recorder produced no records")) (harness.rs:2859-2865), which the prompt arm wraps as EvidenceFinalization

```yaml
id: P17PROD-02
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: invariants
severity: Minor
title: "With capture configured, a run error before the first evidence record (pre-turn cancellation, hook error) is re-typed as AgentError::EvidenceFinalization at the CodingHarness boundary"
claim: "When capture is configured and agent.prompt fails before any evidence record is emitted — cancellation observed at turn start or a transform_context/convert_to_llm hook error, both before the first emit_evidence (unlike prepare_call failures, which emit a diagnostic record first) — finalize_evidence_run hits its records.is_empty() guard and the prompt error arm returns Err(AgentError::EvidenceFinalization) with the original error only stringified inside, so the typed AgentError::Cancelled / AgentError::Hook class is lost at the public product boundary; without capture the original typed error is returned unchanged, so the conflation is specific to the capture-configured path."
evidence:
  - location: "crates/opi-coding-agent/src/harness.rs:2859-2865"
    detail: "records.is_empty() returns Err(EvidenceFinalization, 'evidence recorder produced no records') — reached on the run-error path with a clean sink."
  - location: "crates/opi-coding-agent/src/harness.rs:2518-2527"
    detail: "The Err arm converts this to Err(EvidenceFinalization(format!('{finalization}; original run error: {e}'))) instead of returning the original typed error; same pattern at 2556-2563, 2587-2594, 2614-2621 — the original variant is discarded whenever finalization errors."
  - location: "crates/opi-agent/src/agent_loop.rs:90-102 vs 143-158"
    detail: "Cancellation at turn start and hook errors return before the first emit_evidence; only the earlier windows produce zero records."
criterion_source: "P17-FAL-001 (each boundary MUST expose distinguishable typed failure classes); P17-FAL-003"
reproduction:
  - "Configure a CodingHarness with evidence capture, cancel the harness cancellation token, then call prompt(x) — the returned error is AgentError::EvidenceFinalization containing 'original run error: ... cancelled' rather than AgentError::Cancelled."
confidence: high
status: unverified
```

### 6.4 INFO: Loop-side next-turn candidate validation covers only the model-selection route, not the whole candidate

**File:** `crates/opi-agent/src/agent_loop.rs`
**Lines:** 787-798
**Claim:** The only loop-side validation of a prepare_next_turn candidate is collection.validate_dispatchable_route on model_selection; inference (thinking/max_tokens/temperature) and context fields are accepted unvalidated and would surface as a typed provider failure at the next turn's prepare_call instead of InvalidNextTurnCandidate — all current product producers validate before replace_state, so no incorrect behavior exists today, but the loop-boundary enforcement is narrower than the design's stated validate-the-entire-candidate step.
**Adversarial verification:** CONFIRMED -- All citations verified. The only loop-side validation of an accepted prepare_next_turn candidate is collection.validate_dispatchable_route on candidate.model_selection.to_spec() (agent_loop.rs:787-798); inference and context are assigned unvalidated (state = candidate at 798). validate_dispatchable_route (provider_collection.rs:477-486) only resolves the spec and checks a resolver is registered — no capability/wire/request validation; real reques

```yaml
id: INV-NXT002-VALIDATION-BREADTH
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: invariants
severity: Info
title: "Loop-side next-turn candidate validation covers only the model-selection route, not the whole candidate"
claim: "The only loop-side validation of a prepare_next_turn candidate is collection.validate_dispatchable_route on model_selection; inference (thinking/max_tokens/temperature) and context fields are accepted unvalidated and would surface as a typed provider failure at the next turn's prepare_call instead of InvalidNextTurnCandidate — all current product producers validate before replace_state, so no incorrect behavior exists today, but the loop-boundary enforcement is narrower than the design's stated validate-the-entire-candidate step."
evidence:
  - location: "crates/opi-agent/src/agent_loop.rs:787-798"
    detail: "Candidate acceptance runs validate_dispatchable_route on the candidate's model_selection spec and then assigns state; no inference/context validation occurs at this boundary."
  - location: "crates/opi-ai/src/provider_collection.rs:477-486"
    detail: "validate_dispatchable_route only resolves the spec and checks a resolver is registered; it performs no capability/wire/request validation (that happens later in prepare_call with the real request)."
  - location: "crates/opi-agent/src/loop_types.rs:72-76 and crates/opi-agent/tests/hooks_queues.rs:1324-1356"
    detail: "AgentError::InvalidNextTurnCandidate documents the validation scope as model-selection resolution, matching the implemented breadth; product paths validate before replace_state, and tests cover only route-invalid candidates."
criterion_source: "P17-NXT-002 and design 'Fixed ordering' step 'validate the entire candidate' (design lines 349-361)"
reproduction:
  - "n/a (static finding; cited tests runnable per Section 1)"
confidence: high
status: unverified
```

### 6.5 INFO: Post-run compaction evidence emission failure is swallowed; manifest withholding relies only on sink self-report at that point

**File:** `crates/opi-coding-agent/src/harness.rs`
**Lines:** 2711-2714
**Claim:** The harness discards the Result of Agent::emit_compaction_evidence, so a failed post-run compaction-record emission neither advances EvidenceHealth nor reaches finalize logic; manifest withholding at that point depends entirely on the sink's self-reported has_failure, which all three shipped adapters maintain correctly but a contract-violating sink could omit, allowing a Complete manifest for a run with a failed emission.
**Adversarial verification:** CONFIRMED -- Every load-bearing fact verified at HEAD. harness.rs:2712-2714 discards the Result of emit_compaction_evidence with `let _ =`; Agent::emit_compaction_evidence (agent.rs:161-189) returns Result<(), EvidenceError> and calls sink.emit directly without any EvidenceHealth involvement (Agent has no health field; health lives in AgentLoopContext and the loop local). The harness contains zero references to evidence_health, so finalize_evidence_run (2843-

```yaml
id: INV-COMPACTION-EMISSION-SWALLOWED
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: invariants
severity: Info
title: "Post-run compaction evidence emission failure is swallowed; manifest withholding relies only on sink self-report at that point"
claim: "The harness discards the Result of Agent::emit_compaction_evidence, so a failed post-run compaction-record emission neither advances EvidenceHealth nor reaches finalize logic; manifest withholding at that point depends entirely on the sink's self-reported has_failure, which all three shipped adapters maintain correctly but a contract-violating sink could omit, allowing a Complete manifest for a run with a failed emission."
evidence:
  - location: "crates/opi-coding-agent/src/harness.rs:2711-2714"
    detail: "The emission error is dropped with a let-underscore binding and emit_compaction_evidence never touches EvidenceHealth."
  - location: "crates/opi-agent/src/agent.rs:161-189"
    detail: "emit_compaction_evidence returns Result<(), EvidenceError> and emits directly through sink.emit without any health advancement or failure propagation."
  - location: "crates/opi-coding-agent/src/harness.rs:2843-2895"
    detail: "finalize_evidence_run gates solely on recorder.has_failure(); the loop's evidence_health (which does advance on in-loop emit failures at agent_loop.rs:1571-1577) is not consulted here."
criterion_source: "P17-EVD-008 (design: a sink failure cannot be hidden by emitting a normal finalized record through another path)"
reproduction:
  - "Make the file sink's emit fail during a threshold-compaction run (read-only capture dir): the compaction emission error is swallowed and only the sink's own has_failure keeps the manifest withheld."
confidence: high
status: unverified
```

### 6.6 INFO: require_complete does not enforce the session-branch binding listed in P17-EVD-003

**File:** `crates/opi-agent/src/evidence.rs`
**Lines:** 1121-1187 (with harness.rs:1743-1746,2868-2872)
**Claim:** The strict completeness gate checks completeness, binding variant, digests, route, actual-reason, prompt digest, and artifact finalization, but never requires a session-branch reference, so a harness whose SessionCoordinator::new silently failed (error mapped to None via .ok()) can finalize manifests marked complete with session_branch None, weakening the manifest MUST bind session branch requirement to a best-effort fill.
**Adversarial verification:** CONFIRMED -- Every cited fact holds at HEAD eb5e316. require_complete (evidence.rs:1121-1187) checks completeness, DirectRuntimeInput binding, runtime-input/policy/config digests, resolved route, actual-route-reason, prompt digest, and artifact finalization — never session_branch, which stays Option<SessionBranchRef> (evidence.rs:788). Design P17-EVD-003 (design doc line 656) says a finalized manifest MUST bind 'the resolved execution, session branch, exact .

```yaml
id: P17PROD-07
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: invariants
severity: Info
title: "require_complete does not enforce the session-branch binding listed in P17-EVD-003"
claim: "The strict completeness gate checks completeness, binding variant, digests, route, actual-reason, prompt digest, and artifact finalization, but never requires a session-branch reference, so a harness whose SessionCoordinator::new silently failed (error mapped to None via .ok()) can finalize manifests marked complete with session_branch None, weakening the manifest MUST bind session branch requirement to a best-effort fill."
evidence:
  - location: "crates/opi-agent/src/evidence.rs:1121-1187"
    detail: "require_complete performs no session_branch check; the field remains Option<SessionBranchRef> and None passes."
  - location: "crates/opi-coding-agent/src/harness.rs:1743-1746,2868-2872"
    detail: "Session creation failure is swallowed with .ok() (prompt proceeds without persistence) and finalize fills session_branch only when self.session is Some."
criterion_source: "P17-EVD-003 / INV-008"
reproduction:
  - "Make session-dir creation fail (e.g. unwritable OPI_SESSIONS_DIR parent) on a capture-configured harness; prompt succeeds, the manifest finalizes complete with session_branch absent."
confidence: high
status: unverified
```

## 7. Cross-task Integration Findings

### 7.1 MINOR: FileEvidenceSink is permanently unusable after one unfinalized run on a long-lived harness

**File:** `crates/opi-coding-agent/src/evidence.rs`
**Lines:** 135-140, 203-259 (with harness.rs:2853-2858)
**Claim:** After a mid-run emission (or finalization-phase) failure, finalize_evidence_run returns before ever calling EvidenceSink::finalize_run, so FileEvidenceSink keeps its writer and every subsequent setup_evidence_run on the same harness fails with EvidenceError::Setup (previous evidence run has not been finalized), permanently converting every later prompt to AgentError::EvidenceSetup; InMemoryEvidenceSink resets on setup, so the two product-reachable recorders behave asymmetrically and a long-lived interactive/RPC harness using --trace cannot recover from one transient emission failure without reassembly.
**Adversarial verification:** CONFIRMED -- Core wedge confirmed by code. finalize_evidence_run returns early on recorder.has_failure() (harness.rs:2853-2858) — and also on records.is_empty() (2860-2865) and on require_complete failure (2887-2889) — before ever calling finalize_run, so FileEvidenceSink's writer Mutex stays Some (it is only taken inside finalize_run, opi-coding-agent/src/evidence.rs:209-214). Every subsequent setup then hits the guard returning EvidenceError::Setup("previou

```yaml
id: P17PROD-03
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: integration
severity: Minor
title: "FileEvidenceSink is permanently unusable after one unfinalized run on a long-lived harness"
claim: "After a mid-run emission (or finalization-phase) failure, finalize_evidence_run returns before ever calling EvidenceSink::finalize_run, so FileEvidenceSink keeps its writer and every subsequent setup_evidence_run on the same harness fails with EvidenceError::Setup (previous evidence run has not been finalized), permanently converting every later prompt to AgentError::EvidenceSetup; InMemoryEvidenceSink resets on setup, so the two product-reachable recorders behave asymmetrically and a long-lived interactive/RPC harness using --trace cannot recover from one transient emission failure without reassembly."
evidence:
  - location: "crates/opi-coding-agent/src/harness.rs:2853-2858"
    detail: "if recorder.has_failure() return Err(EvidenceFinalization) — finalize_run is never invoked, so the file sink's writer is never taken."
  - location: "crates/opi-coding-agent/src/evidence.rs:135-140"
    detail: "FileEvidenceSink::setup returns EvidenceError::Setup 'previous evidence run has not been finalized' whenever the writer is still Some."
  - location: "crates/opi-coding-agent/src/evidence.rs:1017-1027"
    detail: "InMemoryEvidenceSink::setup clears records/manifest/failure and resets, so only the file adapter wedges."
  - location: "crates/opi-coding-agent/src/runner.rs:299-306"
    detail: "CLI --trace installs the FileEvidenceSink on the potentially long-lived harness, so the wedge is reachable from the product surface."
criterion_source: "P17-EVD-008 (failure remains observable, manifest withheld); P17-MIG-005 mode consistency"
reproduction:
  - "Build a CodingHarness with a FileEvidenceSink recorder whose emit fails once (e.g. read-only capture dir); first prompt returns Err(EvidenceFinalization); every subsequent prompt returns Err(EvidenceSetup) because setup sees the unfinalized writer."
confidence: high
status: unverified
```

### 7.2 MINOR: turn_offset not advanced after a persisted turn when evidence finalization fails

**File:** `crates/opi-coding-agent/src/harness.rs`
**Lines:** 2509-2534, 2843-2895
**Claim:** CodingHarness::prompt (and siblings) persists the completed turn via persist_turn and only then runs finalize_evidence_run; when finalization fails it returns Err without advancing turn_offset. The next prompt's rewind_agent_context(self.turn_offset) then truncates the already-persisted turn out of the live agent context, so the session file and the live model context diverge (the turn reappears only on resume/fork). The C5 rewind comment scopes it to unpersisted failed-turn messages, but this path rewinds a persisted turn.
**Adversarial verification:** CONFIRMED -- Attempted refutation failed on every element. Ordering: prompt() runs persist_turn (harness.rs:2530) before finalize_evidence_run (2531), and turn_offset is updated only afterwards (2533); the `?` on finalize returns Err before the update, leaving turn_offset at the pre-turn value. The same ordering holds in prompt_with_content (2565-2569), retry_last_prompt (2596-2600), and continue_ (2623-2627). Rewind: prompt()/prompt_with_content begin with r

```yaml
id: P17-INT-002
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: integration
severity: Minor
title: "turn_offset not advanced after a persisted turn when evidence finalization fails"
claim: "CodingHarness::prompt (and siblings) persists the completed turn via persist_turn and only then runs finalize_evidence_run; when finalization fails it returns Err without advancing turn_offset. The next prompt's rewind_agent_context(self.turn_offset) then truncates the already-persisted turn out of the live agent context, so the session file and the live model context diverge (the turn reappears only on resume/fork). The C5 rewind comment scopes it to unpersisted failed-turn messages, but this path rewinds a persisted turn."
evidence:
  - location: "crates/opi-coding-agent/src/harness.rs:2529-2534"
    detail: "persist_turn(new, offset) runs before finalize_evidence_run; turn_offset is set only after finalization succeeds."
  - location: "crates/opi-coding-agent/src/harness.rs:2509-2514"
    detail: "prompt() begins with rewind_agent_context(self.turn_offset), discarding the stale-offset (already persisted) turn from the agent state."
  - location: "crates/opi-coding-agent/src/harness.rs:2854-2865"
    detail: "finalize_evidence_run returns Err(EvidenceFinalization) whenever recorder.has_failure(), records are empty, or the strict gate/finalize_run fails — reachable on disk/full-manifest failures after a successful, persisted run."
criterion_source: "P17-A11 / P17-EVD-008 (preserves the actual execution outcome); Phase 17 spec 'Evidence failure'"
reproduction:
  - "Enable --trace capture; make FileEvidenceSink::finalize_run fail (e.g. read-only run dir) after a successful prompt; call prompt() again — the first turn is absent from the second provider Request though present in the session .jsonl."
confidence: high
status: unverified
```

## 8. Residuals Findings

### 8.1 INFO: UserPolicyFacts.capability is never populated and no permission-scope/scoped-grant reference exists in the manifest

**File:** `crates/opi-agent/src/evidence.rs`
**Lines:** 654-660
**Claim:** UserPolicyFacts.capability is set to None at capture construction and no production path ever sets it to Some, and no permission-scope or scoped-grant reference field exists; the spec's capability permission / permission-scope / scoped-grant references bullet is realized digest-only.
**Adversarial verification:** CONFIRMED -- Tried to refute by searching for any production assignment of UserPolicyFacts.capability and by hunting for a permission-scope/scoped-grant field in the manifest contract; both attempts failed, confirming the finding. EvidenceCapture::new constructs UserPolicyFacts { policy_digest, capability: None } (opi-coding-agent/src/evidence.rs:332-335; the cited 321-324 is the same function's parameter list — an immaterial line offset). A workspace-wide gr

```yaml
id: EVD-POLICY-CAPABILITY-UNWIRED
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: residuals
severity: Info
title: "UserPolicyFacts.capability is never populated and no permission-scope/scoped-grant reference exists in the manifest"
claim: "UserPolicyFacts.capability is set to None at capture construction and no production path ever sets it to Some, and no permission-scope or scoped-grant reference field exists; the spec's capability permission / permission-scope / scoped-grant references bullet is realized digest-only."
evidence:
  - location: "crates/opi-coding-agent/src/evidence.rs:321-324"
    detail: "EvidenceCapture::new constructs UserPolicyFacts with capability None; rebind never touches policy; build_finalized_manifest clones capture.policy — capability is always None in production manifests."
  - location: "crates/opi-agent/src/evidence.rs:654-660"
    detail: "UserPolicyFacts has only policy_digest + capability Option; no permission_scope or scoped-grant reference field exists in the manifest contract."
  - location: "crates/opi-agent/src/agent_loop.rs:1101-1110"
    detail: "Per-call policy_ref/permission_ref/permission_scope exist only inside the Allow decision and are reduced to a decision:allow label in the emitted Tool evidence records."
criterion_source: "Design doc line 614-615 (manifest retention bullet); P17-EVD-003 (policy binding via digest is met)"
reproduction:
  - "grep for any assignment of UserPolicyFacts.capability in production source: none sets Some; inspect any production manifest.json: policy.capability == null."
confidence: high
status: unverified
```

### 8.2 INFO: RunId uniqueness is process-local; cross-process manifest correlation relies on directory scoping

**File:** `crates/opi-agent/src/evidence.rs`
**Lines:** 156-158
**Claim:** RunId uniqueness comes from a process-wide AtomicU64 starting at 0, so evidence/manifests produced by different processes (including after a restart) can carry identical run ids; offline cross-run correlation relies on the product file adapter's per-run directory naming rather than the identity itself.
**Adversarial verification:** CONFIRMED -- Verified and could not refute. RUN_ID_COUNTER is a static AtomicU64 starting at 0 (evidence.rs:156-158) and IdentityAllocator::new mints RunId(fetch_add(1)+1) (113-121), so non-reuse holds only within one process; after a restart or in a second process the first run is RunId(1) again. RunId serializes as a transparent bare u64 (evidence.rs:51-53), so two processes' manifests can both report correlation.run == 1 exactly as the reproduction states.

```yaml
id: EVD-RUNID-PROCESS-LOCAL
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: residuals
severity: Info
title: "RunId uniqueness is process-local; cross-process manifest correlation relies on directory scoping"
claim: "RunId uniqueness comes from a process-wide AtomicU64 starting at 0, so evidence/manifests produced by different processes (including after a restart) can carry identical run ids; offline cross-run correlation relies on the product file adapter's per-run directory naming rather than the identity itself."
evidence:
  - location: "crates/opi-agent/src/evidence.rs:156-158"
    detail: "static RUN_ID_COUNTER: AtomicU64 = AtomicU64::new(0) with fetch_add — non-reuse holds only within one process."
  - location: "crates/opi-coding-agent/src/evidence.rs:105-131"
    detail: "Run directories are named run-{timestamp}-{pid}-{sequence}, so on-disk disambiguation is positional, not identity-derived."
criterion_source: "P17-EVD-001 (opaque, stable, non-reused identities) — spec does not explicitly require cross-process uniqueness; recorded as a scale/future consideration"
reproduction:
  - "Run two processes with capture; both manifests can report correlation.run == 1 — distinguishable only via their distinct run directories."
confidence: high
status: unverified
```

### 8.3 INFO: Manual-compaction evidence run finalizes a manifest with fabricated facts: configured-but-undispatched route requested/resolved, a synthetic manual-compaction prompt digest, and fallback auth_source=Static despite zero provider calls

**File:** `crates/opi-coding-agent/src/evidence.rs`
**Lines:** 433-455, 604-638 (with harness.rs:3175-3260)
**Claim:** compact_with_diagnostic runs setup_evidence_run, emit_manual_compaction_evidence, finalize_evidence_run, finalizing a strict manifest for a compaction-only evidence run with zero dispatches: build_finalized_manifest falls back to capture.configured_route to populate route.requested/resolved, the input_identity.prompt_digest is the digest of the synthetic string manual-compaction:<reason>, and extract_provenance_facts falls back to AuthProvenanceSource::Static when no Provider record exists (or the payload lacks auth_source) — so the manifest asserts a Static auth source and a routed run for a compaction command, despite the construction comment claiming provenance is never assumed Static. emit_manual_compaction_evidence also mints a fresh IdentityAllocator and is a second, differently-shaped compaction-record construction site beside Agent::emit_compaction_evidence. This is design-pinned by tests, but the manifest misrepresents the run.
**Adversarial verification:** CONFIRMED -- All cited code verified. compact_with_diagnostic (harness.rs:3160-3228) runs setup_evidence_run (3175), emit_manual_compaction_evidence (3186/3203/3217), and finalize_evidence_run with prompt_text = format!("manual-compaction:{}", reason) (3188-3193, 3205-3210, 3219-3224), so input_identity.prompt_digest is the digest of that synthetic string. The compaction run's record set contains only a Compaction record, so build_finalized_manifest hits the 

```yaml
id: P17PROD-05
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: residuals
severity: Info
title: "Manual-compaction evidence run finalizes a manifest with fabricated facts: configured-but-undispatched route requested/resolved, a synthetic manual-compaction prompt digest, and fallback auth_source=Static despite zero provider calls"
claim: "compact_with_diagnostic runs setup_evidence_run, emit_manual_compaction_evidence, finalize_evidence_run, finalizing a strict manifest for a compaction-only evidence run with zero dispatches: build_finalized_manifest falls back to capture.configured_route to populate route.requested/resolved, the input_identity.prompt_digest is the digest of the synthetic string manual-compaction:<reason>, and extract_provenance_facts falls back to AuthProvenanceSource::Static when no Provider record exists (or the payload lacks auth_source) — so the manifest asserts a Static auth source and a routed run for a compaction command, despite the construction comment claiming provenance is never assumed Static. emit_manual_compaction_evidence also mints a fresh IdentityAllocator and is a second, differently-shaped compaction-record construction site beside Agent::emit_compaction_evidence. This is design-pinned by tests, but the manifest misrepresents the run."
evidence:
  - location: "crates/opi-coding-agent/src/evidence.rs:452-455"
    detail: "Comment claims provenance is never assumed Static."
  - location: "crates/opi-coding-agent/src/evidence.rs:604-638"
    detail: "With no Provider record the function returns auth_source Static; an unrecognized token also maps to Static."
  - location: "crates/opi-coding-agent/src/evidence.rs:433-436"
    detail: "build_finalized_manifest falls back to capture.configured_route when no Provider record exists, populating requested/resolved for a run with zero dispatches."
  - location: "crates/opi-coding-agent/src/harness.rs:3188-3224"
    detail: "finalize_evidence_run called with prompt_text = manual-compaction:<reason> — synthetic prompt identity."
criterion_source: "P17-PRV-005 (auth source facts MUST remain distinguishable/accurate); P17-EVD-004 (requested/resolved/actual distinguishable — resolved populated from configuration here); P17-OUT-004"
reproduction:
  - "Call harness.compact(Manual) on a capture-configured harness and read manifest.json: provenance.auth_source == static and route.requested/resolved carry the configured route although the run contains no provider call; input_identity.prompt_digest is the digest of manual-compaction:<reason>."
confidence: high
status: unverified
```

### 8.4 INFO: Extra dispatch-route construction failures are discarded without a diagnostic

**File:** `crates/opi-coding-agent/src/provider_factory.rs`
**Lines:** 1533--1553
**Claim:** build_extra_dispatch_routes collects extra routes with `if let Ok(route) = route`, discarding every construction failure (bad proxy, malformed profile) without recording a diagnostic, so a configured-but-unbuildable route is silently absent from the collection and only surfaces later as a typed UnknownProvider when selected.
**Adversarial verification:** CONFIRMED -- Lead auditor direct read at HEAD during the product-provider coverage pass; selection of a skipped route fails closed with a typed error, so severity is observability-only.

```yaml
id: AUD-D01-extra-route-skip-silent
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: residuals
severity: Info
title: "Extra dispatch-route construction failures are discarded without a diagnostic"
claim: "build_extra_dispatch_routes collects extra routes with `if let Ok(route) = route`, discarding every construction failure (bad proxy, malformed profile) without recording a diagnostic, so a configured-but-unbuildable route is silently absent from the collection and only surfaces later as a typed UnknownProvider when selected."
evidence:
  - location: "crates/opi-coding-agent/src/provider_factory.rs:1533-1535"
    detail: "built-in extra routes: `if let Ok(route) = route { routes.push(route); }` - the Err side is dropped; the same pattern repeats at 1541-1542 (custom) and 1549-1551 (openai-compatible)."
  - location: "crates/opi-coding-agent/src/provider_factory.rs:1512-1514"
    detail: "The doc comment itself states \"A provider with invalid non-secret CONFIG (bad proxy, malformed profile) is skipped silently.\""
criterion_source: ""
reproduction:
  - "cargo test -p opi-coding-agent --test provider_factory"
confidence: high
status: unverified
```

---

## 9. Invariant Verification

| Invariant | Code evidence | Test coverage |
|-----------|--------------|---------------|
| One immutable route + one prepared auth across all retries (PRV-003) | PreparedProviderCall freezes provider/request/auth (provider_collection.rs:664-672); start_attempt reuses them with an AcqRel single-active guard (756-771) | provider_collection.rs prepare_call_resolves_route_and_auth_once...; phase17_prepare_call.rs:52; evidence_runtime.rs:263 (all green this audit) |
| Cross-provider switch without Agent reconstruction (OUT-001) | Agent owns Arc<ProviderCollection> (agent.rs:62); loop resolves per-turn via prepare_call (agent_loop.rs:112-144) | agent_wrapper.rs:558; phase17_provider_runtime.rs:218/331; phase17_product_evidence.rs:712 (green) |
| Complete-or-unchanged next-turn replacement (OUT-002/NXT-002) | Candidate built from snapshot, route-validated, single `state = candidate` (agent_loop.rs:786-799); error/cancel return prior state (777-806) | hooks_queues.rs phase17_failed_prepare... + phase17_invalid_prepare_candidate...; phase17_failure_rollback.rs:163-176 (green) |
| Stop observes applied state; stop precedes queue polling (NXT-003/004) | ShouldStopAfterTurnContext{state: state.clone()} built after apply; stop returns before drain_queue (agent_loop.rs:809-822) | hooks_queues.rs:1122-1153; agent_loop_semantics.rs:1184-1229; hooks_queues.rs:598-630 (green) |
| Zero Tool::execute without a current Allow (OUT-003/AUT-005) | execute_tool chain resolve->hook->schema->authorize_and_verify->generation recheck->execute (agent_loop.rs:1255-1415); only AllowFresh reaches execute_authorized | tool_authority.rs 6 zero-execution tests; phase17_tool_authority.rs 13 incl. the FailingAuthorizer leg (green) |
| Untrusted content cannot grant or widen authority (AUT-004) | ToolAuthorizationRequest carries trusted fields only; ProductToolAuthorizer never reads arguments for permission (tool_authority.rs:211-262); ToolRegistry has no mutation surface (authority.rs:165-217) | phase17_model_content_cannot_expand_effective_policy; phase17_untrusted_sources_cannot_forge... (green) |
| Excluded tools absent from provider projection (AUT-008) | Request.tools = registry.definitions() recomputed per turn (agent_loop.rs:80+,112-127) | json_mode policy-excluded bash absent; tool_selection excluded-visibility tests (green) |
| Stable ids + monotonic sequence before emission (EVD-001) | IdentityAllocator mints ids immediately before each emission (agent_loop.rs:74-79,154,271-277,1382) | evidence_contract.rs uniqueness/monotonicity; evidence_runtime.rs:109 (green) |
| Incomplete evidence never yields a finalized manifest (EVD-008/A11) | EvidenceRecorder::completed_manifest returns None after any phase failure (evidence.rs:1058-1070); harness withholds on has_failure (harness.rs:2853-2858) | evidence_contract.rs failure-injection trio; phase17_product_evidence.rs:122/203 (green; outcome-retention leg untested -- finding 5.2) |
| Complete-evidence policy fails closed after health degradation (EVD-009/A12) | Loop advances health on emission failure (agent_loop.rs:1392-1398); authorize_and_verify rejects a stale generation (1154-1229); outer post-emission generation recheck (1356-1383) | evidence_runtime.rs:319; phase17_product_evidence.rs:427; phase17_tool_authority.rs in-flight leg (green) |
| Secrets never cross into sink/files/diagnostics/modes (EVD-005/FAL-004/A10) | RedactedValue::redacted is the sole constructor (evidence.rs:798-816); producers redact before the sink; diagnostic SecretRedactor (diagnostic.rs:522-546) | phase17_canaries_stop_before_sink_file_and_manifest; per-mode canary tests (green; the 401/403 body path is untested -- finding 4.1) |
| Unknown measurements never become zero (EVD-004) | Measurement Known/Unknown{reason} closed (evidence.rs:411-472); product maps None->Unknown (evidence.rs:531-548) | measured_zero_is_not_unknown; unknown_measurement_serializes_with_reason_not_zero (green) |
| Direct runs never claim ActiveSnapshot (EVD-003) | RuntimeInputBinding::direct() is the sole direct constructor (evidence.rs:359-401); require_complete rejects variant/digest gaps (1081-1133) | direct_run_never_fabricates_active_snapshot; phase17_complete_run... (green; session-branch gate gap -- finding 6.6) |
| Legacy session/trace bytes never rewritten (MIG-001/004/A13) | All session ops append-only; fork writes a new file; export is read-only (session_coordinator.rs, session_cli.rs); no trace reader exists | phase17_legacy_migration 7/7 incl. byte-digest fixtures (green) |
| Failure precedence chain (FAL-002) | Early returns at each boundary in fixed order (agent_loop.rs:1100-1415, 725-835) | phase17_failure_precedence_stops_before_later_boundaries; per-boundary zero-count tests (green) |
| No dual dispatch/state/authority/evidence paths (RBK-002/MIG-006) | stream_prepared is the sole Provider entry; removed symbols absent from all src (api_audit comment-aware scan) | phase17_removed_interfaces_are_absent_from_production_source (green; stripper blind spot -- finding 5.7) |

## 10. Minimum-change Conformance

All nine tasks carry the four standardized inference notes (reuse_search, placement,
surface_necessity, simplification_ceiling), so every row is in the trace-complete path; the
overlay compared each admitted trace against the implementation at `eb5e316`. No unadmitted
public/config/state/dependency surface, competing duplicate seam, or exceeded ceiling was
found, and substrate work reaches its scenario-owning task through the recorded dependency
closure and production call path in every workstream.

| Task | Scenario/source | Reuse | Placement | Surface | Production slice | Ceiling/trigger | Status |
|------|-----------------|-------|-----------|---------|------------------|-----------------|--------|
| 17.1 | opi-ai prepare_call substrate | existing registry/auth/capability seams reused; no router trait added | core (opi-ai) as admitted | PreparedProviderCall + AuthProvenance as admitted; no config/state/dependency additions | additive at its commit; cutover owned by 17.2/17.5 via collection.prepare_call (agent_loop.rs:133) | ceiling respected; no trigger | `conforming` (linked finding 3.1: seam validation gap) |
| 17.2 | NextTurnState cutover (P17-A04/A05) | Agent state/cancellation/queues/compaction reused | core (opi-agent) as admitted | NextTurnState + one replace_state seam; piecemeal setters removed | Agent::prompt/continue_ persist the final state via run_with_token (agent.rs:406-441) | ceiling respected; no trigger | `conforming` |
| 17.3 | evidence identity/health/lifecycle substrate | redaction helpers + event vocabulary reused | core (opi-agent) as admitted | typed ids/health/EvidenceSink + no-op/in-memory only; file adapter stayed out | consumed by 17.4 -> 17.6 -> 17.7 through the evidence.rs contract | ceiling respected (EVD-010 test-enforced) | `conforming` |
| 17.4 | trusted authorization cutover (P17-A06..A08) | schema validation/deny hooks/built-in tools/adapter permissions reused | mechanism in core + product policy binding, as admitted | RegisteredTool/ToolAuthorizer/decision as admitted | Agent::new registrations+authorizer; agent_loop execute_tool chain | ceiling respected; no trigger | `conforming` |
| 17.5 | product provider assembly (P17-A02) | provider config/credential resolvers/registry UI reused | reference-product as admitted | existing constructors changed to carry the collection; no alias registry added | main.rs -> build_provider_bundle -> build_harness_collection -> harness | ceiling respected; no trigger | `conforming` (linked findings 2.11/2.10) |
| 17.6 | Agent evidence runtime expand | existing lifecycle emission points/retry machinery reused | core (opi-agent) as admitted | EvidenceSink runtime binding + identities as admitted | Agent::prompt/continue_ -> agent_loop emission points | ceiling respected; no product cutover in-task | `conforming` |
| 17.7 | product evidence/file adapter (P17-A01/A03..A12) | explicit capture option/runners/diagnostics/artifact-audit patterns reused | reference-product as admitted | existing capture/runner outputs; legacy core trace exports removed | main.rs/NonInteractiveRunner/RpcRunner -> harness evidence setup/finalize | ceiling respected (one file adapter, no exporter) | `conforming` (linked findings 2/3/5/6/7) |
| 17.8 | legacy session/trace migration (P17-A13) | JSONL repository/branch logic/model parser/trace paths reused | reference-product as admitted | existing resume/fork/session CLI; no reader/shim added | session_cli::handle_session_cli; harness resume/fork/branch; FileEvidenceSink paths | ceiling respected; no trigger | `conforming` |
| 17.9 | cross-mode/failure/rollback/CI assurance (P17-A14/A15) | mode runners/subprocess seams/CI matrix/smoke/docs reused | assurance as admitted | tests + CI definition only; no runtime source modified | all five mode entry points driven by one fixture | ceiling respected; no trigger | `conforming` (linked findings 5.1/3.2/5.6) |

## 11. Residuals and Recommendations

### Priority recommendations

1. **Restore platform evidence currency (Major, 3.2).** Push `211aba8`+`eb5e316` (origin/main
   sits at the fully green `877c41f`, run 31803930575) and re-run the three-platform matrix
   at the final SHA, then update the PLT-001/A15 evidence record. The remediation payload
   touches `agent_loop.rs` (~427 lines), `authority.rs`, `evidence.rs`, and every phase17
   test file, so HEAD currently has no CI coverage on any OS.
2. **Close the prepare_call request-route seam (Major, 3.1).** Validate that the frozen
   `Request` identifies the resolved canonical provider:model and return the spec-required
   typed request-route-mismatch error before freezing; add a two-provider mismatch test.
3. **Reconcile the A09/A11 exit proofs with HEAD (Major, 5.1/5.2).** Either restore a
   one-run four-kind (Provider+Retry+Tool+Compaction) proof through the product harness
   (the in-prompt threshold-compaction path over the persisted allocator) or revise the
   criterion and ledger text to the separate-manual-compaction-run semantics `211aba8`
   implemented; re-add outcome-retention assertions (messages/session content survive an
   evidence failure) to the two `preserves_outcome` tests.
4. **Fix the capture-configured failure-path cluster (7.1, 6.3, 7.2).** Release or reset
   the FileEvidenceSink writer when a run ends unfinalized, stop re-typing zero-record run
   errors as `EvidenceFinalization` (preserve the original typed error), and advance
   `turn_offset` after a successful `persist_turn` even when finalization fails.
5. **Evidence-fact fidelity (3.3, 3.4, 2.11).** Represent the spec's trigger fact or fix the
   `EnvironmentFacts` doc; separate fallback-used from fallback-allowed and retain
   from/to/reason; register truthful Bedrock provenance instead of discarding
   `resolve_credentials`' source, and make the hardcoded `Static` route label typed or
   inert.
6. **Standards hygiene pass (Section 2).** Decide the scope of the pervasive phase-history
   comment cleanup (153 matches, 2.1), remove the dead code flagged in 2.6/2.7/2.8/2.9/2.14,
   and unify the sequential/parallel tool-evidence record shape (2.3).

### Carry-forward observations (no Phase 17 action required)

- The ledger's PRV-005 residual understates production provenance accuracy: every
  non-Bedrock production resolver branch self-reports truthful provenance (environment,
  keychain, OAuth, layered fallback with reason); the hardcoded `Static` route label is
  inert except in the `Baked` branch where it is correct. Bedrock is the one real mislabel
  (2.11).
- RunId uniqueness is process-local (8.2); acceptable while manifests are scoped by per-run
  directories, but worth revisiting before any cross-process correlation exists.
- `NonInteractiveRunner::cancel` is a dead public seam (5.6); wire a signal handler or
  remove it.

