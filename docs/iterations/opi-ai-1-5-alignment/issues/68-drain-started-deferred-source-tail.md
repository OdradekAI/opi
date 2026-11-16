# Drain the source tail of a started deferred fetch

At #68, `Models::stream_deferred(...).start()` began driving the retained source
through `ChatStream::result()` in the existing execution task. A normal completion
releases the source and settles the enabled poll observer before delivering
the terminal or started result. The task continues draining after consumer
Drop.
[Iteration](../spec.md) ·
[#67 baseline](67-compare-deferred-stream-source-cleanup.md) ·
[#74 early Done contract](74-deliver-started-deferred-done-before-source-release.md) ·
[Execution ownership decision](../decisions/11-telemetry-lifecycle.md#answer)

## Callers and implementation

At Opi HEAD `ee18dd1323a089d536bdc7e293d0b4869777c344` plus the #67 worktree,
the only in-repository caller that started a deferred collector was the
lifecycle probe. `Models::fetch_deferred` drives the same collector directly
through `result()`. The telemetry tests also consume the direct streaming
entry. The agent's main loop and preparation summary start a raw collector
over a prepared attempt. Ordinary chat uses `ChatCallDriver` with its own
finalizer.
[Deferred fetch](../../../../crates/opi-ai/src/models.rs#L1143) ·
[Agent loop](../../../../crates/opi-agent/src/agent_loop.rs#L632) ·
[Preparation summary](../../../../crates/opi-agent/src/agent_loop.rs#L1779) ·
[Ordinary execution](../../../../crates/opi-ai/src/models.rs#L5059)

The #67 deferred control inherited the raw collector's release at the business
terminal. Its backend source dropped before reaching the held EOF gate. The
existing `result()` drain can drive finite backend tail work to completion.
Select that finisher when Models creates a deferred collector. The existing
worker already waits for the finisher before forwarding the terminal and
retains execution after the consumer drops. If the caller read `Done` before
`start()`, the worker goes directly to finalization with the cached terminal,
so its internal event polling cannot settle the observer early or replay
the terminal.
[Deferred constructor](../../../../crates/opi-ai/src/models.rs#L1037) ·
[Collector start](../../../../crates/opi-ai/src/stream.rs#L435) ·
[Tail drain](../../../../crates/opi-ai/src/stream.rs#L560) ·
[Worker finalization](../../../../crates/opi-ai/src/execution.rs#L127)

A pending tail now adds to the started call's completion latency.
Starting a collector with a retained tail requires a Tokio runtime.
`DeferredFetchCall::cancel` before the terminal yields typed `Cancelled`.
Cancelling during a cached successful terminal's tail releases the source
and returns that terminal's result. Explicit `StartedChatStream::abort()`
follows the execution task's force-abort contract while cleanup is pending.
The transport guides describe the public boundaries.
[English](../../../../crates/opi-ai/docs/transport.md#deferred-transport-semantics) ·
[中文](../../../../crates/opi-ai/docs/transport.zh.md#deferred-transport-语义)

## Replay

| Rust control | With source EOF held | After gate release |
| --- | --- | --- |
| Fresh `.start()` | The worker has cached the terminal. Public `next()` is pending and the poll is open with one first-chunk sample. | Natural EOF and source Drop precede public `Done`, EOF and the matching ready result. The poll is settled with seven samples. |
| Drop the unpolled started consumer while the terminal gate is held | After consumer Drop, the worker emits the source terminal and reaches the held EOF gate with the poll open. | The detached worker reaches natural EOF, releases the source and settles the poll. |
| Read direct `Done`, then `.start()` | The worker drives the retained tail and the poll remains open. | After natural EOF, source Drop and poll settlement, iteration yields EOF and `result()` returns the original ready result. |

Each Rust control resolves auth and dispatches once with the original handle
and zero wait. All remote cancel counts are zero. The ready result and
settled poll fields match direct fetch. Frozen Pi `v1.5.0` at
`f9a26365568127461c47a07a37dcb42c140a3ac5` continues to deliver the public
terminal and EOF before source completion in its independent forwarder.
The Pi trace and five direct Rust checkpoints match #67, with the Rust
probe now also recording the auth count. Public delivery timing and the
direct iterator's poll and source lifetimes remained open at this replay.
[Pi source comparison](67-compare-deferred-stream-source-cleanup.md#source-and-assessment) ·
[Rust driver](../../../../crates/opi-ai/examples/lifecycle_differential.rs#L1750) ·
[Comparator](../../../../scripts/pi-sdk-conformance/differential/compare_lifecycle.py#L1211) ·
[#70 direct EOF drain](70-drain-deferred-source-before-direct-eof.md)

## Verification

- [Original-lock manifest](../evidence/phase5-deferred-started-tail-materials.json)
  `21d924a7dc4e8ac4832f50bdfd050d77acb70bec628c05d82c880d4b64ff2ef5`.
  [Paired replay](../evidence/phase5-deferred-started-tail-differential.json)
  `673f3049f96f85309af4bdcf411d1d1799372cc4f3c259d17faaec990e7dd74c`.
- Before the runtime change, the updated probe failed with exit 101 when
  the old started path dropped the source before the EOF gate. The #67
  manifest now returns `STALE_MATERIALS` with exit 2 before either driver
  or build runs. The Rust source tree, Rust driver and comparator are
  the changed bindings.
- All 69 Pi and 23 Rust receipts match their recorded stdout, decoded
  traces and SHA-256. The 22 other case records match #67 exactly.
  All 23 shared comparisons pass. Three pass outright, ten have
  previously accepted scoped differences and ten deferred cases still
  have open differences. The raw verdict is `DIFFERENCES_REQUIRE_DECISION`
  with exit 0, zero material blockers and `strict_alignment: false`.
- The lifecycle differential suite passed 73 Python tests. The
  `ai_telemetry`, `deferred_contract`, `execution_lifecycle`, `stream_events`,
  `telemetry_operations` and `lifecycle_differential` Rust targets passed
  114 tests, including started local cancellation before and after `Done`.
  Clippy on those targets and `cargo doc -p opi-ai --no-deps` passed with
  warnings denied. Format, documentation and diff checks passed.

Test impact `add` for the detached and cached-terminal probe controls,
started cancellation and comparator rejection cases. `update` for the
started probe and paired assertions. `retain` for the other 22 cases
and existing SDK regressions.

Worktree handoff `committed=none-for-this-task` and `staged=none`.
`HEAD` is `ee18dd1323a089d536bdc7e293d0b4869777c344`. The two library
files, probe, comparator and tests, guides and iteration records were
carried in and touched. Related source links were refreshed. This issue
and its two evidence files are new.
