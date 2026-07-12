# Phase 11 Tooling Quality Design

Historical note: this design was originally drafted as Phase 9. After the
`.repo/pi-0.80.2` baseline review, Phase 9 became the documentation/evidence
realignment gate and Phase 10 became core architecture deepening. Tooling
quality is now Phase 11 and depends on the Phase 10 harness/provider/session
seams.

## Overview

Phase 11 improves the built-in coding tools and their policy surface. Earlier
phases made packages and runtime behavior auditable; Phase 11 focuses on the
daily workhorse tools: `read`, `write`, `edit`, `bash`, `grep`, `find`, `ls`,
and `glob`.

The goal is not to add a large IDE layer. The goal is to make tool behavior
consistent, testable, safe to inspect, and predictable across Windows, macOS,
Linux, Unicode paths, large files, line endings, and cancellation.

## Goals

- Normalize built-in tool result shapes and diagnostics.
- Improve `edit` and `write` robustness around line endings, encodings,
  conflicts, and diff previews.
- Clarify `bash` execution policy, environment handling, timeout, truncation,
  cancellation, and mutating-tool classification.
- Align read-only navigation tools around ignore handling, path containment
  reporting, sorting, limits, and error messages.
- Add focused tests for Windows paths, symlinks where supported, Unicode, large
  output, binary files, and cancellation.
- Keep permission gates and environment-specific policies outside core, using
  allowlists and extension hooks instead.

## Non-Goals

- No built-in permission popup system.
- No persistent background bash.
- No remote execution core feature.
- No IDE project index.
- No language server integration.
- No automatic formatting on write/edit.
- No package ecosystem expansion.
- No new workflow tools such as todo, plan mode, or sub-agents in core.
- No sandbox implementation.

## Relationship to pi

Pi ships a small default tool set and expects workflow-specific behavior to be
customized through extensions. Phase 11 preserves that model. The Rust version
may improve correctness and safety around filesystem operations, but it should
not turn core tools into a broad IDE subsystem.

`glob` remains an opi convenience. Core workflows and docs should continue to
describe pi parity in terms of `read`, `write`, `edit`, `bash`, `grep`, `find`,
and `ls`, with `glob` documented as extra read-only navigation.

## Tool Result Contract

All built-in tools should provide consistent fields:

| Field | Meaning |
|---|---|
| content | LLM-visible text or image content |
| details | structured metadata for UI, JSON, RPC, and trace |
| is_error | whether the result represents a tool failure |
| diagnostics | optional structured diagnostics from Phase 7 |
| truncated | whether output was shortened |
| path metadata | normalized path, display path, workspace relation when applicable |

Tool outputs should be useful to the model but not hide important state from
the user. Destructive or mutating operations should show exactly what changed
through diffs or summaries where possible.

## Filesystem Tool Policy

Path-handling behavior should be consistent:

- resolve relative paths from the active workspace or tool cwd;
- preserve user-facing path strings where helpful;
- report whether a path is inside the workspace, outside the workspace, or
  unresolved;
- avoid following symlinks in surprising ways without reporting it;
- handle Windows absolute paths and drive prefixes;
- handle non-UTF-8 or invalid UTF-8 paths with clear diagnostics where the
  platform exposes them;
- distinguish not found, not a file, not a directory, permission denied, binary
  file, and unsupported encoding.

Phase 11 should not forbid access outside the workspace by default unless an
existing policy says so. Restriction belongs to tool selection, mutating opt-in,
extension hooks, containers, or future sandbox work.

## `read`

Improve or verify:

- line-range behavior;
- large file truncation;
- binary file detection;
- UTF-8 and lossy display policy;
- path metadata;
- stable error messages;
- JSON/RPC details.

## `write`

Improve or verify:

- create vs overwrite reporting;
- parent directory errors;
- newline handling;
- binary content policy;
- diff or size summary for user-visible audit;
- no silent partial writes;
- temp-file and rename strategy where appropriate.

## `edit`

Improve or verify:

- exact-match failure diagnostics;
- multiple-match behavior;
- CRLF/LF preservation;
- final newline preservation;
- conflict messages that show why an edit failed;
- diff preview consistency;
- large-file guardrails.

The edit tool should prefer clear failure over clever fuzzy behavior unless a
future explicit design introduces patch application or fuzzy edits.

## `bash`

Improve or verify:

- timeout and cancellation behavior;
- stdout/stderr truncation policy;
- full-output path behavior;
- cwd and environment reporting;
- exit code reporting;
- shell selection per platform;
- no secret leakage in diagnostics;
- mutating-tool classification;
- sequential execution mode.

Phase 11 must not add persistent background shells. Pi explicitly keeps
background bash out of core; users who need it should use tmux or a package.

## Read-Only Navigation Tools

For `grep`, `find`, `ls`, and `glob`, normalize:

- ignore-file behavior;
- hidden file defaults;
- sorting;
- result limits;
- symlink behavior;
- regex/glob parse errors;
- empty-result messages;
- workspace relation metadata;
- performance guardrails for large trees.

## Data Flow

```text
model tool call
  -> schema validation
  -> policy check
  -> tool execution
  -> structured result details
  -> Phase 7 diagnostic/trace mapping
  -> user-visible rendering and LLM-visible content
```

## Error Handling

Tool failures should throw or return error results according to the Phase 8
tool contract. The LLM should receive a clear error result, and the user should
receive enough detail to understand whether the issue is path, permission,
policy, timeout, cancellation, or unsupported input.

Do not return successful-looking content for failed operations.

## Testing Strategy

| Level | Coverage |
|---|---|
| unit | path normalization, truncation, diff metadata, line ending behavior |
| tempdir integration | read/write/edit/grep/find/ls/glob success and failure cases |
| platform-focused | Windows path parsing, CRLF, drive prefixes, shell selection |
| cancellation | bash timeout/cancel, large search cancellation where supported |
| JSON/RPC | tool result details and diagnostics shape |
| snapshot | diff and TUI rendering for changed files |

Tests should avoid destructive operations outside temp directories.

## Documentation Updates

Update tool docs and help output to clarify:

- which tools are read-only;
- which tools are mutating;
- how `--tools`, `--no-tools`, `--no-builtin-tools`, and `--allow-mutating`
  interact;
- what `bash` can execute;
- how truncation and full output paths work;
- why permission prompts are not a core feature.

## Success Criteria

Phase 11 is complete when:

1. Built-in tool result details follow a consistent contract.
2. `edit` handles CRLF/LF and conflict cases predictably.
3. `write` reports create/overwrite behavior and avoids silent partial writes.
4. `bash` timeout, cancellation, cwd, env, exit code, and truncation behavior
   are documented and tested.
5. Read-only navigation tools have consistent ignore, sorting, limit, and error
   behavior.
6. Tool diagnostics integrate with Phase 7 traces.
7. Tool scheduling respects Phase 8 runtime contracts.
8. No permission popup, background bash, remote execution, sandbox, or workflow
   tool is added to core.

## Phase 12 Handoff

Phase 12 should apply the same correctness discipline to provider adapters:
fixture-based tests, clear error taxonomy, stable diagnostics, and no feature
breadth unless a provider's existing supported behavior is wrong or ambiguous.
