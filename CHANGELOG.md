# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Breaking Changes

- Raised the workspace Minimum Supported Rust Version (MSRV) from 1.85 to 1.97 (`rust-version` in `[workspace.package]`, inherited by all crates). Builds now require Rust 1.97 or newer; the workspace remains on edition 2024.
- `opi-ai` 0.x API: `Request` adds `timeout`, `extra_headers`, `cache_retention`, and `session_id`; `Provider` adds the object-safe `refresh_models` and `replace_model_catalog` methods; `ModelInfo` replaces flattened capability fields with one nested `ModelCapabilities` and adds exact `WireApi`, thinking-map, wire-compatibility, and pricing metadata; `Usage.cache_write_1h_tokens` and `Usage.reasoning_tokens` are corrected from `u32` to `Option<u64>` so absent and explicit zero remain distinct; and `CostBreakdown` removes the separate `cache_write_1h_cost` field. Downstream struct literals and custom provider implementations must be updated.
- The development OAuth provider ids `copilot` and `codex` are replaced by canonical `github-copilot` and `openai-codex`. There is intentionally no config alias or keychain credential migration; affected users must log in again with the canonical id.
- `opi-agent` 0.x API: `AgentError` gains an `AccountIdMissing { provider_id }` variant for provider credentials that are present but lack a required account id (e.g. an OpenAI Codex token without `chatgpt_account_id`). The enum is exhaustive, so downstream exhaustive matches on `AgentError` must add the arm. JSON, RPC, and text modes surface it as a typed `/login <provider>` remediation with an AuthFailure exit code, distinct from `CredentialRevoked`.

### Added

- `opi-ai`: IO-free, object-safe `CredentialStore`, `Credential`, `CredentialSource`, `OAuthProvider`, `OAuthCredential`, `LoginPresenter`, `AuthResolver`, and `ResolvedAuth` contracts; secret-free `AuthDescriptor::StoreCredential`; and typed non-retryable `CredentialNeeded` / `CredentialRevoked` provider failures.
- `opi-coding-agent`: OS-keychain credential persistence using Windows Credential Manager, macOS Keychain Services, or Freedesktop Secret Service across the six release targets, with env API-key fallback, cross-process mutation locking, redacted doctor/model-list probes, and explicit interactive `/login <provider>` / `/logout <provider>` flows for Anthropic Browser PKCE, GitHub Copilot Device Code, and OpenAI Codex Browser (default) or Device Code. Device-code flows present the public device code and never request manual paste-back. Non-interactive, JSON, and RPC modes report provider-specific `/login` remediation without constructing a presenter or starting OAuth.
- `opi-ai` / `opi-agent` / `opi-coding-agent`: request timeout and extra-header wire handling; production `session_id` propagation with provider-specific cache-affinity mappings; nested cache capabilities and Anthropic cache markers; and cache-write/reasoning usage accounting without double counting.
- `opi-ai`: deterministic atomic dynamic-model catalog replacement through `Provider::refresh_models` and `ProviderCollection::refresh`. This remains substrate-only with no production trigger.
- `opi-ai` / `opi-coding-agent`: public `ApiMappedProvider` and `[providers.custom.<id>]` multi-wire configuration, exact `WireApi` routing, model/provider API and base-URL precedence, wire-tagged compatibility metadata, thinking maps, model pricing tiers, shared lazy credentials, and reserved managed-header validation.
- `opi-coding-agent`: audited static pi-0.80.6 GitHub Copilot catalog mapped across Anthropic Messages, OpenAI Completions/Chat, and OpenAI Responses, plus the dedicated `openai-codex-responses` provider and audited OpenAI Codex catalog.

### Changed

- Concrete Anthropic, all three GitHub Copilot routes, and dedicated OpenAI Codex Responses re-resolve authentication inside every returned stream. TUI `/help` discovers `/login` and `/logout` through the production dispatcher; after a pre-output `CredentialNeeded`, only a successful explicit login for the same provider retries the same pending turn, exactly once and without a duplicate user message. Missing or revoked credentials never auto-login.
- `CostBreakdown` remains separate and `Copy` with four public lines (`input_cost`, `output_cost`, `cache_read_cost`, and `cache_write_cost`): the weighted one-hour cache-write subset is folded into `cache_write_cost`, reasoning stays inside `output_cost`, and neither subset is counted twice.

### Fixed

- `opi-ai`: enforce selected-model capability preflight on every public dispatch path, collect complete atomic refresh batches, apply Anthropic compatibility metadata, preserve initial Chat tool arguments, materialize mapped-provider catalog overrides, and calculate cumulative cost from exact `u64` totals.
- `opi-coding-agent`: reject ambiguous provider identities, preserve foreign process-default keyring ownership, strictly decode credential envelopes, serialize OAuth refresh against public writes, and exercise the production TUI event dispatcher in debug and release test profiles.

## [0.7.0] - 2026-07-09

### Added

- `opi-coding-agent`: `--json-compact` opt-in flag that makes streamed `text_delta` updates constant-size (~linear byte cost for long streamed turns). It omits the redundant `assistant_event.partial` snapshot and empties the cumulative text in `event.message` for those updates. Default `--json` output and `NDJSON_SCHEMA_VERSION = 2` are unchanged.
- Repository: `scripts/opi-artifact-audit.py` (plus `.ps1`/`.sh` wrappers) — a deterministic, network-free checker for saved runtime, NDJSON, session, provider, and browser evidence (workspace-root leakage, all-zero timestamps, provider-turn mismatch, duplicate `text_delta` partials, failure claims without preserved artifacts) — paired with an `opi-implement` Artifact Truthfulness Gate (Phase D.0a) requiring verified artifacts for runtime/CLI/NDJSON/session claims. Backed by a fixture-driven `opi-coding-agent` integration test.

### Changed

- `opi-coding-agent`: `session_summary` now includes `provider_turns` (provider request/response cycles, i.e. `TurnStart` events) alongside the existing `turns` (user prompt turns), so a tool-using prompt usually reports `provider_turns > turns`.
- `opi-release`: pre-flight checks are documented as deterministic and must not depend on live provider or dogfood runs; any cited dogfood evidence must already have passed the `opi-implement` Artifact Truthfulness Gate.
- Bumped the workspace version to `0.7.0` and refreshed the Phase 4 and Phase 6 specification-hash ledgers to match the current `docs/opi-spec.md`.
- This release publishes the publishable crates to both GitHub Releases and crates.io in dependency order.

### Fixed

- `opi-ai` / `opi-coding-agent`: support custom OpenAI-compatible chat completions endpoint paths for providers whose `base_url` already includes a provider-specific API prefix (e.g. BigModel `/api/paas/v4/...`), configured via `chat_completions_path` on an `openai_compatible` profile. Default OpenAI/OpenRouter/Mistral behavior still posts to `/v1/chat/completions`.
- `opi-ai` / `opi-agent` / `opi-coding-agent`: populate runtime message timestamps instead of emitting `timestamp_ms: 0` for runtime-produced user, assistant, and tool result messages.
- `opi-coding-agent`: avoid exposing resolved absolute workspace paths in successful `read` tool output text and structured `details` for inside-workspace reads.

## [0.6.5] - 2026-07-06

### Added

- `opi-agent`: Phase 13 session entry model. The session journal gains typed, forward-compatible entries (`session_info`, `model_change`, `thinking_level_change`, `label`, `branch_summary`) layered additively on the v1 header, with no automatic migration. `SessionReader` now distinguishes unknown future entry types (reported via `CrashRecovery::unknown_count`) from genuine corruption, and treats a JSON object lacking a `type` field as corrupt. `CrashRecovery` is now a struct `{truncated_line, corrupt_count, unknown_count}`. `SessionFacade` gains `enqueue_*` metadata methods that parent entries to the active content tip without advancing it.
- `opi-agent`: reusable session context reconstruction. `reconstruct_context(&[SessionEntry], &CrashRecovery) -> ReconstructedContext` deterministically walks the active branch by `parent_id`, applies compaction and `branch_summary` entries at their documented positions, collects model/thinking/session_name/labels/extension_state metadata parented to the active chain, and emits diagnostics for missing parents plus forwarded corrupt/truncated/unknown observations.
- `opi-coding-agent`: interactive session metadata commands. `/name`, `/label`, `/unlabel`, and `/session info` (plus RPC `session_info`) append and read typed `session_info`/`label` entries through `SessionCoordinator` parented to the active content tip without advancing it; labels and `session_info` never enter provider context.
- `opi-coding-agent`: local session export. `opi --export-session <id-or-path> --format markdown|json [--output ...]` supports active-branch and full-tree scopes, include/exclude flags for tool output and thinking content, and a Phase 7 redaction mode (`summary|verbose|none`). Dispatch runs before config/provider construction (network-free); the source session is opened read-only and stays byte-for-byte identical on success and failure.
- `opi-coding-agent`: session/tree handoff metadata. RPC `session_info` now emits active-branch `entry_count` and a redacted `branch_summary`, plus a `branches[]` array of `{tip, summary, entry_count, depth, active}` built from `SessionTree`, so embedders can render branch/session pickers without re-parsing JSONL. `session_picker_items` prefixes the typed session name to the display line and appends the label set to metadata.
- `opi-agent` / `opi-coding-agent`: resume and fork re-apply recorded model and thinking metadata when provider-compatible (`ResumeInfo` carries `recorded_model`/`recorded_thinking`), and `--list-sessions --json` emits deterministic rows carrying name, labels, active branch, model, and thinking metadata reconstructed from the active branch.

### Changed

- `opi-coding-agent`: resume, fork, branch-select, non-interactive resume, and `--list-sessions` now route through `opi_agent::session_context::reconstruct_context` instead of a duplicated product-only JSONL walker, which is retired.
- Documented the session format/version policy (v1 header kept, additive entries, no automatic migration), the unknown-future-entry vs corrupt recovery split, and local export/redaction/sensitivity behavior in `docs/opi-spec.md` and the crate READMEs (EN + ZH), with Phase 13 non-goal guard tests pinning the boundaries.
- Bumped the workspace version to `0.6.5` and refreshed the Phase 4 specification-hash ledger to match the current `docs/opi-spec.md`.
- This release publishes the publishable crates to both GitHub Releases and crates.io in dependency order.

### Fixed

- `opi-agent` / `opi-coding-agent`: aligned active-branch reconstruction across resume, fork, export, and session metadata, including invalid leaf fallback, rootless pre-turn metadata, and legacy no-leaf multi-root sessions.
- `opi-agent` / `opi-coding-agent`: surfaced stale `Leaf` fallback diagnostics, preserved metadata parented through active metadata entries during fork/resume seeding, shared harness session-tree reads across branch/RPC paths, and kept verbose session exports from redacting non-secret path evidence.
- `opi-coding-agent`: persisted model and thinking changes before mutating runtime state, forwarded stored branch summaries to providers on resume, redacted branch summaries and custom export messages, and kept RPC/session diagnostics from duplicating or hiding recovery errors.
- `opi-coding-agent`: handled empty `/name`, `/label`, and `/unlabel` commands locally instead of forwarding them to the model; `--format md` is now accepted as a documented export alias.

## [0.6.4] - 2026-07-05

### Added

- `opi-ai`: provider error taxonomy. `ProviderError::category()` returns one of nine documented `ProviderErrorCategory` classes (`Auth`, `Config`, `Request`, `Network`, `RateLimit`, `Provider`, `Stream`, `Capability`, `Cancelled`), distinguishing local pre-request failures from provider 4xx/5xx responses. Provider-side error diagnostics are redacted before reaching public surfaces.
- `opi-ai`: OpenAI-compatible provider profiles accept a `CompatConfig` (per-model `ModelCompatOverride` for role and max-tokens overrides, plus custom `extra_headers`) so Azure/OpenRouter/Mistral-style profiles can carry documented compatibility flags.

### Changed

- `opi-ai`: OpenAI-compatible streaming usage now requests `stream_options.include_usage` when `usage_in_stream` is enabled, preserves usage updates from any streaming chunk, and records OpenAI Chat response IDs from any chunk carrying `id`, not only role chunks.
- `opi-agent`: retry diagnostics now distinguish exhausted retry budgets from retry suppression after partial provider output.
- `opi-ai` / `opi-coding-agent`: missing usage is tracked explicitly as unknown instead of known-zero usage; session cost summaries are omitted when any turn has unknown usage or when pricing is unknown.
- Bumped the workspace version to `0.6.4` and refreshed the Phase 4 and Phase 6 specification-hash ledgers to match the current `docs/opi-spec.md`.
- This release publishes the publishable crates to both GitHub Releases and crates.io in dependency order.

### Fixed

- `opi-ai`: OpenAI Responses tool-call deltas and item completion now route by output item identity instead of the last observed tool call.
- `opi-ai`: OpenAI Responses text lifecycle events now report the actual content index when text follows a tool call.
- `opi-ai`: Bedrock HTTP streaming now flushes a pending terminal `Done` event when metadata is absent.
- `opi-agent`: provider-returned cancellations now surface as `AgentError::Cancelled`.
- `opi-agent`: compaction and session-persistence public events redact secret-looking error text.

## [0.6.3] - 2026-07-01

### Added

- `opi-agent`: the agent loop lifts each tool-owned `ToolDiagnostic` into a Phase 7 `Diagnostic` (per-cause `CODE_TOOL_*` code + structured context) and mirrors it as a diagnostic-linked trace record, instead of collapsing every tool failure to a generic `tool_execution_failed` carrying only the tool name. `bash` failure diagnostics (nonzero exit, timeout, cancellation) now carry operation context (exit_code/cancelled/timed_out/truncated; raw command omitted).
- `opi-agent`: `AgentEvent::ToolExecutionEnd` exposes a `diagnostics` array on the JSON/NDJSON and RPC output paths (additive, `skip_serializing_if` empty; old payloads round-trip via `#[serde(default)]`). The provider-facing `ToolResultMessage` is unchanged.

### Changed

- Clarified Phase 11 tool-result, event, and session metadata contracts in the normative specs, including public redaction boundaries for tool details and diagnostics.
- `opi-coding-agent`: exhausting `max_turns` with tools still pending now returns `AgentError::MaxTurnsExceeded` and emits a `agent_max_turns_exceeded` warning diagnostic + trace, instead of silently returning `Ok(messages)`. Non-interactive/RPC runs that previously exited `0` on turn-cap exhaustion now exit `RuntimeFailure` (`1`). Runs that exhaust without pending tools (e.g. steering-driven continuation, zero-turn runs) still complete normally.
- Bumped the workspace version to `0.6.3` and refreshed the Phase 4 specification-hash ledger to match the current `docs/opi-spec.md`.
- This release publishes the publishable crates to both GitHub Releases and crates.io in dependency order.

### Fixed

- `opi-agent`: redacted command/path-sensitive tool metadata before public events and session persistence while preserving provider-facing tool result content.
- `opi-coding-agent`: hardened Phase 11 built-in tool behavior for read byte caps and line-ending metadata, write-to-directory diagnostics, bash process/temp reliability, and bounded navigation work with skipped-file diagnostics.
- `opi-coding-agent`: corrected CLI help and README descriptions for Phase 11 tool policy, `glob` framing, and unique-match edit behavior.
- `opi-ai`: provider wire-converters now preserve failed tool-result semantics instead of making failure indistinguishable from success. Anthropic emits native `is_error: true` on the `tool_result` content block; OpenAI Chat (incl. Azure/OpenRouter/Mistral via the shared adapter) and OpenAI Responses prefix a deterministic `[tool_error] ` marker to the tool-output string (neither API has a native error field, and Responses does not accept a client-set `status` on input items); Gemini (incl. Vertex) sets `error: true` inside the `functionResponse.response` Struct. Bedrock already used native `toolResult.status`. The `is_error: false` body is byte-identical to the pre-fix shape on every provider.
- `opi-agent`: the `MaxTurnsExceeded` classification (previously dead code) is now constructed and classified at runtime. Tool-failure diagnostics now surface the per-cause filesystem/error code from the 11.2 taxonomy rather than the single generic collapse.
- `opi-coding-agent`: marked the `#[cfg(unix)]` `getuid()` permission-skip-guard `extern "C"` blocks in the find/ls/read-write-edit-bash tests as `unsafe extern` so they compile under the edition-2024 enforcement of newer Rust stable.

## [0.6.2] - 2026-06-28

### Added

- `opi-ai`: provider collection and authentication seam — a dedicated collection/auth runtime that lets model/auth ownership move out of ad hoc provider construction.
- `opi-agent`: generic `AgentHarness` seam separating generic phase/snapshot/session orchestration from the product-specific coding harness.
- `opi-agent`: session repository/facade seam (`SessionFacade`, `SessionRepo`) giving richer session context a first-class entry point instead of CLI-only writes.

### Changed

- `opi-coding-agent`: provider construction centralized into `provider_factory`; when a configured HTTP proxy fails to build, provider construction and `--list-models` now surface `failed to build HTTP client with proxy config: <cause>` instead of the bare cause string (message-wording change only).
- `opi-coding-agent`: `CodingHarness` documented as a product wrapper over the generic `opi-agent` seams, with runtime hook boundaries documented (Phase 10 WS10.4).
- Added Phase 10 documentation guards and an exit-trace completeness gate, and refreshed the root and crate READMEs, `opi-spec`, and the pi alignment matrix for the post-Phase-10 current state.
- Bumped the workspace version to `0.6.2` and refreshed the Phase 4 specification-hash ledger to match the current `docs/opi-spec.md`.
- This release publishes the publishable crates to both GitHub Releases and crates.io in dependency order; it is the first crates.io release since `0.5.4` (`0.6.0` and `0.6.1` were GitHub-only documentation/guard-test releases).

### Fixed

- Addressed Phase 10 audit findings across the centralized provider factory and surrounding documentation.

## [0.6.1] - 2026-06-25

### Added

- `opi-coding-agent`: Phase 9 pi 0.80.2 baseline documentation guard tests pin the durable alignment-matrix evidence baseline, the normative specification, and the pi alignment matrix against `.repo/pi-0.80.2` as the current studied upstream, keep the Phase 9-14 roadmap consistent across English and Chinese counterparts, and reject current-scope overclaims for deferred ecosystem breadth (OAuth parity, image generation, custom extension UI parity, npm/gallery, web/share, and pi session compatibility).

### Changed

- Recorded the Phase 9.1 alignment-matrix, Phase 9.2 normative-specification, and Phase 9.3 supplemental-design baseline evidence, and archived the opi-implement Phase 9 ledger snapshot.
- Bumped the workspace version to `0.6.1` and refreshed the Phase 4 specification-hash ledger to match the current `docs/opi-spec.md`.
- This release is published to GitHub only; crates.io publishing is intentionally skipped because it contains documentation and guard-test changes only.

## [0.6.0] - 2026-06-24

### Changed

- Realigned the implementation roadmap against the `pi` 0.80.2 evidence baseline, adding Phase 9 baseline realignment, Phase 10 architecture deepening, and refreshed Phase 11-14 planning documents.
- Updated the English and Chinese technical specification and pi alignment matrix to reflect the current roadmap and phase boundaries.
- This release is published to GitHub only; crates.io publishing is intentionally skipped because it contains planning and documentation changes only.

## [0.5.4] - 2026-06-24

### Added

- `opi-agent`: README (English and Chinese) classifies the public runtime, extension, event, session, SDK/RPC, and streaming-proxy surfaces as supported 0.x or unstable internal, with their stability mechanism (`#[non_exhaustive]`, module `# Unstable` prose, and the `SDK`/`NDJSON`/`TRACE` schema versions) and an explicit Phase 8 non-goal list; the pi alignment matrix records a Phase 8 runtime-stabilization row. Guard tests pin the classification against the crate-root re-exports and reject the Phase 8 non-goals.

### Changed

- `opi-coding-agent`: RPC JSONL synchronous rejection responses for runtime-contract failures now carry a stable machine-readable `error_code` — `agent_busy`, `harness_unavailable`, `compaction_failed`, `extension_command_not_handled` (alongside the existing `unsupported_trace_request`) — on the additive `SdkResponse::error_code` field. The SDK schema version is unchanged at `3`; idle `set_model` / `set_thinking_level` capability errors remain free-text.

### Fixed

- `opi-coding-agent`: resumed session-recovery diagnostics now reach the in-process diagnostic recording sink (and are counted by run summaries) instead of only `session_info` resource metadata, matching how compaction is already wired.
- `opi-agent`: parallel tool-result handling now satisfies newer Clippy releases by removing a redundant iterator conversion.

## [0.5.3] - 2026-06-22

### Added

- `opi-agent`: shared diagnostic vocabulary (`Diagnostic`, `DiagnosticPayload`, `Severity`, `RedactionMode`, `redact`/`redact_text`, `DiagnosticSink`, `RecordingSink`, `NullSink`) with deterministic, ordered serialization and Summary/Verbose redaction reusing `SecretRedactor`.
- `opi-agent`: provider, retry, cancellation, tool, compaction, session-recovery, package/adapter, config, and RPC paths now record structured diagnostics instead of bare strings.
- `opi-ai` / `opi-agent`: `ProviderErrorCategory` taxonomy (`Auth`, `RateLimit`, `Timeout`, `Request`, `Stream`) with `ProviderError::category()` and `retry_after_ms()` accessors, mapped into the shared diagnostic code/severity/source triple for consistent redacted reporting across stderr, JSON, and trace surfaces.
- `opi-agent`: redaction core extended with GitHub PAT, credentialed-URL userinfo, and `Authorization` header patterns shared by all diagnostic surfaces, plus Phase 7 redaction/shared-shape/non-goal guard tests.
- `opi-agent`: unstable local trace envelope substrate — `TRACE_SCHEMA_VERSION`, non_exhaustive `TraceKind`, Serialize-only `TraceRecord`, `TraceSink` trait with fail-closed `prepare` and fail-open `write`, `TraceCollector` with redaction, and a crash-resilient `FileTraceSink` exposed for embedders.
- `opi-agent` / `opi-coding-agent`: trace envelope wired into the agent loop; run/turn/provider/tool records are emitted via `observe()`; opt-in `--trace <path>` writes a redacted envelope for non-interactive and JSON modes (interactive/RPC excluded).
- `opi-agent`: RPC JSONL gains a `trace` command returning the versioned, redacted envelope; the RPC runner records a `RecordingTraceSink` by default; `SdkResponse` carries a new additive `error_code` field for machine-readable `unsupported_trace_request` errors.
- `opi-agent`: NDJSON mode gains a `StartupDiagnostics` event emitted before `AgentStart` and an additive `diagnostics: SessionDiagnosticCounts { info, warning, error }` tally on `SessionSummary`, both omitted when absent.
- `opi-coding-agent`: top-level `opi doctor` local health check, distinct from `opi package doctor`; network-free, reports shared `Diagnostic` values for `config`, `provider`, `package`, `session`, `tui`, and `rpc` scopes with `--json` NDJSON output and `--scope` filtering, redacting absolute paths at the boundary; exits `0` clean, `2` on any error-severity diagnostic, `1` on internal or argument failure.

### Changed

- `opi-agent` / `opi-coding-agent`: SDK/RPC schema version is now `3` and NDJSON schema version is now `2` to carry the new trace and diagnostic fields; both remain unstable 0.x contracts and existing consumers keep parsing via additive, `#[serde(default)]` fields.

### Fixed

- `opi-coding-agent`: non-interactive JSON mode provider-error stderr now routes through the shared diagnostic redactor instead of emitting raw error strings, keeping a static `provider error` class string.

### Removed

- `opi-web-ui`: removed the unpublished web-facing crate from the workspace; future web UI work should be planned as a separate RPC/SDK consumer surface.

## [0.5.2] - 2026-06-17

### Fixed

- `opi-coding-agent`: RPC JSONL mode now surfaces provider and harness construction diagnostics at startup and documents the session JSONL format as an unstable 0.x contract instead of implying stability.
- `opi-coding-agent`: `opi package` runtime degraded paths (adapter, lock, source, and resource failures) now report actionable diagnostics instead of failing silently.
- `opi-coding-agent`: the process-JSONL adapter protocol (`opi-extension-jsonl-v1`) is documented honestly as an unstable 0.x protocol, and adapter startup diagnostics are enriched.
- `opi-coding-agent`: Phase 6 documentation-truth and reliability audit gaps are closed; current-state docs, the spec hash ledger, and the English/Chinese counterparts now stay synchronized with the workspace version, guarded by Phase 6 alignment tests.

## [0.5.1] - 2026-06-15

### Added

- `opi-coding-agent`: `--fork <session-id>` plus interactive `/tree`, `/fork`, and `/clone` session commands that copy the active branch into a new parented session without rewriting the source JSONL file.
- `opi-agent` / `opi-coding-agent`: RPC/SDK `extension_command` support for dispatching correlated custom commands to registered extension registries.
- `opi-coding-agent`: config-driven OpenAI-compatible provider profiles with model metadata, compatibility flags, runtime provider construction, and registry-backed `--list-models` output.
- `opi-web-ui`: `ConversationState` now tracks resource metadata from `session_info` responses and the last successful compaction response payload.
- `opi-coding-agent`: runtime session persistence now writes meaningful `parent_id` links and `leaf` pointers so continuing from a selected branch tip creates a same-file branch path.
- `opi-coding-agent`: `opi package add/remove/list/doctor` now validates package manifests, writes lock entries, and reports installed package diagnostics.
- `opi-coding-agent`: manifest V2 supports `[adapter]` process adapters with the `opi-extension-jsonl-v1` JSONL protocol.
- `opi-coding-agent`: installed package declarations are loaded during runtime startup so adapter tools, commands, hooks, events, state, and cancellation bridge into the extension API.
- `opi-coding-agent`: example adapter packages demonstrate todo state, permission-gate example hooks, and protected path hooks through a runnable process adapter.

### Changed

- `opi-agent`: moved the core loop implementation out of `lib.rs` into an internal `agent_loop` module while preserving the public `opi_agent::agent_loop` export.

### Fixed

- `opi-tui`: `SelectList` and `BranchPicker` now account for selected-row markers and CJK display width when aligning labels with metadata.
- `opi-coding-agent`: `opi package doctor` now rejects invalid manifest V2 adapter declarations and reports lock/source/resource/adapter diagnostics.
- `opi-coding-agent`: Adapter state snapshots are persisted in session JSONL and restored on resume.
- `opi-coding-agent`: Adapter event drops are diagnostic-visible, shutdown allows a bounded graceful exit, local package identity is canonicalized, SSH git source parsing is URL-aware, and relative adapter commands cannot escape package roots.
- `opi-coding-agent`: Linux build and test correctness — removed a dead Unix-only import that failed `clippy`/`test`/`doc` under `-D warnings`, and test-binary locators no longer match cargo `.d` dep-info siblings (which lack the execute bit and caused `EACCES` when spawning adapters).

## [0.5.0] - 2026-06-07

Phase 4: extension system, RPC JSONL protocol, SDK embedding surface,
progressive resource discovery, session branching, streaming proxy,
custom provider registration, and six extension examples.

### Added

- `opi-coding-agent`: RPC JSONL mode with correlated responses, async agent events, session/model/thinking/compaction commands, and tool-selection support.
- `opi-agent`: shared unstable SDK command/response/event types for embedders.
- `opi-agent`: extension API with lifecycle hooks, custom tools, custom commands, custom messages, and extension state.
- `opi-ai`: custom provider/model registry APIs used by CLI model listing and runtime validation.
- `opi-coding-agent`: config-driven discovery for extensions, packages, skills, prompt fragments, and themes, including package-composed resource layers.
- `opi-coding-agent`: interactive `/branch` session branch selection.
- `opi-agent`: streaming proxy primitives with framing, cancellation, backpressure, and secret redaction.
- `opi-web-ui`: unpublished RPC/SDK event parser, conversation state, component models, and HTML rendering helpers.
- `opi-agent`: session branching with tree reconstruction, branch picker, and branch-aware session writer.
- `opi-tui`: branch picker widget with snapshot-tested rendering.
- `opi-coding-agent`: extension examples for MCP adapter, todo, plan mode, sub-agent, protected paths, and permission gate patterns.
- `opi-coding-agent`: progressive discovery for themes, prompt fragments, skills, and package resources.

### Changed

- `opi-coding-agent`: `--list-models`, interactive model picking, and runtime model validation now use provider registry metadata.
- `opi-coding-agent`: example package manifests use the supported flat `package.toml` schema.
- `opi-agent`: `StreamingProxy::run` is synchronous transport-agnostic I/O instead of an async wrapper around blocking reads.

### Fixed

- `opi-coding-agent`: Windows subprocess tests resolve `opi.exe` correctly.
- `opi-web-ui`: RPC response `data` is preserved and updates session/model state.
- `opi-coding-agent`: same-layer duplicate resource/package names now produce explicit errors.
- `opi-coding-agent`: package resource containment checks no longer fall back to unresolved paths when canonicalization fails.
- `opi-agent`: default secret redaction no longer redacts short benign `sk-` or `eyJ`-like strings.
- `opi-agent`: `SdkResponse` now round-trips through JSON and serialization fallback events use `SdkSerializationError`.
- `opi-web-ui`: `ThinkingBlock` is re-exported from the crate root with the other component models.
- `opi-coding-agent`: phase 4 ledger hash check normalized for cross-platform consistency.

### Removed

- `opi-agent`: stale public `Transport` stub.

## [0.4.0] - 2026-06-02

Phase 3: cloud provider expansion (Vertex AI, Azure OpenAI, Bedrock), image
support across the stack, new built-in tools (find, ls), fuzzy picker, terminal
image rendering, shell completions, and proxy support.

### Added

- `opi-ai`: AWS Bedrock provider with SigV4 signing and credential resolution
- `opi-ai`: Azure OpenAI provider with deployment URL and api-key auth
- `opi-ai`: Google Vertex AI provider with OAuth Bearer auth
- `opi-ai`: HTTP/HTTPS proxy support with env-var and per-provider config
- `opi-ai`: image input support for multimodal prompts
- `opi-ai`: shared HttpClient with connection pooling
- `opi-agent`: image tool result support for visual tool output
- `opi-agent`: `prompt_with_content` method for arbitrary content (text + images)
- `opi-coding-agent`: `--list-models` flag to list available models (table or NDJSON)
- `opi-coding-agent`: `--image` flag for non-interactive image attachment
- `opi-coding-agent`: `/image` slash command for TUI image attachment
- `opi-coding-agent`: `find` built-in tool for file search
- `opi-coding-agent`: `ls` built-in tool for directory listing with metadata
- `opi-coding-agent`: shell completion generation for bash, zsh, fish, powershell, elvish
- `opi-coding-agent`: pi-style tool selection and safety hooks
- `opi-coding-agent`: AGENTS.md / CLAUDE.md context file loading
- `opi-coding-agent`: global context file discovery from user config directory
- `opi-coding-agent`: proxy wiring to all provider factory paths
- `opi-coding-agent`: enhanced tool management and path resolution
- `opi-tui`: fuzzy model/session picker with SelectList widget
- `opi-tui`: terminal image rendering with protocol detection (kitty, sixel, iTerm2)

### Performance

- `opi-ai`: shared HttpClient with connection pooling reduces TLS handshake overhead

### Fixed

- `opi-ai`: Bedrock error mapping now parses Retry-After header for 429 responses
- `opi-ai`: Azure OpenAI endpoint validation -- missing endpoint returns config error
- `opi-ai`: Bedrock URL-sourced images rejected with clear unsupported-error message
- `opi-agent`: compaction summary includes image content placeholders
- `opi-coding-agent`: ls tool truncation count now correctly reports omitted entries
- `opi-coding-agent`: session picker sorted newest-first to avoid filesystem ordering flakes
- `opi-coding-agent`: char-aware truncation in session picker to avoid non-ASCII panic
- `opi-coding-agent`: `--list-models --json` uses serde_json for properly escaped output
- `opi-coding-agent`: `--image` files passed through to interactive mode first prompt
- `opi-coding-agent`: session files excluded from crate package
- `opi-tui`: terminal image protocol hardening

## [0.3.0] - 2026-05-25

Phase 2 hardening: multi-provider support (6 LLM providers), session
persistence, context compaction, configurable TUI, and cost tracking.

### Added

- `opi-ai`: OpenAI-compatible chat provider with SSE streaming
- `opi-ai`: OpenAI Responses API provider with streaming
- `opi-ai`: Google Gemini provider with HTTP streaming
- `opi-ai`: Mistral provider profile
- `opi-ai`: OpenRouter provider profile
- `opi-ai`: retry/backoff/rate-limit support with configurable strategies
- `opi-ai`: usage accumulation and cost tracking across turns
- `opi-agent`: session v1 JSONL storage for conversation persistence
- `opi-agent`: compaction engine with trigger and hook support
- `opi-agent`: thinking config passed through to provider requests
- `opi-agent`: enhanced event handling and message management
- `opi-coding-agent`: session list/resume/delete CLI flags
- `opi-coding-agent`: session persistence wired into harness runtime
- `opi-coding-agent`: compaction wired into session coordinator
- `opi-coding-agent`: `--json` NDJSON output mode for non-interactive use
- `opi-coding-agent`: provider factory extended for all 6 providers
- `opi-coding-agent`: usage accumulation wired to TUI status bar
- `opi-coding-agent`: edit tool captures before/after content
- `opi-coding-agent`: workspace path validation for all tools
- `opi-tui`: configurable keybindings with TOML parsing
- `opi-tui`: Theme struct with default and monokai palettes
- `opi-tui`: DiffView widget for edit/patch visualization

### Fixed

- Session runtime tests serialized to avoid env var races

## [0.2.0] - 2026-05-22

Phase 1 MVP: functional Anthropic-based coding assistant with six tools,
basic TUI, TOML config, and mock-provider integration tests.

### Added

- `opi-ai`: message and stream types with 12 `AssistantStreamEvent` variants
- `opi-ai`: `Provider` trait with `stream(Request) -> EventStream`, `Request`,
  `ThinkingConfig`, `ModelInfo`, `ProviderError`
- `opi-ai`: Anthropic SSE provider with hand-written SSE parser and
  `AnthropicMapper` for event translation
- `opi-ai`: provider registry resolving `anthropic:model` specs with capability
  queries
- `opi-ai`: shared `MockProvider` test harness with builder helpers
- `opi-agent`: `Tool` trait with JSON Schema validation via `jsonschema`
- `opi-agent`: `agent_loop` with turn lifecycle, tool batching (parallel/sequential),
  cancellation via `CancellationToken`, and queue polling
- `opi-agent`: `Agent` wrapper with `prompt`, `continue_`, `abort`, `subscribe`
- `opi-agent`: hooks (`AgentHooks`) with `after_tool_call`, `should_stop_after_turn`,
  `prepare_next_turn`, steering and follow-up queues
- `opi-coding-agent`: `ReadTool`, `WriteTool`, `EditTool`, `BashTool` with workspace
  safety boundaries and confirmation policy
- `opi-coding-agent`: `GlobTool`, `GrepTool` with gitignore-aware file search
- `opi-coding-agent`: `SystemPromptBuilder` with layered prompt construction
- `opi-coding-agent`: TOML config loading with CLI > env > project > user > defaults
  precedence
- `opi-coding-agent`: non-interactive mode with exit codes and high-risk tool safety
  policy
- `opi-coding-agent`: interactive TUI mode using ratatui/crossterm
- `opi-tui`: TUI shell with `MessageList`, `InputEditor`, `StatusBar`, `ToolCallView`
- `opi-tui`: `MarkdownView` and `CodeBlock` rendering widgets
- 213 integration and unit tests across all crates

### Fixed

- SSE parser surfaces malformed events instead of silently dropping them
- SSE parser handles CRLF line endings for cross-platform robustness
- `BashTool` uses `cmd.exe` on Windows, `sh` on Unix
- Agent loop emits `ToolExecutionStart` before parallel tool spawning
- `AuthFailed` error variant maps to exit code 3
- Config: explicit `--config` with non-existent file returns error
- Config: `--config` model not overridden by `OPI_MODEL` env var
- Agent loop uses `tokio::select!` for responsive stream cancellation
- Tool call `input` serialized as JSON object, not string

## [0.1.1] - 2026-05-20

### Added

- `opi-implement` skill for structured implementation workflows with
  phased gates, verification tiers, and JSON ledger tracking.
- CI workflows: `ci.yml` (fmt, clippy, test, doc) and `release.yml`
  (cross-platform binary builds on tag push).
- Opi technical specification document (`docs/opi-spec.md`).

### Fixed

- Release skill: keep SHA256SUMS local-only, use version-based artifact
  directory.

### Changed

- `opi-web-ui` marked as `publish = false` (not ready for crates.io).

## [0.1.0] - 2026-05-20

Initial scaffolding release. Establishes the workspace layout and crate
boundaries; functional implementations land in subsequent releases.

### Added

- Cargo workspace with five crates under lockstep versioning:
  - `opi-ai` — unified multi-provider LLM API (module scaffolding for
    `provider`, `stream`, `model`, `config`).
  - `opi-tui` — terminal UI library (module scaffolding for `render`,
    `editor`, `markdown`).
  - `opi-agent` — agent runtime with tool calling and transport
    abstraction (module scaffolding for `tool`, `transport`, `state`).
  - `opi-web-ui` — reusable web chat components (module scaffolding for
    `components`).
  - `opi-coding-agent` — produces the `opi` binary; supports `--version`
    and `--help`.
- `opi-release` skill (`.claude/skills/opi-release/skill.md`) implementing
  a seven-phase release workflow with explicit irreversibility gates.

### Notes

- All crate APIs are placeholders. Calling them will not do anything
  useful yet.
- This release is published as a GitHub Release only; crates.io publish
  is deferred until the crates have real implementations.

[0.7.0]: https://github.com/OdradekAI/opi/releases/tag/v0.7.0
[0.6.5]: https://github.com/OdradekAI/opi/releases/tag/v0.6.5
[0.6.1]: https://github.com/OdradekAI/opi/releases/tag/v0.6.1
[0.6.0]: https://github.com/OdradekAI/opi/releases/tag/v0.6.0
[0.5.4]: https://github.com/OdradekAI/opi/releases/tag/v0.5.4
[0.5.3]: https://github.com/OdradekAI/opi/releases/tag/v0.5.3
[0.5.2]: https://github.com/OdradekAI/opi/releases/tag/v0.5.2
[0.5.1]: https://github.com/OdradekAI/opi/releases/tag/v0.5.1
[0.5.0]: https://github.com/OdradekAI/opi/releases/tag/v0.5.0
[0.4.0]: https://github.com/OdradekAI/opi/releases/tag/v0.4.0
[0.3.0]: https://github.com/OdradekAI/opi/releases/tag/v0.3.0
[0.2.0]: https://github.com/OdradekAI/opi/releases/tag/v0.2.0
[0.1.1]: https://github.com/OdradekAI/opi/releases/tag/v0.1.1
[0.1.0]: https://github.com/OdradekAI/opi/releases/tag/v0.1.0
