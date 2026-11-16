# #101 Decide deferred stream result timing and direct collector Drop

Retain the cleanup guarantee of Opi's started `result()` and the source
ownership of its direct collector. Use the existing readiness entry for
Pi result-only callers and the started entry for independent tail work.
The open poll after direct Drop with a cached successful `Done` is a
telemetry gap to fix at source release.
[#67 original case](67-compare-deferred-stream-source-cleanup.md) ·
[Frozen replay and caller receipts](../evidence/phase5-deferred-stream-source-cleanup-replay.json) ·
[#102 repair and before/after replay](102-settle-deferred-poll-on-direct-drop.md)

## Held-tail observations

The three Pi runs and one Rust run reproduce the complete #89 stream
cleanup case. Each stop control resolves auth and dispatches one fetch
with the original handle and zero wait. Neither sends a remote cancel.
The successful `Done` carries the same projected ready message.

| Entry or action | While the source EOF gate is held | At local source completion |
| --- | --- | --- |
| Pi `streamDeferred(...).result()` | The result is available at `Done`. The source and its `pi.ai.poll` remain open. | Gate release yields source EOF, one `finally`, one source result read and a settled poll. |
| Opi `stream_deferred(...).start().result()` | The worker has reached the held tail and can deliver `Done` to an iterator. The result future and poll are pending. | The worker reaches source EOF, releases it and settles the poll. `result()` returns the same ready outcome. The join also observes tail panics. |
| Pi stops the public iterator at `Done` | `iterator.return()` ends public iteration while the forwarder stays in the held tail. | On release, the source reaches EOF and the poll settles with seven samples. |
| Opi drops the direct collector at `Done` | The SDK source drops immediately, before entering the EOF gate. Sending the gate release fails. | The poll remains open with one first-chunk duration sample and no final stop, duration, token or cost record. |
| Opi drops the started consumer | The worker continues driving the source to the held tail. | Gate release gives natural EOF, source Drop and a settled poll with seven samples. |

The original #67 probe awaits `result()` after releasing EOF. The
additional Pi result observation comes from the frozen held-tail
`fetchDeferred` control. The public helper delegates to
`streamDeferred(...).result()` at the pinned revision. The Rust
`execution_lifecycle` control polls the started `result()` itself while
the gate is closed. It checks a pending future, unreleased source and
open poll, then checks the outcome, source release and settled poll after
release. The frozen readiness control returns the same projected message
as Pi at the held gate and settles the poll after source release.
[Pi held-tail control](../tools/observe_upstream.mjs#L840) ·
[Pi stop control](../tools/observe_upstream.mjs#L912) ·
[Rust result control](../../../../crates/opi-ai/tests/execution_lifecycle.rs#L1144) ·
[Rust direct Drop](../../../../crates/opi-ai/examples/lifecycle_differential.rs#L1958)

## Callers and disposition

Pi `v1.5.0` at `f9a26365568127461c47a07a37dcb42c140a3ac5` fulfills
the public result in `EventStream.push` at the first terminal. The
forwarder keeps reading the source and observes the poll result after
EOF and source result retrieval. See `packages/ai/src/models.ts:950–975`,
`packages/ai/src/utils/event-stream.ts:43–87` and
`packages/ai/src/api/lazy.ts:37–53`. Pi `v1.3.0` at
`71023aec1546d6d677a65669c5665c0927ce0dd0` already has the independent
forwarder and the terminal-based result (`packages/ai/src/models.ts:924–945`,
`packages/ai/src/api/lazy.ts:31–59`).

Durable's generation poll awaits `fetchDeferred`, then classifies the
message and schedules another poll or writes the answer
(`packages/durable/src/harness/generation.ts:323–338,758–799,858–871`).
The pinned Harness replay schedules a second fetch with the same handle
and commits the answer while both source tails remain held. It completes
both tails after release. Coding-agent's `ModelRuntime.fetchDeferred`
also delegates to `streamDeferred(...).result()`
(`packages/coding-agent/src/core/model-runtime.ts:763–784`). The in-tree
Pi streaming telemetry test reads through public EOF, checks the result,
then separately waits for span settlement (`packages/ai/test/telemetry.test.ts:71–115`).
Opi's deferred callers in the checked tree are SDK entries, tests and
probes. The runtime verification uses controlled sources.
[#75 Durable continuation](75-review-deferred-result-consumer.md#the-caller-and-the-held-tail-proof) ·
[#77 caller inventory](77-review-deferred-stream-eof-consumers.md#pinned-callers)

Keep the wait in `.start().result()` as an intentional timing difference.
The original Opi started path ended the source at `Done` and skipped its
tail. #68 gave the worker the retained-source drain, and #74 then moved
successful `Done` delivery back to the business terminal. Waiting for
`result()` now lets a caller observe local resource release, poll
settlement when recording succeeds, and a tail panic. For the readiness
behavior used by Durable, `fetch_deferred_ready` supplies the same
single-await boundary. A streaming caller can act on `Done` while the
started worker continues draining. Keep the existing #77 decision on
started public EOF and #70's direct public EOF after source release.
[#68 worker drain](68-drain-started-deferred-source-tail.md) ·
[#74 early Done](74-deliver-started-deferred-done-before-source-release.md) ·
[Started result join](../../../../crates/opi-ai/src/execution.rs#L149) ·
[Readiness entry](../../../../crates/opi-ai/src/models.rs#L1172)

Keep the direct collector's immediate source release as a caller-owned
operation. `.start()` transfers ownership to the worker, including after
a direct read of `Done`, so the caller can stop consuming while the same
fetch finishes. The frozen cancellation control also releases a pending
direct tail locally and settles the poll from the cached result. This
proves that natural EOF is unnecessary once the SDK has released the
source and has a definitive business outcome.
[Direct EOF and cancel controls](70-drain-deferred-source-before-direct-eof.md#replay) ·
[Direct and started ownership](../../../../crates/opi-ai/docs/transport.md#deferred-transport-semantics)

Align poll completion after direct Drop at a cached successful `Done`.
The current path inherited explicit settlement on `next()` or `result()`
and loses the observer when the collector goes away. At that point the
local attempt has a known result and has released its source. The
open poll omits request duration, token and cost samples for that
completed provider response. This is a telemetry gap even for a caller
that already has the message.

The follow-up should release the direct source and settle its enabled
poll once from the cached successful message. Check the stop, duration
and usage samples, one fetch, and zero remote cancels. Keep the
started worker as the owner of its tail and settlement, and use the
existing explicit telemetry finish contract. Cover both `Done` followed
by immediate Drop and Drop after polling the tail. Normal completion
must still settle once. Before a terminal is observed, an abandoned
source has no known success to record. The #89 receipts preserve the
current gap for a before-and-after comparison.
[Collector release and finish](../../../../crates/opi-ai/src/stream.rs#L560) ·
[Explicit span owner](../../../../crates/opi-telemetry/src/context.rs#L88)

## Verification

- The [#89 manifest](../evidence/phase5-codex-websocket-idle-timeout-materials.json)
  and bound executable verified before and after replay. Four #67 receipts
  and four held-tail control receipts match the frozen records. The
  original #67 comparison and #89 raw verdict remain recorded. All 24
  shared comparisons pass, with four raw passes, 20 raw differences and
  `strict_alignment: false`.
- The Durable result consumer reproduces its prior receipt on the same
  snapshot. Six relevant caller and SDK source files match the pinned Pi
  Git revision. The evidence SHA-256 is
  `d1ba8398507b8364366f76bbbd27a683e3dd359180f909e059b987366e029321`.
- Cached `test -p opi-ai --test execution_lifecycle --locked --offline -j4`
  passes 40 tests, including held-tail result, `Done` and tail panic.
  `python -B scripts/opi-doc-check.py` and `git diff --check` pass.

Test impact `retain` for the frozen controls and the existing
`execution_lifecycle` tests. Worktree handoff
`committed=none-for-this-task` and `staged=none`. The carried-in spec,
index, lifecycle decision and differential guide are touched. This issue
and replay receipt are new. The [#94 real-provider blocker](94-record-real-deferred-integration-gap.md)
and the original #59 and #64 differences remain recorded.
