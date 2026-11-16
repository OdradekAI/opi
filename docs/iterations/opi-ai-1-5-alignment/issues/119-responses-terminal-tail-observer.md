# #119 Hold OpenAI Responses terminals through repeated frames and observer tails

Status: verified

## Finding and priority

The shared OpenAI Responses HTTP adapter published Done as soon as it mapped
`response.completed` or `response.incomplete`, and then suppressed raw observer
callbacks once `saw_done` was set. Frozen Pi 1.5.0 keeps consuming its provider
async iterable after the terminal response item, so a repeated terminal usage
snapshot or a tail observer failure remained visible to the caller. This was
the highest-priority remaining Responses adapter/observer gap after #118 and
applies to both the OpenAI and Azure Responses HTTP routes.

## Contract

- `response.completed` and `response.incomplete` record the terminal facts and
  final usage as a pending terminal; they do not publish Done immediately.
- Every later provider item reaches the raw observer. A later terminal frame
  can replace the pending usage and stop facts, matching Pi's last consumed
  snapshot behavior.
- Done is published exactly once after source EOF and only if no tail callback,
  mapper, transport or cancellation failure occurred.
- A tail observer failure is a typed `CallbackPhase` stream failure before Done;
  no terminal success follows that failure.
- The existing Rust typed error and owned partial message remain intentional
  shape differences from Pi's error object.

## Implementation and caller map

`ResponsesMapper` now has an opt-in deferred-terminal mode. OpenAI Responses
and Azure Responses HTTP/fixture entry points enable it; the Codex WebSocket
terminal control and existing SSE early-stop behavior remain unchanged.

| Seam | Owner/caller | Evidence |
| --- | --- | --- |
| Shared Responses mapper | `ResponsesMapper::process` and `flush_pending_terminal` | `crates/opi-ai/src/openai_responses_shared.rs`, `tests/responses_repeated_terminal.rs` |
| OpenAI Responses HTTP | `OpenAiResponsesProvider::stream_http` and `stream_from_sse` | `crates/opi-ai/src/openai_responses.rs`, `tests/openai_responses_lifecycle.rs` |
| Azure Responses HTTP | `stream_azure_responses_http` | `crates/opi-ai/src/azure_openai.rs`, `tests/azure_openai_fixtures.rs` |
| Public Chat and Agent/session | Models simple/direct and existing result/cost/transcript consumers | final message snapshot is published after the shared stream settles |

## Verification

- OpenAI Responses regression: repeated terminal usage 10/5 then 12/7 emits
  one Done with the final 12/7/19 snapshot.
- OpenAI Responses production wire regression: the third raw frame fails the
  callback; all three frames are observed and no Done is published.
- Azure Responses production wire regression covers the same tail failure on
  the shared Azure route.
- Full adapter suites pass: OpenAI Responses 14/14 and Azure OpenAI 46/46.
- Frozen Pi revision `f9a26365568127461c47a07a37dcb42c140a3ac5` matches the
  normalized Rust trace for three runs; Pi's provider event list is retained as
  an explicit observation because the Rust fixture helper exposes normalized
  events rather than callback delivery.
- The actual packaged `opi-ai` archive passes the external consumer under
  builtins without tracing and default features.

## Remaining scope

Other adapter/observer fault stages and repeated/pending paths, complete
configuration set V, MSRV/native platforms, quantitative budgets and real
Deferred integration (#94) remain open. This slice does not claim full Pi
strict equivalence or six-gate acceptance.
