# Phase 17 Deep Agent Core Semantic Closure -- Independent Code Audit

**Auditor**: codex (fresh-context same-family; no prior audit reports consulted)
**Date**: 2026-08-22
**Scope**: Phase 17 registered requirements and Tasks 17.1--17.9
**Implementation target**: `136c380f0c5eea541190cc1a0f5c1d62f983b4e8` (current committed implementation)
**Phase exit commit**: `a4cfa4ddc74b4dfac59b4305d4657599af866480` (provenance only)
**History use**: provenance and discovery only; no diff coverage boundary
**Method**: Read the committed ledger, registered specification, parent specification, domain context, repository rules, relevant source, tests, manifests, and CI configuration from the pinned Git object. Separate fresh-context reviewers covered Standards, Spec, and the remaining audit dimensions. Verification ran in a detached checkout of the pinned commit because the main worktree contained unrelated deletions. Reviewer High/Medium/Low priorities are represented by the repository's canonical Major/Minor/Info severities.

---

## 1. Executive Summary

**Verdict: FAIL**

| Severity | Count |
|----------|-------|
| Blocker | 0 |
| Major | 16 |
| Minor | 8 |
| Info | 1 |

The endpoint is well tested and passes its executable gates, but the green suite does not establish the registered semantics. Major gaps remain in durable session authority, queue observability, committed next-turn state, startup route normalization, prepared-call credential termination, compaction failure propagation, tool authority, and evidence/redaction boundaries. The quantity and distribution of these failures are systemic, so the phase cannot retain its `70/70` completion claim.

### Per-task summary

| Task | Title | Verdict |
|------|-------|---------|
| 17.1 | Add collection-owned route and authentication preparation | FAIL -- prepared auth can be redispatched; route/provenance owners remain split |
| 17.2 | Cut over Agent to durable atomic NextTurnState | FAIL -- applied state can be discarded and candidate validation is incomplete |
| 17.3 | Define evidence identities, health, and storage-neutral lifecycle | FAIL -- public evidence errors/artifact strings do not enforce the claimed safe boundary |
| 17.4 | Cut over trusted tool registrations and mandatory authorization | FAIL -- registration and denial boundaries are not closed; fixed IDs differ |
| 17.5 | Wire the Reference Product to dispatchable provider routes | FAIL -- bare startup/model mutation can select the wrong provider or panic |
| 17.6 | Expand Agent evidence runtime over stable identities | FAIL -- post-emission stale Allow is not reauthorized |
| 17.7 | Cut over Reference Product evidence, finalization, and redaction | FAIL -- manifests omit required provenance and compaction failure returns success |
| 17.8 | Migrate legacy session routes and preserve opaque trace artifacts | FAIL -- durable binding and required/ignorable entry semantics are absent |
| 17.9 | Close cross-mode, rollback, documentation, and CI acceptance | FAIL -- queue/rollback coverage is missing and Phase markers remain in production source |

### Verification at the pinned endpoint

| Command | Result |
|---------|--------|
| `python scripts/opi-doc-check.py` | PASS |
| `cargo fmt --check --all` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo test -p opi-ai --test provider_collection` | PASS, 54 tests |
| `cargo test -p opi-agent --test hooks_queues` | PASS, 24 tests |
| `cargo test -p opi-coding-agent --test phase17_tool_authority` | PASS, 13 tests |
| `cargo test -p opi-coding-agent --test phase17_api_audit` | PASS, 22 tests |
| `cargo test -p opi-coding-agent --test phase17_cross_mode` | PASS, 7 tests |
| `cargo test --workspace --all-targets` | FAIL -- 5 `session_cli` E2E tests hard-code `<workspace>/target/debug/opi` and cannot find the binary in the required external Cargo cache |

Test impact: `none` (audit report only).

---

## 2. Standards Findings

### 2.1 MAJOR: ProviderCollection retains parallel lookup-only and dispatchable owners

**File:** `crates/opi-ai/src/provider_collection.rs`
**Lines:** 297--312, 327--397
**Cause:** A logical route is split across a registry plus five maps, with public lookup-only constructors and a product `ListingMetadataProvider` that implements `Provider` but cannot dispatch.
**Impact:** Route mutation requires synchronized edits, keeps a refused dispatch interface, and preserves a metadata/provider path beside the admitted dispatch route.
**Fix:** Store one atomic route entry per provider and use a product catalog DTO for listing-only metadata.

```yaml
id: P17-CODEX-STD-001
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex.md
source_model: codex
independence: fresh-context-same-family
axis: standards
severity: Major
title: "ProviderCollection retains parallel route owners"
claim: "ProviderCollection splits one route across six stores and exposes lookup-only Provider entries that cannot dispatch."
evidence:
  - location: "crates/opi-ai/src/provider_collection.rs:297"
    detail: "Registry, auth, compat, probe, resolver, and source state are separate maps."
  - location: "crates/opi-coding-agent/src/provider_factory.rs:470"
    detail: "ListingMetadataProvider implements Provider but its stream always errors."
criterion_source: "AGENTS.md deep-module and no-compatibility-layer rules; phase design lines 198-203 and 235-237"
reproduction: ["git grep -n -E 'from_registry|RouteNotDispatchable|ListingMetadataProvider' 136c380f0c5eea541190cc1a0f5c1d62f983b4e8 -- crates/opi-ai/src/provider_collection.rs crates/opi-coding-agent/src/provider_factory.rs"]
confidence: high
status: unverified
```

### 2.2 MAJOR: A valid auth provenance is also the unset sentinel

**File:** `crates/opi-ai/src/provider_collection.rs`
**Lines:** 445--459
**Cause:** `Static + NotAttempted` is both a valid resolved fact and `AuthProvenance::default()`, which triggers replacement by registration metadata.
**Impact:** A resolver can truthfully report static auth and evidence can record another source.
**Fix:** Represent omitted provenance explicitly or require every resolver to return authoritative provenance.

```yaml
id: P17-CODEX-STD-002
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex.md
source_model: codex
independence: fresh-context-same-family
axis: standards
severity: Major
title: "Valid auth provenance doubles as an unset sentinel"
claim: "A resolver returning valid default static provenance has that fact overwritten by the route's registered source."
evidence:
  - location: "crates/opi-ai/src/auth.rs:193"
    detail: "Static and NotAttempted are the defaults and valid domain values."
  - location: "crates/opi-ai/src/provider_collection.rs:454"
    detail: "Equality with default is used as the omission test."
criterion_source: "PRIN-004 and P17-PRV-005"
reproduction: ["Add a resolver returning AuthProvenance::default() to a route registered as Environment and inspect PreparedProviderCall::auth_provenance()."]
confidence: high
status: unverified
```

### 2.3 MAJOR: Trusted tool registration is not atomic or self-validating

**File:** `crates/opi-agent/src/authority.rs`
**Lines:** 39--55, 83--157
**Cause:** Registration fields are public, identifiers and the constructor are infallible, the registry checks only duplicate names, and product assembly calls `Tool::definition()` twice.
**Impact:** Trusted identity, indexed name, retained schema, and implementation can disagree at the authority boundary.
**Fix:** Read the definition once and use a fallible constructor with private fields and identity/schema consistency validation.

```yaml
id: P17-CODEX-STD-003
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex.md
source_model: codex
independence: fresh-context-same-family
axis: standards
severity: Major
title: "Trusted tool registration is not atomic"
claim: "A stateful Tool::definition implementation can be indexed under one name while retaining a different provider definition."
evidence:
  - location: "crates/opi-coding-agent/src/tool_authority.rs:78"
    detail: "definition() supplies the indexed name and is called again at line 85."
  - location: "crates/opi-agent/src/authority.rs:142"
    detail: "Registry construction validates only duplicate visible names."
criterion_source: "INV-005, PRIN-004, and the registered-tool immutability contract"
reproduction: ["Use a Tool whose definition() alternates between read and write, then pass it to register_builtin_tools."]
confidence: high
status: unverified
```

### 2.4 MAJOR: Authorizer denial text is trusted as redacted without enforcement

**File:** `crates/opi-agent/src/authority.rs`
**Lines:** 276--283
**Cause:** `AuthorizationDecision::Deny` accepts arbitrary `String` code and reason values.
**Impact:** A custom trusted authorizer can place secrets in diagnostics, tool result details, model-visible text, and subsequent provider context.
**Fix:** Use validated stable-code and redacted-safe summary types, and sanitize at the core boundary.

```yaml
id: P17-CODEX-STD-004
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex.md
source_model: codex
independence: fresh-context-same-family
axis: standards
severity: Major
title: "Denial strings bypass the redaction boundary"
claim: "Arbitrary authorizer strings are copied unchanged into model-visible denial results."
evidence:
  - location: "crates/opi-agent/src/agent_loop.rs:1878"
    detail: "The raw redacted_reason is passed to diagnostics and denial_result."
  - location: "crates/opi-agent/src/agent_loop.rs:2076"
    detail: "denial_result copies reason and code into ToolResult."
criterion_source: "P17-FAL-004 and P17-A10"
reproduction: ["Return AuthorizationDecision::Deny with redacted_reason 'sk-secret-canary' and inspect the next provider request context."]
confidence: high
status: unverified
```

### 2.5 MAJOR: Evidence errors expose unrestricted details and paths

**File:** `crates/opi-agent/src/evidence.rs`
**Lines:** 293--323
**Cause:** Public evidence error variants accept arbitrary detail strings and derive verbatim `Debug` and `Display`; the file sink supplies path-bearing I/O text.
**Impact:** Evidence setup/emission/finalization errors can escape the producer redaction boundary through diagnostics and public errors.
**Fix:** Carry a private safe-summary type publicly and retain raw source/path data only in an internal error chain.

```yaml
id: P17-CODEX-STD-005
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex.md
source_model: codex
independence: fresh-context-same-family
axis: standards
severity: Major
title: "Evidence errors expose unrestricted detail"
claim: "EvidenceError accepts and renders arbitrary unredacted strings across the public evidence boundary."
evidence:
  - location: "crates/opi-agent/src/evidence.rs:303"
    detail: "All three variants derive Debug/Error and carry unrestricted detail String."
  - location: "crates/opi-coding-agent/src/evidence.rs:137"
    detail: "The file adapter constructs lifecycle errors from I/O operations and paths."
criterion_source: "CTRL-003, P17-FAL-004, and P17-A10"
reproduction: ["Format EvidenceError::Setup { detail: 'sk-canary'.into() } with Debug or Display."]
confidence: high
status: unverified
```

### 2.6 MAJOR: Artifact reference types permit unclassified secret-bearing locations

**File:** `crates/opi-agent/src/evidence.rs`
**Lines:** 663--703
**Cause:** `ArtifactReference` fields are public and media/location wrappers accept arbitrary strings without validation.
**Impact:** A producer can serialize a secret-bearing path or URI as supposedly typed artifact metadata.
**Fix:** Make construction fallible and restrict locations to reviewed relative, digest-addressed, or opaque-reference forms.

```yaml
id: P17-CODEX-STD-006
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex.md
source_model: codex
independence: fresh-context-same-family
axis: standards
severity: Major
title: "Artifact metadata types do not enforce classification"
claim: "ArtifactLocation::new accepts arbitrary secret-bearing strings that serialize unchanged."
evidence:
  - location: "crates/opi-agent/src/evidence.rs:667"
    detail: "ArtifactReference fields are publicly constructible."
  - location: "crates/opi-agent/src/evidence.rs:699"
    detail: "ArtifactLocation::new performs no validation."
criterion_source: "CTRL-003 and phase exit condition at design lines 818-819"
reproduction: ["Serialize an ArtifactReference whose location is ArtifactLocation::new('sk-canary')."]
confidence: high
status: unverified
```

### 2.7 MAJOR: Content-bearing Agent Debug output is unredacted

**File:** `crates/opi-agent/src/agent.rs`
**Lines:** 103--113
**Cause:** `AgentRunResult::Debug` prints complete messages and cleanup errors; underlying message/state types also derive raw `Debug`.
**Impact:** Debug logging can expose prompts, tool arguments/results, and other model content outside the controlled diagnostic/evidence surfaces.
**Fix:** Use metadata-only `Debug` for public content-bearing results and states.

```yaml
id: P17-CODEX-STD-007
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex.md
source_model: codex
independence: fresh-context-same-family
axis: standards
severity: Major
title: "Agent Debug prints raw model content"
claim: "Formatting AgentRunResult with Debug exposes the complete message payload."
evidence:
  - location: "crates/opi-agent/src/agent.rs:103"
    detail: "Manual Debug includes messages and evidence_cleanup_error."
criterion_source: "CTRL-003 and repository boundary-safety rules"
reproduction: ["Run with user text 'sk-canary', then evaluate format!(\"{run:?}\")."]
confidence: high
status: unverified
```

### 2.8 MAJOR: Proxy credentials remain visible through Debug-derived configuration

**File:** `crates/opi-coding-agent/src/config.rs`
**Lines:** 22--34, 455--460
**Cause:** Proxy URLs remain raw strings inside `Debug`-derived configuration/HTTP client values although URLs may contain userinfo credentials.
**Impact:** Configuration diagnostics or debug logging can reveal proxy passwords.
**Fix:** Wrap proxy URLs in a type whose `Debug`/`Display` strips userinfo and sensitive query data.

```yaml
id: P17-CODEX-STD-008
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex.md
source_model: codex
independence: fresh-context-same-family
axis: standards
severity: Major
title: "Proxy credentials are Debug-visible"
claim: "A proxy URL containing userinfo is retained by Debug-derived configuration values."
evidence:
  - location: "crates/opi-coding-agent/src/config.rs:22"
    detail: "Proxy-bearing configuration derives Debug over raw string fields."
  - location: "crates/opi-ai/src/http.rs:86"
    detail: "HTTP client configuration also retains proxy values."
criterion_source: "P17-FAL-004 and adapter/config boundary safety"
reproduction: ["Configure http://user:proxy-secret@localhost:8080 and format the resolved config with Debug."]
confidence: high
status: unverified
```

### 2.9 MINOR: Public set_model panics on invalid input

**File:** `crates/opi-coding-agent/src/harness.rs`
**Lines:** 1871--1886
**Cause:** A public boundary calls `expect` after parsing and route validation.
**Impact:** Malformed or unknown model input terminates the process instead of returning the existing typed failure.
**Fix:** Return `Result` and route all model changes through one canonical normalizer.

```yaml
id: P17-CODEX-STD-009
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex.md
source_model: codex
independence: fresh-context-same-family
axis: standards
severity: Minor
title: "Public set_model panics on invalid input"
claim: "CodingHarness::set_model panics when the supplied model is invalid or non-dispatchable."
evidence:
  - location: "crates/opi-coding-agent/src/harness.rs:1872"
    detail: "Public method calls expect after apply_agent_model."
criterion_source: "AGENTS.md fail-closed typed-boundary rule"
reproduction: ["Call set_model('missing:model'.into()) on a harness without that route."]
confidence: high
status: unverified
```

### 2.10 MINOR: Provider refresh is an unadmitted speculative public seam

**File:** `crates/opi-ai/src/provider_collection.rs`
**Lines:** 552--580
**Cause:** Refresh remains a public trait/collection/registry surface despite no production trigger and only one real adapter.
**Impact:** Dynamic catalog state broadens core APIs without the required consumers and conformance leverage.
**Fix:** Narrow or remove refresh until a real trigger and second consumer justify the seam.

```yaml
id: P17-CODEX-STD-010
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex.md
source_model: codex
independence: fresh-context-same-family
axis: standards
severity: Minor
title: "Provider refresh is speculative public surface"
claim: "The public refresh seam has no production trigger and only one real adapter implementation."
evidence:
  - location: "crates/opi-ai/src/provider_collection.rs:552"
    detail: "ProviderCollection publicly exposes refresh lifecycle state."
criterion_source: "AGENTS.md no-hypothetical-seam rule and PRIN-002"
reproduction: ["git grep -n -E '\\.refresh\\(\\)|refresh_models' 136c380f0c5eea541190cc1a0f5c1d62f983b4e8 -- '*.rs'"]
confidence: high
status: unverified
```

### 2.11 MINOR: Provider startup failure classification is triplicated

**File:** `crates/opi-coding-agent/src/main.rs`
**Lines:** 839--858, 1031--1050, 1183--1202
**Cause:** Noninteractive, RPC, and interactive startup repeat bundle construction and the same failure taxonomy.
**Impact:** Redaction and exit behavior require three synchronized edits and can drift across modes.
**Fix:** Return one typed `StartupFailure` from a shared helper; keep only presentation mode-specific.

```yaml
id: P17-CODEX-STD-011
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex.md
source_model: codex
independence: fresh-context-same-family
axis: standards
severity: Minor
title: "Startup failure classification is duplicated across modes"
claim: "Three run-mode startup paths independently classify the same provider bundle failures."
evidence:
  - location: "crates/opi-coding-agent/src/main.rs:839"
    detail: "The same build/classification block recurs at lines 1031 and 1183."
criterion_source: "AGENTS.md minimum-change rule and Fowler Duplicated Code/Shotgun Surgery"
reproduction: ["git grep -n 'build_provider_bundle' 136c380f0c5eea541190cc1a0f5c1d62f983b4e8 -- crates/opi-coding-agent/src/main.rs"]
confidence: high
status: unverified
```

### 2.12 INFO: Production comments retain Phase/workstream history

**File:** `crates/opi-coding-agent/src/rpc.rs`
**Lines:** 1266
**Cause:** `opi-phase17-acceptance` and Workstream markers are used as historical test-discovery metadata.
**Impact:** Production comments encode delivery history and couple acceptance to source markers.
**Fix:** Discover stable behavior/configuration and rewrite comments in current-contract language.

```yaml
id: P17-CODEX-STD-012
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex.md
source_model: codex
independence: fresh-context-same-family
axis: standards
severity: Info
title: "Production comments retain Phase history"
claim: "Phase-specific acceptance markers remain in production source solely for test discovery."
evidence:
  - location: "crates/opi-coding-agent/src/rpc.rs:1266"
    detail: "The opi-phase17-acceptance marker is consumed by the Phase source audit."
criterion_source: "AGENTS.md current-contract comment rule"
reproduction: ["git grep -n -i -E 'phase17|workstream' 136c380f0c5eea541190cc1a0f5c1d62f983b4e8 -- crates/*/src"]
confidence: high
status: unverified
```

---

## 3. Spec Findings

### 3.1 MAJOR: Sessions cannot enforce required/ignorable entries or durable runtime binding

**File:** `crates/opi-agent/src/session.rs`
**Lines:** 15--23, 214--229, 437--541
**Cause:** Every unknown entry is skipped; `read_all` discards recovery metadata; the durable schema has no required/ignorable envelope or `RuntimeInputBinding`.
**Impact:** Resume can report success after losing semantics, while resume/fork/export/evidence cannot derive from the normative committed-prefix/binding pair.
**Fix:** Persist the immutable branch binding and an explicit entry criticality envelope; fail closed on unknown required entries.

```yaml
id: P17-CODEX-SPEC-001
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex.md
source_model: codex
independence: fresh-context-same-family
axis: spec
severity: Major
title: "Session durable authority contract is absent"
claim: "Unknown session entries are always skipped and no immutable RuntimeInputBinding is stored with the committed prefix."
evidence:
  - location: "crates/opi-agent/src/session.rs:510"
    detail: "Unknown tags increment a counter and still return success."
  - location: "crates/opi-agent/src/session.rs:214"
    detail: "SessionEntry has neither entry criticality nor runtime binding."
criterion_source: "docs/opi-spec.md:410-425, INV-007, and P17-AUTH-001"
reproduction: ["cargo test -p opi-agent --test session_facade session_facade_preserves_v1_readability_with_unknown_future_entry"]
confidence: high
status: unverified
```

### 3.2 MAJOR: Agent queues cannot report closure or overflow

**File:** `crates/opi-agent/src/agent.rs`
**Lines:** 492--514, 546--560
**Cause:** Steering and follow-up use unbounded `VecDeque`s; enqueue returns `()` and has no closed/overflow state.
**Impact:** The required observable backpressure outcomes cannot exist or enter public results/evidence.
**Fix:** Use bounded, closable queues with typed enqueue/drain outcomes and direct failure-injection tests.

```yaml
id: P17-CODEX-SPEC-002
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex.md
source_model: codex
independence: fresh-context-same-family
axis: spec
severity: Major
title: "Queue closure and overflow are unobservable"
claim: "AgentControl queue operations are infallible pushes into unbounded VecDeque values."
evidence:
  - location: "crates/opi-agent/src/agent.rs:507"
    detail: "steer and follow_up return unit after push_back."
  - location: "crates/opi-agent/src/agent_loop.rs:2089"
    detail: "Queue reads drain/pop an unbounded mutex-protected VecDeque."
criterion_source: "INV-006 and P17-FAL-003"
reproduction: ["Inspect the public AgentControl API; no closed or overflow result can be constructed."]
confidence: high
status: unverified
```

### 3.3 MAJOR: A committed NextTurnState is discarded after a later ordinary failure

**File:** `crates/opi-agent/src/agent.rs`
**Lines:** 925--973
**Cause:** `run_with_token` stores the loop's final state only on success or uncertain side-effect outcomes.
**Impact:** A state applied in turn N disappears if turn N+1 fails; returned messages and the durable Agent state can disagree.
**Fix:** Persist the loop's final state before every public operation settles; keep rollback local to the transition that failed.

```yaml
id: P17-CODEX-SPEC-003
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex.md
source_model: codex
independence: fresh-context-same-family
axis: spec
severity: Major
title: "Applied next-turn state can be discarded"
claim: "Agent does not retain successfully applied state when a later turn ends in an ordinary error."
evidence:
  - location: "crates/opi-agent/src/agent.rs:950"
    detail: "self.state is assigned only for success, PartialSideEffect, or CleanupUnknown."
criterion_source: "phase design lines 331-337 and P17-NXT-001"
reproduction: ["Add a two-turn Agent test: apply route B after turn 1, fail provider B in turn 2, then inspect state_snapshot()."]
confidence: high
status: unverified
```

### 3.4 MAJOR: Bare startup validation discards the proven provider

**File:** `crates/opi-coding-agent/src/main.rs`
**Lines:** 1336--1397
**Cause:** Validation proves a unique bare-model route but returns only `()`. Harness construction later prefixes the active provider instead of the proven provider.
**Impact:** A bare model uniquely hosted by an extra route validates and then resolves the wrong route or panics. Public `set_model` repeats the same guess.
**Fix:** Return the exact canonical `provider:model` from normalization and use it through builder and mutation paths.

```yaml
id: P17-CODEX-SPEC-004
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex.md
source_model: codex
independence: fresh-context-same-family
axis: spec
severity: Major
title: "Bare startup normalization loses the selected provider"
claim: "A unique bare model on an extra provider validates but is later prefixed with the active provider."
evidence:
  - location: "crates/opi-coding-agent/src/main.rs:1381"
    detail: "Unique matches return Ok(()) without the matched provider."
  - location: "crates/opi-coding-agent/src/harness.rs:1640"
    detail: "canonical_model_spec prefixes provider.id() for bare input."
criterion_source: "phase design lines 249-253 and P17-PRV-002"
reproduction: ["Use active alpha without model shared, extra route beta:shared, and startup model shared."]
confidence: high
status: unverified
```

### 3.5 MAJOR: NextTurnState validation omits model output limits

**File:** `crates/opi-agent/src/loop_types.rs`
**Lines:** 342--402
**Cause:** Candidate validation checks route, model metadata, thinking capability, and finite temperature, but not `max_tokens` against `max_output_tokens` or the thinking budget relationship.
**Impact:** An invalid complete state can be atomically committed and sent to a provider, contrary to whole-candidate validation.
**Fix:** Move model-bound inference validation into the shared core candidate validator and add typed rollback tests for both limits.

```yaml
id: P17-CODEX-SPEC-005
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex.md
source_model: codex
independence: fresh-context-same-family
axis: spec
severity: Major
title: "NextTurnState validation omits output limits"
claim: "A candidate with max_tokens above the resolved model limit passes core validation."
evidence:
  - location: "crates/opi-agent/src/loop_types.rs:370"
    detail: "Validation ends after thinking and finite-temperature checks."
  - location: "crates/opi-ai/src/model_info.rs:21"
    detail: "The resolved model carries max_output_tokens."
  - location: "crates/opi-coding-agent/src/harness.rs:3985"
    detail: "Product-only code already recognizes and checks this constraint."
criterion_source: "phase design lines 317-355 and P17-NXT-002"
reproduction: ["Set candidate.inference.max_tokens above model.capabilities.max_output_tokens and call Agent::replace_state."]
confidence: high
status: unverified
```

### 3.6 MINOR: Evidence failure after Allow does not trigger required reauthorization

**File:** `crates/opi-agent/src/agent_loop.rs`
**Lines:** 1821--1876
**Cause:** Authorization evidence emission can advance health after a fresh Allow, but the caller immediately converts the mismatch to denial rather than rebuilding the request and authorizing again.
**Impact:** The side effect fails closed, but the registered stale-generation protocol and policy decision are bypassed.
**Fix:** Rebuild with current health, reauthorize once, and enforce/record that decision before launch.

```yaml
id: P17-CODEX-SPEC-006
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex.md
source_model: codex
independence: fresh-context-same-family
axis: spec
severity: Minor
title: "Post-emission stale Allow is not reauthorized"
claim: "When authorization evidence emission changes health, the runtime denies without calling the authorizer with the new generation."
evidence:
  - location: "crates/opi-agent/src/agent_loop.rs:1836"
    detail: "Evidence is emitted after authorize_and_verify."
  - location: "crates/opi-agent/src/agent_loop.rs:1846"
    detail: "Emission failure or generation mismatch returns BatchInvalid directly."
criterion_source: "phase design lines 509-512, P17-EVD-009, and P17-A12"
reproduction: ["Use a sink that fails on the authorization record and count ToolAuthorizer calls/generations."]
confidence: high
status: unverified
```

### 3.7 MINOR: Final manifests do not bind artifact or authorization provenance

**File:** `crates/opi-coding-agent/src/evidence.rs`
**Lines:** 443--509, 586--630
**Cause:** Product manifests initialize permission/scope/grant facts to `None` and always emit an empty artifact list despite creating `evidence.jsonl` and `manifest.json`.
**Impact:** Offline verification cannot content-bind the evidence log or recompute the authorization/input provenance represented only by bare digests.
**Fix:** Finalize and content-address the evidence log and applicable resolved-input/authorization artifacts, then include their classified references.

```yaml
id: P17-CODEX-SPEC-007
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex.md
source_model: codex
independence: fresh-context-same-family
axis: spec
severity: Minor
title: "Manifest omits required provenance references"
claim: "The product file manifest is complete while containing no artifact reference to its evidence log and no applicable permission/scope/grant references."
evidence:
  - location: "crates/opi-coding-agent/src/evidence.rs:497"
    detail: "Permission, scope, and grant fields start as None."
  - location: "crates/opi-coding-agent/src/evidence.rs:620"
    detail: "The adapter explicitly treats an empty artifact set as vacuous compliance."
criterion_source: "CTRL-001, CTRL-002, P17-OUT-004, and P17-EVD-003"
reproduction: ["Inspect the manifest produced by file_evidence_sink_writes_records_and_manifest and require an evidence-log artifact reference."]
confidence: high
status: unverified
```

### 3.8 MINOR: Built-in capability IDs differ from the mandatory fixed map

**File:** `crates/opi-coding-agent/src/tool_authority.rs`
**Lines:** 42--65
**Cause:** Implementation uses `opi.workspace.read`, `opi.workspace.write`, and `opi.command.execute`; the registered map requires unprefixed IDs.
**Impact:** Evidence and permission identities do not match the specification or external fixtures that consume it.
**Fix:** Use the exact fixed IDs or revise the specification explicitly.

```yaml
id: P17-CODEX-SPEC-008
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex.md
source_model: codex
independence: fresh-context-same-family
axis: spec
severity: Minor
title: "Built-in capability IDs differ from the fixed map"
claim: "Reference Product capability identities add an opi. prefix absent from the mandatory map."
evidence:
  - location: "crates/opi-coding-agent/src/tool_authority.rs:42"
    detail: "All three product capability constants are prefixed."
criterion_source: "phase design lines 417-423 and Task 17.4 DoD"
reproduction: ["git show 136c380f0c5eea541190cc1a0f5c1d62f983b4e8:crates/opi-coding-agent/src/tool_authority.rs"]
confidence: high
status: unverified
```

### 3.9 MINOR: Required pre-Phase rollback regression was not executed

**File:** `docs/snapshots/phase17/opi-impl-state.json`
**Lines:** 2687
**Cause:** The ledger marks P17-RBK-002 met while explicitly substituting a structural scan for the required revert and pre-Phase regression profile.
**Impact:** Reversibility remains unproven and the phase-exit evidence overclaims the criterion.
**Fix:** In an isolated checkout, revert the registered Phase range and run/record the pre-Phase regression profile and artifact-preservation checks.

```yaml
id: P17-CODEX-SPEC-009
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex.md
source_model: codex
independence: fresh-context-same-family
axis: spec
severity: Minor
title: "Rollback criterion is marked met without its required test"
claim: "P17-RBK-002 is claimed complete although no live revert/pre-Phase regression profile was executed."
evidence:
  - location: "docs/snapshots/phase17/opi-impl-state.json:2687"
    detail: "The evidence text admits the required profile was replaced by a structural current-tree scan."
criterion_source: "P17-RBK-002"
reproduction: ["Inspect the committed P17-RBK-002 evidence at the pinned HEAD."]
confidence: high
status: unverified
```

---

## 4. Correctness, Security, Invariants, and Integration Findings

### 4.1 MAJOR: Static credential rejection does not terminate the prepared call

**File:** `crates/opi-ai/src/provider_collection.rs`
**Lines:** 727--744, 799--827
**Cause:** `credential_terminal` is set only for `CredentialNeeded` and `CredentialRevoked`; a real static 401/403 maps to `AuthFailed`.
**Impact:** Public callers can redispatch the same frozen rejected credential. The Agent retry policy masks the defect but does not repair the prepared-call contract.
**Fix:** Centralize the closed classification of errors that terminate frozen auth, including at least `AuthFailed`, `CredentialNeeded`, `CredentialRevoked`, and `AccountIdMissing`.

```yaml
id: P17-CODEX-INV-001
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex.md
source_model: codex
independence: fresh-context-same-family
axis: invariants
severity: Major
title: "Rejected static credentials can be redispatched"
claim: "After a stream returns AuthFailed for frozen static auth, PreparedProviderCall::start_attempt succeeds again."
evidence:
  - location: "crates/opi-ai/src/provider_collection.rs:727"
    detail: "Only CredentialNeeded and CredentialRevoked set credential_terminal."
  - location: "crates/opi-ai/src/auth.rs:112"
    detail: "Static 401/403 rejection maps to AuthFailed."
  - location: "crates/opi-ai/tests/provider_collection.rs:2168"
    detail: "The no-redispatch test covers only Revoked and Needed."
criterion_source: "phase design lines 242-247 and Task 17.1 DoD"
reproduction: ["Extend stream_time_credential_failure_forbids_redispatch with an AuthFailed provider; the second start_attempt currently returns Ok."]
confidence: high
status: unverified
```

### 4.2 MAJOR: Compaction failure and cleanup uncertainty are returned as success

**File:** `crates/opi-coding-agent/src/harness.rs`
**Lines:** 2869--2927
**Cause:** Automatic compaction persistence errors only update evidence terminal outcome; they never set the public run error, so `persist_turn` and `prompt` return `Ok`.
**Impact:** Print/JSON can exit zero while the manifest says `Failed`/`CleanupUnknown`, including a real partially durable compaction marker after rollback failure.
**Fix:** Preserve the committed turn, finalize truthful evidence, and return a typed compaction/session persistence error without invoking uncommitted-turn rollback.

```yaml
id: P17-CODEX-INT-001
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex.md
source_model: codex
independence: fresh-context-same-family
axis: integration
severity: Major
title: "Compaction failure is converted into public success"
claim: "CodingHarness::prompt returns Ok when automatic compaction persistence fails or rollback leaves cleanup unknown."
evidence:
  - location: "crates/opi-coding-agent/src/harness.rs:2869"
    detail: "I/O failures become CompactionOutcome but execution error remains None."
  - location: "crates/opi-coding-agent/src/harness.rs:4719"
    detail: "A test expects prompt success while asserting CleanupUnknown and a durable partial marker."
  - location: "crates/opi-coding-agent/src/runner.rs:453"
    detail: "Ok maps to exit code zero."
criterion_source: "INV-006 and P17-FAL-003"
reproduction: ["cargo test -p opi-coding-agent --lib automatic_compaction_rollback_failure_emits_cleanup_unknown_terminal -- --nocapture"]
confidence: high
status: unverified
```

### 4.3 MAJOR: Interactive outside-workspace reads are authorized as workspace-only

**File:** `crates/opi-coding-agent/src/tool_authority.rs`
**Lines:** 445--451
**Cause:** Interactive `ReadTool` permits outside-workspace paths, but every read capability receives the constant `workspace:read` permission scope and the policy contains no read path-policy fact.
**Impact:** Trusted authorization and evidence claim a narrower scope than the actual side effect, so offline authority verification is false.
**Fix:** Bind the resolved read path policy/relation into `EffectiveUserPolicy` and the permission scope, and verify it again at the execution boundary.

```yaml
id: P17-CODEX-SEC-001
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex.md
source_model: codex
independence: fresh-context-same-family
axis: security
severity: Major
title: "Outside-workspace reads carry a workspace-only scope"
claim: "Interactive read can access an external path while authorization records permission_scope workspace:read."
evidence:
  - location: "crates/opi-coding-agent/src/harness.rs:3631"
    detail: "Interactive ReadTool uses AllowOutsideWorkspace."
  - location: "crates/opi-coding-agent/src/tool/mod.rs:97"
    detail: "Outside paths are rejected only for WorkspaceOnly."
  - location: "crates/opi-coding-agent/src/tool_authority.rs:445"
    detail: "Every read Allow receives the constant workspace:read scope."
criterion_source: "INV-005, CTRL-002, and phase design lines 433-437"
reproduction: ["In interactive mode authorize read of an absolute external file and inspect ToolAuthorizationFacts.permission_scope."]
confidence: high
status: unverified
```

---

## 5. Test Quality Findings

The executable suite is broad, isolated, and hermetic, but several tests assert the implementation's deviation rather than the registered contract: unknown future session entries remain nonfatal, queue closure/overflow has no public surface, compaction cleanup uncertainty is expected to return success, credential-terminal coverage omits `AuthFailed`, and the rollback profile is replaced by a structural scan. These gaps are recorded under the owning Spec/Invariant/Integration findings to avoid duplicate normalized findings.

### 5.1 MINOR: Session CLI tests bypass the repository's external Cargo cache workflow

**File:** `crates/opi-coding-agent/tests/session_cli.rs`
**Lines:** 835--849
**Cause:** The E2E helper ignores `CARGO_TARGET_DIR` and hard-codes `<workspace>/target/debug/opi`.
**Impact:** The canonical all-target workspace gate fails under the repository-required external cache workflow even after `cargo build -p opi-coding-agent` succeeds.
**Fix:** Resolve the Cargo-provided binary path or honor `CARGO_TARGET_DIR`; keep the test independent of a workspace-local target directory.

```yaml
id: P17-CODEX-TST-001
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex.md
source_model: codex
independence: fresh-context-same-family
axis: test-quality
severity: Minor
title: "Session CLI E2E tests ignore the external Cargo target"
claim: "Five session_cli E2E tests fail because opi_binary() always looks under the worktree's target directory."
evidence:
  - location: "crates/opi-coding-agent/tests/session_cli.rs:835"
    detail: "opi_binary constructs workspace_root/target/debug/opi and ignores CARGO_TARGET_DIR."
  - location: "cargo test --workspace --all-targets"
    detail: "The pinned checkout failed five E2E tests after the binary was successfully built in the resolved external cache."
criterion_source: "AGENTS.md external Cargo cache workflow and Task 17.9 workspace-gate claim"
reproduction: ["Set CARGO_TARGET_DIR from python scripts/opi-cargo-cache.py resolve, run cargo build -p opi-coding-agent, then cargo test -p opi-coding-agent --test session_cli."]
confidence: high
status: unverified
```

---

## 6. Invariant Verification

| Invariant | Code evidence | Test coverage | Verdict |
|-----------|---------------|---------------|---------|
| INV-001 collection-owned provider routing | `ProviderCollection::prepare_call`, Agent uses one prepared call | provider_collection, phase17_prepare_call, retry_agent | PARTIAL -- `AuthFailed` does not terminate frozen auth; startup loses unique provider |
| INV-002 provider-neutral interfaces | `Provider`, `Request`, `EventStream`, inward crate edges | provider fixtures, Cargo review | PASS |
| INV-003 exact turn ordering | `agent_loop` prepare/validate/apply/stop/queues | hooks_queues, cross-provider tests | PASS |
| INV-004 full atomic next state | `NextTurnState`, candidate swap | hooks_queues, agent_wrapper | FAIL -- output limits omitted and committed state can be discarded later |
| INV-005 authority before side effects | registry -> hook -> schema -> authorizer -> evidence -> execute | tool_authority, evidence_runtime | FAIL -- registration/safe-text/scope and post-emission reauthorization gaps |
| INV-006 visible cancellation/backpressure/partial failure | cancellation and tool outcomes are typed | cross_mode, evidence_runtime | FAIL -- queues are unbounded and compaction failure returns success |
| INV-007 session durability and reconstruction | append-only v1 JSONL and recovery | session_* suites | FAIL -- no durable runtime binding or required/ignorable envelope |
| INV-008 finalized resolved-execution evidence | typed records and manifest validation | evidence_contract, evidence_runtime | FAIL -- manifest lacks artifact/authorization binding |

---

## 7. Cross-task Integration

The happy-path handoffs are coherent: collection-prepared calls reach the Agent, next-turn routing reaches the product harness, trusted registrations reach the authorizer, evidence generations reach tool preflight, and file finalization withholds invalid manifests. The failing handoffs are semantic rather than wiring omissions: provider auth classification does not close the prepared call; loop state does not always settle into the Agent; product model normalization does not preserve the provider it proved; evidence emission does not re-enter authority; compaction outcome does not enter public execution outcome; and session persistence does not own the runtime binding required by evidence.

---

## 8. Minimum-change Conformance

The graph is pre-contract because no task records `simplification_trigger=`. Historical note omissions therefore are not findings. All nine tasks do record the available reuse, placement, surface, and ceiling trace.

| Task | Scenario/source | Reuse | Placement | Surface | Production slice | Ceiling/trigger | Status |
|------|-----------------|-------|-----------|---------|------------------|-----------------|--------|
| 17.1 | Later 17.5/17.7 provider scenarios | ProviderCollection/AuthResolver | `opi-ai` | PreparedProviderCall/AuthProvenance | Agent and Reference Product | No router/second registry; lookup-only route state remains | `drifted` -> STD-001/002 |
| 17.2 | A04/A05 | Agent/NextTurnState | `opi-agent` plus product compaction | one complete state | prompt/continue/CodingHarness | no second state owner | `conforming` |
| 17.3 | Later 17.4/17.7 | evidence identities/lifecycle | `opi-agent` | EvidenceSink + no-op/in-memory | Agent runtime | no product storage/exporter in core | `conforming` |
| 17.4 | A06--A08 | Tool registry/authorizer | core mechanism + product policy | registration/authorizer | harness/loop | no policy engine | `conforming` |
| 17.5 | A02 | provider factory/collection | product assembly | existing collection | all run modes | no alias/secondary route path; listing fake Provider remains | `drifted` -> STD-001/SPEC-004 |
| 17.6 | Later 17.7 | evidence runtime | `opi-agent` | existing evidence lifecycle | Agent loop | no product file/exporter code | `conforming` |
| 17.7 | A01/A03/A09--A12 | file adapter/evidence | product | one file adapter | harness finalization | no exporter framework | `conforming` |
| 17.8 | A13 | session route normalization | product/session facade | no new durable format | resume/fork paths | no compatibility reader/upgrade path | `conforming` |
| 17.9 | A14/A15 | existing tests/CI/docs | assurance layer | no runtime API | CI and source audit | Phase-marker discovery adds historical production glue | `drifted` -> STD-012 |

---

## 9. Residuals and Recommendations

### Priority recommendations

1. Correct the durable semantic owners first: session prefix/binding/envelope, bounded queues, Agent final-state settlement, and compaction error propagation.
2. Close authority and evidence boundaries: atomic registration, exact capability/scope facts, post-emission reauthorization, safe denial/error text, and manifest artifact/provenance binding.
3. Fix provider routing as one unit: canonical bare normalization, frozen-auth terminal classification, authoritative provenance, and one atomic route representation.
4. Add focused regression tests for every Major before rerunning Phase 17 acceptance. Do not treat the currently green suite as evidence that the missing states are impossible.
5. Execute P17-RBK-002 exactly as registered in an isolated checkout, then revise the `70/70` exit claim to match independently demonstrated rows.

### Positive observations

- Dependencies still point inward and no new crate, feature flag, unsafe boundary, exporter framework, or policy language was introduced.
- Provider-neutral request/stream types, cancellation propagation, fixed tool ordering, evidence identity correlation, producer-side content redaction, and manifest withholding are generally strong.
- The test suite is hermetic, uses isolated filesystem state, and exercises Linux/macOS/Windows acceptance selection, even though several negative semantic states remain unrepresented.

### Unverified gates

`cargo test --workspace --doc` and `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` were not rerun for this audit because no Rust or public documentation source was modified. The report itself was checked with the repository documentation contract after writing.
