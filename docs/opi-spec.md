# Opi Technical Direction and Architecture Specification

This document defines Opi's durable technical direction after the completion of
its current implementation baseline. It is the normative parent for future
delivery specifications. It is not a progress log, release inventory, or API
catalogue.

## 1. Document Authority and Reading Model

### 1.1 Scope

This specification owns five kinds of decisions:

- the product mission and non-goals;
- durable architecture and dependency invariants;
- the long-term capability ladder;
- admission, evidence, authority, and rollback gates; and
- current strategic priority among goals whose prerequisites are satisfied.

Exact CLI flags, provider catalogues, wire constants, file layouts, release
versions, and implementation status remain in their authoritative sources and
are indexed in [Chapter 11](#11-authoritative-contracts-and-evidence-index).

### 1.2 Normative language

`MUST` and `MUST NOT` identify requirements that every conforming delivery must
satisfy. `SHOULD` identifies the default architecture; deviation requires an
ADR with equivalent evidence. `MAY` identifies an allowed option. `Evidence`
labels non-normative observations from source, alignment, research, or
experiments.

| ID | Requirement | Owner | Verification |
|---|---|---|---|
| AUTH-001 | This specification **MUST** contain only durable direction, invariants, gates, and strategic priority. | Specification maintainers | Documentation contract checks and review against Chapter 1. |
| AUTH-002 | Every normative `MUST` **MUST** identify an owner and a mechanical verification route. | Clause author | Table/schema review and Phase admission review. |
| AUTH-003 | Authority **MUST** flow from this specification to a Phase delivery specification, then to the implementation ledger and historical snapshot; lower layers **MUST NOT** override higher layers. | Phase shaper | Phase source mapping and ledger validation. |
| AUTH-004 | `docs/opi-spec.md` **MUST** be the normative English source; `docs/opi-spec.zh.md` **MUST** preserve the same chapter and clause identifiers with equivalent meaning. | Documentation maintainers | Bilingual heading, identifier, and semantic review. |
| AUTH-005 | Progress, completion state, dates, task lists, and decision history **MUST NOT** be recorded here. | Specification maintainers | Prohibited-content scan. |

### 1.3 Supporting artifacts

- An ADR explains a hard-to-reverse trade-off; it does not create roadmap
  authority by itself.
- `docs/realign/` and `.repo/pi-0.84.1` provide inward alignment evidence.
- `docs/research/` provides outward evidence and design candidates.
- `docs/snapshots/` and the implementation ledger preserve completed delivery
  history.
- Git history and `CHANGELOG.md` preserve document and release history.

A finding becomes normative only through an explicit revision to this document
or an admitted Phase delivery specification.

## 2. Mission, Goals, and Non-Goals

### 2.1 Mission

Opi is a Rust AI Agent toolkit with a terminal-first Reference Product. It
implements and extends the design doctrine demonstrated by pi: keep the Agent
Core small and deep, put product opinion in the harness, and grow optional
capability through extensions and independently reusable products.

Opi is not a line-by-line Rust port. It preserves valuable semantics while
using Rust ownership, enums, traits, explicit error models, bounded concurrency,
portable binaries, and compile-time checks where they produce a safer or deeper
module.

| ID | Requirement | Owner | Verification |
|---|---|---|---|
| GOAL-001 | Opi **MUST** provide a product-neutral Agent Core and a coherent terminal Reference Product. | Opi maintainers | Crate dependency graph, interface tests, and product acceptance. |
| GOAL-002 | Optional workflows and independently reusable capabilities **MUST** remain outside the Agent Core unless they pass every gate in Chapter 8. | Capability owner | Placement Review. |
| GOAL-003 | Rust-specific design **SHOULD** improve correctness, explicit state, testability, portability, or delivery rather than imitate TypeScript/npm structure. | Module owner | Design review and conformance evidence. |
| GOAL-004 | Opi **MUST** remain useful without Eval, Learning, remote hosting, or any extension package. | Reference Product owner | Minimal Runtime acceptance profile. |

### 2.2 Success criteria

The project succeeds when:

- callers obtain substantial behavior through small, stable interfaces;
- provider, Agent, tool, session, and extension semantics are independently
  testable;
- the Reference Product is useful without making its opinions mandatory for
  embedders;
- Agent-neutral capabilities can mature as Independent Companions; and
- later learning and self-iteration claims are supported by reproducible,
  independent evidence and revocable human authority.

### 2.3 Non-goals

Opi does not aim to:

- match every pi package, TypeScript type, npm boundary, session file, or
  provider catalogue;
- make MCP, sub-Agent, plan, task-list, permission-popup, or remote-session
  workflows mandatory Agent Core features;
- expose, persist, or evaluate a model's private raw Chain-of-Thought;
- treat an LLM judge, model confidence, or a single benchmark as independent
  proof of improvement;
- allow a running Agent to broaden its own User Policy or approve its own
  change; or
- perform default online model-weight modification.

## 3. First Design Doctrine

### 3.1 Minimal and deep

The Agent Core owns only semantics that remain valid across harnesses, user
interfaces, and products. A good module hides substantial behavior behind a
small interface. A seam is justified by real variation, not by anticipated
possibility.

| ID | Requirement | Owner | Verification |
|---|---|---|---|
| PRIN-001 | Agent Core additions **MUST** pass the deletion test: removing the module would reintroduce its complexity across multiple core callers. | Agent Core maintainers | Placement case and dependency review. |
| PRIN-002 | A new public seam **MUST** be intrinsic to the Agent state machine or be demonstrated by at least two real adapters or consumers with shared conformance tests. | Interface owner | Conformance inventory and tests. |
| PRIN-003 | Mechanism **MUST** remain below policy; Reference Product or extension policy **MUST NOT** flow into Agent Core dependencies. | Workspace maintainers | Cargo dependency graph and architecture checks. |
| PRIN-004 | Selection at permission, protocol, adapter, evidence, and promotion boundaries **MUST** fail closed. | Boundary owner | Negative-path and degradation tests. |
| PRIN-005 | A claim of correctness or improvement **MUST** follow immutable, reproducible evidence. | Claim owner | Artifact provenance and verification record. |

### 3.2 Alignment with pi

pi is an active design reference, not Opi's release manager. Alignment work
classifies each signal as:

1. a durable Agent Core semantic to preserve;
2. a useful but unstable experiment to observe;
3. a product or ecosystem capability to keep outside Agent Core; or
4. an implementation accident that Rust should not copy.

New pi packages do not establish priority. Source evidence can trigger a
Placement Review or route revision, but human shaping decides whether it changes
this specification.

### 3.3 Rust-native choices

Rust designs should prefer:

- enums for closed protocol and state-machine alternatives;
- explicit ownership for cancellation, session binding, and mutable authority;
- traits at proven seams and generics where concrete types are known;
- typed errors and fail-closed decoding at public boundaries;
- bounded buffers and explicit backpressure for streaming paths; and
- standalone binaries or process protocols when cross-language reuse is more
  valuable than in-process coupling.

Rust is not a reason to split a shallow crate, create a trait with one
hypothetical adapter, or move ecosystem policy into a library.

## 4. System Placement and Dependency Direction

### 4.1 Placement vocabulary

The canonical definitions of **Agent Core**, **Reference Product**, **Extension
Ecosystem**, **Independent Companion**, **Standalone Project**, and **Placement
Review** live in [CONTEXT.md](CONTEXT.md). This chapter applies those terms; it
does not redefine them.

| Layer | Current ownership | May depend on | Must not require |
|---|---|---|---|
| Agent Core | `opi-ai`, `opi-agent` | Product-neutral libraries and proven adapters | TUI, coding workflow, Eval, Learning, Promotion, optional extensions |
| Reference Product | `opi-tui`, `opi-coding-agent` and the `opi` binary | Agent Core and extension interfaces | Adoption of every optional workflow |
| Extension Ecosystem | Skills, themes, prompt fragments, workflow packages, storage and remote-session adapters | Published Opi extension/package interfaces | Changes to Agent Core semantics |
| Independent Companion | `opi-sandbox`, its minimal protocol, and the planned cross-Agent Eval product | Agent-neutral contracts | Opi product crates or Minimal Runtime activation |

This table records present ownership, not permanent crate topology.

| ID | Requirement | Owner | Verification |
|---|---|---|---|
| PLACE-001 | Dependencies **MUST** point inward toward the smallest stable interface; Agent Core **MUST NOT** depend on Reference Product, Eval, Learning, or Promotion modules. | Workspace maintainers | Cargo metadata and architecture review. |
| PLACE-002 | A capability that solves an Agent-neutral problem **MUST** begin outside Opi product crates and expose an Agent-neutral contract. | Capability owner | Placement case and standalone build. |
| PLACE-003 | Workspace location, repository location, brand, and organization **MUST** be decided independently. | Product maintainers | Placement Review and release metadata. |
| PLACE-004 | An experimental capability **MUST NOT** enter Agent Core through a feature flag or unstable label. | Agent Core maintainers | Public-interface and dependency scan. |
| PLACE-005 | Existing code **MAY** remain in place until a Placement Review proves material dependency, authority, release, or maintenance harm. | Module owner | Recorded placement case. |

### 4.2 Reference Product ownership

The Reference Product may own CLI/TUI interaction, configuration assembly,
credential interaction, default coding tools, session navigation, diagnostics,
and package activation. Those choices must not redefine the Agent Core state
machine or become mandatory for embedders.

### 4.3 Ecosystem and independence

Opi-specific optional workflows belong in the Extension Ecosystem. A capability
belongs in an Independent Companion when it has complete value without Opi,
owns its artifacts and error model, can be integrated through a public
contract, and leaves the Minimal Runtime unchanged when absent or failed.

Repository extraction requires independent build/test/release operation, at
least two real consumers with one outside Opi, a proven and versioned seam,
divergent lifecycle needs, complete operational ownership, positive net value,
and a reversible migration. Opi-brand independence additionally requires a
mission, user base, governance, and identity that remain complete without Opi.

## 5. Long-Term Capability Ladder

The ladder expresses capability dependency, not delivery status. Strategic
priority in Chapter 9 selects among goals whose prerequisites are already met.

```text
Model Runtime
    ↓
Reasoning and Context Management
    ↓
Agent Execution
    ↓
Continual Learning
    ↓
Controlled Self-Iteration

Eval / Observability governs Agent Execution and every later rung.
```

| ID | Capability | Durable outcome | Entry evidence for the next rung |
|---|---|---|---|
| CAP-001 | Model Runtime | Provider-neutral messages and streaming, real runtime provider/wire dispatch, capability negotiation, usage accounting, and bounded failure. | Provider conformance and multi-provider routing evidence. |
| CAP-002 | Reasoning and Context Management | Observable context construction, thinking budget, planning, reflection, retry, compaction, model switching, and tool decisions without requiring private raw reasoning. | Reproducible context/compaction behavior and measured task outcomes. |
| CAP-003 | Agent Execution | Deterministic turns, tools, queues, steering, sessions, cancellation, failure semantics, and finalized artifacts. | Cross-Agent Eval can observe and reproduce outcome and efficiency. |
| CAP-004 | Continual Learning | C1 episodic-memory and reusable-skill candidates derived from finalized experience, independently evaluated and reversibly activated. | Multiple frozen seasons demonstrate gain, retention, safety, efficiency, and withdrawal. |
| CAP-005 | Controlled Self-Iteration | A C2 behavior candidate completes independent evaluation, staged activation, monitoring, and rollback within a revocable Delegated Promotion Policy. | Human Authority confirms the loop remains bounded and cannot widen itself. |

| ID | Requirement | Owner | Verification |
|---|---|---|---|
| CAP-006 | A later rung **MUST NOT** be admitted by bypassing evidence required by an earlier rung. | Route maintainers | Phase admission matrix. |
| CAP-007 | External Knowledge Sync **MUST** remain a parallel product route rather than being counted as Continual Learning. | Knowledge product owner | Product interface and authority review. |

## 6. Cross-Cutting Control Planes

### 6.1 Evidence and observability

Agent Core observability supplies explicit context propagation and one stable
domain vocabulary across provider, turn, tool, compaction, retry, and session
activity. Exporters, hosted storage, dashboards, and evaluation policy remain
outside Agent Core.

Evidence artifacts are immutable and content-addressed. Missing measurements
remain `unknown`; they are never silently converted to zero. Sensitive prompts,
tool arguments, results, and environment data require explicit capture and
redaction policy.

| ID | Requirement | Owner | Verification |
|---|---|---|---|
| CTRL-001 | Agent Core **MUST** propagate stable run/turn/call correlation and finalized artifact references without binding an exporter. | Agent Core observability owner | No-op/in-memory adapter conformance and trace tests. |
| CTRL-002 | Evidence **MUST** retain source, permission, time, environment, model, prompt, tool, budget, and artifact provenance sufficient for offline verification. | Evidence Producer | Manifest validation and offline recomputation. |
| CTRL-003 | Sensitive evidence **MUST** be classified and redacted before export; capture **MUST NOT** be enabled merely because an Eval consumer exists. | Runtime owner | Secret scan, redaction tests, and User Policy review. |

### 6.2 Independent cross-Agent evaluation

Evaluation is an Independent Companion, provisionally described as one Rust
library plus CLI with no dependency on Opi crates. Its deep interface accepts a
resolved experiment lock and adapters, produces an immutable run bundle, and
recomputes reports from saved artifacts. Final naming, repository, and brand are
left to Placement Review.

Agent and benchmark differences enter through two process-capable seams:

- an `AgentAdapter` runs or imports an Agent trajectory; and
- a `GraderAdapter` invokes the benchmark's native grader.

The canonical evidence bundle contains:

- a validated [ATIF](https://github.com/harbor-framework/harbor/blob/main/rfcs/0001-trajectory-format.md)
  trajectory;
- a supplemental span graph for run, turn, LLM, tool, compaction, retry, and
  grader call chains;
- grader output with name, version, digest, native metrics, and provenance; and
- a content-addressed artifact manifest covering inputs, logs, final workspace
  result, trajectory, calls, and grader output.

Reports are outcome-first. They present native success, wall and critical-path
time, time to first token, LLM/tool/compaction/retry counts and latency,
input/output/cache/reasoning tokens, known cost and coverage, compression, and
failed-call consumption as separate dimensions. Quality, cost, safety, and
authority are not collapsed into one score.

| ID | Requirement | Owner | Verification |
|---|---|---|---|
| CTRL-004 | Headline benchmark results **MUST** come from the benchmark's native grader; an LLM judge **MAY** provide a separately labelled diagnostic only. | Evidence Producer | Grader provenance and report-schema validation. |
| CTRL-005 | Baseline and candidate runs **MUST** be paired by task and trial under one frozen manifest; missing pairs and telemetry **MUST** remain visible. | Evaluation orchestrator | Pairing and coverage checks. |
| CTRL-006 | A report **MUST** be reproducible offline from immutable run bundles; a new real execution **MUST** receive a new trial identity. | Evaluation product owner | Deterministic recompute/regrade/render conformance. |
| CTRL-007 | Benchmark datasets, native graders, container images, sandboxes, remote schedulers, exporters, leaderboards, and learning policy **MUST** remain outside the evaluation module. | Evaluation product owner | Dependency and package-content review. |

### 6.3 Learning and change authority

The canonical definitions of Evidence Producer, Candidate Producer, Promotion
Controller, Human Authority, Activation Class, Active Snapshot, Control
Baseline, Promotion Lifecycle, Delegated Promotion Policy, and Controlled
Self-Iteration live in [CONTEXT.md](CONTEXT.md).

```text
Agent Core evidence seams
    ↓ AgentAdapter
Evidence Producer → immutable RunBundle
    ↓
Candidate Producer → immutable CandidateBundle
    ↓
Promotion Controller + Human Policy
    ↓ Runtime Adapter
shadow → opt-in canary → active / rollback
```

The Evidence Producer does not propose or activate changes. The Candidate
Producer does not select its grader, thresholds, or approval. The Promotion
Controller does not rewrite evidence, candidates, or the policy that authorizes
it. The runtime consumes an immutable Active Snapshot and cannot write a new one
for itself.

External Knowledge Sync has a different truth source: an authorized Source of
Record. A continuous, verified, scope-preserving upstream revision may be
activated automatically; adding a source, widening scope, or changing authority
requires Human Authority.

| ID | Requirement | Owner | Verification |
|---|---|---|---|
| CTRL-008 | Evidence generation, candidate generation, promotion, and human authorization **MUST** remain separate authorities. | Runtime and learning product owners | Interface/dependency review and negative authorization tests. |
| CTRL-009 | Modules **MUST** exchange immutable bundles and snapshot references; a shared mutable database **MUST NOT** grant cross-authority write access. | Integration owner | Storage ownership and mutation audit. |
| CTRL-010 | Eval, Learning, or Promotion failure **MUST** stop forward transition without changing Agent Core behavior; if active safety cannot be proven, affected new work **MUST** stop. | Promotion Controller owner | Failure-injection and recovery tests. |

### 6.4 Security, privacy, and reversibility

User Policy sets hard limits. A model may request permission but cannot grant
it. Derived evidence or knowledge never receives a wider scope than its narrowest
source. Withdrawal and deletion propagate to all derived views. Existing work
remains bound to the snapshot it started with; new work reads the current active
pointer.

Automatic rollback is always permitted inside the pre-authorized recovery path.
Automatic reactivation is never permitted: a rejected or rolled-back candidate
requires a new identity and a new authorization decision.

## 7. Durable Architecture Invariants

### 7.1 Provider runtime

Model selection is runtime routing, not metadata lookup. The provider collection
owns available models, credentials, and routing to the correct provider and wire
implementation.

| ID | Requirement | Owner | Verification |
|---|---|---|---|
| INV-001 | Selecting a model **MUST** resolve and retain the corresponding provider/wire implementation at runtime; cross-provider selection **MUST NOT** be rejected solely because startup chose another provider. | `opi-ai` | Provider collection conformance and cross-provider routing tests. |
| INV-002 | Provider-specific wire code **MUST** remain behind provider-neutral request, stream, usage, and capability interfaces. | `opi-ai` | Provider wire-format fixtures and interface tests. |

### 7.2 Agent turn transition

Next-turn preparation is a state transition, not a message-append hook. It can
atomically replace context, model, and reasoning settings. Stop and queue
decisions consume the resulting state, and their ordering is part of the public
Agent semantics.

| ID | Requirement | Owner | Verification |
|---|---|---|---|
| INV-003 | The Agent loop **MUST** define and test the exact order of turn completion, next-turn preparation, stop evaluation, and steering/follow-up polling. | `opi-agent` | State-transition unit and integration tests. |
| INV-004 | A next-turn update **MUST** be able to replace the full next request state atomically rather than only append messages. | `opi-agent` | Hook interface and state replacement tests. |

### 7.3 Tools, cancellation, and backpressure

Tool schemas are validated before execution. Hooks run in defined order.
Parallel-safe calls may run concurrently; sequential calls preserve order.
Cancellation propagates across provider streams and tool batches. Bounded
queues make backpressure and overflow visible.

| ID | Requirement | Owner | Verification |
|---|---|---|---|
| INV-005 | Authority and schema validation **MUST** complete before a tool causes side effects. | Agent runtime owner | Negative permission/schema tests. |
| INV-006 | Cancellation, queue closure, overflow, and partial tool failure **MUST** be observable and **MUST NOT** be converted into silent success. | Agent runtime owner | Failure-injection and bounded-queue tests. |

### 7.4 Sessions and artifacts

Session branching, reconstruction, append durability, finalized evidence, and
snapshot binding are semantics. JSONL, SQLite, search indexes, and cloud stores
are adapters. A repository seam should expand only after a second real adapter
and shared conformance demonstrate the necessary variation.

| ID | Requirement | Owner | Verification |
|---|---|---|---|
| INV-007 | Session persistence **MUST** preserve active-branch reconstruction, parent links, leaf selection, and crash recovery. | `opi-agent` | Repository conformance and corruption recovery tests. |
| INV-008 | Finalized run evidence **MUST** identify the session branch and Active Snapshot that produced it. | Agent runtime owner | Artifact-schema and resume/fork tests. |

### 7.5 Extensions and command execution

The standard distribution has a Minimal Runtime with a direct local execution
path. Optional external execution follows five independent lifecycle gates:
**Installed**, **Trusted**, **Enabled**, **Selected**, and **Permitted**. Package
Trust authorizes package code; Capability Permission authorizes an invocation.
Neither implies the other.

Once an external execution adapter is Selected, failure is **fail-closed** and
never falls back to local execution. Extension packages and adapters are trusted
code with the launching user's operating-system permissions; permission
declarations are metadata, not an enforced sandbox.

Native restriction is owned by the Independent Companion `opi-sandbox`, which
depends only on the minimal `opi-protocol` command-execution contract and is not
linked into the `opi` binary. Its public standalone surface includes
`SandboxPolicy`, `SandboxRequest`, `SandboxRunner`, `SandboxEvent`,
`SandboxResult`, and `opi-sandbox backend --stdio`.

The former core `[sandbox]`, `--sandbox`, and `--sandbox-require` surfaces are
removed and have no aliases. Opi does not claim that extension packages are
sandboxed, that file/navigation tools are confined, or that package permission
metadata enforces operating-system policy.

Docker, VM, SSH, remote execution adapters, AppContainer, universal tool
shadowing, and Windows native restriction beyond L0 are outside this invariant.
Official `opi-sandbox` artifacts target Linux and macOS. Windows provides L0
process supervision through a Job Object and makes no stronger confinement
claim.

| ID | Requirement | Owner | Verification |
|---|---|---|---|
| INV-009 | Installed, Trusted, Enabled, Selected, and Permitted **MUST** remain independently observable and enforceable lifecycle states. | Reference Product owner | Package/execution routing tests and diagnostics. |
| INV-010 | A selected external execution backend failure **MUST NOT** fall back to local execution. | Capability Router owner | Adapter failure tests. |
| INV-011 | `opi-sandbox` **MUST** remain reusable without linking Opi product crates; platform degradation **MUST** be explicit. | `opi-sandbox` owner | Standalone build, protocol conformance, and platform acceptance. |

### 7.6 Reference Product

The Reference Product may choose terminal interaction, default coding tools,
configuration layers, credentials, session commands, package management, and
diagnostics. Those choices use Agent Core interfaces and may be replaced by an
embedder.

Exact current surfaces are documented by generated help, README files, crate
documentation, and source. They are not duplicated here.

## 8. Capability Admission and Promotion Gates

### 8.1 Agent Core admission

A capability enters Agent Core only if every gate passes.

| Gate | Required evidence |
|---|---|
| Semantic necessity | Removing it breaks a durable model or Agent state-machine invariant. |
| Product neutrality | It has no terminal workflow, benchmark, organization-knowledge, operating-system, or user-policy opinion. |
| Depth and locality | A small interface hides material complexity that would otherwise return to multiple core callers. |
| Real seam | It is intrinsic state-machine semantics or has two real adapters/consumers with shared conformance. |
| Coupled lifecycle | Independent versioning would create a more complex and fragile contract. |
| Mechanical verification | Invariants, ordering, error modes, and safety behavior have automated evidence. |
| Minimal authority | The addition does not expand default permission, I/O, dependency, or supply-chain surface for optional behavior. |

| ID | Requirement | Owner | Verification |
|---|---|---|---|
| GATE-001 | Failure of any Agent Core admission gate **MUST** keep the capability in the Reference Product, Extension Ecosystem, or an Independent Companion. | Placement Review participants | Signed placement case. |
| GATE-002 | Feature flags, unstable labels, or pi parity **MUST NOT** waive an admission gate. | Agent Core maintainers | Public-interface review. |

### 8.2 Product and independence admission

A Reference Product addition must be necessary to a coherent terminal coding
Agent, carry explicit product opinion, keep dependencies one-way, leave Agent
Core semantics replaceable, lack stronger cross-Agent reuse value, and preserve
User Policy and Minimal Runtime defaults.

Extension Ecosystem versus Independent Companion is decided by problem
ownership: Opi-specific lifecycle and contribution semantics belong to the
ecosystem; an Agent-neutral problem with independent artifacts, errors,
conformance, and value belongs to a companion.

A Placement Review is triggered by a second real consumer/adapter, a new public
interface or cross-layer dependency, increased permission/I/O/platform/supply
chain, divergent release or security lifecycle, a changed deletion-test result,
or new alignment/usage evidence. The placement case records ownership,
dependencies, consumers, interface, conformance, authority, failure behavior,
lifecycle, deletion test, and migration. Cross-layer movement requires an ADR.

### 8.3 Activation Classes

| Class | Candidate | Maximum automated authority |
|---|---|---|
| C0 Evidence-only | Trajectory, run bundle, score, report, diagnostic summary | Generate, store, and recompute; never affect active runtime. |
| C1 Scoped knowledge | Finalized episode, sourced summary, retrieval index, scoped memory or skill view | Generate, shadow, canary, and—inside an explicit delegated scope—activate or roll back. |
| C2 Behavior candidate | Skill behavior, prompt, non-authority configuration, model routing, reasoning/compaction/retry strategy, tool orchestration | Propose, evaluate, shadow, and canary; activation requires per-candidate human approval unless narrowly delegated. |
| C3 Authority/executable | User Policy, permission scope, safety/privacy threshold, network or mutating capability, code, dependency, executable adapter, model weights | Propose, test, tighten, revoke, or roll back; expansion or activation always requires human approval and normal release. |

Automatic authority decreases as impact increases. Passing Eval never grants
permission or release authority.

### 8.4 Baselines and anti-self-confirmation

Every candidate freezes:

- a Control Baseline: the pre-candidate Active Snapshot under an identical
  model, tools, prompt, configuration, policy, budget, data, grader, seed, and
  resource manifest;
- a no-learning ablation for memory and skill candidates;
- the immediately previous rollback artifact; and
- target, retention, safety, and efficiency history across frozen seasons.

If the active snapshot or a material control changes, an undecided candidate
expires or is regenerated and reevaluated.

Candidate-generating episodes and their source-family derivatives do not enter
that candidate's target, retention, or safety cohorts. The Candidate Producer
does not choose cohorts, graders, headline metrics, thresholds, or epsilon.
Current unfinished work does not consume its own derivative. Shadow/canary data
may inform only a later candidate. Failures, conflicts, and safety events remain
in the evidence set.

### 8.5 Six independent promotion gates

| Gate | Admission rule |
|---|---|
| Evidence | Resolved manifest, digests, adapter conformance, source deduplication, privacy scan, holdout isolation, and offline recomputation all pass. |
| Target Gain | On pre-registered headline outcome and paired task/trial results, the paired 95% confidence-interval lower bound is greater than zero. |
| Retention | On prior and holdout tasks, the paired 95% confidence-interval lower bound is at least `-epsilon`; critical correctness and compatibility use `epsilon = 0`. |
| Safety and Authority | High-severity policy, privacy, secret, injection, or authority regression count is zero; derived scope is no wider than the narrowest source. |
| Efficiency | After outcome gates pass, token, known cost, wall time, tool calls, retries, and compaction each remain inside a pre-registered budget; missing data blocks the corresponding claim. |
| Reversibility | Atomic rollback to the previous snapshot is rehearsed; existing work keeps its bound snapshot and new work uses the rollback target. |

The gates do not compensate for one another. Modifying a failed candidate
creates a new identity and requires new evidence and authorization.

### 8.6 Promotion Lifecycle and delegation

```text
offline candidate → shadow → opt-in canary → active
                         ↘ rejected
active ─────────────────→ rolled back
```

Shadow output does not affect tools, user-visible output, or persistent state.
Canary effect is limited to pre-registered users, sessions, tasks, permission
scope, budget, sample size, observation window, and stop conditions. Active
means only the default inside an already authorized scope; it does not widen
authority or imply permanent approval.

Provenance or policy failure, one high-severity event, a rolling retention or
efficiency breach, required telemetry loss, or Human Authority revocation
triggers automatic rollback. Failed candidates and evidence are retained.
Automatic reactivation is prohibited.

A Delegated Promotion Policy is a time-bounded C3 grant created by Human
Authority. It fixes candidate classes/types, cohorts, model/tool/provider/
grader/data versions, metrics, thresholds, budgets, canary limits, lifetime,
maximum promotion count, rollback target, and objects that still require human
approval. Candidate and Promotion modules cannot create, modify, renew, or
expand it. Material environment, authority, safety, provenance, or unexplained
regression changes invalidate it.

## 9. Current Strategic Priorities

Priority is an ordering of unmet goals, not a delivery schedule or progress
statement.

### STRAT-001 — Close deep Agent Core semantic gaps

Give runtime provider dispatch real ownership; make next-turn state replacement
atomic and correctly ordered; establish the minimum product-neutral evidence
and observability seam. Adding more catalogue entries is lower priority than
making the existing abstraction true at runtime.

### STRAT-002 — Establish independent cross-Agent Eval

Deliver Agent/Grader adapters, native-grader provenance, ATIF trajectory plus a
call graph, content-addressed run bundles, paired outcome-first reporting, and
offline recomputation. The product must evaluate Opi, pi, and other Agents
without linking their runtimes.

Shape the initial Eval delivery as a dedicated Phase, separate from Continual
Learning and Promotion. Only the minimum Agent Core evidence seam may be an
explicit prerequisite.

### STRAT-003 — Deepen measurable Agent capability

Improve reasoning/context construction, compaction, model/tool decisions,
reliability, and session behavior only against frozen evaluation evidence.
Experimental pi protocol/client/server and broad harness surfaces remain
observation signals until real Opi consumers prove their seams.

### STRAT-004 — Prototype C1 Continual Learning

Validate episodic memory before reusable skills, shadow before activation, and
retention/privacy/withdrawal before scale. The current knowledge/learning
research document remains evidence, not an implementation specification.

### STRAT-005 — Introduce C2 behavior candidates

Evaluate prompt, non-authority configuration, model-routing, reasoning, and tool
orchestration candidates with per-candidate Human Authority before any
delegation.

### STRAT-006 — Qualify Controlled Self-Iteration

Only after multiple independent frozen seasons may a revocable Delegated
Promotion Policy permit a C2 candidate to complete evaluation, staged
activation, monitoring, and rollback without per-candidate intervention.

### Parallel routes

- External Knowledge Sync may mature independently from Continual Learning.
- `opi-sandbox` may mature as an Independent Companion and later undergo
  Placement Review for repository or brand independence.
- Reference Product and Extension Ecosystem work may remove demonstrated user
  friction when it does not expand Agent Core.
- Model-weight training remains a separately governed, long-term product route.

## 10. Phase Derivation and Verification

A Phase is a finite delivery unit derived from this specification. It is not a
rung, a route revision, or a place to redefine parent requirements.

| ID | Requirement | Owner | Verification |
|---|---|---|---|
| PHASE-001 | A Phase delivery specification **MUST** cite stable clause identifiers and one strategic goal from this document. | Phase shaper | Admission lint and human review. |
| PHASE-002 | It **MUST** state the outcome, non-goals, architecture placement, priority reason, acceptance evidence, risk thresholds, rollback, and platform scope. | Phase shaper | Admission checklist. |
| PHASE-003 | A new capability or cross-layer interface **MUST** include a placement case. | Capability owner | Placement Review. |
| PHASE-004 | Research, realign reports, and ADRs **MUST** be identified as evidence rather than parent authority. | Phase shaper | Source classification review. |
| PHASE-005 | Completion **MUST** update only the implementation ledger and historical snapshot; it **MUST NOT** write progress into this document. | Implementation workflow owner | Ledger/snapshot diff review. |
| PHASE-006 | If implementation exposes a route error, delivery **MUST** stop for an explicit revision to this specification; the Phase **MUST NOT** lower or bypass a parent gate. | Phase lead | Blocked handoff and route-revision review. |

The highest-priority admissible unmet goal is shaped next. A lower goal can move
first only when an explicit dependency, risk-reduction, or evidence-enabling
argument is recorded in its Phase delivery specification.

## 11. Authoritative Contracts and Evidence Index

This chapter points to owners of volatile facts. It does not copy their
inventories.

| Subject | Authority | Role in this specification |
|---|---|---|
| Domain language | [CONTEXT.md](CONTEXT.md) | Canonical product, authority, execution, and safety terms. |
| Current product surfaces | [README.md](../README.md), generated `opi --help`, crate documentation, and source | Current CLI, modes, providers, tools, configuration, and platform behavior. |
| Workspace topology and release state | [`Cargo.toml`](../Cargo.toml), crate manifests, and [CHANGELOG.md](../CHANGELOG.md) | Current version, dependencies, and release history. |
| Wire and schema contracts | Source constants, schemas, fixtures, and `opi-protocol` documentation | Exact current versions and payloads. |
| Completed delivery history | [`docs/snapshots/`](snapshots/) and the implementation ledger | Historical completion and acceptance evidence. |
| pi alignment | [`.repo/pi-0.84.1`](../.repo/pi-0.84.1) primary source and [`docs/realign/`](realign/) indexes | Non-normative inward evidence. |
| External capability research | [`docs/research/`](research/) | Non-normative outward evidence. |
| Independent Eval direction | [Agent benchmark plan](research/2026-07-10-opi-agent-benchmark-plan.zh.md) and official benchmark/trajectory references | Evidence for Chapter 6 and strategic priority. |
| Continual Learning direction | `docs/research/opi-knowledge-sdk-learning-worker-spec.zh.md` and cited primary research | Directional evidence only. |
| Hard-to-reverse trade-offs | Registered ADRs | Rationale for accepted deviations and placement movement. |
| Documentation contracts | [`scripts/opi-doc-check.py`](../scripts/opi-doc-check.py) | Fast structural, synchronization, link, and stable safety checks. |

For benchmark evidence, native grader and harness authorities include
[Harbor/Terminal-Bench](https://www.harborframework.com/docs/tasks),
[SWE-bench](https://www.swebench.com/SWE-bench/reference/harness/), and
[AgentDojo](https://github.com/ethz-spylab/agentdojo). OpenTelemetry GenAI
semantic conventions may inform an import/export adapter, but they do not own
Opi's disk evidence schema.

This index closes the authority loop: durable meaning lives here, domain words
live in `CONTEXT.md`, current facts live with implementations, evidence lives in
realign/research/run artifacts, and delivery history lives in ledgers and
snapshots.
