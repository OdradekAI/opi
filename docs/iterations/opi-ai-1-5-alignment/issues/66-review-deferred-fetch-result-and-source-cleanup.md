# Review deferred fetch result timing and source cleanup

The review of #55–58 is complete. The recommendation covers the observed
public fetch cases. The timing and cancellation decisions remain open.
[Iteration](../spec.md) ·
[#65 material baseline](65-clarify-deferred-api-lookup-diagnostic.md) ·
[Lifecycle decision](../decisions/11-telemetry-lifecycle.md#answer)

## What callers observe

| Case | Frozen Pi 1.5.0 | Current Opi |
| --- | --- | --- |
| #55, two wait0 calls | At each result the poll span is open with one first-chunk duration sample. Both spans settle by the next recorded checkpoint. | At return the poll is settled. The pending call has two duration samples and the ready call also has the four token and one cost samples. |
| #56, ready `Done` with EOF held | The result is already available. The source and poll span finish after the gate is released. | The fetch stays pending. At return the source has reached EOF and been dropped, and the poll is settled. |
| #57, local cancel with a ready `Done` cached | The signal reaches the scripted backend. The already available result is ready, and the held source only finishes when the gate is released. | The cancellation wins against the pending tail. The SDK drops the source without natural EOF, settles the poll and returns the cached ready result. |
| #58, local cancel before `Done` | The cooperative fixture exits and the fetch fulfills with an `aborted` message. The forwarder reads the source result and settles the poll after delivery. | The SDK drops the pending source and settles the poll before returning `Err(CallFailure)` with cause `Cancelled` and stop reason `Aborted`. |

The compared result fields and settled poll measurements match across
each case. Both SDKs pass the same handle and wait budget to a single
fetch dispatch per call. In both cancellation cases, the remote cancel
count is zero and a separate fetch on the original handle succeeds.
The differences affect when results and poll records become visible,
whether a pending source is released on cancellation, and the error
delivery channel before a terminal is cached.
[#55 wait0](55-observe-deferred-wait0-pending-then-ready.md) ·
[#56 held EOF](56-observe-deferred-result-before-source-eof.md) ·
[#57 cancel after Done](57-observe-deferred-local-cancel-after-done.md) ·
[#58 cancel before Done](58-observe-deferred-local-cancel-before-done.md)

## Source history

Pi `v1.3.0` at `71023aec1546d6d677a65669c5665c0927ce0dd0` already
resolves the result at the first terminal push and keeps forwarding in
a separate promise chain until source EOF and source `result()`.
Pi `v1.5.0` at `f9a26365568127461c47a07a37dcb42c140a3ac5` keeps that
result boundary and runs the poll observer from the forwarder after EOF
and the source result read. The relevant files are
`packages/ai/src/utils/event-stream.ts:43–47` in both revisions,
`packages/ai/src/models.ts:924–945` and `packages/ai/src/api/lazy.ts:31–59`
in 1.3.0, and `packages/ai/src/models.ts:950–975` and
`packages/ai/src/api/lazy.ts:37–85` in 1.5.0.
Within the frozen Pi provider sources, Faux implements deferred fetch and
normally calls `end()` immediately after pushing `Done`.
`packages/ai/src/providers/faux.ts:449–456,591–655` at 1.5.0.

Opi's original AIR-043 implementation at
`4b3600fcdeb3ade41bf4f66249337c382630af23` called
`drain_to_completion` from `fetch_deferred`. Its `ChatStream` dropped the
source at the terminal and returned the cached outcome. The same
revision introduced the SDK-owned local cancel wrapper.
`crates/opi-ai/src/models.rs:907–990,2586–2590` and
`crates/opi-ai/src/stream.rs:689–704,741–755`.
The alignment iteration added the EOF drain in
`1495260d1d3be40cab957b5558da57fe8494d937`
(`crates/opi-ai/src/stream.rs:467–479`) and wired the deferred poll
observer and public fetch to the shared collector in
`80e706b9e2f93b21cd185929dd0d4947d044e010`
(`crates/opi-ai/src/models.rs:1024–1043,1125–1131`).
[#10 deferred integration](10-deferred-ai-telemetry.md)

## Assessment and next proof

Retain direct `Models::fetch_deferred` completion after source release for
the observed finite and explicitly cancelled paths. One await gives the
caller the outcome and the final poll record when the telemetry backend is
healthy. When the tail is pending, its duration counts towards the await.
When the local cancellation wins after `Done`, the cached result remains
usable and the call-owned source is dropped. Before `Done`, a typed
`Cancelled` cause lets Rust callers branch while inspecting the aborted
message. This resource boundary has the same rationale as #44's ordinary
successful result. #44's approved scope covers ordinary `stream_simple`.
The deferred decision remains scoped to #55–58 pending confirmation.
[#44 result decision](44-observe-public-result-before-source-eof.md#accepted-decision) ·
[Current fetch](../../../../crates/opi-ai/src/models.rs#L1143) ·
[Current result](../../../../crates/opi-ai/src/stream.rs#L560)

At this review, iterating the deferred `ChatStream` could yield `Done`
and then public EOF while the retained backend tail had yet to run.
Calling `result()` on that stream drove the tail. The started `ChatStream`
released the retained source at the terminal through `result_without_tail`.
#67 used the held-EOF fixture to observe both public streaming entries and
the started collector. #68 changed the deferred started collector to drain
the source in its execution task. #70 moved the direct iterator's drain
before public EOF. The matched fetch
traces establish the lifetime of the SDK-owned source. Adapter-owned tasks,
external requests and application exporter flush have their own boundaries.
[Stream iterator](../../../../crates/opi-ai/src/stream.rs#L619) ·
[Started stream](../../../../crates/opi-ai/src/stream.rs#L435) ·
[Execution owner](../../../../crates/opi-ai/src/execution.rs#L127) ·
[Transport contract](../../../../crates/opi-ai/docs/transport.md#deferred-transport-semantics) ·
[#67 public streaming replay](67-compare-deferred-stream-source-cleanup.md) ·
[#68 started drain](68-drain-started-deferred-source-tail.md) ·
[#70 direct EOF drain](70-drain-deferred-source-before-direct-eof.md)

## Verification

At this review the #65 material manifest verified against the bindings, including
the same Pi original-lock snapshot, toolchains, Rust source tree and
drivers. The driver executable hash matches. The four case records match
their original #55–58 replay records exactly.
All 12 Pi and four Rust receipts match their recorded stdout and hashes.
The sealed report has 22 shared passes, nine open deferred differences
and zero material blockers. The runtime and test inputs match the #65
checks of 71 Python and 43 focused Rust tests.
[Manifest](../evidence/phase5-deferred-lookup-diagnostic-materials.json) ·
[Replay](../evidence/phase5-deferred-lookup-diagnostic-differential.json)

Test impact `retain` for existing differential and runtime tests.
`python -B scripts/opi-doc-check.py` and `git diff --check` pass.
Worktree handoff `committed=none-for-this-task` and `staged=none`.
The guides, iteration spec and index are carried in and touched.
This review issue is new.
