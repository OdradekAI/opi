# Observe a request duration histogram creation failure at the public Models entry points

Status: done
Owner: Codex for user
Parent: [alignment specification](../spec.md)
Predecessor: [first event write failure](48-observe-first-chunk-event-write-failure.md)
Baseline: opi `ee18dd1323a089d536bdc7e293d0b4869777c344` with the #48 worktree inputs

## Frozen replay

`models-request-duration-histogram-creation-failure` registers the same
controlled successful provider used by the other public completion probes.
Each driver uses a fresh in-memory recorder and intercepts creation of the
`pi.ai.request.duration` histogram, measured in ms. The adapter injects
`controlled-request-duration-histogram-creation-failure` on the first
attempt. The attempt and public call boundary both record zero dispatches
and an unstarted source on each side.
[Pi driver](../tools/observe_upstream.mjs#L495) · [Rust driver](../../../../crates/opi-ai/examples/lifecycle_differential.rs#L949)

| Observation | Pi `Models.streamSimple` | Rust `Models::stream_simple` |
| --- | --- | --- |
| Factory fault | One synchronous throw on a cold metrics adapter | One typed `TelemetryError` |
| Public call boundary | The same injected Error escapes synchronously. No call handle | Returns a `ChatCall` |
| Provider execution | Zero dispatches and zero source iterators through a `setImmediate` checkpoint | One dispatch at the source gate before any consumer poll. Public `Start`, successful `Done`, outer EOF and matching `Ok` result. Actual source EOF and Drop |
| `pi.ai.request` | No request span recorded at the fault, catch or checkpoint | Open with `stop` absent at the fault, call return and after the successful request. Backend wrapper dropped by call return. Zero backend `finish` calls |
| Host diagnostics | Zero `unhandledRejection` events through the checkpoint after the caller caught the error | One tracing WARN for observer startup with the injected marker |

The frozen Pi source is `v1.5.0` at
`f9a26365568127461c47a07a37dcb42c140a3ac5`. `packages/ai/src/models.ts:929–940`
evaluates the observer before entering `lazyStream`, as does the executed
`packages/ai/dist/models.js:468–474` export. On a cold adapter,
`packages/ai/src/telemetry.ts:109–133` creates the instruments before
starting the request span at lines 151–153.
`packages/telemetry/src/metrics.ts:141–143` calls the histogram factory
synchronously. The probe uses a real recorder parent span whose metrics
adapter identity is the fresh recorder. Request span snapshots filter for
`pi.ai.request`.

In this replay Rust started the request span before creating the histogram.
The typed factory error exited `AiRequestTelemetry::start` before the observer took
ownership of the span. `SpanOwner::finish` is the explicit settlement
operation. The recorder and wrapper counters show the owner was lost
without settlement on this path. The automatic observer reports the typed
failure and `ChatCall::new` schedules execution with observation disabled.
[Span owner](../../../../crates/opi-telemetry/src/context.rs#L92) · [Observer startup](../../../../crates/opi-ai/src/telemetry.rs#L431) · [Public call construction](../../../../crates/opi-ai/src/models.rs#L5059)

## Follow-up

[#50](50-create-ai-instruments-before-starting-span.md) implements the
initialization order change and replays this fault. Its decision records
the accepted public call and provider execution difference for the
repaired path.
[#50 accepted decision](50-create-ai-instruments-before-starting-span.md#accepted-decision)
[Lifecycle policy](../decisions/11-telemetry-lifecycle.md#answer)

## Verification

- [Material manifest](../evidence/phase5-request-duration-histogram-creation-failure-materials.json), SHA-256 `8b56a97e73737ddd6cd0379fe25b33a5f827ffe81c2cbeb7ae1d3dc3522cf78a`.
- [Replay commands and raw traces](../evidence/phase5-request-duration-histogram-creation-failure-differential.json), SHA-256 `337ae6705461422f1a80b3232877ca685bff854a02aec24a58497dd179059388`.
- The original-lock candidate at `C:/Temp/opi-ai-frozen-lock-20261111`
  supplied 27 TypeScript and nine Rust invocations on Windows x64. Each
  TypeScript triplet has identical stdout. All nine shared comparisons
  pass with zero material blockers. Two cases pass and seven record
  differences, including six with prior decisions. The raw verdict is
  `DIFFERENCES_REQUIRE_DECISION` / exit 0 with `strict_alignment: false`.
- All eight #48 case records, including driver outputs and comparison
  results, match. The two standalone Pi `lazyStream` completion cases match
  all six #42 stdout streams.
- The #48 manifest is rejected before build or driver execution with
  `STALE_MATERIALS` / exit 2. The changed bindings are the Rust example,
  TypeScript observer, comparator and runner. The upstream source, locks,
  installed and generated materials, Rust library trees and tool versions
  match #48.
- The Python differential suite passed 36 tests. Cached Rust
  `test -p opi-ai --example lifecycle_differential --offline -j4` passed four
  tests, and cached Clippy on that example passed with `-D warnings`.
  Node syntax, Cargo format, documentation and diff checks passed.

Test impact `add` for the paired histogram factory failure and comparator
regressions for the instrument, call phase, span settlement and public
outcome. `update` for the drivers, comparator, runner and Rust subprocess
assertions. `retain` for the eight prior replay cases, standalone Pi
completion and Rust Drop and telemetry regressions.

Worktree handoff `committed=none-for-this-task`, `staged=none`. The drivers,
comparator, runner, Python tests, differential README, spec, index and
lifecycle decision were carried in and updated. This issue and its two JSON
records are new untracked paths. The replay exercises existing production
code.
