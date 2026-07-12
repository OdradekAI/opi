# Phase 7 Reliability and Observability Design

## Overview

Phase 7 turns opi from a capable terminal coding agent into an agent whose
failures can be explained and reproduced. It builds on Phase 6 alignment
hardening by adding a uniform local diagnostics model and a minimal trace
envelope across provider requests, agent turns, tool calls, package adapters,
sessions, RPC, and config. Detailed runtime event semantics are finalized in
Phase 8 after the agent contracts are stabilized.

This phase does not add ecosystem breadth. It does not add npm packages,
provider OAuth, marketplace features, or a web product. Its purpose is to make
the existing Rust-native system easier to debug, test, support, and embed.

## Goals

- Define a shared diagnostic vocabulary for runtime, provider, tool, package,
  adapter, session, config, and RPC problems.
- Add a local turn trace envelope that captures enough stable structure to
  explain what happened without leaking secrets or full prompt content by
  default.
- Provide a top-level `opi doctor` command that summarizes local health across
  config, providers, sessions, packages, adapters, and terminal capabilities.
- Make retry, cancellation, compaction, adapter degradation, and provider
  failure visible in JSON/RPC and non-interactive diagnostics.
- Add focused tests for diagnostic shape, redaction, trace envelope emission,
  and doctor exit codes.
- Preserve pi's minimal-core philosophy: observability is local, explicit, and
  opt-in where it could expose sensitive data.

## Non-Goals

- No remote telemetry service.
- No analytics collection.
- No automatic session sharing.
- No package ecosystem expansion.
- No OAuth/provider breadth work.
- No new tracing backend dependency unless the existing `tracing` stack cannot
  satisfy the requirements.
- No full prompt or tool output capture by default.
- No web dashboard.
- No stable 1.0 observability protocol.

## Relationship to pi

Pi exposes rich event streams, RPC mode, session files, JSON mode, startup
messages, and package diagnostics. Phase 7 preserves that direction without
copying pi's TypeScript internals. The Rust version should make local failures
legible through typed diagnostics, versioned JSON, and a focused terminal
doctor command.

The phase should not copy pi's install telemetry or update-check behavior. If
future telemetry is ever considered, it needs a separate product and privacy
design.

## Architecture

| Module | Crate | Responsibility |
|---|---|---|
| Diagnostic types | `opi-agent` first, app-specific extensions in `opi-coding-agent` | Common severity, code, message, source, and redaction rules |
| Trace model | `opi-agent` | Minimal run/turn/provider/tool trace envelope and sink; Phase 8 owns detailed runtime semantic mapping |
| Doctor command | `opi-coding-agent` | Product-level health checks and terminal output |
| Provider diagnostics | `opi-ai` | Provider-specific error classification and safe metadata |
| Package/adapter diagnostics | `opi-coding-agent` | Existing package and adapter diagnostics normalized into the shared model |
| JSON/RPC exposure | `opi-agent`, `opi-coding-agent` | Versioned diagnostic and trace event payloads |

The shared diagnostic interface should be deep: callers should not assemble
ad-hoc strings when structured data can be preserved. Human formatting belongs
near the CLI, not at lower runtime layers.

## Diagnostic Model

Diagnostics should carry:

| Field | Meaning |
|---|---|
| `severity` | `info`, `warning`, or `error` |
| `code` | Stable snake_case identifier suitable for tests |
| `source` | Subsystem such as `provider`, `tool`, `package`, `adapter`, `session`, `config`, `rpc`, or `tui` |
| `message` | Human-readable short explanation |
| `details` | Optional redacted structured metadata |
| `action` | Optional next step when one is known |

Diagnostics must avoid secrets. API keys, bearer tokens, full environment
blocks, full prompts, full tool output, and absolute paths outside the relevant
workspace should not be emitted unless the user explicitly asks for verbose
debug output.

## Trace Model

Phase 7 introduces an unstable 0.x local trace envelope for a single run or
turn. Trace records should be append-only and ordered by a monotonically
increasing sequence.

Required envelope fields:

- schema version;
- run id;
- optional turn id;
- sequence;
- timestamp;
- source;
- kind;
- optional severity or diagnostic code;
- redacted structured details.

Required minimum records:

- run started and ended;
- turn started and ended;
- provider request, stream completion, retry, and failure when already
  observable through current provider paths;
- tool call started, completed, failed, or cancelled when already observable
  through current agent events;
- compaction, package adapter, and session diagnostics as diagnostic-linked
  trace records.

Phase 7 must not freeze detailed ordering for hooks, tool scheduling,
steering/follow-up queues, SDK/RPC command side effects, or future adapter UI
events. Phase 8 attaches the final semantic mapping to the envelope. When a
runtime event is not contract-stable yet, Phase 7 should emit a structured
diagnostic with a source and code rather than inventing a permanent trace kind.

Trace records should support two output modes:

| Mode | Behavior |
|---|---|
| summary | Redacted, safe by default, suitable for support reports |
| verbose | Includes additional local metadata but still redacts known secrets |

Phase 7 should not persist traces automatically forever. A trace is produced
only when requested by CLI flag, config setting, or RPC command.

## `opi doctor`

Add a top-level doctor command distinct from `opi package doctor`.

```text
opi doctor
opi doctor --json
opi doctor --scope config,provider,package,session,tui
```

Checks should include:

| Scope | Checks |
|---|---|
| config | layered config parse, selected model resolution, invalid keys, proxy settings |
| provider | selected provider credentials presence, endpoint shape, model capability metadata |
| package | delegate to installed package resolution and adapter diagnostics |
| session | session directory accessibility, recent corrupt-line recovery, storage permissions |
| tui | terminal capability summary, image protocol detection, color/no-color state |
| rpc | SDK/RPC schema version and startup diagnostics availability |

`opi doctor` should not make paid model calls or require network by default.
Optional network checks require an explicit flag in a later design.

Exit code policy:

| Result | Exit code |
|---|---:|
| no errors | 0 |
| warnings only | 0 |
| one or more errors | 2 |
| doctor command failed internally | 1 |

## JSON and RPC Exposure

JSON mode and RPC mode should expose diagnostics consistently:

- startup diagnostics appear before accepted prompt output;
- run summary includes diagnostic counts;
- trace events are versioned and distinguishable from normal agent events;
- unsupported trace requests fail with structured responses;
- diagnostic payloads have stable fields and tested redaction.

Do not change existing JSON/RPC semantics silently. Add fields in a
backward-compatible 0.x way and update schema/version documentation when
needed.

## Data Flow

```text
subsystem event or error
  -> structured diagnostic
  -> redaction
  -> CLI formatter / JSON event / RPC response / trace sink
```

```text
agent turn
  -> trace collector
  -> redacted local trace records
  -> optional file or stdout stream
```

## Error Handling

Diagnostics should be best-effort. A failure to collect one diagnostic must not
crash the agent unless it is itself the requested command, such as `opi doctor`.

Trace sinks should fail closed for file creation errors before the run starts,
then fail open during a run by emitting a diagnostic and disabling the trace
sink. Agent execution should not block indefinitely on trace output.

## Testing Strategy

| Level | Coverage |
|---|---|
| unit | diagnostic serialization, redaction, severity ordering, code stability |
| provider fixture | provider error classification without network |
| CLI integration | `opi doctor`, `opi doctor --json`, exit codes |
| JSON/RPC | startup diagnostics and trace envelope events are framed correctly |
| session tests | corrupt-line recovery diagnostics and storage errors with temp dirs |
| package tests | package diagnostics normalize into shared shape |

Do not add live provider tests as default gates. Live checks remain ignored and
environment-gated.

## Success Criteria

Phase 7 is complete when:

1. A shared diagnostic shape exists and is used by new Phase 7 surfaces.
2. `opi doctor` reports config, package, session, provider metadata, RPC, and
   TUI capability diagnostics without requiring network calls.
3. JSON/RPC startup and run summaries expose structured diagnostic counts.
4. A local redacted trace envelope can be requested for a run.
5. Retry, cancellation, compaction, adapter degradation, and provider failures
   are represented in diagnostics and, where contract-stable, trace records.
6. Redaction tests cover API keys, bearer tokens, environment values, prompt
   content, and tool output.
7. Documentation states that observability is local and explicit.
8. No telemetry, ecosystem expansion, OAuth, marketplace, or web dashboard is
   added.

## Phase 8 Handoff

Phase 8 should consume the Phase 7 diagnostic shape and trace envelope while
stabilizing runtime contracts. If Phase 8 changes event order, hook behavior,
queue behavior, or SDK/RPC commands, it must define the final diagnostics and
trace mapping on top of the existing envelope rather than adding parallel
ad-hoc event forms.
