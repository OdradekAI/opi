# Public Utilities and Data Contracts

These utilities are public because they define values that cross a provider or
consumer boundary. Their exact signatures and trait bounds are in rustdoc.

## Tool-Argument Normalization and Validation

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

`provider_headers::merge_provider_header_sources` performs case-insensitive,
ordered set/remove merging. Later sources win and move the winning key to the
end; `None` removes a key. Provider-managed authentication headers cannot be
silently replaced by arbitrary request headers.

Provider environment is read through the injected `AuthContext`. The SDK does
not require ambient process reads during model construction, and a scoped
source cannot mutate another provider's environment policy.

## Identity, Overflow and Typed Diagnostics

The public identity helpers provide stable short hashes and UUIDv7 values with
validated timestamp/sequence boundaries. Overflow and length predicates are
pure and return typed diagnostics; they never invoke compaction, retry or
network work.

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
explicit abort or callback failure.

`RequestCallbacks` observes the payload and response boundaries. Payload
callbacks run after projection and validation; response callbacks run at the
source-defined response boundary. A callback failure is a typed functional
failure, is redacted, and is not retried. Callback order is deterministic and
callbacks never install a hidden global transport policy.
