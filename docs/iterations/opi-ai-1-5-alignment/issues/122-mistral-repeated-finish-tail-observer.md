# #122 Mistral repeated finish, tail usage and observer release

Status: verified

## Finding and priority

The native `mistral-conversations` adapter already consumed the response through
`[DONE]`, but its mapper retained an earlier `stop_error` after a later valid
finish. A provider sequence such as `error → usage-only → stop` therefore
failed in Rust even though frozen Pi kept the last `stop` terminal and emitted
`done`. The route also lacked one focused contract covering repeated terminal
usage, a tail raw-observer failure, malformed data after a terminal and the
ability to start a later request after the failed stream.

## Contract

- Every complete Mistral chunk reaches the raw observer before normalization,
  including repeated finish frames and choices-empty usage-only frames.
- The mapper keeps the latest finish reason and latest valid usage snapshot.
  A later valid `stop`, `length` or `tool_calls` terminal clears an earlier
  provider stop error; a later error/unknown reason remains terminal failure.
- `[DONE]` is the publication boundary. A successful source publishes exactly
  one `Done`; a missing finish reason, malformed tail, transport/parse fault,
  cancellation or raw observer failure publishes no stale `Done` and retains
  the latest partial snapshot.
- After a typed tail observer failure the producer and response body are
  released sufficiently for the same provider to accept a subsequent request;
  this is verified through two sequential HTTP calls. The test does not claim
  a particular connection-pool socket reuse policy.

## Implementation and caller map

`MistralSseMapper::consume` now treats a later valid finish as authoritative
and clears only the stale provider error. `MistralConversationsProvider` owns
the HTTP producer, raw callback ordering, cancellation and response-body drop
through the existing producer select. The public callers are the native
`stream_simple` and `stream_prepared` routes, `MistralConversationsChatApi`,
`Models::{stream_simple,complete_simple,stream_direct,complete_direct}` and
the existing Agent result/cost/transcript consumers.

The direct regression helper reports normalized Rust block/terminal events;
the Pi probe additionally reports its synthetic `start` event and preserves
Pi's stale `errorMessage` field after a final successful `stop`. Those are
reporting/state-shape differences, not terminal or usage differences.

## Verification

- `mistral_repeated_finish` covers repeated error/stop finishes, usage-only
  tails, raw observer failure, malformed tails, missing finish and the
  subsequent-call release path: 4/4 passed.
- Existing native/profile Mistral fixtures pass 55/55; the focused test's
  clippy run passes with `-D warnings`.
- Frozen Pi revision `f9a26365568127461c47a07a37dcb42c140a3ac5` and Rust traces
  agree for three stable runs on simple and direct routes: final stop `stop`,
  usage `10/9/19`, one Done, five raw provider events and one request.
- The actual `opi-ai` archive consumer passes the builtins/no-tracing and
  defaults profiles, executing all four regressions. The first default-profile
  run failed only because the shared target exhausted disk; the reviewed
  incremental eviction plan removed 6,144 payload files (14,575,663,936
  allocated bytes), and the refreshed v2 receipt passes.

## Historical evidence and remaining scope

The failed package receipt is retained as `phase7-mistral-repeated-finish-
package-v1.json`; the passing v2 receipt is canonical. Other adapter/observer
fault stages and repeated/pending paths, configuration set V, MSRV/native
platform evidence, quantitative budgets and real Deferred integration (#94)
remain open. This slice does not claim full Pi strict equivalence or six-gate
acceptance.

Test impact: `add` Mistral repeated terminal/tail/release regression, frozen Pi
probe/comparator and archive consumer; `update` Mistral terminal error
precedence and Cargo test target; `delete` none; `retain` all prior adapter,
observer, Chat, preview, usage, numeric, cancellation, deferred and package
evidence. No commit or stage was performed.
