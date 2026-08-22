# Phase 17 Deep Agent Core Semantic Closure -- Independent Code Audit

**Auditor**: glm5.3 (independent, no prior audit reports consulted)
**Date**: 2026-08-22
**Scope**: Phase 17 registered requirements and Tasks 17.1--17.9
**Implementation target**: `136c380f0c5eea541190cc1a0f5c1d62f983b4e8` (current committed implementation)
**Phase exit commit**: `a4cfa4ddc74b4dfac59b4305d4657599af866480` (provenance only)
**History use**: provenance and discovery only; no diff coverage boundary
**Method**: Current-HEAD audit at the pinned commit. A 15-agent workflow performed the
deep read (8 parallel full-file readers covering 102 source/test files across opi-ai,
opi-agent, and opi-coding-agent, including the 5,151-line CodingHarness and all
phase-17 test suites), then five independent axis reviewers (separate Matt Standards
and Spec axes plus correctness/invariants, security/redaction, and
test-quality/integration/residuals), then adversarial refutation verification of every
Blocker/Major candidate (both confirmed). The auditor additionally read the fourteen
central seams in full personally (provider_collection, provider, auth, agent_loop,
agent, loop_types, authority, evidence, tool, hooks, policy, tool_authority,
product evidence adapter, and the harness finalization path) and executed 13 focused
hermetic test suites (263 tests, all green) plus `scripts/opi-doc-check.py` (PASS)
against the pinned commit with the shared external cargo cache. No build, test, or
reproduction command ran against a dirty tree: the only worktree deltas at audit time
were two user-deleted phase-17 docs, and all source paths were verified identical to
`audit_head`.

**Contamination disclosure**: no `docs/snapshots/phase17/audit.*.md`, evaluator
report, remediation plan, or realign/research artifact was opened before this report
was complete. The auditor's persistent session memory contains high-level summaries
of earlier phase-17 audit rounds; none of it was used as evidence -- every finding
below was independently re-derived from code at `audit_head` this session, and both
Major findings survived a dedicated adversarial refutation pass. The two worktree
file deletions (`citation-addendum-2026-08-21.md`, `remediation-plan.md`) are
unrelated user changes that were not read.

---

## 1. Executive Summary

**Verdict: PASS-WITH-FINDINGS**

| Severity | Count |
|----------|-------|
| Blocker  | 0     |
| Major    | 2     |
| Minor    | 37    |
| Info     | 35    |

Phase 17's core semantic closures are genuinely implemented and unusually well
tested. The atomic NextTurnState ordering, single-prepared-call retry semantics,
fail-closed tool authority chain with stale-generation reauthorization, and the
typed evidence lifecycle with producer-boundary redaction all verify cleanly in code
and in the 263 focused tests run at the audit commit. The two Majors are: a
pre-existing Bedrock event-stream parser defect (CRC-invalid frames silently
dropped, allowing content loss to terminate in a clean `Done`), and a vacuous
P17-A07 acceptance test that leaves the extension-to-builtin registration laundering
path without any behavioral guard. Neither blocks shipping, but both need attention
before the next phase.

### Per-task summary

| Task | Title | Verdict |
|------|-------|---------|
| 17.1 | Add collection-owned route and authentication preparation | PASS-WITH-FINDINGS (4.1 pre-existing adapter defect on owned surface) |
| 17.2 | Cut over Agent to durable atomic NextTurnState | PASS (4.2 edge) |
| 17.3 | Define evidence identities, health, and storage-neutral lifecycle | PASS (6.3 fixture depth) |
| 17.4 | Cut over trusted tool registrations and mandatory authorization | PASS-WITH-FINDINGS (6.1) |
| 17.5 | Wire the Reference Product to dispatchable provider routes | PASS (2.5, 2.7) |
| 17.6 | Expand Agent evidence runtime over stable identities | PASS |
| 17.7 | Cut over Reference Product evidence, finalization, and redaction | PASS (3.6, 3.7, 5.5) |
| 17.8 | Migrate legacy session routes and preserve opaque trace artifacts | PASS |
| 17.9 | Close local cross-mode, failure, rollback, documentation, and CI acceptance | PASS (6.2, 6.4) |

Criteria sweep (52 `P17-*` requirements): structurally met in full for 44; 8 carry
evidence-depth partials (A01 actual-route leg, A07, FAL-002 one matrix row, EVD-003
two fixture rows, A15 external runs, PRV-003 secrecy posture, FAL-004 residual
surfaces, plus the Fowler-baseline judgement partial on the Standards axis). No
criterion is unmet; no Non-goal was implemented (no exporter, no Eval, no new
permission language, no OS-sandbox claim, no new crate or facade, no second
registry); the MIG-006 removal set was independently re-verified absent from all
production source.

---

## 2. Standards Findings (Matt axis)

Repo standards sources: `AGENTS.md` (working principles, verification, style) plus
the Fowler smell baseline as judgement calls. The structural posture is clean:
dependency direction holds (opi-ai and opi-agent have no internal deps and no
product-policy symbols -- asserted behaviorally by `phase17_api_audit.rs`), manifests
duplicate no workspace metadata, `thiserror` is used for every library error type
observed, and closed enums model the closed states throughout the phase-17 surface.

### 2.1 MINOR: Phase/task history persists in source comments of phase-17-relevant files

**File:** `crates/opi-coding-agent/src/policy.rs`
**Lines:** 1--8, 18, 80
**Cause:** `policy.rs` and `crates/opi-coding-agent/src/execution/permission.rs`
carry stage/task identifiers in comments ("S8.4, S10", "task 3.8", "Task 2 wiring",
"Phase 4", "Phase 16 design / task 16.10"). AGENTS.md forbids preserving Phase,
task, PR, or review history in source comments.
**Impact:** History belongs in snapshots/CHANGELOG/Git; in-source copies drift (see
2.2 for a live drift case) and violate the documented contract.
**Fix:** Strip the stage/task tokens from these comments, keeping the current
contract text only.

```yaml
id: glm53-001
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: standards
severity: Minor
title: Phase/task history persists in source comments
claim: Source comments in policy.rs and execution/permission.rs cite Phase/task/stage identifiers, violating the documented no-history-in-comments rule.
evidence:
  - location: crates/opi-coding-agent/src/policy.rs:1-8
    detail: Module doc cites "(S8.4, S10)" and "task 3.8"; line 18 "Task 2 wiring"; line 80 "Phase 4".
  - location: crates/opi-coding-agent/src/execution/permission.rs
    detail: Comments cite Phase 16 design / task 16.10.
criterion_source: AGENTS.md working principles (comments describe current contracts; no Phase/task/PR history)
reproduction: []
confidence: high
status: unverified
```

### 2.2 MINOR: Stale Phase-8 hook-contract comment documents the pre-17.2/17.4 ordering the same file disproves

**File:** `crates/opi-agent/tests/hooks_queues.rs`
**Lines:** 715--722
**Cause:** The section comment states `before_tool_call` runs AFTER schema
validation, that `should_stop_after_turn` precedes `prepare_next_turn`, and that a
terminal stop skips preparation. The production loop (agent_loop.rs:1723--1092) and
the phase-17.2 tests in the same file establish the opposite ordering (hook before
schema; prepare before stop; stop still observes the applied state).
**Impact:** A reader of the test file receives a hook contract that is exactly
inverted relative to the shipped semantics -- the drift case 2.1 warns about,
located on the file that pins the ordering.
**Fix:** Rewrite the section comment to the current ordering.

```yaml
id: glm53-002
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: standards
severity: Minor
title: Stale Phase-8 hook-contract comment contradicts implemented ordering
claim: The hooks_queues.rs section comment documents hook/stop/prepare ordering that the implementation and the same file's Phase 17 tests disprove.
evidence:
  - location: crates/opi-agent/tests/hooks_queues.rs:715-722
    detail: Comment asserts before_tool_call after schema, stop before prepare, terminal stop skips prepare.
  - location: crates/opi-agent/src/agent_loop.rs:1723-1092
    detail: Production order is resolve -> hook -> schema -> authorize; prepare -> apply -> stop.
criterion_source: AGENTS.md (comments describe current contracts)
reproduction: []
confidence: high
status: unverified
```

### 2.3 MINOR: Mutating-tool classification duplicated inline instead of calling `policy::is_mutating_tool`

**File:** `crates/opi-coding-agent/src/harness.rs`
**Lines:** 1559 (and the sibling site computing `mutating_allowed`)
**Cause:** The harness re-implements the `write | edit | bash` classification as an
inline `matches!` when computing `EffectiveUserPolicy.mutating_allowed`, while
`policy::is_mutating_tool` owns the same list for the CLI gate.
**Impact:** A future edit to one site silently diverges the authorization fact folded
into the policy digest from the `--allow-mutating` selection gate (Fowler Duplicated
Code with a security-relevant drift direction).
**Fix:** Call `policy::is_mutating_tool` at the harness site.

```yaml
id: glm53-003
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: standards
severity: Minor
title: Mutating-tool list duplicated inline in harness.rs
claim: harness.rs re-implements the mutating-tool name list inline instead of reusing policy::is_mutating_tool, creating drift risk between the authorization fact and the selection gate.
evidence:
  - location: crates/opi-coding-agent/src/harness.rs:1559
    detail: Inline matches! over write/edit/bash feeds EffectiveUserPolicy digest input.
  - location: crates/opi-coding-agent/src/policy.rs:7-9
    detail: is_mutating_tool owns the same classification.
criterion_source: AGENTS.md (reuse, no duplicated logic); Fowler Duplicated Code
reproduction: []
confidence: high
status: unverified
```

### 2.4 MINOR: EffectiveUserPolicy digest hashes Rust `Debug` output, contradicting its documented canonical rendering

**File:** `crates/opi-coding-agent/src/harness.rs`
**Lines:** 1575 (digest inputs), with `tool_authority.rs:139-185`
**Cause:** Two digest inputs to the immutable policy are
`format!("{:?}", trust_decision)` and `format!("{:?}", packages)` renderings.
Derived `Debug` output is not a guaranteed-stable serialization: identical policy
facts can digest differently across toolchain versions, while the type documents "a
stable sha256 over a canonical rendering".
**Impact:** The digest-addressed policy identity (and the `policy_ref` surfaced in
every Allow) is not reproducible across compiler versions; evidence correlation by
policy digest weakens.
**Fix:** Render the two facts through explicit canonical serializers (sorted
key/value pairs) instead of `Debug`.

```yaml
id: glm53-004
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: standards
severity: Minor
title: Policy digest hashes Debug output rather than a canonical rendering
claim: EffectiveUserPolicy digest inputs include format!("{:?}") renderings of TrustDecision and Option<Vec<PackageResource>>, so the digest-addressed policy identity is not toolchain-stable.
evidence:
  - location: crates/opi-coding-agent/src/harness.rs:1575
    detail: Digest inputs include Debug-formatted trust/package facts.
  - location: crates/opi-coding-agent/src/tool_authority.rs:139-141
    detail: Doc promises "stable sha256 over a canonical rendering of the snapshotted facts".
criterion_source: AGENTS.md (contracts documented in comments must hold)
reproduction: []
confidence: high
status: unverified
```

### 2.5 MINOR: Extension-reachable registration failures become startup panics via `.expect` in provider assembly

**File:** `crates/opi-coding-agent/src/provider_factory.rs`
**Lines:** 169, 1817 (and 190 for the id-collision arm)
**Cause:** `build_harness_collection` converts registry registration errors --
reachable from extension-declared provider ids and model overrides, and from an
extension provider id colliding with a built-in route id -- into process panics via
`.expect`, and `route_auth_resolver` `.expect()`s a cross-table invariant
(`route_credentials` returning `Some` for any constructed adapter).
**Impact:** Untrusted-adjacent input (extension declarations) crashes the binary at
startup instead of producing the typed startup error the phase's fail-closed style
uses everywhere else.
**Fix:** Propagate `RegistrationError` as a typed startup diagnostic; replace the
cross-table `expect` with a typed config error.

```yaml
id: glm53-005
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: standards
severity: Minor
title: provider_factory startup panics on extension-reachable registration errors
claim: build_harness_collection and route_auth_resolver panic via .expect on extension-reachable registration errors and cross-table invariants instead of typed startup errors.
evidence:
  - location: crates/opi-coding-agent/src/provider_factory.rs:169
    detail: .expect on register_route result.
  - location: crates/opi-coding-agent/src/provider_factory.rs:1817
    detail: route_auth_resolver .expects route_credentials Some invariant.
criterion_source: AGENTS.md (typed errors, fail-closed at boundaries; no panic on expected input)
reproduction: []
confidence: high
status: unverified
```

### 2.6 MINOR: Public `CodingHarness::set_model` panics where its validated sibling returns the typed error

**File:** `crates/opi-coding-agent/src/harness.rs`
**Lines:** 1872--1874
**Cause:** The public `set_model` resolves the route with `.expect`, while
`set_model_validated` and the internal `apply_agent_model` return the typed
`AgentError` for the same condition.
**Impact:** A caller feeding a mistyped model string (ordinary user input) crashes
the process; the typed failure path already exists.
**Fix:** Return the typed error (or delegate to the validated path with a
fallback-to-current-policy decision documented).

```yaml
id: glm53-006
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: standards
severity: Minor
title: Public set_model panics on unresolvable route
claim: CodingHarness::set_model panics via .expect for any model string whose route does not resolve instead of returning the typed error its validated sibling produces.
evidence:
  - location: crates/opi-coding-agent/src/harness.rs:1872-1874
    detail: .expect on route resolution in the public setter.
criterion_source: AGENTS.md (no panic on expected input; typed errors at boundaries)
reproduction: []
confidence: high
status: unverified
```

### 2.7 MINOR: Six parallel provider-id match cascades force synchronized edits per provider

**File:** `crates/opi-coding-agent/src/provider_factory.rs`
**Lines:** ~1698 and five sibling tables
**Cause:** The same string-dispatch on provider ids is re-implemented in at least
six parallel match tables (id list, explicit-config check, env names,
credentials+scheme, list-models metadata, auth descriptor/availability). Adding or
changing one provider requires coordinated edits across all of them.
**Impact:** Fowler Repeated Switches drift hazard on the dispatch surface phase 17.5
consolidated; a missed table silently degrades one aspect of a provider (see 7.6 for
an existing asymmetry of exactly this shape).
**Fix:** Fold the per-provider facts into one provider-descriptor row consumed by
all sites.

```yaml
id: glm53-007
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: standards
severity: Minor
title: Six parallel provider-id match cascades in provider_factory
claim: provider_factory.rs re-implements the same provider-id switch across at least six parallel match tables, requiring synchronized edits per provider change.
evidence:
  - location: crates/opi-coding-agent/src/provider_factory.rs:1698
    detail: One of six parallel provider-id match cascades.
criterion_source: Fowler Repeated Switches (judgement call)
reproduction: []
confidence: medium
status: unverified
```

### 2.8 INFO: Dead or misleading surface cluster on phase-17-adjacent modules

**File:** `crates/opi-coding-agent/src/harness.rs`, `provider_factory.rs`, `crates/opi-ai/src/{model.rs,provider.rs}`, `crates/opi-ai/src/bedrock/mod.rs`
**Lines:** harness.rs:4081; provider_factory.rs:2257; bedrock/mod.rs:1028
**Cause:** `InteractiveCodingHooks::new(_allow_mutating: bool)` discards its only
(safety-sounding) parameter; `auth_descriptor_for_profile` has no src callers and
survives only because a test pins its symbol string; `opi_ai::{Model, ProviderKind}`
re-exports have zero production consumers; `bedrock::redact_credentials` is a dead
public stub that always returns `"***"`.
**Impact:** Misleading API surface (Speculative Generality / dead code) on files the
phase touched; the grep-pinned function is kept alive by its own test.
**Fix:** Delete or wire each; if retained, document why.

```yaml
id: glm53-008
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: standards
severity: Info
title: Dead or misleading surface kept on phase-17-adjacent modules
claim: InteractiveCodingHooks::new ignores its parameter, auth_descriptor_for_profile is caller-less but test-pinned, Model/ProviderKind re-exports are unused, and bedrock::redact_credentials is a dead stub.
evidence:
  - location: crates/opi-coding-agent/src/harness.rs:4081
    detail: _allow_mutating discarded.
  - location: crates/opi-coding-agent/src/provider_factory.rs:2257
    detail: No src callers; symbol pinned by a source-grep test.
  - location: crates/opi-ai/src/bedrock/mod.rs:1028
    detail: Public stub returning "***" unconditionally.
criterion_source: AGENTS.md (no speculative surface; remove dead code made unused)
reproduction: []
confidence: high
status: unverified
```

---

## 3. Spec Findings (Matt axis)

Sweep of all 52 `P17-*` requirements plus the 15 scenarios against code and tests.
The load-bearing criteria verified in code by this auditor personally:
prepare-call-once-per-logical-call with frozen route/auth across retries
(PRV-003), no-silent-fallback typed failures (PRV-004), canonical-selection
validation before atomic apply with cancel rollback (NXT-002), stop-after-apply
ordering and pre-poll termination (NXT-003/004), the full resolve -> hook ->
schema -> authorize -> stale-verify -> execute chain with zero-execution outcomes
(AUT-001..008), setup-before-effects, withhold-manifest-on-failure, and
stale-allow reauthorization (EVD-007/008/009), byte-identical legacy fixtures and
no trace reader (MIG-001/004), and the removed-interface set (MIG-006).

### 3.1 MINOR: `set_model_validated` persists `model_change` before runtime apply and the two validations differ

**File:** `crates/opi-coding-agent/src/harness.rs`
**Lines:** 1971+ (persist) vs the apply path's `validate_next_turn_candidate`
**Cause:** The harness writes the canonical `model_change` session entry before
applying the model to the Agent, and the pre-persist validation
(`validate_current_thinking_for_model`) omits the `thinking_level_map` check that
`validate_next_turn_candidate` performs at apply time.
**Impact:** For a target model whose level map cannot resolve the current thinking
level, the durable session records a model the runtime then rejects, leaving the
session entry and live state divergent until the next successful change.
**Fix:** Run the shared intrinsic validation before persisting, or apply first and
persist only on success (compensating for the session-write ordering the facade
provides).

```yaml
id: glm53-009
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: spec
severity: Minor
title: model_change persisted before validated runtime apply
claim: set_model_validated persists the model_change entry before applying the model and its pre-persist validation omits the thinking_level_map check the apply path enforces, allowing durable/runtime divergence.
evidence:
  - location: crates/opi-coding-agent/src/harness.rs:1971
    detail: Session write precedes the Agent state application.
  - location: crates/opi-agent/src/loop_types.rs:377-391
    detail: Apply-time validation includes thinking_level_map resolution absent from the pre-persist check.
criterion_source: P17-PRV-002 / 17.5 DoD (canonical selection persists only when provable)
reproduction: []
confidence: medium
status: unverified
```

### 3.2 MINOR: `contains(':')` bare/canonical heuristic silently reinterprets colon-bearing bare model ids

**File:** `crates/opi-coding-agent/src/harness.rs`
**Lines:** 188 (and duplicated at `crates/opi-agent/src/harness.rs:342`)
**Cause:** Classification of model input as canonical vs bare uses
`model.contains(':')`. A bare model id that itself contains a colon and whose
prefix happens to match a provider id is treated as a canonical spec and resolved
against that provider.
**Impact:** A class of valid bare inputs is dispatched to an unintended provider or
rejected with a confusing wrong-provider error, and the durable
`ModelInputSource::Canonical` provenance is false. Bedrock-style ids with colons
are the concrete family.
**Fix:** Parse with `parse_model_spec` and classify on its success plus a
registry-provider-prefix check, not on the raw colon test; share one decision (see
2.3-family duplication).

```yaml
id: glm53-010
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: spec
severity: Minor
title: Colon-bearing bare model ids misclassified as canonical
claim: The contains(':') heuristic classifies colon-bearing bare model ids as canonical provider specs, misrouting or mislabeling them.
evidence:
  - location: crates/opi-coding-agent/src/harness.rs:188
    detail: Bare/canonical decision by contains(':').
  - location: crates/opi-agent/src/harness.rs:342
    detail: Same heuristic duplicated for ModelInputSource classification.
criterion_source: P17-PRV-002 / P17-MIG-002
reproduction: []
confidence: medium
status: unverified
```

### 3.3 MINOR: A01 actual-route agreement asserted against canned mock values that disagree with the dispatched route

**File:** `crates/opi-coding-agent/tests/phase17_product_evidence.rs`
**Lines:** 1968--1974
**Cause:** The A01 acceptance test asserts the beta run's provider-reported actual
route equals `mock`/`mock-model` -- a base fixture artifact that disagrees with the
`beta:b1` route the run actually dispatched -- and never asserts the alpha
manifest's actual route at all.
**Impact:** The scenario's "requested, resolved, actual ... evidence agrees" leg is
proven for requested/resolved only; actual-route agreement (PRV-005's third leg) is
untested at the acceptance level.
**Fix:** Have the mock provider report the dispatched provider/model in
`response_model` and assert all three legs per provider.

```yaml
id: glm53-011
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: spec
severity: Minor
title: A01 actual-route leg pinned to canned fixture values
claim: phase17_harness_switches_providers_with_matching_route_evidence asserts the beta actual route against canned mock:mock-model values and skips the alpha actual-route assertion, so actual-route agreement is unproven.
evidence:
  - location: crates/opi-coding-agent/tests/phase17_product_evidence.rs:1968-1974
    detail: Actual-route assertion uses fixture constants disagreeing with the dispatched beta:b1 route.
criterion_source: P17-PRV-005 / P17-A01
reproduction: []
confidence: high
status: unverified
```

### 3.4 MINOR: Bedrock Converse serializer replays assistant Thinking blocks as ordinary text

**File:** `crates/opi-ai/src/bedrock/mod.rs`
**Lines:** 966--967
**Cause:** The request serializer's `AssistantContent::Thinking` arm maps thinking
content into the plain `{"text": ...}` content block family instead of a Bedrock
reasoning/thinking representation.
**Impact:** For the thinking models the catalog advertises, multi-turn conversation
semantics are corrupted: prior reasoning is replayed as ordinary assistant text
(and conversely the wire cannot round-trip the thinking the model produced).
**Fix:** Map thinking blocks to the Converse reasoning content shape (or omit them
deliberately) with a fixture proving the round-trip.

```yaml
id: glm53-012
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: spec
severity: Minor
title: Bedrock replays Thinking blocks as plain text
claim: The Bedrock Converse serializer maps AssistantContent::Thinking to ordinary text content, corrupting conversation semantics for advertised thinking models.
evidence:
  - location: crates/opi-ai/src/bedrock/mod.rs:966-967
    detail: Thinking arm inside the plain-text content mapping.
criterion_source: P17-PRV-006 (provider-specific encoding behind neutral interfaces, correctly)
reproduction: []
confidence: medium
status: unverified
```

### 3.5 MINOR: Bedrock model-spec parsing splits on the first colon, misparsing colon-bearing Bedrock model ids

**File:** `crates/opi-ai/src/bedrock/mod.rs`
**Lines:** 182--185
**Cause:** The adapter's `split_once(':')` assumes one colon separates provider and
model. Bedrock model ids themselves contain colons (e.g. inference-profile ids), so
a canonical `bedrock:<id-with-colon>` spec misparses the family suffix and produces
a confusing "misparsed family" error instead of resolving the actual model.
**Impact:** Legitimate canonical selections fail at the adapter with a wrong
diagnostic; combined with 3.2 the colon family is mishandled at two layers.
**Fix:** Split on the first colon only at the collection layer (already done) and
pass the bare model id to the adapter, or parse via `parse_model_spec`.

```yaml
id: glm53-013
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: spec
severity: Minor
title: Bedrock split_once(':') misparses colon-bearing model ids
claim: The Bedrock adapter re-splits the model spec on the first colon, misparsing model ids that themselves contain colons.
evidence:
  - location: crates/opi-ai/src/bedrock/mod.rs:182-185
    detail: split_once(':') on the request model string.
criterion_source: P17-PRV-002 (canonical selection must resolve one provider/model)
reproduction: []
confidence: medium
status: unverified
```

### 3.6 MINOR: Manifest artifact set validated against an empty observation while the sink validates against `finalize_artifact` accumulation

**File:** `crates/opi-coding-agent/src/evidence.rs`
**Lines:** 620--631
**Cause:** `build_finalized_manifest` hard-codes an empty artifact list and
validates the candidate against an empty artifact observation, while
`FileEvidenceSink::finalize_run` re-validates the same manifest against the
artifacts accumulated through `finalize_artifact`. Any future producer that starts
finalizing artifacts would pass the builder's check and fail only at the sink.
**Impact:** Two divergent validation inputs for one invariant; the empty-set design
is documented (see 3.9) but the asymmetry is a latent trap.
**Fix:** Thread the recorder's artifact set into the builder observation (the
recorder already exposes it).

```yaml
id: glm53-014
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: spec
severity: Minor
title: Manifest artifacts validated against empty observation at build time
claim: build_finalized_manifest validates an always-empty artifact set while FileEvidenceSink validates the same manifest against finalize_artifact-accumulated artifacts, leaving two divergent validation inputs.
evidence:
  - location: crates/opi-coding-agent/src/evidence.rs:620-631
    detail: artifacts: Vec::new() and EvidenceRunObservation::new(..., &[]).
  - location: crates/opi-coding-agent/src/evidence.rs:328-334
    detail: Sink-side validate_observation uses state.artifacts.
criterion_source: P17-EVD-003 / P17-EVD-011
reproduction: []
confidence: medium
status: unverified
```

### 3.7 MINOR: Public `build_finalized_manifest` panics on an empty record set instead of a typed error

**File:** `crates/opi-coding-agent/src/evidence.rs`
**Lines:** 653--670 (`terminal_correlation` expects)
**Cause:** `terminal_correlation` `.expect`s at least one record. The two internal
call sites guard it (the main path returns a typed Finalization error on empty
records at harness.rs:3038--3045; the standalone-compaction path guarantees two
records because `emit_manual_compaction_evidence` must succeed before the builder
runs), but the function is `pub` and its non-empty precondition is documented only
in an inline comment.
**Impact:** A future caller (or embedder) passing an empty graph panics rather than
receiving the typed `EvidenceError` the contract promises everywhere else.
**Fix:** Return `EvidenceError::Finalization` for the empty case.

```yaml
id: glm53-015
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: spec
severity: Minor
title: build_finalized_manifest panics on empty records
claim: The public build_finalized_manifest panics via expect when called with an empty record set instead of returning a typed EvidenceError; internal callers guard it, the public contract does not.
evidence:
  - location: crates/opi-coding-agent/src/evidence.rs:653-670
    detail: terminal_correlation .expect("manifest correlation requires at least one record").
  - location: crates/opi-coding-agent/src/harness.rs:3038-3045
    detail: The one call site needing it adds its own empty-records guard.
criterion_source: P17-FAL-001 (typed failure classes at the evidence boundary)
reproduction: []
confidence: high
status: unverified
```

### 3.8 INFO: RBK-002's "restores one coherent pre-Phase runtime" half has no in-repo verification

**File:** `crates/opi-coding-agent/tests/phase17_failure_rollback.rs`
**Lines:** ~1869
**Cause:** The rollback fixtures prove byte preservation of sessions/evidence under
a subsequent Phase-17 reload, not behavior under a pre-Phase binary; no test
exercises a reverted tree.
**Impact:** The rollback criterion's coherence half rests on design review; the
preservation half (RBK-003/004) is well covered.
**Fix:** None required locally; record the split explicitly at phase exit (it
already is, in spirit) or add a revert-profile smoke when a rollback ever actually
happens.

```yaml
id: glm53-016
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: spec
severity: Info
title: RBK-002 rollback-coherence half unverified in-repo
claim: No in-repo test exercises a pre-Phase runtime after rollback; only byte preservation under Phase-17 reload is proven.
evidence:
  - location: crates/opi-coding-agent/tests/phase17_failure_rollback.rs:1869
    detail: Rollback fixture verifies preservation, not pre-Phase behavior.
criterion_source: P17-RBK-002
reproduction: []
confidence: medium
status: unverified
```

### 3.9 INFO: Product manifest emits zero artifact references and Unknown time by documented design

**File:** `crates/opi-coding-agent/src/evidence.rs`
**Lines:** 611--627
**Cause:** The Reference Product manifest always carries an empty artifact set and
`EnvironmentFacts.time` = Unknown/NotReported; input identity is carried by
digests. The source documents this as a constraint on producers (an empty set
satisfies it vacuously), not a requirement to emit.
**Impact:** "Finalized artifact references" in the manifest bullet is satisfied
vacuously today; acceptable interpretation, recorded here so the exit claim is read
accurately.
**Fix:** None required; revisit when a real artifact store exists.

```yaml
id: glm53-017
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: spec
severity: Info
title: Manifest artifact set and time are vacuously empty/unknown by design
claim: The product manifest always emits zero artifact references and Unknown time, satisfying those manifest bullets vacuously as documented in source.
evidence:
  - location: crates/opi-coding-agent/src/evidence.rs:611-627
    detail: artifacts: Vec::new() with the documented vacuity rationale; time Unknown/NotReported.
criterion_source: P17-EVD-003 (manifest bullet list)
reproduction: []
confidence: high
status: unverified
```

---

## 4. Correctness and Invariants Findings

Verified clean by direct trace: one `prepare_call` per turn with every retry reusing
the frozen route/auth (provider_collection.rs:410--479, 799--828;
agent_loop.rs:298--332, 911--934); complete-replacement next-turn semantics with
unit validation, atomic apply, and cancellation restoring the prior state
(agent_loop.rs:1028--1111); the fail-closed authority chain (agent_loop.rs:1723--1894)
including the parallel launch-boundary generation check (:728--737); evidence
identities minted immediately before emission with health advanced on any sink
failure (:2137--2169); in-band stream `Error` treated as a typed non-retryable
provider failure (:475--497); and streams ending without a terminal event failing
closed (:992--998, provider_collection.rs:646--664).

### 4.1 MAJOR: Bedrock event-stream parser silently discards CRC-invalid frames, allowing content loss to end in a clean `Done`

**File:** `crates/opi-ai/src/bedrock/event_stream.rs`
**Lines:** 34--63 (drop path), with `bedrock/mod.rs:392--429` (live path) and `event_stream.rs:385--394` (test codifying the drop)
**Cause:** `parse_single_frame` returns `None` on prelude or message CRC mismatch
and `parse_frames` has no error channel, so corrupted frames are silently skipped.
The live consumer's only integrity guard is `!saw_done`; a corrupted mid-stream
`contentBlockDelta` followed by an intact `messageStop` sets `saw_done` and the run
terminates with a normal `Done` plus metadata. The in-file test at :385--394
asserts the silent drop as expected behavior. The sibling Anthropic adapter maps
malformed frames to `Err(StreamError)` (anthropic.rs:914--920), so the codebase
itself establishes the fail-closed norm this parser violates.
**Impact:** Silent model-output corruption on any CRC mismatch -- the model's reply
loses content with no error, no incomplete marker, and finalized evidence records a
successful call. TLS makes the trigger rare, but the defect contradicts the
repository's fail-closed-validation-at-adapter-boundaries rule and the phase's
no-silent-terminal posture. (Attribution: `event_stream.rs` was created with the
Bedrock provider at `99b263d` and last touched by a phase-12 repair -- it predates
Phase 17 and is reported under Current-HEAD authority, not as a phase-17
regression.)
**Fix:** Return a typed parse error for CRC mismatches and surface it as
`ProviderError::StreamError` from the Bedrock stream loop; update the test at
:385--394 accordingly. Adversarially re-verified and confirmed at HEAD.

```yaml
id: glm53-018
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: invariants
severity: Major
title: Bedrock silently drops CRC-invalid frames; clean Done after content loss
claim: The Bedrock event-stream parser discards CRC-invalid frames without an error channel, so a corrupted mid-stream frame produces silent content loss and the stream can still terminate with a clean Done event.
evidence:
  - location: crates/opi-ai/src/bedrock/event_stream.rs:34-63
    detail: parse_single_frame None on CRC mismatch; parse_frames drops it.
  - location: crates/opi-ai/src/bedrock/mod.rs:392-429
    detail: Live loop's only guard is !saw_done; messageStop still yields clean Done.
  - location: crates/opi-ai/src/bedrock/event_stream.rs:385-394
    detail: Test asserts the silent drop of a CRC-corrupted frame.
criterion_source: AGENTS.md (fail-closed validation at protocol/adapter boundaries); P17-FAL spirit
reproduction:
  - cargo test -p opi-ai --test bedrock_fixtures (CRC-corruption case asserts current silent-drop behavior)
confidence: high
status: unverified
```

### 4.2 MINOR: Drained steering/follow-up input is silently lost when a later turn of the same run fails

**File:** `crates/opi-agent/src/agent.rs`
**Lines:** 925--963 (rollback), with `agent_loop.rs:1115--1153` (drain)
**Cause:** The loop drains steering (or pops follow-up) messages out of the shared
queues into loop-local state. If a subsequent turn of the same run then fails or is
cancelled, `run_with_token` discards the loop state for ordinary errors (the
documented rollback contract), so the user's queued message exists neither in the
queue nor in the durable Agent state.
**Impact:** User input silently disappears on a failure path; the FAL-003 posture
(nothing converted into silent loss) is preserved for side effects but not for
queued user intent. No test covers it.
**Fix:** On rollback, restore undelivered drained messages to their queues (or
surface them on the `AgentRunResult`).

```yaml
id: glm53-019
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: invariants
severity: Minor
title: Steering/follow-up input lost when a later turn fails after drain
claim: drain_queue/pop_follow_up remove queued user input irreversibly and the ordinary-failure rollback discards the loop state that held it, silently losing the input.
evidence:
  - location: crates/opi-agent/src/agent.rs:955-963
    detail: Ordinary errors skip self.state = run.state (rollback).
  - location: crates/opi-agent/src/agent_loop.rs:1115-1153
    detail: Steering drained and popped into loop-local state before the failure can occur.
criterion_source: P17-FAL-003 / INV-006 (nothing silently lost)
reproduction: []
confidence: high
status: unverified
```

### 4.3 MINOR: Anthropic stream loop continues after emitting a malformed-frame error

**File:** `crates/opi-ai/src/anthropic.rs`
**Lines:** 914--920 (fixture loop; HTTP loop at ~1030 uses the same shape)
**Cause:** The `Malformed` arm pushes `Err(StreamError)` but does not `break`
(the `UsageError` arm does), so one stream can deliver an `Err` followed by further
`Ok` events including a terminal `Done`.
**Impact:** The adapter violates terminal-event ordering after an error. The agent
loop currently stops at the first `Err`, containing the impact, but any consumer
that drains the full stream (e.g. `drain_to_completion` semantics) sees
post-error events.
**Fix:** `break` after pushing the malformed error, matching the sibling adapters.

```yaml
id: glm53-020
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: invariants
severity: Minor
title: Anthropic stream continues after malformed-frame error
claim: The Anthropic SSE parse loop pushes an Err for a malformed frame without breaking, allowing post-error Ok events including Done in one stream.
evidence:
  - location: crates/opi-ai/src/anthropic.rs:914-920
    detail: Malformed arm lacks break; UsageError arm breaks.
criterion_source: P17-FAL spirit (stream fail-closed); OpenAI adapters stop at first malformed frame
reproduction: []
confidence: high
status: unverified
```

### 4.4 MINOR: `AuthorizationDecision::Deny` carries unvalidated free-form strings in core

**File:** `crates/opi-agent/src/authority.rs`
**Lines:** 276--284
**Cause:** `Deny { stable_code, redacted_reason }` accepts arbitrary `String`s with
no validation or defensive redaction, while the parallel evidence channel
(`ToolAuthorizationFacts::denied`) validates the code as an opaque identity and
redacts the reason at construction (evidence.rs:1367--1379).
**Impact:** A product authorizer that embeds arguments or secrets in a deny reason
crosses into model-visible tool results and (after the evidence-side re-wrap)
partially into evidence; the guarantee currently rests on the product authorizer's
discipline.
**Fix:** Validate/redact at the decision boundary the same way the evidence
constructor does.

```yaml
id: glm53-021
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: invariants
severity: Minor
title: Deny decision strings unvalidated in core
claim: AuthorizationDecision::Deny accepts unvalidated free-form stable_code/redacted_reason strings, unlike the evidence-side denied() constructor which validates and redacts.
evidence:
  - location: crates/opi-agent/src/authority.rs:276-284
    detail: Plain String fields, no constructor validation.
  - location: crates/opi-agent/src/evidence.rs:1367-1379
    detail: Evidence channel validates and redacts the same facts.
criterion_source: P17-AUT-005 / P17-FAL-004
reproduction: []
confidence: medium
status: unverified
```

### 4.5 MINOR: Session ids are millisecond-timestamp hex; same-millisecond creation collides on the same path

**File:** `crates/opi-coding-agent/src/session_coordinator.rs`
**Lines:** 956--963
**Cause:** `generate_session_id()` is `format!("{ts:x}")` of a millisecond
timestamp. The fork path has a collision-avoidance loop; the create path does not,
and concurrent processes share the resolution.
**Impact:** Two sessions created in the same millisecond (including across
processes) target the same `.jsonl` path, interleaving entries from two logical
sessions.
**Fix:** Add a per-process counter or randomness to the id (RunId's UUIDv7 pattern
is the in-repo precedent).

```yaml
id: glm53-022
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: invariants
severity: Minor
title: Session id millisecond collision
claim: Session ids are millisecond-timestamp hex and the create path has no collision avoidance, so same-millisecond sessions (including cross-process) share one file.
evidence:
  - location: crates/opi-coding-agent/src/session_coordinator.rs:956-963
    detail: generate_session_id formats as_millis as hex.
criterion_source: INV-007 (session reconstruction integrity)
reproduction: []
confidence: high
status: unverified
```

### 4.6 MINOR: Active Anthropic construction hard-fails on operational credential-store errors, contradicting the lazy-credential contract

**File:** `crates/opi-coding-agent/src/provider_factory.rs`
**Lines:** ~1368
**Cause:** Constructing the active Anthropic route treats credential-store
operational failures (backend unavailable/corrupt) as startup errors, while the
generic API-key path and the module's stated contract keep credential resolution
lazy per call.
**Impact:** A transient keychain fault can leave the user unable to start the
binary at all -- including being unable to run the remediation commands.
**Fix:** Distinguish operational store errors from absent credentials; register the
route and fail at prepare_call with the typed remediation instead.

```yaml
id: glm53-023
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: invariants
severity: Minor
title: Anthropic startup hard-fails on operational credential-store errors
claim: Active Anthropic construction turns operational credential-store errors into startup failure instead of lazy per-call typed failures, diverging from the module contract and blocking remediation.
evidence:
  - location: crates/opi-coding-agent/src/provider_factory.rs:1368
    detail: Active-route construction propagates store operational errors.
criterion_source: P17-PRV-004 / 17.5 DoD (failures surface at prepare_call with stable remediation)
reproduction: []
confidence: medium
status: unverified
```

### 4.7 INFO: Provenance labeling rests on the default-sentinel convention and Static route registration

**File:** `crates/opi-ai/src/provider_collection.rs`
**Lines:** 454--459 (sentinel), with `provider_factory.rs:190` (Static registration)
**Cause:** `prepare_call` overwrites `auth.provenance` with the route-registered
source whenever the resolver returned exactly `AuthProvenance::default()`
(Static + NotAttempted), so a resolver that truthfully reported Static while the
route was registered with a different classification would be mislabeled; and the
product factory registers every dispatch route with
`AuthProvenanceSource::Static` regardless of what its resolver will report. Benign
at HEAD because product resolvers that make real source/fallback decisions attach
their provenance directly and the sentinel arm only fires for default returns --
but the truthfulness of `Environment{name}`/`CredentialStore{kind}` strings rests
on resolver convention (free-form `String`s, shape-validated only in evidence).
**Impact:** Provenance evidence is convention-truthful rather than
construction-enforced; a future resolver that returns the default after a real
fallback decision would be silently relabeled.
**Fix:** Have the sentinel arm compare against a resolver-reported "made no
decision" marker instead of the default value, and register product routes with
their real source classification.

```yaml
id: glm53-024
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: invariants
severity: Info
title: Provenance default-sentinel overwrite and Static route registration
claim: prepare_call overwrites a resolver's default-valued provenance with the registered source, and the product registers all routes as Static, so provenance truthfulness rests on resolver convention rather than construction.
evidence:
  - location: crates/opi-ai/src/provider_collection.rs:454-459
    detail: Sentinel overwrite arm.
  - location: crates/opi-coding-agent/src/provider_factory.rs:190
    detail: All dispatch routes registered with AuthProvenanceSource::Static.
criterion_source: P17-PRV-005
reproduction: []
confidence: medium
status: unverified
```

### 4.8 INFO: `RegistrationId::new` skips the validation every evidence-side opaque identity enforces

**File:** `crates/opi-agent/src/authority.rs`
**Lines:** 43--55
**Cause:** `RegistrationId::new` accepts any string (empty, padded, control
characters) while the nine `opaque_identity!` types all validate and return
`Result`.
**Impact:** A malformed registration id from trusted assembly surfaces only later
as an evidence-incomplete failure (opaque-identity error at the tool-evidence
boundary) instead of at construction.
**Fix:** Route `RegistrationId` through the same validation.

```yaml
id: glm53-025
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: invariants
severity: Info
title: RegistrationId::new performs no validation
claim: RegistrationId::new accepts empty/padded/control-character strings unlike the validated opaque identities used across evidence.
evidence:
  - location: crates/opi-agent/src/authority.rs:43-55
    detail: Infallible new(); compare opaque_identity! macro at evidence.rs:382-429.
criterion_source: P17-EVD-001 (stable identities)
reproduction: []
confidence: high
status: unverified
```

### 4.9 INFO: Preflight-failure `AgentRunResult` abandons a never-setup sink run

**File:** `crates/opi-agent/src/agent.rs`
**Lines:** 976--987
**Cause:** `preflight_failure` builds an `AgentRunResult` with a fresh
`IdentityAllocator` (an id unrelated to any emitted record), `Active` lifecycle,
and the bound evidence sink. Drop or `into_execution_result` then calls
`abandon_run` on a run for which `setup` was never invoked -- for
`FileEvidenceSink`, after the harness's `setup_evidence_run` already allocated a
run directory that remains behind as an empty `evidence.jsonl`.
**Impact:** Harmless to correctness (abandon on a never-active run is tolerated),
but leaves an empty capture directory and mints a run id that correlates to
nothing.
**Fix:** Build preflight failures with a `FinalizedOrAbandoned` lifecycle when no
setup occurred, or skip sink binding.

```yaml
id: glm53-026
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: residuals
severity: Info
title: Preflight failure abandons a never-setup evidence run
claim: preflight_failure constructs an Active-lifecycle AgentRunResult bound to the sink with an unrelated run id, causing abandon_run on a run whose setup never ran and leaving an empty capture directory for the file adapter.
evidence:
  - location: crates/opi-agent/src/agent.rs:976-987
    detail: preflight_failure with lifecycle Active and sink bound.
criterion_source: P17-EVD (lifecycle coherence)
reproduction: []
confidence: medium
status: unverified
```

### 4.10 INFO: In-memory sink misuse branches and multi-lock sequences are asymmetric

**File:** `crates/opi-agent/src/evidence.rs`
**Lines:** 2385--2493
**Cause:** Post-finalization `emit` returns an error without recording a failure
(while `finalize_run` misuse does record one), and the per-field `Mutex` sequence in
`emit` is not atomic across concurrent emitters, so out-of-order recording is
possible.
**Impact:** Contained: `finalize_run` fails closed on the strictly-increasing
sequence check and the oracle is test-only; noted for conformance symmetry.
**Fix:** Record the failure in the post-finalization emit branch; consider one
state lock.

```yaml
id: glm53-027
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: invariants
severity: Info
title: In-memory sink misuse/lifecycle asymmetries
claim: InMemoryEvidenceSink does not record a failure for post-finalization emit misuse and its multi-lock emit sequence is not atomic across concurrent callers; contained by finalize_run's fail-closed checks.
evidence:
  - location: crates/opi-agent/src/evidence.rs:2401-2406
    detail: Post-finalized emit errors without recording failure.
criterion_source: P17-EVD-011 (conformance oracle semantics)
reproduction: []
confidence: medium
status: unverified
```

### 4.11 INFO: `ApiMappedProvider::replace_model_catalog` can return `Err` with mutated child route catalogs

**File:** `crates/opi-ai/src/api_mapped.rs`
**Lines:** ~256
**Cause:** The rollback documented by the `Provider` trait ("implementations must
leave their current catalog unchanged when returning Err") can itself fail while
restoring children, leaving a partially mutated state while still returning `Err`.
**Impact:** Edge-case violation of the atomic-replacement contract; the collection
layer's replace-all is unaffected (it pre-validates).
**Fix:** Validate children before mutating, or document the weakened guarantee.

```yaml
id: glm53-028
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: invariants
severity: Info
title: api_mapped catalog rollback can leave mutated children on Err
claim: ApiMappedProvider::replace_model_catalog can return Err after partially mutating child route catalogs when the rollback itself fails.
evidence:
  - location: crates/opi-ai/src/api_mapped.rs:256
    detail: Child restoration inside the Err path can partially fail.
criterion_source: Provider::replace_model_catalog contract (provider.rs:44-56)
reproduction: []
confidence: low
status: unverified
```

### 4.12 INFO: `date_to_unix` performs `u32` year arithmetic that underflows for year 0

**File:** `crates/opi-ai/src/retry.rs`
**Lines:** ~165
**Cause:** The IMF-fixdate parser computes with `u32` years; an RFC-valid date with
year 0000 (and month <= 2) underflows -- panic in debug, wrapped garbage in
release.
**Impact:** Malformed/edge server `Retry-After` dates cannot crash a release
build; a debug build would panic on a crafted header.
**Fix:** Use `i32`/`u64` arithmetic with a year floor.

```yaml
id: glm53-029
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: correctness
severity: Info
title: date_to_unix u32 underflow for year 0
claim: retry.rs date_to_unix underflows u32 arithmetic for an IMF-fixdate with year 0 and month <= 2.
evidence:
  - location: crates/opi-ai/src/retry.rs:165
    detail: u32 year arithmetic.
criterion_source: null
reproduction: []
confidence: medium
status: unverified
```

---

## 5. Security / Redaction Findings

Verified clean end to end: secrets live in `SecretString`/`SecretKey` with manual
redacting `Debug`; `PreparedProviderCall`'s Debug prints `<redacted>` auth;
`ProviderErrorSummary` is a sealed type whose `from_untrusted` discards payloads
(with compile_fail doc guards); header injection is rejected against the reserved
auth set before any network call; evidence payloads are typed with `RedactedValue`
as the only structured channel and `RedactedEvidenceText` re-redacting at
construction; the A10 canary matrix across sink/file/diagnostics/print/JSON/RPC
surfaces passed in the suites run at HEAD. The findings below are defense-in-depth
gaps, none reachable with a raw secret crossing a sink boundary as shipped.

### 5.1 MINOR: StreamingProxy echoes raw malformed input and unredacted handler responses under `redact_secrets = true`

**File:** `crates/opi-agent/src/streaming_proxy.rs`
**Lines:** ~243
**Cause:** With redaction enabled, the proxy writes the raw malformed input line
verbatim into the `proxy_error` response (`"raw": trimmed`) and forwards every
handler `SdkResponse` unredacted; only `ProxyEvent::Agent` values are scrubbed.
**Impact:** Secret-bearing text crossing those two output paths leaves the process
without any redaction on a surface whose configuration promises it.
**Fix:** Route the raw-error echo and handler responses through the same
`SecretRedactor` pass.

```yaml
id: glm53-030
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: security
severity: Minor
title: StreamingProxy raw echo and unredacted handler responses
claim: With redact_secrets=true the proxy writes the raw malformed input line and handler SdkResponses to output without redaction; only Agent events are scrubbed.
evidence:
  - location: crates/opi-agent/src/streaming_proxy.rs:243
    detail: proxy_error embeds the raw trimmed input; handler responses forwarded unredacted.
criterion_source: P17-FAL-004
reproduction: []
confidence: medium
status: unverified
```

### 5.2 MINOR: Shared `SecretRedactor` field-name matching is exact-match and misses common credential spellings

**File:** `crates/opi-agent/src/streaming_proxy.rs`
**Lines:** ~453
**Cause:** The sensitive-field check compares lowercased keys for exact equality
against a ten-entry list; `client_secret`, `secret_key`, `secret_token`,
`api-key`, `x-api-key`, `auth_token`, `id_token`, `session_token`, `passwd` are
not matched, so opaque secrets under those keys rely solely on known-prefix value
patterns.
**Impact:** The shared scrubber used by `redact()`/`redact_text()` (and thus
diagnostics and `RedactedValue`) under-redacts a common credential-key family.
**Fix:** Add the spellings (or prefix/substring matching for the `*secret*`/
`*token*`/`*key*` families).

```yaml
id: glm53-031
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: security
severity: Minor
title: SecretRedactor exact-match field list gaps
claim: SecretRedactor matches sensitive field names by exact equality only, missing client_secret/secret_key/api-key/x-api-key/passwd and similar common spellings.
evidence:
  - location: crates/opi-agent/src/streaming_proxy.rs:453
    detail: Exact-match list of ten entries.
criterion_source: P17-FAL-004 / P17-EVD-005
reproduction: []
confidence: high
status: unverified
```

### 5.3 MINOR: Public `opi_ai::ProviderConfig` stores the API key as a plain `Option<String>` in a derived-`Debug` type

**File:** `crates/opi-ai/src/config.rs`
**Lines:** 11--13
**Cause:** `ProviderConfig` derives `Debug, Clone, Serialize, Deserialize` over an
`api_key: Option<String>`, so any Debug print, log, or serialization of a populated
config echoes the credential. The module is re-exported from the crate root and
currently has no workspace consumers.
**Impact:** Inconsistent with the `SecretString`/`SecretKey` discipline everywhere
else in the crate; a public footgun on a 0.x surface.
**Fix:** Wrap in `SecretString` (serde via expose-on-demand) or delete the unused
module.

```yaml
id: glm53-032
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: security
severity: Minor
title: ProviderConfig plain-string api_key with derived Debug
claim: The public ProviderConfig holds api_key as plain Option<String> inside a derived-Debug/Serialize type, echoing the credential on Debug or serialization.
evidence:
  - location: crates/opi-ai/src/config.rs:11-13
    detail: Plain String field with derived Debug/Serialize.
criterion_source: P17-FAL-004; AGENTS.md supply-chain/authority posture
reproduction: []
confidence: high
status: unverified
```

### 5.4 MINOR: Gemini, Azure, and Vertex copy the credential out of the zeroizing `SecretString` into plain `String`s for the request-task lifetime

**File:** `crates/opi-ai/src/gemini.rs`
**Lines:** 861 (and the Azure/Vertex equivalents)
**Cause:** These adapters call `expose_secret().to_string()` at `stream_prepared`
entry and move the plain `String` into the spawned HTTP task, so an un-zeroized
copy lives for the entire streaming request (potentially minutes). Anthropic and
the OpenAI family keep the zeroizing `SecretString` owned by the request path.
**Impact:** Memory-hygiene divergence on three wires; heap-resident credential
copies outlive the call.
**Fix:** Keep the `SecretString` and expose at the header-construction moment
only.

```yaml
id: glm53-033
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: security
severity: Minor
title: Plain-String secret copies in Gemini/Azure/Vertex adapters
claim: The Gemini, Azure, and Vertex adapters copy the prepared secret into plain Strings held for the spawned request-task lifetime instead of retaining the zeroizing SecretString.
evidence:
  - location: crates/opi-ai/src/gemini.rs:861
    detail: expose_secret().to_string() moved into the request task.
criterion_source: 17.1 DoD (secrecy-wrapped auth; drop discards without exposure)
reproduction: []
confidence: high
status: unverified
```

### 5.5 MINOR: Interactive TUI renders error strings without the Summary redaction every headless surface applies

**File:** `crates/opi-coding-agent/src/interactive.rs`
**Lines:** 766--783, 1120--1141
**Cause:** `PromptCompletion::Error(error.to_string())`, `CompactionEnd`
`error_message`, and `SessionPersistError { message }` are rendered into
user-visible messages directly; the same strings pass through
`RedactionMode::Summary` on the print/JSON/RPC paths.
**Impact:** Path-bearing and (for hook/tool-sourced errors) content-bearing text
reaches the interactive surface unredacted -- an inconsistency rather than a
proven leak, since most error Displays are already summary-sealed.
**Fix:** Apply the same `redact_text(.., Summary)` at the TUI render boundary.

```yaml
id: glm53-034
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: security
severity: Minor
title: Interactive TUI renders error strings unredacted
claim: The interactive surface renders prompt/compaction/session-persist error strings without the Summary redaction the headless modes apply to the same strings.
evidence:
  - location: crates/opi-coding-agent/src/interactive.rs:778-783
    detail: PromptCompletion::Error(error.to_string()).
  - location: crates/opi-coding-agent/src/interactive.rs:1120-1141
    detail: CompactionEnd/SessionPersistError messages rendered directly.
criterion_source: P17-FAL-004 / P17-A10
reproduction: []
confidence: medium
status: unverified
```

### 5.6 MINOR: `ask`-policy adapter dispatch rests on `authorized_backend` string and digest-shape guards

**File:** `crates/opi-coding-agent/src/execution/runtime.rs`
**Lines:** 459--475
**Cause:** When `request.authorized_backend` matches the selected adapter, an
`ask`-policy dispatch proceeds with no broker prompt and no session-grant check;
the in-crate guards on that field are a string-prefix and a workspace-digest shape
check. Within the product the field is populated from the parsed
`CommandPermissionScope` of a real `Allow`, so the chain holds as shipped.
**Impact:** The `ask` enforcement in the execution layer is only as strong as an
untyped request field; a future caller setting it bypasses the interactive gate
inside the runtime.
**Fix:** Type `authorized_backend` as a validated scope token (reuse
`CommandPermissionScope`) instead of a plain string field.

```yaml
id: glm53-035
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: security
severity: Minor
title: ask-policy dispatch guarded by untyped authorized_backend field
claim: The execution runtime dispatches ask-policy adapters without a broker when authorized_backend matches, and that field is guarded only by string-prefix and digest-shape checks rather than a typed scope.
evidence:
  - location: crates/opi-coding-agent/src/execution/runtime.rs:459-475
    detail: authorized_backend match short-circuits the ask path.
criterion_source: 17.4 DoD (bash reuses route selection/permission broker; no mutable fallback)
reproduction: []
confidence: medium
status: unverified
```

### 5.7 INFO: Conversation-echo boundary scrubs only recognized credential patterns

**File:** `crates/opi-agent/src/event.rs`
**Lines:** ~275
**Cause:** The public event boundary applies the pattern-based
`CONVERSATION_SECRET_REDACTOR` to user/assistant/tool-result text (a documented
transcript-preservation tradeoff), so an opaque secret without a recognized prefix
echoed in a tool result reaches subscribers.
**Impact:** Known, documented design tension between transcript fidelity and
scrubbing; not a P17 regression.
**Fix:** Revisit if tool outputs ever become an exfiltration surface of record.

```yaml
id: glm53-036
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: security
severity: Info
title: Conversation echo scrubs recognized patterns only
claim: The public event boundary scrubs conversation text by recognized secret patterns only, so unrecognized opaque secrets in tool results pass through by design.
evidence:
  - location: crates/opi-agent/src/event.rs:275
    detail: Pattern-based conversation scrubber with documented tradeoff.
criterion_source: P17-A10 scope
reproduction: []
confidence: medium
status: unverified
```

### 5.8 INFO: Credential read paths leave raw secret-bearing buffers un-zeroized

**File:** `crates/opi-coding-agent/src/credential_store.rs`
**Lines:** ~823 (and the Bedrock `credential_process` read)
**Cause:** The keychain read obtains the envelope JSON as a plain `String` and
drops it after decode (the write path uses `Zeroizing`), and the
`credential_process` path parses credentials out of a plain `Vec<u8>` dropped
un-zeroized -- despite the crate's documented zeroization posture for credential
material.
**Impact:** Defense-in-depth gap in memory hygiene only.
**Fix:** Use `Zeroizing` buffers on both read paths.

```yaml
id: glm53-037
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: security
severity: Info
title: Un-zeroized credential read buffers
claim: Keychain envelope reads and credential_process output buffers are dropped without zeroization, unlike the write path.
evidence:
  - location: crates/opi-coding-agent/src/credential_store.rs:823
    detail: Plain String envelope dropped after decode.
criterion_source: 17.1 DoD secrecy posture
reproduction: []
confidence: medium
status: unverified
```

### 5.9 INFO: `SecretKey` wraps live key material in a plain, non-zeroizing `String`

**File:** `crates/opi-ai/src/provider_collection.rs`
**Lines:** 58--76
**Cause:** The collection's static-key wrapper stores the credential as a plain
`String` with no drop zeroization, unlike `secrecy::SecretString` used elsewhere;
`Debug`/`Display` are redacted, so this is memory hygiene only.
**Impact:** Consistency gap in the crate's secrecy discipline.
**Fix:** Store a `SecretString` internally.

```yaml
id: glm53-038
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: security
severity: Info
title: SecretKey non-zeroizing storage
claim: SecretKey stores live key material in a plain non-zeroizing String despite redacted Debug/Display.
evidence:
  - location: crates/opi-ai/src/provider_collection.rs:58-76
    detail: Plain String inner value.
criterion_source: 17.1 DoD secrecy posture
reproduction: []
confidence: high
status: unverified
```

---

## 6. Test Quality Findings

The acceptance suite is unusually strong: A02/A03--A06, A08--A14 and the
high-risk criteria are exercised through real production call sites
(`Agent::prompt`, `CodingHarness`, real runners) with byte-level and count-based
assertions; isolation (temp dirs, serialized env) holds in the suites read; no
test calls paid providers or requires credentials (PLT-002 verified across the
read set). All 13 suites this auditor executed at HEAD are green (263 tests).

### 6.1 MAJOR: The A07 registration-forgery scenario is not exercised at any production call site, and the name-based builtin filter that enforces exclusion is unguarded

**File:** `crates/opi-coding-agent/tests/phase17_tool_authority.rs`
**Lines:** 414--457
**Cause:** The only test claiming P17-A07 coverage
(`phase17_untrusted_sources_cannot_forge_registration_or_grants` /
`phase17_extension_builtin_names_cannot_acquire_product_registrations`) registers
`MaliciousBuiltinNamesExtension` into an `ExtensionRegistry` and then passes
locally constructed product `RecordingTools` to `register_product_tools` -- the
extension's builtin-named tools are never in the input, so the exclusion filter is
not exercised and the `count == 0` assertion is vacuous (no agent runs). The
production safety is purely structural: `harness.rs:1545--1551` excludes extension
contributions before registration because only `build_tools`' builtin vector
reaches `register_product_tools`, and `register_builtin_tools`
(`tool_authority.rs:61--90`) assigns `ToolOrigin::Builtin` plus the builtin
capability **by name** to anything called read/write/bash regardless of true
origin. The origin-aware seam `collect_tools_with_origin`
(`extension.rs:410--428`), documented as the form trusted assembly must use
whenever origin is security-relevant, has zero production callers (only
`opi-agent/tests/extensions.rs` uses it). The behavioral assertions that do exist
(`harness_resource_integration.rs:690--729/778--817`) use a tool named
`test_tool`, which the name-based filter drops regardless of wiring, so they do
not guard the laundering path either.
**Impact:** If the harness wiring ever changed to pass extension tools into the
registration surface, an extension tool named `bash` would silently acquire
`Builtin` origin and the `command.execute` capability, and no phase-17 test would
fail. The A07 ledger claim ("met") rests on a test that proves the weaker grant-
and-content-forgery legs, not the registration leg.
**Fix:** (1) In the A07 test, actually route the malicious extension's
builtin-named `Tool` implementations through `register_product_tools` (or a
production harness assembly that includes them) and assert they are excluded or
denied with zero executions; (2) add a behavioral guard that origin is derived
from the registration path, not the name -- either use
`collect_tools_with_origin` in the product assembly or assert in
`register_builtin_tools` that inputs come from the builtin assembly seam.
Adversarially re-verified and confirmed at HEAD (with the narrowing that the
resource-integration behavioral tests exist but do not cover this path).

```yaml
id: glm53-039
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: test-quality
severity: Major
title: A07 registration-forgery test vacuous; name-based builtin filter unguarded
claim: The P17-A07 test never routes the malicious extension's builtin-named tools through the production registration surface, and the by-name builtin capability filter plus the structurally-excluded harness wiring have no behavioral guard, so builtin-name laundering would go undetected.
evidence:
  - location: crates/opi-coding-agent/tests/phase17_tool_authority.rs:414-457
    detail: ExtensionRegistry built but its tools never passed to register_product_tools; count==0 vacuous.
  - location: crates/opi-coding-agent/src/tool_authority.rs:61-90
    detail: builtin_capability matches by name; Builtin origin assigned regardless of source.
  - location: crates/opi-coding-agent/src/harness.rs:1545-1551
    detail: Exclusion is structural (only builtin vector reaches registration).
  - location: crates/opi-agent/src/extension.rs:410-428
    detail: collect_tools_with_origin has zero production callers.
criterion_source: P17-A07 / P17-AUT-001 / P17-RBK-001 (extension tool from trust alone)
reproduction:
  - cargo test -p opi-coding-agent --test phase17_tool_authority phase17_untrusted_sources_cannot_forge_registration_or_grants
confidence: high
status: unverified
```

### 6.2 MINOR: FAL-002 precedence row "invalid schema never reaches the authorizer" has no authorizer-call-count assertion

**File:** `crates/opi-agent/tests/tool_validation.rs`
**Lines:** ~640
**Cause:** The implemented schema-before-authorizer ordering
(agent_loop.rs:1768--1821) has no direct regression test: `tool_validation.rs`
uses a non-counting permissive authorizer, and counting authorizers are only used
with valid arguments.
**Impact:** A reorder of the hook/schema/authorizer sequence would not fail this
row of the composed FAL-002 matrix.
**Fix:** Add a schema-failure case with a counting authorizer asserting zero
invocations.

```yaml
id: glm53-040
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: test-quality
severity: Minor
title: Missing authorizer-count assertion for schema failure
claim: No test counts authorizer invocations when schema validation fails, so the FAL-002 schema-before-authorizer row is unasserted.
evidence:
  - location: crates/opi-agent/tests/tool_validation.rs:640
    detail: Non-counting authorizer with invalid-argument cases.
criterion_source: P17-FAL-002
reproduction: []
confidence: high
status: unverified
```

### 6.3 MINOR: Two manifest graph-rule tests are overdetermined -- their fixtures fail kind/payload validation before the named rule runs

**File:** `crates/opi-agent/tests/evidence_contract.rs`
**Lines:** 1037--1086 (and the Retry+Digest fixture at 581--586)
**Cause:** `repeated_call_record_cannot_change_kind` and
`..._cannot_change_parent` construct fixtures that already violate
`validate_kind_payload` (Tool+Digest and Tool-kind-with-Provider-payload), and
`validate_observation` runs the kind/payload check first per record
(evidence.rs:1894--1895), so the named shared-call rules never execute; the
Retry-with-Digest linkage fixture is likewise a record no valid graph can contain.
**Impact:** The named rules are tested vacuously via a different failure; a real
regression in the kind/turn/parent stability rule would still be caught only
accidentally.
**Fix:** Build fixtures with valid kind/payload pairs that differ only in the
field under test.

```yaml
id: glm53-041
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: test-quality
severity: Minor
title: Overdetermined manifest graph-rule fixtures
claim: evidence_contract graph-rule tests use fixtures that fail kind/payload validation before the named rules run, making those assertions vacuous.
evidence:
  - location: crates/opi-agent/tests/evidence_contract.rs:1037-1086
    detail: Fixtures violate validate_kind_payload; observation checks kind first.
  - location: crates/opi-agent/tests/evidence_contract.rs:581-586
    detail: Retry+Digest record impossible in a valid graph.
criterion_source: P17-EVD-003 / P17-FAL-001
reproduction: []
confidence: high
status: unverified
```

### 6.4 MINOR: `store_credential_dispatch_*` tests never construct a `StoreCredential` route, inject a probe, or exercise the probe path

**File:** `crates/opi-ai/tests/provider_collection.rs`
**Lines:** 713--760
**Cause:** The four tests named for the store-credential probe semantics all use
`register_route` (which removes the auth descriptor), never build an
`AuthDescriptor::StoreCredential` entry, never call `set_probe`, and only one
distinguishes anything about probes at all.
**Impact:** The names claim dispatch-gate coverage of the probe contract that the
bodies do not provide; the actual probe behavior is only covered indirectly via
coding-agent listing tests.
**Fix:** Either rename to what they test (static resolver dispatch outcomes) or
add real StoreCredential/probe cases.

```yaml
id: glm53-042
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: test-quality
severity: Minor
title: store_credential_dispatch tests do not exercise the store-credential path
claim: The store_credential_dispatch_* tests never construct StoreCredential descriptors or inject probes, so their names overstate coverage.
evidence:
  - location: crates/opi-ai/tests/provider_collection.rs:713-760
    detail: register_route-based tests with no StoreCredential/probe usage.
criterion_source: P17-PRV-001 (dispatch gate coverage)
reproduction: []
confidence: high
status: unverified
```

### 6.5 MINOR: Factory-built extra dispatch routes are never resolved or dispatched in tests

**File:** `crates/opi-coding-agent/tests/provider_factory.rs`
**Lines:** 343--388
**Cause:** The only test of eagerly-built extra routes asserts provider-id
presence; no test passes factory-built extra routes (real wire adapters over
production catalogs) through `build_harness_collection` and prepares a
cross-provider model on them. Cross-provider dispatch is proven only through the
mock builder seam (`phase17_provider_runtime.rs:374--476`).
**Impact:** The production seam `config -> build_provider_bundle -> extra_routes
-> build_harness_collection -> prepare_call` is covered end-to-end only with
mocks; a wiring regression in the real factory path (e.g. a missing resolver for
an extra route) would surface first in production use.
**Fix:** Add one factory-level test that prepares (not necessarily dispatches
HTTP for) a real extra route through the production collection.

```yaml
id: glm53-043
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: test-quality
severity: Minor
title: Factory extra routes never dispatched in tests
claim: No test resolves or prepares a model on a factory-built extra dispatch route through build_harness_collection; only mock-seam extra routes are dispatched.
evidence:
  - location: crates/opi-coding-agent/tests/provider_factory.rs:343-388
    detail: Presence-only assertion on eagerly-built routes.
criterion_source: P17-OUT-001 / P17-PRV-001
reproduction: []
confidence: high
status: unverified
```

### 6.6 MINOR: Two `json_mode` tests drop the env-isolation guard before running the agent

**File:** `crates/opi-coding-agent/tests/json_mode.rs`
**Lines:** ~1546
**Cause:** Two tests drop the env guard contrary to the invariant sibling tests
document ("the redirected config dir (and its tempdir) must remain alive for the
session-persist step"), risking session writes outside the temp dir during those
tests.
**Impact:** Potential cross-test/host-state leakage in exactly the mode whose
canary tests assert cleanliness.
**Fix:** Keep the guard alive across the run as the sibling tests do.

```yaml
id: glm53-044
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: test-quality
severity: Minor
title: json_mode tests drop the env-isolation guard early
claim: Two json_mode tests drop the env guard before running the agent, contrary to the documented isolation invariant of the sibling tests.
evidence:
  - location: crates/opi-coding-agent/tests/json_mode.rs:1546
    detail: Guard dropped before the agent run.
criterion_source: AGENTS.md (isolated temp directories; serialized env mutation)
reproduction: []
confidence: medium
status: unverified
```

### 6.7 INFO: Only-public-mutation-surface guarantee enforced by raw substring scan

**File:** `crates/opi-agent/tests/agent_wrapper.rs`
**Lines:** 8+
**Cause:** `complete_state_is_the_only_public_next_turn_mutation_surface` greps
the source via `include_str` + `contains` for removed setter names, rather than
the token-level scan `phase17_api_audit` uses; it misses reformatted signatures and
false-positives on doc text.
**Impact:** The NXT/MIG-006 removal claim's behavioral backstop is
formatting-sensitive (the api-audit's lexer scan is the stronger guard).
**Fix:** Reuse the lexer-based scanner.

```yaml
id: glm53-045
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: test-quality
severity: Info
title: Mutation-surface guard uses substring scan
claim: The only-public-mutation-surface test enforces the claim with raw substring matching over source text instead of the token-level scan used elsewhere.
evidence:
  - location: crates/opi-agent/tests/agent_wrapper.rs:8
    detail: include_str/contains based assertion.
criterion_source: P17-MIG-006 / P17-NXT-001
reproduction: []
confidence: high
status: unverified
```

### 6.8 INFO: Offline SSE fixture paths cannot regression-test the terminal-missing fail-closed contract

**File:** `crates/opi-ai/src/anthropic.rs`
**Lines:** ~922 (and the Gemini/Azure/Vertex fixture helpers)
**Cause:** The offline `stream_from_sse`-style helpers iterate all parsed events
without the HTTP paths' terminal-missing guard, so a fixture body lacking a
terminal event simply ends the stream early instead of erroring.
**Impact:** The fail-closed terminal contract is exercised only through wiremock
HTTP tests; fixture-based regressions would pass silently.
**Fix:** Apply the same end-without-terminal error in the fixture helpers.

```yaml
id: glm53-046
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: test-quality
severity: Info
title: Offline fixture paths skip terminal-missing fail-closed
claim: The offline SSE fixture helpers for Anthropic/Gemini/Azure/Vertex do not fail close on bodies lacking a terminal event, unlike their HTTP paths.
evidence:
  - location: crates/opi-ai/src/anthropic.rs:922
    detail: Fixture path iterates events without a terminal guard.
criterion_source: P17-FAL spirit (stream fail-closed)
reproduction: []
confidence: medium
status: unverified
```

### 6.9 INFO: Cross-mode canonical-route assertion uses `ends_with` rather than equality

**File:** `crates/opi-coding-agent/tests/phase17_cross_mode.rs`
**Lines:** ~256
**Cause:** The canonical-route assertion matches `request.model` with
`ends_with("alpha-model")`, so any `provider:model` whose model id merely ends in
`alpha-model` would pass.
**Impact:** Loose matcher weakens the A14 equivalence assertion.
**Fix:** Assert the full canonical spec.

```yaml
id: glm53-047
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: test-quality
severity: Info
title: ends_with matcher in cross-mode route assertion
claim: The cross-mode test matches the canonical route with ends_with(alpha-model) instead of equality.
evidence:
  - location: crates/opi-coding-agent/tests/phase17_cross_mode.rs:256
    detail: ends_with matcher.
criterion_source: P17-A14 / P17-PRV-002
reproduction: []
confidence: high
status: unverified
```

### 6.10 INFO: `main.rs` self-referential include_str counts require manual resynchronization

**File:** `crates/opi-coding-agent/src/main.rs`
**Lines:** ~3197
**Cause:** Production `main.rs` pins hand-counted occurrences of subprocess launch
sites via `include_str!` scans over test sources; adding or removing any launch
site requires manually re-bumping the counts.
**Impact:** Brittle coupling from production source to test-file content.
**Fix:** Count programmatically or move the assertion into the owning test.

```yaml
id: glm53-048
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: test-quality
severity: Info
title: Hand-pinned include_str occurrence counts in main.rs
claim: main.rs asserts hand-pinned occurrence counts over test-source text, requiring manual resynchronization on every launch-site change.
evidence:
  - location: crates/opi-coding-agent/src/main.rs:3197
    detail: include_str scan with literal counts.
criterion_source: null
reproduction: []
confidence: medium
status: unverified
```

### 6.11 INFO: Interactive `--trace` production wiring inside `run_interactive_core` is not exercised through the tested core function

**File:** `crates/opi-coding-agent/src/main.rs`
**Lines:** ~1273
**Cause:** Interactive durable-evidence activation is covered only via direct
`CodingHarness` builder construction; the production `run_interactive_core` wiring
itself is untested (its canary leg runs through the mock harness path).
**Impact:** A regression in the interactive activation branch (the fixed
complete-evidence mapping's third input) would not fail a local test.
**Fix:** Factor the activation into the tested helper the other modes use.

```yaml
id: glm53-049
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: test-quality
severity: Info
title: Interactive --trace wiring untested at the production core
claim: The interactive --trace activation inside run_interactive_core has no test through the production core function; coverage is via direct builder construction.
evidence:
  - location: crates/opi-coding-agent/src/main.rs:1273
    detail: Untested production activation branch.
criterion_source: P17-EVD-006 / 17.7 fixed mapping
reproduction: []
confidence: medium
status: unverified
```

### 6.12 INFO: MIG-006 token scan excludes `build.rs`, examples, and benches

**File:** `crates/opi-coding-agent/tests/phase17_api_audit.rs`
**Lines:** ~2452
**Cause:** The removal audit's `production_sources()` scans `crates/*/src` only;
the alias/shim rows are delegated to other tests by design, and no scan root covers
build scripts, examples, or benches.
**Impact:** A shim reintroduced outside `src/` would evade the audit; low likelihood
given the current tree (no such files reference the symbols -- verified by this
auditor's workspace grep).
**Fix:** Extend the scan roots or document the exclusion.

```yaml
id: glm53-050
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: test-quality
severity: Info
title: MIG-006 scan scope excludes build/examples/benches
claim: The removed-interface token scan covers crates/*/src only, leaving build.rs, examples, and benches outside the audited root.
evidence:
  - location: crates/opi-coding-agent/tests/phase17_api_audit.rs:2452
    detail: production_sources() roots at crates/*/src.
criterion_source: P17-MIG-006
reproduction: []
confidence: high
status: unverified
```

### 6.13 INFO: Bash authorized-execution path has no in-file unit coverage

**File:** `crates/opi-coding-agent/src/tool/bash.rs`
**Lines:** ~560
**Cause:** Only argument validation and the wait-failed builder are tested
in-file; the `execute_authorized` override (which parses the permission scope and
binds the adapter) is covered only through the harness-level authority tests.
**Impact:** The scope-parse/binding logic has no narrow regression test.
**Fix:** Add unit cases for scope parse/cover decisions.

```yaml
id: glm53-051
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: test-quality
severity: Info
title: Bash authorized-execution override lacks unit coverage
claim: bash.rs tests only validation and wait-failure paths; the execute_authorized scope parsing/binding has no in-file unit test.
evidence:
  - location: crates/opi-coding-agent/src/tool/bash.rs:560
    detail: In-file tests skip the authorized path.
criterion_source: P17-AUT (adapter binding)
reproduction: []
confidence: medium
status: unverified
```

### 6.14 INFO: `safety_hooks` tests mutate process-global env with `unsafe set_var` inside async tests

**File:** `crates/opi-coding-agent/tests/safety_hooks.rs`
**Lines:** ~34
**Cause:** `OPI_SESSIONS_DIR` is set via `unsafe { std::env::set_var }` inside
async tokio tests, serialized only by an in-file mutex.
**Impact:** UB risk per the std contract on concurrent env access; contained by
the mutex within the binary but fragile.
**Fix:** Use `temp_env`-style scoped guards or run on a blocking thread.

```yaml
id: glm53-052
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: test-quality
severity: Info
title: unsafe set_var in async safety_hooks tests
claim: safety_hooks.rs mutates OPI_SESSIONS_DIR via unsafe set_var inside async tests with only in-file serialization.
evidence:
  - location: crates/opi-coding-agent/tests/safety_hooks.rs:34
    detail: unsafe set_var in async context.
criterion_source: AGENTS.md (serialize env-mutating tests; safe Rust preference)
reproduction: []
confidence: medium
status: unverified
```

### 6.15 INFO: Test-support `models()` Box-leaks a fresh catalog copy per call

**File:** `crates/opi-ai/tests/provider_collection.rs`
**Lines:** ~953
**Cause:** `RefreshProvider::models` Box-leaks a new clone of the catalog on every
call while the comment calls the leak "bounded"; repeated lookups grow the leak
per call within a test binary.
**Impact:** Test-binary memory growth only.
**Fix:** Leak once (`static`/`OnceLock`) or return a slice.

```yaml
id: glm53-053
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: test-quality
severity: Info
title: Per-call Box leak in test-support models()
claim: RefreshProvider::models leaks a fresh catalog clone on every call despite a comment claiming the leak is bounded.
evidence:
  - location: crates/opi-ai/tests/provider_collection.rs:953
    detail: Box::leak per models() call.
criterion_source: null
reproduction: []
confidence: high
status: unverified
```

---

## 7. Cross-task Integration Findings

Verified clean: the 17.1 expand -> 17.5 contract removal is complete (no dual
dispatch, no resolver-bearing adapters, no `stream(Request)` bridge anywhere); the
17.3 -> 17.6 -> 17.7 evidence chain has one lifecycle with no dual trace/evidence
path (legacy exports gone from core); 17.4's authority and 17.7's evidence-driven
reauthorization compose through the shared generation check; error vocabularies are
consistent across the three crates; and the Reference Product file adapter and the
in-memory oracle share the one conformance contract.

### 7.1 MINOR: `credential_process` declared in the AWS *credentials* file is silently ignored

**File:** `crates/opi-ai/src/bedrock/credentials.rs`
**Lines:** ~286
**Cause:** Only the shared *config* file's `credential_process` is ever executed; a
declaration in `~/.aws/credentials` (a valid AWS location for the directive) is not
consulted, so otherwise-valid setups resolve to `Exhausted`.
**Impact:** Silent divergence from the documented AWS behavior for a class of
real-world configurations; surfaces as a confusing missing-credential failure.
**Fix:** Read `credential_process` from the credentials file profile too, or emit
a diagnostic naming the ignored directive.

```yaml
id: glm53-054
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: integration
severity: Minor
title: credential_process in credentials file ignored
claim: The Bedrock resolver honors credential_process only from the config file, silently ignoring the same directive in the credentials file.
evidence:
  - location: crates/opi-ai/src/bedrock/credentials.rs:286
    detail: Config-file-only credential_process consultation.
criterion_source: P17-PRV-004 (typed auth failure, no silent miss)
reproduction: []
confidence: medium
status: unverified
```

### 7.2 MINOR: Environment-fallback diagnostic is emitted only for the active provider

**File:** `crates/opi-coding-agent/src/provider_factory.rs`
**Lines:** ~1336
**Cause:** The backend-unavailable env-fallback warning fires only on the active
provider's construction; extra dispatch routes that silently fall back to
environment credentials after keychain loss produce no diagnostic.
**Impact:** The user is not told a whole route runs on env credentials -- a
visibility gap in exactly the fallback case PRV-004 requires to be retained as a
fact (the typed provenance is kept; the proactive diagnostic is not).
**Fix:** Emit the fallback diagnostic per route that falls back.

```yaml
id: glm53-055
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: integration
severity: Minor
title: Env-fallback diagnostic active-provider only
claim: The keychain-loss env-fallback warning is emitted only for the active provider; extra routes fall back silently.
evidence:
  - location: crates/opi-coding-agent/src/provider_factory.rs:1336
    detail: Warning scoped to active-route construction.
criterion_source: P17-PRV-004 (allowed fallback retains typed reason and visibility)
reproduction: []
confidence: medium
status: unverified
```

### 7.3 MINOR: Keyring backend I/O runs as synchronous blocking calls on tokio workers

**File:** `crates/opi-coding-agent/src/credential_store.rs`
**Lines:** 810--871 (calls), 902--906 (construction note)
**Cause:** Store construction is offloaded to `spawn_blocking`, but subsequent
`.get()`/`.set()` calls during `prepare_call` auth resolution and store mutation
run inline on runtime worker threads. On Linux (Secret Service via zbus) a slow or
hung keychain blocks a worker.
**Impact:** Runtime stalls during auth preparation under platform-keychain
latency; the fs4 lock path already documents and mitigates the same class.
**Fix:** Wrap backend calls in `spawn_blocking` at the async seam.

```yaml
id: glm53-056
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: integration
severity: Minor
title: Blocking keyring I/O on async workers
claim: Keyring get/set calls run synchronously on tokio workers inside async auth resolution, so a slow platform keychain blocks runtime workers.
evidence:
  - location: crates/opi-coding-agent/src/credential_store.rs:810-871
    detail: Inline backend.get/set in async paths; only construction is spawn_blocking (902-906).
criterion_source: AGENTS.md (bounded concurrency posture)
reproduction: []
confidence: medium
status: unverified
```

### 7.4 MINOR: RPC `trace` command lacks the `agent_busy` guard its sibling read command enforces

**File:** `crates/opi-coding-agent/src/rpc.rs`
**Lines:** ~1008
**Cause:** `session_info` rejects with `ERR_AGENT_BUSY` while a run is active, but
`trace` returns an undocumented mid-run snapshot with no guard.
**Impact:** Inconsistent read-only-command semantics (MIG-005's consistency goal at
the command layer); a client can observe a partial capture state not documented as
a stable output.
**Fix:** Either guard `trace` like `session_info` or document the mid-run snapshot
contract.

```yaml
id: glm53-057
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: integration
severity: Minor
title: RPC trace lacks agent-busy guard
claim: The RPC trace command returns a mid-run snapshot without the ERR_AGENT_BUSY guard the sibling session_info command enforces.
evidence:
  - location: crates/opi-coding-agent/src/rpc.rs:1008
    detail: Unguarded trace handler.
criterion_source: P17-MIG-005 (mode consistency)
reproduction: []
confidence: medium
status: unverified
```

### 7.5 MINOR: `grep` and `glob` embed the absolute workspace root in public `details.workspace_root`

**File:** `crates/opi-coding-agent/src/tool/grep.rs`
**Lines:** ~190 (and the glob equivalent)
**Cause:** The navigation tools place the resolved absolute workspace root into
the public tool-result `details` field, contradicting the stated invariant that
the absolute root must not leak to the model or into public NDJSON details (the
Summary redactor's path scrubbing covers diagnostics, not this structured field).
**Impact:** Host filesystem structure is disclosed in public outputs across all
modes.
**Fix:** Drop the field or replace with a workspace-relative marker.

```yaml
id: glm53-058
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: integration
severity: Minor
title: workspace_root leaked in grep/glob public details
claim: grep and glob include the absolute workspace root in public details.workspace_root contrary to the stated no-leak invariant.
evidence:
  - location: crates/opi-coding-agent/src/tool/grep.rs:190
    detail: workspace_root in public details.
criterion_source: P17-FAL-004 spirit (no unclassified environment disclosure)
reproduction: []
confidence: medium
status: unverified
```

### 7.6 INFO: `openai_compatible` availability is env-only on the listing path while the runtime resolver is keychain-first

**File:** `crates/opi-coding-agent/src/provider_factory.rs`
**Lines:** ~918
**Cause:** Listing/auth-status classification for `openai_compatible` profiles
checks environment presence only, while the runtime resolver for the same provider
id tries the keychain first.
**Impact:** `--list-models` can hide a profile the runtime would authenticate from
the store -- an existing instance of the parallel-table drift 2.7 warns about.
**Fix:** Share one availability probe between the tables.

```yaml
id: glm53-059
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: integration
severity: Info
title: Listing/runtime availability divergence for openai_compatible
claim: openai_compatible listing availability is env-only while the runtime resolver is keychain-first, so --list-models can hide an authenticatable profile.
evidence:
  - location: crates/opi-coding-agent/src/provider_factory.rs:918
    detail: Env-only listing classification.
criterion_source: P17-PRV-001 (collection-owned consistent seams)
reproduction: []
confidence: medium
status: unverified
```

### 7.7 INFO: RPC final event drain ignores emit failures and still exits `Success`

**File:** `crates/opi-coding-agent/src/rpc.rs`
**Lines:** ~562
**Cause:** After quit/EOF the final drain discards emit errors and the runner
exits with Success, unlike the in-loop emit-failure path which returns
RuntimeFailure.
**Impact:** Tail-loss of the final events is silently swallowed at one boundary --
a small FAL-003 inconsistency.
**Fix:** Propagate drain failures into the exit status.

```yaml
id: glm53-060
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: integration
severity: Info
title: RPC final drain ignores emit failures
claim: The RPC runner's post-quit event drain discards emission failures and exits Success.
evidence:
  - location: crates/opi-coding-agent/src/rpc.rs:562
    detail: Errors ignored in the final drain.
criterion_source: P17-FAL-003
reproduction: []
confidence: medium
status: unverified
```

### 7.8 INFO: Codex adapter injects a default system instruction when none is supplied

**File:** `crates/opi-ai/src/openai_codex_responses.rs`
**Lines:** ~78
**Cause:** The Codex adapter substitutes "You are a helpful assistant." whenever
the caller supplies no system instruction, unlike every other adapter which sends
no system field.
**Impact:** Silent per-wire semantic divergence for system-less requests on one
provider.
**Fix:** Send no system instruction, or surface the substitution as model metadata.

```yaml
id: glm53-061
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: integration
severity: Info
title: Codex default system instruction injection
claim: The Codex adapter injects a default system instruction when none is supplied, diverging from the other adapters' no-system behavior.
evidence:
  - location: crates/opi-ai/src/openai_codex_responses.rs:78
    detail: Default instruction substitution.
criterion_source: P17-PRV-006 (neutral interfaces, consistent semantics)
reproduction: []
confidence: medium
status: unverified
```

### 7.9 INFO: Anthropic mapper emits `content_index` from its own block count, deltas address by upstream index

**File:** `crates/opi-ai/src/anthropic.rs`
**Lines:** ~424
**Cause:** `ContentBlockStart` emits the mapper's own running block count as
`content_index` while deltas/stops address blocks by the upstream index; the two
agree only for sequential zero-based upstream streams.
**Impact:** Providers emitting sparse or non-zero-based block indices (none known
today) would produce internally inconsistent event indices.
**Fix:** Use the upstream index for start events too.

```yaml
id: glm53-062
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: integration
severity: Info
title: Anthropic content_index divergence on sparse indices
claim: The Anthropic mapper mixes its own block count into ContentBlockStart while addressing deltas by upstream index, diverging for non-sequential upstream streams.
evidence:
  - location: crates/opi-ai/src/anthropic.rs:424
    detail: Own-count index on start events.
criterion_source: P17-PRV-006
reproduction: []
confidence: low
status: unverified
```

---

## 8. Residuals

### 8.1 MINOR: `terminate_and_fail` discards streaming diagnostics accumulated before a mid-stream protocol violation

**File:** `crates/opi-coding-agent/src/execution/protocol_host.rs`
**Lines:** ~1337
**Cause:** The mid-stream violation terminal drops the backend diagnostics
collected up to that point, unlike the terminal path which preserves them.
**Impact:** Diagnostics loss on exactly the failure path operators need them for.
**Fix:** Carry the accumulated diagnostics into the failure.

```yaml
id: glm53-063
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: residuals
severity: Minor
title: Mid-stream protocol violation discards prior diagnostics
claim: terminate_and_fail drops diagnostics accumulated before a mid-stream protocol violation.
evidence:
  - location: crates/opi-coding-agent/src/execution/protocol_host.rs:1337
    detail: Accumulated diagnostics not forwarded.
criterion_source: P17-FAL-003 (failures stay observable)
reproduction: []
confidence: medium
status: unverified
```

### 8.2 MINOR: Pre-spawn handshake-deadline expiry is classified as `protocol_violation`

**File:** `crates/opi-coding-agent/src/execution/protocol_host.rs`
**Lines:** ~282
**Cause:** A host-side deadline expiring before any protocol traffic is
classified with the protocol-violation code, misreporting a timeout as a backend
protocol fault.
**Impact:** Wrong remediation surface in diagnostics (host slowness vs backend
misbehavior).
**Fix:** Add a distinct handshake-timeout classification.

```yaml
id: glm53-064
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: residuals
severity: Minor
title: Handshake deadline misclassified as protocol violation
claim: Pre-spawn handshake-deadline expiry is reported as protocol_violation although no protocol traffic occurred.
evidence:
  - location: crates/opi-coding-agent/src/execution/protocol_host.rs:282
    detail: Deadline arm reuses the protocol-violation code.
criterion_source: P17-FAL-001 (distinguishable failure classes)
reproduction: []
confidence: medium
status: unverified
```

### 8.3 INFO: Session-CLI e2e binary fallback is unreachable dead code; suite hard-requires a prebuilt binary

**File:** `crates/opi-coding-agent/tests/session_cli.rs`
**Lines:** ~835--852
**Cause:** `build_opi_if_needed` calls `opi_binary()` (which panics when
`target/debug/opi` is absent) before its `if !bin.exists()` branch, so the
cargo-build fallback can never run, and the suite genuinely hard-requires the
prebuilt binary at a fixed shared path.
**Impact:** Host-state-dependent suite (known operational quirk); the fallback
suggests a robustness it does not provide.
**Fix:** Reorder the existence check, or document the prebuilt requirement.

```yaml
id: glm53-065
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: residuals
severity: Info
title: session_cli e2e build fallback unreachable
claim: The e2e helper panics on a missing binary before its build branch, leaving the fallback dead and the suite hard-dependent on a prebuilt target/debug/opi.
evidence:
  - location: crates/opi-coding-agent/tests/session_cli.rs:852
    detail: opi_binary() called before the exists() branch.
criterion_source: null
reproduction: []
confidence: high
status: unverified
```

### 8.4 INFO: Fork session creation is non-transactional with a TOCTOU existence check

**File:** `crates/opi-coding-agent/src/session_cli.rs`
**Lines:** ~816
**Cause:** Fork uses exists()-then-create id allocation; a crash mid-copy leaves a
partial fork file and the existence probe can race a concurrent creator.
**Impact:** Rare partial-fork artifacts; the collision loop on the id softens but
does not remove the race.
**Fix:** Create exclusively (`create_new`) and copy through a temp file + rename.

```yaml
id: glm53-066
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: residuals
severity: Info
title: Non-transactional fork creation
claim: Fork session creation uses a TOCTOU existence check and non-atomic copy, allowing partial fork files after a crash.
evidence:
  - location: crates/opi-coding-agent/src/session_cli.rs:816
    detail: exists()-then-create plus direct copy.
criterion_source: INV-007 (crash recovery)
reproduction: []
confidence: medium
status: unverified
```

### 8.5 INFO: Codex login-method selection can block up to the later of two 15-minute deadlines

**File:** `crates/opi-coding-agent/src/oauth.rs`
**Lines:** ~1367
**Cause:** Method selection uses the later of the browser and device-code
deadlines as its timeout, so the interactive "Select login method" prompt can hang
for up to 15 minutes before timing out.
**Impact:** Poor interactive ergonomics; no correctness impact.
**Fix:** Use a short fixed prompt deadline.

```yaml
id: glm53-067
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: residuals
severity: Info
title: Login-method prompt bounded by flow deadlines
claim: The Codex login-method selection prompt times out only at the later of the browser/device 15-minute deadlines.
evidence:
  - location: crates/opi-coding-agent/src/oauth.rs:1367
    detail: Later-of deadline used for the selection prompt.
criterion_source: null
reproduction: []
confidence: medium
status: unverified
```

### 8.6 INFO: Navigation tools perform synchronous directory walks and file reads on the async runtime

**File:** `crates/opi-coding-agent/src/tool/grep.rs`
**Lines:** ~89 (and read/ls/find/glob)
**Cause:** The four navigation tools do blocking filesystem work directly inside
async tool execution without `spawn_blocking`.
**Impact:** A large workspace blocks tokio workers during navigation bursts.
**Fix:** Wrap heavy walks in `spawn_blocking`.

```yaml
id: glm53-068
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: residuals
severity: Info
title: Sync navigation I/O on the async runtime
claim: grep/read/ls/find/glob perform synchronous walks and reads without spawn_blocking, blocking tokio workers on large workspaces.
evidence:
  - location: crates/opi-coding-agent/src/tool/grep.rs:89
    detail: Inline blocking traversal.
criterion_source: null
reproduction: []
confidence: high
status: unverified
```

### 8.7 INFO: Truncated-command merged output temp files accumulate in the OS temp directory

**File:** `crates/opi-coding-agent/src/tool/operations.rs`
**Lines:** ~1241
**Cause:** Merged full-output temp files for truncated commands are created under
the OS temp dir with no reaping by opi.
**Impact:** Unbounded temp growth over long sessions.
**Fix:** Register files for deletion-on-drop or reap on session end.

```yaml
id: glm53-069
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: residuals
severity: Info
title: Merged-output temp files never reaped
claim: Truncated-command merged output temp files accumulate with no explicit cleanup.
evidence:
  - location: crates/opi-coding-agent/src/tool/operations.rs:1241
    detail: Temp merge files left in place.
criterion_source: null
reproduction: []
confidence: medium
status: unverified
```

### 8.8 INFO: `ls` can render a contradictory "(truncated, 0 entries omitted)" line

**File:** `crates/opi-coding-agent/src/tool/ls.rs`
**Lines:** ~204
**Cause:** When early termination fires before the entry cap fills, the summary
line reports zero omitted entries alongside "truncated".
**Impact:** Cosmetic contradiction in tool output.
**Fix:** Suppress the line when nothing was omitted.

```yaml
id: glm53-070
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: residuals
severity: Info
title: ls truncated-0 summary line
claim: ls can emit "(truncated, 0 entries omitted)" when early termination fires before the cap.
evidence:
  - location: crates/opi-coding-agent/src/tool/ls.rs:204
    detail: Unconditional truncated summary line.
criterion_source: null
reproduction: []
confidence: medium
status: unverified
```

### 8.9 INFO: Routed exit codes narrowed with an unchecked `u32 as i32` cast

**File:** `crates/opi-coding-agent/src/execution/runtime.rs`
**Lines:** ~720
**Cause:** Exit codes above `i32::MAX` wrap negative through the unchecked cast.
**Impact:** Cosmetic misreporting for astronomical exit codes only.
**Fix:** Clamp or carry u32.

```yaml
id: glm53-071
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: residuals
severity: Info
title: Unchecked exit-code narrowing cast
claim: Routed exit codes are narrowed with u32 as i32, wrapping for values above i32::MAX.
evidence:
  - location: crates/opi-coding-agent/src/execution/runtime.rs:720
    detail: Unchecked cast.
criterion_source: null
reproduction: []
confidence: medium
status: unverified
```

### 8.10 INFO: `JobGuard::terminate_with` retains dead error-handling code

**File:** `crates/opi-coding-agent/src/tool/process_tree.rs`
**Lines:** ~542
**Cause:** `let error = last_os_error(); let _ = error;` -- captured and discarded.
**Impact:** Noise; suggests handling that does not exist.
**Fix:** Delete the two lines.

```yaml
id: glm53-072
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: residuals
severity: Info
title: Dead error capture in JobGuard
claim: JobGuard::terminate_with captures last_os_error and discards it without use.
evidence:
  - location: crates/opi-coding-agent/src/tool/process_tree.rs:542
    detail: let error = ...; let _ = error;
criterion_source: null
reproduction: []
confidence: high
status: unverified
```

### 8.11 INFO: `SessionWriter::open` recovers an incomplete tail with a one-syscall-per-byte backward scan

**File:** `crates/opi-agent/src/session.rs`
**Lines:** ~388
**Cause:** Crash-tail recovery seeks and reads one byte at a time backwards.
**Impact:** O(tail-length) syscalls on every reopen after a crash; correctness
unaffected.
**Fix:** Chunked backward reads.

```yaml
id: glm53-073
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: residuals
severity: Info
title: Byte-at-a-time crash-tail scan
claim: SessionWriter::open scans backward one byte per syscall to recover an incomplete tail.
evidence:
  - location: crates/opi-agent/src/session.rs:388
    detail: Per-byte seek+read loop.
criterion_source: null
reproduction: []
confidence: medium
status: unverified
```

### 8.12 INFO: Steering/follow-up queues are unbounded with no closure semantics; the "queue closure/overflow" matrix rows delegate to owner paths

**File:** `crates/opi-agent/src/agent.rs` (queue fields), `crates/opi-coding-agent/tests/phase17_failure_rollback.rs:5,551`
**Cause:** The steering/follow-up queues are unbounded `VecDeque`s with no close
or capacity semantics; the 17.9 precedence matrix satisfies its "queue closure/
overflow" rows by delegating to the Phase-8/12 owner paths (StreamingProxy's
bounded channels), as its header documents.
**Impact:** INV-006's bounded-queue posture is carried by the proxy boundary, not
the steering queues; no overflow can occur there, so nothing is silently
converted (FAL-003 holds vacuously for these queues).
**Fix:** Document the owner split at the Agent's queue fields, or add bounds if
steering ever becomes a producer-facing surface.

```yaml
id: glm53-074
source_kind: audit
source_path: docs/snapshots/phase17/audit.glm5.3.md
source_model: glm5.3
independence: fresh-context-same-family
axis: residuals
severity: Info
title: Unbounded steering queues; closure/overflow delegated to owner paths
claim: Steering/follow-up queues are unbounded with no closure semantics, and the failure matrix's queue rows delegate to StreamingProxy bounded-overflow owner paths.
evidence:
  - location: crates/opi-agent/src/agent.rs:559-560
    detail: Unbounded VecDeque queues.
  - location: crates/opi-coding-agent/tests/phase17_failure_rollback.rs:5,551
    detail: Matrix header delegates queue closure/overflow to Phase 8/12 owners.
criterion_source: INV-006 / P17-FAL-003
reproduction: []
confidence: high
status: unverified
```

---

## 9. Invariant Verification

| Invariant | Code evidence | Test coverage |
|-----------|--------------|---------------|
| INV-001 selection resolves the real provider at runtime | `ProviderCollection::prepare_call` resolves+freezes route per call (provider_collection.rs:410-479); cross-provider switch without reconstruction (harness set_model_validated) | `phase17_provider_runtime::phase17_coding_harness_cross_provider_switch_dispatches_both_providers` (9/9 green at HEAD) |
| INV-002 wire code behind neutral interfaces | `Provider::stream_prepared` sole entry (provider.rs:44); adapters consume `ResolvedAuth` only | provider fixture suites; `phase17_api_audit` policy-in-core scan |
| INV-003 exact loop ordering | finalize -> prepare -> validate -> atomic apply -> stop -> steering -> follow-up -> route (agent_loop.rs:1004-1153) | `hooks_queues` phase17 ordering tests (A04/A05) |
| INV-004 full-state atomic replacement | `NextTurnState` complete value; `validate_next_turn_candidate` then `mem::replace` (loop_types.rs:342-403; agent_loop.rs:1046-1059) | `agent_wrapper` replacement/persistence tests; `phase17_prepare_call` (2/2 green) |
| INV-005 authority before side effects, monotonic combination | resolve -> hook(Continue) -> schema -> authorize -> stale-verify -> launch-boundary generation check -> execute (agent_loop.rs:1723-1894, 728-737); denial-only combinations in ProductToolAuthorizer | `phase17_tool_authority` 13/13 green incl. A06/A08 and CommandExecute matrix |
| INV-006 cancellation/overflow/partial-failure observable | typed terminals + ranking (agent_loop.rs:127-162, 1592-1608); proxy bounded overflow owner path | `phase17_failure_rollback` 19/19 green; steering queues unbounded (8.12, Info) |
| INV-007 session reconstruction/crash recovery | committed-prefix preservation + fail-closed reconciliation (harness.rs:2688-2710, 1923-1951); SessionFacade pending-write ordering (harness.rs:71-133) | byte-identical fixtures (MIG-001); `phase17_legacy_migration` 7/7 green |
| INV-008 finalized evidence binds branch/binding/policy | `ManifestCandidate::validate` + `validate_observation` strict graph checks; DirectRuntimeInput-only (evidence.rs:1801-1993, 2534-2566); product `EvidenceCapture` never fabricates ActiveSnapshot | `evidence_contract` 60/60 green; `phase17_product_evidence` 28/28 green |
| P17 secrecy: secret never in route/evidence/diagnostics | `PreparedProviderCall` redacted Debug; `ProviderErrorSummary` sealed; `RedactedValue` only structured channel | A10 canary matrix across sink/file/diagnostics/modes (green at HEAD) |
| P17 fail-closed authority | `authorize_and_verify` + `BatchInvalid` launch boundary; missing authorizer = zero execution | `tool_authority` (agent) 9/9; `phase17_tool_authority` 13/13 |

Invariants without direct test coverage are flagged above (4.2 steering loss,
8.12 queue posture); all others have named passing tests at the audit commit.

---

## 10. Minimum-change Conformance

The task graph records `simplification_trigger=` nowhere (0 occurrences in the
ledger), so the graph is **pre-contract**: absent standardized fields classify as
`not-recorded` without findings. All nine tasks do record `reuse_search`,
`placement`, `surface_necessity`, and `simplification_ceiling` notes, which were
compared against the complete implementation at `audit_head`.

| Task | Scenario/source | Reuse | Placement | Surface | Production slice | Ceiling/trigger | Status |
|------|-----------------|-------|-----------|---------|------------------|-----------------|--------|
| 17.1 | dispatch substrate, P17-PRV-001..004 | reused registry/per-request-auth/fixtures; collection-owned preparation added | core (opi-ai) -- holds; no product policy in core (api-audit scan green) | PreparedProviderCall + AuthProvenance consumed by Agent and product | via 17.2/17.5 dependency closure (prepare_call is the sole dispatch path) | no router trait/alias registry/breadth -- honored; provenance relocation to ResolvedAuth completed by 17.5 as recorded | `conforming` |
| 17.2 | P17-OUT-002, NXT-001..006 | reused Agent state/queues/compaction fixtures | core (opi-agent) -- holds | NextTurnState + one replacement op; SharedProvider/AgentHarness removed (verified absent) | Agent::prompt/continue_ via run_with_token | no patch protocol/shims -- honored | `conforming` |
| 17.3 | P17-FAL-001 evidence slice | reused event/trace vocabulary, redaction helpers | core -- holds | typed ids/health/binding/lifecycle + no-op/in-memory only | consumed by 17.4/17.6/17.7 closure | no file adapter/exporter in core -- honored | `conforming` |
| 17.4 | P17-OUT-003, AUT-001..008 | reused schema validation, deny hooks, execution routing | mechanism in core, policy in product -- holds | RegisteredTool/ToolAuthorizer consumed by Agent + product assembly | agent_loop tool path + ProductToolAuthorizer via Agent::prompt | no policy engine/allow-all/hook grant -- honored; origin-aware `collect_tools_with_origin` seam has zero production callers (see 6.1) | `conforming` (with 6.1 linked) |
| 17.5 | P17-OUT-001, PRV-001/002/004/006 | reused provider config, credential/OAuth resolvers, startup | product assembly over collection -- holds | ProviderBundle/CodingHarness constructors carry collection; no overload | binary startup -> build_provider_bundle -> prepare_call | no aliases/eager auth/second registry/test-only injection -- honored (bare-model unique-only proven) | `conforming` |
| 17.6 | P17-FAL agent evidence slice | reused lifecycle emission points, retry machinery | core -- holds | EvidenceSink runtime binding, terminal completeness | Agent::prompt/continue_/retry_last_turn (evidence_runtime) | expand-only, no product cutover -- honored | `conforming` |
| 17.7 | P17-OUT-004, PRV-003/005, EVD-001..011, FAL-004, MIG-003 | reused capture option/paths, runners, diagnostics, artifact-audit patterns | product adapter over core lifecycle -- holds | capture/runner outputs; legacy core trace exports removed (verified absent) | all five modes via CodingHarness/runners | one file adapter, strict manifest, producer redaction -- honored | `conforming` |
| 17.8 | P17-MIG-001/002/004 | reused JSONL repo, branch/fork logic, byte-immutability fixtures | product -- holds | existing resume/fork/session CLI typed remediation | session_cli/handle_session_cli + resume/fork call sites | read-only, no reader/shim/rewrite -- honored | `conforming` |
| 17.9 | AUTH/PLT/RBK closure | reused subprocess harnesses, CI matrix, docs, smoke | assurance-only -- holds (no runtime source modified except the recorded rustdoc link fix) | none added | five entry points + CI definition | no runtime repair -- honored | `conforming` |

`production_consumers` verification: for every claimed deletion or exclusivity
(SharedProvider, AgentHarness, MetadataProvider, TraceSink family, alias
registry, legacy reader, second registry), workspace source grep at `audit_head`
returns only the api-audit's forbidden-symbol list and test-scoped names
(`InvalidMetadataProvider` is a test-local mock). No repository-observable
simplification trigger fired; no `triggered` rows.

---

## 11. Residuals and Recommendations

### Priority recommendations

1. **Fix the Bedrock event-stream CRC handling (4.1, Major).** Return a typed
   parse error for CRC-invalid frames instead of dropping them, and flip the test
   that currently asserts the drop. This is the only finding that can corrupt
   model output silently.
2. **Close the A07 coverage gap and guard the registration origin (6.1, Major).**
   Route the malicious extension's builtin-named tools through the production
   registration surface in the test and assert exclusion/zero execution; derive
   origin from the registration path (use `collect_tools_with_origin` in product
   assembly or assert the input source) so a wiring change cannot launder an
   extension tool into a builtin capability.
3. **Restore undelivered steering/follow-up input on rollback (4.2).** User intent
   should not be silently lost when a later turn fails after the queue drain.
4. **Harden the shared redaction surfaces (5.1--5.3).** SecretRedactor field
   spellings, the proxy's raw-error echo, and the plain-string `ProviderConfig`
   are the three cheapest defense-in-depth wins on secret paths.
5. **Make the policy digest canonical (2.4)** and deduplicate the mutating-tool
   list (2.3) -- both sit directly on the authorization facts the phase made
   digest-addressed.
6. **Close the four evidence-depth test gaps** (3.3 A01 actual-route leg, 6.2
   schema-row authorizer count, 6.3 overdetermined graph-rule fixtures, 6.5
   factory extra-route dispatch) so the acceptance matrix's remaining legs rest
   on assertions that can fail.
7. **Comment hygiene pass** (2.1, 2.2): strip phase/task history and correct the
   inverted hook-contract comment on the file that pins the ordering.

### Verified execution at audit_head (evidence for the record)

- `python scripts/opi-doc-check.py` -- PASS
- `cargo test -p opi-agent --test evidence_contract` -- 60/60
- `cargo test -p opi-agent --test phase17_prepare_call` -- 2/2
- `cargo test -p opi-agent --test evidence_runtime --test tool_authority` -- 34+9
- `cargo test -p opi-coding-agent --test phase17_tool_authority` -- 13/13
- `cargo test -p opi-coding-agent --test phase17_provider_runtime` -- 9/9
- `cargo test -p opi-coding-agent --test phase17_product_evidence` -- 28/28
- `cargo test -p opi-coding-agent --test phase17_legacy_migration` -- 7/7
- `cargo test -p opi-coding-agent --test phase17_cross_mode` -- 7/7
- `cargo test -p opi-coding-agent --test phase17_failure_rollback` -- 19/19
- `cargo test -p opi-coding-agent --test phase17_api_audit` -- 22/22
- `cargo test -p opi-ai --test provider_collection --test per_request_auth --test auth_contracts` -- 7+12+54

All runs used the shared external cargo cache
(`E:\opi\cargo-targets\opi-audit-phase17-136c380f-e06b25925576e300`), hermetic
fixtures, and no network or credentials. The worktree was dirty only via two
user-deleted phase-17 documentation files, which were excluded from evidence.
