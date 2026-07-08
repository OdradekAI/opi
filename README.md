# opi

[![CI](https://github.com/OdradekAI/opi/actions/workflows/ci.yml/badge.svg)](https://github.com/OdradekAI/opi/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

> Rust AI agent toolkit and terminal-first coding agent inspired by
> [earendil-works/pi](https://github.com/earendil-works/pi).

opi is an embeddable, multi-provider coding-agent runtime you can drive as an
interactive TUI, a one-shot CLI, an NDJSON event stream, or an RPC harness.

[简体中文](README.zh.md) | [Changelog](CHANGELOG.md)

## Status

The workspace package version in `Cargo.toml` is `0.6.5`. This checkout may
contain unreleased changes on top of the published `0.6.5` crates; check
[CHANGELOG.md](CHANGELOG.md) for the current delta.

`opi` is usable today as a terminal coding agent and as a set of Rust crates
for building agent runtimes. It reimplements selected pi ideas in Rust, but it
is not API-compatible with pi, does not read pi config by default, and uses its
own TOML config plus append-only JSONL session format.

Wire protocols, session/export formats, packages, extensions, SDK/RPC, and
trace envelopes remain unstable 0.x surfaces. Pin exact crate versions for
embedders.

## Install

The CLI binary is named `opi` and is produced by the `opi-coding-agent` crate.

```sh
cargo install opi-coding-agent
opi --version
```

Pre-built binaries for Linux, macOS, and Windows on x64 and arm64 are attached
to [GitHub Releases](https://github.com/OdradekAI/opi/releases).

## Quick Start

Set credentials for one provider:

```sh
export ANTHROPIC_API_KEY=sk-ant-...
# or OPENAI_API_KEY, OPENROUTER_API_KEY, MISTRAL_API_KEY, GEMINI_API_KEY
# or AWS credentials, AZURE_OPENAI_API_KEY, VERTEX_ACCESS_TOKEN
```

Run the interactive TUI:

```sh
opi
```

Run one prompt:

```sh
opi "List the Rust crates in this workspace."
```

Emit NDJSON events:

```sh
opi --json "Summarize this repository."
```

The final `session_summary` line reports `turns` (accepted user prompt turns) and `provider_turns` (provider request/response cycles, i.e. `TurnStart` events).

Attach images to the first prompt:

```sh
opi --image screenshot.png "Review this UI."
```

Select a model with `provider:model` syntax:

```sh
opi -m anthropic:claude-sonnet-4-5-20250514 "Explain crates/opi-agent/src/lib.rs"
opi -m openai:gpt-4o "Review the public API shape."
```

Export a saved session locally:

```sh
opi --export-session <ID_OR_PATH> --output session.md
opi --export-session <ID_OR_PATH> --output session.json --format json
```

## Workspace Crates

All crates share the workspace version, edition, license, repository, and
authors.

| Crate | Purpose |
| --- | --- |
| [`opi-ai`](crates/opi-ai) | Provider-neutral LLM API, streaming events, model registry, retries, HTTP/proxy support, usage, and best-effort cost helpers. |
| [`opi-agent`](crates/opi-agent) | Agent loop, tool contract, hooks, events, queues, sessions, compaction, SDK/RPC types, extensions, diagnostics, and streaming proxy. |
| [`opi-tui`](crates/opi-tui) | Ratatui widgets, transcript rendering, diff view, pickers, terminal images, themes, and keybindings. |
| [`opi-coding-agent`](crates/opi-coding-agent) | The `opi` binary, built-in coding tools, config/session/package handling, and embeddable `CodingHarness`. |

Internal dependency shape:

```text
opi-ai
opi-tui
opi-agent -> opi-ai
opi-coding-agent -> opi-ai + opi-agent + opi-tui -> opi binary
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

Common mode and session flags:

| Flag | Purpose |
| --- | --- |
| `--non-interactive` | Force one-shot text mode. |
| `--json` | One-shot NDJSON event stream. |
| `--json-compact` | Compact `--json`: streamed `text_delta` updates omit the redundant cumulative snapshot (~linear bytes for long turns). |
| `--rpc` | Persistent JSONL command/event protocol over stdin/stdout. |
| `--resume <ID>` | Resume a saved session. |
| `--fork <ID>` | Fork a saved session into a new session. |
| `--export-session <ID_OR_PATH>` | Render a saved session to a local markdown or JSON file. |
| `--output <PATH>` | Required output path for `--export-session`. |
| `--format <markdown\|json>` | Export format; `md` is accepted as a markdown alias. |
| `--full-tree` | Export the whole session tree instead of only the active branch. |
| `--exclude-tool-output` | Omit tool result output from exported transcripts. |
| `--exclude-thinking` | Omit assistant thinking content from exported transcripts. |
| `--redact <summary\|verbose\|none>` | Export redaction mode; default is `summary`. |
| `--tools read,grep` | Enable only the listed built-in tools. |
| `--no-tools` | Disable all tools. |
| `--no-builtin-tools` | Drop built-in tools while leaving extension/custom tools available. |
| `--allow-mutating` | Allow `write`, `edit`, and `bash` in non-interactive/RPC runs. |
| `--trace <PATH>` | Write an opt-in, redacted local trace envelope for a non-interactive or JSON run. |

## Providers

Provider support lives in `opi-ai` and is wired into `opi-coding-agent`.

| Prefix | Backend | Default credentials |
| --- | --- | --- |
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

Compatible OpenAI-style services should normally use configured profiles rather
than new first-class provider modules. For `usage_in_stream`, OpenAI-compatible
profiles request `stream_options.include_usage`; response IDs captured from any
OpenAI Chat chunk carrying `id` round-trip into `response_id`. A profile may
also set `chat_completions_path` for base URLs that already include an API
prefix. Cost summaries are omitted when usage or pricing is unknown.

## Built-in Tools

Available built-in tools are `read`, `write`, `edit`, `bash`, `grep`, `find`,
`ls`, and `glob`.

| Mode | Default tools |
| --- | --- |
| Interactive TUI | `read`, `write`, `edit`, `bash` |
| Non-interactive / RPC | `read`, `grep`, `find`, `ls`, `glob` |
| Non-interactive / RPC with mutating opt-in | `read`, `write`, `edit`, `bash` |

File writes and edits are scoped to the harness workspace root. Interactive
`read` can inspect absolute paths and paths outside the workspace. `bash`
starts in the workspace root but is not path-confined. These are tool policy
checks, not an operating-system sandbox.

Tool results carry LLM-visible `content`, optional structured `details`,
`is_error`, `terminate`, `truncated`, and optional diagnostics. `read` returns
line/path metadata and defaults to a 2000-line preview with a 64 KiB byte cap.
`bash` runs one foreground command, caps combined stdout/stderr at 64 KiB, and
may spill complete output to `details.full_output`.

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
| --- | --- |
| Windows | `%LOCALAPPDATA%\opi\sessions\` |
| Unix | `~/.local/share/opi/sessions/` |

Use `OPI_SESSIONS_DIR` to override the location. Session files are sensitive:
they contain prompts, tool output, and possibly leaked secrets. The v1 JSONL
header is kept, with typed entries for session names, model changes, thinking
levels, labels, and branch summaries. `--resume`, `--fork`, `--list-sessions`,
RPC `session_info`, and `--export-session` all reconstruct the active branch
through the same context path. `opi --export-session` is local-only and applies
redaction by default.

## Extensibility

`opi --rpc` exposes an unstable 0.x JSONL command/event protocol. Current wire
versions are:

| Surface | Current version | Where it appears |
| --- | --- | --- |
| NDJSON mode | `NDJSON_SCHEMA_VERSION = 2` | `opi --json` schema header |
| RPC / SDK | `SDK_SCHEMA_VERSION = 3` | `opi --rpc` `rpc_ready.schema_version` |
| Trace envelope | `TRACE_SCHEMA_VERSION = 1` | `--trace <PATH>` and RPC `trace` payloads |

RPC commands include `prompt`, `continue`, `steer`, `follow_up`, `abort`,
`set_model`, `set_thinking_level`, `compact`, `session_info`,
`extension_command`, `trace`, and `quit`.

Resource discovery supports extensions, packages, skills, prompt fragments, and
themes. `opi package add/remove/list/doctor` works for local and git package
sources. Package manifests can start `process-jsonl` adapters using the
`opi-extension-jsonl-v1` protocol; adapters can expose tools, commands, hooks,
events, state, and model/provider overrides.

## Permissions and Trust Boundaries

`opi` runs with the operating-system permissions of the user and process that
launched it. Tool selection and mutating-tool flags control which built-in
tools the agent can call; they are not an operating-system sandbox.

- `bash` can execute commands with the launching user's OS permissions.
- Packages are trusted code. A package can start child processes with the same OS permissions as `opi`; package permission declarations are metadata, not enforced sandbox policy.
- Observability is local and explicit: `opi` does not collect telemetry or
  analytics, does not share sessions automatically, `opi doctor` is local and
  network-free by default, and `trace` is opt-in.
- Production sub-agent, permission-gate, plan/todo, and MCP workflows are
  examples/package patterns, not built-in core workflows.
- OAuth login, subscription auth, image generation, browser usage, provider
  streaming-adapter protocols for packages, paid live provider calls in default
  tests, and copying pi's provider-specific config file format remain deferred.
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
