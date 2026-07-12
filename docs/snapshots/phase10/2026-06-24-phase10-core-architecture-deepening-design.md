# Phase 10 Core Architecture Deepening Design

## Overview

Phase 10 deepens `opi` core architecture before tool, provider, session, TUI,
or ecosystem expansion proceeds. It follows Phase 9's `.repo/pi-0.80.2`
baseline realignment and turns the most important upstream architecture signals
into Rust-native seams.

The phase focuses on four areas:

- `opi-ai` provider collection and auth seam inspired by `pi-ai` `Models`;
- a generic `AgentHarness` seam in `opi-agent`;
- session repo/facade ownership and durable context boundaries;
- runtime hook boundaries that preserve future extension growth without copying
  the TypeScript extension API.

This is not a broad feature phase. It should make existing capabilities deeper,
more testable, and easier to evolve.

## Goals

- Design and implement a Rust-native provider collection/auth seam in
  `opi-ai`.
- Clarify provider construction versus provider use:
  - `opi-ai` owns provider/model/auth semantics;
  - `opi-coding-agent` owns CLI config, env resolution, package integration,
    and product defaults.
- Move or introduce generic harness primitives in `opi-agent` so library users
  and the CLI can share runtime orchestration semantics.
- Reduce `CodingHarness` to a coding-agent product wrapper over generic
  runtime seams.
- Define session repo/storage/facade responsibilities needed by Phase 13.
- Define hook boundaries across:
  - core runtime hooks;
  - coding-agent product extensions;
  - process adapter bridge;
  - future provider/UI/session lifecycle hooks.
- Preserve existing runtime behavior unless an explicit contract test requires
  a change.

## Non-Goals

- No provider OAuth login implementation.
- No Anthropic/OpenAI Codex/GitHub Copilot subscription auth.
- No broad provider catalog expansion.
- No image generation.
- No custom TUI extension protocol.
- No npm/package marketplace work.
- No browser/web UI work.
- No `pi` TypeScript extension API compatibility.
- No `pi` session file compatibility.
- No shared `opi-types` crate.
- No whole-loop rewrite.

## Upstream Evidence

| Area | Source | Evidence | Phase 10 implication |
|---|---|---|---|
| `Models` runtime | `.repo/pi-0.80.2/packages/ai/CHANGELOG.md:81` | Provider collection owns model reads, refresh, auth resolution, stream and complete methods. | `opi-ai` should expose a deeper provider/model/auth seam than isolated provider constructors. |
| Provider auth | `.repo/pi-0.80.2/packages/ai/CHANGELOG.md:82` | Auth is provider-owned and can include env auth, credential store, OAuth, and auth context. | `opi` should design auth extension points while deferring OAuth implementation. |
| Harness exports | `.repo/pi-0.80.2/packages/agent/src/index.ts:5,28-40` | `pi-agent-core` exports harness, session repos, prompt templates, skills, system prompt, and utilities. | `opi-agent` should own reusable orchestration/session primitives, not only low-level loop/session types. |
| Harness ownership | `.repo/pi-0.80.2/packages/agent/docs/agent-harness.md:3` | Harness owns session persistence, runtime config, resource resolution, operation locking, and extension-facing mutation semantics. | `CodingHarness` currently owns too much generic behavior. |
| Turn snapshots and save points | `.repo/pi-0.80.2/packages/agent/docs/agent-harness.md:58-60,140-150` | A stable turn snapshot is used per LLM turn; save points refresh future state without mutating in-flight provider requests. | Phase 10 should make this an explicit `opi-agent` contract. |
| Pending session writes | `.repo/pi-0.80.2/packages/agent/docs/agent-harness.md:84-88,176-196` | Busy writes are queued and flushed deterministically through a future session facade. | Phase 13 should not build on unordered direct session writes. |
| Semi-durable recovery | `.repo/pi-0.80.2/packages/agent/docs/durable-harness.md:19,26,42,118` | Session owns durable state; hosts recreate runtime dependencies on resume; recovery reduces entries. | `opi-agent` needs session-owned durable runtime state boundaries before long-term workflow features. |
| Existing `opi` ownership rule | `docs/opi-spec.md:322-324` | Generic harness primitives belong in `opi-agent`; coding-specific behavior belongs in `opi-coding-agent`. | Phase 10 follows an existing project rule rather than introducing a new architectural doctrine. |
| No type hub | `docs/opi-spec.md:1306` | Types live with semantic owner. | Do not create `opi-types`; expose lower-level owners directly. |

## Workstream 10.1: `Models/Auth` Seam

### Intent

`opi-ai` currently has provider adapters, model metadata, and a registry, while
`opi-coding-agent` performs much of the runtime construction around config and
env. Phase 10 should introduce a deeper seam that represents a collection of
providers/models and owns the provider-side auth contract.

### Target responsibilities

| Module | Responsibility |
|---|---|
| `opi-ai::Provider` | Wire-specific streaming contract. |
| `opi-ai::ProviderRegistry` or successor | Registered providers and models. |
| New `opi-ai` provider collection | Model lookup, refresh if supported, auth resolution, stream/complete dispatch. |
| New auth types | Provider-owned auth contract, static API key auth, env auth descriptor, explicit future OAuth extension point. |
| `opi-coding-agent` provider factory | Read TOML/env/package inputs and construct the `opi-ai` collection. |

### Required decisions

- Whether the existing `ProviderRegistry` evolves into the collection or a new
  type wraps it.
- Whether auth is resolved per provider request, per run, or per turn snapshot.
- How profile compatibility flags live with model metadata.
- How provider diagnostics expose missing/invalid auth without leaking secrets.
- How custom provider/model registration feeds the collection without creating
  dynamic Rust plugin loading.

### Success criteria

1. Existing provider paths still work.
2. `opi-coding-agent` no longer needs to scatter provider/model/auth policy
   across unrelated modules.
3. OpenAI-compatible profiles have a clear home for compatibility flags.
4. OAuth can be added later without redesigning provider construction.
5. Phase 12 provider correctness can write fixture tests against the collection
   and individual adapters.

## Workstream 10.2: Generic `AgentHarness`

### Intent

`opi-agent` should expose a generic orchestration layer above the low-level
agent loop. `opi-coding-agent::CodingHarness` should own coding-agent product
behavior, not generic turn lifecycle, runtime config mutation, session
persistence ordering, and save-point semantics.

### Target responsibilities

| Module | Responsibility |
|---|---|
| `opi-agent::agent_loop` | Low-level provider/tool loop and event semantics. |
| `opi-agent::Agent` | Stateful runtime wrapper and control handles. |
| New `opi-agent::harness` | Turn snapshot, phase guard, save points, pending session writes, runtime config mutation, generic resources/system prompt hooks. |
| `opi-coding-agent::CodingHarness` | Built-in file tools, CLI config, context files, package resources/adapters, interactive commands, product defaults. |

### Required semantics

- Explicit phases such as `idle`, `turn`, `compaction`, and `branch_summary`.
- Structural operations reject while busy.
- Queue operations are accepted at documented safe points.
- Runtime config setters affect future snapshots, not in-flight provider
  requests.
- Agent-emitted messages persist before pending extension/session writes.
- Pending session writes flush at save points and operation settlement.
- Abort leaves no active operation and does not silently discard accepted
  pending writes.

### Success criteria

1. `opi-agent` has a documented generic harness seam or an approved design for
   one.
2. `CodingHarness` is explicitly a product wrapper and no longer the only
   owner of generic orchestration semantics.
3. Contract tests cover phase guards, save points, busy rejections, queue
   behavior, cancellation, and session write ordering.
4. Existing CLI, RPC, JSON, and interactive behavior are preserved unless
   tests reveal a current bug.

## Workstream 10.3: Session Repo and Facade

### Intent

Phase 13 will add richer session context. Phase 10 should first define the
generic session seam so those entries are not added through ad hoc CLI-only
paths.

### Target responsibilities

| Concept | Owner | Notes |
|---|---|---|
| Session entry types | `opi-agent` | Messages, compaction, leaf, model/thinking changes, labels, branch summaries, custom entries. |
| Session storage/repo traits | `opi-agent` | Generic durable append/load/list/fork behavior. |
| Session directory/product policy | `opi-coding-agent` | User config path, CLI flags, session commands. |
| Harness session facade | `opi-agent` | Ordered read/write semantics during idle and busy phases. |
| Export renderers | likely `opi-coding-agent` initially | Product surface unless embedders need the renderer. |

### Required decisions

- Whether v1 can accept additive entries or session v2 is required.
- How unknown future entries are preserved or skipped.
- Whether branch summaries are context messages, metadata, or both.
- How extension custom messages enter provider context.
- How recovery marks unfinished operations without retrying unsafe tools.

### Success criteria

1. Phase 13 has a stable session seam to extend.
2. Existing v1 sessions remain readable.
3. Branch leaf reconstruction remains deterministic.
4. Pending writes have documented ordering.
5. Durable state belongs to the session log unless a sidecar is explicitly
   justified.

## Workstream 10.4: Runtime Hook Boundaries

### Intent

`pi` has broad TypeScript extension surfaces, including provider hooks, session
lifecycle hooks, custom UI, and message renderers. `opi` should keep future
paths open without copying that API into Rust core.

### Boundary model

| Surface | Phase 10 owner | Phase 10 action |
|---|---|---|
| Core loop hooks | `opi-agent` | Keep contract-tested and narrow. |
| Generic harness events/results | `opi-agent` | Design event/result reducers only where needed by generic lifecycle. |
| Coding-agent extension registry | `opi-coding-agent` / bridge to `opi-agent` | Keep product-specific commands/resources/packages here. |
| Process adapter protocol | `opi-coding-agent` | Keep until a non-CLI embedder needs hosting. |
| Provider request/response hooks | Future candidate | Defer until provider seam and trace/redaction semantics stabilize. |
| Custom TUI UI/message renderer | Future candidate | Defer until Phase 14 built-in TUI is stable and a UI protocol is designed. |

### Success criteria

1. Extension API docs stop implying that all `pi` TS extension surfaces are
   current `opi` scope.
2. Provider and UI hook expansion has explicit prerequisites.
3. Process adapter responsibilities remain out of `opi-agent` unless a concrete
   non-CLI host needs them.
4. Hook result composition is typed and tested where it affects runtime
   behavior.

## Crate Boundary Rules

Phase 10 should use these rules when deciding where code belongs.

| Rule | Consequence |
|---|---|
| If a library consumer needs it without the `opi` binary, it belongs in `opi-agent` or `opi-ai`. |
| If it knows about CLI flags, project config, built-in tools, package stores, or interactive commands, it belongs in `opi-coding-agent`. |
| If it is provider-facing wire/auth/model metadata, it belongs in `opi-ai`. |
| If it is visual rendering or input behavior without product policy, it belongs in `opi-tui`. |
| If it is only shared because moving it feels convenient, do not create a new crate. |

No `opi-types` crate is introduced. Cross-crate types should be owned by the
lower semantic crate and re-exported where necessary.

## Revised Phase Dependencies

```text
Phase 9 baseline/evidence
  -> Phase 10 core architecture deepening
    -> Phase 11 tooling quality
    -> Phase 12 provider correctness
    -> Phase 13 session tree/context/export
    -> Phase 14 TUI product polish
      -> future ecosystem candidates
```

The dependencies are not strictly serial for every small fix, but any major
work in Phase 11-14 should check whether it relies on a Phase 10 seam.

## Testing Strategy

| Level | Coverage |
|---|---|
| unit | provider collection/auth resolution, session reducer helpers, phase state machine helpers. |
| integration | existing provider construction through CLI config, harness prompt/resume/compact flows, RPC busy-state behavior. |
| contract | event order, save-point refresh, runtime config changes during active turns, pending session writes, abort cleanup. |
| regression | current interactive, non-interactive, JSON, and RPC paths keep behavior. |
| docs guard | no OAuth/image generation/custom UI/npm/gallery claims enter Phase 10 scope. |

Use mock providers and temp sessions. Do not add live provider calls to default
tests.

## Success Criteria

Phase 10 is complete when:

1. `opi-ai` has a documented provider collection/auth seam or an explicit
   implementation plan accepted by the project.
2. Provider construction in `opi-coding-agent` routes through that seam.
3. `opi-agent` owns or has a committed design for a generic harness with
   phase, snapshot, save-point, and pending-write semantics.
4. `CodingHarness` is documented as a coding-agent product wrapper.
5. Session repo/facade boundaries are defined for Phase 13.
6. Runtime hook boundaries distinguish current core/product surfaces from
   future provider/UI/session lifecycle ecosystem surfaces.
7. Existing behavior is covered by focused regression tests.
8. No ecosystem breadth feature is added.

## Implementation Notes

- Start with design and type boundaries before moving large code blocks.
- Avoid a large single migration. Prefer thin adapters that let existing
  `CodingHarness` behavior continue while generic seams are introduced.
- Keep public APIs unstable 0.x unless already classified otherwise.
- Preserve current session files and add migration only when a new entry shape
  forces it.
- Use `thiserror` in library crates and keep `anyhow` at application edges.
- Update English and Chinese normative docs when crate ownership or roadmap
  statements change.
