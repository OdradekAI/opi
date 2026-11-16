# Finish a successful request span after duration recording fails

Status: done
Owner: Codex for user
Parent: [alignment specification](../spec.md)
Predecessor: [request duration recording failure baseline](53-observe-request-duration-record-failure.md)
Baseline: opi `ee18dd1323a089d536bdc7e293d0b4869777c344` with the #53 worktree inputs

## Decision

On 2026-11-11 the user replied “继续推进” to the proposal to attempt
span completion after the #53 recording failure. A typed error from
`pi.ai.request.duration` now reaches the same explicit completion path as
the [failed final attribute write](47-finish-request-span-after-result-attribute-write-failure.md).
The automatic observer applies that path to a `pi.ai.request` ending in
`Stop`, `Length` or `ToolUse`. It reports the original error through host
tracing, consumes the observer and attempts to finish the span once. The
attempt preserves the attributes already written and skips the remaining
result measurements. A backend finish error produces a second warning.
The call delivers the provider's successful outcome.
[Result recording](../../../../crates/opi-ai/src/telemetry.rs#L316) · [Automatic observer](../../../../crates/opi-ai/src/telemetry.rs#L462)

The public manual `AiRequestTelemetry::finish` continues to return the
original typed backend error. The owner stays available for a completion
attempt chosen by the caller.
[Manual finish](../../../../crates/opi-ai/src/telemetry.rs#L250)

## Frozen replay

The replay uses the same original-lock Pi `v1.5.0` candidate and the same
controlled provider and single record fault as #53. Both calls reach
natural source EOF, have a single provider dispatch and deliver the
successful `Done`, public EOF and matching result. The fault occurs after
the successful stop and final usage attributes have been recorded. The
Rust driver also checks the recorded `usage.input` after completion.

| Observation | Pi `Models.streamSimple` | Rust `Models::stream_simple` |
| --- | --- | --- |
| Request span at final checkpoint | Open with `stop: stop` | Settled with `stop: stop`, one backend finish and wrapper Drop |
| Recorded measurements | One first chunk duration sample. Zero request duration, token and cost samples | Same |
| Host diagnostic | Zero `unhandledRejection` events through the bounded checkpoint | One WARN carrying the original record error |

The span settlement and host diagnostics for this single recording error
are a documented intentional difference. Closing a completed operation's
span can succeed after an optional measurement fails, and the host
receives the original fault for diagnosis.
[Lifecycle decision](../decisions/11-telemetry-lifecycle.md#成功请求的时长记录失败)

The paired Rust trace differs from #53 only in `span_finish_calls` and the
final span's `settled` field. The other eleven case records and every Pi
invocation match #53. A Rust double-fault regression also rejects the
backend finish attempt. It observes exactly one attempt, an open span,
two independent warnings and a successful business result. The manual
regression checks the original `std::io::Error` cause and one settled span
after an explicit successful retry.
[Double-fault regression](../../../../crates/opi-ai/examples/lifecycle_differential.rs#L2564) · [Manual regression](../../../../crates/opi-ai/examples/lifecycle_differential.rs#L2650)

## Verification

- [Material manifest](../evidence/phase5-request-duration-record-recovery-materials.json), SHA-256 `a34e275e5fcf187dc713fad02de1885314c011158a36786f6c5651ab01d8e5cc`.
- [Replay commands and raw traces](../evidence/phase5-request-duration-record-recovery-differential.json), SHA-256 `fc51e851b5b57f053fcc768c6b0b49a0b06e1f7a336faebc5d6d6e4f82fff404`.
- The original-lock candidate at `C:/Temp/opi-ai-frozen-lock-20261111`
  supplied 36 TypeScript and twelve Rust invocations on Windows x64. Each
  TypeScript triplet has identical stdout. All twelve shared comparisons
  pass with zero material blockers. Two cases pass and all ten observed
  differences have decisions. The raw verdict is
  `DIFFERENCES_REQUIRE_DECISION` / exit 0 with `strict_alignment: false`.
- The #53 manifest is rejected before build and replay with
  `STALE_MATERIALS` / exit 2 for the Rust library tree and example.
  The upstream tree, locks, catalog, installed and generated materials,
  toolchains, TypeScript driver and comparator match #53.
- Before the production change the double-fault regression failed with
  exit 101. The request duration case expected one backend finish attempt
  and observed zero. The receipt is
  `C:/Temp/opi-phase5-duration-recovery-red-w9sg6eof/regression.json`,
  SHA-256 `fa30eaab13acdec290e067c9a9bf9f96a48114205cf6d1b62d9d2fc73067c91b`.
- The Python differential suite passed 44 tests. The cached Rust
  `test -p opi-ai --example lifecycle_differential --test ai_telemetry --test execution_lifecycle --test telemetry_operations --offline -j4`
  passed 66 tests. Clippy on the same targets with `-D warnings`, Cargo
  format, documentation and diff checks passed.

Test impact `update` for the Rust subprocess and the double-fault and
manual completion regressions. `retain` for the #53 baseline, all twelve
paired inputs, the other fault cases and the lifecycle integration tests.

Worktree handoff `committed=none-for-this-task`, `staged=none`. The Rust
observer and example, subprocess test, product READMEs, changelog and
iteration records were carried in and updated. Earlier issue source
links were updated. This issue and its two JSON records are new untracked
paths.
