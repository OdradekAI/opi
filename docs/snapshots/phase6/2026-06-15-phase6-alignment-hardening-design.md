# Phase 6 Alignment Hardening Design

## Overview

Phase 6 is a corrective and hardening phase after the 0.5.1 release. Its job is
to make the project state auditable, align documentation with implementation,
and stabilize the Rust-native extension/package/runtime surfaces without
expanding opi core beyond the pi-inspired product direction.

The phase does not try to make opi a TypeScript-compatible clone of pi. It
preserves the central rule from earlier phases: semantic and product-direction
alignment with pi, implemented through Rust-native crate boundaries and runtime
interfaces.

## Goals

- Make current-version and phase-status documentation match the 0.5.1
  workspace.
- Reconcile conflicting Phase 5 audit artifacts into a single Phase 6 baseline
  that explains which findings are closed, accepted, or carried forward.
- Harden the package-to-runtime path:

  ```text
  opi package add
    -> declaration and lock state
    -> later startup
    -> package resolution
    -> adapter startup
    -> tool/command/hook/event/state bridge
    -> diagnostics
  ```

- Stabilize the documented behavior of `opi-extension-jsonl-v1` enough for
  0.x users and package authors to reason about lifecycle, failures, state,
  cancellation, and hook order.
- Clarify the relationship between opi session JSONL and pi session v3 without
  promising file-format compatibility.
- Ensure RPC startup behavior reflects package diagnostics and adapter
  availability consistently with interactive and non-interactive modes.
- Add documentation guards that prevent overclaiming npm, marketplace, hot
  reload, permission enforcement, pi-web-ui parity, TypeScript extension API
  compatibility, or broad OAuth/provider parity.
- Produce a Future Ecosystem candidate list for expansion without implementing
  those expansion features in Phase 6 or committing them to the next phase.

## Non-Goals

- No npm package registry support.
- No package gallery or marketplace.
- No package update, enable, or disable command.
- No package permission enforcement or sandbox.
- No hot reload.
- No bundled Node.js, TypeScript, or `jiti` runtime.
- No TypeScript extension API compatibility.
- No pi session v3 read/write compatibility.
- No provider OAuth parity work.
- No new first-class provider broadening unless needed for an existing failing
  Phase 6 test.
- No pi-web-ui parity work.
- No new shared `opi-types` crate.
- No migration of adapter protocol types out of `opi-coding-agent` unless a
  concrete non-CLI host needs them in this phase.

## Alignment Principles

| Principle | Phase 6 meaning |
|---|---|
| pi is the product reference | opi should keep pi's minimal terminal coding-agent direction and extension-first customization model. |
| Rust boundaries are authoritative | opi should not copy TS package structure when a Rust trait, enum, module, or crate boundary is clearer. |
| Core stays small | Workflow-heavy capabilities remain packages, examples, or later extension surfaces, not built-ins. |
| Truthful docs are part of correctness | Version, phase, capability, and non-goal statements must match code and tests. |
| Hardening beats feature breadth | Phase 6 closes ambiguity and runtime risk before adding ecosystem breadth. |

## Current State

The 0.5.1 workspace has released the Phase 5 package loop and process adapter
MVP. The root README files identify the workspace as 0.5.1 and describe
trusted package execution and `process-jsonl` adapters. Several design,
alignment, and crate README files still describe the current workspace as
0.5.0. Phase 5 audit snapshots also contain stale or conflicting conclusions
from different points in the remediation timeline.

Phase 6 treats `Cargo.toml`, `CHANGELOG.md`, current code, and the latest
release commit as the source of implementation truth. Older audit findings are
not discarded; they are classified against current code and either closed,
accepted as design tradeoffs, or carried forward as Phase 6 work.

## Architecture

Phase 6 keeps the existing crate ownership model.

| Crate | Phase 6 responsibility |
|---|---|
| `opi-ai` | No planned scope except provider docs or tests needed to keep alignment claims truthful. |
| `opi-agent` | Runtime contracts, session events, extension traits, agent loop guarantees, and state persistence semantics. |
| `opi-coding-agent` | Package store, package CLI, adapter host, RPC startup integration, diagnostics, and product policy. |
| `opi-tui` | No planned feature scope; only touched if documentation or focused tests reveal a current regression. |
| `opi-web-ui` | Scope statement only. It remains unpublished and is not treated as pi-web-ui parity. |

Adapter protocol and package manager code remain in `opi-coding-agent` because
the current adapter host is a coding-agent product surface. `opi-agent` should
only expose the stable runtime concepts the bridge maps to: tools, commands,
hooks, events, state, and session messages.

## Workstreams

### 1. Documentation Truth

Update current-state documentation to match 0.5.1:

- `docs/opi-spec.md`
- `docs/opi-spec.zh.md`
- `docs/pi-alignment-matrix.md`
- `docs/pi-alignment-matrix.zh.md`
- `README.md`
- `README.zh.md`
- `crates/*/README.md`
- `crates/*/README.zh.md`

The update must preserve historical release rows. Historical references to
0.5.0 remain valid when they describe the 0.5.0 release. Current implementation
statements must say 0.5.1.

Crate README status/version lines must match the workspace version unless a
crate intentionally documents a different unpublished status, such as
`opi-web-ui`.

The docs must distinguish:

- completed Phase 5 MVP behavior;
- Rust-native adapter behavior that differs from pi's TypeScript extension
  loading;
- deferred ecosystem features;
- `opi-web-ui` as an unpublished Rust component/state/rendering crate, not a
  browser app equivalent to pi-web-ui.

### 2. Phase Audit Baseline

Add a Phase 6 baseline document under `docs/snapshots/phase6/` that records:

- the exact release and commit being audited;
- which Phase 5 audit findings are closed by 0.5.1;
- which findings are accepted design differences;
- which findings become Phase 6 tasks;
- which findings are moved to the Future Ecosystem candidate backlog.

The baseline should not delete historical Phase 5 audits. Those files are part
of the project record and should remain immutable unless the user explicitly
asks to rewrite them.

### 3. Adapter Protocol Hardening

Document and test `opi-extension-jsonl-v1` behavior around:

- initialize and capability negotiation;
- unsupported protocol handling;
- deterministic lifecycle order;
- hook declaration and hook dispatch order;
- request id correlation;
- timeout behavior;
- best-effort cancellation;
- event observer fire-and-forget behavior;
- state serialize and restore behavior;
- shutdown behavior;
- adapter crash diagnostics.

The goal is not a stable 1.0 protocol. The goal is an honest 0.x protocol whose
observed behavior matches the docs and tests.

### 4. Package Runtime Hardening

Add focused coverage for the package product path:

```text
add local package
  -> write declaration and lock
  -> restart or rebuild runtime
  -> resolve package
  -> start adapter
  -> expose adapter tool or command
  -> persist adapter state when applicable
```

The same path should have failure coverage for:

- stale or mismatched lock state;
- missing package root;
- invalid manifest;
- unsupported adapter protocol;
- adapter initialize timeout;
- adapter process exit during startup;
- duplicate manifest names across the same precedence layer;
- project package overriding a global package with the same manifest name.

### 5. Session and RPC Boundary

Clarify opi's session format as Rust-native session JSONL. Phase 6 should not
promise pi session v3 compatibility. It should document which pi session
concepts are represented today and which are intentionally absent or deferred.

Focused tests should verify that:

- extension state is restored before adapter-backed runtime behavior needs it;
- extension state is persisted after turns that mutate adapter state;
- startup diagnostics are available in RPC mode;
- adapter-backed commands have consistent behavior through RPC and interactive
  command paths where the same runtime abstraction is used.

### 6. Alignment Guards

Add or extend documentation guard tests so docs cannot claim these features are
complete in the current phase:

- npm package installation;
- package marketplace or gallery;
- package update, enable, or disable commands;
- package permission enforcement or sandboxing;
- adapter hot reload;
- custom TUI adapter components;
- provider streaming adapters;
- TypeScript extension API compatibility;
- pi session v3 compatibility;
- pi-web-ui parity;
- broad OAuth/provider parity.

Positive guards should ensure docs continue to state the completed Phase 5 MVP:

- package add/remove/list/doctor;
- local and git package sources;
- manifest V2 adapter declarations;
- `process-jsonl` adapter support;
- `opi-extension-jsonl-v1`;
- adapter tools, commands, selected hooks, events, state, cancellation, and
  diagnostics;
- trusted-code security model.

### 7. Future Ecosystem Candidate Backlog

Create a concise backlog section in the Phase 6 baseline or alignment matrix
for features that are aligned with pi but outside Phase 6:

- package enable and disable;
- package update;
- source filters and package info;
- npm or registry-backed packages;
- package gallery metadata;
- provider OAuth flows;
- additional provider breadth;
- stronger package trust or sandbox model;
- browser web-ui product surface;
- session import or migration tooling;
- extension UI/RPC UI sub-protocol and custom message/rendering surfaces.

The backlog must not be worded as committed next-phase scope. It is a candidate
list for later prioritization.

## Data Flow

### Current-State Documentation Flow

```text
Cargo.toml and CHANGELOG.md
  -> current version and release truth
  -> opi-spec current-state sections
  -> pi alignment matrix
  -> docs guard tests
```

### Package Runtime Flow

```text
package declaration
  -> package lock
  -> resolver
  -> manifest parser
  -> resource composition
  -> adapter startup
  -> extension registry
  -> harness / runner / RPC
  -> session state persistence
```

### Audit Flow

```text
historical audits
  -> classify against 0.5.1 code
  -> closed / accepted / Phase 6 / Future Ecosystem candidate
  -> Phase 6 baseline
```

## Error Handling

Phase 6 should prefer clear diagnostics over broad recovery. When package or
adapter behavior degrades, static resources may still load, but the degraded
runtime state must be visible to `doctor`, startup diagnostics, and RPC where
applicable.

Error messages should identify:

- package source;
- package scope;
- manifest name when available;
- adapter command when relevant;
- expected and actual protocol when negotiation fails;
- timeout surface when a timeout occurs;
- whether the package is disabled at runtime or only degraded.

## Testing Strategy

| Level | Coverage |
|---|---|
| docs guard | current version, Phase 5 MVP claims, and forbidden overclaims |
| unit | manifest parsing, package lock diagnostics, adapter protocol serde, session-state helpers |
| integration | package add/list/doctor/startup with local temp packages |
| adapter contract | mock adapter initialize, tool, command, hook, event, cancel, state, shutdown, crash, timeout |
| RPC | startup diagnostics and adapter command availability through RPC command paths |
| final gates | `cargo fmt --check --all`, focused tests, and `cargo clippy --workspace --all-targets -- -D warnings` |

Tests that touch sessions must isolate state with temp directories or
`OPI_SESSIONS_DIR`. Tests that touch packages must use temp directories and
avoid user package config.

## Success Criteria

Phase 6 is complete when:

1. Current implementation documentation consistently identifies the workspace
   as 0.5.1 while preserving historical release rows.
2. English and Chinese docs remain synchronized for all changed user-facing
   documentation.
3. A Phase 6 baseline audit explains the status of conflicting Phase 5 audit
   findings.
4. Package startup has focused tests for the successful local-package adapter
   path and the key degraded paths listed in this design.
5. Adapter protocol tests cover lifecycle, failure, cancellation, state, and
   shutdown behavior.
6. Session and RPC tests cover extension state persistence and startup
   diagnostics where relevant.
7. Documentation guards prevent Phase 6 from overclaiming deferred pi
   ecosystem features.
8. The Future Ecosystem candidate backlog exists and is explicitly
   non-committal.
9. No npm, marketplace, OAuth parity, pi-web-ui parity, permission enforcement,
   TS extension compatibility, or new shared type crate is added in Phase 6.
10. Final verification gates pass.

## Implementation Notes

- Prefer updating existing focused tests over adding broad, slow end-to-end
  coverage.
- Do not rewrite old snapshot audits. Add a new baseline instead.
- Do not move package or adapter protocol types between crates during Phase 6
  unless a concrete failing test proves the current boundary is wrong.
- Keep any code changes surgical and tied to a Phase 6 success criterion.
- Update localized docs in the same change whenever the English counterpart is
  changed.
