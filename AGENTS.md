# AGENTS.md

This file provides guidance to Codex (Codex.ai/code) when working with code in
this repository.

## Project

`opi` is a Rust reimplementation of selected ideas from
[earendil-works/pi](https://github.com/earendil-works/pi), organized as an AI
agent toolkit and terminal-first coding agent.

Current workspace version: `0.7.1`. The repository may contain unreleased
changes on top of that version; check `CHANGELOG.md` before making release or
documentation claims.

The current implementation includes:

- A working `opi` coding-agent binary produced by `opi-coding-agent`.
- Interactive ratatui TUI mode with model/session/branch/tree pickers and
  terminal image rendering.
- Non-interactive text mode, `--json` NDJSON mode, and `--rpc` JSONL mode.
- Eight built-in tools: `read`, `write`, `edit`, `bash`, `grep`, `find`, `ls`,
  `glob`.
- Mode-aware tool defaults and tool selection flags: `--tools`, `--no-tools`,
  `--no-builtin-tools`, and `--allow-mutating`.
- Image attachments through `--image` and the TUI `/image` command.
- Multi-provider streaming through Anthropic, OpenAI Chat Completions, OpenAI
  Responses, OpenRouter, Mistral, Gemini, AWS Bedrock, Azure OpenAI, Google
  Vertex AI, GitHub Copilot (one static catalog routed across Anthropic Messages
  and OpenAI Completions/Chat/Responses wires), and OpenAI Codex (dedicated
  Responses wire).
- Config-driven OpenAI-compatible provider profiles and custom provider/model
  registration through the provider registry.
- TOML config with layered precedence, per-provider proxy config, image limits,
  thinking/retry/compaction defaults, keybindings/themes, and mutating-tool
  defaults.
- Session JSONL persistence with list/resume/fork/delete CLI commands, active
  branch `parent_id` links, `leaf` pointers, compaction entries, and extension
  state restore.
- AGENTS.md / CLAUDE.md context file loading from workspace ancestors and the
  user config directory.
- Context compaction, retry/backoff, usage accumulation, shell completion
  generation, edit diff rendering, best-effort cost tracking, diagnostics, and
  opt-in redacted trace envelopes.
- Top-level `opi doctor` local health checks and `opi package
  add/remove/list/doctor` package management.
- Extension hooks/tools/state for embedders, config-driven resource discovery
  for extensions, packages, skills, prompt fragments, and themes, and
  `process-jsonl` package adapters.

Repository: https://github.com/OdradekAI/opi

`CLAUDE.md` is the Claude Code-flavored sibling of this file. When project
rules change, update both in lockstep to avoid drift.

Normative design references live in `docs/`: `opi-spec.md` is the technical
spec. Consult it before answering scope or behavior questions. Fresh alignment
audits against upstream are produced under `docs/realign/` by the `opi-realign`
skill.

## Conversational style

- Keep answers short and concise.
- No emojis in commits, issues, PR comments, or code.
- No fluff or cheerful filler text.
- Technical prose only; be kind but direct.
- When the user asks a question, answer it first before making edits or running
  commands.

## Operating principles

These rules bias toward caution over speed. For trivial tasks, use judgment.

- Think before editing. State assumptions that affect the outcome. If
  requirements have multiple reasonable interpretations, present them instead
  of choosing silently. If something is unclear, stop, name the uncertainty,
  and ask.
- Prefer the minimum change that solves the request. Do not add features,
  abstractions, configurability, or error handling for impossible cases unless
  the user asked for them.
- Push back when a simpler approach exists or the requested path would add
  unnecessary complexity. Self-test: if a 200-line change could be 50, rewrite
  it; if a senior engineer would call it overcomplicated, simplify.
- Make surgical changes. Every changed line should trace directly to the
  user's request. Do not refactor, reformat, or improve adjacent code that is
  outside the task.
- Clean up only changes you caused. Remove imports, variables, functions,
  tests, or docs made unused by your work; mention unrelated dead code instead
  of deleting it.
- Transform tasks into verifiable goals, then loop until the check passes:
  "add validation" -> write tests for invalid inputs, then make them pass;
  "fix the bug" -> write a test that reproduces it, then make it pass;
  "refactor X" -> confirm tests pass before and after. For multi-step work,
  state each step as `step -> verify: <check>` and loop until it passes or
  state exactly what remains unverified. Strong criteria ("tests pass") let you
  loop independently; weak ones ("make it work") force clarification loops.

## Code quality

- Read files in full before making wide-ranging changes, before editing files
  you have not already fully inspected, and when the user asks you to
  investigate or audit something. Do not rely only on search snippets for broad
  changes.
- Always ask before removing functionality or code that appears intentional.
- Do not preserve backward compatibility unless the user explicitly asks for
  it.
- Avoid `unsafe` unless absolutely necessary; prefer safe abstractions.
- Prefer `thiserror` for library error types, `anyhow` only in binary/test
  code.
- Use workspace dependencies. Never add a version directly to a crate's
  `Cargo.toml` if it can go through `[workspace.dependencies]`.
- Trait objects (`Box<dyn T>`) are fine at crate boundaries; prefer generics
  within a crate when the concrete type is known at compile time.
- Match the existing module's style. If a file uses `thiserror`, do not switch
  to manual `impl Display + Error`.
- When updating documentation that has a localized counterpart such as
  `README.zh.md` or `docs/*.zh.md`, update the localized counterpart in the
  same change or explicitly state why it does not need synchronization.

## Workspace layout

Cargo workspace with lockstep versioning. All crates share
`version.workspace = true`. Internal dependencies flow through
`[workspace.dependencies]` in the root `Cargo.toml`:

```text
opi-ai      (no internal deps)        - multi-provider LLM API
opi-tui     (no internal deps)        - terminal UI widgets, pickers, diff and image rendering
opi-agent   -> opi-ai                 - agent runtime, tool calling, sessions, compaction
opi-coding-agent -> opi-ai, opi-agent, opi-tui - produces the `opi` binary
```

Adding a new internal dependency means updating `[workspace.dependencies]` in
root `Cargo.toml`, then referencing it as `foo = { workspace = true }` in the
consumer crate's `Cargo.toml`.

When publishing internal crates, path dependencies must also carry a `version`
field. Bare path dependencies cannot be published to crates.io.

## Architecture

The `opi` binary is produced by `opi-coding-agent`. Startup flow:

- Shell completion generation (`--generate-completion <SHELL>`) is handled
  before config/provider construction and then exits.
- Package commands (`opi package add/remove/list/doctor`) are handled before
  provider construction.
- Top-level `opi doctor` is handled before provider construction, is local and
  network-free by default, and reports config/provider/package/session/tui/rpc
  diagnostics.
- Model listing (`--list-models`, optionally with `--json`) resolves config,
  lists models advertised by configured providers, and then exits.
- Session commands (`--list-sessions`, `--delete-session`) are handled before
  full provider construction and then exit.
- `--resume <ID>` loads a JSONL session, reconstructs the active branch, and
  then continues in interactive, non-interactive, or RPC mode.
- `--fork <ID>` copies the source session's active branch into a new JSONL
  session whose `parent_session` points at the source, then continues from the
  fork.
- Tool selection is resolved from `--no-tools`, `--tools`,
  `--no-builtin-tools`, run mode, and mutating-tool opt-in.
- RPC mode (`--rpc`) builds a provider and `CodingHarness`, then runs the
  unstable JSONL command/event protocol over stdin/stdout.
- Non-interactive mode is selected by non-empty positional `[PROMPT]...`,
  `--non-interactive`, or `--json`. It builds a provider, runs
  `NonInteractiveRunner`, prints stdout/stderr or NDJSON, and exits with a
  numeric code.
- Interactive mode is the default with no prompt args. It builds a
  `CodingHarness` with `InteractiveCodingHooks` and launches
  `interactive::run_interactive_tui()`.

Both interactive and non-interactive modes use the same core loop from
`opi-agent`:

```text
agent_loop()
  -> transform_context
  -> convert_to_llm
  -> validate request capabilities
  -> stream provider response
  -> accumulate AssistantStreamEvent values
  -> detect tool calls
  -> validate args with JSON Schema
  -> run before_tool_call hook
  -> execute tools in parallel or sequential batches
  -> run after_tool_call hook
  -> check terminate flags and should_stop_after_turn
  -> prepare_next_turn
  -> poll steering/follow-up queues
  -> repeat
```

Key abstractions in `opi-agent`:

- `AgentHooks`: six hook methods: `transform_context`, `convert_to_llm`,
  `before_tool_call`, `after_tool_call`, `should_stop_after_turn`,
  `prepare_next_turn`.
- `Tool`: `definition()` returns JSON schema, `execute()` runs the tool,
  `execution_mode()` controls parallel vs sequential batching.
- `SessionWriter` / `SessionReader`: append-only JSONL session storage with
  crash recovery.
- `CompactionEngine`: threshold/manual/overflow compaction primitives with hook
  support.
- `AgentSessionEvent`: session-level event protocol used by JSON mode.
- `sdk`: shared SDK/RPC command, response, and event types.
- `extension`: lifecycle hooks, custom tools, custom commands, event observers,
  extension state, custom providers, and model overrides.

Provider implementations live in `opi-ai`:

- `anthropic:` uses Anthropic Messages streaming.
- `openai:` uses OpenAI Chat Completions streaming.
- `openai-responses:` uses OpenAI Responses streaming.
- `openrouter:` uses an OpenAI-compatible OpenRouter profile.
- `mistral:` uses an OpenAI-compatible Mistral profile.
- `gemini:` uses Gemini `streamGenerateContent?alt=sse`.
- `bedrock:` uses AWS Bedrock Converse streaming with SigV4 signing.
- `azure:` uses Azure OpenAI Chat Completions deployments.
- `vertex:` uses Google Vertex AI Gemini streaming.
- `github-copilot:` one audited static catalog routed through Anthropic Messages,
  OpenAI Completions/Chat, and OpenAI Responses; OS keychain via `/login
  github-copilot`.
- `openai-codex:` dedicated OpenAI Codex Responses wire; OS keychain via `/login
  openai-codex` (Browser default or Device Code).

Config resolution for model selection:

1. `--model`
2. `OPI_MODEL` only when `--config` was not passed
3. `model` in `--config <FILE>`
4. Project `.opi/config.toml`
5. User config
6. Built-in defaults

TOML layers merge user -> project -> `--config`. Model specs use
`provider:model` format, for example
`anthropic:claude-sonnet-4-5-20250514`, `openai:gpt-4o`, or
`gemini:gemini-2.5-flash`.

The user config path is `%APPDATA%\opi\config.toml` on Windows and
`~/.config/opi/config.toml` on Unix. Global context files (`AGENTS.md`,
`CLAUDE.md`) live in the same user config directory. `OPI.md` is intentionally
not loaded.

Provider credentials are configurable per provider. Defaults include
`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `OPENROUTER_API_KEY`, `MISTRAL_API_KEY`,
`GEMINI_API_KEY`, `AZURE_OPENAI_API_KEY`, and `VERTEX_ACCESS_TOKEN`; Bedrock
uses AWS SigV4 credentials from env/config/profile sources. GitHub Copilot and
OpenAI Codex persist OAuth tokens in the native OS keychain (Windows Credential
Manager, macOS Keychain Services, Freedesktop Secret Service) through the
interactive TUI `/login <provider>` and `/logout <provider>` commands, with env
API-key fallback for the first-class providers; their canonical ids are
`github-copilot` and `openai-codex` (the development ids `copilot` and `codex`
have no alias or migration). `main()` calls
`dotenvy::dotenv()` at startup, so a local `.env` may change provider behavior.
Both `.env` and `.opi/config.toml` are gitignored and may carry live keys or a
non-default `base_url`.

Wire versions embedders must respect:

- NDJSON mode: `NDJSON_SCHEMA_VERSION == 2`.
- RPC / streaming proxy: `SDK_SCHEMA_VERSION == 3`.
- Trace envelopes: `TRACE_SCHEMA_VERSION == 1`.
- Process JSONL adapters: `protocol == "opi-extension-jsonl-v1"`.

## Built-in tool policy

Available built-in tools are `read`, `write`, `edit`, `bash`, `grep`, `find`,
`ls`, and `glob`.

Default active tools:

| Mode | Tools |
|------|-------|
| Interactive | `read`, `write`, `edit`, `bash` |
| Non-interactive / RPC | `read`, `grep`, `find`, `ls`, `glob` |
| Non-interactive / RPC with mutating opt-in | `read`, `write`, `edit`, `bash` |

File writes and edits are restricted to the harness workspace root.
Non-interactive file tools remain workspace-root scoped by default.
Interactive `read` can inspect absolute paths and paths outside the workspace.
`bash` runs with the workspace root as its initial cwd but is not path-confined.
These are tool-policy checks, not an operating-system sandbox.

In non-interactive/RPC mode, `write`, `edit`, and `bash` require
`--allow-mutating` or `defaults.allow_mutating_tools = true`. Interactive mode
enables the mutating default tool set.

## Edition

Workspace is on Rust edition 2024; the declared MSRV is Rust 1.97.

## Commands

```sh
# Build everything
cargo build
cargo build --release

# Run the CLI binary
cargo run -p opi-coding-agent             # interactive TUI
cargo run -p opi-coding-agent -- --help
cargo run -p opi-coding-agent -- --version
cargo run -p opi-coding-agent -- --list-models
cargo run -p opi-coding-agent -- --generate-completion powershell
cargo run -p opi-coding-agent -- doctor
cargo run -p opi-coding-agent -- package list

# Non-interactive examples
cargo run -p opi-coding-agent -- "Summarize this workspace"
cargo run -p opi-coding-agent -- --json "Summarize this workspace"
cargo run -p opi-coding-agent -- --image screenshot.png "Review this UI"
cargo run -p opi-coding-agent -- --tools read,grep "Inspect without edits"

# Tests
cargo test --workspace --all-targets
cargo test --workspace --doc
cargo test -p opi-ai
cargo test -p opi-ai -- some_test_name
cargo test -p opi-agent --test sdk_embedding

# Lint and format (CI and release gates)
cargo fmt --all
cargo fmt --check --all
cargo clippy --workspace --all-targets -- -D warnings

# Docs with warnings as errors (release gate)
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

After code changes (not documentation-only), run
`cargo clippy --workspace --all-targets -- -D warnings` and fix all warnings
before committing.

If you create or modify a test file, run that test and iterate until it passes.

## Testing

- Tests live in `crates/<crate>/tests/` for integration tests and inline
  `#[cfg(test)]` modules for unit tests.
- Use `opi_ai::test_support::MockProvider` for agent/harness integration tests.
  Never hit a real LLM API or require API keys in tests.
- For provider wire-format tests, use fixtures or `wiremock`; do not require
  external network access.
- For tool tests that touch the filesystem, use `tempfile::tempdir()` and build
  fixtures in the temp directory.
- For session tests, use isolated temp directories or `OPI_SESSIONS_DIR` to
  avoid user data and test races.
- Tests that mutate process environment variables such as `OPI_SESSIONS_DIR`
  must be serialized.
- For snapshot/UI tests, follow the existing `insta` snapshot pattern in
  `opi-tui`. Do not auto-accept snapshot updates without explicit review.
- Run the relevant test after writing it: `cargo test -p <crate> -- <test_name>`.

## Git rules

### Committing

- NEVER commit unless the user asks.
- ONLY commit files YOU changed in THIS session.
- NEVER use `git add -A` or `git add .`; these sweep up changes from other
  agents or unrelated working-tree state.
- ALWAYS use `git add <specific-file-paths>` listing only files you modified.
- Before committing, run `git status` and verify you are only staging YOUR
  files.
- Always include `fixes #<number>` or `closes #<number>` in the commit message
  when there is a related issue.
- NEVER include `Co-Authored-By` trailers in commit messages. No
  `Co-Authored-By: Codex ...` or similar.

### Forbidden git operations

These commands can destroy work:

- `git reset --hard` - destroys uncommitted changes
- `git checkout .` - destroys uncommitted changes
- `git clean -fd` - deletes untracked files
- `git stash` - stashes ALL changes including other agents' work
- `git add -A` / `git add .` - stages other agents' uncommitted work
- `git commit --no-verify` - bypasses required hooks and is never allowed
- `git push --force` - can overwrite shared history

### Safe workflow

```bash
# 1. Check status
git status

# 2. Add ONLY your specific files
git add crates/opi-ai/src/anthropic.rs
git add crates/opi-ai/tests/anthropic_test.rs

# 3. Commit (Conventional Commits format)
git commit -m "fix(opi-ai): handle CRLF in SSE parser"

# 4. Push (pull --rebase if needed, but NEVER reset/checkout)
git pull --rebase && git push
```

### If rebase conflicts occur

- Resolve conflicts in YOUR files only.
- If conflict is in a file you did not modify, abort and ask the user.
- NEVER force push.

## PR workflow

- Analyze PRs without pulling locally first.
- Work in feature branches until everything meets requirements, then merge
  into main and push.
- Never open PRs unless the user explicitly asks.

## Changelog

Location: `CHANGELOG.md` at the repo root (single changelog for the whole
workspace).

Format is [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) with
sections:

- `### Breaking Changes`
- `### Added`
- `### Changed`
- `### Fixed`
- `### Removed`

Rules:

- New entries ALWAYS go under `## [Unreleased]`; create this section if it does
  not exist.
- NEVER modify already-released version sections.
- Each released version section is immutable.

## Releasing

Releases go to both GitHub Releases and crates.io via the `opi-release` skill
at `.claude/skills/opi-release/skill.md`. Invoke with a target semver version,
for example `0.5.3`.

Critical properties:

- Phases 1-4 are reversible.
- Phase 5 pushes a commit/tag and is publicly visible.
- Phase 6 publishes to crates.io and is irreversible; crates can only be
  yanked, never deleted.
- All publishable crates use the same version and publish in dependency order
  computed from `cargo metadata`.
- Never use `git reset --hard` plus `git push --force` for rollback. Use
  `git revert` plus tag deletion.
- Interrupted releases can resume from `.opi-release-state.json` at the repo
  root.
- CI-driven binary builds are recommended: push the tag, then `release.yml`
  builds all six platform targets and uploads them to the GitHub Release.

The design rationale is in
`docs/superpowers/specs/2026-05-19-opi-release-skill-design.md`.

## Implementation workflow

Long-running spec implementations track state in `.opi-impl-state.json` at the
repo root, driven by the `opi-implement` skill. The canonical ledger is tracked
in Git; temporary, draft, candidate, backup, and corrupt copies are not. Do not
delete or hand-edit the canonical file; use the skill's guarded atomic-write
flow to query, advance, recover, or reset progress.

When the user authorizes an implementation task commit, commit task-owned work
first with its `Opi-*` footers, then record that commit SHA in the ledger and
create a separate ledger-checkpoint commit. Blocked handoffs, phase-exit
updates, and reviewed graph reconciliation are also durable checkpoint
boundaries. Before removing a worktree, refuse cleanup unless its canonical
ledger is clean and every required ledger checkpoint is contained in the
destination branch.

The skill runs `scripts/opi-impl-smoke.{sh,ps1}` at Phase A.3. That smoke check
bundles `cargo build`, `cargo fmt --check --all`,
`cargo clippy --workspace --all-targets -- -D warnings`, and
`cargo test --workspace --all-targets`.

Reviewed supplemental implementation specs for phases 5-14 are registered in
`.claude/skills/opi-implement/skill.md`; do not treat arbitrary
`docs/superpowers/specs/` files as normative without that registry.

## CI

Two GitHub Actions workflows live in `.github/workflows/`:

- `ci.yml`: runs on push/PR to `main`. Jobs: `fmt`, `clippy`, `test`,
  `doctest` (`cargo test --workspace --doc`), and `doc`.
- `release.yml`: triggered by `v*` tags or manual `workflow_dispatch`. Builds
  `opi` for linux-x64, linux-arm64, darwin-x64, darwin-arm64, windows-x64, and
  windows-arm64.

## Conventions

- Conventional Commits drive changelog categorization (`feat:` -> Added,
  `fix:` -> Fixed, `feat!:` / `BREAKING CHANGE` -> Breaking Changes).
- Each crate's `description`, `license`, and `repository` come from the
  workspace; do not duplicate them per crate.
- The CLI binary is named `opi` (defined by `[[bin]]` in
  `crates/opi-coding-agent/Cargo.toml`), not `opi-coding-agent`.
- Runtime expansion of prompt fragments, production sub-agent workflows,
  permission gates, plan/todo workflows, and MCP workflows are example/package
  patterns, not built-in core product workflows.

## User override

If the user's instructions conflict with rules set out here, ask for
confirmation that they want to override the rules. Only then execute their
instructions.
