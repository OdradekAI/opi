# Phase 17: Deep Agent Core Semantic Closure

## Status and authority

This document is the human-reviewed Phase 17 delivery specification. It derives
one finite delivery from the durable direction in
[`docs/opi-spec.md`](../../opi-spec.md) and is a registered supplemental source
for `opi-implement`. An executable task graph is admitted only after the
explicit `opi-implement` plan workflow maps its requirements into the
implementation ledger and passes the human graph gate. This document does not
record progress, completion, task status, or release state.

The normative parent remains `docs/opi-spec.md`. Domain language remains owned
by [`docs/CONTEXT.md`](../../CONTEXT.md). Current product and protocol facts
remain owned by source, crate documentation, schemas, fixtures, generated help,
and manifests. The implementation ledger and `docs/snapshots/` remain the only
owners of delivery status and completed Phase history.

| ID | Requirement | Owner | Mechanical verification |
|---|---|---|---|
| P17-AUTH-001 | Phase 17 **MUST** implement the parent clauses cited below without lowering, reinterpreting, or bypassing their gates. | Phase lead | Admission mapping and clause-by-clause design review. |
| P17-AUTH-002 | Implementation status, task state, dates, and completion claims **MUST NOT** be written into this document or `docs/opi-spec.md`. | Implementation workflow owner | Documentation diff review and `scripts/opi-doc-check.py`. |
| P17-AUTH-003 | Any implementation discovery that makes a parent clause infeasible **MUST** stop delivery for an explicit specification revision rather than introduce a silent exception. | Phase lead | Blocked handoff and route-revision review. |

## Parent-clause traceability

Phase 17 derives the currently highest-priority admissible unmet goal,
`STRAT-001 — Close deep Agent Core semantic gaps`.

| Parent clause | Phase 17 responsibility |
|---|---|
| `PRIN-001`–`PRIN-003` | Deepen existing owners, keep mechanism below product policy, and reject speculative runtime facades or crates. |
| `PRIN-004` | Fail closed at provider, state, authority, and evidence boundaries. |
| `PRIN-005` | Make resolved execution and evidence completeness mechanically verifiable. |
| `CTRL-001` | Propagate stable run, turn, and call correlation plus finalized artifact references without binding an exporter. |
| `CTRL-002` | Retain the resolved harness/runtime/adapter, route, policy, source, budget, trigger, environment, measurement, and artifact provenance needed to verify one execution. |
| `CTRL-003` | Classify and redact sensitive evidence before it crosses the Agent Core evidence boundary; do not enable capture merely for a future Eval consumer. |
| `INV-001`–`INV-002` | Make provider collection ownership and provider-neutral runtime dispatch true for every model call. |
| `INV-003`–`INV-004` | Replace the complete mutable next-request state atomically and fix the ordering of preparation, stop evaluation, and queue polling. |
| `INV-005` | Validate registered capability, permission scope, final schema, and trusted authority before a tool can cause side effects. |
| `INV-006` | Preserve visible cancellation, queue, evidence, and partial-failure outcomes. |
| `INV-007` | Preserve existing session reconstruction and crash-recovery semantics while changing runtime routing. |
| `INV-008` | Bind finalized evidence to the session branch, resolved product configuration, effective User Policy, and an explicit runtime-input binding; current direct assembly is labelled `DirectRuntimeInput`, while only a Promotion Controller may supply an `ActiveSnapshot` reference. |
| `PHASE-001`–`PHASE-006` | Keep this delivery finite, placed, testable, reversible, and subordinate to the parent route. |

The minimum evidence seam delivered here is an explicit prerequisite for
`STRAT-002`. Phase 17 does not pull independent Eval into the Agent Core and
does not implement any part of the Eval product.

## Current implementation gap

The current code contains most of the necessary nouns but does not yet make
their semantics true end to end:

- [`ProviderCollection`](../../../crates/opi-ai/src/provider_collection.rs)
  already owns registry lookup, authentication metadata, compatibility
  metadata, and stream dispatch. The Reference Product nevertheless constructs
  only one active runtime provider and uses metadata-only entries for the rest;
  cross-provider model changes are consequently rejected in
  [`provider_factory.rs`](../../../crates/opi-coding-agent/src/provider_factory.rs)
  and [`harness.rs`](../../../crates/opi-coding-agent/src/harness.rs).
- [`Agent`](../../../crates/opi-agent/src/agent.rs) owns one shared provider and
  a separate model string. The loop therefore cannot treat model selection as
  provider routing.
- [`AgentLoopTurnUpdate`](../../../crates/opi-agent/src/loop_types.rs) can only
  append messages. In
  [`agent_loop.rs`](../../../crates/opi-agent/src/agent_loop.rs), stop evaluation
  runs before next-turn preparation, contrary to the parent state-transition
  contract.
- [`BeforeToolCallResult`](../../../crates/opi-agent/src/hooks.rs) uses the word
  `Allow` for a non-authoritative hook result, and the core has no separately
  injected trusted authorization decision at the final side-effect boundary.
- [`TraceRecord`](../../../crates/opi-agent/src/trace.rs) has run and optional
  turn correlation but no stable provider/tool call graph, immutable resolved
  execution manifest, or finalized artifact references. Its file-oriented core
  adapter also conflates the core vocabulary with one storage choice.
- [`AgentHarness`](../../../crates/opi-agent/src/harness.rs) is a published but
  unused Phase 10 seam that owns a second runtime-configuration snapshot while
  exposing no prompt, continue, or step operation. The Reference Product drives
  [`Agent`](../../../crates/opi-agent/src/agent.rs) and
  [`SessionCoordinator`](../../../crates/opi-coding-agent/src/session_coordinator.rs)
  directly, so adopting this harness would add another state owner rather than
  close the current loop.

The pinned pi 0.84.1 source and the current
[realignment report](../../realign/2026-08-11-opi-vs-pi.codex.md) are inward
evidence for the provider and next-turn gaps. In particular, pi's `Models`
collection resolves request authentication before delegating to the selected
provider and retains a non-secret auth-source label; its loop orders
`prepareNextTurn` before stop and queue polling; and its coding-agent registry
retains registration-owned `SourceInfo` separately from the tool definition.
Those are useful ownership patterns, not compatibility requirements. Opi
deliberately requires complete Rust state replacement rather than pi's optional
field patch, adds a mandatory authority boundary that pi does not provide, and
does not adopt pi's still-partial `AgentHarness` surface. The
[local Eval foundation design](../../superpowers/specs/2026-08-11-opi-local-eval-foundation-design.md)
and benchmark research are non-normative evidence and remain outside this
Phase.

## Outcome

After Phase 17, the existing provider, Agent loop, tool, and observation
abstractions describe what the process actually does:

1. each model selection resolves the real provider and wire used for that call;
2. next-turn preparation replaces the entire mutable request state in one
   validated transition before stop and queue decisions consume it;
3. every tool invocation crosses a trusted, fail-closed authorization boundary
   after final schema validation and before execution; and
4. one product-neutral evidence seam correlates the run, turns, provider calls,
   tool calls, retries, compaction, outcomes, and finalized artifacts with the
   resolved execution that produced them.

| ID | Requirement | Owner | Mechanical verification |
|---|---|---|---|
| P17-OUT-001 | One Agent session **MUST** be able to route successive model calls to different registered providers without reconstructing the Agent solely because the provider changes. | `opi-ai` and `opi-agent` maintainers | Cross-provider state-transition integration test with two mock providers. |
| P17-OUT-002 | A next-turn transition **MUST** expose either the complete validated replacement state or the unchanged prior state; partially applied updates **MUST NOT** be observable. | `opi-agent` maintainer | Failure-injection and state-snapshot tests. |
| P17-OUT-003 | A tool **MUST NOT** execute unless its registered identity, capability, final arguments, and effective permission have passed the trusted authorization chain. | Agent runtime owner | Source-to-sink negative authorization tests with execution counters. |
| P17-OUT-004 | Finalized evidence **MUST** identify the resolved execution, its authority and runtime-input binding, its call correlations, and its finalized artifacts without requiring an exporter. | Agent Core observability owner | In-memory evidence conformance and manifest validation tests. |

## Non-goals

Phase 17 excludes:

- adding providers, models, wire implementations, or catalogue breadth;
- independent Eval, benchmark runners, Agent/Grader adapters, ATIF, RunBundle,
  graders, paired reports, or offline report recomputation;
- exporters, hosted telemetry, dashboards, evidence databases, or a new core
  file-storage abstraction;
- permission popups, new policy authoring UX, remote sessions, MCP, sub-Agents,
  plan/todo workflows, or proactive scheduling;
- a new generic tool-permission configuration language or a second package
  permission store; Phase 17 snapshots and enforces already-resolved product
  policy inputs and fails closed when no permission exists;
- a unified `AgentRuntime` facade, a new workspace crate, or a compatibility
  layer for replaced 0.x interfaces;
- pi TypeScript/npm compatibility or adoption of pi's package structure; and
- changes to `opi-sandbox` restriction levels or the `opi-protocol` command
  execution contract.

## Architecture placement case

| Capability | Placement | Why it belongs there | Deletion test and seam evidence |
|---|---|---|---|
| Runtime provider dispatch | `opi-ai` Agent Core | Model selection, provider ownership, per-call auth preparation, capability lookup, and wire dispatch are provider-neutral LLM semantics. | Removing collection dispatch would return provider lookup, auth preparation, and route checks to every Agent caller. The existing registry/collection and multiple provider adapters already prove the seam. |
| Atomic next-turn state | `opi-agent` Agent Core | It is an intrinsic Agent state transition consumed by compaction, model choice, stop logic, and queues. | Removing it would force each harness to reimplement partial state mutation and event ordering. |
| Trusted tool authorization | `opi-agent` mechanism with Reference Product policy | The final pre-execution transition is intrinsic to the Agent loop; the policy that decides it remains product-owned. | Removing the mechanism would duplicate the safety boundary in every harness. Keeping policy behind an injected authorizer prevents product dependencies from entering core. |
| Evidence context and sink | `opi-agent` Agent Core | Run/turn/call identity and lifecycle finalization are intrinsic to provider, turn, and tool state transitions. | Removing the seam would duplicate correlation and finalization across trace, RPC, future Eval, and embedders. No-op and in-memory core adapters plus the Reference Product file adapter share conformance. |
| File capture and product manifest inputs | `opi-coding-agent` Reference Product | Paths, CLI activation, session branch selection, resolved harness configuration, environment allowlists, runtime-input binding, and effective User Policy are product facts. | Removing this adapter does not change Agent Core semantics; another harness can supply different artifact and policy bindings. |

The placement introduces no new crate. The only new public seams are intrinsic
state-machine boundaries: complete next-turn replacement, trusted tool
authorization, and product-neutral evidence delivery. If implementation needs a
broader generic runtime, service locator, policy engine, exporter API, or shared
mutable evidence store, the placement case no longer holds and
`P17-AUTH-003` applies.

## End-to-end runtime model

```text
Trusted Reference Product assembly
    ├── dispatchable ProviderCollection
    ├── initial NextTurnState
    ├── registered tools + registration-owned capability identities
    ├── ToolAuthorizer bound to EffectiveUserPolicy
    ├── session branch + RuntimeInputBinding
    └── EvidenceSink
                 │
                 ▼
Agent turn
    resolve provider route
    → provider call / retry
    → proposed tool calls
    → non-authoritative deny hook
    → final schema validation
    → trusted authorization
    → tool execution
    → turn finalization
    → prepare and atomically apply NextTurnState
    → stop evaluation
    → steering / follow-up polling
                 │
                 ▼
finalized resolved-execution manifest + artifact references
```

The run binding is immutable. It includes system identity/instructions, tool
registrations and implementations, the explicit runtime-input binding,
effective User Policy and authorizer, session branch, evidence sink, and hard
budget ceilings. Next-turn preparation can change only the mutable state
defined below. Tool visibility is projected again from trusted policy for each
request and is never hook-writable state.

## Dispatchable provider collection

### Runtime ownership

`ProviderCollection` remains the deep `opi-ai` owner. The Agent receives a
shareable collection rather than one provider plus unrelated metadata. Product
assembly registers every provider that can actually dispatch and removes the
metadata-only active-provider proxy.

Each collection route contains the concrete provider plus its per-request
`AuthResolver`. Concrete provider adapters consume already-resolved auth; they
no longer own or invoke an unrelated resolver after collection dispatch. The
semantic dispatch boundary is async and prepares one logical model call,
including all of its retry attempts:

```text
ProviderCollection::prepare_call(selection, request)
    -> Result<PreparedProviderCall, CollectionError>

PreparedProviderCall
├── resolved_route: immutable, redacted route facts
└── start_attempt() -> provider event stream
```

`PreparedProviderCall` privately retains the frozen request, selected provider,
and resolved secret-bearing authentication. `start_attempt` may be called again
for a sequential retry, but it neither repeats route/auth preparation nor lets
the Agent inspect or replace the prepared secret. The final provider adapter
interface accepts the already-resolved authentication value with the prepared
request; concrete adapters no longer store or call an `AuthResolver`. This is
the one reviewed breaking provider seam for Phase 17.

Delivery uses an explicit expand-contract sequence. The expand step adds the
prepared-auth attempt method and migrates in-workspace concrete adapters while
temporarily retaining the old `stream(Request)` entry for still-unmigrated
Agent and Reference Product callers. The Agent cutover uses only prepared
calls. The Reference Product contract step migrates its remaining provider and
test implementations, then removes the old method and resolver-bearing adapter
state. A default bridge, if mechanically required during the expand step, is
temporary substrate only and must not remain after that contract step.

The implementation may use existing Rust types and crate-private prepared
request values, but it does not add a router trait, retry-factory trait, or a
second registry. The public collection and opaque prepared call are the runtime
dispatch surface. The provider adapter receives secret-bearing auth only at its
wire boundary; the secret never enters `resolved_route`, Agent-visible state,
evidence, diagnostics, or model-visible state.

At most one attempt stream is active for a prepared call. The frozen request's
CancellationToken governs every attempt; cancellation ends the logical call and
does not permit another attempt. Dropping the call drops its secrecy-wrapped
authentication without exposing it. A credential rejection or expiry ends the
logical call rather than refreshing inside a retry; credential remediation and
the explicit user-level retry path prepare a new call.

The internal model selection is canonical `provider:model`. Phase 17 does not
add an alias registry or new alias configuration. A currently supported or
legacy bare model is normalized before it enters Agent state only when the
product can prove exactly one valid route; the canonical selection and bare
source are retained as separate facts.

### Per-call route and auth preparation

Before model-request HTTP dispatch for one model call, the collection resolves
and freezes:

- requested selection and its source;
- provider identifier;
- model identifier;
- provider wire/API kind;
- non-secret authentication source classification and credential kind;
- catalogue/profile/configuration source; and
- fallback decision and reason.

Authentication preparation may itself perform credential-store IO or a locked
OAuth refresh. Those effects are part of auth preparation and remain visible as
such; the implementation must not claim that no external effect occurred merely
because the model request has not started. A failed stored credential or OAuth
refresh never falls back silently. An environment fallback is permitted only
where the already-reviewed product auth policy explicitly allows it, and that
decision is retained in the route facts.

`ResolvedAuth` therefore carries a non-secret provenance value beside its
secret wire material. Its semantic facts are closed and typed:

```text
AuthProvenance
├── source: static | environment(name) | credential-store(kind) | oauth(kind)
└── fallback
    ├── not-attempted
    └── used { from, to, stable_reason }
```

Provider-specific human-readable labels may supplement these facts but cannot
replace the closed source/fallback classification. No secret value, raw
environment value, token, or credential-store payload enters provenance.

The request's model and capabilities are validated against that same resolved
entry before auth preparation can reach model-request dispatch. All retry
attempts start from the same `PreparedProviderCall`, reuse its immutable route,
request, and resolved authentication, and never re-run auth preparation inside
the logical call. A later user-level retry after credential remediation is a
new logical call and prepares again. The provider response's actual
provider/model/wire metadata is retained separately from the requested and
resolved route; disagreement is visible and cannot be normalized away.

| ID | Requirement | Owner | Mechanical verification |
|---|---|---|---|
| P17-PRV-001 | Agent model calls **MUST** perform route lookup, per-call auth preparation, and dispatch through the registered runtime provider collection rather than a startup-selected provider object. | `opi-ai` and `opi-agent` maintainers | Source-structure assertion and two-provider dispatch test. |
| P17-PRV-002 | Internal model selection **MUST** identify one canonical provider and model before dispatch; an ambiguous bare or legacy selection **MUST** fail before a provider is touched, and Phase 17 **MUST NOT** add an alias registry. | Reference Product provider owner | Bare-selection normalization, ambiguity, source-structure, and provider-call-count tests. |
| P17-PRV-003 | One model call and all of its retries **MUST** retain one immutable resolved route and one prepared authentication result; retries **MUST NOT** invoke route or auth preparation again. | `opi-agent` maintainer | Retry test asserting identical route identity, one resolver call, multiple attempt streams, and parent call correlation. |
| P17-PRV-004 | A route or auth-preparation failure **MUST NOT** silently fall back to another provider, wire, model, credential policy, or local implementation; an allowed environment fallback **MUST** retain its typed reason. | `opi-ai` maintainer | Missing-auth, failed-refresh, allowed-env-fallback, provider-error, and wire-mismatch negative tests. |
| P17-PRV-005 | Requested, resolved, provider-reported actual route, auth source, and fallback facts **MUST** remain distinguishable in evidence. | Agent Core observability owner | Manifest fixture, auth-provenance fixture, and mismatched-response metadata test. |
| P17-PRV-006 | Provider-specific request encoding and response decoding **MUST** remain behind the existing provider-neutral request, stream, usage, and capability interfaces. | `opi-ai` maintainer | Provider fixture suite and Cargo dependency review. |

## Atomic next-turn state

### State boundary

`AgentLoopTurnUpdate { extra_messages }` is replaced by a complete state value
with this semantic shape:

```text
NextTurnState
├── context: complete conversation state
├── model_selection: canonical provider:model
└── inference
    ├── thinking
    ├── max_tokens
    └── temperature
```

The precise Rust field decomposition may reuse existing provider request types,
but the replacement semantics cannot become a patch, merge, or append protocol.
The candidate is built away from the live state, validated as a unit, and then
swapped into the loop in one transition.

`Agent` is the sole durable owner of this mutable runtime state. A loop run
returns its final complete state to `Agent`, and `Agent::prompt`/continue stores
that state before the public operation settles. A replacement that exists only
inside one low-level loop invocation does not satisfy this contract. Public
piecemeal setters for model, thinking, token limits, temperature, or messages
are removed or narrowed behind one complete idle-state replacement operation;
they cannot bypass candidate validation.

`prepare_next_turn` receives a snapshot of the current complete state plus the
completed turn outcome. Its semantic result is:

```text
Result<Option<NextTurnState>, AgentError>
```

`None` retains the state. `Some` replaces it. An error or cancellation leaves
the prior state intact and terminates the transition visibly.

### Fixed ordering

```text
turn provider/tool work reaches a terminal outcome
→ finalize the turn outcome and evidence
→ construct the candidate NextTurnState
→ validate the entire candidate
→ atomically replace the state
→ should_stop_after_turn observes the new state
→ if stopped: terminate without polling queues
→ otherwise: poll steering
→ when applicable: poll follow-up
→ resolve the next provider route from the applied state
```

Compaction participates through complete context replacement. It is not a
message append and cannot mutate the current model or inference configuration
outside the same candidate state. Queue input is applied only after the stop
decision permits polling; it cannot resurrect a transition that already failed
or was cancelled.

The existing `opi-agent::AgentHarness`, `HarnessRuntimeConfig`, and related
phase-guard wrapper are not adopted as another state owner. They do not drive a
production turn and duplicate the state that `NextTurnState` closes. Phase 17
removes that unused harness surface and keeps session persistence in the
existing product `CodingHarness`/`SessionCoordinator` path. Product compaction
must enter the same complete-state transition rather than call a direct message
replacement setter after the loop.

| ID | Requirement | Owner | Mechanical verification |
|---|---|---|---|
| P17-NXT-001 | Next-turn preparation **MUST** return a complete replacement for all mutable next-request state rather than a field patch or message append, and the applied state **MUST** persist through the public `Agent` operation. | `opi-agent` maintainer | Public interface review, full-state replacement test, and consecutive-`Agent::prompt` state test. |
| P17-NXT-002 | The candidate state **MUST** be validated before one atomic apply; validation failure or cancellation **MUST** leave context, model selection, and inference settings unchanged. | `opi-agent` maintainer | Failure injection with before/after state snapshots. |
| P17-NXT-003 | Stop evaluation **MUST** run after successful state application and **MUST** observe the applied state. | `opi-agent` maintainer | Hook-order trace and state-observation test. |
| P17-NXT-004 | A stop decision **MUST** terminate before steering or follow-up polling. | `opi-agent` maintainer | Queue probe asserting zero polls on stop. |
| P17-NXT-005 | Product compaction **MUST** replace complete context through the same transition and **MUST NOT** preserve superseded messages through append-only or post-loop direct-setter behavior. | Reference Product harness owner | CodingHarness compaction replacement and token/context fixture tests. |
| P17-NXT-006 | A provider route for the next call **MUST** be resolved only from the successfully applied next-turn state. | `opi-agent` maintainer | Cross-provider next-turn integration test. |

## Trusted tool authorization

### Trusted boundary

Agent Core owns the fact that a final authorization decision occurs. The
Reference Product or embedder owns the policy that makes the decision. The
authorizer is constructed by trusted runtime assembly, bound to the effective
User Policy for the run, and not replaceable by next-turn state or model-visible
content.

Tools enter the loop only through an immutable registration owned by trusted
assembly, not through `Tool::definition()` alone:

```text
RegisteredTool
├── registration_id
├── provider_visible_name
├── origin: builtin | extension(extension_id) | embedder(embedder_id)
├── capability_id
├── definition
└── implementation
```

The registry rejects duplicate provider-visible names. Built-in identities and
capabilities are assigned by the Reference Product. Extension origin is derived
by `ExtensionRegistry` from the extension registration being traversed, in the
same spirit as pi retaining `SourceInfo` beside each registered tool; an
extension cannot replace that origin with a model-visible field. An embedder is
trusted assembly and supplies its registration and authorizer together.

The Reference Product uses this fixed built-in capability map:

| Tools | Capability |
|---|---|
| `read`, `grep`, `find`, `ls`, `glob` | `workspace.read` |
| `write`, `edit` | `workspace.write` |
| `bash` | `command.execute` |

Extension tool capabilities are namespaced to the registration origin and tool
name. A Reference Product extension tool without an existing exact capability
permission is excluded and denied; project or package trust alone is not a
permission. This Phase does not create an implicit allow rule or a new policy
configuration language to preserve an unpermitted extension tool.

### Effective product policy

`EffectiveUserPolicy` is an immutable, digest-addressed product value assembled
from facts the product already resolves: run mode, active-tool selection,
mutating opt-in, project trust, package artifact/trust/activation state,
`command.execute` adapter permission rules, path/operation scope, grant
lifetime rules, and whether complete evidence is required. Live
session-scoped grants are a separately versioned permission state referenced by
the decision; they do not mutate the policy digest. The policy is not a new
authoring format and cannot be supplied by prompts, hooks, extensions, or
next-turn state.

For the current Reference Product, the complete-evidence fact has one closed
mapping and adds no configuration key: absent capture uses the no-op sink and
sets it to false; explicit capture configured by CLI `--trace`, SDK
`TraceConfig`, or the RPC recording sink sets it to true. There is no separate
best-effort capture mode in Phase 17. An embedder that supplies its own trusted
authorizer remains responsible for its policy assembly, but cannot derive this
fact from model-visible or extension-controlled input.

Built-in read and filesystem mutation permissions derive from the existing
tool-selection, mutating, and path-policy decisions. For `bash`, the product
authorizer reuses the same pure route selection and existing adapter permission
policy used by execution, so the permission reference binds the adapter that
the validated arguments will reach. Interactive `ask` reuses the existing
permission broker before `Tool::execute`; headless `ask` remains fail closed.
There is no mutable fallback to the local adapter after authorization.

Its semantic interface is:

```text
ToolAuthorizer::authorize(ToolAuthorizationRequest)
    -> Result<AuthorizationDecision, AuthorizationError>
```

The request contains core-confirmed facts: run/turn/call identity, the resolved
registered tool identity, a trusted registration-derived capability identity,
the final validated arguments, current invocation/session context, and the
current versioned `EvidenceHealth` snapshot. Full arguments may be inspected
inside this trusted boundary; evidence receives only the classified
representation or digest described later.

The authorizer owns or closes over effective User Policy, Capability Permission,
and Permission Scope. A model-provided policy name, grant, scope, capability, or
risk label is ordinary untrusted content and cannot enter the trusted fields.

The decision is a closed state:

```text
Allow {
    policy_ref,
    permission_ref,
    permission_scope,
    registration_id,
    capability_id,
    evidence_health_generation,
}

Deny {
    stable_code,
    redacted_reason,
}
```

### Invocation order

```text
model proposes tool name and arguments
→ resolve one registered tool and its trusted capability identity
→ before_tool_call hook may deny or continue
→ validate the final arguments against the registered schema
→ trusted ToolAuthorizer decides
→ emit the redacted authorization outcome
→ verify the Allow still matches registration, capability, and evidence health
→ only a current Allow reaches Tool::execute
→ after_tool_call may retain or replace the presentation result
```

If evidence emission changes health after an `Allow` was computed but before
execution, the decision is stale: the runtime rebuilds the request with the new
health generation and authorizes again. A policy requiring complete evidence
then denies the current side effect as well as later ones. No authorizer closes
over a shared mutable evidence store. Already-running side effects cannot be
retroactively revoked; their partial or cleanup-unknown outcome remains
evidence.

Tool-call preparation and authorization occur in deterministic source order.
Parallel execution starts only for calls whose authorization was current at
launch. A later health failure blocks calls that have not yet crossed that
launch boundary but does not rewrite the outcome of an in-flight call.

The non-authoritative hook result is renamed from `Allow` to a term such as
`Continue`; no hook result is an authorization grant. Phase 17 does not add
argument mutation to this hook. A future argument-transforming hook would need
to run before final schema and authority validation and requires a separate
specification decision.

A tool that is wholly unavailable under effective policy is omitted from the
model-visible tool projection. Invocation-specific restrictions are still
checked after final arguments are known. A denial becomes a controlled tool
outcome for the model and a trusted evidence outcome, without invoking the
tool. After-call result replacement cannot rewrite the authorization record or
change future policy.

| ID | Requirement | Owner | Mechanical verification |
|---|---|---|---|
| P17-AUT-001 | Every model-proposed tool invocation **MUST** resolve to one immutable trusted registration, registration-owned origin, and capability identity before hook, schema, or authorization processing can lead to execution. | Agent runtime owner | Unknown-tool, duplicate-name, extension-origin, and forged-capability tests. |
| P17-AUT-002 | The final argument value **MUST** pass the registered schema before authorization, and the exact validated value **MUST** be the value passed to execution. | `opi-agent` maintainer | Schema boundary and argument identity tests. |
| P17-AUT-003 | The final authorization decision **MUST** derive only from the trusted authorizer, immutable EffectiveUserPolicy, scoped permission state, and current EvidenceHealth snapshot. | Reference Product policy owner | Negative source-to-sink taint and stale-health reauthorization tests. |
| P17-AUT-004 | Model content, hooks, extensions, skills, tool output, retrieved content, and child-Agent output **MUST NOT** grant permission, weaken policy, change the trusted capability identity, or widen Permission Scope. | Agent runtime owner | Malicious-content matrix with immutable policy assertions. |
| P17-AUT-005 | Missing authorizer or permission state, authorizer error, denial, expired scope, stale evidence-health generation, unavailable capability, or invalid schema **MUST** result in zero calls to `Tool::execute`. | Agent runtime owner | Failure-injection tests using an execution counter. |
| P17-AUT-006 | A non-authoritative pre-tool hook **MUST** be able to deny but **MUST NOT** represent an authority grant. | `opi-agent` maintainer | Enum/API review and hook-chain tests. |
| P17-AUT-007 | After-call result transformation **MUST NOT** alter the recorded authorization decision or effective authority for later calls. | `opi-agent` maintainer | Replacement-result and subsequent-call policy tests. |
| P17-AUT-008 | A tool excluded by effective policy **MUST NOT** appear in the provider-facing tool schema for that request; projection **MUST** be recomputed from trusted registrations for every provider request. Recomputation is registration composition at request build; evidence-health denial surfaces as the authorization boundary's stable-code denial, not as schema omission. | Reference Product policy owner | Consecutive-request tool-projection snapshot tests. |

## Product-neutral evidence seam

### Core vocabulary and adapters

The Agent Core evidence contract replaces the storage-shaped core trace
contract. It uses opaque, stable, non-reused identities:

```text
RunId
TurnId
CallId
optional ParentCallId
monotonic Sequence within the run
```

Provider, tool, retry, compaction, and other call-like activity use `CallId` and
an explicit kind. Parent correlation expresses retry and nested-call
relationships without requiring a future Eval call-graph schema.

`opi-agent` provides only no-op and in-memory adapters. The no-op adapter is the
default and does not enable content capture. The in-memory adapter owns the
conformance oracle. The existing Reference Product file capture remains a
product adapter to the same lifecycle; file path, CLI activation, on-disk
layout, and retention policy do not enter Agent Core.

The lifecycle distinguishes setup, ordered emission, artifact finalization, and
run finalization. A finalized manifest is immutable. A sink failure cannot be
hidden by emitting a normal finalized record through another path.

`EvidenceHealth` is a closed, versioned run-local value owned by Agent Core:

```text
Healthy { generation }
Incomplete { generation, first_failure_code }
```

Only the loop advances it, immediately when setup, emission, or finalization
fails. Sinks do not expose a mutable health handle, and authorizers receive a
copy in each request.

### Resolved-execution manifest

The domain term `Active Snapshot` remains reserved for the immutable inputs
selected by a Promotion Controller. The current Reference Product has no such
controller and must not fabricate that authority. Every run instead carries one
closed runtime-input binding:

```text
RuntimeInputBinding
├── DirectRuntimeInput { digest, assembly_source }
└── ActiveSnapshot { snapshot_ref }
```

Current direct CLI/SDK assembly uses `DirectRuntimeInput`, whose digest covers
the resolved material runtime inputs. `ActiveSnapshot` is accepted only when a
future trusted Promotion Controller supplies its reference. The two variants
are distinguishable in evidence and cannot be normalized into one another.

The final manifest retains, directly or by immutable reference:

- run, turn, call, parent, and sequence correlation;
- terminal outcomes, including cancellation, retry, compaction, partial
  failure, and cleanup uncertainty;
- session branch and the exact `RuntimeInputBinding` variant;
- resolved harness, runtime, adapter, and material configuration identity;
- requested, resolved, and actual provider/model/wire route;
- non-secret authentication, fallback, catalogue, configuration, and source
  provenance;
- effective User Policy digest, capability permission, permission-scope, and
  scoped-grant references;
- prompt, system instruction, tool schema, and input identity through digest or
  already-classified artifact reference;
- budget, trigger, time, platform/environment identity, and measurement origin;
- provider-reported usage separated from estimated, quota, and billed values;
  and
- finalized artifact references and evidence completeness.

An artifact reference contains a logical role, media type, content digest,
location/reference, sensitivity classification, and finalization state. It does
not embed the artifact payload.

### Redaction boundary

Evidence values crossing into the sink are typed structural values, digests,
redacted diagnostics, and classified artifact references. Raw credentials,
environment values, prompts, tool arguments, tool results, and provider error
bodies do not cross by default. Explicit product capture classifies and redacts
content before constructing the evidence value or artifact reference; sink
implementations are not the redaction boundary.

Missing measurements retain `unknown` plus a reason. Zero is used only for a
measured zero. Requested, resolved, actual, estimated, and provider-reported
facts remain distinct.

### Evidence failure

With the no-op adapter, execution behavior is unchanged and no capture is
implied. When the Reference Product explicitly configures capture, setup
completes before the first provider or tool call and its Effective User Policy
requires complete evidence by the fixed mapping above. A mid-run emission or
finalization failure cannot undo already completed external effects, so the
runtime preserves the actual execution outcome while marking evidence
incomplete and withholding a finalized manifest. The trusted authorizer then
rejects an unhealthy generation. An allow computed against an older generation
is reauthorized before execution as defined above.

| ID | Requirement | Owner | Mechanical verification |
|---|---|---|---|
| P17-EVD-001 | Agent Core **MUST** create stable run, turn, and call identities and monotonic run-local sequence before emitting corresponding lifecycle evidence. | Agent Core observability owner | Identity uniqueness, parent-link, and ordering tests. |
| P17-EVD-002 | Provider calls, retries, tool calls, compaction, and terminal outcomes **MUST** retain explicit run/turn/call correlation and kind. | Agent Core observability owner | In-memory call-graph reconstruction test. |
| P17-EVD-003 | A finalized manifest **MUST** bind the resolved execution, session branch, exact DirectRuntimeInput-or-ActiveSnapshot binding, effective User Policy, and finalized artifact references listed above; a direct run **MUST NOT** claim an Active Snapshot. | Agent runtime owner | Manifest schema, variant, and missing-field tests. |
| P17-EVD-004 | Requested, resolved, actual, provider-reported, estimated, quota, and billed facts **MUST** remain distinguishable; an unknown measurement **MUST NOT** be converted to zero. | Evidence contract owner | Serialization fixtures and unknown-value tests. |
| P17-EVD-005 | Sensitive data **MUST** be classified and redacted before it reaches `EvidenceSink`; a sink adapter **MUST NOT** be responsible for making raw input safe. | Runtime and Reference Product owners | Canary-secret scan at the mock sink boundary. |
| P17-EVD-006 | Evidence capture **MUST NOT** be enabled solely because an Eval consumer or adapter exists. | Reference Product owner | Default configuration and no-op Minimal Runtime tests. |
| P17-EVD-007 | Explicit capture setup failure **MUST** stop the run before its first provider or tool call. | Reference Product evidence owner | Setup-failure call-count test. |
| P17-EVD-008 | Emission or finalization failure **MUST** remain observable, **MUST** mark the evidence incomplete, and **MUST NOT** produce a finalized manifest. | Agent Core observability owner | Write/finalize failure-injection tests. |
| P17-EVD-009 | A policy that requires complete evidence **MUST** cause the current not-yet-launched and every later side effect to fail closed after evidence health becomes incomplete; in-flight effects retain their actual partial outcome. Provider model requests are not authorization-gated side effects under this criterion: fail-closed applies at the tool launch boundary, and the run preserves actual outcomes. | Reference Product policy owner | Evidence-health generation, reauthorization, and parallel in-flight tests. |
| P17-EVD-010 | Core evidence adapters **MUST** be limited to no-op and in-memory implementations; file capture and future exporters **MUST** remain outside `opi-agent`. | Workspace maintainers | Crate-content and dependency review. |
| P17-EVD-011 | No-op, in-memory, and Reference Product file adapters **MUST** satisfy one applicable lifecycle/failure conformance contract; in-memory and file recording tests **MUST** prove values were redacted before sink entry, while no-op proves capture remains disabled. | Evidence contract owner | Shared lifecycle conformance plus producer-boundary redaction tests. |

## Failure, cancellation, and partial-result semantics

Failures are typed at the narrowest owning boundary. Exact Rust variant and
wire code names remain source-owned, but callers can distinguish these semantic
classes:

| Boundary | Required distinguishable classes |
|---|---|
| Provider route | Unknown provider/model, ambiguous selection, unavailable or failed authentication preparation, disallowed fallback, unsupported wire/capability, request-route mismatch. |
| Next-turn transition | Hook failure, invalid candidate state, cancellation, state-application failure. |
| Tool authority | Unknown registration, invalid schema, unavailable capability, permission denial, unavailable authorizer. |
| Evidence | Setup failure, emission failure, finalization failure, incomplete evidence. |
| Execution | Provider/tool/retry/compaction error, cancellation, timeout, partial side effect, cleanup unknown. |

Failure precedence follows the runtime order. An unknown tool never reaches the
hook; a denied hook never reaches schema or authorization; invalid schema never
reaches the authorizer; a denied or unavailable authorizer never reaches the
tool; an invalid next-turn candidate never reaches stop or queues.

Cancellation before auth preparation records no auth or provider side effect.
Cancellation during credential-store IO, OAuth refresh, a provider stream, or
tool execution records the actual known terminal state. When cancellation or
timeout races with an external side effect, the result remains partial or
cleanup-unknown unless the owning adapter proves a stronger outcome.

| ID | Requirement | Owner | Mechanical verification |
|---|---|---|---|
| P17-FAL-001 | Each boundary listed above **MUST** expose distinguishable typed failure classes to its caller. | Boundary owner | Exhaustive error mapping and public-surface tests. |
| P17-FAL-002 | A failure **MUST** stop processing before every later boundary in the fixed order. | Agent runtime owner | Per-boundary downstream call-count tests. |
| P17-FAL-003 | Cancellation, timeout, queue closure/overflow, evidence failure, and partial side effects **MUST NOT** be converted into success or an unqualified denial. | Agent runtime owner | Race and failure-injection tests. |
| P17-FAL-004 | Error and evidence diagnostics **MUST NOT** expose credentials, raw environment values, or unclassified model/tool content. | Runtime and Reference Product owners | Secret-canary matrix across text, JSON/NDJSON, RPC, trace, and manifest surfaces. |

## Compatibility and migration

Phase 17 is an explicit 0.x breaking cleanup. It removes or replaces:

- `Agent` ownership of a single `Arc<dyn Provider>` and its `SharedProvider`
  wrapper;
- concrete provider ownership of live auth resolution after collection
  dispatch;
- Reference Product metadata-only active-provider construction and the
  same-provider model-switch restriction;
- `AgentLoopTurnUpdate { extra_messages }` append-only semantics;
- the unused `opi-agent::AgentHarness`/`HarnessRuntimeConfig` state owner that
  never drove the production loop;
- authorization-suggesting `Allow` naming on an ordinary pre-tool hook; and
- the storage-shaped Agent Core `TraceSink` interface superseded by the
  evidence lifecycle.

It preserves existing user data and product use cases:

- active-branch session reconstruction, resume, fork, and crash recovery;
- explicit Reference Product file capture;
- interactive, non-interactive, print, JSON/NDJSON, and RPC execution modes;
- existing provider login and credential-redaction flows; and
- the no-extension, no-capture Minimal Runtime.

Legacy session model entries are normalized without modifying their source
file. An entry that already identifies provider and model becomes a canonical
route. A bare model is normalized only if the current collection has exactly one
valid match, and the legacy source remains visible in evidence. Missing or
ambiguous routes return a typed error with deterministic remediation instead of
guessing from the active provider.

The current `TraceRecord` contract is serialize-only, and the Reference Product
has no legacy trace reader or user workflow that loads those records. Phase 17
therefore preserves legacy trace files as opaque, byte-identical artifacts and
does not invent a reader solely for compatibility. New evidence files receive a
distinct schema identity and never overwrite, rewrite, or silently upgrade
legacy files. A rollback preserves new files as evidence; it does not
down-convert them for an older binary.

| ID | Requirement | Owner | Mechanical verification |
|---|---|---|---|
| P17-MIG-001 | Existing supported session fixtures **MUST** remain readable and **MUST NOT** be rewritten merely by load, route normalization, resume, or fork. | Session owner | Byte-for-byte legacy fixture tests. |
| P17-MIG-002 | A legacy bare model **MUST** normalize only when one route is provable; ambiguity or absence **MUST** return typed remediation without provider dispatch. | Reference Product provider owner | Unique, ambiguous, and missing legacy fixture tests. |
| P17-MIG-003 | Existing explicit file-capture use cases **MUST** remain available through a Reference Product evidence adapter. | Reference Product evidence owner | CLI/RPC capture acceptance tests. |
| P17-MIG-004 | Legacy trace artifacts **MUST** remain byte-identical at their existing locations, while new schema output **MUST NOT** overwrite, rewrite, silently upgrade, down-convert, or delete them; Phase 17 **MUST NOT** add a reader without a separately registered user workflow. | Reference Product evidence owner | Old/new filesystem coexistence, byte immutability, and no-reader source-structure tests. |
| P17-MIG-005 | Interactive, non-interactive, print, JSON/NDJSON, and RPC modes **MUST** expose consistent route, authority, cancellation, and evidence-completeness semantics. | Reference Product owner | Cross-mode golden and subprocess tests. |
| P17-MIG-006 | Removed 0.x interfaces, including the unused AgentHarness runtime-state owner, **MUST NOT** be retained behind aliases, feature flags, or compatibility shims unless a separately approved consumer requirement is registered. | Workspace maintainers | Public API, production-call-site, and source-structure review. |

## Platform scope

Provider dispatch, next-turn state, authorization order, identity propagation,
redaction, and evidence finalization are platform-neutral Agent Core semantics.
Linux, macOS, and Windows use the same state and failure contracts. Phase 17
does not add an OS sandbox or claim new platform restriction guarantees.

Tests use `opi_ai::test_support::MockProvider`, mock tools and authorizers, local
fixtures, isolated temporary directories, and serialized environment mutation.
They do not call paid providers, require live credentials, or depend on network
availability.

| ID | Requirement | Owner | Mechanical verification |
|---|---|---|---|
| P17-PLT-001 | Core state, routing, authority, evidence, and failure semantics **MUST** be identical on Linux, macOS, and Windows. | Workspace maintainers | Three-platform CI acceptance matrix. |
| P17-PLT-002 | Phase 17 tests **MUST NOT** call paid/live providers or require credentials or network access. | Test owner | Mock-provider source review and hermetic CI. |
| P17-PLT-003 | Phase 17 **MUST NOT** claim that tool authorization provides an operating-system sandbox or stronger confinement than the selected execution adapter supplies. | Documentation and safety owners | Product diagnostics and documentation review. |

## Acceptance scenarios and verification

| ID | Scenario | Observable acceptance |
|---|---|---|
| P17-A01 | One session selects provider A, then provider B. | Collection-owned auth preparation and real requests reach A and B respectively; requested, resolved, actual, auth-source, and fallback evidence agrees. |
| P17-A02 | Provider/model is unknown, ambiguous, unauthenticated, refresh-failed, or wire-incompatible. | No model request is dispatched and the caller receives the owning typed route/auth failure without a silent credential or provider fallback. |
| P17-A03 | One provider call retries. | Every attempt retains the same route, parent call, and terminal retry evidence. |
| P17-A04 | Preparation changes context, model, thinking, maximum tokens, and temperature. | Stop observes the complete replacement; no observer sees a mixed state. |
| P17-A05 | Preparation validation fails or is cancelled. | All mutable fields retain their previous values and neither stop nor queues run. |
| P17-A06 | Model content requests a permission expansion. | Trusted policy is unchanged, authorization denies, and tool execution count is zero. |
| P17-A07 | Hook, extension, skill, retrieval/tool result, or child output forges a registration, capability, or grant. | Registry-owned origin and capability remain unchanged; forged values cannot enter trusted permission, policy, or scope fields. |
| P17-A08 | Permission scope expires or the authorizer fails. | The tool does not run; the denial and authority source are visible without secrets. |
| P17-A09 | A run contains provider calls, retry, tool calls, and compaction. | Run/turn/call/parent/sequence reconstruct one ordered graph and a complete manifest with the exact runtime-input-binding variant. |
| P17-A10 | Canary secrets occur in prompt, arguments, environment, and provider error. | The mock sink, product file adapter, diagnostics, and artifact metadata contain no raw canary. |
| P17-A11 | Evidence emission or finalization fails. | Actual execution outcome is retained, evidence is incomplete, and no finalized manifest exists. |
| P17-A12 | Complete evidence is policy-required after sink failure. | A stale allow is reauthorized; the current not-yet-launched and every later side effect fails closed, while already-running effects retain actual outcomes. |
| P17-A13 | Legacy sessions are loaded while opaque legacy trace files coexist with new evidence. | Session files and legacy trace files remain byte-identical; route normalization succeeds uniquely or returns deterministic remediation, and new evidence never overwrites the legacy trace path. |
| P17-A14 | Every Reference Product mode runs the same fixture. | Route, authority, cancellation, and evidence identities have equivalent semantics across modes. |
| P17-A15 | The suite runs on Linux, macOS, and Windows. | Platform-neutral acceptance passes in the repository CI matrix without a new OS-specific permission implementation. |

The implementation plan derives focused commands from the affected test
binaries. Phase exit additionally requires the repository gates below in this
order:

```sh
python scripts/opi-doc-check.py
cargo fmt --check --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo test --workspace --doc
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

If a test file is created or modified, its exact test binary or filter is run
before the workspace gates. The implementation handoff records test impact as
`add`, `update`, `delete`, `retain`, or `none`.

Task-local verification closes deterministic local behavior and the CI workflow
definition. The actual Linux/macOS/Windows run SHA, URLs, and results are
Phase-exit evidence gathered after a task commit is available to CI; they are
not a circular pre-commit condition for the final local acceptance task.

## Risk thresholds and rollback

Any of these observations blocks Phase exit:

- one cross-provider misroute, ambiguous route accepted, or silent provider,
  model, wire, credential-policy, or local fallback;
- one direct run represented as Promotion-Controller-selected Active Snapshot;
- one tool execution after missing, failed, expired, invalid, or denied
  authorization;
- one extension tool made visible or executable from project/package trust alone
  without an exact capability permission;
- one raw secret crossing into the evidence sink or unclassified artifact
  metadata;
- one incomplete execution represented as finalized evidence;
- one legacy session silently rewritten or guessed from the active provider;
- one unexplained semantic regression in a supported Reference Product mode;
- a new crate, generic runtime facade, product-policy dependency in
  `opi-agent`, exporter/storage API in Agent Core, or duplicate provider
  registry introduced to complete the design; or
- inability to pass the platform-neutral acceptance profile.

Rollback is the complete Phase 17 change set, not a selective fallback that
leaves two routing, state, authorization, or evidence paths active. It restores
the pre-Phase runtime while preserving user sessions, newly written evidence,
and diagnostics. New evidence is never deleted or down-converted as rollback
cleanup. User Policy and permission records are not automatically migrated by
this Phase, so rollback cannot broaden authority.

| ID | Requirement | Owner | Mechanical verification |
|---|---|---|---|
| P17-RBK-001 | Any threshold above **MUST** block Phase exit until the defect is removed or this specification is explicitly revised. | Phase lead | Exit audit against every threshold. |
| P17-RBK-002 | Rollback **MUST** restore one coherent pre-Phase runtime path and **MUST NOT** leave hidden dual dispatch, state, authority, or evidence paths. | Release and module owners | Revert review and pre-Phase regression profile. |
| P17-RBK-003 | Rollback **MUST NOT** delete, rewrite, or down-convert user sessions or evidence artifacts created by the Phase 17 runtime. | Session and evidence owners | Rollback fixture and filesystem immutability tests. |
| P17-RBK-004 | Rollback **MUST NOT** widen User Policy, Capability Permission, or Permission Scope. | Reference Product policy owner | Before/after policy snapshot test. |

## Delivery dependency order and implementation-plan handoff

This specification defines workstream dependencies, not ledger tasks or task
status:

```text
provider route/auth substrate
    └── Agent + atomic next-turn production cutover
            └── Reference Product provider assembly

evidence identity/lifecycle contract
    └── trusted registration + ToolAuthorizer cutover

provider assembly + next-turn + authorization + evidence contract
    └── core evidence runtime integration and TraceSink contraction
            ├── Reference Product file adapter/finalization/redaction
            └── legacy session/route migration + trace preservation
                    └── local cross-mode/failure/rollback acceptance

Phase exit
    └── repository gates + actual three-platform CI evidence
```

The provider route/auth substrate and evidence identity/lifecycle contract may
begin independently. Atomic next-turn production cutover consumes provider
routing. Authorization consumes stable evidence identity/health vocabulary but
does not block provider assembly. Core evidence integration consumes the final
provider, next-turn, and authority identities. Product file capture and legacy
session/route migration plus trace preservation are separate vertical slices:
neither may be hidden inside one monolithic evidence task. Local phase
acceptance exercises the assembled
production path; actual platform CI and the six repository gates remain
Phase-exit evidence.

The `opi-implement` plan handoff derives a finite task graph and registers:

- every `P17-*` requirement and `P17-A*` scenario;
- exact source, test, fixture, documentation, and schema impact;
- task-local definitions of done and success criteria;
- the narrowest focused test command for each change;
- honest substrate records with no fabricated production call site;
- typed failures assigned to the task that owns the boundary rather than one
  late integration task;
- migration and rollback ordering; and
- the full repository exit gates.

The planning workflow does not weaken an acceptance scenario to fit the current
implementation. A discovered need for Eval, a new crate, a product-policy
dependency in Agent Core, a broad runtime facade, or a second live compatibility
path is a route question governed by `P17-AUTH-003`, not an implementation task.
