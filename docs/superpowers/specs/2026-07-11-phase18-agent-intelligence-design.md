# Phase 18 Agent Intelligence Design

Roadmap note: this design was originally numbered Phase 16 by the 2026-07-10
roadmap. On 2026-07-28, Phase 16 was reassigned to Pluggable Extensions and
Command Execution, Phase 17 was reserved for benchmark and regression
evaluation, and Agent Intelligence moved to Phase 18. TUI and graphical
productization moved to Phase 20.

This document still synthesizes tickets T7 (skills/templates runtime), T8 (LLM
compaction + branch-summary generation), and T9 (read-tool inline image), all
resolved on 2026-07-11. The technical decisions remain intact; only roadmap
placement and downstream phase references changed.

Entry conditions:

- Phase 16 has completed its extension/command-execution exit criteria.
- Phase 17 has an approved benchmark specification and a preserved baseline.
- Phase 18 task ordering is reconciled against that baseline before
  implementation begins.

## Overview

Phase 18 closes the agent-intelligence cluster (clusters D + E) identified by
the pi-0.80.6 realignment under posture B (strategic gap-closing). It introduces
three Rust-native agent-capability lifts: a production skills/fragments runtime
that finally wires the existing-but-bypassed `disable_model_invocation` /
`expand_fragment_body` mechanisms into a pi-style `<available_skills>` system
prompt and text-expansion dispatch; an LLM-driven compaction + branch-summary
path that makes the `CompactionHooks` trait provider-backed for the first time;
and a read-tool inline-image path that makes `read` on an image path deliver a
usable multimodal content block to the model instead of rejecting it as binary.

All three subsystems are Rust-native and preserve the construction-ownership
invariant. T7 and the provider-backed T8 hook impl live entirely in
`opi-coding-agent`; the T8 trait widen + boxed-future cascade live in
`opi-agent`; the T9 image decode/resize helper lives in `opi-coding-agent` and
the T9 wire fix touches five `opi-ai` provider files.

The phase is implementation-ready: each subsystem lists concrete types,
signatures, crate placement, and verified source touch points. `opi-implement`
breaks this doc into tasks; it is not itself a task list.

## Goals

- A skills/fragments runtime that keeps the two mechanisms distinct, wires the
  existing dead code (`disable_model_invocation`, `auto_invocable`,
  `expand_fragment_body`, `FragmentRegistry::expand`) into production, and emits
  a pi-style `<available_skills>` XML section replacing the inert prose list
  concatenated into the system prompt today.
- `/skill:<name> [args]` and `/fragment:<name> [args]` text-expansion dispatch
  at every text-entry point (interactive prompt, non-interactive prompt, steer
  queue, follow-up queue), with mode-sensitive failure handling that errors on
  unknown names in interactive mode and passes through with a diagnostic
  otherwise.
- Recursive `.gitignore`/`.ignore`/`.fdignore`-aware skill discovery via the
  existing `ignore = "0.4"` workspace dependency, fragments staying
  single-level, no symlink-following.
- A widened `CompactionHooks::generate_summary` that is async and returns a
  `Result<Option<String>, AgentError>`, threaded through the call cascade as a
  boxed future, with the first provider-backed hook impl
  (`ProviderCompactionHooks`) living in `opi-coding-agent`.
- A `find_split_point` rewrite from the entry-count-fraction heuristic
  (keep last 25%, min 1) to a token-budget cut at user-message turn boundaries
  (`keep_recent_tokens = 20000`, pi parity), never splitting mid-turn.
- A `move_to` primitive on `SessionCoordinator` plus `CodingHarness::generate_branch_summary`
  and a `ForkTarget` enum on `fork_session`, populating
  `BranchSummaryMessage.entry_count` from the abandoned-tail length and
  superseding the dormant `Phase::BranchSummary` path.
- A read-tool inline-image path that decodes and resizes via the `image` crate
  (long side ≤1568px, output ≤5 MiB), emits a companion `OutputContent::Text`
  label plus `OutputContent::Image`, and fixes the 3 wire-fix sites covering 4
  providers + 2 degrade-fix sites covering 5 OpenAI-compat providers so
  `tool_result` image content-blocks reach Anthropic / Gemini / Vertex / Bedrock
  instead of today's `[image: ...]` text degradation.
- No `unsafe` block anywhere in opi code; the `image` crate is pure-Rust with no
  C dependencies.

## Non-Goals

- No skill-body argument substitution (rejected in T7 D2; bodies are wrapped and
  args are appended verbatim, matching pi's `agent-session.ts:1273-1274`;
  bash-style `$1`/`$@`/`$ARGUMENTS`/`${N:-default}` is pi's *template* mechanism
  in `prompt-templates.ts:60-101`, never applied to skills).
- No new `ignore` workspace dependency (T7 D4 premise correction: `ignore =
  "0.4"` already lives at `Cargo.toml:65` and `opi-coding-agent/Cargo.toml:28`).
  Discovery reuses the existing dep.
- No bare-name fragment dispatch (T7 D6 residual; `/translate` instead of
  `/fragment:translate` is deferred — needs a session-command-precedence rule to
  avoid colliding with `/model`, `/session`, `/name`, `/branch`, `/tree`,
  `/image`).
- No dedicated RPC `SdkCommand::Skill`/`Fragment` (T7 D6 residual; reserved
  for the Phase 19 extension surface; RPC gets text-level expansion only in
  Phase 18).
- No `readFiles`/`modifiedFiles` file-operation tracking on `CompactionEntry`
  (T8 D1 scope cut; deferred to a separately reviewed follow-up — full
  additive-entry ritual with no Phase-18 consumer; see P1 guard-awareness row:
  no new entry type is added, so
  the `KNOWN_ENTRY_TYPES` guards at
  `crates/opi-coding-agent/tests/phase13_session_context_docs.rs:143-163` and
  `crates/opi-agent/tests/session_storage.rs:810` continue to pass).
- No new `SessionEntry` variants `active_tools_change`/`custom` (T8 D1 scope cut;
  no producer for `active_tools_change`; `custom` would duplicate
  `ExtensionStateEntry`; `custom_message` is spec-deferred at
  `opi-spec.md:1599-1602`).
- No split-mid-turn compaction (T8 D6 deliberate divergence from pi; avoids
  doubling the provider-call surface and dodges the orphaned-`ToolResult`
  invariant risk; the safe failure mode keeps the whole over-budget turn).
- No `supports_image_input` capability or tool-level image gating (T9 Q7;
  fire-and-forget, with degraded providers emitting an empty image arm and
  surfacing the companion text label via the existing join-all-content
  concatenation).
- No per-model adaptive resize sizing (T9 Q3 residual; resize targets are
  compile-time constants at 1568px/5 MiB).
- No EXIF re-orientation (T9 Q8 residual; the `image` crate default does not
  apply orientation; JPEGs with Orientation ≠ 1 reach the model
  rotated/mirrored).
- No changes to provider/auth (Phase 14), sandbox/trust (Phase 15), pluggable
  command execution (Phase 16), benchmark design (Phase 17), broader extension
  work (Phase 19), or UI productization (Phase 20).

## Relationship to pi

pi keeps skills and templates as two distinct user-facing mechanisms. opi
matches this split: `skill.rs` and `prompt_fragment.rs` are already separate
modules with distinct types (`SkillManifest`/`SkillResource`/`SkillRegistry` vs
`FragmentManifest`/`FragmentResource`/`FragmentRegistry`/`FragmentArgument`).

pi builds the `<skill>` block then appends args verbatim
(`agent-session.ts:1273-1274`: `args ? \`${skillBlock}\n\n${args}\` :
skillBlock`). opi matches this exactly — no substitution on the skill body.
Bash-style positional substitution (`$1`/`$@`/`$ARGUMENTS`/`${N:-default}`)
lives only in pi's `prompt-templates.ts:60-101` `substituteArgs`, applied to
*template* bodies. opi's fragments keep the existing Rust-native `{{name}}`
declared-arg engine instead of pi's bash-style template syntax — a deliberate
§3.2 Rust-native-redesign deviation. opi's `[a-z0-9-]` skill-name rule stays
looser than pi's `name == dirname` requirement.

pi's skill discovery follows symlinks and recurses through `.gitignore`-aware
walkers. opi recurses via the `ignore` crate but does **not** follow symlinks
(safety default, deliberate divergence). pi templates are single-level; opi
fragments match.

pi emits a `<available_skills>` XML section in its system prompt; opi's
`SystemPromptBuilder` matches the format, gated on the `read` tool being
selected (the XML recommends `read` to invoke a skill). pi never references
templates in the system prompt; opi matches (fragments dropped from the prompt,
reachable via RPC metadata + `/help`).

pi uniformly passes unknown skill/fragment names through to the model in all
modes. opi diverges: interactive mode errors on unknown names (typo UX,
deliberate divergence); non-interactive/RPC/steer modes pass through with a
diagnostic (pi parity).

pi's compaction `generateSummary` is async, provider-backed, and uses
`keepRecentTokens = 20000` as the retention budget at user-message turn
boundaries — separate from the trigger `contextWindow − 16384`. opi matches the
async provider-backed shape (with a `Result` widen §9.5 mandates) and the
`keep_recent_tokens = 20000` retention budget. opi diverges on split-mid-turn:
pi issues two LLM calls with `TURN_PREFIX_SUMMARIZATION_PROMPT`; opi never
splits mid-turn (avoids doubling the provider-call surface and the
orphaned-`ToolResult` invariant risk).

pi's `move` appends a `BranchSummary` unconditionally on re-move. opi diverges:
re-move to the current tip is a no-op (INV-5), avoiding duplicate
`BranchSummary`/`Leaf` entries.

pi's read tool detects image MIME (`detectImageMimeType`) and auto-resizes
(`processImage`) so a model invoking `read` on an image path gets usable
multimodal input. opi matches via the pure-Rust `image` crate, with the
Anthropic-resample threshold (1568px) as the long-side cap and 5 MiB as the
output cap. opi additionally fixes the wire layer: 5 provider files / 3 wire
patterns degrade `tool_result` images to `[image: ...]` text today, and T9
fixes 3 sites covering 4 providers while leaving 2 degrade sites covering 5
OpenAI-compat providers (where the API genuinely cannot carry image
tool-result content).

## Load-bearing invariant

The construction-ownership invariant (Phase 14: `opi-agent` must not construct
providers or own provider/auth configuration; `opi-agent` calls
`provider.stream` at `agent_loop.rs:118` but builds nothing) extends to Phase
18's new surfaces: **`opi-agent` must not gain skill, UI, or
image-decode/resize code.** All skill/fragment logic, all image decode/resize,
and the provider-backed hook impl live in `opi-coding-agent`. The widened
`CompactionHooks` trait, the boxed-future cascade, and the new
`FsToolError::ImageDecode` variant + diagnostic code live in `opi-agent` (seam,
not decode logic: the variant carries a `message: String`, not an
`image::ImageError`, so `opi-agent` does not depend on the `image` crate); the
decode/resize helper, the read-tool early branch, and the provider-backed hook
impl live in `opi-coding-agent`.

This matches the T5/T6 deciding principle: a trait (or method) lives in
`opi-agent` only if `opi-agent`'s runtime invokes it.

- T7 — `crates/opi-agent/src` has zero `skill`/`prompt_fragment`/`SkillRegistry`/
  `FragmentRegistry` references today (verified). All dispatch, expansion,
  discovery, and `<available_skills>` emission live in `opi-coding-agent`. The
  steer/follow-up queues on `opi-agent`'s side are raw `String`
  (`loop_types.rs:47-49`), drained inside `agent_loop.rs:552-585`; every
  queue-feeding call site (`harness.rs:1736-1743`, `rpc.rs:648-663`) is in
  `opi-coding-agent` and expands before delegating. The system prompt built at
  `harness.rs:811` (before `Agent::new` at `:823`) reaches `agent_loop` with no
  `opi-agent` change.
- T8 — the `CompactionHooks` trait (`compaction.rs:98-101`) is held as `&dyn
  CompactionHooks` at `compaction.rs:139` and invoked by `CompactionEngine`'s
  `compact` (`compaction.rs:135`), which `agent_loop`/`SessionCoordinator`
  reaches. The widened signature therefore lives in `opi-agent`. The
  provider-backed impl `ProviderCompactionHooks` lives in
  `crates/opi-coding-agent/src/compaction_hooks.rs` because it needs the
  provider Arc and the summary model. `CodingAgentHooks`/`InteractiveCodingHooks`
  (`harness.rs:2258-2282`) override only `convert_to_llm` today — T8 is the
  **first provider-backed hook impl in the workspace**.
- T8 `move_to` lives on `SessionCoordinator` (`opi-coding-agent`), **not**
  `opi-agent`'s `SessionFacade`. The deciding principle: `agent_loop`/`Agent`
  never call `move_to`; the test-only `SessionFacade` has no compaction buffer
  and cannot maintain INV-4 alignment.
- T9 — image decode/resize is a tool-layer helper in `opi-coding-agent`
  (`crates/opi-coding-agent/src/tool/read.rs` early branch). The wire fix is in
  `opi-ai` (5 provider files). `opi-ai`'s `OutputContent::Image`
  (`crates/opi-ai/src/message.rs:119-127`) gains **no schema change**; it already
  carries `{source, media_type}`.

## Implementation Priority and Crate Boundaries

| Priority | Scope | Owner | Requirement |
|---|---|---|---|
| P0 | `<available_skills>` XML system-prompt section | `opi-coding-agent` | `SystemPromptBuilder::available_skills(&mut self, skills: &[SkillResource], read_tool_selected: bool)` receives `registry.auto_invocable()` (already filters `disable_model_invocation=true`; the helper's own filter is therefore belt-and-suspenders). Emits pi-style XML using `s.manifest.name`/`s.manifest.description`/`s.skill_md_path` (the manifest lives on `SkillResource` at `skill.rs:267-273`, not on `SkillManifest` at `skill.rs:95-106`), replacing the inert "Discovered skills" prose list at `harness.rs:800-809` (sections live inside `DiscoveredResourceMetadata::format_for_system_prompt` at `harness.rs:201-206`). Read-tool-gated (tools built at `harness.rs:762` before the builder runs; tool name is exactly `"read"`). Includes a trailing read-tool prose line instructing the model how to invoke a skill (see System prompt D5 for the canonical XML shape). |
| P0 | Wire `disable_model_invocation` / `auto_invocable` into the XML emission | `opi-coding-agent` | `disable_model_invocation=true` skills excluded from the XML but still `/skill:name`-dispatchable; `auto_invocable` (`skill.rs:446-451`) and `disable_model_invocation` (`skill.rs:105`) currently have zero production callers — finally take effect. |
| P0 | `/skill:<name> [args]` dispatch + body-wrap (no substitution) | `opi-coding-agent` | Load SKILL.md, wrap as `<skill name location>\nReferences relative to {baseDir}.\n\n{body}\n</skill>`, append raw args verbatim (pi `agent-session.ts:1273-1274`). |
| P0 | `/fragment:<name> [args]` dispatch + `{{name}}` expansion | `opi-coding-agent` | Wire `expand_fragment_body` (`prompt_fragment.rs:504`) + `FragmentRegistry::expand` (`:583`); missing-required → `FragmentDiscoveryError::MissingArgument`; both currently have zero production callers (only `tests/prompt_fragments.rs`). |
| P0 | Recursive `ignore`-crate skill discovery | `opi-coding-agent` | Replace single-level `discover_skills` (`skill.rs:329-372`) with recursive `.gitignore`/`.ignore`/`.fdignore`-aware walk via existing `ignore = "0.4"` dep. Fragments stay single-level (`prompt_fragment.rs:414-455`). No symlink-follow. |
| P0 | Mode-sensitive dispatch failure handling | `opi-coding-agent` | Interactive → unknown skill/fragment = diagnostic error, do not forward literal `/skill:typo`; non-interactive/RPC/steer → pass original text through + diagnostic. Missing-required-fragment-arg → diagnostic everywhere + fail expansion. Non-interactive `[PROMPT]...` construction entry point is the positional-prompt path in `main.rs` → `NonInteractiveRunner` (`runner.rs:739` region holds `NonInteractiveHooks`); it gains expand-then-forward before the prompt reaches the agent. |
| P0 | Steer/follow-up text-expansion wiring | `opi-coding-agent` | `harness.steer`/`harness.follow_up` (`harness.rs:1736-1743`) gain expand-then-forward; RPC branch `self.control.steer/follow_up` (`rpc.rs:650/658`) calls the expansion helper before delegating. Both `self.control.*` and `harness.*` branches covered. |
| P0 | Widen `CompactionHooks::generate_summary` to async boxed future + `Result` | `opi-agent` | `fn(&self, &[AgentMessage], CancellationToken) -> Pin<Box<dyn Future<Output = Result<Option<String>, AgentError>> + Send>>>`. Boxed future mandatory (dyn-compat over `&dyn CompactionHooks` at `compaction.rs:139`); `Result` honors §9.5 (`opi-spec.md:988`) visible-error-on-overflow. Mirrors `AgentHooks::transform_context` (`hooks.rs:67-73`). |
| P0 | Async cascade through `compact` (opi-agent) + cascade in opi-coding-agent | `opi-agent` + `opi-coding-agent` | (a) `opi-agent/compaction.rs:135` `CompactionEngine::compact` becomes async (+ boxed-future hook widen). (b) `opi-coding-agent/session_coordinator.rs` — `execute_compaction` (`:341`), `on_turn_end_simple` (`:351`), `run_compaction` (called at `:345`) become async. (c) `opi-coding-agent/harness.rs` — `compact_with_diagnostic` (`:1851`) and `compact` (`:1888`) become async (the published-crate **public-API break**, CHANGELOG `### Breaking Changes`). `on_turn_end` (`session_coordinator.rs:279`) stays sync (pure `should_compact` check). |
| P0 | Migrate 20 sync `#[test]` in `tests/compaction.rs` to `#[tokio::test]` | `opi-agent` | All 20 compaction tests are sync `#[test]` today (verified; 0 tokio); ~16 `.await` additions across `session_runtime.rs`/`rpc_jsonl.rs`/`sdk_embedding.rs` (counts approximate; implementation plan should grep to pin exact test names in the `split_point` cluster). |
| P0 | `Agent::provider_arc()` accessor | `opi-agent` | `Agent` already holds the Arc internally (`agent.rs:93`, constructed at `:119`); add `provider_arc(&self) -> Arc<dyn Provider>` as a trivial `Arc::clone`. `Agent::new` signature unchanged; ~15 builder sites untouched (count approximate). `Agent::provider()` (`agent.rs:233-235`) stays. |
| P0 | `SessionCoordinator::with_hooks` builder + `Arc<dyn CompactionHooks>` field | `opi-coding-agent` | `SessionCoordinator` holds `Arc<dyn CompactionHooks>` via `.with_hooks()` builder; constructor arity unchanged so ~30 test constructions are untouched. |
| P0 | `ProviderCompactionHooks` impl + `[compaction] summary_model` config (two-struct) | `opi-coding-agent` | New `crates/opi-coding-agent/src/compaction_hooks.rs`; struct `{ provider: Arc<dyn Provider>, summary_model: String, thinking: Option<ThinkingConfig> }`. Additive config in **both** `CompactionConfig` (`compaction.rs:13-16`) and the user-facing `CompactionConfigSection` (`config.rs:255-258`); TOML key `[compaction] summary_model` parsed as `Option<String>` (None → resolved chat model). `CodingHarness::build` reads `config.compaction.summary_model`; if `None`, defaults to the resolved chat-model spec at `harness.rs` ~813; parses via the same `provider:model` path as `--model`; constructs `ProviderCompactionHooks { provider: agent.provider_arc(), summary_model, thinking }` and passes it to `SessionCoordinator::with_hooks`. `DefaultCompactionHooks` stays the providerless-test fallback. |
| P0 | Rewrite `find_split_point` to token-budget + user-turn snap | `opi-agent` | Walk back from tip accumulating `chars/4` estimate; when `keep_recent_tokens` (default **20000**, pi parity, spec-reserved at `opi-spec.md:869`) exceeded, snap forward to next User-message cut; always keep last complete User-turn; no split-mid-turn. Add `keep_recent_tokens: u64` to **both** `CompactionConfig` (`compaction.rs:13-16`) AND `CompactionConfigSection` (`config.rs:255-258`) plus the `Default` impl (default 20000) and the `config.rs → CompactionConfig` conversion site; TOML key `[compaction] keep_recent_tokens = 20000`. Separate from trigger `threshold_tokens` (100_000). ~9-11 test rewrites (approximate; grep `split_point` in compaction.rs test block to pin). |
| P0 | `SessionCoordinator::move_to(entry_id, summary)` + `CodingHarness::generate_branch_summary` + `/move` parse site | `opi-coding-agent` | Idempotent re-move to current tip = no-op; `/move <entry_id>` parsed in `interactive.rs` session-command parser (parallel to the existing `/branch`, `/session`, `/name`, `/label` commands per Phase 13.4 residual memory) that resolves `<entry_id>` against the active branch and calls `harness.move_to` / `SessionCoordinator.move_to`; RPC `SdkCommand::Move` parallels it. `BranchSummaryMessage.entry_count` populated from abandoned-tail length (was the Phase-13.2 placeholder `0` at `session_context.rs:217`). Persisted `BranchSummary.parent_id == Some(entry_id)`; exactly one `Leaf` per move; total usage NOT reset. Picker UX deferred to Phase 20. |
| P0 | `fork_session` `ForkTarget` enum | `opi-coding-agent` | `ForkTarget { ActiveTip, At{entry_id}, Before{user_msg_id} }`; current `fork_session` (`session_cli.rs:210`) takes `(dir, session_id)` and returns `Result<ResumedSession, SessionCliError>` with no `entry_id` — T8(d) is a **new primitive**, not a pure refactor. New signature `fork_session(dir, session_id, target: ForkTarget) -> Result<ResumedSession, SessionCliError>` (reuses existing `ResumedSession`/`SessionCliError`; no new public types). |
| P0 | `image` crate workspace dependency | root `Cargo.toml` | `image = "<version>"` under `[workspace.dependencies]`; referenced as `image = { workspace = true }` in `opi-coding-agent/Cargo.toml`. Pure-Rust, no C deps. |
| P0 | Read-tool inline-image early branch + decode/resize | `opi-coding-agent` | Early branch in `ReadTool::execute` before `stream_read_window`; `ImageReader::open().with_guessed_format()` magic-byte detection; long side ≤1568px, output ≤5 MiB (iterative downscale), preserve `media_type`; GIF → first-frame PNG; reuse `defaults.max_image_bytes` (20 MiB) as pre-decode input guard. offset/limit ignored for images. |
| P0 | `OutputContent::Text` companion label + image arm | `opi-coding-agent` (read tool) / `opi-ai` (degrade-site arms) | `read` emits companion `OutputContent::Text { text: "image: {media_type}, {WxH}, at {path}" }` alongside `OutputContent::Image`; the read-tool Text prefix (`display_path_for_tool_result` at `read.rs:272-275`) does NOT apply to the image branch — the companion label replaces the body prefix for that branch. Fixed providers render a labeled 2-element content-block array (text block + image block); degraded providers' `Image` arm emits `""` so no double-`[image:]` noise is added; the companion Text label is already in the content Vec and surfaces via the existing join-all-content concatenation independent of the Image arm. |
| P0 | Wire-fix 3 sites / 4 providers | `opi-ai` | Anthropic content-block array (`anthropic.rs:970-998`, reusing the user-message encoder at `:923-935`); Gemini tool-result (`gemini.rs:583-615`, currently joins `[image: {}]` into `functionResponse.response.content`; the `inline_data` reuse-source lives at `gemini.rs:527/535` in the user-message encoder, not at 583-615; Vertex inherits via `vertex.rs:121`); Bedrock Converse `toolResult` bytes (`bedrock/mod.rs:944-956`). Text-only `tool_result` stays byte-identical; content-block array only when `Image` present. |
| P0 | Degrade-fix 2 sites / 5 OpenAI-compat providers | `opi-ai` | `openai_chat.rs:1280-1308` covers Chat/OpenRouter/Mistral/Azure via the shared `OpenAiChatProvider`; `openai_responses.rs:924-949` is separate (confirmed API limitation: tool messages are string-content). Empty image arm; companion label surfaces through join-all-content. |
| P0 | `ReadFileError::ImageDecode` + `FsToolError::ImageDecode` variant + diagnostic code | `opi-coding-agent` + `opi-agent` | New additive `ReadFileError::ImageDecode(image::ImageError)` at `read.rs:292` (surfaces the `image` error inside `opi-coding-agent`). New `FsToolError::ImageDecode { path: PathBuf, message: String }` in `opi-agent` at `crates/opi-agent/src/diagnostic.rs:313` (extract `image::ImageError.to_string()` rather than threading the error across the crate boundary — `opi-agent` must not depend on the `image` crate per the invariant). New diagnostic code `CODE_TOOL_IMAGE_DECODE` in `crates/opi-agent/src/diagnostic.rs` alongside `CODE_TOOL_BINARY_FILE` (mirrors the `BinaryFile { path: PathBuf }` arm at `:327`). The private `ReadFileError` alone does not reach the model — the `FsToolError` variant is the model-facing surface. |
| P1 | `phase13_session_context_docs.rs` + `session_storage.rs` guard awareness | `opi-coding-agent` + `opi-agent` | The additive-entry/`KNOWN_ENTRY_TYPES` policy is enforced by TWO guards: docs-level name-presence at `crates/opi-coding-agent/tests/phase13_session_context_docs.rs:143-163` and source-level round-trip at `crates/opi-agent/tests/session_storage.rs:810` (`known_entry_types_match_session_entry_serde_tags`). T8 does **not** add a new entry type, but D7's `move_to` populates `BranchSummaryMessage.entry_count` from the abandoned-tail length — both guards continue to pass. |

Phase 18 must not satisfy acceptance with the abstract traits alone. Each P0
item needs a production path: T7 from config through `SystemPromptBuilder` /
dispatch into the model context, exercised by skill/fragment dispatch tests;
T8 from `[compaction] summary_model` through `SessionCoordinator.with_hooks` /
`Agent::provider_arc` into `CompactionEngine::compact`, exercised by
`MockProvider`-backed hook tests (never a real LLM API); T9 from a `tempfile`
image fixture through `ReadTool::execute` into the 5 provider wire-format
builders, exercised by `wiremock`-free unit tests on the decoder and provider
content-block constructors.

## Design

### T7 — Skills/templates runtime

**Split.** Skills and fragments stay two distinct mechanisms (pi's model).
`skill.rs` and `prompt_fragment.rs` are already separate modules with distinct
types (`SkillManifest`/`SkillResource`/`SkillRegistry` vs
`FragmentManifest`/`FragmentResource`/`FragmentRegistry`/`FragmentArgument`).

**Skill body/args (D2).** `/skill:<name> [args]` loads the `SKILL.md` body and
wraps it:

```text
<skill {name} {location}>
References relative to {baseDir}.

{body}
</skill>
```

The raw args are appended verbatim after the closing tag. There is **no
substitution** on the body. This mirrors pi's `agent-session.ts:1273-1274`
exactly (`args ? \`${skillBlock}\n\n${args}\` : skillBlock`). Bash-style
positional substitution (`$1`/`$@`/`$ARGUMENTS`/`${N:-default}`) lives only in
pi's `prompt-templates.ts:60-101` `substituteArgs`, applied to template bodies
never skills. The T7(c) framing conflated the two; the design honors pi's
split.

**Fragment args (D3).** Fragments keep opi's existing Rust-native `{{name}}`
declared-arg engine. The currently-dead `expand_fragment_body`
(`prompt_fragment.rs:504`) and `FragmentRegistry::expand` (`:583`) are wired
into production; `FragmentManifest.arguments` stays; a missing required
argument produces `FragmentDiscoveryError::MissingArgument`. Both functions
have zero production callers today (only `tests/prompt_fragments.rs`
constructs them). This is a deliberate §3.2 Rust-native deviation from
pi-template bash syntax.

**Discovery (D4).** Skills move to **recursive +
`.gitignore`/`.ignore`/`.fdignore`-aware** discovery via the existing `ignore =
"0.4"` workspace dep (`Cargo.toml:65`, `opi-coding-agent/Cargo.toml:28` —
verified **not** new, correcting the T7(d) premise). The current single-level
`discover_skills` (`skill.rs:329-372`) is replaced. Fragments stay
**single-level** (`prompt_fragment.rs:414-455` unchanged, mirroring pi
templates). **No symlink-follow** — opi safety default, deliberate pi
divergence. opi's looser `[a-z0-9-]` skill-name rule stays (pi's
`name == dirname` requirement is not adopted).

```rust
// crates/opi-coding-agent/src/skill.rs (replacement for discover_skills)
// The existing helpers are SkillManifest::from_skill_md (skill.rs:113) and
// discover_skill_dir (skill.rs:374); the loop below reuses them. The existing
// layer type is crate::resource::DiscoveryLayer (renamed here to ResourceLayer
// would be a new alias — keep the real name).
pub fn discover_skills(
    layers: &[crate::resource::DiscoveryLayer],
    trust_filter: &dyn Fn(&Path) -> TrustDecision,
) -> Vec<SkillManifest> {
    let mut out = Vec::new();
    for layer in layers {
        if matches!(trust_filter(&layer.root), TrustDecision::Untrusted) {
            continue; // T6 precondition: untrusted project layers skipped
        }
        let mut walker = ignore::WalkBuilder::new(&layer.root)
            .hidden(false)
            .ignore(true)
            .git_ignore(true)
            .git_global(false)
            .git_exclude(true)
            .parents(false)
            .follow_links(false) // opi safety default
            .build();
        while let Some(Ok(entry)) = walker.next() {
            if entry.file_type().is_some_and(|t| t.is_dir())
                && entry.path().join("SKILL.md").exists()
            {
                // Reuse the existing SkillManifest::from_skill_md parser
                // (skill.rs:113); trust resolution is the caller's job.
                if let Some(manifest) =
                    SkillManifest::from_skill_md(&entry.path().join("SKILL.md")).ok()
                {
                    out.push(manifest);
                }
            }
        }
    }
    out
}
```

**System prompt (D5).** `SystemPromptBuilder` emits a pi-style
`<available_skills>` XML block for skills, **replacing** the inert prose
"Discovered skills" / "Discovered prompt fragments" sections currently
concatenated into the system prompt. Those sections live inside
`DiscoveredResourceMetadata::format_for_system_prompt` at `harness.rs:201-206`
(verified: `context_files.rs` is only ~100 lines and contains no such sections;
the task-summary phrasing "context_files :201-206" was ambiguous — the
sections are at `harness.rs:201-206`). `harness.rs:800-809` feeds
`format_for_system_prompt()` into `builder.context_files()`.

The canonical emitted shape (opi's, matching pi's `<available_skills>` format),
including the trailing read-tool prose line promised in Goals:

```text
<available_skills>
  <skill name="{name}" location="{absolute SKILL.md path}">
    {description}
  </skill>
  ...
</available_skills>

Use the `read` tool with the skill's `location` path to load the full skill
body before relying on it.
```

`disable_model_invocation = true` skills are excluded from the XML but remain
`/skill:<name>`-dispatchable. The currently-dead `disable_model_invocation`
(`skill.rs:105`) and `auto_invocable` (`skill.rs:446-451`) flags finally take
effect. **Fragments are dropped from the system prompt** (pi-template parity —
pi's `system-prompt.ts` never references templates); fragments stay reachable
via RPC `list-fragments` and `/help` — implementation plan must verify both
code paths exist; if either is missing today, add "enumerate fragments in
`/help` + RPC `list-fragments`" as a P1 row.

```rust
// crates/opi-coding-agent/src/prompt.rs (new helper on SystemPromptBuilder).
// Input type is &[SkillResource] (not &[SkillManifest]) because skill_md_path
// lives on SkillResource (skill.rs:267-273); the manifest is reachable as
// s.manifest (name/description/disable_model_invocation).
impl SystemPromptBuilder {
    pub fn available_skills(
        &mut self,
        skills: &[SkillResource],
        read_tool_selected: bool,
    ) -> &mut Self {
        if !read_tool_selected || skills.is_empty() {
            return self;
        }
        let visible: Vec<&SkillResource> = skills
            .iter()
            .filter(|s| !s.manifest.disable_model_invocation)
            .collect();
        if visible.is_empty() {
            return self;
        }
        let mut xml = String::from("<available_skills>\n");
        for s in visible {
            xml.push_str(&format!(
                "  <skill name=\"{}\" location=\"{}\">\n    {}\n  </skill>\n",
                s.manifest.name,
                s.skill_md_path.display(),
                s.manifest.description,
            ));
        }
        xml.push_str("</available_skills>");
        xml.push_str(
            "\n\nUse the `read` tool with the skill's `location` path to load the full skill body before relying on it.",
        );
        self.append_section("Available skills", xml);
        self
    }
}
```

**Dispatch (D6).** `/skill:<name> [args]` and `/fragment:<name> [args]` are a
**single text-expansion function** applied at every text-entry point. The
`/fragment:` colon prefix is chosen for **consistency with `/skill:`** and to
avoid colliding with opi's existing session slash-command set (`/model`,
`/session`, `/name`, `/branch`, `/tree`, `/image`, ...). Bare-name fragment
dispatch (pi's template UX, `/translate` instead of `/fragment:translate`) is
**deferred** (needs a session-command-precedence rule).

Text-entry points (all in `opi-coding-agent`):

1. Interactive prompt submission (in `interactive.rs`).
2. Non-interactive `[PROMPT]...` construction — the positional-prompt path in
   `main.rs` feeding `NonInteractiveRunner` (`runner.rs:739` region holds
   `NonInteractiveHooks`); expand before the prompt reaches the agent.
3. `harness.steer` / `harness.follow_up` at `harness.rs:1736-1743` — currently
   forward raw `String` to `opi-agent`; they gain expand-then-forward.
4. RPC running-mode branch `self.control.steer` / `self.control.follow_up` at
   `rpc.rs:650/658` — currently bypasses the harness; must call the expansion
   helper before delegating (or route through `harness.steer`/`harness.follow_up`).
   Both `self.control.*` and `harness.*` branches must be covered.

```rust
// crates/opi-coding-agent/src/dispatch.rs (new)
pub enum ExpansionOutcome {
    Expanded(String),
    UnknownPassThrough { original: String, diagnostic: Diagnostic },
    Failed { original: String, diagnostic: Diagnostic },
}

pub fn expand_text_command(
    text: &str,
    registry: &DispatchRegistry,
    mode: RunMode,
) -> ExpansionOutcome {
    // parse leading /skill:<name> or /fragment:<name> token + remainder
    // dispatch against registry.skills / registry.fragments
    // honor mode-sensitive failure handling (D7)
}
```

RPC gets **text-level expansion only** in Phase 18. A dedicated
`SdkCommand::Skill`/`Fragment` is deferred to the Phase 19 extension surface.

**Failure handling (D7 — deliberate opi divergence).** Mode-sensitive:

- **Interactive** → unknown skill/fragment name produces a diagnostic error;
  the literal `/skill:typo` is **not** forwarded to the model. (pi uniformly
  passes through; opi's interactive-error is a deliberate divergence for typo
  UX.)
- **Non-interactive / RPC / steer** → original text passes through, plus a
  diagnostic. (pi parity.)
- **Missing-required-fragment-arg** → diagnostic everywhere, expansion fails
  (no half-expanded body).
- **Body-load IO failure** → diagnostic, expansion fails.

**Invariant.** All skill/fragment logic lives in `opi-coding-agent`.
`crates/opi-agent/src` has zero `skill`/`prompt_fragment`/`SkillRegistry`/
`FragmentRegistry` references (verified). The steer/follow-up queues are raw
`String` (`loop_types.rs:47-49`) drained inside `agent_loop.rs:552-585`;
`opi-agent` has no expansion hook and needs none.

**T6 precondition.** T7 depends on Phase 15 T6: the trust gate must filter
`layers.skills`/`layers.fragments` at `discover_resources` time
(`harness.rs:775/2041/2057`) so an untrusted project's skills/fragments cannot
resolve via `/skill:`/`/fragment:`. Phase 14 → 15 → 16 → 17 → 18 sequencing
places T6 ahead of T7.

**Scope boundary.** T7 delivers skill/fragment dispatch, expansion, recursive
discovery, `<available_skills>` XML emission with `disable_model_invocation`
wired, mode-sensitive failure handling, and steer/follow-up wiring.

### T8 — LLM compaction + branch-summary generation

**Scope (D1).** (a) async LLM-summary hook + (b) token-budget split + (d)
`move_to` + branch-summary generation are IN Phase 18. (c)
`readFiles`/`modifiedFiles` on `CompactionEntry` and (e) new `SessionEntry`
variants (`active_tools_change`/`custom`) remain future follow-ups (see P1
guard-awareness row: no new entry type is added, so the `KNOWN_ENTRY_TYPES`
guards at `phase13_session_context_docs.rs:143-163` and
`session_storage.rs:810` continue to pass; full additive-entry ritual reserved
for a separately reviewed revival) — no Phase-18 consumer, and both are additive-on-v1
so future revival reads old JSONL cleanly.

**Signature (D2).** The current sync signature at `compaction.rs:98-101`:

```rust
pub trait CompactionHooks: Send + Sync {
    fn generate_summary(&self, messages: &[AgentMessage]) -> Option<String>;
}
```

widens to:

```rust
use std::pin::Pin;
use std::future::Future;
use tokio_util::sync::CancellationToken;
use crate::AgentError;

pub trait CompactionHooks: Send + Sync {
    fn generate_summary(
        &self,
        messages: &[AgentMessage],
        signal: CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<Option<String>, AgentError>> + Send>>;
}
```

Three load-bearing decisions in the signature:

1. **Boxed future, not native `async fn` in trait.** The hook is held as `&dyn
   CompactionHooks` at `compaction.rs:139`. Native `async fn` in trait is not
   dyn-compatible; the existing `AgentHooks::transform_context` precedent at
   `hooks.rs:67-73` uses the same `Pin<Box<dyn Future<...> + Send>>` shape for
   exactly this reason.
2. **`Result`, not bare `Option`.** §9.5 (`opi-spec.md:988`) mandates a visible
   error on overflow. A bare `Option` would swallow provider failures during
   `CompactionReason::Overflow` and silently drop history. `Ok(None)` means the
   hook declines and the core summary generator runs.
3. **`CancellationToken` threaded through.** The token passed to
   `hooks.generate_summary` is the **same** `AgentLoopContext.cancel` /
   `request.cancel` token already raced against `provider.stream` at
   `agent_loop.rs:128-140` (T14 footgun #1 cross-ref), forwarded from the
   compact call site — not a newly-constructed token. Mirrors the agent-loop
   cancel semantics; the provider-backed impl races the LLM call against
   cancellation.

`DefaultCompactionHooks` returns `Box::pin(async { Ok(None) })` (plain block,
no captures — `async move` would be a no-op closure) and stays the
providerless-test fallback.

**Injection (D3).** `SessionCoordinator` (in `opi-coding-agent`) holds
`Arc<dyn CompactionHooks>` via a `.with_hooks()` builder. Constructor arity is
unchanged, so ~30 test constructions are untouched. The provider-backed impl
lives in a new module:

```rust
// crates/opi-coding-agent/src/compaction_hooks.rs
use std::sync::Arc;
use opi_ai::provider::Provider;
use opi_agent::CompactionHooks;
use tokio_util::sync::CancellationToken;

pub struct ProviderCompactionHooks {
    pub provider: Arc<dyn Provider>,
    pub summary_model: String,
    pub thinking: Option<opi_ai::request::ThinkingConfig>,
}

impl CompactionHooks for ProviderCompactionHooks {
    fn generate_summary(
        &self,
        messages: &[opi_agent::AgentMessage],
        signal: CancellationToken,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<Option<String>, opi_agent::AgentError>,
                > + Send,
        >,
    > {
        let provider = Arc::clone(&self.provider);
        let model = self.summary_model.clone();
        let thinking = self.thinking.clone();
        let msgs = messages.to_vec();
        Box::pin(async move {
            // build opi_ai::Request with system = SUMMARY_SYSTEM_PROMPT,
            // messages = compacted_msgs, model, max_tokens, thinking
            // race provider.stream(req) against signal
            // accumulate AssistantStreamEvent text
            // Ok(Some(text)) on success; Err(...) on failure
            //   (surfaced via §9.5 visible-error-on-overflow)
            todo!()
        })
    }
}
```

Additive config (two-struct obligation, both `summary_model` and
`keep_recent_tokens` follow this pattern):

```toml
[compaction]
summary_model = "anthropic:claude-sonnet-4-5-20250514"  # Option<String>; None → resolved chat model
keep_recent_tokens = 20000                              # u64; default 20000 (pi parity)
```

Wiring path: `CodingHarness::build` reads `config.compaction.summary_model`
(`Option<String>`); if `None`, defaults to the resolved chat-model spec at
`harness.rs` ~813; parses via the same `provider:model` path as `--model`;
constructs `ProviderCompactionHooks { provider: agent.provider_arc(),
summary_model, thinking }` and passes it to `SessionCoordinator::with_hooks`.

This is the **first provider-backed hook impl in the workspace.**
`CodingAgentHooks`/`InteractiveCodingHooks` (`harness.rs:2258-2282`) override
only `convert_to_llm` today; `CompositeHooks` (`extension.rs:567`) is a
dispatcher delegating to base+extensions, no provider; `NonInteractiveHooks`
(`runner.rs:739`) overrides `convert_to_llm` + `before_tool_call`
mutating-gate, no provider. T8(a) is genuinely the first.

**Arc accessor (D4).** `Agent` already holds the provider as an `Arc` internally
(`agent.rs:93` field, `Arc::from` at `:119` construction). Add:

```rust
// crates/opi-agent/src/agent.rs
impl Agent {
    pub fn provider_arc(&self) -> Arc<dyn Provider> {
        Arc::clone(&self.provider)
    }
}
```

Minimal cascade. `Agent::new` signature is unchanged; ~15 builder sites are
untouched (count approximate). `Agent::provider()` (`agent.rs:233-235`, returns
`&dyn Provider`) stays. (Rejected: widening the boundary to `Arc<dyn Provider>`
at ~15 sites — speculative; defer until T2/T14 is a concrete driver.)

**Async cascade (D5).** Split by crate: (a) **`opi-agent/compaction.rs`** — only
`CompactionEngine::compact` (`:135`) becomes async (plus the boxed-future hook
widen). (b) **`opi-coding-agent/session_coordinator.rs`** —
`execute_compaction` (`:341`), `on_turn_end_simple` (`:351`), and
`run_compaction` (called at `:345`) become async; `on_turn_end` (`:279`) stays
sync (pure `should_compact` check; no provider call). (c)
**`opi-coding-agent/harness.rs`** — `compact_with_diagnostic` (`:1851`) and
`compact` (`:1888`) become async (the published-crate **public-API break**,
CHANGELOG `### Breaking Changes` with migration guidance: add `.await`, switch
`#[test]` → `#[tokio::test]`). `crates/opi-agent/tests/compaction.rs` has 20
sync `#[test]` (verified — not tokio) which migrate to `#[tokio::test] async`;
~16 `.await` additions land across `session_runtime.rs`/`rpc_jsonl.rs`/
`sdk_embedding.rs` (counts approximate).

```rust
// crates/opi-agent/src/compaction.rs
impl CompactionEngine {
    pub async fn compact(
        &self,
        entries: &[Entry],
        reason: CompactionReason,
        hooks: &dyn CompactionHooks,
        signal: CancellationToken,
    ) -> Result<CompactionOutput, CompactionError> {
        // ... unchanged split + token accounting ...
        let (summary_text, source) = match hooks.generate_summary(&compacted_messages, signal).await {
            Ok(Some(s)) => (s, SummarySource::Hook),
            Ok(None) => (
                generate_core_summary(&compacted_messages),
                SummarySource::Core,
            ),
            Err(e) => {
                // §9.5 visible-error-on-overflow: surface, do not silently drop
                return Err(CompactionError::SummaryFailed(e));
            }
        };
        // ... unchanged output assembly ...
    }
}
```

**find_split_point (D6).** The current heuristic at `compaction.rs:184-201`
keeps the last 25% of entries (minimum 1). It is replaced with a token-budget
cut at user-message turn boundaries. Add `keep_recent_tokens: u64` (default
**20000**, pi parity; spec-reserved at `opi-spec.md:869`) to **both**
`CompactionConfig` (`compaction.rs:13-16`) AND `CompactionConfigSection`
(`config.rs:255-258`) plus the `Default` impl (default 20000) and the
`config.rs → CompactionConfig` conversion site; TOML key
`[compaction] keep_recent_tokens = 20000`. Separate from the trigger
`threshold_tokens` (100_000). The premise correction is load-bearing: pi's
`keepRecentTokens = 20000` is the **retention** budget, distinct from the
trigger `contextWindow − 16384`. They are not the same knob.

```rust
// crates/opi-agent/src/compaction.rs
fn find_split_point(
    entries: &[Entry],
    keep_recent_tokens: u64,
) -> usize {
    // Walk back from the tip accumulating chars/4 token estimate.
    // When the budget is exceeded, snap FORWARD to the next User-message cut
    // (so the split lands at a User-turn boundary).
    // Always keep the last complete User-turn.
    // No split-mid-turn (deliberate pi divergence).
    // Safe failure mode: keep the whole over-budget turn.
    todo!()
}
```

~9-11 test rewrites (the `split_point_keeps_tail` cluster + neighboring tests
pinning the 25% heuristic; counts approximate, grep to pin).

**move_to (D7).** `move_to(entry_id, summary)` lives on
`SessionCoordinator` (`opi-coding-agent`) — **not** `opi-agent`'s
`SessionFacade`. The `SessionFacade` is test-only and has no compaction buffer,
so it cannot maintain INV-4 alignment between the buffer and the new agent
messages. The deciding principle (same as T5/T6): a method lives in
`opi-agent` only if `opi-agent`'s runtime invokes it. `agent_loop`/`Agent`
never call `move_to`.

LLM generation lives in `CodingHarness::generate_branch_summary`, a sibling to
the D3 hook impl. `fork_session` (`session_cli.rs:210`) gains a `ForkTarget`:

```rust
// crates/opi-coding-agent/src/session_cli.rs
// Current signature (verified):
//   pub fn fork_session(dir: &Path, session_id: &str) -> Result<ResumedSession, SessionCliError>
// New signature reuses ResumedSession + SessionCliError (no new public types):
pub enum ForkTarget {
    /// Fork at the active tip (current behavior — forks whole active chain).
    ActiveTip,
    /// Fork at a specific entry id.
    At { entry_id: String },
    /// Fork at the entry just before a given user-message id.
    Before { user_msg_id: String },
}

pub fn fork_session(
    dir: &Path,
    session_id: &str,
    target: ForkTarget,
) -> Result<ResumedSession, SessionCliError> {
    // ActiveTip preserves current behavior (whole active chain at active_tip).
    // At/Before are new primitives — current fork_session takes no entry_id.
    // Error contract:
    //   At { entry_id } / Before { user_msg_id } return SessionCliError for
    //   unknown ids or ids not on the active chain (EntryNotFound /
    //   NotOnActiveChain); Before at the root user message forks an empty
    //   prefix (rejected: return EntryNotFound so callers handle root
    //   explicitly rather than silently producing an empty fork).
    todo!()
}
```

`Phase::BranchSummary` (`crates/opi-agent/src/harness.rs:494-505`, variants
`begin_branch_summary`/`end_branch_summary`) is left dormant — variant stays
`pub` (removing it is a breaking API change; only the production call site is
removed/never added). Doc comment reads
`// Superseded by SessionCoordinator::move_to (Phase 18 T8 D7); retained for API compatibility.`
`BranchSummaryMessage` (`crates/opi-agent/src/message.rs:34`) is reused — no
new entry type (§9.5 additive-on-v1 policy honored).

**move_to behavior (D8).** Idempotent re-move to the current tip is a **no-op**
(INV-5). pi appends unconditionally; opi diverges to avoid duplicate
`BranchSummary`/`Leaf` entries. `/move <entry_id>` is parsed in `interactive.rs`'s
session-command parser (parallel to the existing `/branch`, `/session`, `/name`,
`/label` commands per Phase 13.4 residual memory); the parser resolves
`<entry_id>` against the active branch and calls
`harness.move_to`/`SessionCoordinator.move_to`. RPC `SdkCommand::Move` lands in
Phase 18; the picker UX is deferred to Phase 20 (UI productization).
`BranchSummaryMessage.entry_count` is populated from the abandoned-tail length
(was the Phase-13.2 placeholder `0` at `session_context.rs:217`).

Invariants:

- Persisted `BranchSummary.parent_id == Some(entry_id)` — the new tip.
- Exactly one `Leaf` per move.
- `reconstruct_context == new_agent_messages` (buffer alignment).
- Total usage NOT reset.

**Completed spec retarget (D9).** The paired roadmap now assigns
`branch_summary` generation to Phase 18 and keeps interactive `/export` in
Phase 20 UI productization (terminal command-surface, not Agent Intelligence).
It lists the Phase 16 extension foundation, the deferred Phase 17 benchmark,
this Phase 18 design, the Phase 19 extension follow-up, and Phase 20 UI work.
The documentation and filename retarget is complete. Its spec-hash drift is
reconciled later through the guarded `opi-implement plan` path; the ledger is
not hand-edited while authoring this design.

Binding spec mandates the design honors:

- §9.5 (`opi-spec.md:988`) — record summary source Core-vs-Hook
  (`SummarySource` preserved at `compaction.rs:90-91`); visible error on
  overflow (D2's `Result`).
- `:869` — `keep_recent_tokens` reserved (D6 sanctioned).
- `:922` — `branch_summary` additive-on-v1, reused by `move_to` (no new entry
  type).

**Premise corrections** (ticket framing errors surfaced by the deep pass):

- T8(d) is NOT a "pure refactor" — `fork_session` forks the whole active chain
  at the tip with no `entry_id` param; T8(d) is a new primitive + a new
  `ForkTarget` enum.
- "opi-agent's coordinator" was ambiguous — two coordinators exist
  (`SessionCoordinator` in `opi-coding-agent`, `SessionFacade` in
  `opi-agent`); `move_to` → `SessionCoordinator`.
- No `Agent::provider_arc()` exists today (`Agent::provider()` at
  `agent.rs:233-235` returns `&dyn Provider`).
- §9.5 mandates `Result` not bare `Option`.
- Boxed future is mandatory (not preferential) for dyn-compat.
- T8(a) is the first provider-backed hook in the workspace.
- 20 compaction tests are sync `#[test]` (not tokio).
- pi `keepRecentTokens = 20000` is retention, separate from trigger.
- The async cascade is NOT entirely in `opi-agent`: only
  `CompactionEngine::compact` (`compaction.rs:135`) lives there;
  `run_compaction`/`execute_compaction`/`on_turn_end_simple` live in
  `opi-coding-agent/session_coordinator.rs` (`:341/351/345`), and the 2
  public-API-break fns live in `opi-coding-agent/harness.rs` (`:1851/1888`).

**Guard-attribution correction** (the additive-entry/`KNOWN_ENTRY_TYPES`
policy is enforced by TWO guards in TWO files):

1. Docs-level name-presence guard at
   `crates/opi-coding-agent/tests/phase13_session_context_docs.rs:143-163` —
   pins the entry **names** (`session_info`, `model_change`,
   `thinking_level_change`, `label`, `branch_summary`) in spec + README docs.
   It does NOT read `session.rs` source and does NOT reference
   `KNOWN_ENTRY_TYPES`.
2. Source-level round-trip guard at
   `crates/opi-agent/tests/session_storage.rs:810`
   (`known_entry_types_match_session_entry_serde_tags`) — exercises
   `KNOWN_ENTRY_TYPES` (`session.rs:515-525`) directly, round-tripping all 9
   `SessionEntry` variants (`session.rs:197-208`) through serde.

T8 does **not** add a new entry type, but the design honors both guard layers.

**Scope boundary.** T8 delivers the async `CompactionHooks` widen,
`Agent::provider_arc`, `SessionCoordinator.with_hooks` + `ProviderCompactionHooks`,
the token-budget `find_split_point`, `SessionCoordinator.move_to` +
`CodingHarness.generate_branch_summary` + `ForkTarget`, and the completed
roadmap retarget.

### T9 — Read-tool inline image

**Delivery + wire scope (Q1).** Inline: `read` returns image content and the
wire-layer `tool_result` image serialization fix is **in T9 scope**. The
ticket's "reuse the existing image-input attachment path" was incomplete —
that path is **user-message-only** (`image.rs` → `InputContent::Image` →
`pending_images`) and **5 provider files / 3 wire patterns** degrade
`tool_result` images to `[image: ...]` text today:

- `crates/opi-ai/src/anthropic.rs:976-978`
- `crates/opi-ai/src/gemini.rs:589-591`
- `crates/opi-ai/src/bedrock/mod.rs:952-953`
- `crates/opi-ai/src/openai_chat.rs:1286-1288`
- `crates/opi-ai/src/openai_responses.rs:930-931`

All five emit exactly `format!("[image: {}]", media_type.as_str())`. Staged-via-
`pending_images` was rejected — next-turn semantics break read-then-reason.

**Premise correction (citation).** The T9 Question preamble's claim that
`read.rs:317-327` "returns text only today" cites the wrong line range. The
range `317-327` is the binary-NUL detection loop inside `stream_read_window`
(`if chunk.contains(&0u8) { saw_nul = true; }`). The actual text-only return is
`read.rs:275` — `Ok(result::ok(vec![OutputContent::Text { text }], details))`
in the success branch of `ReadTool::execute`. The text-return path has not
moved into the `317-327` window. Cite `read.rs:275` for the text-only-return
premise.

**Processing (Q2).** Decode + resize via the pure-Rust `image` crate (new
`[workspace.dependencies]` entry, used by `opi-coding-agent`). Format detection
folds into decode — `ImageReader::with_guessed_format()` does magic-byte
detection, so MIME and extension reliance are both sidestepped. No `infer`
crate, no extension reliance. Pure-Rust, no C deps (§3.2).

**Resize targets (Q3) + format→MediaType mapping (Q8).** Compile-time
constants:

- Long side ≤ **1568px** (Anthropic's resample threshold; under OpenAI ~2000
  and Gemini 3072).
- Output ≤ **5 MiB** (iterative downscale to fit).
- Preserve `media_type`.
- GIF → first-frame PNG.

Post-decode re-encoding maps `image::ImageFormat` to `opi_ai::MediaType` via an
explicit helper `reencode_to_supported(img, source_format) -> (MediaType, Vec<u8>)`:

| `image::ImageFormat` source | Re-encode target | `MediaType` |
|---|---|---|
| Jpeg | preserve JPEG | `image/jpeg` |
| WebP | preserve WebP | `image/webp` |
| Png / Gif / Bmp / Tiff / others | re-encode PNG (GIF → first-frame PNG) | `image/png` |

**Integration (Q4).** Early branch in `ReadTool::execute` before
`stream_read_window`. `ImageSource::Bytes` (verified at
`crates/opi-ai/src/message.rs:87-88`) carries **only** `{ data: Vec<u8> }`; each
provider's wire encoder base64-encodes at serialize time (e.g. `anthropic.rs:929-932`
already does this for `InputContent::Image`), so the read tool must **not**
pre-encode — pass raw bytes and mirror the existing encoder path for the
tool_result content-block arrays at the 3 wire-fix sites:

```rust
// crates/opi-coding-agent/src/tool/read.rs (early branch in execute)
async fn try_read_image(
    path: &Path,
    max_input_bytes: u64,
) -> Result<Option<Vec<OutputContent>>, ReadFileError> {
    let meta = tokio::fs::metadata(path).await.map_err(ReadFileError::Io)?;
    if meta.len() > max_input_bytes {
        return Ok(None); // fall through to text path
    }
    let reader = match image::ImageReader::open(path)
        .ok()
        .and_then(|f| f.with_guessed_format().ok())
    {
        Some(r) => r,
        None => return Ok(None),
    };
    let source_format = reader.format();
    let img = match reader.decode() {
        Ok(img) => img,
        Err(e) => return Err(ReadFileError::ImageDecode(e)),
    };
    let (w, h) = (img.width(), img.height());
    let resized = resize_to_fit(img, MAX_LONG_SIDE, MAX_OUTPUT_BYTES);
    let (media_type, bytes) = reencode_to_supported(resized, source_format);
    // ImageSource::Bytes carries { data: Vec<u8> } only (verified at
    // crates/opi-ai/src/message.rs:87-88); provider wire encoders base64-
    // encode at serialize time, so do NOT pre-encode here.
    let source = opi_ai::message::ImageSource::Bytes { data: bytes };
    let label = format!("image: {}, {}x{}, at {}", media_type.as_str(), w, h, path.display());
    Ok(Some(vec![
        OutputContent::Text { text: label },
        OutputContent::Image { source, media_type },
    ]))
}
```

`offset`/`limit` are ignored for images. Processing is a **tool-layer** helper;
FS bytes flow through the T5 `FileOperations` backend.

**T5 dependency / sequencing.** Phase 15 T5 has shipped the `FileOperations`
substrate. T9 must use that existing injected filesystem seam for image bytes;
it must not reintroduce a direct `tokio::fs` fallback. Phase 15 therefore
remains a completed prerequisite rather than a conditional migration.

**Wire fix (Q5).** Fix **3 wire-fix sites covering 4 providers** by reusing the
existing base64 inline encoder:

- **Reuse source**: the user-message `InputContent::Image` encoder at
  `anthropic.rs:923-935`. For Gemini, the reuse source is the user-message
  `inline_data` encoder at `gemini.rs:527/535` (snake_case key).
- **Fix targets**:
  - Anthropic content-block array at `anthropic.rs:970-998` (currently emits
    `format!("[image: {}]", media_type.as_str())` at `:976-978`).
  - Gemini tool-result at `gemini.rs:583-615` (currently joins
    `[image: {}]` text into `functionResponse.response.content`; the
    `inline_data` snake_case key lives at the reuse-source `:527/535`, NOT at
    `583-615`). Vertex inherits via `vertex.rs:121` delegation.
  - Bedrock Converse `toolResult` bytes at `bedrock/mod.rs:944-956`.

Concrete before/after for the Anthropic tool_result arm (the canonical 3-site
pattern):

```text
BEFORE (paraphrase of anthropic.rs:971-998 today): join OutputContent into a
single string, emitting `[image: {mt}]` for Image arms:
  {"type":"tool_result","tool_use_id":"...","content":"{joined-text-with-[image: ...]}"}

AFTER (when any Image is present): emit a content-block array:
  {"type":"tool_result","tool_use_id":"...",
   "content":[
     {"type":"text","text":"..."},
     {"type":"image","source":{"type":"base64","media_type":"image/png","data":"..."}}
   ]}

AFTER (text-only): byte-identical joined-string shape (Phase 11.9 invariant).
```

Per-provider array keys (the array shape differs across the 3 sites):

- Anthropic: `content` array of `{type:"text",...}` / `{type:"image", source:{type:"base64",...}}`.
- Gemini: `functionResponse.response.content` with `inline_data: {mimeType, data}` snake_case (per reuse source `:527/535`).
- Bedrock Converse: `toolResult.content` array of `{text: ...}` / `{image: {format: "png", source: {bytes: "..."}}}` per AWS `ToolResultContentBlock`.

**Backward-compat.** Text-only `tool_result` stays the joined-string
byte-identical shape (Phase 11.9 invariant, `anthropic.rs:987-990`). The
content-block array is emitted **only when an `Image` is present** (same
conditional at Gemini/Bedrock).

**Degrade-fix 2 sites covering 5 OpenAI-compat providers (Q5 cont.).**

- `openai_chat.rs:1280-1308` covers Chat/OpenRouter/Mistral/Azure via the
  shared `OpenAiChatProvider` (verified: `mistral.rs`/`openrouter.rs`/
  `azure_openai.rs` all wrap `OpenAiChatProvider::new_for_profile`).
- `openai_responses.rs:924-949` is separate (confirmed API limitation: tool
  messages are string-content; the Responses API genuinely cannot carry image
  tool-result content today).

**Degraded-provider placeholder (Q5′ — corrects Q5's infeasible wire-layer
rich placeholder).** The wire site cannot build a rich placeholder:
`OutputContent::Image` (`crates/opi-ai/src/message.rs:119-127`) carries only
`{source, media_type}` — no dims/path. The resolution: `read` emits a
**companion `OutputContent::Text`** (`"image: {media_type}, {WxH}, at {path}"`)
alongside the `Image` (Q4 helper above). The read-tool Text prefix
(`display_path_for_tool_result` at `read.rs:272-275`) does NOT apply to the
image branch — the companion label replaces the body prefix for that branch.

Fixed providers iterate the content Vec and render a 2-element content-block
array (text block + image block). Degraded providers' `Image` arm emits `""`
so no double-`[image:]` noise is added; the companion Text label emitted by
the read tool is already in the content Vec and surfaces via the existing
join-all-content concatenation independent of the Image arm — the wire layer
does not synthesize or "surface" the label, it simply does not suppress the
sibling Text block. No `OutputContent` schema change. No regression: the
`Image` variant has **zero producers today**, and `image_tool_results.rs:160-183`
pins the session serde shape and is untouched.

**Config (Q6).** **Always-on, no new knob.** Reuse
`defaults.max_image_bytes` (20 MiB; `DEFAULT_MAX_IMAGE_BYTES` defined at
`image.rs:8`, field at `config.rs:40`, default-wired at `config.rs:51`) as the
pre-decode input guard. Checked **before** `decode()` to avoid OOM on a huge
file.

**Gating (Q7).** **Fire-and-forget.** No `supports_image_input` T3 extension.
Q5′ handles the provider-wire case. Text-only-on-fixed-provider is limited to
**4 Mistral models today** (`supports_images=false`, verified at
`mistral.rs:37/46/55/64`) — covered by the fog entry.

**Errors / format breadth (Q8).** New additive `ReadFileError::ImageDecode`
at `read.rs:292`:

```rust
enum ReadFileError {
    Io(std::io::Error),
    BinaryFile,
    UnsupportedEncoding { byte_offset: usize },
    ImageDecode(image::ImageError),  // new — opi-coding-agent-side only
}
```

plus a matching new `FsToolError::ImageDecode` variant + diagnostic code in
`opi-agent` (NOT threaded as `image::ImageError` across the crate boundary —
`opi-agent` must not depend on the `image` crate per the invariant):

```rust
// crates/opi-agent/src/diagnostic.rs:313 (enum home — NOT read.rs)
pub enum FsToolError {
    // ... existing variants ...
    BinaryFile { path: PathBuf },                                    // mirror at :327
    /// Image decode/re-encode failure. Substrate variant; decode lives in
    /// opi-coding-agent. Carries the user-facing message, not the
    /// `image::ImageError`, so opi-agent does not depend on the `image` crate.
    ImageDecode { path: PathBuf, message: String },                  // new
}

// Diagnostic code (alongside CODE_TOOL_BINARY_FILE at :352):
pub const CODE_TOOL_IMAGE_DECODE: &str = "OPI_0XXX_TOOL_IMAGE_DECODE"; // new
```

The read tool extracts `e.to_string()` from the `image::ImageError` and
crosses into `opi-agent` as the `message: String` field. The private
`ReadFileError` alone does not reach the model — the `FsToolError` variant is
the model-facing surface. Accept the `image` crate's default decode set
(PNG/JPEG/GIF/WebP/BMP/TIFF/...); re-encode to a supported `MediaType`
per the Q3 mapping. Non-image binary is unchanged (NUL → `BinaryFile`
fallthrough).

**Compaction parity note (non-blocking, not drift).** `opi-agent`'s
`extract_text` in `compaction.rs:220-254` degrades BOTH
`InputContent::Image` (`:227-229`) and `OutputContent::Image` (`:248-250`) to
`[image: {}]` text — this is a 6th degrade *consumer* but **not** a 6th
provider wire file, so the map's "5 provider files / 3 wire patterns" framing
(provider-wire-layer scoped) remains accurate. If the Phase 18
image-content-block fix is applied to the 5 provider sites, the compaction
text-render path should be revisited for parity: today it would still emit
`[image: ...]` text even after the provider fix, since it produces compacted
transcript text not wire blocks.

**Scope boundary.** T9 delivers the read-tool inline-image early branch, the
`image` crate workspace dep, the decode/resize helper, the companion label +
image arm, the 3-site/4-provider wire fix, the 2-site/5-provider degrade fix,
and the new error variants.

## Sequencing

T9 is the substrate-independent leaf: the `image` crate dep, the read-tool
early branch, and the wire fixes are all self-contained. It reads through the
shipped Phase 15 `FileOperations` seam; direct `tokio::fs` fallback is not
permitted.

T7 and T8 both depend on Phase 15 T6 (the project-trust gate filtering
`layers.skills`/`layers.fragments` at `discover_resources` time), already
sequenced ahead by the 14 → 15 → 16 → 17 → 18 ordering.

T8 is internally sequenced: D2 (signature widen) + D5 (async cascade) land
together (one atomic migration of the 20 sync tests + the 2 public-API
breaks); D3 (`SessionCoordinator.with_hooks` + `ProviderCompactionHooks`) +
D4 (`Agent::provider_arc`) land next; D6 (`find_split_point` rewrite) is
independent and can proceed in parallel; D7/D8 (`move_to` + `ForkTarget` +
`BranchSummaryMessage.entry_count`) land last.

T7 is internally sequenced: D4 (recursive discovery) lands first (substrate
for D5's XML emission); D5 (XML) + the `disable_model_invocation` wiring land
next; D2/D3 (skill body-wrap + fragment `{{name}}` expansion) land together
(the dispatch helpers); D6 (steer/follow-up wiring) + D7 (mode-sensitive
failure handling) land last.

Phase 18 depends on the Phase 17 benchmark baseline as an entry gate, but has no
hard dependency on Phase 20 UI productization. Phase 20 may eventually want a
`/skill:<name>` picker widget, but Phase 18 ships typed-id `/skill:<name>`
dispatch only.

## Cross-ticket interactions

- **T7 depends on Phase 15 T6 trust filtering.** T6 must filter
  `layers.skills`/`layers.fragments` at `discover_resources` time
  (`harness.rs:775/2041/2057`) so an untrusted project's skills/fragments
  cannot resolve via `/skill:`/`/fragment:`. T7's `discover_skills`
  (`skill.rs:329-372`) consumes the trust-filtered layers and does not
  re-implement trust resolution itself.
- **T9 uses the shipped Phase 15 T5 `FileOperations`.** Image reads traverse
  the existing injected filesystem seam; no direct `tokio::fs` fallback or
  later migration is permitted.
- **T8 is the FIRST provider-backed hook impl in the workspace.**
  `CodingAgentHooks`/`InteractiveCodingHooks` (`harness.rs:2258-2282`) override
  only `convert_to_llm` today. T8's `ProviderCompactionHooks` is the first
  hook impl that calls a provider at runtime.
- **T8 async-cascades across THREE crates**: only `CompactionEngine::compact`
  (`compaction.rs:135`) in `opi-agent`; `run_compaction`/`execute_compaction`/
  `on_turn_end_simple` (`session_coordinator.rs:341/345/351`) in
  `opi-coding-agent`; the 2 PUBLIC fns `compact_with_diagnostic`/`compact`
  (`harness.rs:1851/1888`) in `opi-coding-agent` — a published-crate
  public-API break (CHANGELOG `### Breaking Changes`); ~20 sync `#[test]` in
  `tests/compaction.rs` migrate to `#[tokio::test] async`. `on_turn_end`
  (`session_coordinator.rs:279`) stays sync. The public-API break must be
  documented in the CHANGELOG with migration guidance (add `.await`, switch
  `#[test]` → `#[tokio::test]`).
- **T8 `move_to` lives on `SessionCoordinator` (`opi-coding-agent`), NOT
  `opi-agent` `SessionFacade`** (test-only, no compaction buffer). The
  deciding principle (a method lives in `opi-agent` only if `opi-agent`'s
  runtime invokes it) is the same principle T5/T6 established for traits.
- **T8 roadmap retarget.** `branch_summary` generation is assigned to Phase
  18, while interactive `/export` stays in Phase 20. The paired documentation
  and design filename are already retargeted. Reconcile the resulting
  spec-hash drift only through `opi-implement plan`; do not hand-edit the
  ledger.
- **T9 + T8 compaction parity.** T9 fixes 3 wire-fix sites (4 providers) +
  2 degrade-fix sites (5 OpenAI-compat providers) = 5 provider wire sites
  total in `opi-ai`; the `opi-agent` `compaction.rs` `extract_text`
  (`:227-229` + `:248-250`) still degrades both `InputContent::Image` and
  `OutputContent::Image` to `[image: {}]` text. Revisit for parity after T9
  lands (not blocking; compaction produces transcript text not wire blocks).
- **T7 + T8 both honor the construction-ownership invariant.** T7's skill/
  fragment logic stays entirely in `opi-coding-agent`; T8's widened trait +
  boxed future live in `opi-agent` (because `agent_loop` invokes
  `CompactionHooks`) but the provider-backed impl + `generate_branch_summary`
  live in `opi-coding-agent`. T9's image-decode logic lives in
  `opi-coding-agent`; only the `FsToolError::ImageDecode` seam (carrying a
  `String`, not `image::ImageError`) lives in `opi-agent`.

## Residuals / follow-ups

- **Dedicated RPC `SdkCommand::Skill`/`Fragment`** (T7 D6) — deferred to the
  Phase 19 extension surface; RPC gets text-level expansion only in Phase 18.
- **Bare-name fragment dispatch** (T7 D6, `/translate` instead of
  `/fragment:translate`) — fog; needs a session-command-precedence rule.
- **Skill name-validation strictness** (T7 D4) — opi's looser `[a-z0-9-]` rule
  stays; pi's `name == dirname` not adopted.
- **`/help` + RPC `list-fragments` enumeration** (T7 D5) — verify both code
  paths exist; if either is missing, add as a P1 row.
- **(c) `readFiles`/`modifiedFiles` on `CompactionEntry`** (T8 D1) — future;
  not substrate for a/b/d; full additive-entry ritual with no Phase-18
  consumer.
- **(e) `active_tools_change` + `custom` `SessionEntry` variants** (T8 D1) —
  future; no producer for `active_tools_change`; `custom` would
  duplicate `ExtensionStateEntry`; `custom_message` is spec-deferred at
  `:1599-1602`.
- **Full pi split-mid-turn** (T8 D6, two LLM calls,
  `TURN_PREFIX_SUMMARIZATION_PROMPT`) — fog; deliberate D6 simplification.
- **`/move` picker UX** (T8 D8) — Phase 20 UI productization; D8 ships typed-id +
  RPC only.
- **`ForkTarget::Before { user_msg_id }` at root user message** — rejected
  (returns `EntryNotFound`) so callers handle root explicitly rather than
  silently producing an empty fork.
- **`supports_image_input` capability + tool-level gating** (T9 Q7) — fog;
  graduate if a text-only opi model becomes common (Mistral today).
- **Provider-adaptive resize sizing** (T9 Q3) — fog; per-model dims.
- **APNG / animated WebP multi-frame extraction** (T9 Q8) — fog; the `image`
  crate default first-frame-collapses both (parity with explicit GIF →
  first-frame PNG).
- **EXIF orientation** (T9 Q8) — the `image` crate default does not apply
  orientation; JPEGs with Orientation ≠ 1 reach the model rotated/mirrored.
  Accept in Phase 18 or add EXIF re-orientation (via `kamadak-exif`) as a
  follow-up.
- **Bedrock `toolResult` image is model-gated** (Amazon Nova + Claude 3/4
  only, per AWS `ToolResultContentBlock`) — non-Nova/Claude Bedrock models
  need degradation even though the wire supports it; conservatively degrade or
  gate on model id.
- **Responses-API (`openai_responses.rs:924-949`) degrade is conservative** —
  re-verify against current OpenAI docs before locking; the most likely of the
  5 to gain native image tool-result support.
- **`compaction.rs` `extract_text` image-render parity** (lines `:227-229`
  InputContent::Image + `:248-250` OutputContent::Image) — revisit after T9
  lands.
- **Roadmap rewrite.** Completed with the Phase 18 design filename update.
  The guarded `opi-implement plan` reconciliation remains a separate
  pre-implementation action because `opi-spec.md` and the registered source
  hashes changed.

## Out of scope (cross-ref map)

- Skill-body argument substitution (pi-template mechanism; rejected in T7 D2).
- Provider/auth changes (Phase 14), sandbox/trust changes (Phase 15),
  pluggable command execution (Phase 16), benchmark design (Phase 17),
  broader extension work (Phase 19), and UI productization (Phase 20).
- New `SessionEntry` variants `active_tools_change`/`custom` (future; no
  Phase-18 producer).
- `readFiles`/`modifiedFiles` tracking on `CompactionEntry` (future; not
  substrate for the a/b/d Phase-18 surface).
- Split-mid-turn compaction (deliberate T8 D6 divergence).
- `supports_image_input` capability gating (T9 Q7; fog).
- EXIF re-orientation, APNG/animated-WebP multi-frame extraction (T9 Q8; fog).
- Bare-name fragment dispatch, dedicated RPC `SdkCommand::Skill`/`Fragment`
  (T7 D6; Phase 19 / T11).
