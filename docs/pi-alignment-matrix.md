# pi Alignment Matrix

## Scope

This document compares `opi` against `.repo/pi-0.80.2` by semantic behavior,
Rust crate ownership, product workflow, and durable evidence anchors. It is not
a TypeScript API, package ABI, config-file, or session-file compatibility
checklist.

This file is the durable `pi` 0.80.2 evidence and alignment baseline. The
current conclusion: `opi` is close in core semantics, medium in product parity,
and intentionally low in ecosystem parity until the current capabilities are
deeper and more stable.

## Document Control

| Field | Value |
|---|---|
| Upstream path | `.repo/pi-0.80.2` |
| Upstream package version | `0.80.2` for `@earendil-works/pi-ai`, `@earendil-works/pi-agent-core`, `@earendil-works/pi-tui`, and `@earendil-works/pi-coding-agent` |
| Opi workspace version | `0.7.0` |
| Date sampled | 2026-06-24 |
| Evidence scope | Local files under `.repo/pi-0.80.2`, current `docs/opi-spec.md`, current `docs/pi-alignment-matrix.md`, and current `crates/*` layout |
| Update policy | Update this document whenever the studied `pi` baseline changes or when `opi` closes one of the listed gaps. Preserve useful old evidence as historical context instead of silently rewriting it. |

## Executive Summary

`opi` is still directionally aligned with `pi`: it preserves the
terminal-first coding-agent shape, provider streaming, tool calling, session
persistence, compaction, JSON/RPC surfaces, package/process-adapter ideas, and
extension hooks. The drift is not mainly feature names. The larger drift is
architectural: `pi` 0.80.2 has moved important ownership into `pi-ai`
`Models/Auth` and `pi-agent-core` `AgentHarness`/session repo primitives, while
`opi` still keeps much of the comparable orchestration in
`opi-coding-agent::CodingHarness` and provider construction policy around the
CLI/config layer.

The right adjustment is not to copy TypeScript package structure into Rust.
The right adjustment is to deepen Rust-native seams before adding ecosystem
breadth:

- `opi-ai` should grow a provider collection/auth seam inspired by `pi-ai`
  `Models`, while keeping OAuth and image generation out of the near-term core.
- `opi-agent` should own generic harness/session facade semantics needed by
  embedders, while `opi-coding-agent` remains the product wrapper for CLI,
  tools, config, packages, and interactive commands.
- `opi-tui` should continue using `ratatui`/`crossterm`; custom extension UI
  and message renderers are future ecosystem candidates, not Phase 14 scope.
- Product parity is medium, core semantic parity is high but incomplete, and
  ecosystem parity is intentionally low until the existing product is deep and
  stable.

## Pi Architecture

### `@earendil-works/pi-ai`

`pi-ai` 0.80.2 is no longer just a set of wire adapters. It treats a provider
as the runtime unit that owns model catalog, auth, and stream behavior, while a
`Models` collection routes requests to the owning provider
(`.repo/pi-0.80.2/packages/ai/README.md:227-231`). Provider factories are split
per provider for selective imports and a heavy explicit `providers/all`
entrypoint supplies all built-ins (`README.md:233-261`).

Auth is provider-owned. The `Models` collection resolves auth through the
owning provider for request paths and exposes `getAuth()` for status UIs
(`README.md:321-348`). Stored credentials live behind a small
`CredentialStore` contract with serialized writes; OAuth refresh runs inside
that lock and a stored credential owns the provider without silent env fallback
(`README.md:350-362`). OAuth providers exist for Anthropic, OpenAI Codex, and
GitHub Copilot (`README.md:1361-1369`).

Image generation mirrors the chat-side architecture through a separate
`ImagesModels`/`ImagesProvider` surface (`README.md:634-663`). This means image
generation is aligned with `pi`, but it depends on the same collection/auth
ideas and should not be added to `opi` before the chat provider seam is stable.

### `@earendil-works/pi-agent-core`

`pi-agent-core` exports the low-level agent, loop functions, `AgentHarness`,
harness messages, prompt templates, session repos, skills, system prompt
helpers, harness types, and utilities
(`.repo/pi-0.80.2/packages/agent/src/index.ts:1-40`).

`AgentHarness` is the orchestration layer above the low-level loop. It owns
session persistence, runtime config, resource resolution, operation locking,
and extension-facing mutation semantics
(`.repo/pi-0.80.2/packages/agent/docs/agent-harness.md:1-5`). The harness
separates config, session, pending writes, and turn snapshots. A turn snapshot
is the concrete state used for one LLM turn
(`agent-harness.md:34-60`). Save points flush pending writes after
agent-emitted messages, create fresh snapshots for future turns, and avoid
mutating in-flight provider requests (`agent-harness.md:140-150`).

The durable direction is semi-durable rather than fully serialized runtime
state: the session log is the durable state tree, while the host recreates
runtime dependencies on resume
(`.repo/pi-0.80.2/packages/agent/docs/durable-harness.md:19-28,42-44`).

### `@earendil-works/pi-tui`

`pi-tui` is a reusable TypeScript terminal UI library with its own renderer and
component model. `opi-tui` intentionally does not copy that stack: it uses
Rust-native `ratatui`/`crossterm` widgets. The alignment target is therefore
product behavior and terminal ergonomics, not renderer API compatibility.

### `@earendil-works/pi-coding-agent`

`pi-coding-agent` is the product layer: CLI/TUI modes, tools, sessions,
extensions, package workflows, export/share/update surfaces, and rich extension
integration. Its extension documentation shows a broader surface than `opi`
currently exposes: user prompts, custom UI components, custom commands, event
interception, session lifecycle hooks, provider hooks, message renderers, and
example extensions
(`.repo/pi-0.80.2/packages/coding-agent/docs/extensions.md:10-14,297-299,438-440,2177-2185,2397-2403,2524-2526,2628-2632`).

For `opi`, this is future ecosystem evidence, not immediate scope. The current
Rust process-adapter path is a good substrate, but it does not imply parity
with TypeScript custom UI, npm/gallery behavior, provider payload hooks, or
session publishing.

## Version Evolution Signals

These are the 0.80.2 signals that materially change `opi` planning compared
with the older `.repo/pi-0.75.3` baseline.

| Signal | Evidence | Effect on `opi` |
|---|---|---|
| `Models` runtime in `pi-ai` | `.repo/pi-0.80.2/packages/ai/CHANGELOG.md:81` | Insert Phase 10 before provider correctness so `opi-ai` has a provider collection/model/auth seam to test against. |
| Provider-owned auth substrate | `.repo/pi-0.80.2/packages/ai/CHANGELOG.md:82`; `packages/ai/README.md:321-362` | Separate provider auth semantics from CLI env/config parsing; defer OAuth until the seam is stable. |
| Provider factories and built-in catalog | `.repo/pi-0.80.2/packages/ai/CHANGELOG.md:83`; `packages/ai/README.md:233-261` | Keep Rust profiles explicit and registry-backed; do not hardcode every compatible provider into core. |
| OAuth providers | `.repo/pi-0.80.2/packages/ai/CHANGELOG.md:84`; `packages/ai/README.md:1361-1369` | Aligned but future ecosystem scope because it needs credential store, login UX, redaction, doctor, and revocation semantics. |
| Image generation collection | `.repo/pi-0.80.2/packages/ai/CHANGELOG.md:86`; `packages/ai/README.md:634-663` | Future ecosystem candidate after chat-side provider collection/auth correctness. |
| `AgentHarness` export and docs | `.repo/pi-0.80.2/packages/agent/src/index.ts:5,28-40`; `packages/agent/docs/agent-harness.md:3` | `opi-agent` should be marked Partial until it owns a Rust-native generic harness seam. |
| Turn snapshot/save-point semantics | `.repo/pi-0.80.2/packages/agent/docs/agent-harness.md:58-60,140-150` | Phase 10 should define snapshot/save-point contracts before Phase 13 session work. |
| Pending session writes and planned facade | `.repo/pi-0.80.2/packages/agent/docs/agent-harness.md:84-90,176-196` | Phase 13 builds on the session facade instead of ad hoc CLI-only writes. |
| Semi-durable harness | `.repo/pi-0.80.2/packages/agent/docs/durable-harness.md:19-28,38-44,118-121` | Long-running context belongs in session entries unless a sidecar has a clear durable reference model. |
| Extension UI and lifecycle breadth | `.repo/pi-0.80.2/packages/coding-agent/docs/extensions.md:10-14,297-299,438-440,2177-2185,2397-2403,2524-2526` | Do not claim extension UI parity; keep it as future ecosystem work after built-in TUI is stable. |
| Provider hook breadth | `.repo/pi-0.80.2/packages/agent/docs/agent-harness.md:443`; `.repo/pi-0.80.2/packages/coding-agent/docs/extensions.md:1646-1681` | Provider request/response adapter hooks are future work after provider seam, trace, and redaction contracts are stable. |

## Evidence Index

| Source | Evidence summary | Affected `opi` area | Roadmap implication |
|---|---|---|---|
| `docs/superpowers/specs/2026-06-24-phase9-pi-0-80-2-baseline-realignment-design.md:59-61` | Phase 9 design records that pre-realignment docs used `.repo/pi-0.75.3` as the studied baseline. | Documentation baseline | Phase 9 rebases current comparisons to `.repo/pi-0.80.2`. |
| `docs/opi-spec.md:342-345` | Existing `opi` rule already says generic harness primitives belong in `opi-agent`, coding behavior in `opi-coding-agent`. | Crate boundaries | Phase 10 follows existing Rust ownership guidance rather than inventing a new split. |
| `docs/opi-spec.md:248,1462` | The spec and ADR-003 reject a shared `opi-types` crate. | Crate boundaries | Keep cross-crate types in semantic owners. |
| `.repo/pi-0.80.2/packages/ai/README.md:229-231` | Provider owns catalog/auth/stream behavior; `Models` routes requests. | `opi-ai` | Provider collection/auth seam belongs in `opi-ai`. |
| `.repo/pi-0.80.2/packages/ai/README.md:323-348` | Auth resolves through owning provider and can be inspected without a request. | `opi-ai`, diagnostics, TUI status | Phase 10 should expose missing/available auth state without leaking secrets. |
| `.repo/pi-0.80.2/packages/ai/README.md:350-362` | Credential store uses serialized writes and prevents silent env fallback after stored credential refresh failure. | Future auth ecosystem | OAuth requires a deliberate credential-store and revocation design. |
| `.repo/pi-0.80.2/packages/ai/README.md:634-663` | Image generation uses a separate collection mirroring chat-side design. | Future `opi-ai` ecosystem | Do not add image generation before chat provider collection/auth is stable. |
| `.repo/pi-0.80.2/packages/agent/src/index.ts:1-40` | Agent core exports harness, messages, prompt templates, session repos, skills, system prompt, and utilities. | `opi-agent` | Current `opi-agent` alignment is Partial until generic harness/session repo breadth exists. |
| `.repo/pi-0.80.2/packages/agent/docs/agent-harness.md:3` | Harness owns persistence, runtime config, resources, locking, and mutation semantics. | `opi-agent`, `opi-coding-agent` | `CodingHarness` should become a product wrapper over generic runtime seams. |
| `.repo/pi-0.80.2/packages/agent/docs/agent-harness.md:58-60,140-150` | Turn snapshot and save points keep future state updates out of in-flight provider requests. | `opi-agent` | Add explicit contract tests when implementing Phase 10 harness seam. |
| `.repo/pi-0.80.2/packages/agent/docs/agent-harness.md:84-90,176-196` | Pending writes are queued and a session facade is planned. | `opi-agent` sessions | Phase 13 depends on Phase 10 session facade definition. |
| `.repo/pi-0.80.2/packages/agent/docs/durable-harness.md:19-28,38-44,118-121` | Session log is the durable state tree; hosts recreate runtime dependencies. | `opi-agent` sessions | Avoid hidden global memory for long-running workflow state. |
| `.repo/pi-0.80.2/packages/coding-agent/docs/extensions.md:10-14` | Extensions include custom tools, event interception, user interaction, custom UI, and commands. | `opi-coding-agent`, process adapters, TUI | Existing adapter substrate is Partial; custom UI is future ecosystem scope. |
| `.repo/pi-0.80.2/packages/coding-agent/docs/extensions.md:297-299,438-440` | Session compaction/tree hooks can customize or observe flows. | Sessions, extensions | Preserve future route for session lifecycle hooks without copying TS API. |
| `.repo/pi-0.80.2/packages/coding-agent/docs/extensions.md:2177-2185,2397-2403,2524-2526` | UI prompts and custom components receive TUI/theme/keybinding integration. | `opi-tui`, extension UI | Phase 14 should polish built-in TUI, not promise custom extension UI parity. |
| `.repo/pi-0.80.2/packages/coding-agent/docs/extensions.md:1646-1681` | Custom providers can include OAuth support for login. | Provider ecosystem | Future provider extension path depends on auth seam and product login design. |

## Alignment Levels

| Level | Meaning |
|---|---|
| Full | Implemented with equivalent user-visible or library-visible behavior and covered by tests. |
| Partial | Implemented as a substrate or narrower Rust-native equivalent. |
| Intentional Divergence | Opi deliberately uses a different Rust-native interface, storage format, renderer, or packaging model. |
| Missing | Capability exists in `pi`, but `opi` has no equivalent yet. |
| Out of Scope | Capability should not enter core without a later design changing the scope. |

## Alignment Dashboard

| Layer | Current level | Summary | Next adjustment |
|---|---|---|---|
| Core semantic parity | High but incomplete | Agent loop semantics, provider streaming, tool scheduling, compaction, session tree basics, JSON/RPC, extension hooks, and package adapters exist. The major remaining gaps are `Models/Auth`, generic `AgentHarness`, session facade, and explicit save-point semantics. | Insert Phase 9 and Phase 10 before further hardening. |
| Product parity | Medium | The `opi` binary is useful across TUI, non-interactive, JSON, RPC, sessions, packages, providers, diagnostics, and image input. Pi remains broader in provider auth UX, extension UI, package lifecycle, export/share, and polished session workflows. | Recast old Phase 9-12 as Phase 11-14 after core seams. |
| Ecosystem parity | Low by design | OAuth/subscription auth, broad provider catalog, image generation, npm/gallery/update/enable/disable, custom extension UI/message renderers, web/share, provider payload hooks, and pi session import are not current product claims. | Track as future candidates with entry conditions. |

## Package Alignment

| pi package | opi crate | Level | Implemented | Gaps / adjustment |
|---|---|---|---|---|
| `@earendil-works/pi-ai` | `opi-ai` | Partial | Provider trait, provider adapters, provider registry, model metadata, image input, usage/cost, retry/backoff, proxy config, OpenAI-compatible profiles, and custom provider/model registration. | Add a Rust-native provider collection/auth seam in Phase 10. Keep OAuth, image generation, and broad catalog work as future candidates. |
| `@earendil-works/pi-agent-core` | `opi-agent` | Partial | Agent loop, stateful `Agent`, hooks, tool batching, queues, sessions, compaction, SDK types, extension trait, diagnostics, streaming proxy primitives, and runtime contract tests. | Generic `AgentHarness`/session facade seams exist, but product turn/session adoption is still partial. |
| `@earendil-works/pi-tui` | `opi-tui` | Partial | Rust-native `ratatui`/`crossterm` widgets, transcript rendering, markdown/code, diff, pickers, branch/session picker snapshots, themes, keybindings, terminal images, and CJK display-width coverage. | Renderer API compatibility is an intentional divergence. Custom extension UI/message renderers remain future ecosystem work; Phase 14 should polish built-in product UI. |
| `@earendil-works/pi-coding-agent` | `opi-coding-agent` | Partial | CLI modes, built-in tools, config, sessions, context files, images, JSON/RPC, resources, packages, skills, prompt fragments, themes, custom provider registration, extension commands, branch/tree/fork/clone flows, package CLI, process-jsonl adapter hosting, diagnostics, and doctor checks. | Pi remains broader in custom extension UI, provider hooks/login, npm/gallery lifecycle, export/share, and update surfaces. Keep these gated behind future ecosystem designs. |

## Detailed Feature Alignment

### `pi-ai` / `opi-ai`

| Feature | Opi level | Evidence / current state | Adjustment |
|---|---|---|---|
| Provider stream lifecycle | Full | Provider-neutral stream events and adapters are implemented and tested. | Preserve fixture coverage for start/delta/end/done/error behavior. |
| Provider registry and model metadata | Partial | Registry and custom provider/model registration exist. | Evolve toward a provider collection seam rather than scattering construction policy. |
| `Models` collection | Partial | `opi` has registry/profile construction, but no direct equivalent of `createModels()` as the runtime owner. | Phase 10 should define model lookup, refresh, auth, and dispatch ownership in `opi-ai`. |
| Provider-owned auth | Partial | Static env/config credentials exist per provider. | Move auth semantics into `opi-ai`; keep CLI/env/package config as construction input. |
| OAuth/subscription auth | Missing | Not implemented. | Future candidate after credential store, redaction, doctor, login UX, refresh, and revocation design. |
| Image input | Partial | Image attachments and provider serialization are implemented. | Keep provider-specific image input correctness in Phase 12. |
| Image generation | Missing | Not implemented. | Future candidate after chat provider collection/auth stabilizes. |
| Broad provider catalog | Partial | Many first-class and OpenAI-compatible providers exist, but not `pi` breadth. | Prefer profiles unless wire/auth semantics require first-class adapters. |

### `pi-agent-core` / `opi-agent`

| Feature | Opi level | Evidence / current state | Adjustment |
|---|---|---|---|
| Low-level agent loop | Full | Event order, tool scheduling, hooks, queues, and cancellation semantics are implemented and tested. | Keep Phase 8 contracts as regression gates. |
| Stateful `Agent` wrapper | Full | Prompt/continue/abort/subscribe and queue behavior exist. | Preserve API as 0.x unless later stabilized. |
| Generic `AgentHarness` | Partial | `CodingHarness` owns much of the comparable orchestration. | Phase 10 should define generic harness phases, snapshots, save points, busy guards, and runtime mutation semantics in `opi-agent`. |
| Session storage | Partial | Append-only JSONL, resume/list/delete/fork, branch `parent_id`, `leaf`, compaction, extension state, and Phase 13 additive typed entries (`session_info`, `model_change`, `thinking_level_change`, `label`, `branch_summary`) on the v1 header exist; `custom_message` is deferred. | Phase 10 defined the session facade; Phase 13 implemented richer context entries. `branch_summary` generation UX and interactive `/export` are deferred to Phase 14. |
| Pending session write ordering | Partial | Generic harness/session facade seams expose ordered pending-write behavior, but product session adoption remains partial. | Phase 13 builds richer entries on the seam and keeps ordering contract tests. |
| Compaction | Full | Threshold/manual/overflow primitives and session events exist. | Keep branch-aware compaction tests. |
| Extension trait/hooks/state | Partial | Rust in-process extension API and process adapter bridge exist. | Keep narrow; future provider/UI/session lifecycle hooks need separate designs. |

### `pi-tui` / `opi-tui`

| Feature | Opi level | Evidence / current state | Adjustment |
|---|---|---|---|
| Terminal renderer | Intentional Divergence | `opi-tui` uses `ratatui`/`crossterm`, not `pi` TypeScript renderer. | Keep Rust-native renderer unless a separate reusable TUI product is approved. |
| Transcript and markdown/code rendering | Partial | Built-in transcript, markdown, code, and snapshots exist. | Phase 14 should polish dense terminal workflows. |
| Diff and image rendering | Partial | Diff rendering and terminal image primitives exist. | Keep product-focused; expand only where CLI workflows need it. |
| Pickers and keybindings | Partial | Model/session/branch pickers, themes, keybindings, and CJK-width snapshots exist. | Phase 14 should improve discovery, status, and accessibility. |
| Custom extension UI/message renderers | Missing | Not supported. | Future candidate after built-in TUI and UI/RPC protocol design. |

### `pi-coding-agent` / `opi-coding-agent`

| Feature | Opi level | Evidence / current state | Adjustment |
|---|---|---|---|
| CLI modes | Full | Interactive, non-interactive, JSON, RPC, model listing, completions, sessions, doctor, and package commands exist. | Keep command contracts documented and tested. |
| Built-in tools | Partial | `read`, `write`, `edit`, `bash`, `grep`, `find`, `ls`, and `glob` exist with mode-aware policy. | Phase 11 should harden paths, encodings, truncation, cancellation, and diagnostics. |
| Config/resource discovery | Partial | TOML layers, provider profiles, context files, resources, skills, prompt fragments, themes, packages, and extensions exist. | Keep precedence and diagnostics explicit. |
| Sessions and branch workflows | Partial | Resume/list/delete/fork, `/tree`, `/branch`, `/fork`, `/clone`, active branch continuation, compaction, stable metadata, labels, local export, and branch-summary context substrate exist. | Branch-summary generation UX remains deferred to Phase 14. |
| Package/process adapter substrate | Partial | Local/git package sources, manifest V2, `process-jsonl`, adapter tools/commands/hooks/events/state/cancellation, examples. | Stabilize before npm/gallery/update/enable/disable. |
| Provider hooks/login UX | Missing | Custom provider registration exists; provider request/response hook parity and login flows do not. | Future candidate after Phase 10/12 provider seam and redaction/trace design. |
| Export/share/web surfaces | Partial | Local session export exists; web/share is not implemented. | Future candidate after Phase 13 sensitivity and redaction rules. |

## Phase Alignment

| Phase | Scope | Crate(s) | Current level | Notes / adjustment |
|---:|---|---|---|---|
| 1 | MVP foundation: Anthropic, core loop, basic tools, TUI, config | all crates | Full to Partial | Core shipped; read-only tool breadth later completed with `find`/`ls` and extra `glob`. |
| 2 | Multi-provider, sessions, compaction, JSON mode, retry/cost/thinking | `opi-ai`, `opi-agent`, `opi-coding-agent`, `opi-tui` | Partial | Core exists; provider collection/auth and richer sessions remain gaps. |
| 3 | Production hardening: enterprise providers, image input, context files, tool policy, completions, proxy | all crates | Partial | Image input exists; image generation remains future. |
| 4 | Extensibility substrate: RPC, SDK, extensions, resources, skills, themes, packages, custom providers, branch UI, streaming proxy | all crates | Partial | Strong substrate, but not TypeScript extension or custom UI parity. |
| 5 | Package/process-adapter MVP | `opi-coding-agent`, `opi-agent` | Partial | Local/git/process-jsonl path exists; npm/gallery/update/enable/disable remain future. |
| 6 | Alignment and reliability hardening | workspace | Partial | Documentation and runtime integration hardened without expanding ecosystem scope. |
| 7 | Reliability and observability | `opi-agent`, `opi-coding-agent`, `opi-ai` | Partial | Diagnostics, redaction, trace envelopes, doctor checks are local and explicit. |
| 8 | Runtime stabilization | `opi-agent`, `opi-coding-agent` | Partial | Event order, hooks, tool scheduling, cancellation, SDK/RPC contracts, and API surface classification are tested. |
| 9 | pi 0.80.2 baseline realignment | docs | Planned | Documentation/evidence gate; no runtime changes. |
| 10 | Core architecture deepening | `opi-ai`, `opi-agent`, `opi-coding-agent` | Partial | `Models/Auth`, generic `AgentHarness`, session facade, runtime hook boundaries. |
| 11 | Tooling quality | `opi-coding-agent`, `opi-agent`, `opi-tui` | Planned | Recast from old Phase 9; depends on Phase 10 boundaries. |
| 12 | Provider correctness | `opi-ai`, `opi-coding-agent` | Partial | Recast from old Phase 10; fixture lifecycle/error/taxonomy/profile-flags/cache/response-id/retry/cancel/auth-diagnostic coverage shipped through the provider collection/auth seam. `usage_in_stream` now requests `stream_options.include_usage`, OpenAI Chat captures response IDs from any chunk carrying `id`, missing usage stays explicitly unknown so session cost summaries can be omitted, and `previous_response_id` plus request-side cache-control breakpoints remain deferred. |
| 13 | Session tree and context reconstruction | `opi-agent`, `opi-coding-agent`, `opi-tui` | Partial | Recast from old Phase 11; generic harness/session facade is present. Generation UX polish continues in Phase 14. |
| 14 | TUI product polish | `opi-tui`, `opi-coding-agent` | Planned | Recast from old Phase 12; built-in TUI only, not custom extension UI parity. |

## Roadmap Implications

| Phase | Name | Reason |
|---:|---|---|
| 9 | pi 0.80.2 Baseline Realignment | The previous docs were based on an older upstream snapshot and overstated `opi-agent` parity. |
| 10 | Core Architecture Deepening | `pi` 0.80.2 makes `Models/Auth` and `AgentHarness` central enough that tool/provider/session/TUI work should build on those seams. |
| 11 | Tooling Quality | Old Phase 9 remains useful, but it should depend on Phase 10 contracts for harness/tool scheduling and diagnostics. |
| 12 | Provider Correctness | Old Phase 10 remains useful, but it should test through the provider collection/auth seam, not just isolated constructors. |
| 13 | Session Tree and Context Reconstruction | Old Phase 11 should depend on a generic harness/session facade. |
| 14 | TUI Product Polish | Old Phase 12 should focus on built-in terminal product polish and explicitly exclude custom extension UI parity. |
| Future | Ecosystem Candidates | OAuth, broad provider catalog, image generation, custom extension UI, npm/gallery/update, web/share, provider hooks, and pi session import enter only after clear prerequisites. |

## Current Remediation Priorities

| Priority | Area | Status | Next move |
|---|---|---|---|
| P0 | Baseline truth | Current docs describe the `0.7.0` workspace and use `.repo/pi-0.80.2` as the studied upstream baseline. | Keep this matrix and `opi-spec` synchronized; preserve embedded evidence anchors. |
| P1 | `Models/Auth` | Registry/profile/provider construction exists, but `opi-ai` does not yet own a full collection/auth runtime. | Phase 10 design/implementation before Phase 12 provider correctness. |
| P1 | Generic harness | `CodingHarness` owns too much generic orchestration. | Move or define generic phase/snapshot/save-point/session facade semantics in `opi-agent`. |
| P1 | Session facade | Sessions work, but richer context should not be added through ad hoc CLI-only writes. | Phase 10 seam first, Phase 13 entries second. |
| P2 | Tooling quality | Built-in tools are functional. | Phase 11 hardens normalization, diagnostics, cancellation, truncation, and policy. |
| P2 | TUI polish | Product TUI is usable. | Phase 14 improves built-in workflows without promising custom extension UI. |

## Future Ecosystem Candidates

| Candidate | Current matrix level | Entry condition |
|---|---|---|
| Provider OAuth / subscription auth | Missing | `Models/Auth` stable; credential store, login UX, refresh, redaction, doctor, and revocation designed. |
| Broad provider catalog | Partial | Phase 12 provider correctness stable; compatibility-profile quirks documented. |
| Image generation | Missing | Chat provider collection/auth/model metadata/cost/error semantics stable. |
| Custom extension UI / message renderer | Missing | Phase 14 built-in TUI stable; UI/RPC subprotocol designed. |
| npm/gallery/update/enable/disable | Missing | Package adapter lifecycle, trust/source model, diagnostics, and lock/update policy stable. |
| Web/share/session publishing | Missing | Phase 13 export, redaction, and session sensitivity rules stable. |
| Provider request/response adapter hooks | Missing | Provider seam, hook ordering, redaction, and trace semantics stable. |
| `pi` session import/migration | Missing | `opi` session v2 stable and user value clear. |

## Maintenance Rules

- Update this matrix whenever a phase completes or the studied upstream baseline changes.
- Keep status conservative. Do not mark `Full` unless there is a working user path or library path plus tests.
- Distinguish semantic alignment from TypeScript API/file compatibility.
- Prefer Rust-native crate ownership over copying `pi` package internals.
- Preserve evidence anchors for the studied upstream copy. If line numbers
  shift in a future `pi` snapshot, add new anchors rather than deleting useful
  old rationale.
- Do not use future ecosystem candidates as current product claims.
- Keep English and Chinese versions synchronized.
