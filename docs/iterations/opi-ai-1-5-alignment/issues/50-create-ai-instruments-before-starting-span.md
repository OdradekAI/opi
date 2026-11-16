# Create AI metric instruments before starting the span

Status: done
Owner: Codex for user
Parent: [alignment specification](../spec.md)
Predecessor: [request duration histogram creation failure](49-observe-request-duration-histogram-creation-failure.md)
Baseline: opi `ee18dd1323a089d536bdc7e293d0b4869777c344` with the #49 worktree inputs

## Implementation

`AiRequestTelemetry::start` creates both histograms and both counters from
the supplied context before it opens a `pi.ai.request` or `pi.ai.poll`
span. A factory error returns before the backend starts the span. This
ordering applies to the manual API and the shared automatic observer used
by ordinary Models, observed raw and prepared streams, and deferred
fetch. The manual API returns the original typed backend error. The
automatic observer reports the error through host tracing and the
operation proceeds without AI observation.
[Starter](../../../../crates/opi-ai/src/telemetry.rs#L141) · [Automatic observer](../../../../crates/opi-ai/src/telemetry.rs#L431)

On success the supplied context parents the span, and the observer's
elapsed time starts after both stages succeed. The backend's span start
follows instrument creation. The instrument error takes precedence if
both instrument creation and span start would fail.

## Frozen replay

Both sides encounter one `pi.ai.request.duration` histogram creation
failure with zero dispatches at the factory fault and public call boundary.
Pi synchronously throws the injected Error and records zero request spans
and provider dispatches through its event-loop checkpoint. Rust returns a
`ChatCall` and reports one startup WARN. The source dispatches once before
consumption, reaches natural EOF and is dropped, and the public call
delivers a successful `Done`, outer EOF and matching `Ok` result.

Rust records zero request spans at the factory fault, call return and
post-result checkpoint. Backend finish calls are zero, and no span wrapper
Drop is observed.
Compared with #49, the Rust request span snapshots and the wrapper Drop
observations are the only changes to the recorded runtime observations for
this case.
[Recovery replay](../evidence/phase5-request-duration-histogram-creation-recovery-differential.json)

## Accepted decision

On 2026-11-11 the user replied “继续推进” to the recommendation to keep
automatic observer startup fault isolation in this case. The accepted
`request-duration-histogram-creation-failure` difference covers one typed
`TelemetryError` while creating `pi.ai.request.duration` before provider
dispatch at the public Models boundary. Rust keeps the call available and
reports the telemetry fault through host WARN, so the operation can reach
the provider's own outcome. The acceptance covers public call delivery,
subsequent provider dispatch and business result, and the host diagnostic
in this case.
[Lifecycle policy](../decisions/11-telemetry-lifecycle.md#answer)

## Verification

- [Material manifest](../evidence/phase5-request-duration-histogram-creation-recovery-materials.json), SHA-256 `74e591a9aa3edcb4ed5d816acf05fc61cecf47b74eeb8bf72494495deffe0fb0`.
- [Replay commands and raw traces](../evidence/phase5-request-duration-histogram-creation-recovery-differential.json), SHA-256 `639706999b1d3183fd2fac6f70b0f21edbce21c2344ba56cf70ead4a68ab6eda`.
- The original-lock candidate at `C:/Temp/opi-ai-frozen-lock-20261111`
  supplied 27 TypeScript and nine Rust invocations on Windows x64. All
  nine shared comparisons pass with zero material blockers. All Pi
  invocations and the eight other case records match #49. Two cases pass
  and seven have documented intentional differences. The raw verdict is
  `DIFFERENCES_REQUIRE_DECISION` / exit 0 with `strict_alignment: false`.
- The #49 manifest is rejected before build and replay with
  `STALE_MATERIALS` / exit 2 for the changed Rust source tree and example.
  Its raw evidence is retained. The original upstream source, locks,
  installed and generated materials, TypeScript driver and toolchains
  match #49.
- A manual starter with poll attributes fails creation of the final
  instrument, the `pi.ai.cost` counter. The regression found one open poll span before
  the fix and zero after it. It checks the original typed error and one
  settled poll span after a successful retry.
  [Rust regression](../../../../crates/opi-ai/examples/lifecycle_differential.rs#L2602)
- The Python differential suite passed 36 tests. The cached Rust example
  and `ai_telemetry`, `execution_lifecycle` and `telemetry_operations`
  targets passed 66 tests in total. Affected-target Clippy with
  `-D warnings`, Cargo format, documentation and diff checks passed.
- The decision update reverified the frozen inputs, toolchain, drivers,
  executable identity and both evidence digests.

Test impact `add` for the manual late instrument failure and retry.
`update` for the Rust subprocess assertions requiring zero span records
at all startup fault checkpoints. `retain` for the nine replay inputs,
the #49 historical comparator checks and prior lifecycle regressions.
Test impact for the decision update is `none`.

Worktree handoff `committed=none-for-this-task`, `staged=none`. The AI
starter, example, subprocess test, product READMEs, changelog and iteration
records were carried in and updated. This issue and its two JSON records
are new untracked paths.
