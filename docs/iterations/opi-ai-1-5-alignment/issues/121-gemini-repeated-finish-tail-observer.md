# Gemini repeated finish tails and observer fault stages

Status: verified against the frozen Pi 1.5.0 snapshot and the current `opi-ai`
archive.

The Google Generative AI adapter receives `finishReason` and usage in separate
response chunks. A finish marker therefore records a pending terminal state;
it does not publish `Done` while the source can still deliver usage or a raw
observer outcome. Repeated finish markers are consumed through source EOF, and
the last usage snapshot is retained. EOF closes the open text block and emits
one `Done` with the final stop reason and usage.

The same `GeminiMapper` owns the Google Generative AI and Vertex adapter paths.
The public `GeminiProvider::stream_prepared` and `VertexProvider::stream_prepared`
routes use the deferred source terminal, while the offline fixture helpers use
the mapper's fixture finish boundary. The simple and direct public routes were
paired in the probe; the shared mapper and existing Vertex fixture coverage
provide the complementary adapter mapping.

## Fault-stage contract

Raw provider callbacks run before normalization for each complete response
chunk. If the callback fails on a tail chunk after a finish marker, the typed
callback failure is delivered with the accumulated partial message and no
deferred `Done` is flushed. A parsed provider error and transport/cancellation
failure follow the same terminal ownership rule. A normal EOF with at least one
finish marker is the only successful publication boundary.

The public offline fixture helpers use the same fault boundary. A malformed
tail after a finish marker now suppresses the deferred fixture `Done`, matching
the HTTP path's immediate typed failure instead of returning an error followed
by an obsolete terminal event.

The regression fixture has one text chunk, a `STOP`/usage-5 chunk, and a
repeated `STOP`/usage-7 chunk. The local Rust fixture helper emits the two
normalized route labels used by the public simple/direct pairing, then
separately exercises the HTTP adapter and fails the raw observer on the third
frame. That check confirms a typed `CallbackPhase` failure and no `Done`.

## Pairing and package evidence

- Frozen Pi revision: `f9a26365568127461c47a07a37dcb42c140a3ac5`.
- The Pi probe records all three provider chunks through the raw observer. It
  and the Rust normalized trace agree for three stable runs on simple/direct
  routes: `stop`, raw `STOP`, input 10, output 7, one `Done`, and the event
  sequence `start`, `text_start`, `text_delta`, `text_end`, `done`.
- The actual archive consumer copies the test and fixture from the generated
  `opi-ai` package and passes both `builtins`/no-tracing and default feature
profiles. The archive run executes the repeated-tail, observer-fault and
malformed-tail regressions.

Evidence is recorded in
[`phase7-gemini-repeated-finish-materials.json`](../evidence/phase7-gemini-repeated-finish-materials.json),
[`phase7-gemini-repeated-finish-differential.json`](../evidence/phase7-gemini-repeated-finish-differential.json),
[`phase7-gemini-repeated-finish-package.json`](../evidence/phase7-gemini-repeated-finish-package.json),
and [`phase7-gemini-repeated-finish-checks.json`](../evidence/phase7-gemini-repeated-finish-checks.json).

Pi exposes raw provider callback records in its trace. The Rust normalized
fixture intentionally exposes the public event sequence instead; the HTTP
regression independently verifies raw callback ordering and typed failure.
This is a reporting-shape difference, not a terminal behavior difference.

Test impact: `add` the repeated finish/usage, tail observer-failure and
malformed-tail regressions, frozen probe/comparator, and archive consumer;
`update` the Gemini/Vertex shared terminal contract; `retain` all prior
adapter, Chat, preview, numeric, cancellation, deferred, and package evidence.
