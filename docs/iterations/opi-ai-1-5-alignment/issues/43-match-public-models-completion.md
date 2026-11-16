# Compare source completion at the public Models entry points

Status: done
Owner: Codex for user
Parent: [alignment specification](../spec.md)
Predecessor: [preserve setup failure identity](42-preserve-chat-setup-failure-identity.md)
Baseline: opi `ee18dd1323a089d536bdc7e293d0b4869777c344` with the #39–#42 worktree changes

## Frozen replay

Both completion cases now compare Pi's compiled public `Models.streamSimple`
and Rust's public `Models::stream_simple`. The controlled provider dispatch
counter is zero at call return and one at the source gate before any
consumer poll in all four executions per case. After gate release, the
source produces one terminal, reaches EOF, and the retained call returns
`stop` with `lifecycle-differential:mock-model`. The probe awaits the public
result after observing source EOF.

| Case | Consumer action | Comparison |
| --- | --- | --- |
| `models-no-consumption` | Await source EOF before reading the result | Pass |
| `models-stop-consumption` | Read the start event, return the Pi outer iterator or stop polling the Rust call, then wait for source EOF | Pass |
| `models-missing-provider` | Observe the route failure, EOF and result | Shared observations pass with an intentional error channel difference |
| `models-auth-setup-failure` | Release the gated resolver to fail, then observe the failure, EOF and result | Shared observations pass with intentional resolver timing and error channel differences |

Returning the Pi outer iterator left the source return count at zero.
The Rust replay observed source Drop after EOF. The Rust example's two
Drop tests separately verify source completion when the consumer handle
itself is released. The raw `lazyStream` cases measure the setup callback
at construction and can be run through the standalone observer.

The same original-lock candidate at `C:/Temp/opi-ai-frozen-lock-20261111`
supplied all 12 TypeScript and four Rust runs. The upstream source, installed
dependencies, locks, generated trees and Rust library source hashes match #42.
The two setup cases reproduce all #42 runtime traces. The raw replay verdict is
`DIFFERENCES_REQUIRE_DECISION` / exit 0 with zero material blockers.

## Accepted decisions

1. Retain scheduled setup for ordinary Rust Models. The explicit API key
   selects a Pi API-key resolver before the first store await, and that
   resolver has entered by call return. The current-thread Rust trace enters
   it in the scheduled task before the consumer polls. The Rust contract
   schedules work at construction and lets the runtime determine when the
   task first runs. This preserves progress without a consumer and keeps
   setup with the task that owns execution. The timing evidence applies to
   the explicit-key fixture and current-thread Tokio runtime. See the
   [lifecycle policy](../decisions/11-telemetry-lifecycle.md).
2. Retain the typed error channel for the two public setup failures. Pi
   sends an `error` event and fulfills `result()` with the error message.
   Rust returns `Err(CallFailure)` in the stream and result, carrying the
   matching message IDs and a `Config` or `AuthFailed` cause. The
   [streaming example](../../../../crates/opi-ai/examples/quick_start_stream.rs)
   propagates the cause through its Rust error branch. The agent's main turn
   and summary use prepared attempts and handle typed source errors at that
   boundary. See the [agent loop](../../../../crates/opi-agent/src/agent_loop.rs)
   and the [public contract policy](../decisions/04-public-contract.md).

The 2026-11-11 follow-up accepted both proposals for the observed resolver
path and setup faults. The contract index records both as intentional
differences. The replay record carries `strict_alignment: false`. The raw
helper, partial aliasing and observer fault paths remain open.
[Specification](../spec.md#实施前置条件与重新讨论的触发点)

## Verification

- [Material manifest](../evidence/phase5-public-models-lifecycle-materials.json), SHA-256 `84246edf567eff235512ca03cce659c3728af0ab3d921a8f32ed17dea23593d9`.
- [Replay commands and raw traces](../evidence/phase5-public-models-lifecycle-differential.json), SHA-256 `a93cfa234a338a06f1b9c43a59e510f3b1d02bdad74e2757edf20eddebcf46fe`.
- The runner rejects the #42 manifest before driver execution with exit 2.
  The reported drift is limited to the Rust example, TypeScript observer,
  comparator and runner. Rerunning the 12 recorded TypeScript commands from
  #42 reproduced their stdout byte for byte.
- The Python differential suite passed 21 tests. Cached
  `test -p opi-ai --example lifecycle_differential --offline -j4` passed
  two tests. Cached Clippy passed on the example with `-D warnings`.
  Cargo format, the documentation checker and diff checks passed.

Test impact `add` for rejecting cross-layer dispatch checkpoints and
unexpected dispatch counts or source return. `update` for the completion
probes, comparator fixtures and Drop assertions. `retain` for the setup
failure cases, raw probes and previous evidence.

Worktree handoff `committed=none-for-this-task`, `staged=none`. The modified
probe, comparator, runner, their tests, differential README, iteration spec
and contract index were carried in and updated. This issue and the two JSON
records are new untracked paths. The replay and checks ran on Windows x64.

The decision follow-up revalidated the material manifest, replay hashes and
decision links. The documentation checker and diff checks passed. Test impact
`none` for this follow-up, which reused the frozen #43 runs.
Worktree handoff `committed=none-for-this-task`, `staged=none`. The decision
documents, carried-in spec, index and differential README are unstaged.
This carried-in issue is untracked.
