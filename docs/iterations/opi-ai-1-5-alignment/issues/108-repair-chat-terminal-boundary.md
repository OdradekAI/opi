# #108 Repair the shared Chat terminal boundary

Status: scoped repair and paired/consumer verification complete. The workspace failures below are historical; [#109](109-repair-baseline-checks-and-refresh-packages.md) resolves them and refreshes package consumption.

The user authorized the coherent repair recommended by [#107](107-observe-chat-finish-terminal-boundary.md).
Finish reasons and usage now accumulate metadata. `[DONE]` or valid natural
HTTP EOF closes blocks in content order, finalizes tool arguments and emits
one `Done`. Tail failures remain effective until then. The HTTP adapter stops
reading and releases the response at its terminal, even without usage.

## Implementation and compatibility

- The parser exposes `ParsedEvent::Done`. Custom mapper consumers replace
  `flush_pending_done()` with `finish(supports_finish_reason)`, returning the
  full block-end/terminal sequence. The runtime frame mapping is shared by
  ordinary Chat and Azure Chat, including malformed frames and usage failures.
- OpenAI Chat, OpenRouter, legacy Mistral Chat and Azure Chat share the repair.
  Azure also reuses the Chat SSE chunk decoder, including split CRLF frames.
  Mistral Conversations and Azure Responses are separate implementations.
- Blocks close in content order, including tool-first/text-late and late
  thinking. Grammar completion, textual argument validation and strict null
  normalization occur only on a successful end. Failed partials retain the
  received arguments, usage and raw finish reason.
- Missing finish reasons fail by default. Explicit `supports_finish_reason=false`
  permits inferred stop/tool use without inventing a raw reason. Azure simple
  dispatch now preserves the profile's `with_compat` configuration while
  replacing the selected model. Azure Chat records have no model-level finish
  compatibility fields. A body flag control also checks this configuration path.
- HTTP requires blank-line SSE dispatch; unterminated fragments at natural
  EOF are discarded. The standalone full-body parser retains its existing
  acceptance of a final unterminated frame. This repair does not change that
  parser input convention.

## Explicit cancellation difference

Opi retains cancellation priority over queued provider events and does not
synthesize block ends. Pi 1.5.0's OpenAI SDK can unwind its iterator normally
on transport abort, allowing Pi to close blocks before returning `aborted`.
This observed block-end difference is intentional for Opi's existing public
cancellation contract. Both results must be aborted, release the response and
execute no tools. Agent ignores ToolCallEnd and authorizes tools only after
Done; ThinkingEnd is progress. Telemetry counts only delivered block ends and
settles the request once. No shared cancellation policy was weakened to copy
an upstream SDK unwind artifact.

The paired comparator explicitly reports `strict_alignment=false` and checks
this difference. Its scope is checkpoint timing, event kinds/order, final
message/token usage and peer release. Event-time partial aliasing/stop snapshots
remain outside this comparison, as in #107; the original full traces remain
available for inspection.

## Verification and evidence

The final source-bound candidate passes five SDK tests (75 scenarios), the
real Agent test (three scenarios), workspace all-target Clippy with `-D warnings`,
and all sixteen comparator tests. The SDK suite includes eight original pairs,
forty shared-route tail/order cases, nine strict/grammar/incomplete argument
cases, eight finishless compatibility cases and ten standalone SSE cases.
Every HTTP case checks a finished request span, matching stop attribute,
exactly one request-duration measurement and the expected delivered blocks.

[Final paired evidence](../evidence/phase5-chat-terminal-repaired-verified.json)
records eight matched requests against frozen original-lock Pi 1.5.0
`f9a26365568127461c47a07a37dcb42c140a3ac5`, repeated three times per side.
Terminal results, usage, checkpoint timing and response release agree. Both
held and final event snapshots are checked. Cancellation's block ends remain
the explicit difference, not a hidden normalization.

[Final caller evidence](../evidence/phase5-chat-terminal-consumers-repaired.json)
repeats the three real Agent cases and three Durable projection pairs three
times. Before the tail there are no tools or successful completion/storage.
Normal completion invokes the tool; errors/cancellation do not. Durable's
paired final states, including stored assistants, agree.
[Final frozen materials](../evidence/phase5-chat-terminal-repair-final-materials.json)
bind the source/dependency/runtime state. Intermediate repair evidence remains
retained and is not substituted for this final candidate.

The workspace run completed 272 targets: 6,464 tests passed, 26 failed and five
were ignored. Three failures were the HTTP trailer fixture's old assumption
that `[DONE]` still waits for HTTP EOF. That fixture now omits the sentinel,
retaining finish/usage and the original trailer validation controls. All 21
HTTP acceptance tests and affected-target Clippy then passed. The other failed
targets are independent of this repair:

- `platform_delivery::unsafe_inventory_is_frozen` counts `unsafe` in a Codex
  WebSocket comment as code. That source is byte-identical to the task-start
  copy; the inventory test is unchanged from HEAD.
- `json_mode::json_mode_compact_text_deltas_are_constant_size` uses MockProvider,
  bypassing Chat. The existing serialized `usage.cost` adds 129 fixed bytes,
  taking a compact update from 969 to 1,098 bytes and exceeding the old 1,000-byte
  threshold. The compacting path and cost contract are unchanged by this task.
  This panic poisons the shared environment lock, causing 20 further failures
  in the workspace run. A single-thread rerun reproduces the root failure;
  a diagnostic run excluding that one test passes the other 35 tests. This
  exclusion is a causal control, not a passing full-suite claim.
- `spec_ledger::live_ledger_spec_hash_matches_current_spec` reads the absent
  legacy `docs/opi-spec.md`. The guard and root ledger are unchanged from HEAD,
  where that path is already absent; this iteration's spec is a different file.

At this checkpoint these unrelated failures were retained for a follow-up;
the full workspace gate was not green. [#109](109-repair-baseline-checks-and-refresh-packages.md)
records the subsequent repairs and passing workspace/package checks. All 11 doctests and rustdoc with `-D warnings`
passed. [Check receipt](../evidence/phase5-chat-terminal-repair-checks.json)
records commands, raw logs, applicability, outcomes and failure attribution.

The first Clippy run caught an unused Result in the Azure compatibility repair,
which is now handled. Later compilation attempts exhausted E: and failed;
those raw logs remain preserved. After the
user explicitly authorized the reviewed incremental-only maintenance plan,
177,348 rebuildable payload files were removed, recovering approximately
112.7 GiB of observed net free space (69,632 to 121,020,440,576 bytes immediately
after maintenance). Verified planning took 210.298 seconds and application,
including preflight, took 248.027 seconds by log wall-clock intervals; these
intervals may include host suspension. The first application rejected changed ownership metadata before
any deletion; a new plan selected exactly the same reviewed payloads and
passed preflight. Source, executable and historical evidence bytes were kept.
Maintenance plans/receipts live outside iteration evidence under the external
task directory and do not substitute for build or behavior checks.

The original #107 JSON receipts, source snapshots, logs and sealed executables
are retained. Pre-edit workspace files are copied under
`C:/Temp/opi-chat-terminal-repair-20261114/before/`; historical evidence remains
an observation of its original input, not proof of the repaired candidate.
[Archive mapping](../evidence/phase5-chat-terminal-history-preserved.json) verifies all five original receipts unchanged and preserves six changed input versions.
New receipts use distinct paths and refuse overwrite. Intermediate repair
receipts remain retained when a subsequent source change invalidates them.

The real Opi Agent loop is exercised against a gated HTTP Chat peer. Frozen
Pi Durable uses recorded SDK event projections into its actual Harness and
MemoryStorage; that is not a live Rust bridge or an on-disk Opi JSONL session
test. Session persistence effects beyond that controlled consumer remain a
source-chain inference.

Test impact: `update` for the two existing Rust timing probes, mapper fixture
helpers, HTTP trailer fixtures and two comparator scripts plus their unit tests; `add` for shared
route, thinking/order, grammar/strict/incomplete arguments, finishless, malformed,
transport, observer, raw callback and telemetry controls inside the SDK probe;
`retain` for #107 historical fixtures/evidence and existing affected suites;
`delete=none`.

Delivery is worktree-only at unchanged HEAD
`ee18dd1323a089d536bdc7e293d0b4869777c344`: no commits or staged changes.
Fifteen carried-in paths were deliberately updated; the previously clean
`http_acceptance.rs` is also task-owned. Other carried-in files are unchanged.
The new report and receipts are untracked; no historical receipt was overwritten.

This scoped repair does not close direct SDK argument preview, real Deferred
integration (#94), minimum features, other-platform/MSRV evidence, performance
budgets or the iteration's six gates. The earlier #106 package-consumption
receipts still describe that historical candidate; this task changes runtime
sources and does not renew package-consumption acceptance.
