# opi-coding-agent

[![Crates.io](https://img.shields.io/crates/v/opi-coding-agent.svg)](https://crates.io/crates/opi-coding-agent)
[![Docs.rs](https://docs.rs/opi-coding-agent/badge.svg)](https://docs.rs/opi-coding-agent)

> The `opi` binary and embeddable coding harness.

[Simplified Chinese](README.zh.md) | [opi workspace](../../README.md)

## Status

Current crate version: `0.6.4`, inherited from the workspace package version.

This crate connects `opi-ai`, `opi-agent`, and `opi-tui` into a terminal coding
agent. It provides:

- the `opi` CLI binary;
- interactive ratatui TUI mode;
- one-shot text mode and `--json` NDJSON mode;
- `--rpc` JSONL command/event mode;
- model, session, branch, and session-tree pickers;
- image attachments through `--image` and `/image`;
- session list/resume/fork/delete commands;
- eight built-in tools;
- config, context-file loading, session persistence, compaction, retry, usage,
  cost summaries, package/resource discovery, diagnostics, and opt-in traces.

The workspace package version is `0.6.4`; the checkout may also contain
unreleased Phase 13 session work. Recent phases harden existing behavior without
adding new core workflow tools. Phase 11 makes filesystem failures typed,
read/bash output truncation explicit, write/edit metadata auditable, navigation
tools bounded and gitignore-aware, and failed tool results visible to provider
adapters. Phase 12 wires provider correctness through the CLI and harness:
provider construction validates auth/config with redacted diagnostics,
OpenAI-compatible profiles carry the documented compatibility flags and
`extra_headers`, provider errors reach text/JSON/RPC diagnostics, and session
cost summaries are omitted when any turn has unknown usage or when pricing is
unknown. Phase 13 adds typed session metadata, branch summaries, active-branch
reconstruction diagnostics, persisted model/thinking state, and local session
export commands.

The crate is usable as a library through `CodingHarness`, but most users should
start with the CLI.

## Install

```sh
cargo install opi-coding-agent
opi --version
```

Pre-built binaries are attached to
[GitHub Releases](https://github.com/OdradekAI/opi/releases).

## Quick Start

```sh
export ANTHROPIC_API_KEY=sk-ant-...

# Interactive TUI
opi

# One prompt, assistant text to stdout
opi "Find TODO comments in this repository."

# NDJSON event stream
opi --json "Summarize this workspace."

# Select a provider/model
opi -m openai:gpt-4o "Explain crates/opi-coding-agent/src/main.rs"

# Attach images to the first prompt
opi --image screenshot.png "Review this screenshot."

# Allow write/edit/bash in non-interactive automation
opi --allow-mutating "Update the README."
```

## CLI Commands and Flags

Run `opi --help` for the exact current surface. Important commands and flags:

| Command / flag | Purpose |
|----------------|---------|
| `[PROMPT]...` | Non-empty positional prompt selects one-shot text mode. |
| `-m, --model <SPEC>` | Model spec such as `anthropic:claude-sonnet-4-5-20250514`. |
| `-c, --config <FILE>` | Explicit TOML config file; it must exist. |
| `-s, --system <FILE>` | Append a user system prompt file to the built-in coding prompt. |
| `--non-interactive` | Force one-shot text mode; prompt text is still required. |
| `--json` | Emit NDJSON session/agent events to stdout. |
| `--rpc` | Start bidirectional JSONL command/event mode over stdin/stdout. |
| `--allow-mutating` | Allow `write`, `edit`, and `bash` outside interactive mode. |
| `--tools <TOOLS>` | Comma-separated built-in tool allowlist. |
| `--no-tools` | Disable all tools. |
| `--no-builtin-tools` | Disable built-in tools while leaving extension/custom tools available. |
| `--image <PATH>` | Attach one image to the initial prompt; repeatable. |
| `--list-models` | List models exposed by configured providers and exit. |
| `--list-sessions` | List stored sessions and exit. |
| `--resume <ID>` | Resume a stored session. |
| `--fork <ID>` | Fork a stored session into a new session. |
| `--delete-session <ID>` | Delete a stored session and exit. |
| `--export-session <ID\|PATH>` | Export a session as markdown or JSON to a local file. |
| `--format <md\|json>` | Output format for `--export-session`. |
| `--output <FILE>` | Output path for `--export-session`. |
| `--full-tree` | Export the full session tree, not just the active branch. |
| `--exclude-tool-output` | Omit tool output from the export. |
| `--exclude-thinking` | Omit thinking content from the export. |
| `--redact <summary\|verbose\|none>` | Phase 7 redaction mode for `--export-session`. |
| `--generate-completion <SHELL>` | Generate completion for `bash`, `zsh`, `fish`, `powershell`, or `elvish`. |
| `--trace <PATH>` | Write an opt-in, redacted local trace envelope for a non-interactive/JSON run. |
| `doctor [--json] [--scope ...]` | Local, network-free health check. |
| `package <add|remove|list|doctor>` | Manage local/git extension packages. |

## Providers

| Prefix | Backend | Default credentials/config |
|--------|---------|----------------------------|
| `anthropic:` | `AnthropicProvider` | `ANTHROPIC_API_KEY` |
| `openai:` | `OpenAiChatProvider` | `OPENAI_API_KEY` |
| `openai-responses:` | `OpenAiResponsesProvider` | `OPENAI_API_KEY` |
| `openrouter:` | OpenRouter profile | `OPENROUTER_API_KEY` |
| `mistral:` | Mistral profile | `MISTRAL_API_KEY` |
| `gemini:` | `GeminiProvider` | `GEMINI_API_KEY` |
| `bedrock:` | `BedrockProvider` | AWS env vars or shared AWS profile/config |
| `azure:` | `AzureOpenAIProvider` | `AZURE_OPENAI_API_KEY`; endpoint/deployments in config |
| `vertex:` | `VertexProvider` | `VERTEX_ACCESS_TOKEN`; project/location in config |
| configured profile | OpenAI-compatible profile | profile-specific `api_key_env`, `base_url`, and model list |

Provider credential env names, base URLs, model lists, and proxies can be
overridden in config.

## Built-in Tools

Tools live under `src/tool/`.

| Tool | Args | Notes |
|------|------|-------|
| `read` | `path`, optional `offset`, `limit` | 1-based line offset; parallel. |
| `ls` | `path`, optional `max_entries`, `max_depth` | Deterministic directory listing; gitignore-aware; parallel. |
| `glob` | `pattern` | Gitignore-aware file discovery; parallel. |
| `find` | `pattern`, optional `path` | Gitignore-aware file discovery scoped to an optional subdirectory; parallel. |
| `grep` | `pattern` | Gitignore-aware regex search; parallel. |
| `write` | `path`, `content` | Creates parent dirs; sequential; mutating. |
| `edit` | `path`, `old_string`, `new_string` | Replaces the unique exact match and records before/after details; sequential; mutating. |
| `bash` | `command`, optional `timeout_secs` | Runs in workspace root via `cmd /C` on Windows or `sh -c` on Unix; sequential; mutating. |

`glob` is an opi convenience tool; pi-compatible workflows should not depend on it being the only discovery path.

Default active tools:

| Mode | Tools |
|------|-------|
| Interactive | `read`, `write`, `edit`, `bash` |
| Non-interactive / RPC | `read`, `grep`, `find`, `ls`, `glob` |
| Non-interactive / RPC with mutating opt-in | `read`, `write`, `edit`, `bash` |

In non-interactive/RPC mode, explicit allowlists containing `write`, `edit`, or
`bash` require `--allow-mutating` or `defaults.allow_mutating_tools = true`.

## Tool Result Contract

Every built-in tool returns the same runtime shape:

| Field | Meaning |
|---|---|
| `content` | LLM-visible text or image output. |
| `details` | Structured metadata for UI, JSON/RPC, session, and trace boundaries. |
| `is_error` | Set when the operation failed or `bash` exited nonzero. |
| `terminate` | Reserved for tools that deliberately end the run. |
| `truncated` | Set when output is shortened by line, byte, or walk limits. |
| `diagnostics` | Structured cause records lifted into opi diagnostics and traces. |

Failure details are deliberately bounded and redacted at public boundaries.
Provider requests receive the LLM-visible content and failure state, not raw
command/path-sensitive diagnostic context.

## Tool Policy

The eight built-in tools split into a read-only set and a mutating set. Mutating
tools run only where the resolved policy allows them; the rest is enforced
through tool selection, not through an OS sandbox or interactive permission
prompts.

### Read-only vs mutating

| Tool | Class |
|------|-------|
| `read`, `grep`, `find`, `ls`, `glob` | read-only |
| `write`, `edit`, `bash` | mutating |

`write` and `edit` are confined to the workspace root; non-interactive `read` is
too, but interactive `read` may follow absolute and outside-workspace paths.
`bash` is not path-confined. Default tool sets per mode, and the
`--allow-mutating` requirement for non-interactive/RPC mutating tools, are
listed under [Built-in Tools](#built-in-tools) above.

### Flag precedence

Tool flags resolve with deterministic precedence:

`--no-tools` > `--tools <list>` > `--no-builtin-tools` > default

`--no-tools` disables every tool; `--tools` keeps only the named built-ins;
`--no-builtin-tools` drops built-ins while leaving extension/custom tools
available; otherwise the mode default applies.

### bash execution

| Aspect | Behavior |
|--------|----------|
| Shell | `cmd /C` on Windows, `sh -c` on Unix. |
| Working directory | Workspace root. |
| Timeout | 30 seconds by default; `timeout_secs` overrides. |
| Cancellation | A cancellation token reports `cancelled=true` / `timed_out=false`; a timeout reports `timed_out=true` / `cancelled=false`. |
| Path confinement | None — `bash` is not restricted to the workspace. |
| Environment | Inherited from the parent process, but never copied into details: `details.env = { "inheritance": "inherited", "values_included": false }`. A value is exposed only if the command itself prints it. |
| Exit code | Reported in details; a nonzero exit sets `is_error`. `exit_code` is null when the process is cancelled or times out before exiting. |
| Output | Combined stdout and stderr are capped at 64 KiB. See [Output truncation](#output-truncation). |

### Output truncation

| Tool | Cap | Truncation behavior |
|------|-----|---------------------|
| `read` | 2000 lines by default | Sets `truncated`, appends an `... N lines omitted` marker, and records `details.truncated` / `omitted` / `line_count`. An explicit `limit` is not capped by the default line cap, but the 64 KiB byte cap still applies; `limit: 0` returns no lines and flags `truncated`. |
| `bash` | 64 KiB combined stdout+stderr | When total output exceeds the cap, the preview is the first 64 KiB of merged stdout-then-stderr, `truncated` and `details.truncated` are set, and opi best-effort spills the complete merged output to a temp file reported in `details.full_output`. If the spill file cannot be created, only `truncated` is set. |

### Navigation bounds

`grep`, `find`, `ls`, and `glob` use the same gitignore-aware walker, include
dotfiles, do not follow symlinks, and sort paths deterministically. `grep`,
`find`, and `glob` return at most 200 inline results per call; all four
navigation tools stop walking after 10,000 visited entries. `grep` also skips
files larger than 1 MiB and stops after reading 8 MiB total. Skipped files and
early termination are reported through `details` and diagnostics when available.

### Non-goals

The following are intentionally out of scope for the built-in tools (broader
product boundaries are listed under [Boundaries](#boundaries)):

- built-in permission popup or interactive approval prompt
- persistent background bash or shell sessions
- remote execution
- IDE project index
- language-server integration
- automatic formatting on `write` / `edit`
- package ecosystem expansion
- workflow tools such as todo, plan mode, or sub-agents
- sandbox implementation

Mutating-tool safety is a tool-selection check, not a permission or sandbox subsystem.

## Modes

### Interactive

With no prompt args, `opi` starts the ratatui TUI. Slash commands include:

| Command | Effect |
|---------|--------|
| `/model` | Open the model picker for the active provider. |
| `/session` | Open the session picker. |
| `/session info` | Show session name, labels, active branch, model, and thinking metadata. |
| `/name <name>` | Set the active session's typed name (`session_info` entry). |
| `/label <label>` | Add a label to the active session (`label` entry). |
| `/unlabel <label>` | Remove a label from the active session. |
| `/branch` | Open the branch picker. |
| `/tree` | Open the session tree picker. |
| `/fork` | Fork the active branch into a new parented session. |
| `/clone` | Clone the active branch into a new parented session. |
| `/image <path>` | Queue an image for the next prompt. |
| `exit` / `quit` | Exit. |

### Non-interactive and JSON

Text mode writes assistant text to stdout and diagnostics to stderr. `--json`
writes a schema header, serialized session/agent events, and a final
`session_summary` as NDJSON. The current NDJSON schema version is
`NDJSON_SCHEMA_VERSION = 2`.

Exit codes:

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | Runtime failure |
| `2` | Config error |
| `3` | Auth failure |
| `4` | Provider failure |
| `5` | Tool failure |
| `130` | Interrupted |

### RPC JSONL

`--rpc` starts a persistent bidirectional JSONL protocol for IDEs, custom UIs,
and other embedders. This is an unstable 0.x protocol; clients must check the
`schema_version` in the `rpc_ready` header. The current SDK/RPC schema version
is `3`. Startup diagnostics are surfaced in the `startup_diagnostics` field of
that ready header.

Commands include `prompt`, `continue`, `steer`, `follow_up`, `abort`,
`set_model`, `set_thinking_level`, `compact`, `session_info`,
`extension_command`, `trace`, and `quit`.

Runtime-state rejection responses may include `error_code` values:
`unsupported_trace_request`, `agent_busy`, `harness_unavailable`,
`compaction_failed`, and `extension_command_not_handled`. Idle capability
validation failures from `set_model` and `set_thinking_level` remain free-text
errors without `error_code`.

## Config, Sessions, and Context Files

Config layers merge user config, project config, and explicit `--config` files.
Model precedence is `--model`, then `OPI_MODEL` when no `--config` was passed,
then explicit config, project `.opi/config.toml`, user config, and built-in
defaults.

User config paths:

- Windows: `%APPDATA%\opi\config.toml`
- Unix: `~/.config/opi/config.toml`

Sessions are append-only JSONL files under `%LOCALAPPDATA%\opi\sessions\` on
Windows and `~/.local/share/opi/sessions/` on Unix, unless `OPI_SESSIONS_DIR`
is set.

`CodingHarness` loads `AGENTS.md` and `CLAUDE.md` from the workspace ancestors
up to the git root, then from the user config directory. Empty files and files
larger than 128 KiB are ignored. `OPI.md` is intentionally not loaded.

## Resources and Packages

Resource discovery covers extensions, packages, skills, prompt fragments, and
themes from user, project, explicit, and package layers. Higher-precedence
layers override lower-precedence layers; duplicate names within the same layer
are reported as diagnostics.

Package commands:

```sh
opi package add ./vendor/todo
opi package add --local ./vendor/todo
opi package add git:github.com/user/pkg@v1
opi package list
opi package list --json
opi package doctor
opi package doctor --json
opi package remove todo
```

Packages can start `process-jsonl` adapters using the
`opi-extension-jsonl-v1` protocol. That adapter protocol is an unstable 0.x
contract. Packages are trusted code and are not sandboxed by the package
manager.

## Library Use

`CodingHarness` is the embedding entry point. It can be built directly or
through `CodingHarness::builder`, with optional custom hooks, session resume
data, tool selection, runtime package state, resource metadata, and startup
diagnostics.

Common methods include `prompt`, `prompt_with_content`, `queue_images`,
`subscribe`, `cancel`, `set_model`, `model_picker_items`, `branch_picker_items`,
`resource_metadata`, `resolve_theme`, and `session`.

## Boundaries

- `opi` does not collect telemetry or analytics and does not share sessions
  automatically.
- `opi doctor` makes no paid model calls or network checks by default.
- Mutating-tool policy is not an OS sandbox.
- Production sub-agent, permission-gate, plan/todo, and MCP workflows are
  examples/package patterns, not built-in core workflows.
- OAuth or subscription login flows are not implemented. Phase 12 provider
  correctness is a correctness phase, not a breadth phase; the following
  remain deferred product decisions: OAuth login, Anthropic / OpenAI Codex /
  GitHub Copilot subscription auth, a broad new first-class provider list
  (compatible providers stay config-driven OpenAI-compatible profiles), image
  generation (image support is input-only), browser usage, a provider
  streaming-adapter protocol for packages, paid live provider calls in default
  tests, and copying pi's provider-specific config file format. Per-provider
  proxy config (`proxy.url` / `proxy.no_proxy`, env `HTTPS_PROXY` >
  `HTTP_PROXY` > `NO_PROXY`) and best-effort cost (explicit unknown values over
  false confidence) are implemented. See the `opi-ai` README for the per-family
  behavior matrix, OpenAI-compatible profile flags (`system_role_override`,
  `max_tokens_field`, `tool_result_name_field`, `usage_in_stream`,
  `strict_tool_schema`, `reasoning_effort`, `cache_key`,
  `require_assistant_after_tool_result`; plus per-profile `extra_headers` for
  static request headers, which is a profile config field, not a `CompatConfig`
  flag), OpenAI Responses native semantics (`store` / `reasoning_effort` /
  `strict_tools` implemented; `previous_response_id` deferred), and cache /
  response-ID / session-affinity behavior. In particular, `usage_in_stream`
  requests `stream_options.include_usage`, OpenAI Chat captures response IDs
  from any chunk carrying `id`, `require_assistant_after_tool_result` stays
  metadata-only in the shared adapter, and session cost summaries are omitted
  when any turn has unknown usage or when pricing is unknown. Phase 13 session
  work may rely on provider-correct usage, model, thinking, and error data
  through the shared `opi-ai` types, without depending on provider-specific
  internals.

## License

MIT. See the workspace [LICENSE](../../LICENSE).
