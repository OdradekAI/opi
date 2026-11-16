# Observe a result attribute write failure at the public Models entry points

Status: done
Owner: Codex for user
Parent: [alignment specification](../spec.md)
Predecessor: [public terminal and request span timing](45-observe-public-terminal-and-span-before-source-eof.md)
Baseline: opi `ee18dd1323a089d536bdc7e293d0b4869777c344` with the #39–#45 worktree changes

## Frozen replay

`models-result-attribute-write-failure` uses the original-lock candidate
and the controlled successful source from #45. Each driver reads public
`Start`, releases the source terminal gate and observes source EOF. The
test adapters delegate to the actual in-memory recorders and intercept
the first request span write carrying `stop` and `usage.input`. The hook
samples the source at the point of failure. The drivers observe one
failed write after source EOF and collect public `Done`, outer EOF and
the result from the same retained call.

| Observation | Pi `Models.streamSimple` | Rust `Models::stream_simple` |
| --- | --- | --- |
| Failed write | Synchronous throw after source EOF | Typed `TelemetryError` after source EOF and source Drop |
| Public outcome | One `Done`, outer EOF and matching successful `result()` | One `Done`, outer EOF and matching successful `result()` |
| `pi.ai.request` | Open at the post-fault checkpoint, with `stop` absent | Open after execution completes, with `stop` absent. Zero backend `finish` calls and observed wrapper Drop |
| Host diagnostics | Zero `unhandledRejection` events through the post-fault event-loop checkpoint | One tracing `WARN` with the injected error marker, captured by the probe's subscriber |

In the frozen Pi `v1.5.0` source at `f9a26365568127461c47a07a37dcb42c140a3ac5`,
`packages/ai/src/api/lazy.ts:37–52,68–83` forwards `Done`, reads the source
to EOF and calls `onResult` after marking it observed. The injected throw
enters the forwarder's catch. The outer stream already holds the terminal,
and the guard prevents another `onResult` invocation. The write at
`packages/ai/src/telemetry.ts:181–207` throws before the callback releases
the span promise. The driver samples the recorder after the injected throw
and a `setImmediate` checkpoint. The observation is bounded to that checkpoint.

In the #46 replay, the Rust result write returns an error before the explicit
`SpanOwner::finish` call. The automatic observer logs the error and clears
its inner telemetry state. The span backend wrapper is released. The recorder
retains the open span because logical completion requires an explicit finish.
[Span owner](../../../../crates/opi-telemetry/src/context.rs#L87)

The comparator records the host diagnostic difference as
`result-attribute-write-failure`. The shared assertions verify the same
fault stage, source completion, business result and request identity.
Both recorders show the same open state in this case.

## Follow-up

The user replied “继续推进” after the proposal to attempt span completion
and preserve the successful model outcome and write diagnostic. #47
implements that behavior for the automatic observer on a completed chat
request and replays this same fault on the original-lock candidate.
[#47 span completion after a failed write](47-finish-request-span-after-result-attribute-write-failure.md)

## Verification

- [Material manifest](../evidence/phase5-result-attribute-write-failure-materials.json), SHA-256 `4431c34b91f0c62cccd6efda3db970db021fd85d5e8fa1f58a6fea93810cd535`.
- [Replay commands and raw traces](../evidence/phase5-result-attribute-write-failure-differential.json), SHA-256 `5d8fa143b5a25852e4471b7029c3ce362f76014be53a2225ba26ce613f9bceb2`.
- The candidate at `C:/Temp/opi-ai-frozen-lock-20261111` supplied 21
  TypeScript and seven Rust invocations. All seven shared comparisons
  passed with zero material blockers. The raw verdict is
  `DIFFERENCES_REQUIRE_DECISION` / exit 0 with `strict_alignment: false`.
  The six prior comparison records and driver stdout match #45 byte for
  byte. The two standalone Pi `lazyStream` completion cases reproduce
  the six recorded #42 stdout streams.
- The runner rejects the #45 manifest before driver execution with
  `STALE_MATERIALS` / exit 2. The Rust example, TypeScript observer,
  comparator and runner are the four changed bound inputs. The upstream
  source, locks, installed/generated materials, Rust library trees and
  runtime versions match #45.
- The Python differential suite passed 30 tests. Cached
  `test -p opi-ai --example lifecycle_differential --offline -j4` passed two
  Drop tests. Cached Clippy on the example passed with `-D warnings`.
  Cargo format, the documentation checker and diff checks passed.

Test impact `add` for the injected result write failure and invalid fault,
span and diagnostic traces. `update` for the two drivers and their
comparator and runtime assertions. `retain` for the six prior cases,
standalone probes and Drop regressions.

Worktree handoff `committed=none-for-this-task`, `staged=none`. The probes,
comparator, runner, tests, differential README, spec, index, lifecycle
decision and #45 issue were carried in and updated. This issue and its
two JSON records are new untracked paths. The replay and checks ran on
Windows x64.
