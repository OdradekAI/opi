# Phase 13 Remediation Plan

**Date**: 2026-07-06
**Audit sources**:
- `docs/snapshots/phase13/audit.codex.md` (Codex)
- `docs/snapshots/phase13/audit.glm5.2.md` (GLM-5.2)
- `docs/snapshots/phase13/audit.opus4.6.md` (Opus 4.6)

**Commit range audited**: `b98c6110..5b4ba67` (Phase 13.1 entry-model through Phase 13 archive snapshot).
**Design spec**: `docs/superpowers/specs/2026-06-24-phase13-session-tree-context-reconstruction-design.md`.

## Working-tree state (read this first)

The audits targeted the committed state `5b4ba67`. At remediation time the
working tree contains **~1200 lines of uncommitted changes (+1205/-300 across
25 files)** that constitute an in-progress remediation pass for these same
audits. There is no stash and no prior remediation commit in the reflog
(`HEAD@{0} = 5b4ba67`).

The user directed this run to **build on the uncommitted remediation** and plan
only the remaining residuals. Accordingly:

- Every finding whose audited defect is no longer present in the working tree
  is classified **"Already addressed (verify-only)"** and enters the plan as a
  verification item, not a fix item.
- Findings whose cited doc strings, status words, or code structures do not
  exist in the current tree are classified **"Refuted"** (misreading or stale
  snapshot) with primary evidence.
- Only genuinely remaining residuals produce fix items below.

This plan is **plan-only**. No execution (Phase F) without explicit user
confirmation. Per the project git rules, the pre-existing uncommitted changes
are not assumed to be this session's; if Phase F runs and commits, it must
stage only the specific files Phase F modifies.

## Audit cross-reference summary

Unified severity: Blocker / Major / Minor / Info. Consensus: Full (3/3),
Majority (2/3), Unique (1/3).

| Cluster | Theme | Auditors | Consensus | Severity | Verification |
|---|---|---|---|---|---|
| C1 | `BranchSummary` provider drop (SC5 in-between) | Codex P1, GLM H2 (Opus missed) | Majority | Major | **Already addressed (WT)** |
| C2 | Model picker bypasses durable `model_change` | Codex P1 | Unique | Major | **Already addressed (WT)** |
| C3 | Model/thinking write failures reported as success | Codex P1 | Unique | Major | **Already addressed (WT)** |
| C4 | Rootless metadata (`parent_id: None`) rejected | Codex P2, GLM M4, Opus M2 | Full | Major | **Already addressed (WT)** |
| C5 | Invalid-leaf fallback divergence | Codex P2, GLM M1, Opus M1 | Full | Major | **Refuted (divergence)**; residual C5' (silent fallback) → fix |
| C6 | Resume recovery diagnostics duplicated | Codex P3 | Unique | Minor | **Already addressed (WT)** |
| C7 | Docs claim unknown entries "preserved on read" | GLM H1 | Unique | Major | **Refuted** |
| C8 | RPC `session_info` swallows `read_all` errors | GLM M2, Opus m6 | Majority | Minor | **Already addressed (WT)** |
| C9 | `pi-alignment-matrix` Phase 13 status drift | GLM M3, Opus m12 | Majority | Minor | **Refuted** |
| C10 | Triplicated `SessionTree` build; no shared accessor | GLM M5 (Opus i6) | Majority | Minor | **Confirmed → fix** |
| C11 | `KNOWN_ENTRY_TYPES` not compile-time synced | GLM L8, Opus m1 | Majority | Minor | **Partially confirmed → test** |
| C12 | `AgentMessage::Custom` unredacted in export | GLM L6, Opus m5 | Majority | Minor | **Refuted (WT)** |
| C13 | No test: sibling model_change loses on resume | GLM M6 | Unique | Minor | **Already addressed (WT)** |
| C14 | No behavioral test for `--redact verbose` | GLM L1 | Unique | Minor | **Gap confirmed → test** |
| C15 | Recovery tests hand-build `CrashRecovery` | GLM L2 | Unique | Minor | **Confirmed → test** |
| C16 | `/label`+`/unlabel` dispatcher aggregation readback | GLM L3 | Unique | Minor | **Gap confirmed → test** |
| C17 | `--format md\|json` doc vs clap value mismatch | GLM L4 | Unique | Minor | **Already addressed (WT)** |
| C18 | Complete last entry w/o trailing newline dropped | GLM L5 | Unique | Minor | **Info / no-action** |
| C19 | RPC `session_info` four absence conventions | GLM L7 | Unique | Minor | **Confirmed → doc-only** |
| C20 | Header-only file false `truncated_line: true` | GLM L9 | Unique | Minor | **Info / no-action** |
| C21 | `BranchSummaryMessage` dead placeholder fields | GLM L10 | Unique | Minor | **Confirmed → doc note** |
| C22 | `read_all` "strict mode" doc claim | Opus M3 | Unique | Minor | **Refuted (wording)** |
| C23 | No export test for metadata/branch_summary entries | Opus M4 | Unique | Minor | **Gap confirmed → test** |
| C24 | No circular `parent_id` test in reconstruct | Opus M5 | Unique | Minor | **Refuted** |
| C25 | "crash-safe flush" wording | Opus m2 | Unique | Minor | **Refuted** |
| C26 | Fork drops pre-first-turn metadata | Opus m3 | Unique | Minor | **Refuted (WT)** |
| C27 | `/name` `/label` empty arg falls through to LLM | Opus m4 | Unique | Minor | **Refuted (WT)** |
| C28 | Session picker omits model/thinking metadata | Opus m7 | Unique | Minor | **Deferred to Phase 14** |
| C29 | Branch summaries not redacted in picker | Opus m8 | Unique | Minor | **Already addressed (WT)** |
| C30 | No Unicode round-trip test for metadata | Opus m9 | Unique | Minor | **Gap confirmed → test** |
| C31 | Empty-string label/name behavior untested | Opus m10 | Unique | Minor | **Gap confirmed → test** |
| C32 | `read_all` masks recovery in RPC | Opus m11 | Unique | Minor | **Already addressed (WT)** |

## Decision record

| ID | Finding cluster(s) | Decision | Rationale | Decided by |
|---|---|---|---|---|
| D1 | C1, C2, C3, C4, C6, C8, C13, C17, C29, C32 | Treat as already-addressed in the working tree; plan records primary evidence, no new code. | WT diffs match each audited defect one-for-one (e.g. rpc.rs `read_all`→`read_with_recovery`+`tree_read_error`; `cli.rs` `alias = "md"`; `picker.rs` `redact_text`; `collect_metadata` `None => chain_set.is_empty()`; `set_model_validated` `?` propagation + failing-writer tests; `BranchSummary` provider arm + `runner_resume_forwards_branch_summary_to_provider`). User chose to build on WT. | auto (user-scoped) |
| D2 | C5 | Divergence refuted (both walkers route through `SessionTree::active_tip`); only the silent trunk-fallback residual remains. | `select_ordered_entries` delegates to `opi_agent::session_context::active_chain_entry_ids` → `SessionTree::active_tip`; the "empty chain vs trunk chain" framing is not reachable. Genuine residual is "no diagnostic when the leaf tip is missing and falls back to trunk." | auto |
| D3 | C7, C9, C22, C24, C25, C26, C27 | Refuted; no action. | Cited doc strings/status words/wording not present in current tree (C7 "not preserved across read+rewrite" on all 4 surfaces; C9 dashboard already "Partial"; C22 doc says "discarding recovery metadata"; C24 `reconstruct_context_terminates_on_circular_parent_ids` exists; C25 already "recoverable"; C26 `entry_on_active_chain` accepts None; C27 parser returns `Usage`). | auto |
| D4 | C12 | Refuted in WT. | Both markdown and JSON renderers route `c.data` through `redact_export_value` (session_cli.rs:567-571, 691-696). | auto |
| D5 | C18, C20 | Info / no-action. | Documented design tradeoffs: `CrashRecovery` doc (session.rs:252-253) frames the no-newline last line as a crashed partial write; `truncated_line` is defined as "no trailing newline". Changing reader recovery semantics is out of audit-remediation scope. | auto |
| D6 | C21 | Optional doc note. | `BranchSummaryMessage.parent_session_id`/`entry_count` are intentional forward-compat placeholders (synthesizer comment at session_context.rs:198-204); spec can note this so readers do not interpret `""`/`0` as data. | auto |
| D7 | C28 | Defer to Phase 14. | Design spec L301-305 scopes picker/tree-view presentation polish to Phase 14. Two-line display change is terminal presentation, not session semantics. | auto |
| D8 | C19 | Doc-only; defer wire-format normalization. | Normalizing absence conventions (e.g. `name` Null→omitted) is an embedder-facing wire change against `SDK_SCHEMA_VERSION == 3`; not justified for a Minor consistency nit. Document the convention instead. | auto |
| D9 | C10 | Extract `harness.session_tree()` shared accessor; route all three sites through it using `read_with_recovery`; preserve each site's existing error policy. | Closes the triplication and the inconsistent recovery handling (rpc.rs uses `read_with_recovery`; both harness sites use bare `read_all` hard-fail) without changing observable error behavior at any call site. | auto |
| D10 | C11, C14, C15, C16, C23, C30, C31 | Add the missing tests (additive). | Pure coverage gaps; tests are the correct remediation. | auto |

## Remediation layers

### Layer 2: opi-agent (substrate; depends on opi-ai)

**Verification**:

    cargo fmt --all
    cargo clippy -p opi-agent --all-targets -- -D warnings
    cargo test -p opi-agent --all-targets

#### Fix 2.1: Diagnostic when leaf tip falls back to trunk (C5 residual)

- **Audit source**: Codex P2, GLM M1, Opus M1 (cluster C5; residual C5').
- **Decision**: D2.
- **Verification status**: Confirmed (residual).
- **File(s)**: `crates/opi-agent/src/session_branch.rs` ~L173-182 (`active_tip` fallback); `crates/opi-agent/src/session_context.rs` ~L237-244 (`active_chain_entry_ids` consumer).
- **Change**: When `SessionTree::active_tip` falls back to the trunk tip because the recorded Leaf `entry_id` is not in `entries_by_id`, surface a stable diagnostic (e.g. `CODE_SESSION_LEAF_TIP_MISSING`) so the silent trunk fallback is observable. Thread it through `reconstruct_context`'s diagnostics so resume/RPC see it. Purely additive; no behavior change to chain selection.
- **Test plan**: new test `active_tip_fallback_emits_diagnostic_when_leaf_target_missing` in `tests/session_branching.rs` (or `session_context.rs`), asserting the diagnostic fires and the trunk tip is still returned.

#### Fix 2.2: Pin `KNOWN_ENTRY_TYPES` to the enum's serde tags (C11)

- **Audit source**: GLM L8, Opus m1.
- **Decision**: D10.
- **Verification status**: Partially confirmed.
- **File(s)**: `crates/opi-agent/src/session.rs` ~L515-525 (const) and enum at ~L196-208; test in `crates/opi-agent/tests/session_storage.rs` near the existing `known_entry_types_match_session_entry_serde_tags` (~L810).
- **Change**: strengthen (or add) a test that serializes one minimal instance of every `SessionEntry` variant and asserts each emitted `type` tag is in `KNOWN_ENTRY_TYPES`, AND asserts the const size equals the number of serde-emitting variants. This pins the manual const to the enum so a future variant added without updating the const is caught.
- **Test plan**: extend `known_entry_types_match_session_entry_serde_tags` (or add `known_entry_types_covers_every_serde_tag`) to enumerate all variants and assert set equality.

#### Fix 2.3: Joined reader→reconstruct_context corrupt-middle test (C15)

- **Audit source**: GLM L2.
- **Decision**: D10.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-agent/tests/session_context.rs` near the three `*_forwarded_from_crash_recovery` tests (~L628-668).
- **Change**: add one joined test that writes a session with a corrupt middle line via `SessionWriter`, reads it back with `SessionReader::read_with_recovery`, feeds the result to `reconstruct_context`, and asserts `CODE_SESSION_CORRUPT_ENTRIES` appears in `ctx.diagnostics`. The three existing tests hand-build `CrashRecovery` literals and do not exercise the reader→reconstruction pipeline.
- **Test plan**: new test `read_with_recovery_feeds_corrupt_middle_into_reconstruct_context`.

### Layer 3: opi-coding-agent (depends on opi-ai, opi-agent, opi-tui)

**Verification**:

    cargo fmt --all
    cargo clippy -p opi-coding-agent --all-targets -- -D warnings
    cargo test -p opi-coding-agent --all-targets

#### Fix 3.1: Extract `harness.session_tree()` shared accessor (C10)

- **Audit source**: GLM M5.
- **Decision**: D9.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-coding-agent/src/harness.rs` ~L1316-1318 (`branch_picker_items`), ~L1330-1332 (`resume_session_branch_tip`); `crates/opi-coding-agent/src/rpc.rs` ~L834-851 (`session_info`).
- **Change**: add `AgentHarness::session_tree(&self) -> Result<SessionTree, String>` (or a method returning `(SessionTree, CrashRecovery)`) that performs `SessionReader::read_with_recovery` on the live session path and builds `SessionTree::from_entries`. Route all three sites through it. Keep each site's existing error policy: rpc.rs maps `Err` → `tree_read_error` (already does); the two harness sites keep `?` propagation. This removes the triplication and switches the two harness sites off bare `read_all` so recovery diagnostics are no longer discarded there.
- **Test plan**: existing `phase13_rpc_session_metadata_shape`, `phase13_branch_picker_session_metadata`, and resume tests cover the three call sites; add `session_tree_accessor_returns_same_tree_as_direct_build` asserting the accessor and a direct `SessionTree::from_entries` agree on a fixture with a known branch shape.

#### Fix 3.2: `--redact verbose` behavioral test (C14)

- **Audit source**: GLM L1.
- **Decision**: D10.
- **Verification status**: Gap confirmed.
- **File(s)**: `crates/opi-coding-agent/tests/session_export.rs`.
- **Change**: add a test exporting a message containing an absolute path and a secret under `ExportRedactMode::Verbose`, asserting the absolute path survives (Verbose preserves paths via `redact_summary_paths`) while the secret is still scrubbed (`SecretRedactor` is unconditional). The distinguishing behavior between `Verbose` and `None` is path handling; current tests exercise only `Summary` and `None`.
- **Test plan**: new test `phase13_export_verbose_preserves_paths_and_scrubs_secrets`.

#### Fix 3.3: `/label`+`/unlabel` dispatcher aggregation readback (C16)

- **Audit source**: GLM L3.
- **Decision**: D10.
- **Verification status**: Gap confirmed (narrow).
- **File(s)**: `crates/opi-coding-agent/tests/interactive_mock.rs` ~L514-614 (`phase13_name_label_and_session_info_commands_persist_typed_entries`).
- **Change**: after dispatching `/label <x>` then `/unlabel <x>` through the parsed slash-command dispatcher, read back the aggregated live set via `harness.session_metadata()` and assert the label is no longer present. The durable Add/Remove entries are already asserted; the aggregation half through the dispatcher is not closed.
- **Test plan**: extend the existing test (or add `phase13_unlabel_removes_label_from_live_aggregated_set_via_dispatcher`).

#### Fix 3.4: BranchSummary export test (C23)

- **Audit source**: Opus M4.
- **Decision**: D10.
- **Verification status**: Gap confirmed (narrow).
- **File(s)**: `crates/opi-coding-agent/tests/session_export.rs`.
- **Change**: add a fixture with a `SessionEntry::BranchSummary` entry, export markdown and JSON, and assert the rendered summary appears (redacted per `--redact`). The export tests currently use only message/compaction/tool fixtures; the BranchSummary renderer branch (session_cli.rs:558-566 / 685-690) is unexercised. (Note: by `reconstruct_context` contract, only `BranchSummary` of the metadata-style entries can appear in exports; `SessionInfo`/`ModelChange`/`Label` never enter `messages`.)
- **Test plan**: new test `phase13_export_renders_branch_summary_entry`.

#### Fix 3.5: Unicode round-trip test for metadata fields (C30)

- **Audit source**: Opus m9.
- **Decision**: D10.
- **Verification status**: Gap confirmed.
- **File(s)**: `crates/opi-coding-agent/tests/session_export.rs` (or `crates/opi-agent/tests/session_context.rs` for the substrate half).
- **Change**: build a session whose `branch_summary.summary` (and optionally `session_info.name`) contains non-ASCII content (e.g. `"会话🎉"`), serialize → JSONL write → read → `reconstruct_context` → export, and assert the content survives byte-for-byte. Nothing currently pins Unicode handling through the full chain.
- **Test plan**: new test `phase13_unicode_metadata_survives_full_round_trip`.

#### Fix 3.6: Empty-string label/name embedder contract (C31)

- **Audit source**: Opus m10.
- **Decision**: D10.
- **Verification status**: Gap confirmed (with mitigation).
- **File(s)**: `crates/opi-coding-agent/tests/session_runtime.rs` near existing label/session_info tests (~L2688, 2803).
- **Change**: add a test calling `append_label("".into(), LabelAction::Add)` and `append_session_info("".into())` directly and pin the current accept-without-validation behavior (the coordinator API takes any `String`). The TUI slash-command dispatcher pre-validates and rejects empty/whitespace, so this pins the embedder-facing contract explicitly.
- **Test plan**: new test `phase13_empty_string_label_and_name_are_accepted_at_coordinator_api`.

### Layer 4: Documentation

**Verification**: EN + ZH updated in the same change; no broken internal references; terminology consistent with the code.

#### Fix 4.1: Document `BranchSummaryMessage` placeholder fields (C21)

- **Audit source**: GLM L10.
- **Decision**: D6.
- **Verification status**: Confirmed (intentional placeholder, not a defect).
- **File(s)**: `docs/opi-spec.md` and `docs/opi-spec.zh.md` (Phase 13 entry-model section).
- **Change**: add a one-line note that `BranchSummaryMessage.parent_session_id` and `entry_count` are forward-compatibility placeholders currently emitted as `""` and `0`; cross-session injection (13.3 fork wiring) does not populate them. The synthesizer already documents this in code (`session_context.rs:198-204`); surface it in the spec so readers do not read the zeros as data.
- **Test plan**: existing `phase13_session_context_docs.rs` guard; extend if it pins the placeholder wording.

#### Fix 4.2: Document RPC `session_info` absence conventions (C19, doc-only)

- **Audit source**: GLM L7.
- **Decision**: D8.
- **Verification status**: Confirmed.
- **File(s)**: `docs/opi-spec.md` and `docs/opi-spec.zh.md` (RPC session_info schema section).
- **Change**: document the four absence encodings explicitly: `name` → JSON `null` when absent; `labels` → always a possibly-empty array; `active_branch` and `branch_summary` → omitted when absent. Normalizing the wire format is intentionally deferred to avoid breaking embedders on `SDK_SCHEMA_VERSION == 3`.
- **Test plan**: extend the `phase13_session_context_docs.rs` or RPC-schema doc guard to pin the documented convention.

## Final verification

    cargo fmt --all
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace --all-targets
    cargo test --workspace --doc

(Host note: per the disk-aware constraint, run per-crate gates first, then the
full-workspace smoke once the `D:` drive has headroom; `cfg(unix)`-gated tests
are invisible on Windows and are exercised by `ci.yml` on Linux.)

## Scope exclusions

| Cluster | Status | Reason / evidence |
|---|---|---|
| C1 | Already addressed (WT) | `harness.rs:2206` explicit `BranchSummary => Message::User` arm; provider-conversion test `runner_resume_forwards_branch_summary_to_provider` (`non_interactive.rs:358`). SC5 satisfied. |
| C2 | Already addressed (WT) | `interactive.rs:417` calls `set_model_validated` (durable path), not `set_model`. |
| C3 | Already addressed (WT) | `set_model_validated` (`harness.rs:951`) and `persist_thinking_level_change` (`:1056`) propagate append errors via `?`; failing-writer tests at `session_runtime.rs:2628` and `:2661`. |
| C4 | Already addressed (WT) | `collect_metadata` on_chain closure (`session_context.rs:370-373`): `None => chain_set.is_empty()`. |
| C5 (divergence) | Refuted | `select_ordered_entries` delegates to `active_chain_entry_ids` → `SessionTree::active_tip`; both walkers use the same validated tip resolution. |
| C6 | Already addressed (WT) | `main.rs:94` uses `ctx.diagnostics.clone()` only; `harness.rs:1181-1183` documents that `session.diagnostics` is intentionally not appended. |
| C7 | Refuted | All 4 cited surfaces say "not preserved across read+rewrite" verbatim; no overstatement. |
| C8 | Already addressed (WT) | `rpc.rs` switched `read_all` (no-else) → `read_with_recovery` with explicit `Err` arm surfacing `tree_read_error`. |
| C9 | Refuted | Dashboard row already reads "Partial"/"部分" (`pi-alignment-matrix.md:253`, `.zh.md:190`); only phases 9/11/14 read "Planned". |
| C12 | Refuted (WT) | Both export renderers route `c.data` through `redact_export_value` (`session_cli.rs:567-571`, `:691-696`). |
| C13 | Already addressed (WT) | `active_branch_metadata_wins_over_later_inactive_metadata` (`session_context.rs:700`) covers active-vs-sibling at the reconstruct layer. |
| C17 | Already addressed (WT) | `cli.rs:21` registers `#[clap(name = "markdown", alias = "md")]`. |
| C18 | Info / no-action | Documented design tradeoff (`session.rs:252-253`); changing reader recovery alters the documented contract. |
| C19 (wire) | Deferred | Wire-format normalization breaks embedders on `SDK_SCHEMA_VERSION == 3`; Fix 4.2 documents the convention instead. |
| C20 | Info / no-action | `truncated_line` is defined as "no trailing newline"; flag is honestly set by its own documented definition. |
| C22 | Refuted (wording) | `read_all` doc says "discarding recovery metadata", not "strict mode"; rpc.rs uses `read_with_recovery`. |
| C24 | Refuted | `reconstruct_context_terminates_on_circular_parent_ids` exists (`session_context.rs:612`). |
| C25 | Refuted | `SessionWriter` doc says "fsync and recoverable line-boundary handling", not "crash-safe flush". |
| C26 | Refuted (WT) | `entry_on_active_chain` accepts `None` when `active_set.is_empty()` (`session_cli.rs:266-293`). |
| C27 | Refuted (WT) | Parser returns `Usage(...)` for bare/empty `/name` `/label`; caller routes via `apply_session_metadata_command` and never reaches the LLM. |
| C28 | Deferred to Phase 14 | Picker display polish is Phase 14 terminal-presentation scope (design spec L301-305). |
| C29 | Already addressed (WT) | `picker.rs:58-63` applies `redact_text(summary, RedactionMode::Summary)`. |
| C32 | Already addressed (WT) | `rpc.rs:834` uses `read_with_recovery`; `tree_recovery` payload surfaced. |

## Notes for execution (Phase F)

- The active fix items are **all additive or small-consistency**: 7 tests, 1
  substrate diagnostic, 1 shared-accessor refactor, 2 doc notes. No public API
  removal, no wire-format change.
- Layered order matters only for Fix 3.1 (the accessor must exist before the
  three call sites are routed through it; they are in the same layer so it is
  one commit's worth of work).
- If Phase F runs and the user asks to commit, stage **only the specific files
  Phase F modifies**. Do not `git add -A`. The pre-existing ~1200 lines of
  uncommitted remediation are not this session's to attribute; the user should
  decide how to commit those separately.
