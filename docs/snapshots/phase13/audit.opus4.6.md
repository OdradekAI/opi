# Phase 13 Session Tree and Context Reconstruction -- Independent Code Audit

**Auditor**: Opus 4.6 (independent, no prior audit reports consulted)
**Date**: 2026-07-06
**Scope**: Tasks 13.1--13.7, commits `b98c6110..93dd1bf1`
**Method**: Full source read of ~25 files, 5 parallel deep-audit passes
(entry model, context reconstruction, cross-task integration, test quality,
spec compliance)

---

## 1. Executive Summary

**Verdict: PASS-WITH-FINDINGS**

| Severity | Count |
|----------|-------|
| Blocker  | 0     |
| Major    | 5     |
| Minor    | 11    |
| Info     | 15    |

Phase 13 delivers a sound session-native context reconstruction system. The
typed entry model is clean, serde round-trips are correct, the
`reconstruct_context` algorithm is deterministic, and the v1 additive-entry
compatibility strategy works as designed.

Five major findings require attention before Phase 14. The most impactful is the
`select_ordered_entries` vs `SessionTree` divergence on corrupt Leaf targets
(M1), which can cause coordinator/agent buffer misalignment. The pre-first-turn
metadata inconsistency (M2) affects observable behavior across resume, list, and
export paths. Three major test gaps (M3--M5) leave important edge cases
unverified.

No data loss, security breach, or correctness blocker was found.

### Per-task summary

| Task | Title | Verdict |
|------|-------|---------|
| 13.1 | Session v2 entry model | PASS (1 major doc bug, 2 minor) |
| 13.2 | Reusable context reconstruction API | PASS (1 major integration divergence) |
| 13.3 | Resume, fork, and branch reconstruction | PASS-WITH-FINDINGS (M1, M2, m3) |
| 13.4 | Session metadata commands | PASS (1 minor UX gap) |
| 13.5 | Local session export with redaction | PASS (1 minor redaction gap) |
| 13.6 | TUI and RPC session handoff metadata | PASS (2 minor) |
| 13.7 | Session docs, non-goal guards | PASS (1 minor doc drift) |

---

## 2. Session Entry Model Correctness (13.1)

### 2.1 Typed entry definitions -- PASS

All five Phase 13 entry structs (`SessionInfoEntry`, `ModelChangeEntry`,
`ThinkingLevelChangeEntry`, `LabelEntry`, `BranchSummaryEntry`) are correctly
defined in `crates/opi-agent/src/session.rs` with `id`, `parent_id`,
`timestamp`, and type-specific fields. The `SessionEntry` enum uses
`#[serde(tag = "type", rename_all = "snake_case")]` and `#[non_exhaustive]`,
enabling forward-compatible deserialization.

`entry_id()` and `parent_id()` use exhaustive match arms covering all 9
variants. The `#[non_exhaustive]` attribute ensures the compiler enforces
updates when new variants are added.

### 2.2 CrashRecovery three-bucket classification -- PASS

The separation of `truncated_line`, `corrupt_count`, and `unknown_count` in
`CrashRecovery` (L260-265) is clean and well-tested:

- Valid JSON + unknown type tag -> `unknown_count`
- Valid JSON + known type + malformed payload -> `corrupt_count`
- Invalid JSON -> `corrupt_count`
- JSON without `type` field -> `corrupt_count`
- Truncated last line (no trailing newline) -> `truncated_line: true`

### 2.3 FORMAT_VERSION and SESSION_FORMAT_POLICY -- PASS

`FORMAT_VERSION = 1` (L53) and `SESSION_FORMAT_POLICY` (L58-61) correctly
describe the v1-additive strategy. Tests pin both values.

### 2.4 `read_with_recovery` line parsing -- PASS

Uses `content.lines()` which handles both LF and CRLF correctly. The two-stage
parse (JSON Value then SessionEntry) enables the unknown-vs-corrupt distinction.
Version mismatch produces a hard `InvalidData` error.

### Findings

**M3** `read_all` doc comment claims "strict mode" but silently discards
recovery

- **File**: `crates/opi-agent/src/session.rs`, L405
- **Severity**: Major
- **Description**: The doc comment reads `"strict mode -- errors on corrupt
  data"`, but the implementation merely discards the `CrashRecovery` struct
  without inspecting it. Corrupt and unknown entries are silently dropped.
- **Impact**: Callers relying on `read_all` may assume corrupt data triggers an
  error, leading to false confidence. `rpc.rs:835` uses `read_all` for the
  session tree branch, meaning corrupt entries are silently lost without any
  diagnostic.
- **Fix**: Either change the doc to `"convenience wrapper -- discards recovery
  metadata"`, or make `read_all` return `Err` when `corrupt_count > 0`.

**m1** `KNOWN_ENTRY_TYPES` lacks compile-time sync guarantee

- **File**: `crates/opi-agent/src/session.rs`, L516-526
- **Severity**: Minor
- **Description**: The `KNOWN_ENTRY_TYPES` constant is a hand-maintained
  `&[&str]` array. No compile-time mechanism enforces it stays in sync with the
  `SessionEntry` enum. If a future variant is added to `SessionEntry` but not
  to `KNOWN_ENTRY_TYPES`, malformed instances of that type would be
  misclassified as "unknown" instead of "corrupt."
- **Impact**: Classification error only (unknown vs corrupt bucket); no data
  loss. Existing round-trip tests would catch the new variant's tag correctness
  but not the `KNOWN_ENTRY_TYPES` omission.
- **Fix**: Add a unit test that serializes a minimal instance of each
  `SessionEntry` variant and asserts `value["type"]` is in
  `KNOWN_ENTRY_TYPES`.

**m2** `SessionWriter` doc says "crash-safe flush" but is crash-recoverable

- **File**: `crates/opi-agent/src/session.rs`, L327
- **Severity**: Minor
- **Description**: The module doc reads `"Append-only JSONL writer with
  crash-safe flush"`. The actual mechanism is `writeln!` + `sync_all()`, which
  is not atomic (a partial write can leave an incomplete line). Recovery depends
  on `SessionWriter::open()` detecting and truncating the incomplete tail.
- **Impact**: Terminology mismatch. "Crash-safe" implies atomic-rename-level
  guarantees; "crash-recoverable" is the accurate term.
- **Fix**: Change doc to `"crash-recoverable flush"`.

**i1** Bare CR line ending (Info)

- **File**: `crates/opi-agent/src/session.rs`, L424
- **Severity**: Info
- **Description**: `!content.ends_with('\n') && !content.ends_with('\r')`
  means a file ending with bare `\r` (legacy Mac OS 9) is treated as complete.
  In practice this never occurs on modern systems; the incomplete line would
  fall into the corrupt bucket.

**i2** SessionWriter::open truncates to 0 when no newline exists (Info)

- **File**: `crates/opi-agent/src/session.rs`, L385
- **Severity**: Info
- **Description**: If the entire file has no newline (header write interrupted),
  `open()` truncates to 0. This is documented behavior and tested by
  `writer_truncates_all_when_no_newline_in_file`.

---

## 3. Context Reconstruction Correctness (13.2)

### 3.1 `reconstruct_context` algorithm -- PASS

The algorithm in `crates/opi-agent/src/session_context.rs` (L110-253) is
correct:

1. Builds `entries_by_id` HashMap indexing all entries (L123-124).
2. Constructs `SessionTree` to find `active_tip` (L129-130).
3. Scans all entries for missing-parent diagnostics (L136-145).
4. Walks parent chain from active tip to root with cycle guard (L150-171).
5. Reverses chain to root->tip order.
6. Builds messages, applying compaction and BranchSummary injection (L200-237).
7. Collects metadata via `collect_metadata` (L242).

### 3.2 Determinism -- CONFIRMED SOUND

No HashMap or HashSet iteration order leaks into the output. All ordering
derives from:
- Input slice order (file order) for metadata, labels, BranchSummary grouping
- Chain walk order (tip->root then reverse) for messages
- Stable Vec push/insert operations

Test `reconstruction_is_deterministic_across_calls` verifies.

### 3.3 Metadata exclusion from LLM context -- CONFIRMED SOUND

`SessionInfo`, `ModelChange`, `ThinkingLevelChange`, `Label`,
`ExtensionState`, and `Leaf` all fall into the no-op match arm (L214-221) and
never produce `AgentMessage` entries. `BranchSummary` is intentionally injected
into LLM context through the `summaries_by_parent` mechanism (L223-234),
producing `AgentMessage::BranchSummary` -- this is the documented Phase 13
design decision (module doc L33-36).

### 3.4 Compaction logic -- PASS

`apply_compaction` (L261-277) correctly:
- Retains entries at `chain_index >= kept_idx`
- Handles missing `first_kept_entry_id` (no truncation, summary still emitted)
- Handles nested compactions (later compaction drops earlier summary)
- `insert(0, ...)` is O(n) but bounded by session size; functionally correct

### 3.5 SessionTree (session_branch.rs) -- PASS

- `from_entries` only indexes Message/Compaction in the branch graph
- `active_tip()` validates Leaf target exists, falls back to trunk tip
- Last-Leaf-wins semantics correct (tested by `branched_session_last_leaf_wins`)
- Cycle detection prevents infinite loops (tested by
  `cycle_in_parent_id_detected`)
- Branch depth calculation correct

### Findings

**M2** `collect_metadata` rejects pre-first-turn metadata (`parent_id = None`)

- **File**: `crates/opi-agent/src/session_context.rs`, L297-299
- **Severity**: Major
- **Description**: The `on_chain` closure returns `false` when `parent_id` is
  `None`:
  ```rust
  let on_chain = |parent_id: Option<&str>| -> bool {
      parent_id.and_then(|pid| chain_set.get(pid)).is_some()
  };
  ```
  Metadata entries written before the first content message (e.g., session name
  set at creation, initial model spec) have `parent_id: None` because
  `active_tip_entry_id` is `None` at that point. These entries are silently
  excluded from the reconstructed context.

  In contrast, `seed_metadata_from_entries` in `session_coordinator.rs`
  (L675-682) explicitly accepts rootless metadata when `active_ids` is empty:
  ```rust
  let on_active = |parent: &Option<String>| -> bool {
      match parent {
          None => active_ids.is_empty(),
          Some(id) => active_ids.contains(id.as_str()),
      }
  };
  ```
- **Impact**: After resume, `/session info` (coordinator path) shows pre-first-
  turn name/labels, but `--list-sessions --json`, `--export-session`, and RPC
  `session_info` (reconstruct_context path) do not. Observable metadata
  inconsistency across product surfaces.
- **Fix**: Add the rootless-metadata guard to `collect_metadata`:
  ```rust
  let on_chain = |parent_id: Option<&str>| -> bool {
      match parent_id {
          None => chain_set.is_empty(),
          Some(pid) => chain_set.contains(pid),
      }
  };
  ```

**i3** Duplicate `entry_id` collision silent (Info)

- **File**: `crates/opi-agent/src/session_context.rs`, L123-124
- **Severity**: Info
- **Description**: If two entries share an `entry_id` (corruption), the HashMap
  keeps the last occurrence. No diagnostic is emitted for the collision.

**i4** BranchSummary with `parent_id = None` silently dropped (Info)

- **File**: `crates/opi-agent/src/session_context.rs`, L186-195
- **Severity**: Info
- **Description**: BranchSummary grouping only collects entries with
  `Some(pid)`. A rootless BranchSummary is quietly ignored; no diagnostic.

**i5** Leaf advisory `parent_id` triggers generic missing-parent diagnostic
(Info)

- **File**: `crates/opi-agent/src/session_context.rs`, L136-145
- **Severity**: Info
- **Description**: A Leaf's `parent_id` is an advisory location marker. If
  stale, it triggers `CODE_SESSION_CONTEXT_MISSING_PARENT` -- the same
  diagnostic used for broken content chains. Users cannot distinguish the two
  cases from the diagnostic message alone.

---

## 4. Cross-Task Integration Correctness (13.3, 13.4, 13.6)

### 4.1 Resume/Fork paths -- PASS with findings

`resume_session_id` (harness.rs:1146-1194) correctly calls
`reconstruct_context`. ResumeInfo correctly captures `recorded_model` and
`recorded_thinking` from the reconstructed context.
`apply_recorded_model`/`apply_recorded_thinking` correctly emit Phase 7
diagnostics on incompatibility and fall back to CLI/config values.

`entry_on_active_chain` (session_cli.rs:266-304) handles all 9 `SessionEntry`
variants with explicit match arms plus a forward-compatible wildcard.

### 4.2 Metadata commands -- PASS

`parse_session_metadata_command` correctly parses `/name <name>`,
`/label <label>`, `/unlabel <label>`, and `/session info`. Unicode content
works correctly. Bare `/session` returns `None` and routes to the session
picker; no collision with `/session info`.

### 4.3 RPC/Picker handoff -- PASS with findings

RPC `session_info` (rpc.rs:780-873) builds SessionTree from the live session
file and returns entry_count, branch_summary (redacted), and branches[].
Session picker items show name and labels.

### Findings

**M1** `select_ordered_entries` vs `SessionTree` fallback divergence on
corrupt Leaf

- **File**: `crates/opi-coding-agent/src/session_cli.rs` L926-943,
  `crates/opi-coding-agent/src/session_coordinator.rs` L136,
  `crates/opi-agent/src/session_branch.rs` L173-182
- **Severity**: Major
- **Description**: When the last Leaf's `entry_id` points to a non-existent
  entry (corrupt session):
  - `select_ordered_entries` -> `walk_active_branch` returns an **empty Vec**
    because `by_id.get(tip)` returns `None` in a HashMap that only indexes
    Message/Compaction.
  - `SessionTree::active_tip()` validates the Leaf target against
    `entries_by_id.contains_key(tip)`, finds it missing, and **falls back to
    the trunk tip**.

  `SessionCoordinator::open_existing` uses `select_ordered_entries` (empty
  chain). `reconstruct_context` uses `SessionTree` (trunk chain). The two
  paths produce different active branches for the same input.
- **Impact**: After resume of a session with a corrupt final Leaf, the
  coordinator's compaction buffer (empty) is misaligned with the agent's
  message buffer (trunk messages). This can cause duplicate compaction entries
  or incorrect compaction cuts.
- **Fix**: Refactor `select_ordered_entries` to use `SessionTree::active_tip()`
  for tip resolution, or make it fall back to the trunk tip when the Leaf
  target is invalid.

**m3** Fork drops pre-first-turn metadata (`parent_id: None`)

- **File**: `crates/opi-coding-agent/src/session_cli.rs`, L266-304
- **Severity**: Minor
- **Description**: `entry_on_active_chain` uses
  `parent_id.as_deref().is_some_and(...)` for metadata entries, which rejects
  `parent_id: None`. Pre-first-turn metadata (set via `/name` or `/label`
  before the first message) is excluded from the fork.
- **Impact**: If a user names/labels a session before the first message and
  then forks, the fork loses those metadata entries. Low probability but
  violates the fork contract.
- **Fix**: Accept `parent_id: None` when `active_set` is empty (same logic as
  `seed_metadata_from_entries`).

**m4** `/name` or `/label` with empty arg falls through to LLM

- **File**: `crates/opi-coding-agent/src/interactive.rs`, L746-771
- **Severity**: Minor
- **Description**: `/name ` (trailing space, no name) and `/label ` return
  `None` from `parse_session_metadata_command`. The input falls through to the
  normal prompt handler and is sent to the LLM as a user message. No error or
  usage hint is shown.
- **Impact**: User confusion. The LLM receives a literal `/name` or `/label`
  prompt.
- **Fix**: Return a usage-hint error variant or handle empty-arg commands
  before falling through.

**m6** RPC `session_info` silently swallows `SessionReader::read_all` failure

- **File**: `crates/opi-coding-agent/src/rpc.rs`, L833-866
- **Severity**: Minor
- **Description**: The branch-tree section is guarded by
  `if let Ok((_, entries)) = SessionReader::read_all(...)`. If the read fails
  (I/O error, corrupt file), `branches`, `entry_count`, and `branch_summary`
  are omitted without any error indication. The client receives `success: true`.
- **Impact**: Clients cannot distinguish "session has no branches" from "branch
  data could not be read." Matters for Phase 14 handoff where branch tree is
  an input to picker rendering.
- **Fix**: Surface the read error as a diagnostic in the response, or add a
  `branch_tree_error` field.

**m7** Session picker items omit model/thinking metadata

- **File**: `crates/opi-coding-agent/src/picker.rs`, L83-114
- **Severity**: Minor
- **Description**: `session_picker_items` shows name, cwd, timestamp, and
  labels, but omits model and thinking level -- both available in the
  `SessionInfo` struct from `list_sessions`.
- **Impact**: Interactive users see less metadata than CLI/JSON users. Low
  severity (picker is a quick selection UI).

**m8** Branch summaries not redacted in interactive picker

- **File**: `crates/opi-coding-agent/src/picker.rs`, L57-59
- **Severity**: Minor
- **Description**: `branch_picker_item` formats `branch.summary` directly
  without redaction. RPC `session_info` (rpc.rs:843-855) applies
  `redact_text(summary, RedactionMode::Summary)`. Inconsistent redaction at
  display boundaries.
- **Impact**: Low for interactive mode (user is the session owner), but
  violates the principle of consistent redaction.

**i6** Duplicated `SessionTree` build between RPC and harness (Info)

- **File**: `crates/opi-coding-agent/src/rpc.rs` L837,
  `crates/opi-coding-agent/src/harness.rs` L1308-1311
- **Severity**: Info
- **Description**: Both re-read the session file and rebuild
  `SessionTree::from_entries`. Acceptable for correctness (each gets a fresh
  view); performance bounded by session file size.

**i7** `collect_full_tree_messages` drops non-Message entries (Info, by design)

- **File**: `crates/opi-coding-agent/src/session_cli.rs`, L446-459
- **Severity**: Info
- **Description**: Full-tree export only collects `SessionEntry::Message`,
  dropping Compaction, BranchSummary, etc. This is documented: "does not apply
  compaction or branch summaries (those are active-branch-only semantics)."

---

## 5. Export/Redaction Security (13.5)

### 5.1 Export data flow -- PASS

`export_session` (session_cli.rs:417-439) reads via
`SessionReader::read_with_recovery` (read-only), reconstructs or collects
messages, applies redaction, and writes only to `options.output`. The source
path is never opened for writing. Test
`phase13_export_invalid_output_path_fails_and_preserves_source` verifies the
source is byte-for-byte unchanged even on write failure.

### 5.2 Redaction coverage -- PASS with finding

The `redact_export_text` helper is applied to:
- User message text
- Assistant message text
- Thinking content
- Tool call arguments and names
- Tool results
- Compaction summaries
- Branch summaries
- Session header CWD

### 5.3 `--redact` defaults -- PASS

Default is `Summary`. `None` is explicitly opt-in. The CLI doc string explains
the trust boundary.

### Findings

**m5** `AgentMessage::Custom` data not redacted in export

- **File**: `crates/opi-coding-agent/src/session_cli.rs`, L563-566 (markdown),
  L687-692 (JSON)
- **Severity**: Minor
- **Description**: In the markdown renderer, custom message data is emitted via
  `c.data.to_string()` without passing through `redact_export_text`. In the
  JSON renderer, the raw `c.data` value is included. All other text fields are
  properly redacted.
- **Impact**: If an extension stores sensitive data in custom messages,
  `--export-session --redact summary` would leak it unredacted. Currently
  unreachable because `reconstruct_context` does not produce
  `AgentMessage::Custom` (ExtensionState is metadata-only), but the code path
  exists and should be hardened.
- **Fix**: Apply `redact_export_text` to `c.data.to_string()` in markdown;
  serialize through a redaction pass in JSON.

**i8** `ExportRedactMode::None` is safe by design (Info)

- The default is `Summary`. `None` is opt-in for trusted-local contexts.

**i9** Source session preservation confirmed (Info)

- `export_session` only reads the source file. Write failures do not affect the
  source. Tests verify byte-for-byte preservation.

---

## 6. Test Quality Assessment

### 6.1 Coverage matrix

| Entry Type | Serde RT | JSONL W/R | reconstruct_context | Export | RPC/Display | Command |
|------------|----------|-----------|---------------------|--------|-------------|---------|
| session_info | `session_info_entry_round_trip_and_shape` | `facade_metadata_entries_attach_to_tip...` | `model_thinking_session_name_metadata_latest_wins` | **MISSING** | `phase13_rpc_session_info_returns_metadata` | `phase13_session_picker_command_remains_resume_picker` |
| model_change | `model_change_entry_round_trip_and_shape` | `facade_metadata_entries_...` | `model_thinking_session_name_metadata_latest_wins` | **MISSING** | `phase13_rpc_session_info_returns_metadata` | `rpc_parse_set_model_command` |
| thinking_level_change | `thinking_level_change_entry_round_trip_and_shape` | `facade_metadata_entries_...` | `model_thinking_session_name_metadata_latest_wins` | **MISSING** | `phase13_rpc_session_info_returns_metadata` | `rpc_parse_set_thinking_level_command` |
| label | `label_entry_round_trip_and_shape_for_add_and_remove` | `phase13_session_info_and_label_appends_are_durable...` | `labels_excluded_from_llm_context_but_in_metadata` | **MISSING** | `phase13_rpc_session_info_returns_metadata` | `phase13_session_picker_command_remains_resume_picker` |
| branch_summary | `branch_summary_entry_round_trip_and_shape` | `facade_metadata_entries_...` | `branch_summary_included_in_llm_context` | **MISSING** | `phase13_rpc_session_metadata_shape` | MISSING (deferred) |

### 6.2 Assertion strength

**Strong**: session_storage.rs (field-level equality), session_context.rs
(message count + text + metadata values + diagnostic codes), session_contract.rs
(byte-level JSON comparison + proptest), session_facade.rs (flush ordering +
tip invariance).

**Acceptable but weaker**: session_export.rs (canary substring checks for
redaction), picker_integration.rs (substring metadata checks).

### 6.3 Isolation -- Excellent

All tests use `tempfile::tempdir()` + `OPI_SESSIONS_DIR`. Env-var-dependent
tests serialize via Mutex. No cross-test interference observed.

### 6.4 Positive highlights

- Property-based testing via proptest in session_contract.rs
- Forward-compatibility testing for unknown entry types
- Layered coverage: same semantic verified at storage/facade/context/runtime/
  CLI/RPC/picker levels
- Non-goal guard tests in phase13_session_context_docs.rs

### Findings

**M4** Export column entirely missing in coverage matrix

- **Severity**: Major
- **Description**: No test verifies that Phase 13 metadata entries
  (session_info, model_change, thinking_level_change, label, branch_summary)
  are correctly rendered in markdown or JSON exports. The export tests verify
  message content and canary redaction but not metadata presence.
- **Fix**: Add tests that build sessions with metadata entries, export, and
  verify metadata appears in the output.

**M5** No test for circular `parent_id` in `reconstruct_context`

- **Severity**: Major
- **Description**: While `SessionTree::from_entries` has a tested cycle
  detection path (`cycle_in_parent_id_detected`), `reconstruct_context`'s own
  chain walk (session_context.rs:150-171) uses a `visited` HashSet for cycle
  breaking but has no dedicated test. The two cycle-handling mechanisms are
  independent and could diverge.
- **Fix**: Add a test with circular parent_id entries (A->B->A), verify
  `reconstruct_context` terminates and emits appropriate diagnostics.

**m9** No Unicode round-trip test for metadata fields

- **Severity**: Minor
- **Description**: No test verifies that CJK/emoji/RTL characters in
  `session_info.name`, `label.label`, or `branch_summary.summary` survive the
  full chain: serde -> JSONL write -> JSONL read -> `reconstruct_context` ->
  export.
- **Fix**: Add a test with Unicode metadata (e.g., `name: "会话🎉"`,
  `label: "バグ"`).

**m10** Empty string label and session name behavior untested

- **Severity**: Minor
- **Description**: No test verifies the behavior of
  `enqueue_label("".into(), LabelAction::Add)` or
  `enqueue_session_info("".into())`. The code accepts them without validation;
  an empty label would persist silently.
- **Fix**: Either add validation that rejects empty strings, or add a test that
  pins the "accept empty" behavior.

**m11** `read_all` used by RPC but masks recovery information

- **Severity**: Minor
- **Description**: `rpc.rs:835` uses `SessionReader::read_all` to build the
  branch tree. Since `read_all` discards `CrashRecovery` (see M3), corrupt or
  unknown entries are silently dropped. The RPC response shows a potentially
  incomplete branch tree with no diagnostic.
- **Impact**: Phase 14 TUI consumers receive truncated branch data without
  warning.

**i10** No test for `SessionWriter::create` on read-only path (Info)

**i11** No test for `SessionReader::read_all` on empty (0-byte) file (Info)

**i12** No test for >64KB branch_summary JSONL serialization (Info)

**i13** No test for 1000+ label accumulation performance (Info)

---

## 7. Spec Compliance

### 7.1 Success Criteria trace

| SC | Verdict | Source | Test | Documentation |
|----|---------|--------|------|---------------|
| SC1 | PASS | session.rs entry types; session_coordinator.rs append_* | session_storage round-trip; session_runtime phase13_* | opi-spec.md L1567-1589 |
| SC2 | PASS | session_context.rs reconstruct_context | session_context.rs 20 tests; deterministic test | opi-spec.md L906-933 |
| SC3 | PASS | session.rs FORMAT_VERSION=1; read_with_recovery | session_storage v1 fixture; format_version test | opi-spec.md L906-909 |
| SC4 | PASS | session_context.rs module doc L22-47 | phase13_session_context_docs guard | opi-spec.md L1567-1589; opi-agent README |
| SC5 | PASS | branch_summary substrate; custom_message deferred | session_context branch_summary tests; doc guard | opi-spec.md L1580-1589 |
| SC6 | PASS | session_cli.rs export_session; cli.rs flags | session_export 6 tests; session_cli E2E | opi-spec.md L1554-1578 |
| SC7 | PASS | rpc.rs session_info; picker.rs metadata | rpc_jsonl phase13; picker_integration phase13 | Phase 14 handoff section |
| SC8 | PASS | -- | doc guard: "sensitive" keyword | opi-spec L946; README L196 |
| SC9 | PASS | No forbidden crates; no forbidden modules | phase13_non_goals_not_in_core guard | opi-spec L1597-1607 |

### 7.2 Non-Goals trace

All 10 non-goals PASS. No vector database, semantic memory, global profile,
cross-project injection, pi session v3, cloud sync, sharing service, web UI,
package expansion, or provider auth refactor was added. Verified by Cargo.toml
dependency scan and lib.rs module scan.

### 7.3 custom_message and branch_summary deferral

Both are explicitly documented:
- branch_summary: substrate implemented, generation UX (branch switch, fork,
  manual command, extension hook) deferred to Phase 14
  (opi-spec.md L1582-1586)
- custom_message: provider-context semantics deferred
  (opi-spec.md L1586-1589; session.rs L33-38)

### 7.4 Localization (EN/ZH) alignment

Core documents (opi-spec, README, opi-agent README, opi-coding-agent README)
are in sync. Entry type names use English identifiers consistently in both
languages.

### Findings

**m12** pi-alignment-matrix.md/zh status drift (5 occurrences)

- **File**: `docs/pi-alignment-matrix.md` L210, 232, 235;
  `docs/pi-alignment-matrix.zh.md` L169, 172, 190
- **Severity**: Minor
- **Description**: Five lines still use pre-Phase-13 wording:
  - L232: "Phase 13 should add stable metadata, summaries, labels, and export"
    (future tense for delivered features)
  - L235: "Local session/export direction is planned" (export already shipped)
  - L190 (ZH): Phase 13 row status is "计划中" instead of "已实现基底"
  - L210 vs L232 internal contradiction (L210 says "implemented", L232 says
    "should add")
- **Impact**: Readers may believe Phase 13 features are unimplemented.
- **Fix**: Update affected lines to reflect delivered status.

---

## 8. Invariant Verification

### INV-1: Metadata entries do not move the content tip

**Code trace**: All metadata `enqueue_*` methods in `SessionFacade`
(harness.rs:794-856) use `PendingWriteKind::Metadata`, which does **not**
update `content_tip_entry_id`. Only `enqueue_message` (L753-768) sets
`content_tip_entry_id = Some(entry.id)`.

In `SessionCoordinator` (session_coordinator.rs:495-564), all `append_*`
methods call `facade.enqueue_*` for metadata types, preserving the invariant.

**Test coverage**: `facade_metadata_entries_attach_to_tip_without_advancing_it`,
`phase13_model_thinking_metadata_does_not_advance_leaf`,
`phase13_session_info_and_label_appends_are_durable_and_do_not_advance_leaf`,
`facade_metadata_append_does_not_advance_active_tip`

**Verdict**: PASS -- invariant holds across all write paths.

### INV-2: Last-valid-Leaf-wins determines active branch

**Code trace**: `SessionTree::from_entries` (session_branch.rs) updates
`last_leaf_tip` on each `Leaf` entry encountered. `active_tip()` returns
`last_leaf_tip` if it exists in `entries_by_id`, else falls back to trunk.

**Test coverage**: `multiple_leaf_entries_last_leaf_wins_as_active_tip`,
`branched_session_last_leaf_wins`

**Verdict**: PASS.

### INV-3: Context reconstruction is deterministic

**Code trace**: No HashMap/HashSet iteration order leaks into output. All
ordering derives from input slice order (file order) or chain walk order
(single-path traversal).

**Test coverage**: `reconstruction_is_deterministic_across_calls`

**Verdict**: PASS.

### INV-4: Export does not modify source session

**Code trace**: `export_session` (session_cli.rs:417-439) reads via
`SessionReader::read_with_recovery` (opens file read-only) and writes only to
`options.output`. The source path is never passed to any write API.

**Test coverage**: `phase13_export_active_branch_markdown_redacts_and_preserves_source`,
`phase13_export_invalid_output_path_fails_and_preserves_source`,
`e2e_export_session_writes_output_and_preserves_source`

**Verdict**: PASS.

### INV-5: v1 sessions load without migration

**Code trace**: `FORMAT_VERSION = 1` (session.rs:53). `read_with_recovery`
accepts only version 1. Phase 13 entries are additive (new `type` tags under
the same version). The reader skips unknown types and reports via
`unknown_count`.

**Test coverage**: `session_format_version_is_v1_and_policy_disclaims_pi_v3_compatibility`,
`crash_recovery_reports_unknown_future_type_separately_from_corrupt`,
`v1_v2_fixture_parity_messages_identical`

**Verdict**: PASS.

---

## 9. Residuals and Recommendations

### All findings consolidated

| ID | Severity | File(s) | Issue | Impact | Fix |
|----|----------|---------|-------|--------|-----|
| M1 | **Major** | session_cli.rs:926, session_coordinator.rs:136, session_branch.rs:173 | `select_ordered_entries` returns empty on invalid Leaf; `SessionTree` falls back to trunk | Coordinator/agent buffer misalignment on corrupt sessions | Unify tip resolution via `SessionTree::active_tip()` |
| M2 | **Major** | session_context.rs:297-299 vs session_coordinator.rs:675-682 | `collect_metadata` rejects `parent_id=None`; `seed_metadata_from_entries` accepts it | Pre-first-turn metadata invisible in list/export/RPC | Add rootless-metadata guard to `collect_metadata` |
| M3 | **Major** | session.rs:405 | `read_all` doc says "strict mode" but silently discards corrupt data | Callers assume corrupt data triggers errors | Fix doc or make `read_all` error on corruption |
| M4 | **Major** | session_export.rs (absent tests) | No test verifies metadata entries in export output | Export correctness for metadata entries unverified | Add export tests for all 5 entry types |
| M5 | **Major** | session_context.rs:150-171 (absent test) | No dedicated test for circular parent_id in `reconstruct_context` | Cycle-handling correctness unverified | Add circular-reference test |
| m1 | Minor | session.rs:516-526 | `KNOWN_ENTRY_TYPES` not compile-time synced with `SessionEntry` | Future variant could be misclassified | Add sync-verification test |
| m2 | Minor | session.rs:327 | Doc says "crash-safe" but is "crash-recoverable" | Terminology mismatch | Update doc |
| m3 | Minor | session_cli.rs:266-304 | Fork drops pre-first-turn metadata | Fork loses name/labels set before first message | Accept `parent_id: None` when active set empty |
| m4 | Minor | interactive.rs:746-771 | `/name`/`/label` empty arg falls through to LLM | User confusion | Add usage hint |
| m5 | Minor | session_cli.rs:563-566, 687-692 | `AgentMessage::Custom` data not redacted in export | Sensitive data in custom messages could leak | Apply redaction to custom data |
| m6 | Minor | rpc.rs:833-866 | RPC session_info silently swallows SessionReader failure | Client sees incomplete data with no error | Surface read error as diagnostic |
| m7 | Minor | picker.rs:83-114 | Session picker omits model/thinking metadata | Less metadata than CLI/JSON output | Add model suffix to picker |
| m8 | Minor | picker.rs:57-59 | Branch summaries not redacted in picker | Inconsistent redaction | Apply `redact_text` |
| m9 | Minor | (absent tests) | No Unicode round-trip test for metadata | Unicode correctness unverified | Add Unicode metadata test |
| m10 | Minor | (absent tests) | Empty string label/name behavior untested | Undefined behavior not pinned | Add test or validation |
| m11 | Minor | rpc.rs:835 | `read_all` masks recovery in RPC branch tree | Corrupt entries silently dropped | Use `read_with_recovery` |
| m12 | Minor | pi-alignment-matrix.md/zh | 5 status drift occurrences | Docs say "planned" for delivered features | Update to reflect delivered status |
| i1 | Info | session.rs:424 | Bare CR line ending treated as complete | Legacy Mac OS 9 only |
| i2 | Info | session.rs:385 | Writer truncates to 0 when no newline | Documented behavior |
| i3 | Info | session_context.rs:123-124 | Duplicate entry_id collision silent | Corruption scenario |
| i4 | Info | session_context.rs:186-195 | BranchSummary with parent_id=None dropped | Unlikely in practice |
| i5 | Info | session_context.rs:136-145 | Leaf advisory parent_id triggers generic diagnostic | Ambiguous diagnostic text |
| i6 | Info | rpc.rs:837, harness.rs:1308 | Duplicated SessionTree build | Acceptable for correctness |
| i7 | Info | session_cli.rs:446-459 | Full-tree export drops non-Message | By design |
| i8 | Info | session_cli.rs:462-464 | ExportRedactMode::None is opt-in | Safe by design |
| i9 | Info | session_cli.rs:417-439 | Source session preservation confirmed | Read-only access |
| i10 | Info | (absent test) | No test for create on read-only path | Low priority |
| i11 | Info | (absent test) | No test for read_all on empty file | Low priority |
| i12 | Info | (absent test) | No test for >64KB branch_summary | Low priority |
| i13 | Info | (absent test) | No test for 1000+ label accumulation | Low priority |
| i14 | Info | session_context.rs:276 | `insert(0, ...)` is O(n) | Bounded by session size |
| i15 | Info | session_context.rs:301-328 | Unnecessary on_chain check for content variants | Trivial overhead |

### Priority recommendations

1. **Fix M1 first** -- the `select_ordered_entries` / `SessionTree` divergence
   is the highest-impact correctness issue. Unify tip resolution.
2. **Fix M2** -- the `collect_metadata` rootless guard is a one-line change
   with high consistency payoff.
3. **Fix M3** -- the `read_all` doc comment is misleading. A doc-only fix.
4. **Add M4 tests** -- export metadata coverage is a gap that could hide real
   rendering bugs.
5. **Add M5 test** -- circular parent_id is a correctness edge case that should
   be pinned.
6. **m5 before Phase 14** -- custom message redaction should be hardened before
   extensions can produce custom messages.

### Concurrency/thread safety (low priority)

`SessionWriter` uses `sync_all()` after each append. `SessionReader` reads the
entire file via `read_to_string`. Both are single-caller patterns in the
current async-single-threaded architecture. No file locking is used, but no
concurrent access pattern exists in the current codebase. No issue found.

---

*End of audit.*
