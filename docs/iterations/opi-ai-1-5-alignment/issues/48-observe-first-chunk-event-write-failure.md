# Observe a first nonterminal event write failure at the public Models entry points

Status: done
Owner: Codex for user
Parent: [alignment specification](../spec.md)
Predecessor: [span completion after a failed result attribute write](47-finish-request-span-after-result-attribute-write-failure.md)
Baseline: opi `ee18dd1323a089d536bdc7e293d0b4869777c344` with the #47 worktree inputs

## Frozen replay

`models-first-chunk-event-write-failure` drives the public `Models` entries
with a controlled successful provider and a gate before its `Done`. An
adapter delegates to the real in-memory recorder and fails the first
`pi.ai.first_chunk` event write on the request span. Both writes occur on
the provider's `Start`, with one dispatch, zero source terminals and source
EOF still pending. Both drivers read public `Start`, sample the next item
and the request span while the terminal gate is held, then observe the
public terminal, outer EOF and the matching `result()` on the same call.
[Pi driver](../tools/observe_upstream.mjs#L460) · [Rust driver](../../../../crates/opi-ai/examples/lifecycle_differential.rs#L906)

| Observation | Pi `Models.streamSimple` | Rust `Models::stream_simple` |
| --- | --- | --- |
| Event write fault | One synchronous throw | One typed `TelemetryError` |
| While the provider terminal is held | `next()` yields an in-band `error` with `controlled-first-chunk-event-write-failure` | `next()` remains pending after `Start` |
| Source release | One completed iterator `return()` and one generator `finally`. Zero source terminals and zero calls to the source `result()`. Natural EOF is false | The source remains held at the gate. After release it emits one `Done`, reaches natural EOF and is dropped |
| Public result after outer EOF | `result()` fulfills with the same message as the error event, `stopReason: "error"` | `result()` returns `Ok` with the same message as `Done`, `stop_reason: Stop` |
| `pi.ai.request` | Settled at the held gate with `stop: "error"` | Settled at the held gate with `stop` absent. One backend `finish` call and wrapper Drop observed |
| Host channel | Zero `unhandledRejection` events through the post-result event-loop checkpoint | One tracing `WARN` carrying the injected fault marker |

The frozen Pi source is `v1.5.0` at
`f9a26365568127461c47a07a37dcb42c140a3ac5`. `packages/ai/src/api/lazy.ts:37–52`
pushes `Start` before invoking the observer. The synchronous throw exits
the `for await` loop and the iterator closes before the catch publishes its
synthetic `error` (`api/lazy.ts:74–83`). `packages/ai/src/telemetry.ts:155–161`
sets `first = false` before the failing write, so observing the synthetic
error does not retry it. The final observer records `stop: "error"` and
ends the request span (`telemetry.ts:181–207`). The `return()` and `finally`
observations establish release for this controlled generator.

Rust records the warning and invokes `finish(None)` while disabling the
failed observer. The collector continues to forward the event and process
the provider source. The probe holds a single public `next()` across gate
release. Both probes call `result()` after public EOF.
[Rust event handling](../../../../crates/opi-ai/src/stream.rs#L698) · [Observer failure](../../../../crates/opi-ai/src/telemetry.rs#L445) · [Explicit close](../../../../crates/opi-ai/src/telemetry.rs#L487)

## Accepted decision

On 2026-11-11 the user replied “继续推进” to the recommendation to keep
Rust observer fault isolation in this case. The accepted
`first-chunk-event-write-failure` difference covers one typed
`pi.ai.first_chunk` write failure on the provider's `Start` at the public
Models entry point. Rust reports the original fault, disables observation
and makes one explicit attempt to end the request span. The in-memory span
settled before the provider terminal with no `stop` attribute. The provider
then ran through natural EOF and release, and the call delivered its
successful terminal and result. The choice preserves the provider's business
outcome and gives the host a diagnostic for the telemetry failure.

Startup, other event writes, repeated observer throws, Rust panics and
backend finish failures during this stage require their own evidence.
[Lifecycle policy](../decisions/11-telemetry-lifecycle.md#answer)

## Verification

- [Material manifest](../evidence/phase5-first-chunk-event-write-failure-materials.json), SHA-256 `93284f92f7d5527d241203dfd411d965beddd1334ac294cf7e8b36aee35bcee4`.
- [Replay commands and raw traces](../evidence/phase5-first-chunk-event-write-failure-differential.json), SHA-256 `18bc1e63b2a7c30e2e5af8c365515d68fdb3e76d389b395b4b5d24bbf225d7c0`.
- The original-lock candidate at `C:/Temp/opi-ai-frozen-lock-20261111`
  supplied 24 TypeScript and eight Rust invocations on Windows x64.
  All eight shared comparisons passed with zero material blockers. Two
  cases pass and six have recorded differences. The raw verdict is
  `DIFFERENCES_REQUIRE_DECISION` / exit 0 with `strict_alignment: false`.
- The seven #47 comparison records and all corresponding driver outputs
  match byte for byte. The two standalone Pi `lazyStream` completion cases
  match all six #42 stdout streams. The new cleanup counters are emitted
  by the first-chunk failure case.
- The #47 manifest is rejected before build or driver execution with
  `STALE_MATERIALS` / exit 2. The changed bindings are the Rust example,
  TypeScript observer, comparator and runner. The upstream source, locks,
  installed and generated materials, Rust library trees and tool versions
  match #47.
- The Python differential suite passed 33 tests. Cached Rust
  `test -p opi-ai --example lifecycle_differential --offline -j4` passed
  four tests. Cached Clippy on the example passed with `-D warnings`.
  Node syntax, Cargo format, documentation and diff checks passed.
- The decision update verified the frozen inputs, toolchain, driver and
  executable identities and both evidence digests. Two cases pass and six
  have accepted differences in the decision records. The frozen verdict
  remains `DIFFERENCES_REQUIRE_DECISION` with `strict_alignment: false`.

Test impact `add` for the paired first event failure and comparator
regressions for fault phase, actual cleanup and outcome. `update` for the
two drivers, comparator, runner scope and Rust subprocess assertions.
`retain` for the seven prior cases, standalone Pi completion and Rust
Drop and telemetry regressions. Test impact for the decision update is
`none`.

Worktree handoff `committed=none-for-this-task`, `staged=none`. The drivers,
comparator, runner, Python tests, differential README, spec, index, lifecycle
decision and #47 issue were carried in and updated. This issue and its two
JSON records are new untracked paths. The replay exercises existing
production code.
