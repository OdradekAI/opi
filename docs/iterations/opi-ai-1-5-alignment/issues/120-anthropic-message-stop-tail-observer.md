# #120 Hold Anthropic `message_stop` through repeated deltas and observer tails

Status: verified

## Finding and priority

The Anthropic HTTP adapter published Done as soon as it mapped `message_stop`
and stopped reading the source after a terminal mapper state. Frozen Pi 1.5.0
keeps invoking `onProviderStreamEvent` for every item yielded by its source and
publishes `done` only after the async iterable reaches EOF. A repeated
`message_delta` after `message_stop` could therefore update usage in Pi while
the Rust adapter had already settled the message; a tail observer failure could
also be hidden behind Done.

## Contract

- HTTP and fixture adapter routes record `message_stop` as a pending terminal;
  the source remains open through repeated `message_delta` and `message_stop`
  frames.
- The last valid usage and stop facts observed before EOF are authoritative;
  exactly one Done is published after a successful source EOF.
- Raw observers see every wire frame in order. A tail callback failure is a
  typed `CallbackPhase` stream failure before Done, and the successful terminal
  is not published afterward.
- Typed transport failures such as a mid-output fallback still terminate the
  source immediately; this preserves the existing failure partial and avoids
  treating an error tail as a successful terminal.
- A parse or mapper usage error after a pending `message_stop` terminates
  without flushing a stale Done; the error path owns the terminal outcome.
- Direct `AnthropicMapper::new().process(...)` retains its immediate terminal
  behavior for low-level callers; deferred publication is enabled by the
  adapter-owned streaming entry points.

## Implementation and caller map

`AnthropicMapper` now has an opt-in deferred-terminal mode with an EOF flush.
`AnthropicProvider::stream_from_sse` and the shared Anthropic HTTP route enable
it. Models simple/direct, prepared calls, Agent Chat, cost settlement and
transcript/session consumers receive the final EOF-settled message through the
existing shared stream owner.

| Seam | Owner/caller | Evidence |
| --- | --- | --- |
| Anthropic mapper | `AnthropicMapper::process`, `flush_pending_terminal` | `crates/opi-ai/src/anthropic.rs`, `tests/anthropic_repeated_message_stop.rs` |
| Shared HTTP route | `AnthropicProvider::stream_prepared` | `crates/opi-ai/src/anthropic.rs`, `tests/anthropic_fixtures.rs` |
| Public Chat | `Models::stream_simple`, `stream_direct`, `complete_simple`, `complete_direct` | existing Chat result/cost/transcript consumers receive the settled final snapshot |
| Agent/session | existing Agent Chat and persistence/recovery callers | no new ownership seam; terminal settlement remains in the shared Chat stream |

## Verification

- Repeated `message_delta` usage 5 then 7 after `message_stop` emits one Done
  with output usage 7 and the original `end_turn` stop fact.
- A post-stop usage error produces a typed failure and no Done, so a pending
  terminal cannot mask a later adapter failure.
- A raw observer sees `message_start`, both deltas and `message_stop` before a
  tail callback failure; no Done follows the typed failure.
- Anthropic adapter fixtures pass, including existing fallback and callback
  failure controls.
- Frozen Pi revision `f9a26365568127461c47a07a37dcb42c140a3ac5` agrees with the
  normalized Rust trace for three runs on simple and direct caller routes.
  Pi's provider event list is retained as an explicit observation because the
  Rust low-level fixture helper exposes normalized events rather than callback
  delivery.
- The actual packaged `opi-ai` archive passes the external consumer under
  builtins without tracing and default features.
- The initial parallel full-suite no-space failure and the reviewed
  incremental-only cache eviction are retained in the phase 7 checks and
  cache receipts; the follow-up single-job full suite exits successfully.

## Remaining scope

Other adapter/observer fault stages and repeated/pending paths, complete
configuration set V, MSRV/native platforms, quantitative budgets and real
Deferred integration (#94) remain open. This slice does not claim full Pi
strict equivalence or six-gate acceptance.
