# #124 OpenAI Chat response observer failure and response release

Status: verified

## Finding and priority

The shared OpenAI-compatible Chat producer already invokes the response
observer before body processing on successful responses. The remaining
adapter/observer gap was an executable failure-stage proof: a response
observer can reject after headers are acquired, and the same provider must
still accept a later request after the rejected response is released.

This is a caller-visible lifecycle contract for the native OpenAI Chat route
and its shared `stream_prepared` producer. The source comparison found no
additional Rust behavior defect in this stage, so the slice adds the missing
regression and paired evidence without changing production code.

## Contract

- Each acquired successful response invokes `onResponse` once before body
  normalization. A functional observer failure becomes typed `CallbackPhase`
  failure and publishes no normalized stream event or `Done`.
- The failed response is released when the producer ends. A subsequent
  request through the same `OpenAiChatProvider` can complete normally and
  preserve its final usage (`2/1/3` in the fixture).
- Ordinary `Models::stream_simple` and the low-level
  `OpenAiChatProvider::stream_prepared` route share this ownership boundary;
  direct and other OpenAI-compatible Chat callers retain the existing shared
  producer contract.
- Frozen Pi emits an `error` AssistantMessage event and resolves its result;
  Rust returns a typed `CallFailure` with `CallbackPhase`. This is a reporting
  shape difference, not a response-release or subsequent-call difference.

## Verification

- `openai_chat_response_observer` passes 1/1. The first call records a typed
  callback failure with no successful event; the second call succeeds, emits
  `stop`, and reports input/output/total usage `2/1/3`.
- Focused Clippy with `-D warnings` passes.
- Frozen Pi 1.5.0 revision `f9a26365568127461c47a07a37dcb42c140a3ac5` and Rust
  agree for three stable runs: observer statuses `200,200`, first result
  `error`, second result `stop`, two requests, and the same usage. The
  canonical receipt is
  `phase7-openai-chat-response-observer-differential.json`.
- The actual `opi-ai` archive consumer passes builtins/no-tracing and defaults
  profiles. The first defaults run failed only because the shared incremental
  target had no free space; its receipt is retained as `-v1`. After the
  user-authorized reviewed incremental eviction plan v2 removed 5,024
  rebuildable payload files and recovered 7,924,396,032 bytes, the refreshed
  `-v2` package receipt passed and became canonical.

## Historical evidence and remaining scope

The probe, comparator, package runner and receipts bind the frozen source,
fixture, Rust executable and exact archive source bytes. No live provider call,
publication, commit or stage was performed. Other adapter/observer fault
stages and repeated/pending paths, complete configuration set V,
MSRV/native-platform evidence, quantitative budgets and real Deferred
integration (#94) remain open.

Test impact: `add` the OpenAI Chat response-observer failure/release
regression, frozen Pi probe/comparator and archive consumer; `update` the
Cargo test target list; `delete` none; `retain` all prior adapter, observer,
Chat terminal/preview, usage, numeric, cancellation, deferred and package
evidence.
