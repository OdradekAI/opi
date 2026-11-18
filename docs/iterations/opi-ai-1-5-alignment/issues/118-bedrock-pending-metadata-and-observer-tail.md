# #118 Hold Bedrock pending terminal through repeated metadata and observer tails

Status: verified

## Finding and priority

The Bedrock Converse adapter recorded a messageStop as a pending Done, then
released it on the first metadata frame. A repeated metadata frame could
therefore arrive after the terminal event, and a raw observer failure on that
frame was suppressed because the HTTP loop had already observed Done. This was
the highest-priority remaining adapter/observer gap after #117: final usage and
tail failures were observable to callers, Agent accounting and session
persistence.

## Contract

- messageStop records the normalized and raw stop reason but does not publish
  Done.
- Every later provider item, including repeated metadata, reaches the raw
  observer and mapper before terminal publication.
- The last valid metadata usage is authoritative for the final message.
- Done is published only after the provider stream reaches EOF and all open
  blocks are flushed.
- A tail observer or mapper failure is emitted as a typed stream failure before
  Done; no terminal success follows that failure.
- A stream that reaches EOF without metadata still publishes the pending stop
  with the usage accumulated so far.

This is the same item-consumption boundary as frozen Pi 1.5.0. Opi keeps its
typed Rust ProviderError::CallbackPhase wrapper and owned partial message; that
error-shape difference remains intentional.

## Implementation and caller map

BedrockMapper now retains pending_done through all metadata frames and flushes
it only at source EOF. The shared Bedrock HTTP route continues to call on_raw
before mapping each frame, so the change covers:

| Seam | Owner/caller | Evidence |
| --- | --- | --- |
| Event stream mapper | BedrockProvider::stream_from_fixture / BedrockMapper | crates/opi-ai/src/api/bedrock.rs, tests/bedrock_pending_metadata.rs |
| Production HTTP adapter | BedrockProvider::stream_prepared and Converse stream route | tests/bedrock_fixtures.rs |
| Public Chat | Models::stream_simple/complete_simple and direct prepared callers | shared provider stream contract |
| Agent/session | existing Chat result, cost and transcript persistence consumers | existing Agent/session suites; no new ownership seam |

The non-Chat image/classifier/deferred-cancel observer boundary from #19 is
unchanged.

## Verification

- Rust fixture: repeated metadata produces one Done, stop, raw end_turn, final
  usage 12/7/cacheRead=3, and start -> text_start -> text_delta -> text_end ->
  done.
- The full 179-case Bedrock fixture suite also verifies EOF flushing keeps
  cache_write_1h_tokens; the first 178/1 run that exposed this regression is
  retained as historical evidence.
- Production wire test: a raw observer failure on the second metadata frame is
  delivered as a typed callback failure, after two metadata observations and
  before any Done.
- Frozen Pi revision f9a26365568127461c47a07a37dcb42c140a3ac5 matches the Rust
  trace for three repetitions.
- The actual packaged opi-ai archive consumes the test from an external project
  under builtins without tracing and default features.
- Cargo formatting, focused clippy, documentation checks and git diff --check
  are recorded in the phase-7 v3 evidence files.

## Remaining scope

Other adapter/observer failure stages, complete configuration set V,
MSRV/native platforms, quantitative budgets and real Deferred integration
(#94) remain open. This does not claim full Pi strict equivalence or the
six-gate acceptance result.
