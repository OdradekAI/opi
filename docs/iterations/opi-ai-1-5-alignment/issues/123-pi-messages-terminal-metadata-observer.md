# #123 Pi Messages terminal metadata, observer failure and response release

Status: verified

## Finding and priority

The native `pi-messages` adapter already stopped at the first `done`/`error`
event and therefore matched Pi's immediate terminal boundary, repeated terminal
events and `[DONE]` tails were not reprocessed. Its terminal mapper parsed
`responseId` and `providerThinkingLevel` but dropped Pi's `responseModel`, which
is caller-visible metadata on the final `AssistantMessage`. The focused route
also needed an explicit proof that a raw observer failure at the terminal does
not publish `Done` and does not prevent the provider from accepting a later
request.

## Contract

- A `done` or `error` event is the terminal boundary for `pi-messages`; the
  first terminal is normalized and later wire events, including repeated
  `done` and `[DONE]`, are not observed by the adapter.
- Terminal `responseModel`, `responseId`, provider thinking level, usage and
  rewrite metadata are copied into the final message before the single `Done`
  or typed error is published.
- The raw observer runs before normalization. If it fails on the terminal, the
  stream returns a typed `CallbackPhase` failure with partial state and no
  `Done`; the response body is released sufficiently for a subsequent request
  through the same provider.
- Rust retains its typed provider error and normalized event shape. Pi's error
  path and provider-event trace remain reporting differences; they do not alter
  the terminal boundary or final metadata contract.

## Implementation and caller map

`PiMessagesSseMapper` now maps `responseModel` in the shared `done`/`error`
branch. The public producer is `PiMessagesProvider::stream_prepared`, with
`stream_simple`, `PiMessagesChatApi`, `Models::{stream_simple,complete_simple,
stream_direct,complete_direct}` and the existing Agent result/cost/transcript
consumers using the same terminal message. `RequestCallbacks::on_raw` remains
the raw observer seam before normalized events. No pending terminal or EOF
flush is added because the Pi Messages protocol's terminal event is itself the
publication boundary.

## Verification

- `chat_pi_messages` passes 42/42, including the existing usage/rewrite fixture
  now asserting `responseModel`.
- `pi_messages_terminal` passes 2/2. Its trace covers simple and direct routes,
  one `Done`, `responseModel=radius-final`, usage `10/5/15`, raw events through
  the first terminal only, and a second request after a terminal observer
  failure.
- Focused clippy for `pi_messages_terminal` passes with `-D warnings`.
- Frozen Pi 1.5.0 revision `f9a26365568127461c47a07a37dcb42c140a3ac5` agrees
  with Rust for three stable runs on both routes. The canonical differential
  receipt is `phase7-pi-messages-terminal-differential.json`; the pre-assertion
  receipt and refreshed v2 receipt remain as `-v1` and `-v2` history.
- The actual `opi-ai` archive consumer passes builtins/no-tracing and defaults
  profiles, including metadata, clippy and both regressions. The canonical
  receipt is `phase7-pi-messages-terminal-package.json`; the pre-assertion
  package receipt and refreshed v2 receipt remain as `-v1` and `-v2` history.

## Historical evidence and remaining scope

The source probe, comparator, package runner and receipts record the frozen
source hash, fixture, Rust executable and exact command outputs. This slice
does not claim strict equivalence for Pi's error AssistantMessage shape, full
configuration set V, MSRV/native-platform evidence, quantitative budgets or
real Deferred integration (#94). Other adapter/observer fault stages and
repeated/pending paths remain for the next slice.

Test impact: `add` Pi Messages terminal metadata, repeated boundary, observer
failure/release regression, frozen Pi probe/comparator and archive consumer;
`update` the shared Pi Messages terminal mapping and fixture assertion;
`delete` none; `retain` all prior adapter, observer, Chat, preview, usage,
numeric, cancellation, deferred and package evidence. No commit or stage was
performed.
