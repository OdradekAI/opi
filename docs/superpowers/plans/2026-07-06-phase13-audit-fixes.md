# Phase 13 Audit Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Verify and fix the Phase 13 audit findings that still hold in the current checkout.

**Architecture:** Put degraded-session active branch selection in `opi-agent` and make product paths delegate to it. Preserve the Phase 13 substrate semantics by forwarding `BranchSummary` to the provider, while keeping `Custom` deferred and hardened at export boundaries.

**Tech Stack:** Rust 2024, Cargo workspace, `opi-agent`, `opi-coding-agent`, JSONL session fixtures, existing `MockProvider` tests.

## Global Constraints

- Do not commit unless the user asks.
- Do not stage unrelated files or untracked audit snapshots.
- Use workspace dependencies only.
- Use TDD for behavior changes: write the regression test, verify it fails, then implement.
- For code changes, run `cargo fmt --all`, targeted tests, and `cargo clippy --workspace --all-targets -- -D warnings`.
- Keep documentation EN/ZH counterparts synchronized when touched.

---

## Verified Scope

- Accepted: rootless metadata is skipped by `reconstruct_context`.
- Accepted: `session_cli::select_ordered_entries` diverges from `SessionTree::active_tip` on invalid `Leaf` targets and from `reconstruct_context` when parent links pass through metadata.
- Accepted: TUI model picker bypasses durable `model_change`.
- Accepted: model/thinking metadata append failures are swallowed after mutating memory.
- Accepted: resume paths duplicate recovery diagnostics because `reconstruct_context` already forwards them.
- Accepted: RPC `session_info` silently omits branch tree data on session read failure and uses `read_all`, losing recovery diagnostics.
- Accepted: `AgentMessage::Custom` export branches skip redaction.
- Accepted: branch picker summary skips redaction.
- Accepted: `/name `, `/label `, and `/unlabel ` fall through to the LLM.
- Accepted: `--format md` is documented but not accepted by clap.
- Accepted: docs overstate unknown future entry preservation and BranchSummary provider status is contradictory.
- Accepted as test hardening: circular parent chain, known entry type tag sync, active-vs-inactive model metadata, export metadata, BranchSummary provider forwarding.
- Rejected/deferred: a cached `harness.session_tree()` accessor is useful but not required to fix the verified bug; current plan standardizes error handling without adding cache state.
- Rejected/deferred: session picker model/thinking display is product polish not necessary for the audited correctness contract.

## Task 1: Shared Active Chain And Metadata Semantics

**Files:**
- Modify: `crates/opi-agent/src/session_context.rs`
- Modify: `crates/opi-agent/tests/session_context.rs`

**Interfaces:**
- Produces: `pub fn active_chain_entry_ids(entries: &[SessionEntry]) -> Vec<String>`
- Consumes: existing `SessionTree::active_tip()` and all-entry `parent_id` walk semantics from `reconstruct_context`

- [ ] Add failing tests:
  - `rootless_metadata_applies_when_chain_is_empty`
  - `active_chain_entry_ids_falls_back_like_reconstruct_context_for_invalid_leaf`
  - `reconstruct_context_terminates_on_circular_parent_ids`
- [ ] Run: `cargo test -p opi-agent --test session_context rootless_metadata_applies_when_chain_is_empty`
  Expected: fails before the `collect_metadata` fix.
- [ ] Run: `cargo test -p opi-agent --test session_context active_chain_entry_ids_falls_back_like_reconstruct_context_for_invalid_leaf`
  Expected: fails before the new API exists.
- [ ] Implement `active_chain_entry_ids` by extracting the current all-entry chain walk used by `reconstruct_context`.
- [ ] Change `collect_metadata` so `parent_id: None` applies only when the active chain is empty.
- [ ] Run: `cargo test -p opi-agent --test session_context rootless_metadata_applies_when_chain_is_empty active_chain_entry_ids_falls_back_like_reconstruct_context_for_invalid_leaf reconstruct_context_terminates_on_circular_parent_ids`
  Expected: pass.

## Task 2: Product Paths Delegate To Shared Active Chain

**Files:**
- Modify: `crates/opi-coding-agent/src/session_cli.rs`
- Modify: `crates/opi-coding-agent/src/session_coordinator.rs`
- Modify: `crates/opi-coding-agent/tests/session_runtime.rs`

**Interfaces:**
- Consumes: `opi_agent::session_context::active_chain_entry_ids`
- Produces: product active entry selection that matches `reconstruct_context`

- [ ] Add failing test proving a stale final `Leaf` yields the same non-empty active chain for resume and coordinator/fork selection.
- [ ] Add failing test proving fork keeps rootless `session_info` and `label` entries when no content exists.
- [ ] Replace product-side `walk_active_branch` with delegation to `active_chain_entry_ids`, then filter to content entries where needed.
- [ ] Change `entry_on_active_chain` so rootless metadata applies when the active set is empty.
- [ ] Update stale `SessionCoordinator::open_existing` comments to describe delegation instead of duplicate semantics.
- [ ] Run: `cargo test -p opi-coding-agent --test session_runtime phase13_resume_and_fork_use_context_builder`
  Expected: pass.

## Task 3: Durable Metadata Mutations

**Files:**
- Modify: `crates/opi-coding-agent/src/harness.rs`
- Modify: `crates/opi-coding-agent/src/interactive.rs`
- Modify: `crates/opi-coding-agent/tests/session_runtime.rs`

**Interfaces:**
- Produces: `set_model_validated` performs validate -> append -> mutate
- Produces: `set_thinking_level` validates and appends before mutating runtime thinking state

- [ ] Add failing test for `set_model_validated` with a removed session file: returns `Err` and leaves `harness.model()` unchanged.
- [ ] Add failing test for `set_thinking_level` with a removed session file: returns `Err` and leaves thinking config unchanged.
- [ ] Refactor thinking into validate-only calculation plus apply step.
- [ ] Change TUI model picker selection to call `set_model_validated` and update UI state only on success.
- [ ] Run targeted tests for the two new failure cases.

## Task 4: Provider Conversion And Export Boundaries

**Files:**
- Modify: `crates/opi-coding-agent/src/harness.rs`
- Modify: `crates/opi-coding-agent/src/session_cli.rs`
- Modify: `crates/opi-coding-agent/tests/non_interactive.rs`
- Modify: `crates/opi-coding-agent/tests/session_export.rs`

**Interfaces:**
- Produces: `BranchSummary` is rendered as a synthetic provider-facing user message.
- Keeps: `Custom` is not provider-facing.

- [ ] Add failing provider-conversion test showing `BranchSummary` reaches `MockProvider` request messages.
- [ ] Add failing export test showing custom JSON data is redacted in markdown and JSON exports.
- [ ] Render `BranchSummary` in `agent_messages_to_llm`.
- [ ] Redact `AgentMessage::Custom` stringified data in both export formats.
- [ ] Run the new provider and export tests.

## Task 5: RPC, Picker, CLI, And Slash Command Polish

**Files:**
- Modify: `crates/opi-coding-agent/src/rpc.rs`
- Modify: `crates/opi-coding-agent/src/picker.rs`
- Modify: `crates/opi-coding-agent/src/cli.rs`
- Modify: `crates/opi-coding-agent/src/interactive.rs`
- Modify: relevant tests under `crates/opi-coding-agent/tests/`

**Interfaces:**
- Produces: RPC `session_info` reports read/recovery diagnostics instead of silent omission.
- Produces: `/name `, `/label `, `/unlabel ` produce local usage hints.
- Produces: `--format md` works as an alias for markdown.

- [ ] Add or update tests for empty metadata slash commands, branch picker redaction, `--format md`, and RPC recovery diagnostics.
- [ ] Use `SessionReader::read_with_recovery` in RPC `session_info`.
- [ ] Add `tree_read_error` for hard read failures and `tree_recovery` for corrupt/unknown/truncated recovery.
- [ ] Redact branch picker summaries through `RedactionMode::Summary`.
- [ ] Add clap alias `md` to `ExportFormat::Markdown`.
- [ ] Run targeted tests.

## Task 6: Documentation Synchronization

**Files:**
- Modify: `docs/opi-spec.md`
- Modify: `docs/opi-spec.zh.md`
- Modify: `docs/pi-alignment-matrix.md`
- Modify: `docs/pi-alignment-matrix.zh.md`
- Modify: `crates/opi-agent/README.md`
- Modify: `crates/opi-agent/README.zh.md`
- Modify: `crates/opi-agent/src/session.rs`
- Modify: `crates/opi-agent/src/session_context.rs`

**Interfaces:**
- Produces: docs match implemented behavior: unknown future entries are skipped/counted, BranchSummary reaches provider conversion, Custom remains deferred.

- [ ] Fix unknown future entry wording in EN/ZH docs and README surfaces.
- [ ] Fix `SessionWriter` and `SessionReader::read_all` comments.
- [ ] Update pi alignment Phase 13 status from planned to partial/partially implemented.
- [ ] Update BranchSummary docs to state generation UX is deferred but reconstructed summaries are provider-facing when present.
- [ ] Run doc guard tests.

## Task 7: Final Verification

- [ ] Run `cargo fmt --all`.
- [ ] Run `cargo test -p opi-agent --all-targets`.
- [ ] Run `cargo test -p opi-coding-agent --all-targets`.
- [ ] Run `cargo clippy --workspace --all-targets -- -D warnings`.
- [ ] Run `cargo test --workspace --doc`.
- [ ] Report any command that cannot be run or does not pass.
