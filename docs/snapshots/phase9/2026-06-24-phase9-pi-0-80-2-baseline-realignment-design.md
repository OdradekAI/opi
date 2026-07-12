# Phase 9 pi 0.80.2 Baseline Realignment Design

## Overview

Phase 9 realigns `opi` with `.repo/pi-0.80.2` before additional feature work.
At the start of this realignment, the baseline documents still compared
against `.repo/pi-0.75.3`, while `pi` had since shifted important architecture
around provider collections, provider-owned auth, `AgentHarness`, session
durability, extension UI surfaces, and TUI polish.

This phase is a documentation, evidence, and roadmap gate. It does not migrate
code or change runtime behavior. The purpose is to make the next development
phases traceable to current `pi` design evidence, while keeping `opi` Rust
native and focused on deepening existing capability before ecosystem expansion.

## Goals

- Establish `.repo/pi-0.80.2` as the current studied upstream baseline.
- Maintain `docs/pi-alignment-matrix.md` as the durable evidence and
  architecture reference.
- Analyze `pi` 0.80.2 architecture, recent version evolution, and product
  direction.
- Add a three-layer `opi` alignment dashboard:
  - core semantic parity;
  - product parity;
  - ecosystem parity.
- Rework the roadmap into Phase 9 through Phase 14:
  - Phase 9: baseline realignment;
  - Phase 10: core architecture deepening;
  - Phase 11: tooling quality;
  - Phase 12: provider correctness;
  - Phase 13: session tree and context reconstruction;
  - Phase 14: TUI product polish.
- Move ecosystem breadth into explicit future candidates with entry conditions.
- Preserve evidence paths and line anchors so future planning can audit why a
  phase exists.

## Non-Goals

- No code migration.
- No `CodingHarness` relocation.
- No `Models/Auth` implementation.
- No provider OAuth or subscription login.
- No image generation.
- No custom extension UI protocol.
- No npm package registry, gallery, update, enable, or disable feature.
- No browser/web UI product.
- No session sharing service.
- No `pi` session file compatibility promise.
- No shared `opi-types` crate.

## Problem Statement

Before Phase 9, the `opi` documents were internally coherent for the older
baseline, but under-specified against `.repo/pi-0.80.2`.

Important examples:

- `docs/opi-spec.md` listed upstream studied as `.repo/pi-0.75.3`.
- `docs/pi-alignment-matrix.md` said it compared against
  `.repo/pi-0.75.3`.
- The current phase plan jumps from runtime stabilization into tool/provider
  hardening without first handling the upstream `Models/Auth` and
  `AgentHarness` shifts.
- `opi-agent` is still described as fully aligned with `pi-agent-core` in the
  matrix, even though `pi-agent-core` now exports harness/session/repo/resource
  primitives that `opi-agent` does not yet own.

The immediate risk is not that `opi` has copied TypeScript architecture. The
larger risk is continuing to deepen tools, providers, sessions, and TUI on an
outdated baseline, causing future rework around the wrong seams.

## Design Principles

| Principle | Phase 9 interpretation |
|---|---|
| Current `pi` is the product reference | Study `.repo/pi-0.80.2`, not stale 0.75.3 documents. |
| Semantic alignment beats API parity | Preserve behavior and product direction, not TypeScript APIs, npm ABI, config files, or session files. |
| Rust ownership stays authoritative | Use Rust-native crates, traits, enums, error types, and storage boundaries. |
| Deep before broad | Stabilize existing runtime/provider/session/TUI seams before OAuth, custom UI, package gallery, or provider catalog breadth. |
| Evidence stays inspectable | Every major roadmap shift must cite local `pi` evidence and its effect on `opi`. |
| Ecosystem is gated | Future ecosystem candidates need entry conditions, not optimistic phase promises. |

## Evidence Anchors

The embedded evidence baseline in the alignment matrix should preserve at least
these anchors.

| Area | Source | Evidence | Implication for `opi` |
|---|---|---|---|
| Prior stale baseline | `docs/superpowers/specs/2026-06-24-phase9-pi-0-80-2-baseline-realignment-design.md:59-61` | Pre-realignment docs named `.repo/pi-0.75.3` as the studied upstream. | Phase 9 updates the baseline to `.repo/pi-0.80.2`. |
| Existing Rust ownership rule | `docs/opi-spec.md:342-345` | Generic harness primitives belong in `opi-agent`; coding-specific behavior belongs in `opi-coding-agent`. | This rule already supports moving generic harness/session orchestration out of the binary crate later. |
| No shared type hub | `docs/opi-spec.md:248,1462` | ADR-003 keeps types with semantic owners. | Phase 10 must not introduce `opi-types`. |
| `pi-ai` Models runtime | `.repo/pi-0.80.2/packages/ai/CHANGELOG.md:81` | `createModels()` owns provider collection, sync model reads, refresh, auth resolution, and stream/complete methods. | Phase 10 should design an `opi-ai` provider collection/auth seam before provider correctness work. |
| Provider auth substrate | `.repo/pi-0.80.2/packages/ai/CHANGELOG.md:82` | Provider auth includes api keys, OAuth, credential store, env auth, lazy OAuth, and auth context. | `opi` should separate provider-owned auth seams from CLI env parsing, while deferring OAuth implementation. |
| OAuth as ecosystem breadth | `.repo/pi-0.80.2/packages/ai/CHANGELOG.md:84` | Anthropic, OpenAI Codex, and GitHub Copilot have OAuth adapters. | OAuth is aligned with `pi`, but should wait until `Models/Auth` is stable. |
| Image generation | `.repo/pi-0.80.2/packages/ai/CHANGELOG.md:86` | Image generation mirrors chat-side provider collection/auth. | Image generation is future ecosystem scope, not Phase 10 correctness. |
| Harness ownership in `pi-agent-core` | `.repo/pi-0.80.2/packages/agent/src/index.ts:5,28-40` | `pi-agent-core` exports `AgentHarness`, harness messages, prompt templates, session repos, skills, system prompt, and harness types. | The current `opi-agent` alignment level should drop from Full to Partial until a Rust-native generic harness seam exists. |
| Harness purpose | `.repo/pi-0.80.2/packages/agent/docs/agent-harness.md:3` | `AgentHarness` owns session persistence, runtime configuration, resource resolution, operation locking, and extension-facing mutation semantics. | `CodingHarness` should become a product wrapper over a generic harness, not the only orchestration home. |
| Harness state and snapshots | `.repo/pi-0.80.2/packages/agent/docs/agent-harness.md:34,58-60,140-150` | Harness separates config, turn snapshot, session, and pending writes; save points refresh future turn state without mutating in-flight provider requests. | Phase 10 needs explicit turn snapshot/save-point semantics before Phase 13 session work. |
| Pending session writes | `.repo/pi-0.80.2/packages/agent/docs/agent-harness.md:84-88,176-196` | Busy session writes are queued, flushed deterministically, and should eventually go through a harness-scoped session facade. | Phase 13 session v2 must depend on a stable facade, not ad hoc session writes. |
| Semi-durable harness | `.repo/pi-0.80.2/packages/agent/docs/durable-harness.md:19,26,42,118` | Session is the durable state tree; runtime dependencies are recreated by the host on resume; recovery reduces session entries. | Phase 13 should focus on session-owned durable state, not hidden global memory. |
| Extension UI breadth | `.repo/pi-0.80.2/packages/coding-agent/docs/extensions.md:12-13,2172-2403,2524` | `pi` extensions support UI prompts, custom components, widgets, custom editors, and message renderers. | `opi` should not claim extension UI parity in Phase 14; this belongs in future ecosystem candidates. |
| Provider hooks in extensions | `.repo/pi-0.80.2/packages/coding-agent/docs/extensions.md:297,632-639,2630` | Extensions can inspect or replace provider payloads and observe provider responses. | Provider request/response hooks are future ecosystem work after core provider seams stabilize. |
| Session hooks in extensions | `.repo/pi-0.80.2/packages/coding-agent/docs/extensions.md:326,434-439` | Extensions can customize/cancel session compaction/tree flows. | Phase 13 should preserve a future route for session lifecycle hooks, but not copy TS extension API. |

## Evidence Baseline in the Alignment Matrix

Maintain:

```text
docs/pi-alignment-matrix.md
```

The matrix should be both the durable evidence baseline and the operational
alignment view, not a temporary audit note.

Required sections:

1. **Document Control**
   - upstream path: `.repo/pi-0.80.2`;
   - upstream package version: `0.80.2`;
   - `opi` workspace version;
   - date sampled;
   - update policy.

2. **Executive Summary**
   - concise statement of where `opi` is close, where it is intentionally
     narrower, and where current architecture should deepen before feature
     breadth.

3. **Pi Architecture**
   - `@earendil-works/pi-ai`: Models collection, providers, API
     implementations, auth, image generation, compatibility entrypoint;
   - `@earendil-works/pi-agent-core`: agent loop, `Agent`, `AgentHarness`,
     session repo/storage, compaction, resource/system-prompt helpers,
     execution environment;
   - `@earendil-works/pi-tui`: custom terminal renderer, components,
     overlays, autocomplete, terminal image/color support;
   - `@earendil-works/pi-coding-agent`: CLI/TUI product, tools, sessions,
     extensions, package management, RPC/SDK, export/share/update surfaces.

4. **Version Evolution Signals**
   - changes since the prior 0.75.3 baseline that materially affect `opi`;
   - especially `Models/Auth`, `AgentHarness`, session durability, extension
     UI/provider hooks, and TUI polishing.

5. **Evidence Index**
   - table with source path, line anchor, evidence summary, affected `opi`
     crate, and roadmap implication.

6. **Opi Alignment Dashboard**
   - three-layer progress view:

| Layer | Meaning | Expected current state after Phase 9 |
|---|---|---|
| Core semantic parity | Agent loop, event order, provider stream, tool scheduling, session tree concepts. | High but not complete; `AgentHarness` lowers `opi-agent` from Full to Partial. |
| Product parity | CLI/TUI/tools/sessions/packages/RPC/docs/diagnostics users can exercise. | Medium; product depth remains behind `pi`, but direction is aligned. |
| Ecosystem parity | OAuth, broad provider catalog, npm/gallery, custom extension UI, web/share, image generation. | Low by design; future candidates only. |

7. **Roadmap Implications**
   - why Phase 9 and Phase 10 are inserted;
   - why previous Phase 9-12 move to Phase 11-14;
   - why ecosystem work is deferred.

8. **Maintenance Rules**
   - update the document when changing upstream baseline version;
   - preserve old evidence instead of rewriting history when useful;
   - never mark parity as Full without working `opi` path plus tests;
   - update English/Chinese normative counterparts when they reference the
     baseline.

## Normative Documentation Changes

Phase 9 should update these documents.

| File | Required change |
|---|---|
| `docs/opi-spec.md` | Update current upstream baseline to `.repo/pi-0.80.2`; add Phase 9-14 roadmap; strengthen generic harness ownership guidance; link the alignment matrix as the durable evidence baseline. |
| `docs/opi-spec.zh.md` | Keep in lockstep with English changes. |
| `docs/pi-alignment-matrix.md` | Rebase comparison to `.repo/pi-0.80.2`; add three-layer alignment dashboard; update package and phase rows; mark `opi-agent` as Partial until generic harness gaps close. |
| `docs/pi-alignment-matrix.zh.md` | Keep in lockstep with English changes. |
| `docs/superpowers/specs/2026-06-24-phase10-core-architecture-deepening-design.md` | New Phase 10 design document. |
| `docs/superpowers/specs/2026-06-24-phase11-tooling-quality-design.md` | Recast from the old Phase 9 and state dependency on Phase 10. |
| `docs/superpowers/specs/2026-06-24-phase12-provider-correctness-design.md` | Recast from the old Phase 10 and state dependency on the `Models/Auth` seam. |
| `docs/superpowers/specs/2026-06-24-phase13-session-tree-context-reconstruction-design.md` | Recast from the old Phase 11 and state dependency on generic harness/session facade. |
| `docs/superpowers/specs/2026-06-24-phase14-tui-product-polish-design.md` | Recast from the old Phase 12 and explicitly exclude custom extension UI parity. |

## Revised Roadmap

| Phase | Name | Purpose | Exit posture |
|---:|---|---|---|
| 9 | pi 0.80.2 Baseline Realignment | Evidence, architecture analysis, alignment dashboard, roadmap gate. | Documents truthfully explain current baseline and next phases. |
| 10 | Core Architecture Deepening | Design and implement `Models/Auth`, generic `AgentHarness`, session repo/facade, and runtime hook boundaries. | Existing capabilities sit on deeper Rust-native seams. |
| 11 | Tooling Quality | Normalize and harden built-in tools. | Tool behavior is predictable across paths, encodings, cancellation, truncation, and diagnostics. |
| 12 | Provider Correctness | Harden existing provider families and compatibility profiles. | Providers have fixture-backed lifecycle, error, auth, image-input, thinking, usage, retry, and compat behavior. |
| 13 | Session Tree and Context Reconstruction | Add session v2 entries where needed, branch summaries, labels, model/thinking history, export, and recovery. | Session-native context is durable, bounded, auditable, and exportable. |
| 14 | TUI Product Polish | Improve built-in terminal workflows. | Model/session/branch UX, transcript rendering, status, accessibility, and command discovery are polished. |
| Future | Ecosystem Candidates | OAuth, broad provider catalog, image generation, custom extension UI, npm/gallery/update, web/share. | Enter only when listed prerequisites are met. |

## Phase 9 Success Criteria

Phase 9 is complete when:

1. `docs/pi-alignment-matrix.md` includes architecture analysis, evidence
   anchors, and the three-layer alignment dashboard.
2. `docs/opi-spec*.md` and `docs/pi-alignment-matrix*.md` no longer describe
   `.repo/pi-0.75.3` as the current studied upstream baseline.
3. English and Chinese normative docs are synchronized.
4. Phase 9-14 roadmap is documented consistently.
5. `opi-agent` package alignment reflects the generic harness gap honestly.
6. `Models/Auth` and `AgentHarness` are named as Phase 10 deepening targets.
7. Future ecosystem candidates are documented with entry conditions.
8. Documentation guards reject overclaims for OAuth parity, custom UI parity,
   npm/gallery, image generation, web/share, and `pi` session compatibility.
9. No runtime behavior changes occur.

## Future Ecosystem Candidates

These are aligned with `pi` but not committed near-term phases.

| Candidate | Entry condition |
|---|---|
| Provider OAuth / subscription auth | `Models/Auth` seam is stable; credential store, redaction, doctor, and session interaction are designed. |
| Broad provider catalog | Phase 12 provider correctness is stable; compatibility profiles cover most OpenAI-compatible quirks. |
| Image generation | Chat-side provider collection, auth, model metadata, cost, and error semantics are stable. |
| Custom extension UI / message renderer | Phase 14 built-in TUI is stable; a separate RPC/UI subprotocol is designed. |
| npm/package gallery/update/enable/disable | Package adapter lifecycle, trust model, diagnostics, lock/source model, and local/git flow are stable. |
| Web/share/session publishing | Phase 13 export, redaction, and session sensitivity rules are stable; privacy design is explicit. |
| Provider request/response adapter hooks | Core provider seam is stable; hook ordering, redaction, and trace semantics are clear. |
| `pi` session import/migration | `opi` session v2 is stable; user value is clear; normal resume remains unaffected. |

## Testing and Guard Strategy

Phase 9 is documentation-heavy, so verification should focus on consistency and
guard tests.

| Test area | Coverage |
|---|---|
| docs guard | Current baseline says `.repo/pi-0.80.2`; stale `.repo/pi-0.75.3` appears only in historical notes. |
| docs guard | Phase numbering is consistent across spec, alignment matrix, and phase design docs. |
| docs guard | Forbidden current-scope claims are absent: OAuth parity, image generation, custom extension UI parity, npm/gallery, web/share, `pi` session compatibility. |
| docs guard | Positive claims are present: Phase 9 evidence baseline, Phase 10 architecture deepening, future ecosystem candidates. |
| localization | English and Chinese normative docs carry equivalent baseline and roadmap statements. |

No live provider, package adapter, or runtime tests are required because Phase 9
does not change behavior.

## Implementation Notes

- Prefer editing current-state statements over rewriting historical release
  rows.
- Keep old phase specs as files but update their visible phase numbering and
  dependency notes.
- Do not delete historical audits or snapshots.
- Use source paths and line anchors in the evidence baseline sections, but keep
  long explanations in summaries rather than copying large upstream text.
- Keep ecosystem wording non-committal. Candidate does not mean scheduled.
- Do not add an `opi-types` crate to solve documentation organization.
