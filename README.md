# opi

[![CI](https://github.com/OdradekAI/opi/actions/workflows/ci.yml/badge.svg)](https://github.com/OdradekAI/opi/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

> Rust AI agent toolkit and terminal-first coding agent inspired by
> [earendil-works/pi](https://github.com/earendil-works/pi).

opi is an embeddable, multi-provider coding-agent runtime you can drive as an
interactive TUI, a one-shot CLI, an NDJSON event stream, or an RPC harness.

[简体中文](README.zh.md) | [Changelog](CHANGELOG.md)

## Status

The workspace package version in `Cargo.toml` is `0.7.3`. This checkout may
contain unreleased changes on top of the published `0.7.3` crates; check
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

Inside the TUI, stored OAuth credentials are managed explicitly:

```text
/login anthropic
/login github-copilot
/login openai-codex
/logout <provider>
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
| [`opi-tui`](crates/opi-tui) | Ratatui widgets, transcript rendering, diff view, pickers, trust/permission prompts, terminal images, themes, and keybindings. |
| [`opi-coding-agent`](crates/opi-coding-agent) | The `opi` binary, built-in coding tools, config/session/package handling, and embeddable `CodingHarness`. |
| [`opi-protocol`](crates/opi-protocol) | Protocol types, bounded codecs, JSON schemas, and fixtures for `command.execute` (wire identity `command-execution-jsonl-v1`). |
| [`opi-sandbox`](crates/opi-sandbox) | Standalone, Opi-independent command-execution restriction package: L0 process-tree supervision, native Linux/macOS restriction, a library SDK, and a human CLI. |

Internal dependency shape:

```text
opi-ai
opi-tui
opi-agent -> opi-ai
opi-protocol
opi-sandbox -> opi-protocol
opi-coding-agent -> opi-ai + opi-agent + opi-tui + opi-protocol -> opi binary
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
| `--execution-strategy <fixed\|rules\|model>` | Select the `command.execute` routing strategy. |
| `--execution-backend <local\|ADAPTER_ID>` | Select the fixed execution backend or override the configured backend. |
| `--trust` / `--no-trust` | One-shot project-trust override for the session; mutually exclusive. |
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
| `github-copilot:` | One audited static catalog routed through Anthropic Messages, OpenAI Completions/Chat, and OpenAI Responses | OS keychain via `/login github-copilot` |
| `openai-codex:` | Dedicated OpenAI Codex Responses wire | OS keychain via `/login openai-codex` |
| configured profile | OpenAI-compatible Chat Completions profile | profile-specific `api_key_env` |

Compatible OpenAI-style services should normally use configured profiles rather
than new first-class provider modules. For `usage_in_stream`, OpenAI-compatible
profiles request `stream_options.include_usage`; response IDs captured from any
OpenAI Chat chunk carrying `id` round-trip into `response_id`. A profile may
also set `chat_completions_path` for base URLs that already include an API
prefix. Cost summaries are omitted when usage or pricing is unknown.

## Credentials, OAuth, and Provider Metadata

`opi-ai` defines the IO-free `CredentialStore`, `Credential`,
`OAuthProvider`, and `AuthResolver` contracts. `opi-coding-agent` supplies the
OS-keychain `CredentialResolver`, an env-var fallback for API keys, and one
secret-free coordination file (`credential.lock`). It never writes an
opi-managed plaintext credential file. `opi doctor` and `--list-models` probe
only redacted credential state. On Windows, macOS, and Linux, persisted
credentials use Windows Credential Manager, macOS Keychain Services, and
Freedesktop Secret Service, respectively.

GitHub Copilot uses the canonical `github-copilot` identity and one audited
static pi-0.80.6 catalog across Anthropic Messages, OpenAI Completions/Chat,
and OpenAI Responses routes. This intentionally differs from live account
entitlement filtering: `--list-models` reads the static catalog without an
OAuth secret or entitlement/model-enable request.

OpenAI Codex uses the canonical `openai-codex` identity, the dedicated
`openai-codex-responses` wire, and Browser (default) plus Device Code login.
Only Browser PKCE flows await a manual code or callback; GitHub Copilot and
OpenAI Codex Device Code call `present_device_code` and never
`await_manual_code`.

Persisted credentials use the native OS keychain; the development ids
`copilot` and `codex` have no alias or credential migration, so affected users
must log in again with the canonical id. After a pre-output
`CredentialNeeded`, a successful explicit login for the same provider makes
the outer TUI retry the same pending turn exactly once without appending a
duplicate user message. Non-interactive text, JSON, and RPC modes emit
canonical provider remediation and fail without constructing a
`LoginPresenter`, opening a browser, or waiting for input.
`CredentialRevoked` is non-retryable; opi does not auto-relogin mid-stream.
`AccountIdMissing { provider_id }` is a distinct non-retryable auth failure:
the credential exists but lacks the account identity required by the selected
wire. Before output, the outer TUI retains the turn and `/login <provider>` can
repair and retry it; text mode exits with the auth-failure code, while JSON and
RPC emit `CredentialNeeded` remediation with an `AccountIdMissing` diagnostic.

`Request` now carries `timeout`, `extra_headers`, `CacheRetention`, and
`session_id`. Only `session_id` has a production harness producer;
providers map it through their reviewed prompt-cache/session-affinity
rules. `ModelInfo` uses the single nested `ModelCapabilities` value, including
Anthropic cache-control capabilities. `cache_write_1h_tokens` remains a subset
of cache writes and `reasoning_tokens` a subset of output, so totals and costs
do not double-count them. `Provider::refresh_models` and collection refresh are
substrate-only with no production trigger.

## Built-in Tools

Available built-in tools are `read`, `write`, `edit`, `bash`, `grep`, `find`,
`ls`, and `glob`.

| Mode | Default tools |
| --- | --- |
| Interactive TUI | `read`, `write`, `edit`, `bash` |
| Non-interactive / RPC | `read`, `grep`, `find`, `ls`, `glob` |
| Non-interactive / RPC with mutating opt-in | `read`, `write`, `edit`, `bash` |

File writes and edits are scoped to the harness workspace root. After
`PathPolicy` accepts a workspace path, the local file backend resolves it
relative to a held workspace-root capability so an ancestor symlink or
junction swap cannot redirect the operation outside the workspace.
Interactive `read` can inspect explicitly allowed absolute paths outside the
workspace through the ambient filesystem. `bash` starts in the workspace root
but is not path-confined. These are tool-policy and file-operation hardening
measures, not an operating-system sandbox.

Tool results carry LLM-visible `content`, optional structured `details`,
`is_error`, `terminate`, `truncated`, and optional diagnostics. `read` returns
line/path metadata and defaults to a 2000-line preview with a 64 KiB byte cap.
`bash` runs one foreground command, caps combined stdout/stderr at 64 KiB, and
may spill complete output to `details.full_output`.

## Config and Sessions

Config layers merge user config, an authorized project config, and an explicit
`--config` file. Project config is not read until startup trust resolution
allows it; an explicit `--config` remains user-authorized. Model precedence is:

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
events, state, and model/provider overrides. A package can also declare a
`command.execute` adapter (wire identity `command-execution-jsonl-v1`) that the
`bash` tool selects through the execution backend; see
[Command Execution and opi-sandbox](#command-execution-and-opi-sandbox) below.

## Command Execution and opi-sandbox

Phase 16 adds a pluggable `command.execute` capability for the model-callable
`bash` tool. The default `opi` process stays in the Minimal Runtime on a direct
local execution path; it can instead select an installed external adapter
through the execution backend.

- Five independent lifecycle gates: Installed, Trusted, Enabled, Selected, and
  Permitted. Installing a package never trusts or enables it: `opi package add
  <source>` installs, `opi package enable <name>` grants Package Trust and
  enables, and user permission policy grants per-invocation or persistent
  approval. Project-local executable/process package contributions are
  rejected; install globally, review, and enable.
- Routing and permission: `[execution] strategy = "fixed"|"rules"|"model"` with
  `[execution] backend = "local"|<adapter-id>` (or `--execution-strategy` /
  `--execution-backend`) selects an eligible adapter. `rules` matches in order
  and fails closed rather than falling through; `model` routing proposes a
  backend under user policy. Permission outcomes are `deny`, `ask`, and
  `allow`; a project layer may not set `[execution.permissions]`.
- No fallback: once an external adapter is selected, failure is fail-closed and
  never retries through `local`. Stable redacted failure codes (for example
  `package_not_installed`, `permission_required`, `protocol_violation`) carry
  actionable remediation on text, NDJSON, RPC, and interactive surfaces;
  `package doctor` and `opi doctor` preserve the same actionable lifecycle
  codes and remediation used by runtime execution.
- The Opi binary never links `opi-sandbox`. Native restriction and its
  helper/capability-selection code left the core (16.16.1); `[sandbox]`,
  `--sandbox`, and `--sandbox-require` are rejected without compatibility
  aliases. L0 subprocess-tree supervision stays in core for local and adapter
  processes.
- `opi-sandbox` is a standalone crate (library SDK plus human CLI) that depends
  only on `opi-protocol` and has no Opi configuration, session, or package-store
  dependency. It offers `opi-sandbox run --workspace <PATH> --profile
  workspace-write ...`, a `backend --stdio` protocol peer, and
  `opi-sandbox doctor --json`, and confines only the target process tree (it is
  not a security boundary). Linux uses Landlock plus a fixed seccomp danger
  blocklist (with `network = deny` new-socket/TCP restrictions); macOS uses
  `sandbox-exec` with writes confined to the workspace and invocation
  temporary roots, failing closed when the helper is missing or rejected;
  Windows Job Objects provide L0 supervision only, and no official Windows
  `opi-sandbox` artifact is published.
- `opi-protocol` owns only the versioned `command-execution-jsonl-v1` execution
  protocol. `opi-sandbox` release archives are built for Linux and macOS; the
  ordinary `opi` binary keeps its six release targets.
- Phase 16 non-goals (see the spec for the full list): Docker/VM/SSH/Gondolin
  or remote adapters; routing file, navigation, or other built-in tools;
  extensions replacing a core tool by name; a universal extension protocol or
  migration of `opi-extension-jsonl-v1`, RPC, NDJSON, or trace envelopes;
  dynamic native-library loading; composing multiple adapters for one
  invocation; host-read or environment-variable confidentiality; sandboxing the
  extension process; publisher authentication; project-local executable
  contributions; Windows AppContainer or restricted-token restriction; and
  preserving unreleased Phase 15 sandbox configuration aliases.

## Permissions and Trust Boundaries

`opi` runs with the operating-system permissions of the user and process that
launched it. Tool selection and mutating-tool flags control which built-in
tools the agent can call; they are not an operating-system sandbox.

- `bash` can execute commands with the launching user's OS permissions.
- Packages are trusted code. A package can start child processes with the same OS permissions as `opi`; package permission declarations are metadata, not enforced sandbox policy.
- Observability is local and explicit: `opi` does not collect telemetry or
  analytics, does not share sessions automatically, `opi doctor` is local and
  network-free by default, and `trace` is opt-in.
- Production sub-agent, plan/todo, MCP, and general-purpose permission-gate
  workflows beyond `command.execute` are examples/package patterns, not
  built-in core workflows.
- OAuth providers beyond Anthropic, GitHub Copilot, and OpenAI Codex; provider
  catalogs beyond the two audited pi-0.80.6 snapshots; image generation;
  browser automation; provider
  streaming-adapter protocols for packages; paid live provider calls in
  default tests; and copying pi's provider-specific config file format remain
  deferred.
- Dynamic Rust plugin loading from arbitrary extension paths is not supported.

### Project Trust

- Project trust is resolved once at startup, before any project resource or
  project config consumer (including `doctor` and `--list-models`) runs. The
  store is a flat `Map<canonical_path, bool>` at
  `{user_config_dir}/trust.json` (`%APPDATA%\opi\trust.json` on Windows,
  `~/.config/opi/trust.json` on Unix), with no schema version. When a project
  is untrusted, its `.opi/config.toml`, `.opi/{skills,fragments,themes,
  extensions}`, project-scope `.opi/packages.toml` adapter declarations, and
  project `AGENTS.md`/`CLAUDE.md` do not load (the context files remain
  readable via the `read` tool). Trust gates resource *loading*, not tool
  execution. CLI: `--trust` / `--no-trust`; `[defaults]
  default_project_trust = "ask"|"always"|"never"` (default `ask`, global-only).
- There is no built-in `/trust` command, no live mid-session trust mutation,
  and no project-resource reload. Trust resolvers are registered through an
  explicit embedder-only API; the standard CLI ships an empty resolver
  registry, exposes no CLI `-e` flag, and performs no native resolver loading.

If you need stronger isolation, run `opi` inside a container, VM, or external
sandbox appropriate for the tools and credentials you expose to it.

## Development

Rust 1.97 or newer is required (workspace MSRV; the workspace uses edition 2024).

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
