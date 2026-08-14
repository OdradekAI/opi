# Phase 17 Deep Agent Core Semantic Closure -- Independent Code Audit

**Auditor**: gpt5 (independent, no prior audit reports consulted)  
**Date**: 2026-08-14  
**Scope**: Phase 17 registered requirements and Tasks 17.1--17.9  
**Implementation target**: `877c41fd6c7b0c7850839f41c8fd2824e90436a6` (current committed implementation)  
**Phase exit commit**: `a4cfa4ddc74b4dfac59b4305d4657599af866480` (provenance only)  
**History use**: provenance and discovery only; no diff coverage boundary  
**Method**: Pinned-HEAD review of the normative specification, all task DoDs and 70 exit criteria, full relevant production call paths, tests, CI, documentation, and minimum-change traces. Standards and Spec were reviewed independently and are preserved as separate axes. Existing Phase 17 audit reports were neither opened nor searched.

---

## 1. Executive Summary

**Verdict: FAIL**

| Severity | Count |
|----------|------:|
| Blocker | 1 |
| Major | 19 |
| Minor | 3 |
| Info | 0 |

The implementation contains substantial provider-routing, authority, and evidence-lifecycle work, and the focused Phase 17 tests pass. It is nevertheless not a semantic closure: the interactive `Ask` path executes commands without a broker grant; production auth provenance is fabricated; configured OAuth routes are omitted unless active at startup; and finalized evidence can be incomplete, mutable, stale, or extended after finalization. Several findings overlap across the mandatory Standards and Spec axes; those independent observations are intentionally retained rather than merged.

The current-HEAD verification also is not wholly green. Formatting, clippy, doctests, rustdoc, documentation checks, and all focused Phase 17 binaries passed, but `cargo test --workspace --all-targets` fails under the repository-required external Cargo cache because `doctor_cli` searches only `<workspace>/target/debug/opi.exe`.

### Per-task summary

| Task | Title | Verdict |
|------|-------|---------|
| 17.1 | Collection route/auth preparation | FAIL -- provenance and cancellation/stream terminal semantics are incomplete |
| 17.2 | Atomic complete next-turn state | PASS-WITH-FINDINGS -- core ordering works, but public piecemeal surfaces remain |
| 17.3 | Evidence identities and lifecycle | FAIL -- the strict manifest contract accepts materially incomplete identity |
| 17.4 | Trusted tool authority | FAIL -- interactive commands may execute without a broker grant or scoped authorization |
| 17.5 | Product provider routes | FAIL -- OAuth routes and truthful credential provenance are absent |
| 17.6 | Agent evidence runtime | FAIL -- sequential side effects outrun evidence-health observation |
| 17.7 | Product evidence/finalization/redaction | FAIL -- capture is unwired in modes and file/run finalization is not immutable |
| 17.8 | Legacy migration | PASS |
| 17.9 | Cross-mode/failure/docs/CI assurance | FAIL -- A14 is not exercised as specified and the workspace gate is not green in the required cache workflow |

---

## 2. Standards Findings

The Standards reviewer assessed the complete relevant implementation against `AGENTS.md` and the Fowler-smell baseline. This axis fails independently of the Spec axis.

### 2.1 MAJOR: Extension tool names can be laundered into trusted built-ins

**File:** `crates/opi-coding-agent/src/harness.rs`; `crates/opi-coding-agent/src/tool_authority.rs`  
**Lines:** 1252--1297; 47--63  
**Cause:** Extension implementations are concatenated with product tools, and trusted origin, registration ID, and capability are inferred solely from the model-visible tool name.  
**Impact:** An extension tool named `read`, `write`, `edit`, or `bash` can acquire a built-in identity/capability, defeating the authority boundary and the intended extension exclusion.  
**Fix:** Preserve origin before concatenation and register product-created built-ins only; never infer trusted identity from a tool-supplied name.

```yaml
id: P17-AUD-GPT5-S01
source_kind: audit
source_path: docs/snapshots/phase17/audit.gpt5.md
source_model: gpt5
independence: unknown
axis: standards
severity: Major
title: Extension tool names can be laundered into trusted built-ins
claim: >-
  An extension tool using a built-in name is assigned Builtin origin, a builtin registration ID, and the corresponding trusted capability.
evidence:
  - location: crates/opi-coding-agent/src/harness.rs:1252-1297
    detail: Extension tools are appended to the same vector before trusted registration.
  - location: crates/opi-coding-agent/src/tool_authority.rs:47-63
    detail: Origin, registration ID, and capability are derived only from definition.name.
criterion_source: 'AGENTS.md Design boundaries; Phase 17 task 17.4 authority ownership'
reproduction:
  - 'rg -n "collect_tools|tools.extend\(extension_tools\)|builtin_capability|ToolOrigin::Builtin" crates/opi-coding-agent/src/harness.rs crates/opi-coding-agent/src/tool_authority.rs'
confidence: high
status: unverified
```

### 2.2 MAJOR: Provider replacement can retain a stale resolver and provenance

**File:** `crates/opi-ai/src/provider_collection.rs`  
**Lines:** 282--372  
**Cause:** One route is split across the registry and several maps, while `register` and `register_route` update different subsets.  
**Impact:** Re-registering a provider ID through the other public path can pair the new provider with the old credential resolver/source or stale metadata.  
**Fix:** Store a route as one aggregate and replace it atomically, or route every update through one private all-fields replacement operation.

```yaml
id: P17-AUD-GPT5-S02
source_kind: audit
source_path: docs/snapshots/phase17/audit.gpt5.md
source_model: gpt5
independence: unknown
axis: standards
severity: Major
title: Provider replacement can retain a stale resolver and provenance
claim: >-
  Registering a provider through one public replacement path after the other can rejoin the new provider with stale resolver, provenance, or probe state under the same string ID.
evidence:
  - location: crates/opi-ai/src/provider_collection.rs:282-296
    detail: Route state is owned by multiple parallel registries and maps.
  - location: crates/opi-ai/src/provider_collection.rs:336-372
    detail: register and register_route update different subsets of the route state.
criterion_source: 'AGENTS.md explicit ownership and fail-closed adapter boundaries'
reproduction:
  - 'rg -n "pub struct ProviderCollection|pub fn register\(|pub fn register_route|self\.resolvers|self\.sources" crates/opi-ai/src/provider_collection.rs'
confidence: high
status: unverified
```

### 2.3 MAJOR: Production route assembly discards credential provenance

**File:** `crates/opi-coding-agent/src/provider_factory.rs`; `crates/opi-ai/src/provider_collection.rs`  
**Lines:** 139--195, 336--362; 403--418  
**Cause:** The product route bundle omits provenance, registers every route as `Static`, discards `ResolvedApiKey.source`, and overwrites resolver output with `NotAttempted`.  
**Impact:** Evidence describing environment, credential-store, OAuth, or allowed fallback use is false.  
**Fix:** Make the resolver return the actual typed provenance and preserve it through preparation and evidence.

```yaml
id: P17-AUD-GPT5-S03
source_kind: audit
source_path: docs/snapshots/phase17/audit.gpt5.md
source_model: gpt5
independence: unknown
axis: standards
severity: Major
title: Production route assembly discards credential provenance
claim: >-
  Production credentials resolved from environment, store, or OAuth are recorded as Static with fallback NotAttempted.
evidence:
  - location: crates/opi-coding-agent/src/provider_factory.rs:188-197
    detail: Every production route is registered with AuthProvenanceSource::Static.
  - location: crates/opi-ai/src/provider_collection.rs:409-418
    detail: Resolver provenance is overwritten and fallback is hardcoded to NotAttempted.
criterion_source: 'AGENTS.md typed correctness and evidence-boundary truthfulness'
reproduction:
  - 'rg -n "ProviderAuthPair|AuthProvenanceSource::Static|AuthProvenance::default|AuthFallback::NotAttempted" crates/opi-ai/src/provider_collection.rs crates/opi-coding-agent/src/provider_factory.rs'
confidence: high
status: unverified
```

### 2.4 MAJOR: File evidence emission ignores flush failure

**File:** `crates/opi-coding-agent/src/evidence.rs`  
**Lines:** 118--142  
**Cause:** `emit` handles `write_all` failure but discards the result of `BufWriter::flush`, then mirrors the record as successful.  
**Impact:** A run can publish a healthy manifest while durable JSONL evidence is missing or truncated.  
**Fix:** Treat flush as emission, latch and return the first failure, and mirror the record only after successful flush.

```yaml
id: P17-AUD-GPT5-S04
source_kind: audit
source_path: docs/snapshots/phase17/audit.gpt5.md
source_model: gpt5
independence: unknown
axis: standards
severity: Major
title: File evidence emission ignores flush failure
claim: >-
  FileEvidenceSink returns success and records an item in memory when flushing that item to the evidence file fails.
evidence:
  - location: crates/opi-coding-agent/src/evidence.rs:128-142
    detail: write errors are handled, writer.flush is ignored, and the record is then pushed.
criterion_source: 'AGENTS.md fail-closed evidence boundaries'
reproduction:
  - 'rg -n "let _ = writer\.flush|records.*push|fn has_failure" crates/opi-coding-agent/src/evidence.rs'
confidence: high
status: unverified
```

### 2.5 MAJOR: `ContentDigest` accepts arbitrary strings at a strict identity boundary

**File:** `crates/opi-agent/src/evidence.rs`  
**Lines:** 310--320, 1081--1132  
**Cause:** `ContentDigest::from_hex` performs no hex or length validation, while the finalization gate checks only for emptiness.  
**Impact:** Invalid digest identities can enter a manifest that is represented as strictly complete.  
**Fix:** Use a validated fixed-width or algorithm-tagged digest and validate deserialization through `TryFrom`.

```yaml
id: P17-AUD-GPT5-S05
source_kind: audit
source_path: docs/snapshots/phase17/audit.gpt5.md
source_model: gpt5
independence: unknown
axis: standards
severity: Major
title: ContentDigest accepts arbitrary strings at a strict identity boundary
claim: >-
  ContentDigest::from_hex accepts non-hex and wrong-length strings that require_complete accepts when non-empty.
evidence:
  - location: crates/opi-agent/src/evidence.rs:310-320
    detail: The constructor wraps the supplied string without validation.
  - location: crates/opi-agent/src/evidence.rs:1087-1131
    detail: Manifest validation rejects empty digest strings only.
criterion_source: 'AGENTS.md typed correctness and fail-closed evidence boundaries'
reproduction:
  - 'rg -n "struct ContentDigest|fn from_hex|ContentDigest\(d\) if d\.is_empty" crates/opi-agent/src/evidence.rs'
confidence: high
status: unverified
```

### 2.6 MAJOR: The complete-state API retains public piecemeal mutators

**File:** `crates/opi-agent/src/agent.rs`  
**Lines:** 57--60, 214--280  
**Cause:** Seven public convenience setters remain after recording `replace_state` as the one complete replacement seam; one panics on validation and the others discard the result.  
**Impact:** The public surface duplicates the state-transition protocol and retains failure behavior that is inconsistent with the typed replacement operation.  
**Fix:** Migrate product call sites to candidate construction plus `replace_state`, then remove or privatize the piecemeal methods and preserve typed errors.

```yaml
id: P17-AUD-GPT5-S06
source_kind: audit
source_path: docs/snapshots/phase17/audit.gpt5.md
source_model: gpt5
independence: unknown
axis: standards
severity: Major
title: The complete-state API retains public piecemeal mutators
claim: >-
  Agent publicly exposes seven field-level mutation methods despite documenting replace_state as the one public replacement operation.
evidence:
  - location: crates/opi-agent/src/agent.rs:57-60
    detail: Type documentation identifies replace_state as the sole complete replacement seam.
  - location: crates/opi-agent/src/agent.rs:214-280
    detail: Seven public mutators remain; set_model panics and several discard replacement results.
criterion_source: 'AGENTS.md small/deep interfaces and Phase 17 task 17.2 surface-necessity trace'
reproduction:
  - 'rg -n "pub fn (set_model|set_max_tokens|set_thinking_config|set_initial_messages|inject_message|replace_messages|rewind_to)" crates/opi-agent/src/agent.rs'
confidence: high
status: unverified
```

### 2.7 MAJOR: `ProviderRegistry::register` is an unchecked compatibility bypass

**File:** `crates/opi-ai/src/registry.rs`  
**Lines:** 147--170  
**Cause:** The checked `register_provider` boundary coexists with a public backward-compatible alias that installs the same type without validating an empty provider ID.  
**Impact:** Invalid registry state remains constructible, and the Phase 17 removal audit misses an explicit compatibility surface.  
**Fix:** Migrate tests/callers to `register_provider` and remove the unchecked alias.

```yaml
id: P17-AUD-GPT5-S07
source_kind: audit
source_path: docs/snapshots/phase17/audit.gpt5.md
source_model: gpt5
independence: unknown
axis: standards
severity: Major
title: ProviderRegistry register is an unchecked compatibility bypass
claim: >-
  The public backward-compatible register alias accepts a provider that the checked register_provider boundary rejects.
evidence:
  - location: crates/opi-ai/src/registry.rs:147-170
    detail: register_provider validates the ID; the documented compatibility alias inserts without that validation.
criterion_source: 'AGENTS.md do not preserve compatibility unless requested; fail-closed adapter validation'
reproduction:
  - 'rg -n "Backward-compatible alias|pub fn register\(" crates/opi-ai/src/registry.rs crates/opi-ai/tests'
confidence: high
status: unverified
```

### 2.8 MINOR: Typed evidence identities do not reach authorization

**File:** `crates/opi-agent/src/agent_loop.rs`; `crates/opi-agent/src/authority.rs`  
**Lines:** 74--89, 271--277, 1041--1048; 231--243  
**Cause:** The loop mints typed run/turn/call identities but sends `run_id: None`, a separately formatted turn string, and the provider tool-call string to authorization.  
**Impact:** Authority decisions cannot be correlated to the core-owned evidence identity graph without a second string protocol.  
**Fix:** Carry typed run/turn/call IDs into the authorization request and name any provider ID separately as untrusted correlation data.

```yaml
id: P17-AUD-GPT5-S08
source_kind: audit
source_path: docs/snapshots/phase17/audit.gpt5.md
source_model: gpt5
independence: unknown
axis: standards
severity: Minor
title: Typed evidence identities do not reach authorization
claim: >-
  ToolAuthorizationRequest never receives the typed run and tool-call identities already minted by the loop.
evidence:
  - location: crates/opi-agent/src/agent_loop.rs:1041-1048
    detail: Authorization is called with run_id None and string turn/call IDs.
  - location: crates/opi-agent/src/agent_loop.rs:271-277
    detail: A typed evidence call ID exists before tool execution.
criterion_source: 'AGENTS.md typed correctness and deep-module surface necessity'
reproduction:
  - 'rg -n "let turn_id = format|run_id: None|ToolAuthorizationRequest|tool_call_ids" crates/opi-agent/src/agent_loop.rs crates/opi-agent/src/authority.rs'
confidence: high
status: unverified
```

---

## 3. Spec Findings

The Spec reviewer checked every registered criterion, task claim, DoD item, production call site, and acceptance scenario. It assessed 45 rows met, 11 partial, 12 not met, and 2 unverified; this axis fails independently of Standards.

### 3.1 MAJOR: Production auth evidence fabricates `Static` / `NotAttempted`

**File:** `crates/opi-ai/src/provider_collection.rs`; `crates/opi-coding-agent/src/provider_factory.rs`  
**Lines:** 403--418; 188--197  
**Cause:** Production discards the resolver's actual credential source/fallback and replaces it with route constants.  
**Impact:** P17-A01 cannot establish agreement among requested, resolved, actual, auth-source, and fallback evidence.  
**Fix:** Preserve typed resolver provenance and add production keychain/environment/OAuth/fallback manifest tests.

```yaml
id: P17-AUD-GPT5-P01
source_kind: audit
source_path: docs/snapshots/phase17/audit.gpt5.md
source_model: gpt5
independence: unknown
axis: spec
severity: Major
title: Production auth evidence fabricates Static and NotAttempted
claim: >-
  Every production credential source and fallback is finalized as Static and NotAttempted regardless of the resolver outcome.
evidence:
  - location: crates/opi-coding-agent/src/provider_factory.rs:188-197
    detail: All live product routes use Static provenance.
  - location: crates/opi-ai/src/provider_collection.rs:409-418
    detail: The collection overwrites resolver provenance and hardcodes fallback.
criterion_source: 'P17-PRV-004, P17-PRV-005, P17-EVD-003, P17-A01, P17-A02, CTRL-002'
reproduction:
  - 'rg -n "AuthProvenanceSource::Static|AuthFallback::NotAttempted|provenance:.*default" crates/opi-ai/src/provider_collection.rs crates/opi-coding-agent/src/provider_factory.rs crates/opi-coding-agent/src/credential_store.rs'
confidence: high
status: unverified
```

### 3.2 MAJOR: Configured OAuth providers are not dispatchable after startup

**File:** `crates/opi-coding-agent/src/provider_factory.rs`  
**Lines:** 1513--1525  
**Cause:** `build_extra_dispatch_routes` explicitly omits `github-copilot` and `openai-codex`; they are built only when active at startup.  
**Impact:** A session starting on another provider cannot switch to those configured providers through its collection, despite the multi-provider runtime contract.  
**Fix:** Register lazy-auth OAuth routes independently of startup selection and keep credential IO deferred to `prepare_call`.

```yaml
id: P17-AUD-GPT5-P02
source_kind: audit
source_path: docs/snapshots/phase17/audit.gpt5.md
source_model: gpt5
independence: unknown
axis: spec
severity: Major
title: Configured OAuth providers are not dispatchable after startup
claim: >-
  Copilot and OpenAI Codex remain metadata-only when another provider is selected at startup.
evidence:
  - location: crates/opi-coding-agent/src/provider_factory.rs:1513-1525
    detail: The extra-route builder explicitly excludes both OAuth-only provider IDs.
  - location: crates/opi-coding-agent/tests/phase17_product_evidence.rs
    detail: Cross-provider tests inject synthetic routes rather than exercising production OAuth assembly.
criterion_source: 'INV-001, P17-OUT-001, P17-PRV-001, P17-A01, task 17.5 DoD'
reproduction:
  - 'rg -n -C 12 "OAuth-only providers|github-copilot.*openai-codex" crates/opi-coding-agent/src/provider_factory.rs'
confidence: high
status: unverified
```

### 3.3 MAJOR: `--trace` is ignored by interactive and binary RPC modes

**File:** `crates/opi-coding-agent/src/main.rs`; `crates/opi-coding-agent/tests/phase17_cross_mode.rs`  
**Lines:** 883, 1057--1069, 1192--1222; 23--27  
**Cause:** The CLI forwards trace only to `NonInteractiveRunner`; RPC and interactive construction omit the recorder. The test substitutes injected builder/recorder seams and documents the asymmetry.  
**Impact:** Explicit capture is unavailable or semantically divergent in two claimed Reference Product modes.  
**Fix:** Construct the product recorder before mode branching, pass it into every entry path, and test the real mode entry cores.

```yaml
id: P17-AUD-GPT5-P03
source_kind: audit
source_path: docs/snapshots/phase17/audit.gpt5.md
source_model: gpt5
independence: unknown
axis: spec
severity: Major
title: Trace capture is ignored by interactive and binary RPC modes
claim: >-
  Supplying --trace configures non-interactive capture only; interactive and binary RPC do not receive a file evidence recorder.
evidence:
  - location: crates/opi-coding-agent/src/main.rs:1057-1069
    detail: RPC construction omits cli.trace and an evidence recorder.
  - location: crates/opi-coding-agent/src/main.rs:1192-1222
    detail: Interactive harness assembly does not call evidence configuration.
  - location: crates/opi-coding-agent/tests/phase17_cross_mode.rs:23-27
    detail: The acceptance test explicitly records both capture asymmetries.
criterion_source: 'P17-MIG-003, P17-MIG-005, P17-EVD-007, P17-A14, task 17.7 DoD'
reproduction:
  - 'rg -n "cli\.trace|new_with_runtime_packages|\.evidence\(" crates/opi-coding-agent/src/main.rs crates/opi-coding-agent/tests/phase17_cross_mode.rs'
confidence: high
status: unverified
```

### 3.4 MAJOR: The strict manifest gate accepts incomplete or stale execution facts

**File:** `crates/opi-agent/src/evidence.rs`; `crates/opi-coding-agent/src/evidence.rs`; `crates/opi-coding-agent/src/harness.rs`  
**Lines:** 612--692, 764--791, 1081--1132; 295--330; 1527--1556, 2551--2554  
**Cause:** `require_complete` does not check `EvidenceCompleteness`, system/tool-schema identities, permission scope/grants, artifacts, branch identity, or several environment facts. Product construction hardcodes several required fields empty and freezes binding/config at harness construction.  
**Impact:** A manifest can claim completeness without reconstructing the resolved execution or current route/model/branch.  
**Fix:** Represent and populate every required field at run setup/finalization, then make the strict gate exhaustive.

```yaml
id: P17-AUD-GPT5-P04
source_kind: audit
source_path: docs/snapshots/phase17/audit.gpt5.md
source_model: gpt5
independence: unknown
axis: spec
severity: Major
title: The strict manifest gate accepts incomplete or stale execution facts
claim: >-
  FinalizedManifest::require_complete accepts Incomplete manifests and production manifests lacking required system, tool, permission, artifact, branch, or current-run identity facts.
evidence:
  - location: crates/opi-agent/src/evidence.rs:1081-1132
    detail: The validator checks binding, four config digests, route, and prompt only; it never checks completeness.
  - location: crates/opi-coding-agent/src/evidence.rs:295-330
    detail: Production writes empty system/tool-schema and artifact facts.
  - location: crates/opi-coding-agent/src/harness.rs:1527-1556
    detail: Binding/config identity is frozen from the startup model rather than rebuilt per run.
criterion_source: 'P17-OUT-004, P17-EVD-003, INV-008, P17-A09, tasks 17.3 and 17.7 DoD'
reproduction:
  - 'rg -n "system_digest: None|tool_schema_digests: Vec::new|artifacts: Vec::new|pub fn require_complete|EvidenceCompleteness" crates/opi-agent/src/evidence.rs crates/opi-coding-agent/src/evidence.rs crates/opi-coding-agent/src/harness.rs'
confidence: high
status: unverified
```

### 3.5 MAJOR: Compaction is appended after its run manifest is finalized

**File:** `crates/opi-coding-agent/src/harness.rs`; `crates/opi-coding-agent/tests/phase17_product_evidence.rs`  
**Lines:** 2230--2254, 2835; 1032--1092  
**Cause:** `prompt` finalizes the run, and a later `compact` call appends a record under the same run identity without re-finalizing. The acceptance test checks record presence but not manifest terminal correlation.  
**Impact:** The immutable manifest's sequence/correlation omits a claimed member of the same run graph.  
**Fix:** Include compaction before finalization or model it as a separately finalized correlated run; reject post-finalization emission.

```yaml
id: P17-AUD-GPT5-P05
source_kind: audit
source_path: docs/snapshots/phase17/audit.gpt5.md
source_model: gpt5
independence: unknown
axis: spec
severity: Major
title: Compaction is appended after its run manifest is finalized
claim: >-
  Manual compaction emits a newer record under a run whose manifest has already been finalized with an earlier terminal sequence.
evidence:
  - location: crates/opi-coding-agent/src/harness.rs:2230-2254
    detail: prompt finalizes evidence before returning.
  - location: crates/opi-coding-agent/src/harness.rs:2835
    detail: compact emits evidence after that finalization.
  - location: crates/opi-coding-agent/tests/phase17_product_evidence.rs:1069-1092
    detail: The test performs prompt then compact but does not compare manifest correlation with the final record.
criterion_source: 'P17-EVD-002, P17-EVD-003, P17-A09, immutable finalization contract'
reproduction:
  - 'cargo test -p opi-coding-agent --test phase17_product_evidence phase17_one_run_graph_includes_tool_execution_record -- --exact'
confidence: high
status: unverified
```

### 3.6 MAJOR: Evidence failure cannot stop the next sequential tool in a batch

**File:** `crates/opi-agent/src/agent_loop.rs`  
**Lines:** 292--365, 462--513  
**Cause:** Every sequential tool is authorized and executed before the loop emits any tool evidence or observes the resulting evidence-health transition.  
**Impact:** If evidence for tool 1 fails, tool 2 has already launched even when complete evidence is policy-required.  
**Fix:** For each sequential call, authorize, emit/observe authorization, recheck health, execute, emit outcome, then proceed.

```yaml
id: P17-AUD-GPT5-P06
source_kind: audit
source_path: docs/snapshots/phase17/audit.gpt5.md
source_model: gpt5
independence: unknown
axis: spec
severity: Major
title: Evidence failure cannot stop the next sequential tool in a batch
claim: >-
  A two-tool sequential batch can execute tool 2 before an evidence failure from tool 1 is observed.
evidence:
  - location: crates/opi-agent/src/agent_loop.rs:292-365
    detail: The sequential execution loop completes all calls first.
  - location: crates/opi-agent/src/agent_loop.rs:462-513
    detail: Tool evidence and health advancement occur only in the later post-batch loop.
criterion_source: 'P17-EVD-009, P17-AUT-003, P17-AUT-005, P17-FAL-002, P17-FAL-003, P17-A12'
reproduction:
  - 'rg -n "if batch_is_sequential|execute_tool\(|Emit one Tool record|emit_evidence\(" crates/opi-agent/src/agent_loop.rs'
confidence: high
status: unverified
```

### 3.7 MAJOR: File-sink flush failures are converted to success

**File:** `crates/opi-coding-agent/src/evidence.rs`; `crates/opi-coding-agent/src/harness.rs`  
**Lines:** 118--166; 2564--2572  
**Cause:** The file adapter ignores flush errors, and the harness discards finalization errors.  
**Impact:** P17-A11 can produce a visible finalized manifest for non-durable/incomplete evidence without a typed failure reaching the caller.  
**Fix:** Latch and propagate flush/finalization errors, publish the manifest only after durable record completion, and add real-file lifecycle failure tests.

```yaml
id: P17-AUD-GPT5-P07
source_kind: audit
source_path: docs/snapshots/phase17/audit.gpt5.md
source_model: gpt5
independence: unknown
axis: spec
severity: Major
title: File-sink flush failures are converted to success
claim: >-
  A failed evidence-file flush does not mark the run incomplete and does not prevent manifest publication.
evidence:
  - location: crates/opi-coding-agent/src/evidence.rs:139
    detail: writer.flush return value is discarded.
  - location: crates/opi-coding-agent/src/harness.rs:2572
    detail: recorder.finalize_run return value is discarded.
criterion_source: 'P17-EVD-008, P17-EVD-011, P17-FAL-003, P17-A11, PRIN-005'
reproduction:
  - 'rg -n "let _ = writer\.flush|let _ = recorder\.finalize_run" crates/opi-coding-agent/src/evidence.rs crates/opi-coding-agent/src/harness.rs'
confidence: high
status: unverified
```

### 3.8 MINOR: Authorization lacks stable run/call correlation identities

**File:** `crates/opi-agent/src/agent_loop.rs`  
**Lines:** 271--277, 1041--1048  
**Cause:** Authorization receives no run ID and uses provider strings although core identities already exist.  
**Impact:** Authority evidence cannot be joined directly to the stable core-owned graph.  
**Fix:** Thread typed `RunId`, `TurnId`, and pre-minted `CallId` through authorization.

```yaml
id: P17-AUD-GPT5-P08
source_kind: audit
source_path: docs/snapshots/phase17/audit.gpt5.md
source_model: gpt5
independence: unknown
axis: spec
severity: Minor
title: Authorization lacks stable run and call correlation identities
claim: >-
  The trusted authorization boundary receives run_id None and provider strings instead of the typed identities minted by Agent Core.
evidence:
  - location: crates/opi-agent/src/agent_loop.rs:1041-1048
    detail: The authorization request carries no run ID and uses the provider call string.
  - location: crates/opi-agent/src/agent_loop.rs:271-277
    detail: The core-owned evidence call ID is available before execution.
criterion_source: 'Task 17.4 DoD, P17-EVD-001, P17-EVD-002, P17-OUT-004'
reproduction:
  - 'rg -n "run_id: None|tool_call_ids|authorize_and_verify" crates/opi-agent/src/agent_loop.rs'
confidence: high
status: unverified
```

---

## 4. Correctness and Invariant Findings

### 4.1 MAJOR: Unterminated provider streams are silently reported as success

**File:** `crates/opi-agent/src/agent_loop.rs`; `crates/opi-ai/src/provider_collection.rs`  
**Lines:** 220--233, 722--727; 579--602, 682--704  
**Cause:** When the stream reaches natural EOF without a terminal assistant event, the Agent emits `AgentEnd` and returns `Ok`. The collection-level drain helper treats the same condition as `StreamError`; the prepared-call active flag also clears only on an error or terminal event.  
**Impact:** A partial provider failure becomes successful completion with unchanged state, and the logical prepared call can remain active.  
**Fix:** Convert EOF without a terminal event into a typed stream failure and clear the active attempt on all stream termination/drop paths.

```yaml
id: P17-AUD-GPT5-I01
source_kind: audit
source_path: docs/snapshots/phase17/audit.gpt5.md
source_model: gpt5
independence: unknown
axis: invariants
severity: Major
title: Unterminated provider streams are silently reported as success
claim: >-
  A provider stream that ends without a terminal assistant event causes agent_loop to return Ok rather than a typed stream failure.
evidence:
  - location: crates/opi-agent/src/agent_loop.rs:722-727
    detail: Missing terminal_assistant returns Ok with preserved state.
  - location: crates/opi-ai/src/provider_collection.rs:579-602
    detail: The collection drain helper classifies terminal-less EOF as StreamError.
  - location: crates/opi-ai/src/provider_collection.rs:682-704
    detail: Attempt activity is cleared only for yielded error or terminal event.
criterion_source: 'P17-FAL-003, INV-006'
reproduction:
  - 'Add a provider fixture that yields a non-terminal delta and EOF; assert Agent::prompt returns a stream error and a second prepared attempt is not stuck active.'
confidence: high
status: unverified
```

### 4.2 MAJOR: Cancellation cannot interrupt credential-store or OAuth preparation

**File:** `crates/opi-agent/src/agent_loop.rs`; `crates/opi-ai/src/provider_collection.rs`  
**Lines:** 129--140; 390--418  
**Cause:** The agent awaits `prepare_call` directly, and `prepare_call` awaits `AuthResolver::resolve` without selecting on the request cancellation token. The resolver contract has no cancellation input.  
**Impact:** Cancellation during credential-store IO or OAuth refresh is not promptly observable and can wait indefinitely for the external preparation effect.  
**Fix:** Make preparation cancellation-aware, selecting on the frozen request token and defining resolver cancellation/partial-effect reporting.

```yaml
id: P17-AUD-GPT5-I02
source_kind: audit
source_path: docs/snapshots/phase17/audit.gpt5.md
source_model: gpt5
independence: unknown
axis: invariants
severity: Major
title: Cancellation cannot interrupt credential-store or OAuth preparation
claim: >-
  Cancelling a request while AuthResolver::resolve is pending does not cancel or terminate prepare_call until the resolver itself returns.
evidence:
  - location: crates/opi-agent/src/agent_loop.rs:129-140
    detail: prepare_call is awaited without a cancellation select.
  - location: crates/opi-ai/src/provider_collection.rs:409
    detail: Resolver IO is awaited directly despite the request carrying a cancellation token.
criterion_source: 'Phase 17 design lines 268-270 and 685-688; P17-FAL-003'
reproduction:
  - 'Use a resolver blocked on a channel, start Agent::prompt, cancel the Agent, and assert the prompt settles as Cancelled before releasing the resolver.'
confidence: high
status: unverified
```

---

## 5. Security and Redaction Findings

### 5.1 BLOCKER: Interactive `Ask` executes without a broker grant or scoped permission

**File:** `crates/opi-coding-agent/src/tool_authority.rs`; `crates/opi-coding-agent/tests/phase17_tool_authority.rs`  
**Lines:** 211--285; 485--508  
**Cause:** In interactive mode, `command.execute=Ask` returns `Allow` merely because the mode is interactive. The authorizer intentionally ignores final arguments, uses `LOCAL_ADAPTER_ID` rather than the actual routed adapter, and returns capability identity as permission scope/reference.  
**Impact:** An arbitrary registered command tool can reach `Tool::execute` with no permission-broker grant and without path/operation/adapter scope being authorized. This is a normal-path authority bypass with potential user-data and command-execution impact.  
**Fix:** Route every `Ask` decision through the actual permission broker using validated final arguments and actual adapter/path/operation scope; absent, pending, stale, or mismatched permission must result in zero tool executions.

```yaml
id: P17-AUD-GPT5-SEC01
source_kind: audit
source_path: docs/snapshots/phase17/audit.gpt5.md
source_model: gpt5
independence: unknown
axis: security
severity: Blocker
title: Interactive Ask executes without a broker grant or scoped permission
claim: >-
  Interactive command.execute Ask returns Allow and invokes Tool::execute without a permission-broker grant bound to final arguments, the actual adapter, and path or operation scope.
evidence:
  - location: crates/opi-coding-agent/src/tool_authority.rs:265-285
    detail: Ask is allowed solely when run_mode is Interactive and no grant is required.
  - location: crates/opi-coding-agent/src/tool_authority.rs:211-259
    detail: Final arguments are intentionally ignored and command permission is checked against LOCAL_ADAPTER_ID.
  - location: crates/opi-coding-agent/tests/phase17_tool_authority.rs:485-508
    detail: The focused test confirms one Tool::execute call for the interactive Ask case.
criterion_source: 'INV-005, P17-OUT-003, P17-AUT-003, P17-AUT-005, P17-A08, Phase 17 broker-before-execute order'
reproduction:
  - 'cargo test -p opi-coding-agent --test phase17_tool_authority phase17_command_execute_ask_interactive_allows_and_executes -- --exact'
confidence: high
status: unverified
```

No credential or raw user-content leak was found in the inspected evidence/diagnostic paths. Producer-side redaction, typed evidence payload channels, and secret-canary tests are otherwise strong.

---

## 6. Test Quality Findings

### 6.1 MAJOR: P17-A14 does not exercise the claimed cross-mode conjunction

**File:** `crates/opi-coding-agent/tests/phase17_cross_mode.rs`  
**Lines:** 1--27, 134--224, 455--505  
**Cause:** The test labels a `CodingHarness` builder seam as the interactive binary, uses no tool calls, does not inject cancellation in the same fixture, and manually injects an RPC recorder instead of exercising binary RPC trace wiring.  
**Impact:** Route-only equivalence passes while authority, cancellation, durable evidence, and actual entry-point differences remain undetected.  
**Fix:** Run one tool-bearing/cancellable fixture through the real interactive launcher, harness, print, JSON/NDJSON, and RPC paths and compare the same route, authority, cancellation, evidence, and legacy facts.

```yaml
id: P17-AUD-GPT5-T01
source_kind: audit
source_path: docs/snapshots/phase17/audit.gpt5.md
source_model: gpt5
independence: unknown
axis: test-quality
severity: Major
title: P17-A14 does not exercise the claimed cross-mode conjunction
claim: >-
  The A14 acceptance test can pass without spawning the interactive binary or exercising authority, cancellation, and durable evidence through the same fixture.
evidence:
  - location: crates/opi-coding-agent/tests/phase17_cross_mode.rs:143-186
    detail: Interactive coverage is a CodingHarness builder seam.
  - location: crates/opi-coding-agent/tests/phase17_cross_mode.rs:455-505
    detail: Tool counts are zero and TUI/RPC trace behavior is classified as source-inferred.
criterion_source: 'P17-A14, P17-MIG-005, task 17.9 DoD'
reproduction:
  - 'rg -n "interactive assembly|Interactive TUI loop not spawned|RPC binary path|tool.*zero|source-inferred" crates/opi-coding-agent/tests/phase17_cross_mode.rs'
confidence: high
status: unverified
```

### 6.2 MINOR: The workspace test gate fails with the required external Cargo cache

**File:** `crates/opi-coding-agent/tests/doctor_cli.rs`  
**Lines:** 1269--1296  
**Cause:** `opi_bin` searches only `<workspace>/target/{debug,release}/opi`, ignoring `CARGO_TARGET_DIR` and Cargo's integration-test binary location.  
**Impact:** `cargo test --workspace --all-targets` fails in the repository-mandated external-cache workflow even though the binary was built successfully.  
**Fix:** Use Cargo's binary environment contract or resolve the configured target directory rather than assuming a workspace-local target path.

```yaml
id: P17-AUD-GPT5-T02
source_kind: audit
source_path: docs/snapshots/phase17/audit.gpt5.md
source_model: gpt5
independence: unknown
axis: test-quality
severity: Minor
title: The workspace test gate fails with the required external Cargo cache
claim: >-
  doctor_unknown_scope_exits_one cannot locate opi.exe when CARGO_TARGET_DIR points at the repository external cache.
evidence:
  - location: crates/opi-coding-agent/tests/doctor_cli.rs:1269-1296
    detail: The helper searches only workspace_root/target and then panics when spawning that path.
  - location: 'cargo test -p opi-coding-agent --test doctor_cli -- --nocapture'
    detail: At audit HEAD with the external cache, 43 tests passed and doctor_unknown_scope_exits_one failed with os error 3 for workspace/target/debug/opi.exe.
criterion_source: 'AGENTS.md external Cargo cache workflow; task 17.9 workspace gate'
reproduction:
  - 'Set CARGO_TARGET_DIR to an external directory, then run cargo test -p opi-coding-agent --test doctor_cli doctor_unknown_scope_exits_one -- --exact --nocapture.'
confidence: high
status: unverified
```

---

## 7. Integration Findings

### 7.1 MAJOR: File evidence reuses mutable paths and exposes stale manifests

**File:** `crates/opi-coding-agent/src/evidence.rs`; `crates/opi-coding-agent/src/harness.rs`  
**Lines:** 98--166; 2513--2519  
**Cause:** Each prompt truncates fixed `evidence.jsonl`, overwrites fixed `manifest.json`, and leaves the prior durable manifest in place during setup or a later failed run.  
**Impact:** A new/incomplete run can destroy prior evidence while still presenting the previous run's manifest as finalized; run artifacts are not immutable.  
**Fix:** Allocate per-run immutable paths, remove no prior finalized data, and atomically publish a manifest exactly once after durable completion.

```yaml
id: P17-AUD-GPT5-X01
source_kind: audit
source_path: docs/snapshots/phase17/audit.gpt5.md
source_model: gpt5
independence: unknown
axis: integration
severity: Major
title: File evidence reuses mutable paths and exposes stale manifests
claim: >-
  Reusing an explicit trace directory truncates prior evidence and can leave its old manifest visible while the new run is incomplete or failed.
evidence:
  - location: crates/opi-coding-agent/src/evidence.rs:98-115
    detail: Setup truncates evidence.jsonl but does not remove or version manifest.json.
  - location: crates/opi-coding-agent/src/evidence.rs:152-166
    detail: Finalization replaces the fixed manifest path.
  - location: crates/opi-coding-agent/src/harness.rs:2513-2519
    detail: The same recorder is set up again for each prompt.
criterion_source: 'PRIN-005, P17-EVD-008, immutable finalization contract, INV-008'
reproduction:
  - 'Run two prompts through one FileEvidenceSink directory, fail the second run after setup, and inspect that the first manifest remains while its JSONL was truncated.'
confidence: high
status: unverified
```

### 7.2 MAJOR: In-memory capture mixes records and bindings across prompts

**File:** `crates/opi-agent/src/evidence.rs`; `crates/opi-coding-agent/src/harness.rs`; `crates/opi-coding-agent/src/evidence.rs`  
**Lines:** 997--1031; 1523--1560, 2513--2572; 351--370  
**Cause:** `InMemoryEvidenceSink::setup` does not clear prior records/manifest/failure, while `EvidenceCapture` and its binding are created once per harness. Finalization consumes all accumulated records, uses the first record's run ID and the last record's terminal fields, and keeps the startup model digest.  
**Impact:** A second prompt or RPC command can finalize a graph mixing two runs and a stale model/config binding while still passing `require_complete`.  
**Fix:** Make capture and recorder state run-scoped, clear or replace all per-run state during setup, and finalize only a single run's correlated record slice with current dynamic binding.

```yaml
id: P17-AUD-GPT5-X02
source_kind: audit
source_path: docs/snapshots/phase17/audit.gpt5.md
source_model: gpt5
independence: unknown
axis: integration
severity: Major
title: In-memory capture mixes records and bindings across prompts
claim: >-
  Reusing a harness with InMemoryEvidenceSink can finalize records from multiple runs under the first run ID and a startup-time model binding.
evidence:
  - location: crates/opi-agent/src/evidence.rs:997-1004
    detail: InMemoryEvidenceSink setup returns without clearing records, artifacts, manifest, or prior success state.
  - location: crates/opi-coding-agent/src/harness.rs:1523-1560
    detail: EvidenceCapture binding/config is created once from the startup model.
  - location: crates/opi-coding-agent/src/evidence.rs:351-370
    detail: Manifest correlation takes run from the first record and terminal fields from the last.
criterion_source: 'P17-EVD-001, P17-EVD-002, P17-EVD-003, P17-A01, P17-A09, CTRL-002'
reproduction:
  - 'Reuse one evidence-enabled CodingHarness for two prompts with different selected providers; compare recorder run IDs and the second manifest correlation/binding.'
confidence: high
status: unverified
```

---

## 8. Invariant Verification

| Invariant | Code evidence | Test coverage |
|-----------|---------------|---------------|
| INV-001: one resolved provider route owns dispatch | **FAIL** -- OAuth-only configured providers are excluded from extra dispatch routes | Synthetic alpha/beta tests pass; no production OAuth switching test |
| INV-002: provider wire details stay behind provider-neutral contracts | PASS -- prepared calls retain provider-neutral request/stream seams | Provider/wire fixture suites cover the inspected adapters |
| INV-003: next-turn preparation is one atomic complete replacement | PASS -- candidate is built then applied before stop/queues | `hooks_queues` and `agent_wrapper` cover valid/invalid ordering |
| INV-004: failed/cancelled replacement preserves all mutable fields | PASS -- route validation precedes assignment and tests snapshot state | Focused next-turn failure tests pass |
| INV-005: no side effect without trusted scoped authorization | **FAIL** -- interactive Ask and unbound arguments/adapter/scope allow execution | The focused test currently asserts the violating execution count of one |
| INV-006: failures/partial effects are not success and later effects fail closed | **FAIL** -- terminal-less EOF returns `Ok`; sequential tools outrun evidence failure | No terminal-less EOF or two-sequential-tool failure test |
| INV-007: legacy/session data is not rewritten | PASS -- normalization/remediation and byte-preservation paths are present | Legacy migration/rollback focused binaries pass |
| INV-008: a finalized manifest reconstructs one immutable resolved run | **FAIL** -- required facts are empty/stale, paths are mutable, and compaction follows finalization | Negative tests cover only a subset of required facts and miss repeated-run lifecycle |

---

## 9. Minimum-change Conformance

The standardized overlay was active for all nine tasks. Placement and dependency direction are mostly sound; the dominant drift is semantic shortcutting and retained surface rather than new crates or speculative dependencies.

| Task | Scenario/source | Reuse | Placement | Surface | Production slice | Ceiling/trigger | Status |
|------|-----------------|-------|-----------|---------|------------------|-----------------|--------|
| 17.1 | Provider collection preparation | Existing registry/resolver reused | `opi-ai` is appropriate | Route state split across maps | Productive in Agent loop | Provenance/cancellation/EOF gaps trigger revisit | `triggered` |
| 17.2 | Complete next-turn replacement | Existing Agent state reused | `opi-agent` is appropriate | Seven public piecemeal methods remain | Productive across prompts | Recorded surface ceiling exceeded | `drifted` |
| 17.3 | Evidence schema/lifecycle | Existing diagnostic/redaction types reused | Core schema placement appropriate | Strict identities remain string/optional | Product manifest consumes it | Required facts/gate incomplete | `drifted` |
| 17.4 | Trusted authority | Existing registry/schema validation reused | Core/product split mostly appropriate | Untyped name-to-trust and string correlation remain | Productive tool path | Permission broker/scope threshold triggered | `triggered` |
| 17.5 | Product provider routes | Existing provider factories reused | Product placement appropriate | Compatibility alias and provenance shortcut remain | OAuth routes missing from live collection | Multi-provider production trigger unmet | `triggered` |
| 17.6 | Agent evidence runtime | Existing loop and sink reused | Core placement appropriate | No new speculative dependency | Productive, but sequential barrier is late | Evidence-health trigger unmet | `triggered` |
| 17.7 | Product file/finalization adapter | Existing recorder contract reused | Product-only file adapter is correct | No exporter/database added | Live capture exists only in some modes | Immutable/fail-closed ceiling exceeded | `triggered` |
| 17.8 | Legacy migration | Existing session/config seams reused | Product placement appropriate | No legacy reader/shim found | Productive migration path | Ceiling respected | `conforming` |
| 17.9 | Cross-mode/failure/docs/CI | Existing runners/tests/CI reused | Assurance placement appropriate | No runtime seam added | Claimed binary conjunction substituted with builder seams | Actual-mode acceptance trigger unmet | `triggered` |

---

## 10. Verification, Residuals, and Recommendations

### Verification at audit HEAD

- `python scripts/opi-doc-check.py` -- PASS.
- `cargo fmt --check --all` -- PASS.
- `cargo clippy --workspace --all-targets -- -D warnings` -- PASS.
- `cargo test --workspace --doc` -- PASS.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` -- PASS.
- Focused Phase 17 binaries for provider collection/auth, next-turn state, evidence contract/runtime, authority, product evidence, legacy migration, cross-mode, failure/rollback, and API audit -- PASS.
- `cargo test --workspace --all-targets` -- FAIL in `doctor_cli::doctor_unknown_scope_exits_one` under the required external Cargo cache; 43/44 tests in that binary passed.
- Current HEAD remained `877c41fd6c7b0c7850839f41c8fd2824e90436a6` throughout the audit.

### Residuals

- Actual Linux/macOS/Windows evidence in the ledger is tied to `40f2e6e`, not the current audit HEAD. Later commits may be source-compatible, but prior-SHA CI is not current-HEAD proof; P17-A15/PLT-001 therefore remain unverified here.
- Task 17.7 is marked passing while its archived task `evidence` field is null. This weakens the trace but is not a separate defect beyond the concrete evidence findings above.
- No blocker was found in legacy byte preservation, provider-neutral wire isolation, next-turn atomic ordering, or producer-side secret redaction.

### Priority recommendations

1. Close the authority bypass first: make interactive `Ask` and every command capability depend on the real broker grant, final validated arguments, routed adapter, and immutable scope.
2. Repair evidence as a run-scoped immutable transaction: truthful provenance, exhaustive manifest facts/gate, per-run recorder state and paths, durable flush, no post-finalization emission, and sequential launch barriers.
3. Register every configured provider route, including lazy-auth OAuth routes, and add production credential-source/fallback switching tests.
4. Replace the A14 builder approximation with real mode entry-point tests using one tool-bearing, cancellable fixture.
5. Fix the external-target binary lookup, rerun the complete six workspace gates, then obtain current-HEAD three-platform CI evidence before reasserting Phase 17 exit.

**Test impact:** none (audit report only).
