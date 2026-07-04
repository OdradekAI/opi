# opi

[![CI](https://github.com/OdradekAI/opi/actions/workflows/ci.yml/badge.svg)](https://github.com/OdradekAI/opi/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

> Rust AI agent toolkit and terminal-first coding agent inspired by
> [earendil-works/pi](https://github.com/earendil-works/pi).

[Simplified Chinese](README.zh.md) | [Changelog](CHANGELOG.md) | [Spec draft](docs/opi-spec.md)

## Status

The workspace package version in `Cargo.toml` is `0.6.3`. `opi` is usable as a
terminal coding agent and as a set of Rust crates for embedding agent runtime
pieces. The repository may also contain unreleased changes on top of that
version; check [CHANGELOG.md](CHANGELOG.md) for the current delta.

`opi` reimplements selected pi ideas in Rust. It is not API-compatible with pi,
does not read pi config by default, and uses its own TOML config and JSONL
session format.

The `0.6.3` release includes the Phase 10 provider collection/auth,
generic-harness, session-facade, and runtime-hook boundary seams, plus the
Phase 11 tooling-quality work
(see [CHANGELOG.md](CHANGELOG.md)): built-in tool results now carry consistent
metadata, truncation flags, and structured diagnostics; public event/session
surfaces redact sensitive tool details; provider adapters preserve failed
tool-result semantics. Treat wire protocols, extension/package surfaces, trace
payloads, and Phase 10/11 seams as unstable 0.x unless a crate README explicitly
says otherwise.

## Install

The CLI binary is named `opi` and is produced by the `opi-coding-agent` crate.

```sh
cargo install opi-coding-agent
opi --version
```

Pre-built binaries for Linux, macOS, and Windows on x64 and arm64 are attached
to [GitHub Releases](https://github.com/OdradekAI/opi/releases).

## Quick Start

Set credentials for the provider you want to use:

```sh
export ANTHROPIC_API_KEY=sk-ant-...
# or OPENAI_API_KEY, OPENROUTER_API_KEY, MISTRAL_API_KEY, GEMINI_API_KEY
# or AWS credentials, AZURE_OPENAI_API_KEY, VERTEX_ACCESS_TOKEN
```

Run the interactive TUI:

```sh
opi
```

Run a single prompt:

```sh
opi "List the Rust crates in this workspace."
```

Emit NDJSON events for automation:

```sh
opi --json "Summarize this repository."
```

Attach images to the first prompt:

```sh
opi --image screenshot.png "Review this UI."
```

Select a model with `provider:model` syntax:

```sh
opi -m anthropic:claude-sonnet-4-5-20250514 "Explain crates/opi-agent/src/lib.rs"
opi -m openai:gpt-4o "Review the public API shape."
```

## Main CLI Surface

```sh
opi --help
opi --list-models
opi --list-models --json
opi --generate-completion powershell
opi doctor
opi package list
```

Common mode flags:

| Flag | Purpose |
|------|---------|
| `--non-interactive` | Force one-shot text mode. |
| `--json` | One-shot NDJSON event stream. |
| `--rpc` | Persistent JSONL command/event protocol over stdin/stdout. |
| `--resume <ID>` | Resume a saved session. |
| `--fork <ID>` | Fork a saved session into a new session. |
| `--tools read,grep` | Enable only the listed built-in tools. |
| `--no-tools` | Disable all tools. |
| `--allow-mutating` | Allow `write`, `edit`, and `bash` in non-interactive/RPC runs. |
| `--trace <PATH>` | Write an opt-in, redacted local trace envelope for the run. |

## Wire Versions

Automation and embedder surfaces are versioned, but still unstable 0.x:

| Surface | Current version | Where it appears |
|---------|-----------------|------------------|
| NDJSON mode | `NDJSON_SCHEMA_VERSION = 2` | `opi --json` schema header |
| RPC / SDK | `SDK_SCHEMA_VERSION = 3` | `opi --rpc` `rpc_ready.schema_version` |
| Trace envelope | `TRACE_SCHEMA_VERSION = 1` | `--trace <PATH>` and RPC `trace` payloads |

RPC runtime-state rejections may carry a stable machine-readable `error_code`:
`unsupported_trace_request`, `agent_busy`, `harness_unavailable`,
`compaction_failed`, or `extension_command_not_handled`. Idle capability errors
from `set_model` and `set_thinking_level` remain free-text validation errors.

## Providers

Provider support lives in `opi-ai` and is wired into `opi-coding-agent`.

| Prefix | Backend | Default credentials |
|--------|---------|---------------------|
| `anthropic:` | Anthropic Messages streaming | `ANTHROPIC_API_KEY` |
| `openai:` | OpenAI Chat Completions streaming | `OPENAI_API_KEY` |
| `openai-responses:` | OpenAI Responses streaming | `OPENAI_API_KEY` |
| `openrouter:` | OpenAI-compatible OpenRouter profile | `OPENROUTER_API_KEY` |
| `mistral:` | OpenAI-compatible Mistral profile | `MISTRAL_API_KEY` |
| `gemini:` | Gemini streaming | `GEMINI_API_KEY` |
| `bedrock:` | AWS Bedrock Converse streaming | AWS env vars or shared AWS config |
| `azure:` | Azure OpenAI deployment | `AZURE_OPENAI_API_KEY` plus endpoint config |
| `vertex:` | Google Vertex AI Gemini streaming | `VERTEX_ACCESS_TOKEN` plus project/location config |
| configured profile | OpenAI-compatible Chat Completions profile | profile-specific `api_key_env` |

## Built-in Tools

Available built-in tools are `read`, `write`, `edit`, `bash`, `grep`, `find`,
`ls`, and `glob`.

Default active tools depend on run mode:

| Mode | Default tools |
|------|---------------|
| Interactive TUI | `read`, `write`, `edit`, `bash` |
| Non-interactive / RPC | `read`, `grep`, `find`, `ls`, `glob` |
| Non-interactive / RPC with mutating opt-in | `read`, `write`, `edit`, `bash` |

File writes and edits are scoped to the harness workspace root. Interactive
`read` can inspect absolute paths and paths outside the workspace. These rules
are tool policy, not an operating-system sandbox.

Phase 11 makes tool results more inspectable without expanding the built-in
tool set:

- Built-in tool results carry `content`, optional `details`, `is_error`,
  `terminate`, `truncated`, and optional structured diagnostics.
- `read` returns line and path metadata, caps omitted output by default at 2000
  lines, and still applies a 64 KiB byte cap.
- `bash` runs one foreground command, reports cwd/shell/exit/timeout/cancel
  metadata, caps combined stdout/stderr at 64 KiB, and may spill the complete
  output path to `details.full_output`.
- `grep`, `find`, `ls`, and `glob` share gitignore-aware walking, deterministic
  ordering, bounded result counts, and skipped-file diagnostics.
- Tool-selection safety is not a permission system. Use external containers,
  VMs, or sandboxes when OS-level isolation is required.

## Config and Sessions

Config layers merge user config, project config, and an explicit `--config`
file. Model precedence is:

1. `--model`
2. `OPI_MODEL` when `--config` was not passed
3. `model` in `--config <FILE>`
4. `<CWD>/.opi/config.toml`
5. User config (`%APPDATA%\opi\config.toml` on Windows,
   `~/.config/opi/config.toml` on Unix)
6. Built-in defaults

Sessions are append-only JSONL files written automatically.

| Platform | Default session directory |
|----------|---------------------------|
| Windows | `%LOCALAPPDATA%\opi\sessions\` |
| Unix | `~/.local/share/opi/sessions/` |

Use `OPI_SESSIONS_DIR` to override the location.

## Workspace Crates

All crates share the workspace version, edition, license, repository, and
authors.

| Crate | Published | Purpose |
|-------|-----------|---------|
| [`opi-ai`](crates/opi-ai) | yes | Provider-neutral LLM API, streaming events, model registry, retries, HTTP/proxy support, usage and cost helpers. |
| [`opi-agent`](crates/opi-agent) | yes | Agent loop, tool execution, hooks, events, queues, sessions, compaction, SDK types, extensions, streaming proxy. |
| [`opi-tui`](crates/opi-tui) | yes | Ratatui widgets, transcript rendering, diff view, pickers, terminal images, themes, keybindings. |
| [`opi-coding-agent`](crates/opi-coding-agent) | yes | The `opi` binary and embeddable coding harness. |

Internal dependency shape:

```text
opi-ai
opi-tui
opi-agent -> opi-ai
opi-coding-agent -> opi-ai + opi-agent + opi-tui -> opi binary
```

## Extensibility

`opi --rpc` exposes an unstable 0.x JSONL command/event protocol with schema
version checks. `opi-agent` also exposes shared SDK types and extension
registry primitives for embedders. RPC commands include `prompt`, `continue`,
`steer`, `follow_up`, `abort`, `set_model`, `set_thinking_level`, `compact`,
`session_info`, `extension_command`, `trace`, and `quit`.

Resource discovery supports extensions, packages, skills, prompt fragments, and
themes. Package manifests can start `process-jsonl` adapters that expose custom
tools, commands, hooks, event observers, state, and model/provider overrides.

## Permissions and Trust Boundaries

`opi` runs with the operating-system permissions of the user and process that
launched it. Tool selection and mutating-tool flags control which built-in tools
the agent can call; they are not an operating-system sandbox.

- File writes and edits are scoped to the harness workspace root. `bash` starts
  in the workspace root but can execute commands with the launching user's OS
  permissions.
- Packages are trusted code. A package can start child processes with the same
  OS permissions as `opi`; package permission declarations are metadata, not enforced sandbox policy.
- Observability is local and explicit: `opi` does not collect telemetry or
  analytics, does not share sessions automatically, `opi doctor` is local and
  network-free by default, and traces are opt-in.
- Production sub-agent, permission-gate, plan/todo, and MCP workflows are not
  built into the core CLI. The repository contains examples and package
  scaffolds for those patterns.
- OAuth or subscription login flows are not implemented. Provider correctness
  is a Phase 12 focus, not a breadth phase: OAuth login, Anthropic / OpenAI
  Codex / GitHub Copilot subscription auth, a broad new first-class provider
  list, image generation, browser usage, a provider streaming-adapter
  protocol for packages, paid live provider calls in default tests, and
  copying pi's provider-specific config file format remain deferred. See the
  `opi-ai` README for the per-family behavior matrix, OpenAI-compatible
  profile flags, cache / response-ID / session-affinity behavior, proxy, and
  best-effort cost, including `usage_in_stream` -> `stream_options.include_usage`,
  response IDs captured from any OpenAI Chat chunk carrying `id`, and omitted
  cost summaries when usage or pricing is unknown.
- Dynamic Rust plugin loading from arbitrary extension paths is not supported.

If you need stronger isolation, run `opi` inside a container, VM, or external
sandbox appropriate for the tools and credentials you expose to it.

## Development

Rust 1.85 or newer is required because the workspace uses Rust edition 2024.

```sh
cargo build
cargo run -p opi-coding-agent -- --help
cargo test --workspace --all-targets
cargo fmt --check --all
cargo clippy --workspace --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

See [AGENTS.md](AGENTS.md) for repository working rules and
[docs/opi-spec.md](docs/opi-spec.md) for the technical spec draft.

## License

MIT (c) OdradekAI. See [LICENSE](LICENSE).
