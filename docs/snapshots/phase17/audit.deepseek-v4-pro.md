# Phase 17 Deep Agent Core Semantic Closure — Independent Code Audit

**Auditor**: deepseek-v4-pro (independent, no prior audit reports consulted)
**Date**: 2026-08-14
**Scope**: Phase 17 registered requirements and Tasks 17.1–17.9
**Implementation target**: `877c41fd6c7b0c7850839f41c8fd2824e90436a6` (current committed implementation)
**Phase exit commit**: `a4cfa4ddc74b4dfac59b4305d4657599af866480` (provenance only)
**History use**: provenance and discovery only; no diff coverage boundary
**Method**: Six independent dimension reviewers (Spec, Standards, Correctness+Invariants, Security/redaction, Test quality, Integration+Residuals) read the complete relevant implementation at `audit_head` in full and produced normalized findings against the 55 `P17-*` criteria + 15 `P17-A*` scenarios. Findings were adjudicated and the two highest-severity claims re-verified directly against source.

---

## 1. Executive Summary

**Verdict: FAIL**

| Severity | Count |
|----------|-------|
| Blocker  | 1     |
| Major    | 2     |
| Minor    | 18    |
| Info     | 19    |

The Phase 17 core mechanisms — atomic `NextTurnState` replacement, trusted tool authorization, fail-closed evidence handling, and the redaction boundary — are substantially well-built and the redaction boundary in particular is clean (no secret crosses `EvidenceSink`). However, the audit found one **Blocker**: non-interactive (`opi "prompt"`) and RPC (`opi --rpc`) startup drop the active provider's per-call auth resolver and silently fall back to a mock credential (`"opi-mock-auth"`), so real provider calls in those two core modes cannot authenticate. Two **Majors** follow: the resolved-execution evidence always reports credential provenance as `Static`/`NotAttempted` regardless of the actual source (OAuth/keychain/env), and the spec-mandated stale-Allow "reauthorize with new health generation" mechanism is a dead facade that reuses the captured health snapshot. Both Majors and the Blocker are invisible to the mock-provider test suite and to CI (which never exercises real credentials).

### Per-task summary

| Task | Title | Verdict |
|------|-------|---------|
| 17.1 | Add collection-owned route and authentication preparation | PASS-WITH-FINDINGS |
| 17.2 | Cut over Agent to durable atomic NextTurnState | PASS |
| 17.3 | Define evidence identities, health, and storage-neutral lifecycle | PASS |
| 17.4 | Cut over trusted tool registrations and mandatory authorization | PASS |
| 17.5 | Wire the Reference Product to dispatchable provider routes | FAIL |
| 17.6 | Expand Agent evidence runtime over stable identities | PASS-WITH-FINDINGS |
| 17.7 | Cut over Reference Product evidence, finalization, and redaction | PASS-WITH-FINDINGS |
| 17.8 | Migrate legacy session routes and preserve opaque trace artifacts | PASS |
| 17.9 | Close local cross-mode, failure, rollback, documentation, and CI acceptance | FAIL |

The Blocker is a cross-mode handoff defect owned by the 17.5 provider-assembly cutover and surfaced by 17.9's cross-mode acceptance (which asserts mode *equivalence* yet uses a mock provider that ignores auth, so it cannot catch it). The `Static`-provenance Major is the 17.1/17.5 contract half of the provenance story. The stale-reauthorization Major sits in the 17.4/17.6/17.7 authority-and-health boundary.

---

## 2. Integration Findings

### 2.1 BLOCKER: Active-provider auth resolver dropped for non-interactive and RPC modes

**File:** `crates/opi-coding-agent/src/main.rs`, `crates/opi-coding-agent/src/runner.rs`, `crates/opi-coding-agent/src/rpc.rs`, `crates/opi-coding-agent/src/harness.rs`
**Lines:** `main.rs:721-745`, `main.rs:867-885`, `main.rs:1052-1086`, `runner.rs:232-244`, `harness.rs:1407-1415`, `provider_factory.rs:1519-1548`
**Cause:** `build_provider_bundle` returns the active provider's real per-call resolver in `ProviderBundle.auth_resolver` (separate from `extra_routes`, which `build_extra_dispatch_routes` builds with `if provider_id == active_provider_id { continue; }`). The interactive path threads it: `main.rs:1214 .auth_resolver(bundle.auth_resolver.clone())`. But `with_provider_bundle` (`main.rs:721-745`) destructures the bundle, passes only `(provider, diagnostics, extra_routes)` to its callback, and then `drop((store, resolver, registry, auth_resolver))`. Both `run_non_interactive_core` (`main.rs:867-885`) and `run_rpc_core` (`main.rs:1052-1086`) build their harness through this path, so `NonInteractiveRunner::build` (`runner.rs:232-238`) and the RPC builder never call `.auth_resolver(...)`. `CodingHarness` then falls back to `StaticAuthResolver::new(AuthScheme::ApiKey, SecretString::from("opi-mock-auth"))` (`harness.rs:1407-1413`).
**Impact:** In non-interactive (`opi "prompt"`) and RPC (`opi --rpc`) modes, `prepare_call` for the active/default provider resolves the literal `"opi-mock-auth"` secret; real provider calls fail authentication against the configured default model. Extra routes carry real lazy resolvers, so a cross-provider `/model` switch works while the first/default dispatch fails — a confusing partial failure. Interactive mode is unaffected. This violates P17-MIG-005 (consistent route semantics across modes) and P17-PRV-001 (route lookup + per-call auth through the collection). Mock-provider tests (cross-mode A14) cannot catch it because the mock ignores resolved auth.
**Fix:** Extend the `with_provider_bundle` callback to also receive `auth_resolver`, and thread it through `NonInteractiveRunner`/`RpcRunner` into `.auth_resolver(...)` mirroring `main.rs:1214`. Add a real-credential (or canary-auth-asserting) non-interactive/RPC test that asserts the active route's resolved auth is not the mock fallback.

```yaml
id: active-auth-resolver-dropped-noninteractive-rpc
source_kind: audit
source_path: docs/snapshots/phase17/audit.deepseek-v4-pro.md
source_model: deepseek-v4-pro
independence: independent-family
axis: integration
severity: Blocker
title: Active provider auth resolver dropped for non-interactive and RPC modes
claim: Non-interactive and RPC startup discard ProviderBundle.auth_resolver, so the active route resolves the mock credential "opi-mock-auth" instead of the real per-call resolver.
evidence:
  - location: crates/opi-coding-agent/src/main.rs:743
    detail: "with_provider_bundle drops (store, resolver, registry, auth_resolver) after invoking the callback with only (provider, diagnostics, extra_routes)"
  - location: crates/opi-coding-agent/src/main.rs:1214
    detail: "interactive path passes .auth_resolver(bundle.auth_resolver.clone()); non-interactive and RPC do not"
  - location: crates/opi-coding-agent/src/runner.rs:238
    detail: "NonInteractiveRunner::build sets .extra_routes(...) but never .auth_resolver(...)"
  - location: crates/opi-coding-agent/src/harness.rs:1407-1413
    detail: "absent auth_resolver falls back to StaticAuthResolver with SecretString 'opi-mock-auth'"
  - location: crates/opi-coding-agent/src/provider_factory.rs:1522-1533
    detail: "build_extra_dispatch_routes skips the active provider, so it is not present in extra_routes"
criterion_source: P17-MIG-005, P17-PRV-001
reproduction: []
confidence: high
status: unverified
```

### 2.2 Minor: Three divergent `provider:model` parsers (trim vs no-trim)

**File:** `crates/opi-agent/src/loop_types.rs:92-103`, `crates/opi-coding-agent/src/provider_factory.rs:375-383`, `crates/opi-ai/src/registry.rs:379-391`
**Cause:** `ModelSelection::parse_spec` trims both halves; `parse_model_spec` and `registry::split_spec` do not. A whitespace-padded spec therefore resolves differently depending on which crate parses it.
**Impact:** Divergent-change risk and a real inconsistency for a padded spec; low practical severity (config validation canonicalizes first).
**Fix:** Single canonical parser in `opi-ai` that all three call.

```yaml
id: divergent-provider-model-parsers
source_kind: audit
source_path: docs/snapshots/phase17/audit.deepseek-v4-pro.md
source_model: deepseek-v4-pro
independence: independent-family
axis: integration
severity: Minor
title: three divergent provider:model parsers
claim: parse_model_spec (coding-agent) and registry::split_spec (ai) do not trim, while ModelSelection::parse_spec (agent) trims.
evidence:
  - location: crates/opi-agent/src/loop_types.rs:92
    detail: "parse_spec trims provider_id and model_id"
  - location: crates/opi-coding-agent/src/provider_factory.rs:375
    detail: "parse_model_spec splits on ':' with no trimming"
  - location: crates/opi-ai/src/registry.rs:379
    detail: "registry::split_spec splits without trimming"
criterion_source: null
reproduction: []
confidence: high
status: unverified
```

### 2.3 Minor: `--trace <PATH>` evidence capture undocumented in both READMEs

**File:** `crates/opi-coding-agent/src/cli.rs:266`, `README.md`, `README.zh.md`
**Cause:** The 17.7 opt-in `--trace` flag (and 17.8 legacy bare-model normalization) are documented only in `CHANGELOG.md`, not in either README. EN/ZH are in lockstep with each other (both omit it).
**Impact:** Users cannot discover the new evidence-capture feature from the primary docs.
**Fix:** Add a `--trace` entry and a legacy-normalization note to both READMEs.

```yaml
id: trace-flag-undocumented-in-readmes
source_kind: audit
source_path: docs/snapshots/phase17/audit.deepseek-v4-pro.md
source_model: deepseek-v4-pro
independence: independent-family
axis: residuals
severity: Minor
title: --trace evidence capture flag absent from both READMEs
claim: The Phase 17.7 --trace flag is documented only in CHANGELOG.md, not README.md/README.zh.md.
evidence:
  - location: crates/opi-coding-agent/src/cli.rs:266
    detail: "pub trace: Option<PathBuf> is the user-facing flag"
  - location: CHANGELOG.md:50
    detail: "--trace evidence capture listed under Added only"
criterion_source: null
reproduction: []
confidence: high
status: unverified
```

### 2.4 Info: `Agent::new` does not validate the initial selection resolves to a route

`crates/opi-agent/src/agent.rs:99-126` parses but does not resolve the initial `model_spec`, while `replace_state`/`validate_state` (`agent.rs:194-212`) do. A parseable-but-unknown model constructs successfully and only fails at first `prepare_call`. Not a correctness loss (dispatch fails closed later); noted as an asymmetric validation boundary.

---

## 3. Spec Findings

### 3.1 MAJOR: Auth provenance hardcoded to `Static`/`NotAttempted` for every production route

**File:** `crates/opi-coding-agent/src/provider_factory.rs:189-198`, `crates/opi-ai/src/provider_collection.rs:403-418`, `crates/opi-coding-agent/src/credential_store.rs:1324-1409`
**Cause:** `build_harness_collection` registers every dispatchable route with a literal `AuthProvenanceSource::Static` (`provider_factory.rs:194`). `prepare_call` then overwrites the resolver's provenance with `AuthProvenance { source, fallback: AuthFallback::NotAttempted }` (the comment admits "the live resolver does not yet surface a typed fallback fact"). Every concrete resolver (`AuthSource::Baked`/`Store`/`EnvOAuthToken`/`Layered`) returns `provenance: Default::default()`, so the real credential source — OS-keychain OAuth, `ENV` OAuth token, static API key, or `Layered` store→OAuth-env→API-key precedence — is never classified. Bedrock additionally discards its real AWS credential source.
**Impact:** The resolved-execution manifest's `auth_source` is always `"static"` and `fallback` always `"not_attempted"`, even for OAuth/keychain/env credentials and even when `AuthSource::Layered` performed an environment fallback. Evidence misrepresents credential provenance, defeating the spec's "closed and typed" source/fallback classification and acceptance scenario P17-A01 ("auth-source and fallback evidence agrees"). The `AuthProvenance`/`AuthFallback` enums are richer than any live path can populate — the non-`Static`/`Used` variants are test-only.
**Fix:** Wire a real resolver path that classifies `environment(name)`/`credential-store(kind)`/`oauth(kind)` and emits `AuthFallback::Used { from, to, reason }` when `Layered` actually falls back; thread the resolver-returned provenance through `prepare_call` instead of overwriting it.

```yaml
id: auth-provenance-hardcoded-static
source_kind: audit
source_path: docs/snapshots/phase17/audit.deepseek-v4-pro.md
source_model: deepseek-v4-pro
independence: independent-family
axis: spec
severity: Major
title: auth provenance source/fallback hardcoded to Static/NotAttempted for all production routes
claim: Every dispatchable route is registered with AuthProvenanceSource::Static and prepare_call overwrites resolver provenance with NotAttempted, so evidence never records the real credential source or fallback.
evidence:
  - location: crates/opi-coding-agent/src/provider_factory.rs:194
    detail: "register_route(..., opi_ai::AuthProvenanceSource::Static, CompatMetadata::default())"
  - location: crates/opi-ai/src/provider_collection.rs:415
    detail: "auth.provenance = AuthProvenance { source, fallback: AuthFallback::NotAttempted }"
  - location: crates/opi-coding-agent/src/credential_store.rs:1324-1409
    detail: "AuthSource::Baked/Store/EnvOAuthToken/Layered all return provenance: Default::default()"
criterion_source: P17-PRV-005, P17-PRV-004, P17-EVD-003
reproduction: []
confidence: high
status: unverified
```

### 3.2 Minor: `ToolAuthorizationRequest` carries `run_id: None` and string turn/call ids

**File:** `crates/opi-agent/src/agent_loop.rs:1041-1049`
**Cause:** `authorize_and_verify` builds the request with `run_id: None`, `turn_id: "t{turn_idx}"`, `call_id` = provider tool-call string, while the loop holds typed `RunId`/`TurnId`/`CallId` (`agent_loop.rs:78-79,154`). `authority.rs:23-26` documents this as an intentional boundary, but the typed ids exist and are not reused.
**Impact:** Authority-boundary correlation is weakened; the authorization record cannot be correlated to the evidence run graph by run id.
**Fix:** Thread the typed `RunId`/`TurnId`/`CallId` into the request (or document why the boundary deliberately degrades to strings).

```yaml
id: authz-request-string-ids
source_kind: audit
source_path: docs/snapshots/phase17/audit.deepseek-v4-pro.md
source_model: deepseek-v4-pro
independence: independent-family
axis: spec
severity: Minor
title: ToolAuthorizationRequest uses run_id None and string turn/call ids instead of typed ids
claim: The authorizer is handed string correlation handles while the loop holds typed RunId/TurnId/CallId.
evidence:
  - location: crates/opi-agent/src/agent_loop.rs:1042-1044
    detail: "run_id: None, turn_id: turn_id.to_owned(), call_id: call_id.to_owned()"
criterion_source: P17-AUT-003, P17-EVD-002
reproduction: []
confidence: high
status: unverified
```

### 3.3 Minor: `Agent::set_model` panics; other piecemeal setters silently discard validation errors

**File:** `crates/opi-agent/src/agent.rs:224-281`
**Cause:** `set_model` uses `.expect("model change must keep a dispatchable route")` after `replace_state`, so an unresolvable selection panics; `set_max_tokens`/`set_thinking_config`/`set_initial_messages`/`inject_message`/`replace_messages`/`rewind_to` do `let _ = self.replace_state(candidate)` and discard the error. `CodingHarness::set_model` (`harness.rs:1619-1622`) exposes this unvalidated.
**Impact:** Latent process abort on invalid model via a public API, and silent no-op on validation failure. Currently unreachable from normal user input (command surfaces use `set_model_validated`), hence Minor rather than higher.
**Fix:** Return the `replace_state` error from the setters rather than panicking or discarding.

```yaml
id: agent-set-model-panics
source_kind: audit
source_path: docs/snapshots/phase17/audit.deepseek-v4-pro.md
source_model: deepseek-v4-pro
independence: independent-family
axis: spec
severity: Minor
title: Agent::set_model panics on unresolvable model; other setters discard validation errors
claim: A public setter panics via expect() and sibling setters silently ignore replace_state errors.
evidence:
  - location: crates/opi-agent/src/agent.rs:234
    detail: "replace_state(candidate).expect('model change must keep a dispatchable route')"
  - location: crates/opi-agent/src/agent.rs:239-281
    detail: "set_max_tokens/set_thinking_config/... do let _ = self.replace_state(candidate)"
criterion_source: P17-NXT-001
reproduction: []
confidence: medium
status: unverified
```

### 3.4 Info: Tool projection computed once per run, not per provider request

`crates/opi-agent/src/agent_loop.rs:80` and `crates/opi-coding-agent/src/harness.rs:1305` compute `tool_defs` once; the `ToolRegistry` is immutable for the run so the result is stable. P17-AUT-008's literal "recomputed for every provider request" is not met, but this is a defensible optimization (see 6.1 for the test gap).

### 3.5 Info: `FileEvidenceSink::setup` truncates evidence on every run

`crates/opi-coding-agent/src/evidence.rs:99-116` opens the records file with `.truncate(true)` on every `prompt`/`continue_` (a fresh `IdentityAllocator` per run), so a multi-prompt session retains only the last run's evidence graph. Spec allows product-owned retention policy; "finalized manifest is immutable" holds in-memory within a run, not across runs on disk.

---

## 4. Correctness and Invariants Findings

### 4.1 MAJOR: Stale-Allow "reauthorize with new health" is a dead facade that reuses the captured snapshot

**File:** `crates/opi-agent/src/agent_loop.rs:1031-1097`
**Cause:** `authorize_and_verify` receives `evidence_health` **by value** (line 1035) and reuses `evidence_health.clone()` for both `attempt in 0..2` iterations (line 1048). `ProductToolAuthorizer` echoes `request.evidence_health.generation()` (`tool_authority.rs:253`), so the freshness check `evidence_health_generation == evidence_health.generation()` (line 1078) always passes for an honest authorizer and the stale branch (line 1086) is only reachable by a "lying" authorizer that returns a generation different from the one it was handed. Tool evidence is emitted **after** execution (lines 466-513), so no production path advances health between `authorize` and the generation check. The spec (lines 509-515) requires "the runtime rebuilds the request with the new health generation and authorizes again" — this rebuild never happens with a genuinely new generation.
**Impact:** No wrong behavior today, because fail-closed is preserved by two other means: (a) each tool authorization is called with a fresh clone of the *live* health (advanced at lines 158/491/605 via `emit_evidence`/`advance_on_failure` at 1397), and (b) `ProductToolAuthorizer` denies `!is_healthy()` under complete-evidence policy (`tool_authority.rs:225-231`). But the designed reauthorization is dead code, and a future reorder (moving evidence emission before execution, or a concurrent emitter) would let a stale `Allow` reach `Tool::execute` without the verify step catching it — a latent fail-open. The comment at `agent_loop.rs:1027-1030` is stale and internally contradictory ("run-start snapshot … advancement arrives in 17.6/17.7" while `emit_evidence` already advances live).
**Fix:** Either re-read live health before the second authorization, or remove the dead reauthorization loop and document that staleness is handled by the fresh-per-tool clone plus the authorizer's own `is_healthy()` gate. Correct the stale comment either way.

```yaml
id: stale-reauth-reuses-captured-health
source_kind: audit
source_path: docs/snapshots/phase17/audit.deepseek-v4-pro.md
source_model: deepseek-v4-pro
independence: independent-family
axis: correctness
severity: Major
title: stale-health reauthorization reuses the same captured health, never re-reads live health
claim: authorize_and_verify reauthorizes with the same by-value EvidenceHealth it captured, so it cannot detect a genuine mid-call health advance.
evidence:
  - location: crates/opi-agent/src/agent_loop.rs:1036
    detail: "evidence_health is a by-value parameter; both loop attempts clone it (line 1048)"
  - location: crates/opi-agent/src/agent_loop.rs:1076-1086
    detail: "generation match compares against the captured clone; second attempt re-authorizes with identical health"
  - location: crates/opi-agent/src/agent_loop.rs:466-513
    detail: "Tool evidence (with decision) is emitted after execution, so health never advances between authorize and verify"
  - location: crates/opi-coding-agent/src/tool_authority.rs:225-231
    detail: "ProductToolAuthorizer denies !is_healthy() under complete-evidence policy — the incidental fail-closed path"
criterion_source: P17-AUT-003, P17-EVD-009
reproduction: []
confidence: medium
status: unverified
```

### 4.2 Minor: `prepare_next_turn` candidate path is never produced by any production hook

`crates/opi-coding-agent/src/harness.rs:3370-3382` (InteractiveCodingHooks), `harness.rs:3358-3364` (CodingAgentHooks), and `runner.rs:1026-1040` (NonInteractiveHooks) all return `Ok(None)` from `prepare_next_turn`; `should_stop_after_turn` returns `false`. Compaction applies post-loop via `replace_messages → replace_state` (`harness.rs:2421`). The atomic in-loop transition (P17-NXT-001/002/003) is therefore unit-tested but not production-driven, while P17-NXT-005's "compaction replaces context wholesale through validation" is still satisfied via the post-loop seam.

```yaml
id: prepare-next-turn-seam-unused-in-production
source_kind: audit
source_path: docs/snapshots/phase17/audit.deepseek-v4-pro.md
source_model: deepseek-v4-pro
independence: independent-family
axis: invariants
severity: Minor
title: prepare_next_turn candidate path is dead outside tests
claim: All three production AgentHooks return Ok(None) from prepare_next_turn, so the atomic in-loop next-turn transition is never exercised in production.
evidence:
  - location: crates/opi-coding-agent/src/runner.rs:1026-1040
    detail: "NonInteractiveHooks::prepare_next_turn returns Ok(None)"
  - location: crates/opi-coding-agent/src/harness.rs:2421
    detail: "compaction applies via replace_messages -> replace_state post-loop"
criterion_source: P17-NXT-001, P17-NXT-005
reproduction: []
confidence: high
status: unverified
```

### 4.3 Minor: Next-turn validation checks registry resolvability but not resolver dispatchability

`crates/opi-agent/src/agent.rs:200-212` (`validate_state`) and `agent_loop.rs:742-752` call `collection.resolve(spec)`, which succeeds for any registered provider+model even when the route has no `AuthResolver`; only `prepare_call` (`provider_collection.rs:396-402`) rejects that as `RouteNotDispatchable`. A candidate selecting a registered-but-non-dispatchable route passes validation and is applied, then fails at the next `prepare_call`. State is not corrupted (the loop errors and `Agent` does not persist), but P17-NXT-002's "validated before apply" is weaker than the actual dispatch gate.

```yaml
id: validate-state-misses-dispatchability
source_kind: audit
source_path: docs/snapshots/phase17/audit.deepseek-v4-pro.md
source_model: deepseek-v4-pro
independence: independent-family
axis: correctness
severity: Minor
title: Next-turn validation misses resolver dispatchability
claim: A registered provider:model without an AuthResolver passes validate_state but fails later at prepare_call.
evidence:
  - location: crates/opi-agent/src/agent.rs:200-212
    detail: "validate_state rejects only when collection.resolve(...) errors"
  - location: crates/opi-ai/src/provider_collection.rs:450-452
    detail: "resolve() delegates to registry.resolve(), which does not check resolvers"
criterion_source: P17-NXT-002, P17-NXT-006
reproduction: []
confidence: high
status: unverified
```

### 4.4–4.8 Info (correctness/invariant observations)

| ID | Finding | Location |
|----|---------|----------|
| 4.4 | Tool authorization outcome emitted after execution, not before (spec's emit-before-execute ordering); no safety hole because the decision is made pre-execution inside `execute_tool` | `agent_loop.rs:466-513` |
| 4.5 | Stale comment claims evidence-failure health advancement "arrives in 17.6/17.7" while `emit_evidence` already advances live | `agent_loop.rs:1027-1030`, `1397` |
| 4.6 | Provider evidence `actual` route copies resolved wire while marking provider/model `not_reported` — mildly conflates resolved and actual facts | `agent_loop.rs:172-180` |
| 4.7 | `emit_compaction_evidence` does not advance `EvidenceHealth` on failure (only `emit_evidence` advances it); manifest still withheld via `has_failure()` | `agent.rs:159-187` |
| 4.8 | Compaction evidence emitted before `execute_compaction` in `persist_turn`, after in `compact_with_diagnostic` — inconsistent ordering | `harness.rs:2410-2421` vs `2826-2836` |

---

## 5. Security / Redaction Findings

The redaction boundary is **solid** — no raw secret crosses `EvidenceSink`, `ResolvedAuth`'s non-secret side carries only variant *names* and closed fallback facts, `PreparedProviderCall` redacts its `Debug`, `AuthorizationDecision::Deny` embeds no raw args, and provider error bodies are scrubbed via `safe_excerpt` (`http.rs:291-308`). No Blocker/Major/Minor. Three Info observations:

| ID | Finding | Location |
|----|---------|----------|
| 5.1 | Tool-result content is intentionally unredacted at the public event boundary; RPC gets a whole-event `SecretRedactor` pass but JSON/NDJSON does not, so a secret in tool-output *text* is scrubbed in RPC but emitted verbatim in `--json`/NDJSON. Cross-mode consistency gap under P17-FAL-004. | `event.rs:163,303`; `runner.rs:316-355`; `streaming_proxy.rs:279-283` |
| 5.2 | `Diagnostic::fmt::Display` prints `message`/`action` without redaction; secret-bearing data is confined to `details` (which `Display` omits), so no current leak, but a footgun if a dynamic message becomes secret-bearing. | `diagnostic.rs:166-181` |
| 5.3 | Legacy `SecretKey` wraps a plain `String` (no zeroize, `as_str()` exposes raw) and is dead in production (`AuthDescriptor::StaticApiKey` never constructed). | `provider_collection.rs:54-84` |

---

## 6. Test Quality Findings

The Phase 17 tests are largely strong: they drive real production entry points (cross-provider switch, fail-closed execution counters, byte-identical legacy fixtures, in-flight-retains-actual-outcome) rather than degenerate seams. The gaps:

### 6.1 Minor: P17-AUT-008 tool-projection has no snapshot test and is computed once

`register_builtin_tools` (`tool_authority.rs:51-67`) correctly drops all non-builtin (extension/embedder) tools at registration — so "a tool excluded by policy does not appear in the projection" is implemented at registration time, not per request. But no test captures `Request.tools` across consecutive requests (the spec's required "consecutive-request tool-projection snapshot tests"), and the projection is computed once (`harness.rs:1305`) rather than per request. The earlier "unimplemented" hypothesis is refuted; the residual is a missing spec-mandated test plus a defensible once-per-run computation.

```yaml
id: aut-008-projection-snapshot-test-missing
source_kind: audit
source_path: docs/snapshots/phase17/audit.deepseek-v4-pro.md
source_model: deepseek-v4-pro
independence: independent-family
axis: test-quality
severity: Minor
title: P17-AUT-008 tool projection has no consecutive-request snapshot test
claim: No Phase 17 test captures Request.tools across consecutive provider requests, and projection is computed once rather than per request.
evidence:
  - location: crates/opi-coding-agent/src/harness.rs:1305
    detail: "tool_defs = registrations.iter().map(|r| r.definition.clone()) — computed once"
  - location: crates/opi-agent/src/agent_loop.rs:80
    detail: "let tool_defs = context.registry.definitions() — before the turn loop"
criterion_source: P17-AUT-008
reproduction: []
confidence: high
status: unverified
```

### 6.2 Minor: P17-A10 / EVD-005 canary coverage is prompt-channel only

`phase17_canaries_stop_before_sink_file_and_manifest` (`phase17_product_evidence.rs:1154`) plants the canary only in the prompt. Arguments, environment, provider-error, diagnostics, and artifact-metadata channels (all named by the A10 scenario) are untested. The producer-boundary `RedactedValue` scrub is unit-tested (`evidence_contract.rs:435/452`), which is the only place redaction is forced to run.

```yaml
id: evd-005-canary-channels-incomplete
source_kind: audit
source_path: docs/snapshots/phase17/audit.deepseek-v4-pro.md
source_model: deepseek-v4-pro
independence: independent-family
axis: test-quality
severity: Minor
title: P17-A10 canary test covers only the prompt channel
claim: The canary is planted only in the prompt; args/env/provider-error/diagnostics/artifact-metadata channels are unexercised.
evidence:
  - location: crates/opi-coding-agent/tests/phase17_product_evidence.rs:1163
    detail: "canary injected only into prompt text"
criterion_source: P17-A10
reproduction: []
confidence: high
status: unverified
```

### 6.3 Minor: P17-FAL-001 test is a construct-then-match tautology

`phase17_failure_boundaries_expose_distinguishable_typed_classes` (`phase17_failure_rollback.rs:61-136`) constructs each error enum variant and asserts it `matches!` its own variant — proving only that the variants exist. It does not drive any production boundary to show the boundaries return these classes (real coverage lives in `provider_collection.rs`/`prepare_call`/`evidence_contract.rs`).

```yaml
id: fal-001-typed-classes-tautology
source_kind: audit
source_path: docs/snapshots/phase17/audit.deepseek-v4-pro.md
source_model: deepseek-v4-pro
independence: independent-family
axis: test-quality
severity: Minor
title: phase17_failure_boundaries test matches each variant against itself
claim: Every assertion constructs an error variant and matches it against its own variant, proving no production behavior.
evidence:
  - location: crates/opi-coding-agent/tests/phase17_failure_rollback.rs:62
    detail: "constructs CollectionError::AuthNotConfigured then asserts matches!(...AuthNotConfigured)"
criterion_source: P17-FAL-001
reproduction: []
confidence: high
status: unverified
```

### 6.4 Minor: P17-A05 cancellation leg untested

The prepare-failure and validation-failure legs of A05 are covered (`hooks_queues.rs:1243,1324`), but no test cancels a `prepare_next_turn` in-flight and asserts context/model/inference unchanged.

```yaml
id: a05-prepare-cancel-leg-missing
source_kind: audit
source_path: docs/snapshots/phase17/audit.deepseek-v4-pro.md
source_model: deepseek-v4-pro
independence: independent-family
axis: test-quality
severity: Minor
title: P17-A05 covers prepare-failure but not prepare-cancellation
claim: No test cancels a prepare_next_turn transition and asserts prior-state preservation.
evidence:
  - location: crates/opi-agent/tests/hooks_queues.rs:1243
    detail: "failure and validation-failure legs covered; no cancellation leg"
criterion_source: P17-A05
reproduction: []
confidence: medium
status: unverified
```

### 6.5 Minor: P17-PLT-002 hermetic test is a token scan

`phase17_tests_are_hermetic_no_network_no_paid_providers` (`phase17_api_audit.rs:279-308`) greps source for `http://`/`https://`; it cannot catch a programmatically-built URL or an IP literal. Low risk (tests use `MockProvider`), but PLT-002 is only weakly enforced.

```yaml
id: plt-002-token-scan
source_kind: audit
source_path: docs/snapshots/phase17/audit.deepseek-v4-pro.md
source_model: deepseek-v4-pro
independence: independent-family
axis: test-quality
severity: Minor
title: hermetic no-network test greps for http:// literals only
claim: The PLT-002 test cannot detect network access without a URL literal.
evidence:
  - location: crates/opi-coding-agent/tests/phase17_api_audit.rs:298
    detail: "let scheme = 'ht' + 'tp://'; asserts !raw.contains(&scheme)"
criterion_source: P17-PLT-002
reproduction: []
confidence: medium
status: unverified
```

### 6.6–6.8 Info

| ID | Finding | Location |
|----|---------|----------|
| 6.6 | fn-local `SESSION_TEST_LOCK` statics are distinct locks per declaration site (safe today, fragile if a second env-mutating test is added to the same binary) | `phase17_artifact_truthfulness.rs:38`, `phase17_provider_runtime.rs:337` |
| 6.7 | `crates/opi-agent/tests/trace_envelope.rs` is listed in task verification but does not exist at HEAD | `crates/opi-agent/tests/` |
| 6.8 | A few listed tests assert nothing or only a length bound (`provider_error_has_timeout_variant`, `compact_summary_contains_reasonable_text`); pre-Phase-17 legacy, out of scope | `provider_trait.rs:46`, `compaction.rs:223` |

---

## 7. Standards Findings

| ID | Sev | Finding | Location |
|----|-----|---------|----------|
| 7.1 | Minor | `harness.rs` module/type docs still describe the removed `AgentHarness` struct in present tense (module doc, `Phase`, `HarnessError`, `SessionFacade` at lines 3/18/21/30/54/72/214/341/600); README correctly records the removal but the source docs were not updated | `crates/opi-agent/src/harness.rs` |
| 7.2 | Minor | Stale `Provider::stream` references after the rename to `stream_prepared` (dangling rustdoc link + README snippets that no longer compile) | `auth.rs:176`, `provider_factory.rs:1971`, `opi-ai/README.md:317`, `opi-agent/README.md:91` |
| 7.3 | Minor | Two `AuthProvenanceSource` enums (`opi-ai` `OAuth{kind}` vs `opi-agent` `Oauth`) bridged by untyped string tokens with silent `_ => "static"` fallthrough; the `Oauth` spelling is a naming inconsistency against the `OAuth` domain term | `auth.rs:114-158`, `evidence.rs:623-632`, `agent_loop.rs:1407-1425` |
| 7.4 | Minor | Loop-local `Authorized::Deny` is a field-for-field copy of `AuthorizationDecision::Deny` | `agent_loop.rs:1009-1019` vs `authority.rs:258-286` |
| 7.5 | Minor | `ProviderCollection` holds five parallel keyed maps populated by two disjoint constructors (`register` vs `register_route`) — a half-populated entry is representable | `provider_collection.rs:282-373` |
| 7.6 | Minor | Repeated stringly-typed tool-name switches (`is_mutating_tool` in `policy.rs` vs `builtin_capability` in `tool_authority.rs`) that can silently diverge | `policy.rs:7-24`, `tool_authority.rs:36-45` |
| 7.7 | Minor | Vestigial `AgentState` (`Vec<serde_json::Value>`) still exported and used only in a Send/Sync bounds assertion, superseded by typed `NextTurnState` | `state.rs:3-7`, `lib.rs:43`, `tests/transport.rs:39` |
| 7.8 | Info | Reserved/forward-looking evidence vocabulary (`ActiveSnapshot`, `ArtifactReference` family, `MeasurementOrigin::{Estimated,Quota,Billed}`, `TerminalOutcome::{PartialSideEffect,CleanupUnknown}`) with no current producer. **Not speculative generality** — these variants are normative-spec-mandated (P17-EVD-003/004, INV-008, P17-FAL-003) as distinguishable-but-deferred surface | `evidence.rs:346-520` |
| 7.9 | Info | Evidence field named `decision` (not `authorization`) to avoid the `SecretRedactor`'s substring heuristic — brittle coupling of a semantic name to a redactor | `agent_loop.rs:504-507` |
| 7.10 | Info | `empty_selection()` fabricates `WireApi::OpenAiCompletions` as an "unknown" sentinel for a non-optional field | `evidence.rs:521-527` |
| 7.11 | Info | `digest_bytes` is a trivial one-line middle-man wrapper over `ContentDigest::as_hex()` | `evidence.rs:263-265` |

---

## 8. Invariant Verification

| Invariant | Code evidence | Test coverage |
|-----------|--------------|---------------|
| P17-NXT-001 complete replacement persists | `Agent::run_with_token` persists loop final state (`agent.rs:503-509`); `replace_state` for idle changes | partial (unit) |
| P17-NXT-002 validate before atomic apply | `prepare_next_turn` + inline resolve check; error returns without persisting | yes (`hooks_queues.rs`, `agent_loop_semantics.rs`) |
| P17-NXT-003/004 stop after apply, no poll on stop | `agent_loop.rs:766-773` then return before queue drain | yes (`agent_loop_semantics.rs:1185`) |
| P17-NXT-005 compaction complete replacement | `replace_messages` → `replace_state` (post-loop seam) | partial |
| P17-AUT-003/005 zero execution on denial/missing/stale | `execute_tool` chain + `authorize_and_verify` | yes (missing/deny/stale-synthetic) |
| P17-EVD-009 stale reauthorization with new health | **not as specced** — reuses captured health (finding 4.1) | synthetic only |
| P17-EVD-001 typed identities + monotonic sequence | `IdentityAllocator` | yes (`evidence_contract.rs`) |
| P17-EVD-003 direct run not ActiveSnapshot | `RuntimeInputBinding::direct` + `require_complete` | yes |
| P17-EVD-005 redaction before sink | `RedactedValue` sole constructor applies `crate::redact` | yes (producer-boundary unit) |
| P17-PRV-003 retry reuses frozen route, one resolver call | `PreparedProviderCall::start_attempt` | yes (`phase17_prepare_call.rs`) |
| P17-PRV-005 auth source/fallback distinguishable | **not met** — hardcoded `Static`/`NotAttempted` (finding 3.1) | no (tests construct the variants directly) |

---

## 9. Minimum-change Conformance

| Task | Scenario/source | Reuse | Placement | Surface | Production slice | Ceiling/trigger | Status |
|------|-----------------|-------|-----------|---------|------------------|-----------------|--------|
| 17.1 | provider route/auth substrate | reuses existing registry/auth adapters | `opi-ai` core (correct) | `PreparedProviderCall` + `AuthProvenance` admitted | prepare_call/start_attempt real | provenance variants unproduced (3.1) | `drifted` |
| 17.2 | atomic NextTurnState | reuses state/queues/compaction | `opi-agent` core (correct) | `NextTurnState`/`ModelSelection` admitted | agent_loop replacement real | vestigial `AgentState` + stale docs (7.1/7.7) | `conforming` |
| 17.3 | evidence identity/lifecycle | reuses redaction helpers + event vocab | `opi-agent` core (correct) | `EvidenceSink`/`EvidenceHealth` admitted | no-op/in-memory real | reserved vocab spec-mandated (7.8) | `conforming` |
| 17.4 | trusted tool authorization | reuses schema validation + permission broker | core mechanism + product policy (correct) | `RegisteredTool`/`ToolAuthorizer` admitted | execute_tool chain real | extension tools fail-closed (correct); `run_id` None (3.2) | `conforming` |
| 17.5 | dispatchable provider routes | reuses provider config/credential resolvers | product (correct) | canonical route + bare normalization admitted | collection assembly real | provenance hardcoded Static (3.1); auth resolver dropped in 2 modes (2.1) | `drifted` |
| 17.6 | Agent evidence runtime | reuses lifecycle emission points | `opi-agent` core (correct) | `IdentityAllocator` + emit_evidence admitted | route/tool/retry records real | stale-reauth facade (4.1) | `drifted` |
| 17.7 | evidence/finalization/redaction | reuses explicit capture + redaction helpers | product (correct) | file adapter + strict manifest admitted | redaction boundary real | redaction clean; per-run truncation (3.5) | `conforming` |
| 17.8 | legacy session migration | reuses JSONL repo + model parser | product (correct) | read-only normalization admitted | byte-identical fixtures real | no reader; byte-preservation clean | `conforming` |
| 17.9 | cross-mode/failure/rollback/docs/CI | reuses mode runners + smoke + CI | assurance (correct) | no new surface admitted | cross-mode equivalence asserted | mock-only cross-mode misses auth-resolver drop (2.1) | `drifted` |

---

## 10. Residuals and Recommendations

### Priority recommendations

1. **Fix the auth-resolver drop (2.1, Blocker)** — thread `ProviderBundle.auth_resolver` into `NonInteractiveRunner`/`RpcRunner` exactly as the interactive path does; add a non-interactive/RPC test that asserts the active route's resolved auth is not the mock fallback. This is the single item that must land before shipping.
2. **Wire real auth provenance (3.1, Major)** — classify `environment`/`credential-store`/`oauth` and emit `AuthFallback::Used` when `Layered` actually falls back; stop overwriting resolver provenance in `prepare_call`.
3. **Reconcile the stale-reauthorization mechanism (4.1, Major)** — either re-read live health before the second authorization or delete the dead loop and document that staleness is handled by the fresh-per-tool clone + `ProductToolAuthorizer::is_healthy()`; correct the stale/contradictory comment.
4. **Add the missing spec-mandated tests** (6.1–6.5): consecutive-request tool-projection snapshot, canary across args/env/provider-error channels, A05 cancellation leg, and a real (non-tautological) FAL-001 boundary test.

### Residual notes

- The redaction boundary is the strongest part of the phase — no secret crosses `EvidenceSink`, and the `decision`-vs-`authorization` field-name workaround (7.9) is a redactor-heuristic coupling worth resolving by making the redactor schema-aware rather than substring-based.
- The three `provider:model` parsers (2.2) and two `AuthProvenanceSource` enums + string tokens (7.3) are the most likely future divergence points; a single canonical parser/conversion in `opi-ai` would remove both.
- `--trace` is a real user-facing feature that is currently discoverable only via CHANGELOG (2.3).
