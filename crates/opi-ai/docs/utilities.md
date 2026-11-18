# Public Utilities and Data Contracts

These utilities are public because they define values that cross a provider or
consumer boundary. Their exact signatures and trait bounds are in rustdoc.

## Tool-Argument Normalization and Validation

On Chat Completions routes, a partial `arguments` holds the raw text seen so far.
For incremental display, keep one `streaming_json::ToolCallArgumentsStream` per
tool within the call. Feed `ToolCallDelta` text to `append`; call `snapshot` at
your rendering cadence. Each returned `serde_json::Value` is an independent
object, safe to retain or edit. Remove the owner when the call ends or is
cancelled. Execute only the final tool call after validation.

`text` exposes the unchanged accumulated input. `replace` accepts a complete
prefix: extensions return the appended suffix, identical input returns `Some("")`,
and a non-prefix replacement resets state and returns `None`. `finish` delegates
to the strict final-argument contract and returns `(text, error)` without
consuming the owner; a successful partial preview cannot make an invalid final
executable. A provider's authoritative final replacement must still be used.

The SDK enables `serde_json/float_roundtrip` for correctly rounded final JSON
decoding, including the integer spelling of `1e64`. Cargo unifies this feature
with callers using the same dependency version, so Agent execution and session
save/resume use the same decoding. Raw final text remains unchanged; session
storage still applies its existing redaction and serialization policy. Native
`i64`/`u64` JSON integers stay exact, and overflow remains a final error. This
does not convert every Rust integer to JavaScript's floating-point representation.

Missing values are omitted, duplicate keys retain their earlier value until a
replacement is available, and incomplete escapes wait for more input. Object
keys retain their first position, with canonical JavaScript array-index keys
first in numeric order. Non-object roots display `{}`. An unpaired UTF-16
surrogate in a key or value holds the previous representable snapshot until a
pair arrives; raw text is preserved and strict final validation still rejects
lone surrogates. Snapshots deeper than 128 containers also retain the previous
value, including after closure, to bound recursive JSON clone/drop. A reachable
non-finite numeric value retains the entire previous snapshot instead of
omitting members or shifting array positions. Overwriting the offending subtree
with a representable value resumes publication; a non-prefix `replace` starts
with an empty snapshot. Strict final parsing still rejects overflow, even in
an earlier duplicate member that the preview has already replaced.
These Unicode, depth and numeric boundaries differ from Pi's JavaScript values.

Parsing scans new input only. Snapshot publication follows Pi 1.5.0's open-path
copy credit: each append earns `256 + 8 * UTF-16 length`; refresh spends the open
containers' member/item counts plus one per container and one pending scalar.
Empty appends earn credit; repeated reads and identical replacements do not.
Large open containers can temporarily return an older snapshot. Materializing
owned JSON still copies the visible tree: requesting every tiny chunk of a
long value can incur quadratic total copying. Choose a display cadence suited
to the consumer; this is not a total linear-time guarantee.

The separate whole-prefix `message::parse_streaming_json` helper remains
available. It may display missing values as `null`; complete `true`, `false`
and `null` literals remain visible before container closure. It retains its
existing behavior and is not the incremental owner above.

`schema::validate_tool_arguments` and `schema::validate_tool_call` validate a
caller-owned tool definition before request construction. An existing
`arguments_error` is rejected before coercion; raw caller text is preserved.
Empty textual arguments normalize to `{}`. Incomplete or non-object finals keep
the owned raw text and a typed error instead of becoming a partial-prefix call.

The normalizer applies the supported JSON-schema coercions and rejects invalid
root/schema shapes before authorization or network IO. Provider adapters may
project the validated value into their wire format, but they do not repair an
invalid tool call by guessing.

## Canonical Replay Projection

`project_replay` converts canonical `Message` chronology into a route-local wire
view. It validates tool association by batch position and source identity,
rejecting duplicate calls/results, orphan results and name mismatches before
request construction. Canonical history remains unchanged.

Wire IDs are legalized locally and remain synchronized across call/result
pairs. Images preserve association, order and failure state on routes that
support them. `ReplayProjection::Strict` rejects lossy transforms instead of
silently recording a different history. Provider adapters consume the
projection at the head of request construction.

## Context and Transcript Chronology

`transcript::normalize_context` folds the shorthand `Context` into one
canonical chronology. System content, named sections, tool additions and
removals are applied in order; redundant changes are omitted and a rejected
append leaves the previous transcript unchanged.

Adapters resolve the transcript when building a request. Native routes may keep
mid-conversation system messages; collapse routes project one replay header.
The canonical transcript is caller-owned and is not rewritten by projection.

## Headers and Scoped Environment

`api::shared::headers::merge_provider_header_sources` performs case-insensitive,
ordered set/remove merging. Later sources win and move the winning key to the
end; `None` removes a key. Provider-managed authentication headers cannot be
silently replaced by arbitrary request headers.

Provider environment is read through the injected `AuthContext`. The SDK does
not require ambient process reads during model construction, and a scoped
source cannot mutate another provider's environment policy.

## Identity, Overflow and Typed Diagnostics

The public identity helpers provide stable short hashes and UUIDv7 values with
validated timestamp/sequence boundaries.

`message::is_context_overflow` recognizes token-window and request-size
failures. Error diagnostics include LiteLLM's `Context Window exceeded` and
HTTP 413 at the start of the diagnostic, either as a raw status or in a
`<provider> API error (413):` prefix. Throttling and rate-limit diagnostics
take precedence. Bodyless HTTP 400 indicates overflow for Cerebras only.
The overflow and length predicates return booleans. Callers own compaction
and retry decisions.

Diagnostic summaries are redacted and bounded. Provider status, retry hints and
safe excerpts retain their typed category without echoing secrets or an
unbounded response body.

## JSON Repair and Portable Schema Utilities

The public JSON repair helpers preserve the caller's raw text while repairing
only the admitted escape and delimiter cases. A repair result exposes whether
normalization changed the text; it never fabricates a tool call from an
incomplete value.

The schema utilities support the documented JSON dialect and source-free
validation path. Unsupported dialect markers, invalid references, no-progress
recursion and resource limits produce typed failures. Data-version differences
from a supported dependency profile do not change schema admission rules.

## Text/System Rendering

Text, thinking, redacted-thinking, tool-call and tool-result content retain
order and failure metadata through rendering and replay. Renderers do not
convert private thinking into visible text when a route cannot preserve it.

The system-message helpers maintain named sections and tool declarations with
stable ordering. A tool redefinition is an explicit add/change operation; a
removed name is represented as a removal rather than an implicit mutation.

## Whole-Call Retry and Request Callbacks

`opi_ai::retry_assistant_call` applies a caller-owned, bounded whole-call retry
policy. It classifies only the documented transient failures, preserves the
last error after exhaustion, observes cancellation and never retries an
explicit abort or callback failure. Recognized context overflow takes
precedence over transient diagnostics.

For a call failure, `retry::classify_whole_call_retry(failure.message())`
classifies the canonical diagnostic. It recognizes the ChatGPT Business
workspace usage cap and Z.ai code `1308` as quota limits. Retryable HTTP
statuses match at ASCII alphanumeric boundaries. An OpenAI-compatible 429
with a recognized limit carries a redacted quota diagnostic and its typed
rate-limit cause with the retry hint.

For `response.failed`, OpenAI Responses and Azure use the nested error code
and message. Codex uses the message on SSE and WebSocket. A recognized
transient failure, quota limit or context overflow carries a fixed diagnostic
in the stream cause, result message and telemetry. Successful response
handoff seals prepared acquisition. The explicit whole-call policy can start
a fresh call for a transient failure.

For a top-level Responses `error`, the OpenAI and Azure SDK envelope uses
the selected message, with scalar code and type as the fallback. A flat
data-only `type: "error"` uses both its code and message. Codex selects
each field from a top-level string or its nested counterpart, then uses the
nonempty message or code. The fixed diagnostic carries the recovery decision
through the result and telemetry. Classification is limited to those fields.

`RequestCallbacks` observes the payload and response boundaries. Payload
callbacks run after projection and validation; response callbacks run at the
source-defined response boundary. A callback failure is a typed functional
failure, is redacted, and is not retried. Callback order is deterministic and
callbacks never install a hidden global transport policy.
