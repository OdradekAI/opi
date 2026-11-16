# Observe a request span start failure at the public Models entry points

Status: done
Owner: Codex for user
Parent: [alignment specification](../spec.md)
Predecessor: [instrument creation before span start](50-create-ai-instruments-before-starting-span.md)
Baseline: opi `ee18dd1323a089d536bdc7e293d0b4869777c344` with the #50 worktree inputs

## Frozen replay

`models-request-span-start-failure` uses the public Models calls, fresh
in-memory recorders and a controlled successful provider. Both adapters
record four successful instrument creations before attempting to open
`pi.ai.request` once. The attempt injects
`controlled-request-span-start-failure` before provider dispatch. The Pi
adapter returns a rejected Promise before invoking the span callback.
The Rust adapter returns a typed `TelemetryError` before delegating the
span start to the recorder.
[Pi driver](../tools/observe_upstream.mjs#L538) · [Rust driver](../../../../crates/opi-ai/examples/lifecycle_differential.rs#L1021)

| Observation | Pi `Models.streamSimple` | Rust `Models::stream_simple` |
| --- | --- | --- |
| Public construction | Returns the stream with zero provider dispatches at return | Returns `ChatCall` with zero provider dispatches at return |
| Source while the provider terminal is held | One dispatch and public `Start`. Iterator `return()` completes and the generator enters `finally`. Zero source terminals and source `result()` calls. Natural EOF is false | One dispatch and public `Start`. The next public item is pending while the source is held |
| Public terminal and result | In-band `error` from the `addEvent` TypeError, outer EOF and fulfilled `result()` carrying the same error message | Source `Done`, natural EOF and Drop followed by public `Done`, outer EOF and matching `Ok` result |
| `pi.ai.request` | Zero request spans at the fault, call return, held gate and final checkpoint | Zero at the same checkpoints, zero backend finish calls and no span wrapper Drop |
| Host diagnostic | Two `unhandledRejection` events through the post-result checkpoint. The first matches the injected Promise and Error by identity. The second is a `setAttributes` TypeError | One tracing WARN with the original startup fault marker |

The executed Pi `v1.5.0` revision is
`f9a26365568127461c47a07a37dcb42c140a3ac5`. The observer obtains instruments
before invoking the starter with `void start(...)`
(`packages/ai/src/telemetry.ts:109–153`). The typed starter returns the
adapter Promise (`packages/telemetry/src/index.ts:427–443`). Its rejection
leaves the observer's span unset. The public entry proceeds to `lazyStream`
and returns its handle (`packages/ai/src/models.ts:929–940`,
`api/lazy.ts:60–85`).

The forwarding loop pushes `Start` before running `onEvent`. Access to
`span.addEvent` then throws, causing `for await` to close the controlled
source iterator (`api/lazy.ts:37–52`, `telemetry.ts:155–161`). The catch
pushes the synthetic error before calling the result observer. The error
event locks the public result (`api/lazy.ts:68–83`,
`utils/event-stream.ts:43–57`). The result observer's access to
`span.setAttributes` throws on the catch chain
(`telemetry.ts:181–207`). The probe installs an `unhandledRejection`
listener, identifies the original rejection by Promise and Error identity
and samples through the final `setImmediate` checkpoint. The iterator
observations describe this controlled generator.

Rust propagates the typed start error to the automatic observer, which
reports WARN and disables AI observation for the call. The call processes
the source through its own successful terminal and release.
[Starter](../../../../crates/opi-ai/src/telemetry.rs#L168) · [Automatic observer](../../../../crates/opi-ai/src/telemetry.rs#L431)

## Accepted decision

On 2026-11-11 the user replied “继续推进” to the recommendation to keep
Rust's automatic observer startup fault isolation in this case. The
accepted `request-span-start-failure` difference covers the business
terminal and result, source exit and host diagnostics after a single
typed `TelemetryError` from starting `pi.ai.request` at the public Models
boundary. The error occurs after all four instrument creations and before
provider dispatch. Rust disables AI observation for the request and reports
the original startup fault through host WARN. The call can then reach the
provider's own outcome and release the source through natural EOF.
[Lifecycle decision](../decisions/11-telemetry-lifecycle.md#请求-span-启动失败)

## Verification

- [Material manifest](../evidence/phase5-request-span-start-failure-materials.json), SHA-256 `ad81332281b5c15cf24157b17fb04e576ac7e8e2f95c7f5f06c79c3ce0f40152`.
- [Replay commands and raw traces](../evidence/phase5-request-span-start-failure-differential.json), SHA-256 `8fa3ab15efb786c3c5335a85f09352dd544e56f15844a5b8ce29553ce2c56eec`.
- The original-lock candidate at `C:/Temp/opi-ai-frozen-lock-20261111`
  supplied 30 TypeScript and ten Rust invocations on Windows x64. Each
  TypeScript triplet has identical stdout. All ten shared comparisons
  pass with zero material blockers. Two cases pass and all eight recorded
  differences have documented decisions. The raw verdict remains
  `DIFFERENCES_REQUIRE_DECISION` / exit 0 with `strict_alignment: false`.
- The nine #50 case records, including driver outputs and comparisons,
  match. The two standalone Pi `lazyStream` completion cases match all
  six #42 stdout streams.
- The #50 manifest is rejected before build and replay with
  `STALE_MATERIALS` / exit 2. Its three changed inputs are the two
  drivers and comparator. The original source, locks, installed and
  generated materials, Rust library trees and toolchains match #50.
- The first example build failed with `E0507` because the shared helper
  borrowed the `ChatCall` that `result(self)` consumes. The helper now
  takes the call by value. The first manifest and failure receipt are
  retained at `C:/Temp/opi-phase5-span-start-initial-f30e04c545ef4f65b481f71055ab302f`.
- The Python differential suite passed 39 tests. The cached Rust
  `test -p opi-ai --example lifecycle_differential --offline -j4` passed
  five tests. Clippy on that example with `-D warnings`, Node syntax,
  Cargo format, documentation and diff checks passed.
- The decision update reverified the frozen material inputs and toolchain,
  executable identity, both evidence digests and all 40 invocation
  records. The eight raw difference cases are covered by the decision index.

Test impact `add` for the paired span start failure and comparator
regressions for the phase, source release and host rejection identity.
`update` for the shared terminal observations and Rust subprocess
assertions. `retain` for the nine prior replays, standalone Pi completion
and Rust Drop and telemetry regressions.
Test impact for the decision update is `none`.

Worktree handoff `committed=none-for-this-task`, `staged=none`. The
drivers, comparator, Python tests, differential README, spec, index and
decision were carried in and updated. Prior issue source links were
updated. This issue and its two JSON records are new untracked paths.
The replay exercises existing production code.
