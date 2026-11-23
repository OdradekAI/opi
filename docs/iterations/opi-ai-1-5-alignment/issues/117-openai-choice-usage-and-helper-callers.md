# #117 Preserve OpenAI-compatible choice-level usage and map public callers

Status: verified

## Finding and priority

The frozen Pi 1.5.0 OpenAI Completions adapter accepts usage from
`choices[0].usage` when a compatible endpoint omits the standard chunk-level
`usage` member. The current Rust adapter only inspected `chunk.usage`, so a
Moonshot-style response reached the public Chat result with unreported zero
usage. This was the highest-priority remaining adapter gap found while
cross-checking the #116 remainder: it changes accounting visible to Agent and
session consumers, while the image/classifier observer boundary is already an
approved #19 contract.

## Contract

- `chunk.usage` remains authoritative when both locations are present.
- When it is absent, `choices[0].usage` is parsed with the same cache,
  reasoning, total-token and OpenRouter-cost mapping.
- Reasoning subset validation applies to the fallback object as well; an
  invalid subset emits the same typed usage error before a successful Done.
- No new AI observer is installed for image, classifier or deferred-cancel
  operations. Those public helpers continue to forward the caller's
  `TelemetryContext` only, matching #19.

## Implementation

`RawChoice` now carries an optional `usage` object. The parser selects the
top-level object first, otherwise the first choice's object, and the mapper
keeps the existing terminal and cost behavior. The public caller chain is:

| Public seam | Owner/caller | Contract evidence |
| --- | --- | --- |
| `parse_sse_events` / `OpenAiChatMapper` | `providers::openai::OpenAiChatProvider::stream_from_sse` and the OpenAI-compatible provider adapter | `crates/opi-ai/src/api/openai.rs` (wire) + `crates/opi-ai/src/providers/openai.rs` (profile) |
| `Models::stream_simple` / `complete_simple` | ordinary Models callers and the `opi-agent` prepared/ordinary Chat consumers | `crates/opi-ai/src/models.rs`, `crates/opi-agent/src/agent_loop.rs` |
| `Models::stream_direct` / `complete_direct` | explicit-auth SDK callers | `crates/opi-ai/src/models.rs` |
| `parse_streaming_json` / `finalize_tool_arguments` | direct SDK preview/finalization callers; unchanged legacy whole-prefix owner | `crates/opi-ai/tests/public_utilities/tool_arguments.rs`, #110–#113 |
| `Models::generate_images`, `Models::classify`, `Models::cancel_deferred` | operation-specific adapters | #19 carrier-only observer contract; no automatic `pi.ai.*` span |

The last two rows are mapping evidence, not a request to expand the approved
observer schema.

## Verification

- The new `openai_choice_usage` integration target covers fallback use,
  top-level precedence and fallback subset rejection.
- Frozen Pi 1.5.0 revision `f9a26365568127461c47a07a37dcb42c140a3ac5` and the
  current Rust probe run three stable repetitions with
  `input=25/cacheRead=75/output=20/total=120`, `stop=stop`, one request and
  zero fixture cost.
- The actual `opi-ai` and `opi-telemetry` archives pass the same test from an
  external consumer under `builtins` without tracing and default features.
- Existing classifier, image-provider, deferred, telemetry-carrier and public
  utility regression targets pass while the caller/observer mapping remains
  carrier-only for those non-Chat helpers.
- Prior receipts and sealed artifacts remain untouched; the new materials,
  differential and package receipts are separate files.

## Remaining scope

This closes only the choice-level usage adapter gap. Other adapter/observer
fault stages, complete feature set V, MSRV/native platforms, quantitative
budgets and real Deferred integration (#94) remain open. No strict full-Pi or
six-gate acceptance claim is made.
