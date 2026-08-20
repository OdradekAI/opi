# Phase 17 Deep Agent Core Semantic Closure — Independent Code Audit

- Auditor: Codex
- Audit date: 2026-08-15
- Audited revision: `eb5e3166834f804c9b47f5d17f8131652931c601`
- Recorded phase-exit commit: `a4cfa4ddc74b4dfac59b4305d4657599af866480`
- Scope: tasks 17.1–17.9, their registered requirements, parent invariants, minimum-change overlays, and current committed implementation
- Verdict: **FAIL**
- Findings: **0 Blocker, 14 Major, 3 Minor**

The phase does not satisfy its admitted semantic-closure claim. The full local
workspace gate is green, but independent committed-HEAD inspection found
systemic gaps in provider-call ownership, complete-state validation,
authorization/evidence ordering, and evidence truthfulness. The passing tests
do not exercise the negative paths that expose these gaps.

## Method and evidence boundary

The audit pinned committed `HEAD` before inspection and used `git show`,
`git grep`, and committed-object searches for implementation evidence. Existing
Phase 17 `audit.*.md` reports were not read, searched, or used. Historical
evidence was used only to understand the registered requirements, task ledger,
and claimed phase-exit gates; findings target current committed code rather than
diff shape.

The independent Standards and Spec reviews were kept as separate axes. A third
technical review checked correctness, security/redaction, tests, invariants,
integration, and residual risks. All three reviewers targeted the same pinned
revision.

## Standards review

### P17-STD-001 — Published Agent hook ordering and ownership documentation contradicts runtime

**Severity: Major.** Both `opi-agent` READMEs describe stop before preparation
and schema validation before `before_tool_call`, while the runtime prepares and
applies before stop and invokes the hook before schema validation. The API table
also assigns product-owned `EffectiveUserPolicy` to `opi-agent`. Evidence:
`crates/opi-agent/README.md:121-126,147-150,184,363`,
`crates/opi-agent/README.zh.md:110-114,131-134,162,318`,
`crates/opi-agent/src/agent_loop.rs:767-820,1271-1299`, and
`crates/opi-coding-agent/src/tool_authority.rs:145`. This violates the README
source-of-truth rule, bilingual synchronization obligation, and `INV-003`.

Recommended fix: update both READMEs together to the implemented public order
and remove `EffectiveUserPolicy` from the `opi-agent` surface table.

```yaml
id: P17-STD-001
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex.md
source_model: codex
independence: unknown
axis: standards
severity: Major
criterion_source: "AGENTS.md source-of-truth and bilingual-sync rules; docs/opi-spec.md INV-003"
reproduction:
  - "git grep -n -E 'should_stop_after_turn|prepare_next_turn|before_tool_call|EffectiveUserPolicy' eb5e3166834f804c9b47f5d17f8131652931c601 -- crates/opi-agent/README.md crates/opi-agent/README.zh.md crates/opi-agent/src/agent_loop.rs crates/opi-coding-agent/src/tool_authority.rs"
confidence: high
status: unverified
```

### P17-STD-002 — Agent Core closes public types over Reference Product policy

**Severity: Major.** `AssemblySource` is closed over `Cli | Sdk | Rpc`, and
`CapabilityClass` is closed over the coding product's built-in permission
families (`crates/opi-agent/src/evidence.rs:347-358,652-673` and
`crates/opi-agent/src/authority.rs:79-95`). The matching tool-policy map belongs
to the Reference Product (`crates/opi-coding-agent/src/tool_authority.rs:40-50`).
Another embedder must either misclassify itself or change Agent Core, contrary
to `PRIN-003` and the admitted product/core placement.

Recommended fix: keep opaque validated source/capability identities in core and
define CLI/SDK/RPC plus workspace/tool constants in `opi-coding-agent`.

```yaml
id: P17-STD-002
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex.md
source_model: codex
independence: unknown
axis: standards
severity: Major
criterion_source: "docs/opi-spec.md PRIN-003; Phase 17 design placement review"
reproduction:
  - "git grep -n -E 'enum AssemblySource|enum CapabilityClass|Capability::Builtin|fn builtin_capability' eb5e3166834f804c9b47f5d17f8131652931c601 -- crates/opi-agent/src crates/opi-coding-agent/src/tool_authority.rs"
confidence: high
status: unverified
```

### P17-STD-003 — Provider evidence crosses crates through a duplicated string protocol

**Severity: Major.** Agent Core serializes provider route/provenance as ad-hoc
JSON tokens (`crates/opi-agent/src/agent_loop.rs:183-205,1581-1603`); the product
redeclares and reparses the shape (`crates/opi-coding-agent/src/evidence.rs:522-645`).
Missing, malformed, or future values default to configured routes or
`Static`/`NotAttempted`, although typed `RouteFacts` and `ProvenanceFacts`
already exist at `crates/opi-agent/src/evidence.rs:599-650`. Schema drift can
therefore produce a complete-looking manifest with invented facts. This
violates the typed, fail-closed boundary rule and creates repeated-switch and
shotgun-surgery pressure.

Recommended fix: use one typed provider-evidence payload end to end and make
conversion failures mark evidence incomplete.

```yaml
id: P17-STD-003
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex.md
source_model: codex
independence: unknown
axis: standards
severity: Major
criterion_source: "AGENTS.md typed fail-closed boundary rule; docs/opi-spec.md CTRL-002"
reproduction:
  - "git grep -n -E 'requested_route|RoutePayload|auth_source_token|auth_source_from_token|capture_recorder_failed|empty_selection' eb5e3166834f804c9b47f5d17f8131652931c601 -- crates/opi-agent/src/agent_loop.rs crates/opi-agent/src/evidence.rs crates/opi-coding-agent/src/evidence.rs"
confidence: high
status: unverified
```

### P17-STD-004 — Finalized-manifest validity is optional at the public sink boundary

**Severity: Major.** `FinalizedManifest` exposes mutable public fields,
`require_complete` is an optional caller-side check, and `EvidenceSink::finalize_run`
accepts the unvalidated type (`crates/opi-agent/src/evidence.rs:777-810,893-908,1058-1070,1113-1187`).
The file sink writes it directly (`crates/opi-coding-agent/src/evidence.rs:203-258`).
Only `CodingHarness` manually gates the value
(`crates/opi-coding-agent/src/harness.rs:2884-2894`), so another public embedder
can publish an invalid finalized manifest.

Recommended fix: make finalized fields private and require a validated manifest
type at every sink boundary.

```yaml
id: P17-STD-004
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex.md
source_model: codex
independence: unknown
axis: standards
severity: Major
criterion_source: "AGENTS.md fail-closed adapter-boundary rule; Phase 17 finalized-manifest contract"
reproduction:
  - "git grep -n -E 'pub struct FinalizedManifest|pub completeness|fn require_complete|fn finalize_run|incomplete.completeness' eb5e3166834f804c9b47f5d17f8131652931c601 -- crates/opi-agent/src/evidence.rs crates/opi-coding-agent/src/evidence.rs crates/opi-coding-agent/src/harness.rs crates/opi-agent/tests/evidence_contract.rs"
confidence: high
status: unverified
```

### P17-STD-005 — Source comments preserve stale phase history

**Severity: Minor.** Phase/task identifiers remain pervasive in source, and
some current comments still say policy will arrive in 17.7 or refer to the
removed `DiagnosticLinked` trace contract. Representative evidence is at
`crates/opi-agent/src/agent_loop.rs:1538-1544,1639-1646` and
`crates/opi-coding-agent/src/tool/bash.rs:447-454,509-514`. This conflicts with
the repository rule that source comments describe current contracts rather
than phase history.

Recommended fix: rewrite affected comments as current invariants and keep
delivery history in snapshots or Git.

```yaml
id: P17-STD-005
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex.md
source_model: codex
independence: unknown
axis: standards
severity: Minor
criterion_source: "AGENTS.md current-contract comment rule"
reproduction:
  - "git grep -n -E 'Phase [0-9]|Task [0-9]|task [0-9]|P17-|17\\.[0-9]' eb5e3166834f804c9b47f5d17f8131652931c601 -- crates/opi-ai/src crates/opi-agent/src crates/opi-coding-agent/src"
confidence: high
status: unverified
```

## Spec review

### P17-SPEC-001 — Bedrock bypasses collection-owned per-call authentication

**Severity: Major.** The production Bedrock route resolves AWS credentials at
provider construction, discards the reported source, registers a placeholder
static resolver, and ignores the `ResolvedAuth` frozen for each call
(`crates/opi-coding-agent/src/provider_factory.rs:1913-1975` and
`crates/opi-ai/src/bedrock/mod.rs:195-207`). The implementation added a
"compound-credential exemption" to the provider contract
(`crates/opi-ai/src/provider.rs:29-48`), but no registered Phase 17 requirement
admits that exception. Bedrock therefore cannot rotate/remediate credentials
per logical call and reports false static provenance.

Recommended fix: extend the closed secret-bearing prepared-auth value to carry
compound SigV4 material; resolve it in `prepare_call` and remove provider-owned
credentials and the placeholder resolver.

```yaml
id: P17-SPEC-001
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex.md
source_model: codex
independence: unknown
axis: spec
severity: Major
criterion_source: "Phase 17 P17-PRV-001, P17-PRV-003, P17-PRV-004, P17-PRV-005; tasks 17.1 and 17.5 DoD"
reproduction:
  - "git grep -n -E 'compound-credential exemption|bedrock-compound-credential|build_bedrock|stream_prepared.*_auth' eb5e3166834f804c9b47f5d17f8131652931c601 -- crates/opi-ai/src crates/opi-coding-agent/src/provider_factory.rs"
confidence: high
status: unverified
```

### P17-SPEC-002 — `prepare_call` never validates request-route identity

**Severity: Major.** `ProviderCollection::prepare_call` resolves `spec`, checks
the resolved model's capabilities against the request, and freezes both, but it
never compares `request.model` with the resolved route
(`crates/opi-ai/src/provider_collection.rs:398-457`). `CollectionError` has no
request-route mismatch variant (`:239-274`). A public caller can resolve one
provider/model while dispatching a request naming another, contrary to the
explicit request-route-mismatch failure requirement.

Recommended fix: reject mismatched canonical request/route identities with a
typed error before authentication or dispatch and add zero-call tests.

```yaml
id: P17-SPEC-002
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex.md
source_model: codex
independence: unknown
axis: spec
severity: Major
criterion_source: "Task 17.1 DoD; Phase 17 provider failure table and P17-FAL-001"
reproduction:
  - "git show eb5e3166834f804c9b47f5d17f8131652931c601:crates/opi-ai/src/provider_collection.rs | Select-Object -Skip 238 -First 220"
confidence: high
status: unverified
```

### P17-SPEC-003 — Stream-time credential rejection does not terminate a prepared call

**Severity: Major.** `AttemptStream` clears only the shared active-attempt flag
on every terminal error, and `start_attempt` checks only cancellation and that
flag (`crates/opi-ai/src/provider_collection.rs:664-771`). Consequently a
provider stream ending in `CredentialRevoked` or `CredentialNeeded` releases
the slot and permits another attempt with the same rejected frozen credential.
The rejection tests at `crates/opi-ai/tests/provider_collection.rs:1906-1957`
cover resolver failure before a prepared call exists, not stream-time rejection.

Recommended fix: retain a terminal-call state shared with `AttemptStream`, set
it for credential rejection/expiry, return a typed terminated-call error from
later attempts, and test that dispatch count remains one.

```yaml
id: P17-SPEC-003
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex.md
source_model: codex
independence: unknown
axis: spec
severity: Major
criterion_source: "Task 17.1 DoD; Phase 17 logical-call credential rejection contract"
reproduction:
  - "git show eb5e3166834f804c9b47f5d17f8131652931c601:crates/opi-ai/src/provider_collection.rs | Select-Object -Skip 663 -First 110"
  - "git show eb5e3166834f804c9b47f5d17f8131652931c601:crates/opi-ai/tests/provider_collection.rs | Select-Object -Skip 1889 -First 75"
confidence: high
status: unverified
```

### P17-SPEC-004 — Complete next-turn candidates are validated only for route existence

**Severity: Major.** The complete state includes context, model selection,
thinking, maximum tokens, and temperature, but both public replacement and loop
preparation validate only `model_selection.to_spec()`
(`crates/opi-agent/src/agent.rs:191-206`,
`crates/opi-agent/src/agent_loop.rs:767-799`, and
`crates/opi-agent/src/loop_types.rs:122-147`). A candidate can select a valid
text model while enabling unsupported thinking, be applied, and be observed by
stop; the capability error appears only on a later provider dispatch. Existing
rollback coverage changes only the model to an unknown route.

Recommended fix: validate route plus all candidate request/capability
constraints through one shared validator before assignment; add unsupported
thinking and invalid inference-setting rollback tests.

```yaml
id: P17-SPEC-004
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex.md
source_model: codex
independence: unknown
axis: spec
severity: Major
criterion_source: "P17-NXT-002; task 17.2 DoD; Phase 17 entire-candidate validation contract"
reproduction:
  - "git grep -n -E 'validate_state|validate_dispatchable_route|state = candidate|pub thinking|pub max_tokens|pub temperature' eb5e3166834f804c9b47f5d17f8131652931c601 -- crates/opi-agent/src/agent.rs crates/opi-agent/src/agent_loop.rs crates/opi-agent/src/loop_types.rs"
confidence: high
status: unverified
```

### P17-SPEC-005 — Parallel tools execute before authorization evidence can fail closed

**Severity: Major.** The sequential path emits authorization evidence before
execution. The parallel path passes `tool_evidence = None`, runs all futures
through `join_all`, and emits records only after the tools finish
(`crates/opi-agent/src/agent_loop.rs:430-479,515-566,1339-1416`). Because tools
default to parallel (`crates/opi-agent/src/tool.rs:58-61`), a custom trusted
side-effecting tool can execute before an authorization-record sink failure
advances health. This violates the required deterministic source-order
`resolve → hook → schema → authorize → emit → freshness check → execute` handoff.

Recommended fix: preflight every call in source order through authorization,
evidence emission, and freshness validation; then launch only fresh allowed
parallel calls.

```yaml
id: P17-SPEC-005
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex.md
source_model: codex
independence: unknown
axis: spec
severity: Major
criterion_source: "P17-AUT-003, P17-EVD-009, P17-A12, PRIN-004, INV-005"
reproduction:
  - "git show eb5e3166834f804c9b47f5d17f8131652931c601:crates/opi-agent/src/agent_loop.rs | Select-Object -Skip 429 -First 140"
confidence: high
status: unverified
```

### P17-SPEC-006 — Provider evidence loses auth provenance and fabricates the actual wire

**Severity: Major.** `ResolvedAuth` carries named environment/store/OAuth and
fallback details (`crates/opi-ai/src/auth.rs:105-170`), but the Agent/product
reduce them to four source tokens and a boolean-like fallback
(`crates/opi-agent/src/agent_loop.rs:1581-1603`,
`crates/opi-agent/src/evidence.rs:627-650`, and
`crates/opi-coding-agent/src/evidence.rs:601-645`). Separately, provider response
metadata has no exact `WireApi`; finalization assigns
`actual.wire = configured.wire` and clears the unknown reason
(`crates/opi-ai/src/message.rs:28-38`,
`crates/opi-coding-agent/src/harness.rs:2897-2919`, and
`crates/opi-coding-agent/src/evidence.rs:437-440`). Offline evidence therefore
cannot reproduce auth selection and can falsely attest the configured wire as
the wire actually used.

Recommended fix: carry full redacted typed auth provenance and exact
provider-observed wire metadata; if the actual wire is unavailable, preserve a
typed unknown reason rather than copying configured state.

```yaml
id: P17-SPEC-006
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex.md
source_model: codex
independence: unknown
axis: spec
severity: Major
criterion_source: "P17-PRV-005, P17-EVD-003, P17-EVD-004, docs/opi-spec.md CTRL-002"
reproduction:
  - "git grep -n -E 'auth_source_token|auth_fallback_token|fallback_allowed|actual_route_from_messages|configured.wire' eb5e3166834f804c9b47f5d17f8131652931c601 -- crates/opi-ai/src/auth.rs crates/opi-agent/src crates/opi-coding-agent/src/evidence.rs crates/opi-coding-agent/src/harness.rs"
confidence: high
status: unverified
```

### P17-SPEC-007 — Automatic compaction ignores evidence failure and records no terminal outcome

**Severity: Major.** `persist_turn` discards the result of
`emit_compaction_evidence` and unconditionally calls `execute_compaction`
(`crates/opi-coding-agent/src/harness.rs:2686-2754`). The emitted record contains
only the reason (`crates/opi-agent/src/agent.rs:161-188`) and is never completed
with success, abort, or failure. Under required-complete policy, a sink failure
can therefore be followed by a not-yet-launched session mutation, while failed
or empty compaction still lacks the required terminal graph outcome.

Recommended fix: abort before session mutation on evidence failure and model
compaction as start plus typed terminal outcome records; propagate the outcome
to run finalization.

```yaml
id: P17-SPEC-007
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex.md
source_model: codex
independence: unknown
axis: spec
severity: Major
criterion_source: "P17-EVD-002, P17-EVD-009, P17-A12, P17-FAL-003, PRIN-004"
reproduction:
  - "git show eb5e3166834f804c9b47f5d17f8131652931c601:crates/opi-coding-agent/src/harness.rs | Select-Object -Skip 2685 -First 75"
  - "git show eb5e3166834f804c9b47f5d17f8131652931c601:crates/opi-agent/src/agent.rs | Select-Object -Skip 160 -First 30"
confidence: high
status: unverified
```

### P17-SPEC-008 — Finalization failure cannot advance Agent Core evidence health

**Severity: Minor.** Evidence health is loop-local and not returned
(`crates/opi-agent/src/agent.rs:406-440`). Product finalization occurs afterward
(`crates/opi-coding-agent/src/harness.rs:2836-2894`), so a finalization failure
returns an error and withholds the manifest but cannot perform the specified
core-owned `advance_on_failure(Finalization)` transition. The external behavior
is fail-closed, but the registered core lifecycle/state-transition claim is not
implemented.

Recommended fix: move finalization into the core-owned lifecycle or return the
health state and explicitly advance it on finalization failure.

```yaml
id: P17-SPEC-008
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex.md
source_model: codex
independence: unknown
axis: spec
severity: Minor
criterion_source: "P17-EVD-008; tasks 17.6 and 17.7 DoD"
reproduction:
  - "git grep -n -E 'EvidenceHealth::healthy|finalize_evidence_run|finalize_run|advance_on_failure' eb5e3166834f804c9b47f5d17f8131652931c601 -- crates/opi-agent/src crates/opi-coding-agent/src"
confidence: high
status: unverified
```

### P17-SPEC-009 — Three-platform phase-exit evidence does not cover the audited revision

**Severity: Major.** The ledger records Linux/macOS/Windows success at
`40f2e6ee4866f1cd44eefb952b8f40afcbb029ac`
(`docs/snapshots/phase17/opi-impl-state.json:2260,2659,2804`), but the audited
revision is `eb5e316...` and includes later Phase 17 runtime/test remediation at
`211aba87fcf89bacea72fec7ac2c874df45a9aa3`. The required current implementation
therefore lacks immutable three-platform evidence, so the 70/70 phase-exit
claim is not reproducible for this target.

Recommended fix: rerun the complete required matrix and six gates at the
audited implementation commit, then record the immutable run through the owning
workflow.

```yaml
id: P17-SPEC-009
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex.md
source_model: codex
independence: unknown
axis: spec
severity: Major
criterion_source: "P17-PLT-001, P17-A15, P17-RBK-001, PRIN-005"
reproduction:
  - "git grep -n -E '31798070731|40f2e6ee4866f1cd44eefb952b8f40afcbb029ac' eb5e3166834f804c9b47f5d17f8131652931c601 -- docs/snapshots/phase17/opi-impl-state.json"
  - "git log --format='%H %s' 40f2e6ee4866f1cd44eefb952b8f40afcbb029ac..eb5e3166834f804c9b47f5d17f8131652931c601 -- crates/opi-ai crates/opi-agent crates/opi-coding-agent"
confidence: high
status: unverified
```

## Correctness, security, tests, invariants, integration, and residuals

### P17-INV-001 — Partial-side-effect and cleanup-unknown terminal outcomes are unreachable

**Severity: Major.** The manifest declares `PartialSideEffect` and
`CleanupUnknown` (`crates/opi-agent/src/evidence.rs:749-763`), but production
never constructs them. Tool execution exposes only generic failure/cancellation
and flattens errors into ordinary `ToolResult`s
(`crates/opi-agent/src/tool.rs:111-118` and
`crates/opi-agent/src/agent_loop.rs:1453-1477`); the harness maps all
non-cancellation loop errors to `Failed`
(`crates/opi-coding-agent/src/harness.rs:90-97`). A run with uncertain external
effects can therefore finish as generic failure—or later success—without the
admitted terminal classification.

Recommended fix: add typed partial-effect and cleanup-unknown outcomes at the
owning execution boundary and propagate them through loop and manifest
finalization, with behavioral tests for both variants.

```yaml
id: P17-INV-001
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex.md
source_model: codex
independence: unknown
axis: invariants
severity: Major
criterion_source: "P17-FAL-003; docs/opi-spec.md INV-006"
reproduction:
  - "git grep -n -E 'TerminalOutcome::PartialSideEffect|TerminalOutcome::CleanupUnknown' eb5e3166834f804c9b47f5d17f8131652931c601 -- crates"
  - "git show eb5e3166834f804c9b47f5d17f8131652931c601:crates/opi-agent/src/agent_loop.rs | Select-Object -Skip 1452 -First 30"
confidence: high
status: unverified
```

### P17-INV-002 — `RunId` is reused after every process restart

**Severity: Minor.** `RunId` is documented as stable and non-reused, but it is
allocated from a process-local `AtomicU64` initialized to zero
(`crates/opi-agent/src/evidence.rs:49-53,97-115,156-158`). Every fresh CLI
process can therefore emit `RunId(1)`. Tests cover multiple allocators only
within one process (`crates/opi-agent/tests/evidence_contract.rs:121-129`). File
bundle directory uniqueness reduces local collision risk, but offline
correlation cannot treat `RunId` itself as non-reused.

Recommended fix: use a process-independent opaque 128-bit identity and keep
run-local counters only for child identities; test two subprocesses.

```yaml
id: P17-INV-002
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex.md
source_model: codex
independence: unknown
axis: invariants
severity: Minor
criterion_source: "P17-EVD-001; docs/opi-spec.md CTRL-001"
reproduction:
  - "git show eb5e3166834f804c9b47f5d17f8131652931c601:crates/opi-agent/src/evidence.rs | Select-Object -Skip 48 -First 112"
confidence: high
status: unverified
```

### P17-SEC-001 — Raw provider strings cross diagnostic and public Agent error boundaries

**Severity: Major.** Arbitrary provider-supplied strings are copied into
`Diagnostic.details` (`crates/opi-agent/src/diagnostic.rs:671-704,730-785`),
stored unchanged by public `RecordingSink`
(`crates/opi-agent/src/diagnostic_sink.rs:41,72-89`), and converted with
`e.to_string()` into public `AgentError::Provider(String)`
(`crates/opi-agent/src/agent_loop.rs:882-895` and
`crates/opi-agent/src/agent.rs:212-219,406-432`). Concrete HTTP adapters use
`safe_excerpt`, but the public `Provider` contract does not enforce it for
custom providers or arbitrary request/stream errors.

Recommended fix: classify and redact at the producer boundary, expose only a
stable class plus safe summary, and keep raw causes unavailable through public
`Display` and diagnostic snapshots. Add a non-pattern secret canary test using
a custom provider.

```yaml
id: P17-SEC-001
source_kind: audit
source_path: docs/snapshots/phase17/audit.codex.md
source_model: codex
independence: unknown
axis: security
severity: Major
criterion_source: "P17-FAL-004; docs/opi-spec.md CTRL-003; Phase 17 producer-boundary redaction contract"
reproduction:
  - "git show eb5e3166834f804c9b47f5d17f8131652931c601:crates/opi-agent/src/diagnostic.rs | Select-Object -Skip 670 -First 116"
  - "git show eb5e3166834f804c9b47f5d17f8131652931c601:crates/opi-agent/src/agent_loop.rs | Select-Object -Skip 881 -First 20"
confidence: high
status: unverified
```

## Test-quality assessment

The existing tests strongly cover sequential authorization call counts,
stale-health denial, complete-state replacement visibility, route reuse across
retry, legacy byte identity, lifecycle setup/finalization failures, cross-mode
dispatch, and rollback snapshots. They do not cover the negative paths that
produce the findings above:

- request/route identity mismatch;
- stream-time credential rejection followed by another attempt;
- unsupported inference settings in an otherwise valid next-state candidate;
- authorization-evidence failure before an all-parallel batch;
- exact actual-wire disagreement;
- automatic-compaction evidence failure before mutation and terminal outcome;
- production partial-side-effect and cleanup-unknown finalization;
- cross-process `RunId` uniqueness;
- a non-pattern provider-body canary through raw diagnostics and public errors.

The absence of these focused adversarial tests explains why all local gates can
pass while the registered semantic contracts remain unmet.

## Parent invariant matrix

| Invariant | Status | Audit result |
|---|---|---|
| `PRIN-001` | Partial | Core evidence types are useful, but product-specific closed enums and bypassable validation weaken the deletion/depth case. |
| `PRIN-002` | Partial | The evidence seam has multiple adapters, but duplicated string conversion is not one stable conformance boundary. |
| `PRIN-003` | Fail | Reference Product run-mode and capability policy appears in Agent Core. |
| `PRIN-004` | Fail | Parallel authorization evidence and automatic compaction fail open with respect to not-yet-launched effects. |
| `PRIN-005` | Fail | Required three-platform evidence predates later runtime remediation. |
| `CTRL-001` | Fail | `RunId` is reused across processes. |
| `CTRL-002` | Fail | Exact actual wire and full authentication/fallback provenance are not retained. |
| `CTRL-003` | Fail | Arbitrary provider strings reach public diagnostic/error surfaces unredacted. |
| `INV-001` | Fail | Bedrock bypasses collection-owned runtime route/auth semantics; request-route mismatch is accepted. |
| `INV-002` | Pass | Provider wire implementations remain behind provider-neutral request/stream interfaces. |
| `INV-003` | Pass in runtime / Fail in docs | Runtime ordering is fixed and tested; published README ordering is wrong. |
| `INV-004` | Partial | One atomic complete value is applied, but only its route is validated. |
| `INV-005` | Fail | Parallel effects can start before authorization evidence/freshness completes. |
| `INV-006` | Fail | Partial-side-effect and cleanup-unknown terminal states cannot be produced. |
| `INV-007` | Pass | No active-branch, parent-link, leaf, or crash-recovery contradiction was found. |
| `INV-008` | Fail | Actual route/auth facts can be lost or fabricated, and manifest validation is bypassable. |

## Per-task verdicts

| Task | Verdict | Governing findings |
|---|---|---|
| 17.1 | **FAIL** | `P17-SPEC-001`, `P17-SPEC-002`, `P17-SPEC-003` |
| 17.2 | **FAIL** | `P17-SPEC-004` |
| 17.3 | **FAIL** | `P17-STD-002`, `P17-STD-004`, `P17-INV-002` |
| 17.4 | **FAIL** | `P17-STD-002`, `P17-SPEC-005` |
| 17.5 | **FAIL** | `P17-SPEC-001` |
| 17.6 | **FAIL** | `P17-STD-003`, `P17-SPEC-005`, `P17-SPEC-008` |
| 17.7 | **FAIL** | `P17-STD-004`, `P17-SPEC-006`, `P17-SPEC-007`, `P17-INV-001`, `P17-SEC-001` |
| 17.8 | **PASS** | No contradiction found in unique legacy-route normalization, fail-closed ambiguous/missing routes, or byte preservation. |
| 17.9 | **FAIL** | `P17-STD-001`, `P17-STD-005`, `P17-SPEC-009`; the acceptance matrix omits the negative paths above. |

## Minimum-change overlay

`R/P/S/C` means reuse search, placement, surface necessity, and simplification
ceiling. `Drifted` records a current implementation that no longer satisfies
its registered overlay even where the ledger field itself is present.

| Task | R | P | S | C | Status | Reason |
|---|---:|---:|---:|---:|---|---|
| 17.1 | Pass | Pass | Fail | Fail | Drifted | The unadmitted Bedrock exemption broadens the prepared-auth contract while required mismatch/termination outcomes are absent. |
| 17.2 | Pass | Pass | Pass | Pass | Conforming | One complete state and one assignment are retained; the failure is semantic validation depth, not surface growth. |
| 17.3 | Pass | Fail | Partial | Pass | Drifted | Product-specific assembly/capability values entered the neutral core evidence vocabulary. |
| 17.4 | Pass | Partial | Pass | Pass | Drifted | Product capability families leaked into core and the parallel path bypasses the shared preflight. |
| 17.5 | Partial | Pass | Fail | Fail | Drifted | Bedrock retains eager provider-owned credential state and a placeholder resolver. |
| 17.6 | Partial | Pass | Pass | Pass | Drifted | Runtime evidence duplicates a string protocol rather than reusing typed route/provenance facts. |
| 17.7 | Partial | Pass | Partial | Pass | Drifted | Product parsing invents defaults and file finalization can bypass strict validation. |
| 17.8 | Pass | Pass | Pass | Pass | Conforming | No compatibility shim, legacy trace reader, or rewrite/down-conversion path was found. |
| 17.9 | Pass | Pass | Pass | Pass | Drifted | Assurance-only placement is retained, but current docs and platform evidence do not describe/prove the audited implementation. |

## Verification

The first PTY-backed invocation of the full smoke script was terminated after
the PTY kept `adapter_host_mock` stdin open. The identical command was rerun
without a PTY and completed successfully.

- `python scripts/opi-cargo-cache.py status` — passed; external Cargo caches remained enabled.
- `cargo test -p opi-agent --test evidence_contract --test evidence_runtime --test tool_authority --test agent_loop_semantics --test phase17_prepare_call` — 65 passed, 0 failed.
- `cargo test -p opi-coding-agent --test phase17_api_audit --test phase17_artifact_truthfulness --test phase17_cross_mode --test phase17_failure_rollback --test phase17_legacy_migration --test phase17_product_evidence --test phase17_provider_runtime --test phase17_tool_authority` — 55 passed, 0 failed.
- `cargo test -p opi-ai --test provider_diagnostics --test provider_error_classes` — 36 passed, 0 failed.
- `powershell -ExecutionPolicy Bypass -File scripts\\opi-impl-smoke.ps1 full` — passed: format, Clippy all targets, rustdoc, and workspace all-target tests; final output `=== smoke PASSED [full] ===`.
- `python scripts/opi-doc-check.py` — passed: `opi documentation contracts: PASS`.

Test impact: **none**. This audit adds only this report and does not change
runtime, tests, specifications, or the implementation ledger.

## Residual risk and next action

The green full gate establishes that the committed suite is internally
consistent; it does not rebut the falsifiable negative paths above. The highest
priority remediation order is:

1. make tool authorization/evidence and automatic compaction fail closed before
   side effects;
2. restore truthful typed provider/auth/actual-wire evidence and enforce
   validated manifests at the sink boundary;
3. close the prepared-call mismatch/credential-terminal gaps and remove the
   Bedrock exception;
4. validate the complete next-turn candidate and propagate partial/unknown
   outcomes;
5. add the listed adversarial tests, synchronize both READMEs, and rerun the
   required three-platform evidence profile at the remediated commit.

No source change should be admitted as Phase 17 remediation without revisiting
the registered spec if maintainers intend to retain the Bedrock exception,
product-specific core enums, lossy provenance, or configured-as-actual wire
behavior.
