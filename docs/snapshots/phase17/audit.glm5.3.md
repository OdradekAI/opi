# Phase 17 Deep Agent Core Semantic Closure — Independent Code Audit

**Auditor**: glm5.2 (independent, no prior audit reports consulted)
**Date**: 2026-08-14
**Scope**: Phase 17 registered requirements (55 `P17-*` + `A01`–`A15`) and Tasks 17.1–17.9
**Implementation target**: `877c41fd6c7b0c7850839f41c8fd2824e90436a6` (current committed implementation)
**Phase exit commit**: `a4cfa4ddc74b4dfac59b4305d4657599af866480` (provenance only)
**History use**: provenance and discovery only; no diff coverage boundary
**Method**: Pinned-HEAD full-read audit. The auditor read the 14 requirement-dense core files in full (`provider_collection.rs`, `auth.rs`, `provider.rs`, `registry.rs`, `agent.rs`, `agent_loop.rs`, `loop_types.rs`, `hooks.rs`, `evidence.rs` (both crates), `authority.rs`, `compaction.rs`, `tool_authority.rs`, `policy.rs`); eight parallel read-only mapping agents covered every remaining relevant source/test/docs/CI file group with full reads; two fresh independent reviewers ran the Matt Standards and Spec axes. Every Major/Minor finding below was independently re-verified by the auditor against the committed source before inclusion. `python scripts/opi-doc-check.py` was executed (PASS). Worktree contained only two untracked `docs/research/` files (outside audit paths); all evidence came from committed objects.

---

## 1. Executive Summary

**Verdict: PASS-WITH-FINDINGS**

| Severity | Count |
|----------|-------|
| Blocker  | 0     |
| Major    | 3     |
| Minor    | 16    |
| Info     | 4     |

The Phase 17 semantic core is genuinely implemented and strongly tested: collection-owned prepared dispatch with one auth resolution per logical call, atomic validated `NextTurnState` replacement with the exact prepare→apply→stop→queue ordering, a fail-closed trusted authorization chain with zero-execution proofs on every failure class, and a redacted, correlated evidence lifecycle with strict manifest gating. Removed interfaces are verifiably absent from production source.

The three Majors share one root pattern — **evidence facts that are structurally present but never populated on the production path**: (1) auth provenance is hardcoded `Static`/`NotAttempted` for every route despite real keychain/env/OAuth resolution and a real allowed env fallback; (2) the provider-reported actual route is available on every response (`AssistantMessage.provider/model/response_model`) but never captured, so manifests always report `not_reported`; (3) `github-copilot`/`openai-codex` are unconditionally excluded from extra-route registration, contradicting task 17.5's DoD ("each configured and dispatchable route"). None are behavioral or security regressions; all three degrade evidence truthfulness or the advertised cross-provider capability.

### Per-task summary

| Task | Title | Verdict |
|------|-------|---------|
| 17.1 | Add collection-owned route and authentication preparation | PASS |
| 17.2 | Cut over Agent to durable atomic NextTurnState | PASS |
| 17.3 | Define evidence identities, health, and storage-neutral lifecycle | PASS |
| 17.4 | Cut over trusted tool registrations and mandatory authorization | PASS |
| 17.5 | Wire the Reference Product to dispatchable provider routes | PASS-WITH-FINDINGS (3.1, 3.3, 3.8, 3.9) |
| 17.6 | Expand Agent evidence runtime over stable identities | PASS |
| 17.7 | Cut over Reference Product evidence, finalization, and redaction | PASS-WITH-FINDINGS (3.1, 3.2, 3.4, 3.5) |
| 17.8 | Migrate legacy session routes and preserve opaque trace artifacts | PASS (6.1) |
| 17.9 | Close local cross-mode, failure, rollback, documentation, and CI acceptance | PASS-WITH-FINDINGS (2.1, 5.1, 5.2) |

---

## 2. Standards Review (Matt axis)

Independent reviewer output, preserved (lightly compressed):

> **(a) Documented-standard violations.** 1. *Speculative abstraction — auth-provenance vocabulary has no real producer (hard).* CLAUDE.md "Design boundaries": "Do not add … abstraction for hypothetical future use". `AuthProvenanceSource::Environment/CredentialStore/OAuth` (auth.rs:119-135) and `AuthFallback::Used{from,to,reason}` (auth.rs:144-157) are never constructed by any production code: the only non-test `register_route` caller hardcodes `AuthProvenanceSource::Static` (provider_factory.rs:194), and fallback is hardcoded `NotAttempted` (provider_collection.rs:417). The token pipeline (agent_loop.rs:184,1407-1425; coding-agent evidence.rs:482-497) transmits a constant; the passing test feeds a hand-written `"auth_source": "environment"` fixture (phase17_product_evidence.rs:273-282), masking this. 2. *Doc-contract drift (judgement).* `loop_types.rs:177-179`/`agent.rs:147-148` claim "no identities are minted" when the sink is `None`, but `agent_loop.rs:89,154,274` mint turn/call ids unconditionally; `loop_types.rs:171-175`/`agent_loop.rs:1027-1030` describe health advancement in future tense ("arrive in 17.6/17.7") though `agent_loop.rs:1392-1398` implements it now; `harness.rs:2489-2494` states the Agent API "doesn't expose a getter" two lines before calling it. 3. *Speculative future surface* (`SnapshotRef::new`/`ActiveSnapshot`, `RoutePayload` pre-17.7 `Option` fields) — defensible: both variants are spec-mandated (INV-008, design §Resolved-execution manifest); not counted as a violation.
>
> Met standards, verified: thiserror-only in libraries; trait objects only at crate boundaries; dependencies point inward (evidence/authority in opi-agent, product policy in opi-coding-agent); AGENTS.md/CLAUDE.md lockstep.
>
> **(b) Baseline smells.** Duplicated Code — agent_loop.rs:292-465 (sequential vs parallel tool branches; `ToolExecutionEnd`+`ToolResultMessage` block near-verbatim at 338-364 vs 433-458); harness.rs:2231-2333 (four public entries repeat the identical setup→outcome→finalize→persist body); opi-agent evidence.rs:972-994 vs 1058-1071 (byte-for-byte duplicate accessor impls); coding-agent evidence.rs:401-408 vs 456-463 (identical last-Provider-record lookup); harness.rs:1312-1314 re-implements `policy.rs:7-9`; `agent.rs:254-258` (`set_initial_messages`) behaviorally identical to `replace_messages`. Speculative Generality — `AuthorizationDecision::Allow` carries `policy_ref`/`permission_ref`/`permission_scope` (authority.rs:264-271) but the only authorizer fills two of three with the same value (tool_authority.rs:249-250). Repeated Switches/Shotgun Surgery — the built-in tool inventory is re-encoded in four places (policy.rs:19-24, policy.rs:7-9, tool_authority.rs:36-45, harness.rs:1312-1314). Data Clumps — `EffectiveUserPolicy::build` takes 8 params incl. three parallel digest strings. Dead code (mention) — `AgentError::Tool` never constructed in production; `let _ = terminal_msg` (agent_loop.rs:762); `let _ = active_tool_names` (tool_authority.rs:160); `ToolSelection::NoBuiltin` doc "reserved for Phase 4". Primitive Obsession — empty-`RouteSelection` sentinel uses empty strings plus arbitrary `WireApi` filler (coding-agent evidence.rs:521-527).
>
> **(c) Worst Standards issue**: the auth-provenance seam is speculative machinery fed a hardcoded `Static` at its only production wiring point, so manifests systematically misreport credential origin — unused vocabulary plus false evidence in one defect.

Actionable Standards findings follow.

### 2.1 Minor: Published crate READMEs drifted from the Phase 17 API surface, including a non-compiling example

**File:** `crates/opi-ai/README.md`
**Lines:** 317 (example), 60; `crates/opi-agent/README.md` 69, 92, 407-410 (+ ZH counterparts)
**Cause:** The opi-ai Minimal Example still calls `provider.stream(request)`, a method removed by the 17.5 contract step (only `stream_prepared` exists, provider.rs:48) — the published example cannot compile. The opi-agent README describes `Agent` with "tool registration helpers" (removed with `Agent::add_tool`), its loop diagram names `provider.stream(Request)`, and the Public Modules list omits `authority` (lib.rs:7), leaving the 17.4 `RegisteredTool`/`ToolRegistry`/`ToolAuthorizer` surface undocumented. All drift is identical in both language pairs. Task 17.9's DoD requires the bilingual README pairs to "document breaking interfaces".
**Impact:** Embedder-facing documentation understates the breaking API surface and the flagship example fails to compile against the shipped crate.
**Fix:** Update the example to `ProviderCollection::register_route` + `prepare_call` + `start_attempt`; add `authority` to the module list; remove `add_tool`/`provider.stream` references (EN+ZH).

```yaml
id: S1
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: standards
severity: Minor
title: Published READMEs drifted from Phase 17 API incl. non-compiling opi-ai example
claim: The opi-ai README minimal example calls Provider::stream which no longer exists, and the opi-agent README omits the authority module and references removed Agent surface.
evidence:
  - location: crates/opi-ai/README.md:317
    detail: "let mut stream = provider.stream(request);" — Provider::stream removed; only stream_prepared exists (crates/opi-ai/src/provider.rs:48)
  - location: crates/opi-agent/README.md:407-410
    detail: Public Modules list omits `authority` (declared at crates/opi-agent/src/lib.rs:7)
  - location: crates/opi-agent/README.md:69,92
    detail: "tool registration helpers" and `provider.stream(Request)` diagram reference removed surface
criterion_source: CLAUDE.md#Sources of truth (source owns current product facts); task 17.9 DoD (bilingual READMEs document breaking interfaces)
reproduction:
  - sed -n '317p' crates/opi-ai/README.md
confidence: high
status: unverified
```

### 2.2 Minor: Stale in-source doc comments describe removed interfaces and superseded behavior

**File:** `crates/opi-ai/src/auth.rs`
**Lines:** 173-179 (and others below)
**Cause:** The `AuthResolver` trait doc still says each concrete provider "holds an `Arc<dyn AuthResolver>` and calls resolve inside the stream returned by `Provider::stream`" — both claims false since 17.5 (collection-owned resolution; `stream` removed). Same-era docs: `api_mapped.rs:3-9`; `provider_factory.rs:24-26` ("credentials are validated at build time" — stale after lazy per-call auth); the opi-agent `harness.rs` module docs reference the removed `AgentHarness` as an existing orchestration layer in 12 places; `tests/provider_error_classes.rs:9` names `Provider::stream`; `loop_types.rs:171-179`/`agent.rs:147-148` claim no identities are minted with no sink (minting is unconditional; only emission is gated); `loop_types.rs:171-175`/`agent_loop.rs:1027-1030` use future tense for health advancement that shipped; `harness.rs:2489-2494` contradicts itself about a missing getter.
**Impact:** Doc-contract drift misleads maintainers and embedders about the current auth ownership and evidence behavior.
**Fix:** Rewrite the cited doc comments to describe collection-owned preparation, the evidence minting/emission split, and the retained `SessionFacade`.

```yaml
id: S2
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: standards
severity: Minor
title: Stale doc comments describe removed interfaces and superseded runtime behavior
claim: Multiple production doc comments reference Provider::stream, provider-owned AuthResolver state, AgentHarness, and future-tense 17.6/17.7 behavior that contradict the shipped code.
evidence:
  - location: crates/opi-ai/src/auth.rs:176-178
    detail: trait doc claims providers hold AuthResolver and call resolve inside Provider::stream
  - location: crates/opi-agent/src/harness.rs:3,18,21,30,31,54,72,83,214,341,361,600
    detail: twelve doc references to the removed AgentHarness as if extant
  - location: crates/opi-agent/src/loop_types.rs:177-179
    detail: "no identities are minted" vs agent_loop.rs:89,154,274 minting unconditionally
criterion_source: CLAUDE.md#Sources of truth; CLAUDE.md#Working principles (documentation matches code)
reproduction:
  - grep -n "Provider::stream" crates/opi-ai/src/auth.rs
confidence: high
status: unverified
```

### 2.3 Minor: Dead public surfaces survive the AgentHarness removal

**File:** `crates/opi-agent/src/harness.rs`
**Lines:** 65-88, 202-252; `crates/opi-agent/src/state.rs` 3-7; `crates/opi-coding-agent/src/picker.rs` 27-40; `crates/opi-agent/src/tool/result.rs` 126-129; `crates/opi-agent/src/loop_types.rs` 32-33; `crates/opi-agent/src/agent.rs` 254-258
**Cause:** 17.2 removed `AgentHarness`/`HarnessRuntimeConfig` but retained its orphaned machinery as dead public API: `HarnessSnapshot` and `HarnessSession`/`JsonlHarnessSession` have zero consumers; `Phase::Turn/Compaction/BranchSummary` and `HarnessError::Busy` are never constructed; `AgentState` (state.rs) is a vestigial Value bag; `picker::model_picker_items_from_provider` has zero callers (the only provider-bypass seam); `WorkspaceRelation::Unresolved` is documented "Reserved … not populated"; `AgentError::Tool` is never constructed in production; `set_initial_messages` duplicates `replace_messages`.
**Impact:** Published surface area that nothing exercises, in tension with the repo's no-speculative-surface rule and the 17.2 "removed without aliases" intent.
**Fix:** Delete the dead types/variants (or document retention if a registered consumer exists); consolidate `set_initial_messages` into `replace_messages`.

```yaml
id: S3
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: standards
severity: Minor
title: Dead public API retained after AgentHarness removal
claim: HarnessSnapshot, HarnessSession/JsonlHarnessSession, unused Phase variants, HarnessError::Busy, AgentState, picker::model_picker_items_from_provider, and WorkspaceRelation::Unresolved have zero production consumers.
evidence:
  - location: crates/opi-agent/src/harness.rs:202-252
    detail: HarnessSnapshot + HarnessSession/JsonlHarnessSession — zero repo-wide consumers
  - location: crates/opi-coding-agent/src/picker.rs:27-40
    detail: provider-bypass listing helper with zero callers in src/ and tests/
  - location: crates/opi-agent/src/state.rs:3-7
    detail: AgentState serde_json::Value bag, no production consumer
criterion_source: CLAUDE.md#Design boundaries (no abstraction for hypothetical future use)
reproduction:
  - grep -rn "HarnessSnapshot\|JsonlHarnessSession\|model_picker_items_from_provider" crates/*/src crates/*/tests
confidence: high
status: unverified
```

### 2.4 Minor: Duplication clusters, four-fold tool-inventory encoding, and Debug-format policy digest inputs

**File:** `crates/opi-coding-agent/src/harness.rs`
**Lines:** 2231-2333, 1312-1314, 1332-1333; plus citations below
**Cause:** The four public prompt entries repeat the identical setup→evidence→outcome→finalize→persist body; the sequential/parallel tool branches in agent_loop.rs duplicate the result/event/context-push block; opi-agent evidence.rs duplicates its own accessor impls byte-for-byte; the closed built-in tool inventory is encoded in four places (policy.rs:7-9, policy.rs:19-24, tool_authority.rs:36-45, harness.rs:1312-1314); three hand-maintained provider→env tables in provider_factory.rs:1571-1679 must be kept in sync; `EffectiveUserPolicy::build` digests fold `format!("{:?}", trust_decision)`/`installed_packages` Debug renderings into the policy identity (harness.rs:1332-1333), making the digest sensitive to Debug shape rather than a defined canonical form; `crates/opi-ai/Cargo.toml` declares an unused `async-trait` dependency plus `futures-util`/`secrecy` duplicated in dev-deps.
**Impact:** Drift-prone maintenance surface; policy identity stability depends on Debug formatting; spec-mandated tool additions require four synchronized edits.
**Fix:** Extract the shared prompt-entry body and tool-result emission; derive the built-in inventory from one table; hash canonical serializations instead of Debug output; drop the unused dependency.

```yaml
id: S4
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: standards
severity: Minor
title: Duplication clusters, 4x tool-inventory encoding, Debug-format digests, unused dependency
claim: Phase 17 code introduces copy-paste duplication across harness entries and tool branches, re-encodes the built-in tool inventory four times, feeds policy digests from Debug formatting, and retains an unused async-trait dependency.
evidence:
  - location: crates/opi-coding-agent/src/harness.rs:2231-2333
    detail: four public entries repeat the identical run body
  - location: crates/opi-coding-agent/src/harness.rs:1332-1333
    detail: policy input digests computed over format!("{:?}", trust_decision)
  - location: crates/opi-ai/Cargo.toml:19
    detail: async-trait declared but no #[async_trait] usage in the crate
criterion_source: CLAUDE.md#Working principles (simplest approach; no speculative surface)
reproduction:
  - grep -c "async_trait" crates/opi-ai/src -r
confidence: high
status: unverified
```

---

## 3. Spec Review (Matt axis)

Independent reviewer output, preserved (lightly compressed):

> **Findings.** 1. **WRONG — P17-PRV-004/005** (allowed env fallback must retain its typed reason; auth source/fallback distinguishable in evidence): production registration passes `AuthProvenanceSource::Static` for every route (provider_factory.rs:191-194); `CredentialAuthResolver` discards the real `ApiKeySource` (credential_store.rs:1068-1103) and returns `AuthProvenance::default()` (provider_factory.rs:356-362); the collection hardcodes `fallback: NotAttempted` over the resolver value (provider_collection.rs:409-418); `AuthFallback::Used` is constructed nowhere in production. 2. **WRONG — PRV-005 actual route**: `AssistantMessage` carries provider-reported `provider`/`model`/`response_model` (message.rs:28-39), but the Provider evidence record is emitted pre-dispatch with `actual` empty + `not_reported` (agent_loop.rs:172-181) and no path updates it from the response; the manifest's `actual` is always unknown (coding-agent evidence.rs:400-436). 3. **PARTIAL — AUT-003 request facts**: `ToolAuthorizationRequest` has no invocation/session-context field; the loop supplies `run_id: None` and loop-local `"tN"` labels rather than the minted run/turn identities (agent_loop.rs:1041-1044, 88). 4. **PARTIAL — bash adapter binding**: `ProductToolAuthorizer` checks only `LOCAL_ADAPTER_ID` (tool_authority.rs:265-287) though the policy is multi-adapter; arg-driven binding is deferred to `Tool::execute`. 5. **PARTIAL — manifest completeness**: `system_digest: None`, `tool_schema_digests: Vec::new()`, budget/time always `Unknown{NotReported}` (coding-agent evidence.rs:306-318); `require_complete` never checks them. 6. **PARTIAL — terminal outcomes**: only Success/Cancelled/Failed are produced (harness.rs:96-100); `PartialSideEffect`/`CleanupUnknown` have no producer. 7. **PARTIAL — bare-source visibility**: the bare/canonical fact is retained only in the session `model_change` entry; evidence carries no input-source fact. 8. **Minor — FAL-001**: `map_collection_error` collapses all registry errors into one `AgentError::RouteNotDispatchable` with a string detail (agent_loop.rs:872-875). 9. **Note — AUT-008 letter**: `tool_defs` computed once per run (agent_loop.rs:80,116) — registry immutable, substance holds.
>
> **Verified satisfied**: PRV-001/002/003/006; NXT-001..006; AUT-001/002/004/005/006/007/008; EVD-001/002/006/007/008/009/010/011; FAL-002/003/004; MIG-001..006; PLT-002; RBK-002..004. Findings 1 and 2 are the material defects: spec-mandated evidence facts structurally present but operationally never populated.

The auditor independently confirmed findings 1 and 2 in source (see below) and adds finding 3.3 from task-DoD analysis. Actionable Spec findings follow.

### 3.1 Major: Auth provenance and fallback facts are never populated on the production path despite real non-static resolution and a real allowed env fallback

**File:** `crates/opi-coding-agent/src/provider_factory.rs`
**Lines:** 194 (hardcoded source), 356-362 (resolver discards source); `crates/opi-ai/src/provider_collection.rs` 403-418 (route overwrite + hardcoded fallback); `crates/opi-coding-agent/src/credential_store.rs` 1066-1103
**Cause:** `CredentialAuthResolver` resolves via `resolve_api_key`, which classifies the true origin — `ApiKeySource::Store` or `ApiKeySource::Env { env_var, backend_unavailable }` — and then discards it, returning `AuthProvenance::default()` (= Static). `build_harness_collection` registers every route (keychain, env, OAuth-layered, OAuth-store alike) with `AuthProvenanceSource::Static`. `ProviderCollection::prepare_call` then overwrites the resolver's provenance with that route classification and hardcodes `AuthFallback::NotAttempted` (its own comment admits "the hardcode is retained rather than guessed"). The evidence pipeline (`agent_loop.rs:184-185` → coding-agent `evidence.rs:455-497`) faithfully transmits the constant, so every finalized manifest reports `auth_source: static`, `fallback: not_attempted`. Meanwhile the product's keychain→env fallback (credential_store.rs:1085-1088) is a real, allowed environment fallback whose typed reason P17-PRV-004 requires to be retained — `AuthFallback::Used` has no production constructor anywhere.
**Impact:** P17-PRV-004's typed-reason clause is unimplemented on the one real fallback path; P17-PRV-005's auth-source fact is indistinguishable in production evidence (a keychain-credentialed run and a static-key run produce identical provenance), degrading the CTRL-002 offline-verification provenance chain. The passing PRV-005 fixture test feeds a hand-written `"auth_source": "environment"` record, so the test cannot detect this. The ledger disclosed a narrow version of the source half as a PRV-005 residual; the fallback-retention half was not disclosed.
**Fix:** Thread `ApiKeySource` into `ResolvedAuth.provenance` inside `CredentialAuthResolver`; register routes with the resolver's actual classification (or make `prepare_call` prefer resolver-reported provenance over the registration default); construct `AuthFallback::Used{from,to,reason}` on the env-fallback path; add a production-path test asserting a keychain/env-credentialed run's manifest carries the real source.

```yaml
id: P1
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: spec
severity: Major
title: Production auth provenance always Static/NotAttempted; real env fallback reason never retained
claim: No production code path populates AuthProvenanceSource::Environment/CredentialStore/OAuth or AuthFallback::Used, so evidence manifests misreport auth source as static and never retain the typed reason of the real keychain-to-env fallback.
evidence:
  - location: crates/opi-coding-agent/src/provider_factory.rs:194
    detail: every route registered with AuthProvenanceSource::Static regardless of resolver kind
  - location: crates/opi-coding-agent/src/provider_factory.rs:356-362
    detail: CredentialAuthResolver discards resolved.source and returns AuthProvenance::default()
  - location: crates/opi-ai/src/provider_collection.rs:415-418
    detail: prepare_call overwrites provenance and hardcodes AuthFallback::NotAttempted
  - location: crates/opi-coding-agent/src/credential_store.rs:1093-1102
    detail: env fallback classifies ApiKeySource::Env{env_var, backend_unavailable} — computed then dropped upstream
criterion_source: docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md#Per-call route and auth preparation (P17-PRV-004, P17-PRV-005); docs/opi-spec.md CTRL-002
reproduction:
  - grep -rn "AuthFallback::Used" crates/*/src
  - grep -n "AuthProvenanceSource" crates/opi-coding-agent/src/provider_factory.rs
confidence: high
status: unverified
```

### 3.2 Major: Provider-reported actual route is never captured; manifest `actual` is always unknown

**File:** `crates/opi-agent/src/agent_loop.rs`
**Lines:** 146-189 (pre-dispatch record); `crates/opi-ai/src/message.rs` 31-33; `crates/opi-coding-agent/src/evidence.rs` 400-436
**Cause:** The design requires "the provider response's actual provider/model/wire metadata … retained separately from the requested and resolved route; disagreement is visible and cannot be normalized away." Every completed response carries the actual route on `AssistantMessage` (`provider: String`, `model: String`, `response_model: Option<String>`), but the Provider evidence record is emitted pre-dispatch with `actual` as empty strings plus `not_reported` (and `actual.wire` copied from the resolved route — itself inconsistent with the reason), and no code path ever updates the record or the manifest from the response. The manifest therefore reports `actual` as unknown on every run, including runs whose provider reported the model.
**Impact:** The requested/resolved/actual disagreement visibility P17-PRV-005 and scenario A01 require is structurally present but never exercised by production data; manifests assert `not_reported` for facts that were reported — the same false-evidence pattern as 3.1.
**Fix:** After the terminal assistant message, emit (or update) the Provider record's actual route from `AssistantMessage.provider/model/response_model`, leaving `actual_reason: None` when populated; extend the A01 product test to assert a reported actual route round-trips into the manifest.

```yaml
id: P2
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: spec
severity: Major
title: Actual route never captured from provider responses; manifest actual always not_reported
claim: Despite AssistantMessage carrying provider-reported provider/model/response_model on every response, no code path copies it into the Provider evidence record or manifest, so actual route facts are always reported as unknown/not_reported.
evidence:
  - location: crates/opi-ai/src/message.rs:31-33
    detail: AssistantMessage.provider/model/response_model populated by adapters on responses
  - location: crates/opi-agent/src/agent_loop.rs:172-181
    detail: pre-dispatch Provider record emits actual as empty strings with actual_reason not_reported and actual.wire copied from resolved
  - location: crates/opi-coding-agent/src/evidence.rs:417-427
    detail: manifest extraction reads the never-updated record; actual is structurally always unknown
criterion_source: design spec#Per-call route and auth preparation (P17-PRV-005); P17-A01
reproduction:
  - grep -rn "response_model" crates/opi-agent/src crates/opi-coding-agent/src
confidence: high
status: unverified
```

### 3.3 Major: OAuth-only providers are unconditionally excluded from extra-route registration, contradicting the 17.5 DoD

**File:** `crates/opi-coding-agent/src/provider_factory.rs`
**Lines:** 1513-1526 (exclusion + doc comment); `crates/opi-coding-agent/src/harness.rs` 1610-1616 (picker filtered to current provider)
**Cause:** Task 17.5's DoD requires startup to construct "one ProviderCollection containing each configured and dispatchable route". `build_extra_dispatch_routes` skips `github-copilot` and `openai-codex` unconditionally ("their OAuth builder runs only for the active provider"), regardless of whether stored OAuth credentials exist. A user logged into Copilot with another provider active cannot switch (`/model github-copilot:…` fails with an unknown-model diagnostic that misleads — the provider is configured and credentialed). Bedrock shows the conditional pattern is available (it registers only when its credential chain resolves). The exclusion is not registered as an accepted deviation in the ledger. The interactive model picker additionally filters to the current provider only, so registered cross-provider routes are invisible there.
**Impact:** The phase's headline capability (cross-provider switching without Agent reconstruction) is not delivered for two of eleven built-in providers even when they are configured and dispatchable; the failure is typed but the remediation text is wrong.
**Fix:** Register OAuth providers as extra routes when a stored credential probe succeeds (mirroring the bedrock conditional), or register the accepted deviation in the ledger and fix the unknown-model remediation text to name the login requirement.

```yaml
id: P3
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: spec
severity: Major
title: github-copilot/openai-codex excluded from extra routes even when configured and dispatchable
claim: build_extra_dispatch_routes unconditionally skips the two OAuth providers, so a logged-in but inactive copilot/codex cannot be selected cross-provider, contradicting task 17.5's DoD "each configured and dispatchable route" without a registered deviation.
evidence:
  - location: crates/opi-coding-agent/src/provider_factory.rs:1521-1526
    detail: "matches!(*provider_id, \"github-copilot\" | \"openai-codex\") => continue" unconditional on credential presence
  - location: crates/opi-coding-agent/src/provider_factory.rs:1510-1511
    detail: bedrock contrasts: registers only when its credential chain resolves
  - location: crates/opi-coding-agent/src/harness.rs:1610-1616
    detail: model_picker_items filters to the current provider's models only
criterion_source: task 17.5 definition_of_done; design spec#Runtime ownership; P17-OUT-001
reproduction:
  - grep -n "github-copilot" crates/opi-coding-agent/src/provider_factory.rs
confidence: high
status: unverified
```

### 3.4 Minor: Finalized manifest binds the session id, not the active session branch

**File:** `crates/opi-coding-agent/src/harness.rs`
**Lines:** 2551-2554
**Cause:** `SessionBranchRef::new(s.session_id().to_owned())` binds the session, which is invariant across in-session branch switches (`resume_session_branch_tip` changes the active tip without changing the session id). P17-EVD-003 and the design require the manifest to bind the "session branch".
**Impact:** A run executed on a non-default branch produces a manifest indistinguishable from a trunk run at the same session.
**Fix:** Bind the active branch tip entry id (or session id + branch tip) at finalization.

```yaml
id: P4
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: spec
severity: Minor
title: Manifest session_branch is the session id, invariant across branch switches
claim: Finalized manifests bind SessionBranchRef to session_id, which does not distinguish active branches within a session.
evidence:
  - location: crates/opi-coding-agent/src/harness.rs:2551-2554
    detail: session_branch built from s.session_id()
criterion_source: P17-EVD-003; design spec#Resolved-execution manifest
reproduction:
  - grep -n "SessionBranchRef::new" crates/opi-coding-agent/src/harness.rs
confidence: high
status: unverified
```

### 3.5 Minor: Manifest under-binding — system/tool-schema digests absent, budget/time always unknown, partial/cleanup-unknown outcomes never produced

**File:** `crates/opi-coding-agent/src/evidence.rs`
**Lines:** 306-318; `crates/opi-coding-agent/src/harness.rs` 96-100
**Cause:** `build_finalized_manifest` hardcodes `system_digest: None` and `tool_schema_digests: Vec::new()` although the run binding carries a system prompt and registered tool schemas; `budget`/`time` are always `Unknown{NotReported}` though the turn budget (`max_turns`) is known; `require_complete` checks neither. `evidence_outcome` maps only Success/Cancelled/Failed, so `TerminalOutcome::PartialSideEffect`/`CleanupUnknown` have no producer even though the protocol layer types cleanup-unconfirmed outcomes.
**Impact:** The strict manifest finalizes without facts the design's manifest list requires, and partial-effect runs are recorded as plain Failed/Success.
**Fix:** Digest the bound system prompt and registered tool schemas; map budget; translate execution-layer partial/cleanup-unknown outcomes into the corresponding manifest terminal outcomes.

```yaml
id: P5
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: spec
severity: Minor
title: Manifest leaves system/tool-schema/budget facts unbound and never produces partial/cleanup-unknown outcomes
claim: The manifest producer hardcodes system_digest None, empty tool_schema_digests, Unknown budget/time, and maps only three terminal outcomes, so design-required manifest facts are structurally present but never populated.
evidence:
  - location: crates/opi-coding-agent/src/evidence.rs:306-310
    detail: system_digest: None, tool_schema_digests: Vec::new()
  - location: crates/opi-coding-agent/src/harness.rs:96-100
    detail: evidence_outcome maps Ok/Cancelled/other only
criterion_source: design spec#Resolved-execution manifest (P17-EVD-003 list); #Evidence failure (terminal outcomes)
reproduction:
  - grep -n "system_digest" crates/opi-coding-agent/src/evidence.rs
confidence: high
status: unverified
```

### 3.6 Minor: Authorization request lacks session context and carries loop-local rather than evidence identities

**File:** `crates/opi-agent/src/authority.rs`
**Lines:** 237-253; `crates/opi-agent/src/agent_loop.rs` 1041-1044, 88
**Cause:** The design specifies the request carries "run/turn/call identity … current invocation/session context". `ToolAuthorizationRequest` has no invocation/session-context field; the loop passes `run_id: None` and the loop-local `"t{idx}"` label rather than the minted `RunId`/`TurnId` (the `turn_id` string at line 88 is the format label, distinct from `identities.next_turn()`), so authorization decisions never correlate to the evidence call graph that 17.6 wires.
**Impact:** The registered AUT-003 criterion (derivation sources) is satisfied, but the prose request-content contract is partial; authorization records cannot be joined to evidence records.
**Fix:** Pass the minted run/turn identity strings and a session-context field (or document the deliberate decoupling already stated in authority.rs:23-26).

```yaml
id: P6
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: spec
severity: Minor
title: ToolAuthorizationRequest lacks session context and evidence-graph identities
claim: The authorization request carries run_id None and loop-local turn labels instead of the minted evidence identities, and has no invocation/session-context field.
evidence:
  - location: crates/opi-agent/src/agent_loop.rs:1041-1044
    detail: run_id: None, turn_id from the t{idx} label
criterion_source: design spec#Effective product policy (request contents)
reproduction:
  - grep -n "run_id: None" crates/opi-agent/src/agent_loop.rs
confidence: high
status: unverified
```

### 3.7 Minor: bash authorizer binds only the local-adapter permission entry, not the adapter the arguments will reach

**File:** `crates/opi-coding-agent/src/tool_authority.rs`
**Lines:** 265-287 (13-16 documents the deviation)
**Cause:** The design states the product authorizer "reuses the same pure route selection and existing adapter permission policy used by execution, so the permission reference binds the adapter that the validated arguments will reach". The implementation checks only `LOCAL_ADAPTER_ID` and defers arg-driven adapter binding to `Tool::execute`; a call that will route to a non-local adapter is authorized against the local entry. Security is preserved — the execution-layer router gates the reached adapter fail-closed (execution/runtime.rs, no local fallback) — but the design's authorizer-level binding is not implemented, and the deviation is documented only in a code comment, not the ledger.
**Impact:** The `Allow.permission_ref` for `command.execute` does not identify the adapter that will execute; the trusted-authorization record under-describes the binding.
**Fix:** Either perform the pure route selection in the authorizer over the validated arguments (the design permits argument inspection inside the trusted boundary) or register the accepted two-boundary binding as a deviation.

```yaml
id: P7
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: spec
severity: Minor
title: command.execute authorization consults only the local adapter entry
claim: ProductToolAuthorizer decides command.execute solely from the LOCAL_ADAPTER_ID permission entry while actual adapter routing happens later, so the permission reference does not bind the reached adapter.
evidence:
  - location: crates/opi-coding-agent/src/tool_authority.rs:273-275
    detail: decision_for(LOCAL_ADAPTER_ID) only
  - location: crates/opi-coding-agent/src/tool_authority.rs:13-16
    detail: code comment documents the deferral to the routed bash backend
criterion_source: design spec#Effective product policy (bash binding clause)
reproduction:
  - grep -n "LOCAL_ADAPTER_ID" crates/opi-coding-agent/src/tool_authority.rs
confidence: high
status: unverified
```

### 3.8 Minor: azure extra route derives its deployment from the active provider's model spec

**File:** `crates/opi-coding-agent/src/provider_factory.rs`
**Lines:** 1806-1827
**Cause:** When the azure `deployments` list is empty, the single-model catalog and dispatch deployment are derived from `config.defaults.model` — the *active* provider's spec. If azure is registered as an extra route while another provider is active, the registered azure route is keyed to the active provider's model id, producing a registered-but-wrong route that fails visibly at the Azure wire.
**Impact:** Valid configuration (azure section without explicit deployments + non-azure active provider) yields a mis-keyed dispatchable route and a misleading unknown-model/deployment failure.
**Fix:** Derive the azure default deployment from the azure config (or refuse to register the extra route when no deployment can be proven).

```yaml
id: P8
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: spec
severity: Minor
title: azure extra-route deployment keyed to active provider's model when deployments empty
claim: The azure adapter's empty-deployments path reads the deployment from config.defaults.model, so an azure extra route is catalogued and dispatched under another provider's model id.
evidence:
  - location: crates/opi-coding-agent/src/provider_factory.rs:1808-1819
    detail: deployment = config.defaults.model.split_once(':') model half
criterion_source: task 17.5 DoD (route assembly correctness)
reproduction:
  - sed -n '1806,1827p' crates/opi-coding-agent/src/provider_factory.rs
confidence: high
status: unverified
```

### 3.9 Minor: non-active anthropic extra route loses OAuth layering

**File:** `crates/opi-coding-agent/src/provider_factory.rs`
**Lines:** 1619-1622 vs 1274-1289
**Cause:** The active anthropic route uses `AuthSource::Layered` (store-OAuth → `ANTHROPIC_OAUTH_TOKEN` → api-key env); a non-active anthropic extra route gets the plain api-key `CredentialAuthResolver` only. A user with stored Claude OAuth who switches away and back loses OAuth on the return switch (CredentialNeeded or api-key auth instead).
**Impact:** Inconsistent auth capability between the same provider's active and extra route registrations.
**Fix:** Use the layered resolver for anthropic extra routes as well.

```yaml
id: P9
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: spec
severity: Minor
title: anthropic extra route registered with api-key resolver instead of OAuth layering
claim: Extra-route anthropic registrations skip the AuthSource::Layered resolver used when anthropic is active, degrading auth for switch-back users.
evidence:
  - location: crates/opi-coding-agent/src/provider_factory.rs:1619-1622
    detail: extra-route anthropic falls into the plain CredentialAuthResolver branch
criterion_source: task 17.5 DoD; design spec#Runtime ownership
reproduction:
  - sed -n '1614,1625p' crates/opi-coding-agent/src/provider_factory.rs
confidence: high
status: unverified
```

### 3.10 Minor: `--trace` accepted but silently ignored in interactive and binary RPC modes

**File:** `crates/opi-coding-agent/src/cli.rs`
**Lines:** 263-266; `main.rs` run_interactive_core/run_rpc_core (never read `cli.trace`); `rpc.rs:263`
**Cause:** `--trace PATH` parses in every mode; the interactive binary never reads it, and the RPC binary hardwires an always-on in-memory sink, so the flag is silently ignored in 2 of 5 modes. Only the help text discloses "non-interactive / `--json` only"; there is no clap guard or runtime warning. The semantic asymmetry itself is honestly recorded (cross-mode test header, ledger accepted deviations) — the finding is the silently-accepted flag.
**Impact:** A user passing `--trace` in an unsupported mode gets no evidence files and no signal.
**Fix:** Reject `--trace` with a clap conflict in unsupported modes, or emit a one-line warning.

```yaml
id: P10
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: spec
severity: Minor
title: --trace silently ignored in interactive and RPC binary modes
claim: The CLI accepts --trace in all modes but only print/JSON consume it; interactive and RPC binaries ignore it without warning.
evidence:
  - location: crates/opi-coding-agent/src/cli.rs:263-266
    detail: flag declared without mode guards
  - location: crates/opi-coding-agent/src/rpc.rs:263
    detail: RPC hardwires InMemoryEvidenceSink; --trace path unused
criterion_source: P17-MIG-005 (mode consistency); task 17.9 documentation DoD
reproduction:
  - opi --rpc --trace out (no file written, no warning)
confidence: high
status: unverified
```

### 3.11 Minor: registry failures collapse into one AgentError variant at the loop boundary

**File:** `crates/opi-agent/src/agent_loop.rs`
**Lines:** 872-875
**Cause:** `CollectionError::Registry(RegistryError)` — distinguishably typed in opi-ai (`InvalidSpec`/`UnknownProvider`/`UnknownModel`) — maps to a single `AgentError::RouteNotDispatchable { detail: "unknown or ambiguous provider:model selection" }`, erasing the variant distinction for agent-loop callers. Ambiguity remains typed at the product resume boundary (`RouteRemediation::Ambiguous`), where it can actually arise.
**Impact:** P17-FAL-001's "distinguishable typed failure classes" holds for opi-ai callers but not for agent-loop callers on this slice.
**Fix:** Preserve the registry variant (typed sub-variant or distinct detail codes) in the mapping.

```yaml
id: P11
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: spec
severity: Minor
title: Registry error variants collapse into one RouteNotDispatchable at the loop boundary
claim: map_collection_error merges InvalidSpec/UnknownProvider/UnknownModel into one AgentError variant with a string detail, losing typed distinguishability for loop callers.
evidence:
  - location: crates/opi-agent/src/agent_loop.rs:872-875
    detail: CollectionError::Registry(reg) => RouteNotDispatchable { provider: reg.to_string(), detail: "unknown or ambiguous..." }
criterion_source: P17-FAL-001
reproduction:
  - sed -n '858,877p' crates/opi-agent/src/agent_loop.rs
confidence: high
status: unverified
```

---

## 4. Security and Redaction Findings

Canary matrices verified strong across diagnostics/JSON/NDJSON/RPC/evidence/manifest surfaces (prompt/args/env/provider-error vectors, cross-suite). One posture inconsistency:

### 4.1 Minor: Gemini/Vertex/Bedrock stream-error paths carry raw upstream text, unlike the four neutralized wires

**File:** `crates/opi-ai/src/gemini.rs`
**Lines:** 150-153 → 409-417 (also below)
**Cause:** Anthropic/OpenAI-Chat/Responses/Codex substitute bounded neutral constants for upstream error text "because a proxy may echo credential material"; Gemini (and Vertex via the shared mapper) and Bedrock place the raw upstream `message` into `AssistantStreamEvent::Error`, and malformed-frame errors on azure/gemini/vertex interpolate up to 80 chars of raw SSE data. Downstream diagnostics/public events re-scrub via SecretRedactor, but the Error event becomes model-visible assistant content unneutralized on those wires.
**Impact:** P17-FAL-004's posture is inconsistent across provider families; a proxy echoing credential material into a Gemini/Vertex/Bedrock stream error reaches model-visible context unneutralized.
**Fix:** Apply the same neutral-constant substitution (and `safe_excerpt`) on the Gemini/Vertex/Bedrock error and malformed-frame paths.

```yaml
id: SEC1
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: security
severity: Minor
title: Raw upstream error text on Gemini/Vertex/Bedrock stream errors and malformed frames
claim: Three of eight provider wires pass raw upstream error message text (and raw SSE data excerpts) into stream events while the other wires substitute neutral constants.
evidence:
  - location: crates/opi-ai/src/gemini.rs:409-417
    detail: raw upstream error message placed in Error event payload
  - location: crates/opi-ai/src/bedrock/mod.rs:868-876
    detail: same pattern for Bedrock exception frames
  - location: crates/opi-ai/src/gemini.rs:746-749
    detail: malformed-frame StreamError interpolates raw data
criterion_source: P17-FAL-004; P17-A10 (provider-error vector posture)
reproduction:
  - grep -n "UPSTREAM_STREAM_ERROR\|STREAM_ERROR" crates/opi-ai/src/*.rs
confidence: high
status: unverified
```

---

## 5. Test-quality Findings

Overall: Phase 17's behavioral assertions are strong — resolver-call counters, `Tool::execute` atomic counters with positive controls, byte-equality preservation, strict monotonic sequence windows, wiremock-verified header boundaries, and real production entry points (CodingHarness::prompt, NonInteractiveRunner, RpcRunner, real ProductToolAuthorizer/EffectiveUserPolicy). Weak spots are confined to named-criteria tests that verify less than their names claim.

### 5.1 Minor: Tautological or vacuous legs in tests that close named criteria

**File:** `crates/opi-coding-agent/tests/phase17_failure_rollback.rs`
**Lines:** 61-136, 466-499, 513-565; `crates/opi-agent/tests/evidence_contract.rs` 148-159, 311-317, 427; `crates/opi-ai/tests/provider_collection.rs` 1448-1452; `crates/opi-coding-agent/tests/phase17_api_audit.rs` 68-70; `crates/opi-coding-agent/tests/phase17_artifact_truthfulness.rs` 135-155
**Cause:** The FAL-001 test constructs each enum value then `matches!`es it against its own variant (would pass even if production collapsed all failures into one string error — the real classification coverage lives in other suites: provider_error_classes.rs, protocol_conformance.rs). RBK-003's evidence/manifest byte-identity legs assert over files the reloader harness cannot touch (no recorder wired). RBK-004 rebuilds the post-run policy from the same constant inputs (near-tautological). Three vocabulary-level tautologies in evidence_contract.rs; a never-failing `legacy_hits` assertion on a counter nothing can increment; the `strip_comments` safety comment claims over-stripping "can only make the audit fail louder" when the true direction is the opposite (over-stripping removes source text and could hide a retained symbol); artifact-truthfulness writes a "Session persisted: verified" row unconditionally, fabricating a synthetic session file when none was copied.
**Impact:** Named criteria (FAL-001, RBK-003/004, MIG-006 audit soundness) appear closed by tests that verify less than claimed; the covering evidence for FAL-001's substance and RBK-004's digest semantics exists elsewhere, so these are quality gaps, not coverage holes.
**Fix:** Drive FAL-001 through real failure injections; wire a recorder into the RBK-003 reloader or drop those assertion legs; derive RBK-004's post-run policy from the harness; fix the inverted strip_comments claim (under-approximate by stripping only real comments or document the true direction); make the artifact row reflect actual persistence.

```yaml
id: T1
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: test-quality
severity: Minor
title: Tautological/vacuous legs in FAL-001, RBK-003/004, and audit-infrastructure tests
claim: Several named-criterion tests would pass even if the behavior regressed: enum self-matching, assertions over unreachable files, policy rebuilt from the same constants, a never-incrementable counter, an inverted comment-stripping safety claim, and an unconditional verified artifact row.
evidence:
  - location: crates/opi-coding-agent/tests/phase17_failure_rollback.rs:74-77
    detail: constructs AuthNotConfigured then matches AuthNotConfigured
  - location: crates/opi-coding-agent/tests/phase17_failure_rollback.rs:468-480
    detail: reloader wires no evidence recorder; byte assertions over files it cannot touch
  - location: crates/opi-coding-agent/tests/phase17_api_audit.rs:68-70
    detail: strip_comments safety comment states the wrong direction
criterion_source: P17-FAL-001, P17-RBK-003/004, P17-MIG-006 (mechanical verification quality)
reproduction:
  - cargo test -p opi-coding-agent --test phase17_failure_rollback -- phase17_failure_boundaries
confidence: high
status: unverified
```

### 5.2 Minor: declared coverage gaps — authorizer-Err counter, env-fallback positive case, mid-attempt cancellation, EVD-006 walk scope

**File:** `crates/opi-agent/tests/tool_authority.rs`
**Lines:** (absent); plus gaps below
**Cause:** No execution-counter test drives an authorizer returning `Err(AuthorizationError)` (branch code-verified only; the ledger's AUT-005 caveat disclosed this). P17-PRV-004's allowed-env-fallback positive case has no test anywhere (consistent with 3.1 — the path cannot currently be exercised). No collection-level test cancels during an active attempt (loop-level cancellation is covered). The EVD-006 no-evidence test walks only the user-config tree while its module comment claims "no evidence is minted or written anywhere".
**Impact:** Named failure classes rest on code inspection rather than counters; the fallback criterion is unverifiable as shipped.
**Fix:** Add an `Err`-returning authorizer counter test; after 3.1's fix, an env-fallback provenance test; a mid-attempt cancellation test at the collection seam; align the EVD-006 comment with the walked scope.

```yaml
id: T2
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: test-quality
severity: Minor
title: Missing counter tests for authorizer-Err and env-fallback; EVD-006 comment overclaims scope
claim: The authorizer-Err zero-execution branch and the allowed-env-fallback typed-reason path have no behavioral tests, and the default-harness no-evidence test walks a narrower tree than its comment claims.
evidence:
  - location: crates/opi-agent/tests/tool_authority.rs
    detail: no Err(AuthorizationError) counter test in the 6-test matrix
  - location: crates/opi-coding-agent/tests/phase17_product_evidence.rs:1101-1103
    detail: comment claims "anywhere"; walk() covers user.path() only
criterion_source: P17-AUT-005, P17-PRV-004, P17-EVD-006
reproduction:
  - grep -rn "AuthorizationError::Failed" crates/opi-agent/tests crates/opi-coding-agent/tests
confidence: high
status: unverified
```

---

## 6. Cross-task Integration and Residuals Findings

Cross-task expand→contract handoffs verified coherent: 17.1's prepared seam is the sole dispatch path (zero `fn stream(` in opi-ai), 17.2/17.4/17.6 substrate claims match their deferral notes, 17.7's contract step left exactly no-op/in-memory core adapters, and removed interfaces are absent from all production source (comment-stripped scan plus independent grep). No dual dispatch/state/authority/evidence paths exist (RBK-002 supported).

### 6.1 Minor: resume path silently discards session reopen failure

**File:** `crates/opi-coding-agent/src/harness.rs`
**Lines:** 1922-1930 (also 1493, 1509)
**Cause:** `SessionCoordinator::open_existing(...).ok()` on `resume_session_id` (and at construction) discards IO/corruption failures: the harness continues session-less, subsequent turns are not persisted, and no diagnostic is emitted — while the fork path (2106-2116) surfaces the same error.
**Impact:** On a resume IO failure the user silently loses persistence of all new turns; inconsistent with the fork path and INV-007's durability posture.
**Fix:** Surface the reopen error (typed diagnostic at minimum, or fail the resume).

```yaml
id: R1
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: residuals
severity: Minor
title: Session reopen failure silently discarded on resume
claim: resume_session_id and the constructor map open_existing errors to None via .ok(), leaving a session-less harness that never persists new turns and emits no diagnostic.
evidence:
  - location: crates/opi-coding-agent/src/harness.rs:1922-1930
    detail: ".ok()" on open_existing in resume_session_id
criterion_source: docs/opi-spec.md INV-007 (session durability posture)
reproduction:
  - sed -n '1922,1931p' crates/opi-coding-agent/src/harness.rs
confidence: high
status: unverified
```

### 6.2 Info: collection-seam edge cases — auth IO after cancellation, attempt-slot release on dropped streams

`prepare_call` performs credential-store IO even when the request token is already cancelled (rejection surfaces only at `start_attempt`), and `start_attempt`'s active-slot guard releases only when the returned stream is polled to a terminal item — a caller that drops a non-terminal stream without polling leaks the slot for that prepared call (provider_collection.rs:697-704). Neither is reachable as a defect through the agent loop (biased cancel select; streams drained to terminal).

```yaml
id: R2
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: residuals
severity: Info
title: Auth IO on pre-cancelled calls and attempt-slot leak on dropped streams
claim: prepare_call resolves credentials before any cancellation check, and the one-active-attempt guard leaks if an attempt stream is dropped unpollled.
evidence:
  - location: crates/opi-ai/src/provider_collection.rs:697-704
    detail: release closure runs only when the mapped stream is polled to terminal
criterion_source: null
reproduction:
  - sed -n '682,705p' crates/opi-ai/src/provider_collection.rs
confidence: high
status: unverified
```

### 6.3 Info: cross-adapter behavioral divergences in opi-ai (pre-existing)

`Request.timeout` and `Request.extra_headers` are honored by only 4 of 8 adapters (azure/gemini/vertex/bedrock ignore them despite the documented contract, provider.rs:115-124); cancellation inside those four adapters' streams returns silent end-of-stream instead of `Cancelled` (the loop's biased cancel select masks this on the agent path); azure/vertex catalogs hard-code `with_images(true)` capability constants; Anthropic alone injects a default `max_tokens: 8192`; Codex drops HTTP error bodies entirely; six adapters fall back to an empty JSON body on serialization failure (`unwrap_or_default`). None violate the Phase 17 neutral-interface requirement (PRV-006 holds); all predate the phase.

```yaml
id: R3
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: residuals
severity: Info
title: Adapter-level divergences: timeout/extra_headers ignored, cancellation shape, capability constants
claim: Four of eight adapters ignore documented Request fields and return silent stream end on cancellation; capability catalogs on two wires are fabricated constants.
evidence:
  - location: crates/opi-ai/src/gemini.rs:722-725
    detail: cancel arm returns Ok(()) silently
  - location: crates/opi-ai/src/azure_openai.rs:189-198
    detail: with_images(true) constant for every catalog entry
criterion_source: null
reproduction:
  - grep -n "request.timeout" crates/opi-ai/src/*.rs
confidence: high
status: unverified
```

### 6.4 Info: redaction/perf/robustness residuals

`redact()` constructs `SecretRedactor::default()` per call, recompiling ~9 regexes on hot paths (diagnostic.rs:528); the streaming proxy drops events silently on channel overflow with no gap marker (streaming_proxy.rs:252-257) and echoes raw malformed input lines in `proxy_error`; Bedrock's credential chain performs blocking `std::fs`/`std::process` IO (config-supplied `credential_process` execution — AWS-standard semantics) and hand-rolls date arithmetic the `time` dependency already provides; `agent_loop` returns silent `Ok` when a stream ends with no terminal event (agent_loop.rs:725-728, pre-existing); legacy constructors install a dummy `StaticAuthResolver("opi-mock-auth")` when no resolver is supplied (harness.rs:1407-1413) — a fake-credential footgun for SDK embedders skipping the builder seam; RPC reuses the `SessionPersistError` event type for shutdown/JoinError runtime failures (rpc.rs:587-631).

```yaml
id: R4
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.2.md
source_model: glm5.2
independence: fresh-context-same-family
axis: residuals
severity: Info
title: Assorted residuals: per-call regex compilation, proxy event loss, bedrock blocking IO, dummy default resolver
claim: Several pre-existing quality residuals were confirmed in the phase-adjacent code; none are Phase 17 regressions.
evidence:
  - location: crates/opi-agent/src/diagnostic.rs:528
    detail: SecretRedactor::default() rebuilt per redact() call
  - location: crates/opi-coding-agent/src/harness.rs:1407-1413
    detail: dummy StaticAuthResolver installed when auth_resolver is None (not test-gated)
criterion_source: null
reproduction:
  - grep -n "SecretRedactor::default()" crates/opi-agent/src/diagnostic.rs
confidence: high
status: unverified
```

---

## 7. Invariant Verification

| Invariant | Code evidence | Test coverage |
|-----------|--------------|---------------|
| INV-001 runtime provider resolution, cross-provider not rejected | `Agent` holds `Arc<ProviderCollection>` (agent.rs:62); every turn `prepare_call` from applied state (agent_loop.rs:133) | `phase17_next_call_routes_from_applied_state_nxt006`; `phase17_coding_harness_cross_provider_switch_dispatches_both_providers` (bounded by 3.3) |
| INV-002 wire code behind neutral interfaces | Provider trait surface provider.rs:22-77; wire types private in every adapter; no SDK crates (Cargo.toml) | ~340 fixture tests; per_request_auth wiremock header proofs |
| INV-003 exact order of completion/preparation/stop/polling | agent_loop.rs:725-815 (prepare→validate→apply→stop→queues) | `phase8_hook_contract_order`; `phase17_stop_observes_complete_next_turn_state`; `phase8_event_order_terminal_stop_runs_prepare_then_stops` |
| INV-004 full atomic next-request replacement | `NextTurnState` complete value (loop_types.rs:130-153); candidate validated then single assignment (agent_loop.rs:740-754); durable persist on Ok (agent.rs:503-508) | `phase17_agent_persists_complete_next_turn_state`; `phase17_failed_prepare_preserves_state...`; `phase17_invalid_prepare_candidate...` |
| INV-005 authority/capability/scope/schema before side effects; model content grants nothing | execute_tool chain agent_loop.rs:1119-1236; closed decision inputs (authority.rs:236-253); authorizer never reads arguments (tool_authority.rs:232-245) | zero-execution counter matrix (6 substrate + 8 product tests); `phase17_model_content_cannot_expand_effective_policy` |
| INV-006 cancellation/queue/partial failure observable, never silent success | typed `AgentError::Cancelled` on all four cancel sites; bounded proxy channel; typed TerminalOutcome vocabulary | `phase8_cancellation_contract_*`; `phase17_cancellation_and_evidence_failure_are_not_converted_to_success`; (partial/cleanup outcomes unproduced — 3.5) |
| INV-007 session reconstruction/branching/crash recovery preserved | append-only writer, branch tree, recovery reader unchanged; normalization in-memory only (harness.rs:1949-2004) | `phase17_legacy_*` byte-identity suite; session_contract/session_branching |
| INV-008 finalized evidence binds session, runtime-input binding, config, policy | `require_complete` gate evidence.rs:1081-1133; `direct()` never fabricates ActiveSnapshot; DirectRuntimeInput digest at capture construction | `phase17_complete_run_reconstructs_graph_and_rejects_missing_bindings`; `direct_run_never_fabricates_active_snapshot` (branch binding coarse — 3.4) |

---

## 8. Minimum-change Conformance

All nine tasks carry the four standardized notes (`reuse_search`, `placement`, `surface_necessity`, `simplification_ceiling`); no `not-recorded`/`drifted-by-omission` rows.

| Task | Scenario/source | Reuse | Placement | Surface | Production slice | Ceiling/trigger | Status |
|------|-----------------|-------|-----------|---------|------------------|-----------------|--------|
| 17.1 | Dispatchable provider collection (substrate) | Existing registry + per-request auth adapters reused; no second registry | core (opi-ai collection/auth) — verified | `PreparedProviderCall` + `stream_prepared` opaque seam; no router trait | none claimed (substrate_only, `production_call_sites: []` honest) | no product cutover/alias/breadth — held | `conforming` (provenance-overwrite mechanism is 17.5's slice — 3.1) |
| 17.2 | Atomic NextTurnState | Agent state, queues, compaction fixtures reused | core (opi-agent loop) — verified | `NextTurnState` + one `replace_state`; setters narrowed | Agent::prompt/continue_ + agent_loop + CodingHarness — real | AgentHarness/SharedProvider/AgentLoopTurnUpdate removed — verified absent | `conforming` (dead supporting machinery — 2.3) |
| 17.3 | Evidence identity/health/lifecycle contract | Redaction helpers + event vocabulary reused | core (opi-agent) — verified | typed ids, health, binding, sink; no-op/in-memory only (impl-site scan: exactly 2) | none claimed — honest | no file adapter/exporter in core — held | `conforming` |
| 17.4 | Trusted registration + ToolAuthorizer | schema validation, deny hooks, execution counters reused | mechanism in opi-agent, policy in product — verified | RegisteredTool/ToolAuthorizer closed decision; no policy engine | trusted assembly + real ProductToolAuthorizer — real | no allow-all/hook grant/argument mutation — held | `conforming` (request-facts prose partial — 3.6/3.7) |
| 17.5 | Reference Product dispatchable routes | provider config, credential/OAuth resolvers, registry UI reused | reference product — verified | ProviderCollection/NextTurnState carriers; no alias registry | build_provider_bundle + set_model_validated + prompt — real | "each configured and dispatchable route" — NOT held (OAuth exclusion, azure mis-key, anthropic OAuth degradation) | `drifted` → 3.3, 3.8, 3.9, 3.1 |
| 17.6 | Agent evidence runtime | lifecycle emission points, retry machinery reused | core — verified | sink binding on Agent/loop; no product cutover | Agent::prompt/continue_/agent_loop — real | no product consumer/file adapter — held | `conforming` |
| 17.7 | Product evidence cutover | capture option/runners/diagnostics/canary patterns reused | reference product — verified | EvidenceBuilderConfig, file adapter, strict manifest | run_interactive_core/runner/RpcRunner/harness — real | no exporter/dual path — held; PRV-005 evidence facts not populated in production | `drifted` → 3.1, 3.2, 3.5 |
| 17.8 | Legacy session/trace migration | JSONL repo, branch/fork logic, byte fixtures reused | reference product — verified | typed RouteRemediation codes only | session_cli/resume/fork/branch — real | no reader/rewrite/upgrade — held | `conforming` (resume `.ok()` — 6.1) |
| 17.9 | Local acceptance/docs/CI | mode runners, CI matrix, doc contracts reused | assurance — verified | none (tests/CI/docs only) | five mode entries + ci.yml — real | no runtime source modified — held | `conforming` (README drift — 2.1; test-quality — 5.1/5.2) |

---

## 9. Residuals and Recommendations

### Priority recommendations

1. **Populate evidence facts on the production path** (3.1, 3.2): thread `ApiKeySource` and `AuthFallback::Used` through `CredentialAuthResolver`/`prepare_call`, and capture the provider-reported actual route from `AssistantMessage` into the Provider record/manifest. Add production-path tests (a keychain/env-credentialed run must manifest its real source; a reported actual route must round-trip). This is the single change class that closes all three Major-adjacent evidence gaps (with 3.5).
2. **Register or remove the OAuth extra-route exclusion** (3.3): either probe stored credentials and register copilot/codex routes (bedrock-conditional pattern), or record the deviation in the ledger and fix the remediation text.
3. **Close the audit-identified doc drift before the next release** (2.1, 2.2): the opi-ai example is user-facing on crates.io and does not compile.
4. **Strengthen the named-criterion tests** (5.1, 5.2): de-tautologize FAL-001/RBK-003/RBK-004, add the authorizer-Err counter test, correct the `strip_comments` safety claim.
5. **Carry-forward minors** (3.4-3.11, 4.1, 6.1): manifest branch binding, manifest under-binding facts, session-reopen surfacing, Gemini/Vertex/Bedrock error neutralization, azure/anthropic extra-route auth parity.

### Notes

- CI evidence (three-platform green at the final exit SHA 40f2e6e, run 31798070731) was accepted from the ledger's recorded trace; it was not independently re-queried during this audit. The audit HEAD 877c41f contains only post-exit CI-script fixes; no Phase 17 runtime source differs from the exit state.
- The `evaluator_summary` and `audit_notes` in `phase_exit` were used as structural context only; no prior audit report or evaluator transcript was read (contamination isolation maintained; `docs/snapshots/phase17/` contained no audit files before this report).
- Pre-existing (pre-Phase 17) defects found incidentally were reported as Info/Minor with explicit pre-existence noted, per the current-HEAD authority model.
