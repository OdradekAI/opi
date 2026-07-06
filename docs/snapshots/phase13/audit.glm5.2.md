# Phase 13 Independent Code Audit (GLM-5.2)

- Phase: 13 — Session Tree and Context Reconstruction
- Auditor: GLM-5.2 (ultracode mode, 6-dimension workflow + independent ground-truth reading)
- Date: 2026-07-06
- Scope: commits `388fda8` (13.1) through `5b4ba67` (archive); diff range `b98c6110..HEAD`, 41 files, +7,631 / -337
- Normative baseline: `docs/superpowers/specs/2026-06-24-phase13-session-tree-context-reconstruction-design.md`
- Output language: English

## Contamination statement

This audit was performed independently. Before and during the review I did **not** open, read, grep, cite, or summarize any other audit / review / critique document, in this repository or elsewhere. At the time of review, `docs/snapshots/phase13/` contained only `opi-impl-state.json` (no sibling `audit*` / `review*` files existed). Every workflow subagent was placed under the same off-limits rule and was directed not to open `opi-impl-state.json` or `phase4/opi-impl-state.json`.

The user pointed me at two inputs: the design spec and `docs/snapshots/phase13/opi-impl-state.json`. I used `opi-impl-state.json` only as a map (task graph, DoDs, file:line citation hypotheses) and the design spec as the normative baseline. Every factual claim below was re-derived by reading the actual code, tests, docs, and diffs. Findings are my own; I did not assume any prior evaluator verdict was correct.

## Methodology

1. Diff/stat mapping via `git` (commit range, file list, changed line counts).
2. Independent ground-truth reading in full of the six load-bearing surfaces: `opi-agent/src/session.rs`, `opi-agent/src/session_context.rs`, `opi-agent/src/session_branch.rs`, `opi-coding-agent/src/session_coordinator.rs`, `opi-coding-agent/src/session_cli.rs`, `opi-coding-agent/src/interactive.rs`, plus the `rpc.rs` `session_info` handler and the `agent_messages_to_llm` provider-conversion path.
3. Six-dimension independent review workflow (spec-conformance, correctness, test-quality, docs-accuracy, safety-error-handling, compat-api-stability) with adversarial per-finding verification (26 agents, 1.8M tokens, 15 min). Each medium/high/critical finding was independently re-checked against the cited code by one or two skeptic agents.
4. Targeted empirical gates (per-crate, per the host's disk-aware and `cfg(unix)`-invisible-on-Windows constraints).

## Empirical verification (run by the auditor)

Per-crate scoped gates, `CARGO_INCREMENTAL=0`, Windows host:

| Gate | Result |
|---|---|
| `cargo build -p opi-agent -p opi-coding-agent` | clean (17s) |
| `cargo clippy -p opi-agent --all-targets -- -D warnings` | clean |
| `cargo clippy -p opi-coding-agent --all-targets -- -D warnings` | clean |
| `cargo test -p opi-agent` (session_context/storage/facade/contract/branching/diagnostics) | 36 / 23 / 20 / 16 / 14 / 34 — all green |
| `cargo test -p opi-coding-agent` (session_export/cli/runtime, interactive_mock, rpc_jsonl, picker_integration, phase13_session_context_docs) | 6 / 41 / 46 / 8 / 92 / 12 / 3 — all green |

The ledger's "all green" claim reproduces. The previously-flaky `phase13_name_label_and_session_info_commands_persist_typed_entries` passed on this run. Note: full-workspace smoke (`cargo test --workspace --all-targets`) was deliberately not run (host `D:` had 70 GB free; the full smoke is known to fill the 452 GB disk via ~106 GB of `target/` bloat). Per-crate coverage is broad for the Phase 13 surface; `cfg(unix)`-gated tests are invisible on this Windows host and were not exercised locally.

## Executive summary

Phase 13 is structurally sound and the substrate is genuinely well-built. The deterministic context-reconstruction API lives in `opi-agent`, v1 readability is preserved, metadata appends provably do not advance the content tip, export preserves the source byte-for-byte, and the recovery split (truncated / corrupt / unknown) is correct and tested. All success criteria are met or source-cited-deferred, and no non-goal leaked in.

The audit found **2 High**, **6 Medium**, and **9 Low** issues. The two headline problems are documentation-vs-code contradictions rather than live data-loss bugs on today's production paths, but both ship silently and one (H1) describes a real forward-compat data-loss window:

- **H1**: Four owned doc surfaces claim unknown future entry types are "preserved on read." The reader drops them. Silent loss of additive entries written by a newer opi when an older opi reads + rewrites.
- **H2**: `AgentMessage::BranchSummary` is injected into the agent buffer by `reconstruct_context` but silently dropped by `agent_messages_to_llm` before the provider, contradicting the `session_context.rs` module doc ("Branch summaries **do** enter LLM context") and the spec table. The test named `branch_summary_included_in_llm_context` over-promises (asserts the in-memory buffer, not provider delivery). SC5 is in the spec-forbidden in-between state.

The remaining findings are consistency / drift / coverage gaps (two parallel active-branch walkers that diverge on degraded input, RPC `session_info` swallowing read errors, `pi-alignment-matrix` stale status, a few under-pinned tests, and minor polish).

## Strengths (independently confirmed)

- **SC1 metadata-as-attachment invariant holds.** `append_model_change` (session_coordinator.rs:495), `append_thinking_level_change` (510), `append_session_info` (529), `append_label` (549), `append_extension_state` (476) each parent to `active_tip_entry_id` and none mutate it or call `append_leaf_for_tip`. Verified by reading the bodies.
- **Recovery split is correct.** `read_with_recovery` (session.rs:412-507) classifies invalid-JSON or missing-`type` as corrupt, present-but-unrecognized `type` as unknown, and skips a truncated trailing line; the three buckets are independent and pinned by `crash_recovery_unknown_and_corrupt_are_independent_buckets`, `crash_recovery_reports_unknown_future_type_separately_from_corrupt`, `crash_recovery_json_object_without_type_field_is_corrupt`.
- **Export preserves the source.** `export_session` (session_cli.rs:417-440) opens the source read-only via `SessionReader::read_with_recovery`, renders in memory, then a single `std::fs::write` to `options.output`. Source is byte-for-byte unchanged on write failure; pinned on both success and failure paths in `session_export.rs`.
- **Fork preserves the source.** `fork_session` (session_cli.rs:210-257) never opens the source for writing; it writes only to a freshly-allocated file.
- **Resume applies recorded model + thinking on both paths** — CLI `--resume` (`main.rs` → `ResumeInfo.recorded_model/recorded_thinking` → `apply_recorded_*`) and interactive (`resume_session_id` → `reconstruct_context` → `ctx.model/thinking_level`).
- **Compaction layout is correct.** `apply_compaction` (session_context.rs:261-277) produces `[Summary, ...kept]`; nested compactions compose (verified by `nested_compactions_latest_compaction_wins`).
- **Cycle guards present** in all three parent-walkers (session_context.rs:154-158, session_cli.rs:1025-1030, session_branch.rs:215-222).
- **No new panic paths** introduced by the Phase 13 substrate (`session.rs`, `session_context.rs` are panic-free; remaining `unwrap/expect` in `rpc.rs`/`session_branch.rs` are pre-Phase-13 patterns on provably-serialized inputs).
- **Redaction at the boundary is consistent** for every text-derived field in export (`session_cli.rs` routes user/assistant/thinking/tool_call.arguments/tool_result.text/compaction/branch_summary through `redact_export_text`) and for RPC summaries (`rpc.rs:843-844, 854-855` via `redact_text(Summary)`). See L7 for the one exception (`AgentMessage::Custom`).

## Findings

Severity legend: **High** = ships silently with real correctness/spec impact; **Medium** = real defect or drift worth fixing before the contract is frozen; **Low** = polish / coverage / latent gap.

### High

#### H1 — Docs claim unknown future entries are "preserved on read"; the reader drops them

- **Files**: `docs/opi-spec.md:941`; `docs/opi-spec.zh.md:840`; `crates/opi-agent/README.md:241-242`; `crates/opi-agent/README.zh.md:209`
- **Code (drops)**: `crates/opi-agent/src/session.rs:481-497` (the `unknown` branch only increments `unknown_count`; the entry is never pushed into the returned `Vec`), `session.rs:506` (returns only `(header, entries, recovery)` — no side-channel carries unknown entries), `session.rs:257-259` (the `CrashRecovery::unknown_count` doc honestly says "skipped (not preserved on rewrite)")
- **Test (pins the drop)**: `crates/opi-agent/tests/session_storage.rs:840-841` — `assert_eq!(entries.len(), 1, "known entry survives; unknown skipped")`
- **Cause**: The reader's forward-compatibility path is skip-and-count. Four doc surfaces reworded this into a stronger "preserved on read" claim. The normative design spec (line 247) hedges correctly ("preserved or skipped according to documented compatibility rules"); the downstream user-facing surfaces drifted into an unqualified guarantee.
- **Impact**: An embedder trusting the documented forward-compat guarantee will silently lose additive entries when a newer opi writes them and an older opi loads + saves (e.g. via `--fork`). The loss is silent except for the `unknown_count` diagnostic. All four surfaces agree, so nothing internal flags the contradiction. This is also a stated success criterion (SC3 forward-compat).
- **Recommendation**: Correct all four surfaces to state unknown future entries are **skipped and counted via `CrashRecovery::unknown_count`**, and are **not preserved across a read+rewrite** — matching `session.rs:258`, `SESSION_FORMAT_POLICY`, and the pinning test. (Lossless rewrite via a preserved side-channel would be over-engineering and is inconsistent with the design's forward-compat-reads, not lossless-rewrite, non-goal.) Then strengthen `phase13_session_context_docs.rs` to assert the actual wording so it cannot drift back.

#### H2 — `BranchSummary` is synthesized into the agent buffer but dropped before the provider

- **Files**: `crates/opi-coding-agent/src/harness.rs:2180-2200` (drop); `crates/opi-agent/src/session_context.rs:31-36` (contradicting doc), `:222-234` (injection); `docs/opi-spec.md` Session Entry Model table ("enters LLM context: yes"); `crates/opi-agent/tests/session_context.rs:284-308` (over-promising test)
- **Cause**: `agent_messages_to_llm` matches `AgentMessage::Llm` (forward) and `AgentMessage::CompactionSummary` (render as synthetic `User`), then `_ => {}` silently discards `AgentMessage::BranchSummary` and `AgentMessage::Custom`. `reconstruct_context`, however, injects `AgentMessage::BranchSummary` into `ctx.messages` at the parent's chain position, and its module doc (session_context.rs:31-36) states "Branch summaries **do** enter LLM context (Phase 13 decision)." The harness.rs doc at 2178-2179 honestly says these variants "are dropped — they have no provider-facing representation yet." The two module docs contradict each other; the spec table and the substrate doc claim provider entry; the production converter strips it.
- **Impact**: A session file containing a `branch_summary` entry (reachable today via manual JSONL, hand-edit, a partially-recovered file, or any future generation trigger) will show the summary in export/JSON output but it will **never reach the model** — defeating the entry's documented purpose ("preserve context when leaving or forking a branch"). There is no provider-conversion test for `BranchSummary`, and no test pins the drop, so a future change that adds `BranchSummary` to the match (or removes it) goes undetected. The test named `branch_summary_included_in_llm_context` only inspects `ctx.messages`, so the name over-promises. SC5 ("branch_summary ... either implemented with provider-conversion tests OR explicitly deferred with reasons") is in the forbidden in-between: synthesized then dropped, with no test and no spec-level deferral at the contradicting doc site.
- **Blast-radius caveat**: Phase 13 ships no automatic branch-summary writer in the `opi` binary (`SessionCoordinator` has `append_session_info/label/model_change/thinking_level_change` but no `append_branch_summary`; the generic `AgentHarness::enqueue_branch_summary` is not called by `CodingHarness`). So the drop is reachable only via non-standard session files today. This keeps it out of Critical.
- **Recommendation**: Pick one and pin it. (a) Render `AgentMessage::BranchSummary` as a synthetic user/system marker in `agent_messages_to_llm` (mirroring `CompactionSummary`) and add a `MockProvider`/`wiremock` provider-conversion test asserting the summary reaches `request.messages`; or (b) explicitly defer provider-conversion with a product reason, rename the unit test to `branch_summary_synthesized_in_reconstructed_messages`, add `branch_summary_dropped_by_convert_to_llm` asserting `agent_messages_to_llm(&[BranchSummary(..)])` is empty, and correct the spec table to "substrate-only at the provider boundary" (EN + ZH) and the `session_context.rs` module doc. Either way, reconcile the two contradicting doc blocks.

### Medium

#### M1 — Two parallel active-branch walkers with different graph semantics (SC2 drift)

The deterministic reconstruction required by SC2 is owned by `opi-agent` (`reconstruct_context` via `SessionTree`), but `opi-coding-agent` retains a second walker (`select_ordered_entries` / `walk_active_branch` / `active_content_entry_ids` at session_cli.rs:926-1043) used by `SessionCoordinator::open_existing` (session_coordinator.rs:136), `fork_session` (session_cli.rs:216), `seed_metadata_from_entries` (session_coordinator.rs:673), and `latest_extension_state_entry_for_active_branch` (session_cli.rs:962). The two walkers have different graph semantics and diverge on degraded input:

- **M1a — malformed Leaf target (independent finding).** `SessionTree::active_tip` (session_branch.rs:173-182) **validates** the leaf tip: if the last Leaf's `entry_id` is not in `entries_by_id`, it falls back to the trunk tip. `select_ordered_entries` (session_cli.rs:931-943) does **not** validate; it walks from the raw Leaf `entry_id`, and `walk_active_branch`'s `by_id` (Message/Compaction only) returns `None` → the chain breaks immediately → **empty chain**. Concrete divergence: a session whose Leaf target is the corrupt/skipped entry (a realistic scenario — a corrupt middle content entry that the Leaf still points at) yields `reconstruct_context` = trunk (non-empty) but `SessionCoordinator::open_existing` = empty compaction buffer + `active_tip_entry_id = None`. The Agent message buffer and the coordinator's compaction buffer then desync, contradicting the coordinator's own doc (session_coordinator.rs:132-135 "uses the same Leaf-based branch selection as reconstruct_context so the coordinator's internal state stays aligned with the Agent's message buffer"). Subsequent compaction operates on a stale/empty buffer.
- **M1b — Message `parent_id` pointing at a metadata/non-content entry (workflow finding).** `reconstruct_context`'s `entries_by_id` (session_context.rs:123-124) indexes **all** entry variants, so the chain walk (159-168) walks **through** a metadata/BranchSummary id if a corrupt/hand-edited Message parents to one. `walk_active_branch`'s `by_id` (session_cli.rs:1007-1019) indexes only Message/Compaction, so it breaks. Result: `reconstruct_context` returns a longer/redirected chain than the coordinator replays.

- **Impact**: Well-formed production sessions do not trigger this (the Phase 13.1 invariant parents every new message to a content tip). The trigger is corrupt, hand-edited, or partially-recovered input. M1a is the more severe facet: it produces a non-empty Agent buffer but an empty compaction buffer on resume, which can compound on the next turn.
- **Recommendation**: Make the two walkers use identical graph semantics. The cleanest fix is to expose one opi-agent API that returns the raw active-chain entries (e.g. `SessionTree::active_chain_entries(&entries)` or `session_context::reconstruct_active_chain`) using the **same** tip-resolution and walk rules as `reconstruct_context`, and have `select_ordered_entries` delegate to it. That eliminates both facets and the SC2 "product-side walk" concern together. Add a regression test with a Message parented to a metadata id and a Leaf pointing at a corrupt target, asserting the two paths agree.

#### M2 — RPC `session_info` swallows `SessionReader::read_all` errors and emits a partial payload with no diagnostic

- **Files**: `crates/opi-coding-agent/src/rpc.rs:832-867`
- **Cause**: The entire branch-tree payload (`entry_count` at 841, `branch_summary` at 843, `branches[]` at 865) is inside `if let Ok((_, entries)) = SessionReader::read_all(&session_path) { ... }`. There is no `else` arm. On `Err` (file removed mid-session, I/O error, corrupt header per session.rs:436-441, version mismatch per 450-458) the block is skipped silently and the response still succeeds (868) but omits the tree fields with no indication why.
- **Impact**: An embedder calling `session_info` cannot distinguish "session has no branches" from "the read failed." The design spec's Error Handling section requires failed branch navigation to "record diagnostics without destroying branch navigation" — navigation survives here, but the diagnostic is missing.
- **Recommendation**: On `Err`, emit a stable diagnostic (e.g. `CODE_SESSION_TREE_READ_FAILED`) into the response's diagnostics, or add a `tree_error` field. Re-reading the whole file on every `session_info` RPC is also O(n); a cached `harness.session_tree()` accessor (see M5) would address both.

#### M3 — `pi-alignment-matrix` still labels Phase 13 "Planned" while `opi-spec` declares it implemented

- **Files**: `docs/pi-alignment-matrix.md:253`; `docs/pi-alignment-matrix.zh.md:190`
- **Cause**: The dashboard status column reads "Planned" / "计划中" for Phase 13 while `docs/opi-spec.md:1556` states "Status: implemented substrate plus product paths" and the Phase 12 row was updated. The Detailed Feature Alignment row (matrix.md:210) correctly notes Phase 13 entries exist, so the staleness is confined to the dashboard column in both languages.
- **Impact**: The matrix positions itself as the durable pi-0.80.2 evidence baseline and a P0 remediation priority (matrix.md:272), and its own maintenance rule says "Update this matrix whenever a phase completes." Readers get a stale status that contradicts the spec and shipped code.
- **Recommendation**: Update both rows to "Partial" (mirroring spec:1556 — "implemented substrate plus product paths; branch_summary generation UX and interactive /export deferred to Phase 14"), matching how Phase 12 was updated.

#### M4 — `seed_metadata_from_entries` diverges from `reconstruct_context` on rootless metadata

- **Files**: `crates/opi-coding-agent/src/session_coordinator.rs:670-698` (accepts rootless); `crates/opi-agent/src/session_context.rs:294-306` (rejects rootless)
- **Cause**: `seed_metadata_from_entries` treats metadata with `parent_id == None` as on-chain when `active_ids.is_empty()` (the pre-first-turn case, session_coordinator.rs:675-682). `collect_metadata`'s `on_chain(None)` returns false unconditionally (session_context.rs:297-306), so rootless metadata is never applied. Its own doc (session_coordinator.rs:668-669) claims it "Mirrors the `reconstruct_context` derivation" — it does not.
- **Impact**: A `/name` or `/label` written before the first content turn yields a live `name()`/`labels()` view (coordinator) but `reconstruct_context(...).session_name/labels` = `None`/`[]`. So `/session info` (coordinator-backed) and an export/resume via `reconstruct_context` disagree on the same session. Low blast radius (only pre-first-turn metadata), but it is a real two-source-of-truth inconsistency.
- **Recommendation**: Pick one rule. Either teach `collect_metadata` to accept rootless metadata when the chain is empty (matching the coordinator), or drop the rootless acceptance in `seed_metadata_from_entries` and document that metadata must be parented to a content tip. Either way, correct the doc comment.

#### M5 — Triplicated `SessionTree::from_entries` + `SessionReader::read_all` build (unstable Phase 14 handoff)

- **Files**: `crates/opi-coding-agent/src/rpc.rs:835-837`; `crates/opi-coding-agent/src/harness.rs:1308-1311` (`branch_picker_items`); `crates/opi-coding-agent/src/harness.rs:1322-1334` (`resume_session_branch_tip`)
- **Cause**: Three independent sites each call `SessionReader::read_all` on the live session path and rebuild `SessionTree::from_entries`. There is no shared `harness.session_tree()` accessor (this is the documented task-13.6 R2 residual).
- **Impact**: The three sites can produce inconsistent tree shapes if any one diverges (e.g. one swallows read errors — see M2; the others may not). Phase 14 inherits an unstable handoff contract.
- **Recommendation**: Extract `harness.session_tree()` (cached, with a single error-handling policy) and route all three sites through it. This also resolves M2's silent-failure path in one place.

#### M6 — No test pins that `recorded_model` / `recorded_thinking` from an inactive branch are excluded on resume

- **Files**: `crates/opi-coding-agent/tests/session_runtime.rs:2834-2935`; `crates/opi-agent/tests/session_context.rs:591-609`
- **Cause**: The resume tests cover only the happy path (a single active-chain `model_change`/`thinking_level_change`). `metadata_on_inactive_branch_does_not_apply` puts a `model_change` only on the inactive branch and asserts `ctx.model == None`. No test puts `model_change` on **both** the active chain and an inactive sibling and asserts the active one wins on the **resume re-application** path (`ResumeInfo.recorded_model`).
- **Impact**: A future refactor that weakened `collect_metadata` to "latest `model_change` in file order" (a plausible simplification) would pass every existing test and silently let a sibling-branch model bleed into `ResumeInfo.recorded_model` on resume.
- **Recommendation**: Add a test with a forked session (`m1 → m2a` active leaf, `m1 → m2b` sibling), `model_change mc_a` parented to `m2a` and `mc_b` parented to `m2b` (different provider/model), resume, and assert `harness.model() == mc_a.model` and `!= mc_b.model` on both the interactive and CLI `--resume` paths.

### Low

- **L1 — No behavioral test for `--redact verbose` (export path).** `crates/opi-coding-agent/tests/session_cli.rs:786-798` only asserts the `Verbose` enum value parses; `session_export.rs` exercises only `Summary` and `None`. The only `Summary`-vs-`Verbose` difference on the text path is absolute-path handling (`redact_summary_paths`, diagnostic.rs:561-566: `Summary` redacts paths, `Verbose` preserves them), so a `Verbose`→`None` drift on free-text paths would leak (secrets are still scrubbed by `SecretRedactor` unconditionally). Add a test exporting a path-bearing message under `Verbose` asserting the path survives while a secret is scrubbed.
- **L2 — `reconstruct_context` recovery tests hand-build `CrashRecovery`.** `crates/opi-agent/tests/session_context.rs:543-584` (`corrupt_middle_entries_forwarded_from_crash_recovery`, `unknown_entries_forwarded_from_crash_recovery`, `truncated_line_forwarded_from_crash_recovery`) construct `CrashRecovery { .. }` literals. They pin the diagnostic-mapping but not the reader→reconstruction pipeline. The reader-side corrupt-middle detection is tested separately (`session_storage.rs:522`) but its output is never fed to `reconstruct_context`. If `read_with_recovery` stopped populating `corrupt_count`, these tests would still pass. Add one joined test: corrupt middle line → `read_with_recovery` → `reconstruct_context` → assert `CODE_SESSION_CORRUPT_ENTRIES` appears.
- **L3 — `/label` + `/unlabel` dispatcher test does not close the aggregation loop.** `crates/opi-coding-agent/tests/interactive_mock.rs:488-588` asserts the durable Add/Remove entries are persisted but never reads back the aggregated live set via `session_metadata()` after `/unlabel`. The aggregation half is tested through `harness.add_label/remove_label` directly (`session_runtime.rs:2688`), not through the parsed slash-command dispatcher. The dispatcher itself is a thin delegation, so production risk is low, but the two halves are never joined through the actual command path.
- **L4 — `--format` documented as `md|json` but the clap value is `markdown|json`.** `crates/opi-coding-agent/src/cli.rs:20-25` registers `markdown`/`json` only (no `md` alias). `crates/opi-coding-agent/README.md:100` and `README.zh.md:94` document `--format <md|json>`. `docs/opi-spec.md:948` is correct. A user copying the README invocation hits a loud clap parse error. One-character fix in two README files (or register `#[clap(alias = "md")]`).
- **L5 — A valid complete final entry written without a trailing newline is dropped.** `crates/opi-agent/src/session.rs:424, 466-473`. `last_line_incomplete` is true whenever the file lacks a trailing `\n`; the last data line is then skipped (`continue`) and flagged via `truncated_line`, not recovered. If a crash leaves a complete entry whose only fault is the missing newline, that entry is lost. Narrow window (the writer `writeln!`s then `sync_all`s), but a real data-loss-on-crash edge. Consider attempting to parse the trailing line and recover it when it deserializes cleanly.
- **L6 — `AgentMessage::Custom` is emitted unredacted in export.** `crates/opi-coding-agent/src/session_cli.rs:563-567` (markdown: `c.data.to_string()`) and `:687-692` (JSON: `c.data` raw). Neither routes `c.data` through `redact_export_text`. Today this is unreachable (`custom_message` is deferred; `reconstruct_context` never synthesizes `AgentMessage::Custom` — session_context.rs:38-41), so it is a latent trap, not a live leak. If a future change makes `Custom` reachable, export would leak regardless of `--redact`. Route `c.data` through redaction (or stringify-then-redact) and add a guard test.
- **L7 — RPC `session_info` uses four different absence conventions in one response.** `crates/opi-coding-agent/src/rpc.rs:797-867`. `name` → `Null` when absent (808-811); `labels` → always a possibly-empty array (812-817); `active_branch` → omitted entirely when absent (818-820); `branch_summary` → omitted when absent (842-845). Embedders must special-case each field. Pick one convention (recommend `skip_serializing_if` everywhere, matching `SessionInfo`).
- **L8 — `KNOWN_ENTRY_TYPES` is a manual mirror of the enum's serde tags.** `crates/opi-agent/src/session.rs:516-526`. Adding a `SessionEntry` variant without updating this `&[&str]` const would round-trip the new entry as "unknown" on read → silent drop on rewrite. The round-trip tests pin existing tags but there is no compile-time linkage. Low because the failure mode is safe (skip-and-count) and tests would likely catch a real new variant, but it is a maintenance hazard. Consider deriving the list from the enum, or adding a test that asserts the const equals the set of tags produced by `serde_json` round-trip.
- **L9 — Header-only file with no trailing newline reports `truncated_line: true`.** `crates/opi-agent/src/session.rs:424`. Nothing was truncated (the file is a complete header). Minor false-positive in the recovery signal.
- **L10 — `BranchSummaryMessage.parent_session_id` and `entry_count` are dead on every current path.** `crates/opi-coding-agent/src/session_context.rs:226-230` hardcode `""` and `0`; the export renderer emits them verbatim (session_cli.rs:558-559, 684-685). The fields exist for forward compatibility (cross-session injection is owned by 13.3 fork wiring, which does not populate them). Documented carry-over; emit them only when non-default, or note in the export schema that they are placeholders.

### Refuted / non-finding (for completeness)

- **"Export always emits `tool_call.arguments` even when `include_tool_output` is false."** Refuted. `include_tool_output` gates the separate `Message::ToolResult` block (session_cli.rs:535, 653), not the `AssistantContent::ToolCall` rendered inside an Assistant message. Tool-call arguments are also already redacted via `redact_export_text` (session_cli.rs:526, 639) on top of the persist-time redaction in `tool_arguments_for_session`. Emitting the (redacted) tool call is intended transcript behavior.

## Phase 13 success-criteria assessment (independent)

| SC | Status | Note |
|---|---|---|
| SC1 | Met, with H2 caveat | Entries defined in `opi-agent`; production append path exists for `session_info`/`model_change`/`thinking_level_change`/`label`. `branch_summary` is defined + read-injected but has no production writer and is dropped at the provider (H2). |
| SC2 | Met, with M1 caveat | The LLM-context reconstruction is singularly owned by `reconstruct_context`. A second walker in `opi-coding-agent` (`select_ordered_entries`) seeds the compaction buffer and diverges on degraded input (M1). |
| SC3 | Met at the code level; **H1** at the doc level | v1 sessions load; unknown entries are skipped+counted. But four doc surfaces falsely claim "preserved." |
| SC4 | Met | Branch/label/name/model/thinking/compaction/summary semantics are documented. |
| SC5 | **Not cleanly met** | `branch_summary` is in the spec-forbidden in-between: substrate-injected, provider-dropped, no provider-conversion test, no spec-level deferral at the contradicting doc site (H2). `custom_message` is cleanly deferred. |
| SC6 | Met | Export supports markdown + json with redaction and preserves the source on success and failure (tested). |
| SC7 | Met, with M5 caveat | RPC + picker handoff metadata is stable; the tree-build triplication (M5) is a cleanup obligation before the contract is frozen. |
| SC8 | Met | Session files documented as sensitive (EN + ZH). |
| SC9 | Met | No vector DB, semantic memory, global profile, cross-project injection, pi-v3 compat, cloud sync, sharing service, web UI, package expansion, or provider-auth refactor added. Verified structurally (no forbidden crates in `Cargo.toml`; no forbidden modules). |

## Non-goals (independent check)

All 10 non-goals are deferred. No forbidden crates (`qdrant`/`pinecone`/`weaviate`/`milvus`/`chromadb`/`oauth2`/`openidconnect`) in `Cargo.toml`. No forbidden modules (`cloud_sync`/`session_share`/`vector_memory`/`semantic_memory`/`global_profile`/`web_ui`). Export writes a local file only (`session_cli.rs:437`). `provider_collection.rs` OAuth remains unimplemented.

## Prioritized recommendations

1. **Fix H1** — correct the four "preserved on read" doc surfaces to "skipped + counted via `CrashRecovery::unknown_count`, not preserved across rewrite"; strengthen the docs guard. (Doc-only, low risk.)
2. **Resolve H2** — pick (a) provider-conversion + test or (b) explicit deferral + rename the over-promising test; reconcile the two contradicting doc blocks and the spec table.
3. **Fix M1** — expose one opi-agent API for the raw active-chain entries and delete the product-side walker; add the corrupt-Leaf and metadata-parent regression tests. (Most substantive substrate fix; bounds the resume/compaction desync.)
4. **Fix M2 + M5 together** — extract `harness.session_tree()` with a single read-error policy (emit `CODE_SESSION_TREE_READ_FAILED` instead of silent partial payload).
5. **Fix M3** — update `pi-alignment-matrix` EN + ZH Phase 13 row to "Partial."
6. **Fix M4** — unify rootless-metadata handling between `seed_metadata_from_entries` and `collect_metadata`; correct the doc comment.
7. **Add M6 test** — active-vs-inactive `model_change` on the resume re-application path.
8. Address L1–L10 as polish; L4, L7, L9 are one-line fixes.

## Notes on review limitations

- `cfg(unix)`-gated tests are compiled out on this Windows host and were not exercised locally; `ci.yml` (linux) is the authoritative runner for those.
- Full-workspace smoke was not run (host disk). Per-crate scoped gates covered the entire Phase 13 test surface.
- The 6-dimension workflow's automated bucketing classified "partial"-verdict findings as "refuted." I re-read every verdict and reclassified: of 7 bucketed-"refuted" findings, only 1 was a true non-finding (the `tool_call.arguments` item, listed above); the other 6 are real and appear above as M2, M5, L7, and the related facets of M1/H2.
