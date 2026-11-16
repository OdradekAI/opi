# Review the deferred result-only consumer

Pi Durable can schedule another fetch and write a ready answer while the
previous source tails are still held. Rust callers can process an early
`Done` from a started fetch. Awaiting the Rust fetch helper or the started
`result()` includes the tail drain. The wait affects when a consumer can
act on the message even when the eventual messages agree.
[#74 started delivery](74-deliver-started-deferred-done-before-source-release.md) ·
[#56 paired result timing](56-observe-deferred-result-before-source-eof.md)

## The caller and the held-tail proof

In Pi `v1.5.0` at `f9a26365568127461c47a07a37dcb42c140a3ac5`, Durable's
`poll` phase awaits `runtime.models.fetchDeferred` with its task signal
and telemetry, then calls `classify`. A pending message stores the handle
and schedules a later `pollAt`. A ready message runs `afterResponse` and
can write an assistant answer or start a tool round. The result-only caller
uses the same boundary in Pi `v1.3.0` at
`71023aec1546d6d677a65669c5665c0927ce0dd0`.
`packages/durable/src/harness/generation.ts:323–338,758–799,858–871,1013–1055`
in 1.5.0 and `:236–244` in 1.3.0.

The Pi result fulfills when the outer stream receives its first terminal.
The forwarder continues through source EOF and the source's own result.
See `packages/ai/src/models.ts:950–975`,
`packages/ai/src/utils/event-stream.ts:43–87`, and
`packages/ai/src/api/lazy.ts:37–85` at 1.5.0. Opi's original AIR-043 code
at `4b3600fcdeb3ade41bf4f66249337c382630af23` released its source at the
terminal. The explicit EOF drain and poll observer came from Opi's
`1495260d1d3be40cab957b5558da57fe8494d937` and
`80e706b9e2f93b21cd185929dd0d4947d044e010`.
[#66 source history](66-review-deferred-fetch-result-and-source-cleanup.md#source-history)

The application probe runs the frozen Pi Durable Harness with `MemoryStorage`
and Faux. Faux normally ends the source with its terminal. The wrapper holds
the tail of each fetch after forwarding that terminal. The first fetch returns
`deferred` and Durable dispatches a second fetch with the same handle before
the first source reaches EOF. The second returns `stop` and Durable writes the
assistant answer while both tails are held. At the write, neither iterator has
run `finally` or exposed its source result. After release, each reaches EOF,
runs `finally` once and has its source result read once. The run records one
submission, two application-scheduled fetches and zero remote cancels. Durable
omits the provider wait override. Pi documents zero as the default in
`packages/ai/src/types.ts:274–280`. The held tail is a controlled source
condition using an actual application caller.
[Application probe](../tools/observe_deferred_result_consumer.mjs#L44) ·
[Pinned run receipt](../evidence/phase5-deferred-result-consumer.json)

On Rust, `Models::fetch_deferred` awaits the direct collector's `result()`.
The paired #74 replay keeps that future pending at the ready source EOF gate.
The new Rust test uses the same ready payload and held tail for two started
consumers. Returning from the first successful `Done` delivers the exact
message and drops the consumer while the source and poll remain open.
Awaiting `StartedChatStream::result()` is still pending there. With release,
both workers reach natural EOF, release the source and settle one poll. Each
case resolves auth and dispatches once with the original handle and zero wait.
[Direct fetch](../../../../crates/opi-ai/src/models.rs#L1143) ·
[Started result](../../../../crates/opi-ai/src/execution.rs#L149) ·
[Rust control](../../../../crates/opi-ai/tests/execution_lifecycle.rs#L1144)

## Recommendation

Use the cleanup wait when a caller needs a released source and a settled
poll. For a result-only caller like Durable, a pending tail can indefinitely
delay classification, a scheduled poll, or the answer write. A provider's
wait budget applies to its status check. A ready result and the end of local
source work are separate milestones.

The existing `fetch_deferred` and `result()` are the completion APIs for
callers that await cleanup. Map Pi's result-only use to a single-await opt-in
that gets the business outcome from the existing Tokio task. A caller can
already use that path by returning at `Done` and leaving the task to drain.
Replacing the direct fetch helper would change both its release guarantee
and the runtime and Drop behavior for custom backends. A readiness entry
lets callers choose the trade-off. A subsequent poll can overlap an earlier
source tail, as in the Pi application probe. A caller that needs serialized
source release can await completion before dispatching again.

Use public EOF as the cleanup boundary for this slice. The direct collector
owns its source. The started iterator joins on the final read, observing
release or propagating a tail panic. The result-only caller advances from
the returned message. Review an
early started EOF separately if a streaming migration needs it. The next
proof for a readiness entry should cover a `Deferred` pending message,
ready success, typed failures and cancellation. It should use the existing
worker and preserve one dispatch and explicit local tail cancellation.

## Verification

- All 35 `execution_lifecycle` tests and targeted Clippy with `-D warnings`
  pass. Test impact `add` for the consumer continuation test, `retain` for
  the existing EOF, cancellation and cleanup controls.
- The standalone Node probe passes with the original-lock snapshot. The
  driver passes `node --check`, and invoking it without `--snapshot`
  exits 2. The receipt binds the driver, original lock and relevant
  pinned source files.
- The #74 manifest, executable and 92 recorded receipts revalidate. All
  23 shared comparisons pass with zero material blockers. Ten deferred
  differences remain open. This consumer probe uses a controlled provider.
  Real-provider wire evidence and six-gate acceptance remain open.
- Cargo format, `python -B scripts/opi-doc-check.py` and `git diff --check`
  pass.

Worktree handoff `committed=none-for-this-task` and `staged=none`. The
Rust test, spec, contract index and differential README are carried in
and touched. The issue, application probe and receipt are new.
