# Finish a request span after a result attribute write fails

Status: done
Owner: Codex for user
Parent: [alignment specification](../spec.md)
Predecessor: [public result attribute write failure](46-observe-public-result-attribute-write-failure.md)
Baseline: opi `ee18dd1323a089d536bdc7e293d0b4869777c344` with the #46 replay inputs

## Decision

On 2026-11-11 the user replied “继续推进” after the proposal to attempt
span completion on the failed final attribute write and preserve the
successful model result and write diagnostic. The automatic observer
applies this to `pi.ai.request` on a completed chat with `Stop`, `Length`
or `ToolUse`. Ordinary `Models` calls and the observed `ChatStream`
helpers use the same observer. The manual `AiRequestTelemetry::finish`
returns the original backend error and leaves the completion choice
with its caller.
[Lifecycle policy](../decisions/11-telemetry-lifecycle.md#成功-chat-请求的最终遥测属性写入故障)

The failed result attribute write is identified inside the request
telemetry finalizer. The automatic observer warns with the original
error and reuses its explicit error-close path. That path takes ownership
of the observer's telemetry state and attempts to finish its span once.
It skips the remaining result telemetry writes. A backend finish error
produces a second warning, and the business outcome is delivered.
[Automatic observer](../../../../crates/opi-ai/src/telemetry.rs#L462)

## Frozen replay

The paired public `models-result-attribute-write-failure` replay uses
the same Pi `v1.5.0` original-lock candidate and the same controlled
source and fault as #46. The other six comparison records and Rust
driver outputs match #46 byte for byte. All 21 Pi driver records match
#46, including the three repetitions of this fault.

| Observation after source EOF | Pi `Models.streamSimple` | Rust `Models::stream_simple` |
| --- | --- | --- |
| Failed result attribute writes | One synchronous throw | One typed error, after source Drop |
| Business outcome | One successful `Done`, public EOF and matching `result()` | One successful `Done`, public EOF and matching `result()` |
| `pi.ai.request` | Open with `stop` absent at the post-fault checkpoint | One explicit backend finish, settled with `stop` absent. Backend wrapper dropped |
| Host channel | Zero `unhandledRejection` events through the checkpoint | One tracing `WARN` with the write fault marker |

`result-attribute-write-failure` records the span settlement and host
diagnostic difference. The Rust behavior gives the backend a chance to
close its observation when it rejects the final attributes. This decision
covers the observed successful result and typed write failure. The Rust
regression for a second fault injects a finish error before the in-memory
backend records completion. It observes one finish attempt, an open
recorded span, two independent warnings and the same successful outcome.
[Rust failure tests](../../../../crates/opi-ai/examples/lifecycle_differential.rs#L2564)

## Verification

- [Material manifest](../evidence/phase5-result-attribute-write-recovery-materials.json), SHA-256 `4f37ac3c6a635effe3e8d08d6dbf313b851ed9d5f04e5882966eb6775690f987`.
- [Replay commands and raw traces](../evidence/phase5-result-attribute-write-recovery-differential.json), SHA-256 `58e1f5f893064bfd111ed59cbfcdc3e8eafa67521167712422120ff399e85b3b`.
- All seven shared comparisons passed on Windows x64 with 21 TypeScript
  and seven Rust invocations and zero material blockers. Two cases pass
  and five have recorded differences. The raw runner verdict is
  `DIFFERENCES_REQUIRE_DECISION` / exit 0 with `strict_alignment: false`.
- The #46 manifest is rejected before any driver runs with
  `STALE_MATERIALS` / exit 2. The only bound changes are the Rust source
  tree and Rust example. The upstream source, locks, installed and
  generated materials, Rust telemetry dependency and tool versions match
  #46. The historical #46 JSON digests were verified.
- The Python differential suite passed 30 tests. Cached Rust
  `test -p opi-ai --test ai_telemetry --test execution_lifecycle --test telemetry_operations --example lifecycle_differential --offline -j4`
  passed 65 tests (20, 29, 12 and 4). Cached Clippy on the library and
  these targets passed with `-D warnings`. Cargo format, documentation
  and diff checks passed.

Test impact `add` for the Rust double-fault and manual completion tests.
`update` for the expected span state in the comparator fixtures and the
Rust subprocess. `retain` for all seven replay inputs, prior evidence and
existing lifecycle coverage.

Worktree handoff `committed=none-for-this-task`, `staged=none`. The Rust
telemetry source edit is task-owned. The Rust driver, Python test fixtures,
changelog, bilingual README, differential README, spec, index, lifecycle
decision and #46 issue were carried in and updated. This issue and its
two evidence files are new untracked paths.

## Remaining

The [nonterminal event fault probe](48-observe-first-chunk-event-write-failure.md)
observes the failure before the business terminal is locked in.
Full differential and six-gate acceptance remain open.
