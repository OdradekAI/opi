# Phase 13 Session Tree and Context Reconstruction Design

Historical note: this design was originally drafted as Phase 11. After the
`.repo/pi-0.80.2` baseline review, Phase 11 became tooling quality and Phase 12
became provider correctness. Session tree and context reconstruction is now
Phase 13 and depends on the Phase 10 generic harness/session facade seam.

## Overview

Phase 13 deepens opi's session system and long-running workflow support. The
scope is session-native context: branch trees, labels, names, summaries, model
and thinking changes, exports, recovery, and deterministic context
reconstruction. It is not a global memory system, vector database, RAG layer,
or user-profile store.

The goal is to make opi better at preserving and navigating development work
over time while staying faithful to pi's terminal-first, append-only session
model.

Post-Phase-12 drift review refinement: Phase 13 is primarily a core session
semantics phase, not a CLI convenience phase. Richer entries and context
reconstruction must be owned by `opi-agent` through the session facade/context
builder seam from Phase 10. `opi-coding-agent` may expose commands, exports,
and TUI/RPC feedback, but it must not implement the new semantics through
ad-hoc product-only JSONL scans or one-off CLI writes.

## Goals

- Introduce an opi session v2 design where needed, preserving safe migration
  from existing opi v1 sessions.
- Make `opi-agent` the semantic owner of new session entries, ordered session
  reads, active-branch reconstruction, and context-building rules.
- Add richer session entries for model changes, thinking level changes,
  session info, labels, and branch summaries when they are product-supported.
- Improve tree reconstruction, branch navigation metadata, and context
  building.
- Add export formats for local review and handoff.
- Make compaction and branch summaries more explicit and auditable.
- Keep persisted context bounded to session files and explicit exports.

## Non-Goals

- No vector database.
- No semantic memory service.
- No global user profile memory.
- No automatic cross-project memory injection.
- No pi session v3 read/write compatibility.
- No cloud sync.
- No session sharing service.
- No web UI product.
- No package ecosystem expansion.
- No provider runtime, provider auth, OAuth, or `ProviderCollection` dispatch
  refactor. Phase 13 may record provider/model identifiers as session data,
  but provider routing remains outside this phase.

## Relationship to pi

Pi session v3 includes tree entries for messages, model changes, thinking level
changes, compaction, branch summaries, extension custom entries, custom
messages, labels, and session info. Opi should learn from that shape but should
not promise pi file compatibility.

Phase 13 should define opi's own session v2 only if new entries cannot be added
cleanly to the existing v1 format. Compatibility means opi can load its own
older sessions and explain migration, not that opi can read arbitrary pi
sessions.

The current opi session format already records messages, compaction entries,
leaf pointers, and extension state. It does not yet make pi-inspired branch
summaries, extension custom messages, labels, or session info first-class in
context reconstruction. Phase 13 closes that semantic gap without claiming pi
session v3 file compatibility.

The alignment target is pi's append-only tree/context model, not pi's exact
TypeScript storage API. Rust crate ownership should remain explicit: core
session semantics belong in `opi-agent`; product commands, storage location,
and user presentation belong in `opi-coding-agent`.

## Session-Native Context Boundary

Allowed:

- session names;
- labels/bookmarks;
- branch summaries;
- compaction summaries;
- explicit export;
- model/thinking history;
- extension state that is scoped to a session;
- local metadata needed to reconstruct context.

Not allowed:

- automatic retrieval from unrelated sessions;
- global facts about the user;
- background embedding/indexing;
- hidden prompt injection from old sessions;
- remote context sync.

## Implementation Priority and Crate Boundaries

| Priority | Scope | Owner | Requirement |
|---|---|---|---|
| P0 | Session entry types and compatibility rules | `opi-agent` | Define durable typed entries, migration/read behavior, and unknown-entry policy before product commands depend on them. |
| P0 | Active branch and LLM context reconstruction | `opi-agent` | Provide a deterministic context builder that walks the active branch, applies compaction/branch summaries, and returns agent-runtime messages. |
| P0 | Production append/load path | `opi-agent`, `opi-coding-agent` | Product commands must append through the session facade or an equivalent ordered writer, not by hand-editing raw JSONL shapes. |
| P1 | Session metadata commands and local export | `opi-coding-agent` | Commands and export renderers consume typed session reads/context results and apply Phase 7 redaction rules. |
| P1 | TUI/RPC handoff metadata | `opi-coding-agent` | Expose stable session/tree metadata for Phase 14 without redesigning the TUI in Phase 13. |
| P2 | Extension custom context messages | `opi-agent`, `opi-coding-agent` | Implement only if provider-context and transcript semantics are clear; otherwise defer explicitly. |

Phase 13 must not close an acceptance scenario by adding helper structs only.
Each implemented entry needs at least one exercised production path that writes
or reads it, plus tests for deterministic reconstruction where the entry affects
LLM context.

## Session Entry Model

Define or explicitly defer entries for:

| Entry | Purpose | Phase 13 priority |
|---|---|---|
| `session_info` | Store user-visible name and optional metadata | P0 if `/name` or session rename is product-supported |
| `model_change` | Record provider/model change on the active branch | P0 for resumed run correctness |
| `thinking_level_change` | Record reasoning/thinking level change | P0 for resumed run correctness |
| `label` | Bookmark or label an entry | P1; UI-visible, not LLM context |
| `branch_summary` | Preserve context when leaving or forking a branch | P0 semantics; P1 generation UX |
| `custom_message` | Extension-injected LLM-context message with display semantics | P2 or explicit defer |

Existing entries for messages, compaction, leaf pointers, and extension state
remain. Any new entry must have clear context-building semantics and tests.
Entries that participate in LLM context, especially `branch_summary` and
`custom_message`, must have provider-conversion tests or be explicitly deferred
with a product reason.

## Context Building

Context reconstruction should be deterministic:

```text
session file
  -> recover valid entries
  -> find active leaf
  -> walk branch to root
  -> apply model/thinking/session metadata
  -> apply compaction and branch summaries
  -> produce app messages for agent runtime
```

The context builder should live in `opi-agent`. `opi-coding-agent` should call
it when resuming, exporting, or presenting session context instead of
duplicating branch-walk logic.

Rules should define:

- how multiple `leaf` entries resolve;
- how corrupt trailing lines are handled;
- how labels affect UI but not LLM context;
- how branch summaries enter LLM context;
- how custom messages enter LLM context and transcript rendering;
- how model/thinking changes affect resumed runs;
- how extension state is restored.

## Branch Summaries

Branch summaries should be explicit and optional. They may be generated:

- when switching branches;
- when forking or cloning;
- manually by command;
- by an extension hook if the runtime supports it.

Phase 13 should not require live provider calls for every branch operation.
If a summary cannot be generated, the branch action should still work and
record a diagnostic.

## Export

Add local export support for:

| Format | Purpose |
|---|---|
| markdown | readable review and handoff |
| json | structured local tooling |
| html | optional static transcript if low-cost and aligned with existing rendering |

Exports are local files. No sharing service is part of Phase 13.

Export should support:

- active branch only;
- full tree;
- include/exclude tool output;
- include/exclude thinking content;
- redaction options using Phase 7 rules.

Export renderers are product-owned. They should consume typed session reads or
context-builder output, not raw ad-hoc scans that duplicate session semantics.

## Commands and UI Surface

Candidate commands:

```text
/name <name>
/label <label>
/unlabel
/export [path]
/session
```

CLI candidates:

```text
opi --list-sessions --json
opi --export-session <id-or-path> --format markdown --output <file>
```

Only implement commands that have clear tests and do not require a broad TUI
redesign. Phase 14 can polish interactive presentation.

## Data Flow

```text
interactive/session command
  -> SessionCoordinator
  -> SessionFacade or ordered session writer
  -> append typed session entry
  -> update active leaf where applicable
  -> emit AgentSessionEvent / diagnostics
  -> TUI or JSON/RPC presentation
```

```text
export command
  -> SessionReader
  -> branch/tree selection
  -> redaction policy
  -> renderer
  -> local file
```

## Error Handling

Session operations should prefer recoverability:

- corrupt final lines are recovered when possible;
- unknown future entries are preserved or skipped according to documented
  compatibility rules;
- failed metadata writes produce errors before claiming success;
- failed branch summary generation records diagnostics but should not destroy
  branch navigation;
- export failures must not modify the session.

## Testing Strategy

| Level | Coverage |
|---|---|
| session storage | new entry round trips, migration, unknown entry behavior |
| context building | model/thinking changes, compaction, branch summaries |
| branch tree | labels, session names, active leaf resolution |
| production path | at least one command/resume/export path exercises each implemented entry class |
| CLI | list/export/session metadata commands |
| TUI snapshot | branch picker metadata if touched |
| redaction | export redaction for prompts, tool output, secrets |

All tests must use isolated temp session directories or `OPI_SESSIONS_DIR`.

## Documentation Updates

Update docs to state:

- opi session format version and compatibility policy;
- opi does not promise pi session v3 compatibility;
- session files are sensitive;
- session-native context is explicit and bounded to session files/exports;
- export is local and user-controlled.

## Success Criteria

Phase 13 is complete when:

1. New session metadata and context entries, if added, are defined in
   `opi-agent` and reached through a production append/read path.
2. Context reconstruction is deterministic, tested, and owned by a reusable
   `opi-agent` session/context API rather than product-only CLI scans.
3. Existing opi v1 sessions continue to load, with documented compatibility
   behavior for unknown or future entries.
4. Branch, label, name, model, thinking, compaction, and summary semantics are
   documented where implemented.
5. `branch_summary` and `custom_message` are either implemented with
   provider/context semantics and provider-conversion tests or explicitly
   deferred with reasons.
6. Local export supports at least markdown or JSON with redaction options and
   does not modify the source session.
7. TUI/RPC handoff metadata is stable enough for Phase 14 without requiring
   Phase 13 to redesign terminal rendering.
8. Session files are documented as sensitive.
9. No vector memory, global profile, cloud sync, session sharing service,
   provider auth refactor, web UI, or pi session compatibility claim is added.

## Phase 14 Handoff

Phase 14 should improve the terminal presentation of the session model:
pickers, tree views, command palette, keyboard flow, and transcript rendering.
It should not change session semantics without updating this design.
