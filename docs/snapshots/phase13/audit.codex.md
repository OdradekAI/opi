# Phase 13 Independent Audit

## Findings

### P1 - `branch_summary` is reconstructed but dropped before provider context

- Location: `crates/opi-agent/src/session_context.rs:222` injects `AgentMessage::BranchSummary`; `crates/opi-coding-agent/src/harness.rs:2178` documents and implements dropping `BranchSummary` in `agent_messages_to_llm`.
- Why this is a problem: Phase 13 documents `branch_summary` as an implemented context entry. `docs/opi-spec.md:922` marks it as entering LLM context, and `docs/opi-spec.md:1574` says `reconstruct_context` injects it into reconstructed LLM context. The product hooks are the actual provider conversion path, and they currently discard every non-LLM/non-compaction variant at `harness.rs:2196`.
- Impact: a resumed or forked session can show branch summaries in reconstructed `AgentMessage` tests and exports, but the next provider call will not receive that summary. This breaks the core "preserve context when leaving or forking a branch" behavior for real agent turns.
- Suggested fix: convert `AgentMessage::BranchSummary` to a synthetic provider-facing user message in `agent_messages_to_llm`, analogous to `CompactionSummary`, and add a provider-conversion test that starts with a reconstructed branch summary and asserts the mock provider receives it. If the intended behavior is not provider-facing, downgrade the docs and success criteria from "LLM context" to "transcript/export metadata" and make the deferral explicit.

### P1 - Interactive model picker bypasses durable `model_change`

- Location: `crates/opi-coding-agent/src/interactive.rs:414` handles model picker selection by calling `h.set_model(model.clone())`; `crates/opi-coding-agent/src/harness.rs:933` defines `set_model` as a direct in-memory mutation only. The durable path is `set_model_validated` at `harness.rs:944`.
- Why this is a problem: Phase 13 requires implemented metadata entries to have production append/read paths, and `docs/opi-spec.md:1571` specifically says idle `set_model_validated` appends `model_change`. The most visible interactive model-change path does not use that method.
- Impact: users switching models through the TUI model picker see `[model switched: ...]` and `/session info` reflects the in-memory model, but the session file gets no `model_change`. Resume/fork/list later reconstruct the old model metadata or none at all.
- Suggested fix: replace the picker call with `set_model_validated`, update UI state only on success, and surface the error message in the TUI if validation or persistence fails. Add an interactive/model-picker regression test that reads the session JSONL and asserts a `model_change` entry was appended without advancing the active leaf.

### P1 - Model/thinking metadata write failures are reported as success

- Location: `crates/opi-coding-agent/src/harness.rs:944` validates and changes model, but `harness.rs:951` ignores `append_model_change` errors. `crates/opi-coding-agent/src/harness.rs:987` applies thinking changes, then `harness.rs:1035` persists them through `persist_thinking_level_change`, whose `harness.rs:1047` also ignores append errors.
- Why this is a problem: the Phase 13 design requires "failed metadata writes produce errors before claiming success" at `docs/superpowers/specs/2026-06-24-phase13-session-tree-context-reconstruction-design.md:249`. RPC uses these methods directly at `crates/opi-coding-agent/src/rpc.rs:674` and `rpc.rs:711`, so it will emit a success response even if the session metadata was not written.
- Impact: disk-full, permission, or append failures can leave runtime state changed but durable session state stale. Later resume may silently revert to the previous model/thinking metadata while the command that changed it reported success.
- Suggested fix: make model/thinking persistence part of the success path. For model changes, validate, append the metadata entry, then mutate the agent model. For thinking changes, split validation/calculation from mutation so the metadata append can fail before applying the new runtime config, or explicitly roll back on append failure. Add failing-writer tests for both RPC and harness methods.

### P2 - Rootless metadata is written but not reconstructed by the shared context API

- Location: metadata writers parent entries to `self.active_tip_entry_id.clone()` (`crates/opi-coding-agent/src/session_coordinator.rs:495`, `session_coordinator.rs:514`, `session_coordinator.rs:529`, `session_coordinator.rs:554`), which is `None` before the first message. The shared metadata collector then requires `Some(parent_id)` to be in the active chain (`crates/opi-agent/src/session_context.rs:297`), so `parent_id: None` metadata is skipped. `list_sessions` consumes that shared context at `crates/opi-coding-agent/src/session_cli.rs:153`.
- Why this is a problem: `/name`, `/label`, model, and thinking changes are valid idle commands before any prompt. `SessionCoordinator::open_existing` has a special rootless path at `crates/opi-coding-agent/src/session_coordinator.rs:675`, and the test comment at `crates/opi-coding-agent/tests/session_runtime.rs:456` confirms this divergence is known, but `--list-sessions --json`, CLI resume metadata, fork filtering, and the public context API do not see those same entries.
- Impact: a metadata-only or pre-first-turn named/labeled session can show correct live metadata, then lose that metadata in list/session picker output and in reconstructed resume metadata. Forking such a session can also drop those rootless metadata entries because `entry_on_active_chain` only accepts metadata whose parent is in the active content set (`crates/opi-coding-agent/src/session_cli.rs:272`).
- Suggested fix: define empty-trunk semantics in `opi-agent::session_context`: when there is no active content chain, metadata with `parent_id: None` should apply in file order. Then remove the product-only rootless special case or make it delegate to the shared API. Add tests for name/label/model/thinking before the first prompt across `reconstruct_context`, `list_sessions --json`, resume, and fork.

### P2 - Invalid leaf fallback diverges between `SessionTree` and product branch selection

- Location: `SessionTree::active_tip` falls back to the trunk when the last leaf points at a missing entry (`crates/opi-agent/src/session_branch.rs:173`). Product code reimplements active-chain selection in `crates/opi-coding-agent/src/session_cli.rs:926`; when any leaf exists, it calls `walk_active_branch`, whose own comment says a missing tip returns an empty vector (`session_cli.rs:997`). `fork_session` uses that product helper at `session_cli.rs:216`, and `SessionCoordinator::open_existing` uses it for the compaction buffer at `crates/opi-coding-agent/src/session_coordinator.rs:158`.
- Why this is a problem: the reusable context API and the product raw-entry helper disagree on corrupt/stale leaf behavior. Resume builds the agent buffer through `reconstruct_context` and may fall back to trunk, while fork and compaction seeding can see an empty active chain for the same file.
- Impact: sessions with a stale/corrupt final `leaf` can resume with trunk messages but fork into an empty session, or resume with messages while the compaction coordinator has no matching entry buffer/usage state. The first later compaction or fork can therefore operate on different context than the agent just resumed with.
- Suggested fix: expose active-chain entry selection from `opi-agent` next to `reconstruct_context`, or have `select_ordered_entries` use `SessionTree::active_tip()` instead of raw "last leaf always wins" logic. Add tests where the last leaf references a missing entry and assert resume, fork, list, and coordinator seeding all choose the same fallback.

### P3 - Resume recovery diagnostics are duplicated

- Location: `reconstruct_context` starts with `recovery.diagnostics()` at `crates/opi-agent/src/session_context.rs:114` and returns those diagnostics at `session_context.rs:252`. The CLI resume path then builds `diagnostics = session.diagnostics` and extends it with `ctx.diagnostics` at `crates/opi-coding-agent/src/main.rs:94`. The interactive `resume_session_id` path similarly pushes `session.diagnostics` and then every `ctx.diagnostics` at `crates/opi-coding-agent/src/harness.rs:1168`.
- Why this is a problem: `session.diagnostics` is already the same recovery set that `reconstruct_context` forwards. Treating `ctx.diagnostics` as "missing-parent only" is contradicted by the implementation.
- Impact: corrupt/unknown/truncated recovery warnings appear twice in resource/RPC diagnostics and can inflate diagnostic counts. This is noisy for embedders and makes automated health checks less reliable.
- Suggested fix: either make `reconstruct_context` return only context diagnostics and keep recovery reporting outside it, or stop appending `session.diagnostics` when using a context result that already includes recovery diagnostics. Add a test with one corrupt middle entry and assert exactly one `session_corrupt_entries` payload is exposed through resume/RPC resources.

## Scope

- Inputs: `docs/snapshots/phase13/opi-impl-state.json` and `docs/superpowers/specs/2026-06-24-phase13-session-tree-context-reconstruction-design.md`.
- Diff audited: `b98c6110f14cc34e5a6c0ffeac197bdfee242046..93dd1bf13c6cabe7bb1eed18692a4753f7495399`, plus the current Phase13 snapshot commit `5b4ba674a5f1c13e1402d0202098abeb317b598b`.
- Existing audit/review reports were not opened or searched. The Phase13 state file was used to bound tasks, files, and commits; this audit does not rely on prior evaluator conclusions embedded in that state file.

## Test Coverage Gaps

- No provider-conversion test asserts `BranchSummary` reaches the mock provider; current tests stop at `AgentMessage::BranchSummary`.
- No TUI model-picker test asserts the session file receives `model_change`.
- No failing-session-writer test covers model/thinking metadata append failures.
- No shared context test covers rootless metadata before the first message.
- No product-path test covers a stale final `leaf` and compares resume/fork/coordinator behavior.
