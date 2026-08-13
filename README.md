# opi

[![CI](https://github.com/OdradekAI/opi/actions/workflows/ci.yml/badge.svg)](https://github.com/OdradekAI/opi/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/opi-coding-agent.svg)](https://crates.io/crates/opi-coding-agent)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

Opi is a Rust AI Agent toolkit and terminal-first coding Agent. Use it as an
interactive TUI, a one-shot CLI, an NDJSON/RPC automation process, or a set of
embeddable Rust crates.

Opi follows the small-core, extensible-harness design demonstrated by
[earendil-works/pi](https://github.com/earendil-works/pi), while using its own
Rust APIs, TOML configuration, and append-only JSONL sessions.

[简体中文](README.zh.md) | [Releases](https://github.com/OdradekAI/opi/releases) | [Changelog](CHANGELOG.md)

## Highlights

- Interactive coding TUI with streaming output, session and branch navigation,
  model selection, image attachments, diffs, themes, and terminal images.
- One-shot text, NDJSON, and persistent JSONL RPC modes for scripts and
  embedders.
- Anthropic, OpenAI, OpenRouter, Mistral, Gemini, Bedrock, Azure OpenAI, Vertex
  AI, GitHub Copilot, OpenAI Codex, and custom OpenAI-compatible providers.
- Eight built-in coding tools with different interactive and automation
  defaults.
- Local append-only sessions with resume, fork, export, compaction, and crash
  recovery.
- Rust crates for provider APIs, Agent runtime, terminal UI, coding harness,
  command-execution protocol, and optional process restriction.

## Status

The workspace package version in `Cargo.toml` is `0.8.0`. This checkout may
contain unreleased changes; see [CHANGELOG.md](CHANGELOG.md) before making a
release or compatibility decision.

The terminal Agent is usable today. Public crates and wire formats are still
0.x: pin exact versions when embedding Opi, and expect protocol/API changes to
be documented in the changelog.

## Installation

Install the `opi` binary from crates.io. Building from source requires Rust
1.97 or newer.

```sh
cargo install opi-coding-agent
opi --version
```

Pre-built `opi` archives for Linux, macOS, and Windows on x64 and arm64 are
available from [GitHub Releases](https://github.com/OdradekAI/opi/releases).
Download the archive for your platform and place the `opi` executable on your
`PATH`.

`opi-sandbox` is a separate optional product. It is not installed or linked by
`opi-coding-agent`.

## Quick start

Opi's built-in default uses Anthropic. Set one provider credential, then start
the TUI:

```sh
export ANTHROPIC_API_KEY=sk-ant-...
opi
```

PowerShell:

```powershell
$env:ANTHROPIC_API_KEY = "sk-ant-..."
opi
```

Run a single prompt and print the final answer:

```sh
opi "Summarize this repository."
```

Choose a model with `provider:model` syntax:

```sh
opi -m openai:gpt-4o "Review the public API."
```

Inside the TUI, OAuth-backed providers can be configured without an environment
API key:

```text
/login anthropic
/login github-copilot
/login openai-codex
/logout <provider>
```

Non-interactive, NDJSON, and RPC modes never open a login flow; they report the
provider and the `/login` remediation instead.

## Common workflows

| Goal | Command |
| --- | --- |
| Start the interactive TUI | `opi` |
| Run one prompt | `opi "PROMPT"` |
| Stream NDJSON events | `opi --json "PROMPT"` |
| Reduce long NDJSON stream size | `opi --json --json-compact "PROMPT"` |
| Start the JSONL RPC process | `opi --rpc` |
| Attach images to the first prompt | `opi --image screenshot.png "Review this UI"` |
| List configured models | `opi --list-models` |
| Check local configuration and health | `opi doctor` |
| Generate shell completion | `opi --generate-completion powershell` |

Use `opi --help` for the current complete flag reference.

### Sessions

Sessions are written automatically as append-only JSONL files. They can contain
prompts, tool output, paths, and secrets; protect them like local development
logs.

```sh
opi --list-sessions
opi --resume <ID>
opi --fork <ID>
opi --delete-session <ID>
opi --export-session <ID_OR_PATH> --output session.md
opi --export-session <ID_OR_PATH> --output session.json --format json
```

Exports use `summary` redaction by default. Advanced controls include
`--full-tree`, `--exclude-tool-output`, `--exclude-thinking`, and
`--redact summary|verbose|none`.

Default session locations:

| Platform | Directory |
| --- | --- |
| Windows | `%LOCALAPPDATA%\opi\sessions\` |
| Linux/macOS | `~/.local/share/opi/sessions/` |

Set `OPI_SESSIONS_DIR` to use a different directory.

## Providers and models

Run `opi --list-models` after configuring credentials or provider settings.

| Prefix | Backend | Default authentication |
| --- | --- | --- |
| `anthropic:` | Anthropic Messages | `ANTHROPIC_API_KEY` or `/login anthropic` |
| `openai:` | OpenAI Chat Completions | `OPENAI_API_KEY` |
| `openai-responses:` | OpenAI Responses | `OPENAI_API_KEY` |
| `openrouter:` | OpenRouter | `OPENROUTER_API_KEY` |
| `mistral:` | Mistral | `MISTRAL_API_KEY` |
| `gemini:` | Gemini | `GEMINI_API_KEY` |
| `bedrock:` | AWS Bedrock Converse | AWS environment or shared profile credentials |
| `azure:` | Azure OpenAI | `AZURE_OPENAI_API_KEY` plus endpoint configuration |
| `vertex:` | Vertex AI Gemini | `VERTEX_ACCESS_TOKEN` plus project/location configuration |
| `github-copilot:` | GitHub Copilot | `/login github-copilot` and the OS keychain |
| `openai-codex:` | OpenAI Codex Responses | `/login openai-codex` and the OS keychain |
| Custom profile | OpenAI-compatible Chat Completions | Configured `api_key_env` |

OAuth credentials are stored in Windows Credential Manager, macOS Keychain, or
Freedesktop Secret Service. Opi does not create a plaintext credential file.

## Configuration and project trust

Configuration uses TOML. The common model-selection precedence is:

1. `--model`
2. `OPI_MODEL` when `--config` is not supplied
3. `model` in an explicit `--config <FILE>`
4. trusted project `.opi/config.toml`
5. user config
6. built-in defaults

User configuration is `%APPDATA%\opi\config.toml` on Windows and
`~/.config/opi/config.toml` on Linux/macOS. Opi also loads a local `.env` at
startup, so never commit credentials stored there.

A minimal user configuration can select a default model:

```toml
model = "openai:gpt-4o"

[defaults]
default_project_trust = "ask"
allow_mutating_tools = false
```

Project trust is resolved once at startup. An untrusted project skips its
`.opi/config.toml`, project packages and resources, and project
`AGENTS.md`/`CLAUDE.md`; it does not disable tools. Use `--trust` or
`--no-trust` for a one-run override. There is no built-in `/trust` command or
mid-session resource reload.

See the [coding-agent crate guide](crates/opi-coding-agent/README.md) for custom
provider profiles and the complete configuration surface.

## Tools and automation policy

Built-in tools are `read`, `write`, `edit`, `bash`, `grep`, `find`, `ls`, and
`glob`.

| Mode | Default tools |
| --- | --- |
| Interactive TUI | `read`, `write`, `edit`, `bash` |
| Non-interactive / RPC | `read`, `grep`, `find`, `ls`, `glob` |
| Non-interactive / RPC with `--allow-mutating` | `read`, `write`, `edit`, `bash` |

Use `--tools read,grep`, `--no-tools`, or `--no-builtin-tools` to narrow the
available set. `write` and `edit` are workspace-root scoped. `bash` starts in
the workspace root but is not path-confined and runs with the launching user's
operating-system permissions.

These controls select Agent tools; they are not an operating-system sandbox.

## Packages and command execution

Packages can contribute skills, prompt fragments, themes, extensions, tools,
commands, hooks, and process adapters.

```sh
opi package add <PATH_OR_GIT_URL>
opi package list
opi package doctor
opi package enable <NAME>
opi package disable <NAME>
opi package remove <NAME_OR_SOURCE>
```

Executable contributions follow five independent states: Installed, Trusted,
Enabled, Selected, and Permitted. `package add` only installs; the first
`package enable` confirms Package Trust and enables the contribution. User
execution policy separately permits each invocation.

The Minimal Runtime executes `bash` through the built-in `local` backend.
`--execution-strategy fixed|rules|model` and
`--execution-backend local|ADAPTER_ID` can select an installed
`command.execute` adapter. Once an external adapter is selected, any adapter or
protocol failure is fail-closed and never falls back to local execution.

The former core `[sandbox]`, `--sandbox`, and `--sandbox-require` settings were
removed. `opi-sandbox` is the optional standalone SDK/CLI and protocol backend
for native process restriction. It depends only on `opi-protocol`; the `opi`
binary does not link it.

Official `opi-sandbox` artifacts target Linux and macOS. Windows provides L0
Job-Object process-tree supervision only. Opi does not include Docker, VM, SSH,
or remote execution adapters, and package processes run with the same OS
permissions as `opi`. Treat packages as trusted code. `opi-sandbox` is
defense-in-depth for its target process tree, not a complete security boundary.

## Embedding and protocols

| Crate | Purpose |
| --- | --- |
| [`opi-ai`](crates/opi-ai) | Provider-neutral model API, streaming, usage, retries, credentials, and model registry |
| [`opi-agent`](crates/opi-agent) | Product-neutral Agent loop, tools, hooks, queues, sessions, compaction, SDK, and extensions |
| [`opi-tui`](crates/opi-tui) | Ratatui components, transcript and diff rendering, pickers, themes, and terminal images |
| [`opi-coding-agent`](crates/opi-coding-agent) | The `opi` CLI, built-in tools, package/config/session assembly, and `CodingHarness` |
| [`opi-protocol`](crates/opi-protocol) | Versioned `command-execution-jsonl-v1` protocol types, schemas, codecs, and fixtures |
| [`opi-sandbox`](crates/opi-sandbox) | Standalone command restriction SDK/CLI and protocol backend |

Machine-facing surfaces remain unstable 0.x contracts:

| Surface | Version |
| --- | --- |
| NDJSON | `NDJSON_SCHEMA_VERSION = 2` |
| SDK / RPC | `SDK_SCHEMA_VERSION = 3` |

Use crate documentation and the emitted schema/version headers as the exact
contract. Do not infer compatibility from the workspace package version.

## Development and contributing

The workspace uses Rust edition 2024 and requires Rust 1.97 or newer.

```sh
cargo build
cargo run -p opi-coding-agent -- --help
cargo test --workspace --all-targets
cargo fmt --check --all
cargo clippy --workspace --all-targets -- -D warnings
```

Read [AGENTS.md](AGENTS.md) before changing the repository. It contains the
source-of-truth map, architecture boundaries, testing policy, and Git safety
rules. The durable technical direction is in
[docs/opi-spec.md](docs/opi-spec.md). Bug reports and feature discussions belong
in [GitHub Issues](https://github.com/OdradekAI/opi/issues).

## License

[MIT](LICENSE) © OdradekAI.
