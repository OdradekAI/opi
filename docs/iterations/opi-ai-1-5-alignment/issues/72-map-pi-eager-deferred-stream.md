# Map Pi's eager deferred stream to the Rust started entry

Use `Models::stream_deferred(...).start()` in a Tokio runtime when
migrating Pi's `Models.streamDeferred`. Both paths can drive a single
fetch before the first event read and continue when the caller stops
consuming. The call to `.start()` gives the Tokio task ownership of
execution and source release.
[Lifecycle decision](../decisions/11-telemetry-lifecycle.md#answer) ·
[English guide](../../../../crates/opi-ai/docs/transport.md#deferred-transport-semantics) ·
[中文指南](../../../../crates/opi-ai/docs/transport.zh.md#deferred-transport-语义)

## Startup, delivery and cleanup

The timing below records #71. #74 moves started successful `Done` before
source release and retains the cleanup boundary at EOF and `result()`.
[#74 implementation](74-deliver-started-deferred-done-before-source-release.md)

| Entry | Startup | Ready terminal with source EOF held | Caller stops consuming |
| --- | --- | --- | --- |
| Pi `Models.streamDeferred` | The call enters setup and attaches a promise chain to forward the source. | Public `Done`, EOF and the fulfilled result are available. The poll is open. | Returning the outer iterator leaves the forwarder running. At source EOF it observes `finally`, reads the source result and settles the poll. |
| Rust direct `Models::stream_deferred` | The caller drives auth and fetch by polling the collector. | `Done` is available. The final `next()` or `result()` drives the tail and waits for release. | Dropping the whole collector at `Done` releases the held source without natural EOF and leaves the controlled poll open. A retained collector can still drive the tail. |
| Rust `Models::stream_deferred(...).start()` | The Tokio task can reach the source before the caller reads an event. | A new `Done`, public EOF and result wait for source release. The poll is open. | Dropping the consumer leaves the task driving the source. With the gate released it reaches EOF, drops the source and settles the poll. |

In #71 the Rust task reaches the source's terminal gate before the first
public read. In the detached control it continues to the EOF gate after
consumer Drop. A separate direct control drops the source at `Done`
and observes the still-open poll. Both SDKs dispatch the same handle
with zero wait in these controls and agree on the normalized ready fields
and settled telemetry on normal completion. The stop controls each resolve
auth and dispatch once with zero remote cancels. These observations use
controlled backends.
[Pi held-tail driver](../tools/observe_upstream.mjs#L912) ·
[Rust started control](../../../../crates/opi-ai/examples/lifecycle_differential.rs#L1825) ·
[#56 result timing](56-observe-deferred-result-before-source-eof.md) ·
[#71 replay](71-resume-deferred-tail-after-pending-next.md)

## Source revisions and rationale

Pi `v1.5.0` at `f9a26365568127461c47a07a37dcb42c140a3ac5` invokes
`setup()` during `streamDeferred` construction. The forwarder pushes
the business terminal into the outer stream, then continues through
source EOF and its result. The outer stream marks itself done and
fulfills its result on that first terminal. Pi `v1.3.0` at
`71023aec1546d6d677a65669c5665c0927ce0dd0` already has this startup,
delivery and ownership split. The 1.5.0 forwarder adds observation of
the existing path.
`packages/ai/src/models.ts:950–975` and
`packages/ai/src/api/lazy.ts:37–85` in 1.5.0,
`packages/ai/src/models.ts:924–945` and
`packages/ai/src/api/lazy.ts:31–59` in 1.3.0, and
`packages/ai/src/utils/event-stream.ts:43–87` in both.

Opi AIR-043 at `4b3600fcdeb3ade41bf4f66249337c382630af23` drove
deferred setup from the caller and dropped the source at the business
terminal. #68 let the existing started worker drain the source with
`result()`. #70 moved the direct iterator's public EOF after that
drain. The late started `Done` follows from the execution task
awaiting its finisher before sending the terminal. These changes
give callers a source release boundary and reuse the worker that
owns execution.
[Source history](66-review-deferred-fetch-result-and-source-cleanup.md#source-history) ·
[Started drain](68-drain-started-deferred-source-tail.md) ·
[Direct EOF drain](70-drain-deferred-source-before-direct-eof.md) ·
[Rust finisher](../../../../crates/opi-ai/src/execution.rs#L127)

Use the started entry where the fetch must outlive its consumer.
Keep direct iteration caller-driven for callers who own polling and
can explicitly hand over a retained stream with `.start()`. Keep
Rust's public EOF and `result()` tied to release of the call-owned
source. The caller can then await completion with the poll
settled when recording succeeds. A pending tail adds to the wait.

At #72 the started entry also delays presentation of a ready message
when the source tail is held. Evaluate delivering its `Done` at
business readiness, while public EOF and `result()` continue to wait
for cleanup. This helps a consumer that reads the ready message from
the stream. A Rust caller using the result-only entry still awaits
cleanup. Pi's durable poll uses the result-only pattern by awaiting
`fetchDeferred` before classifying the message
(`packages/durable/src/harness/generation.ts:320–338` in 1.5.0).

A controlled proof should check one terminal, a retained result,
continued cleanup after consumer Drop, and cancellation before and
after the terminal. Explicit `abort()` and task interruption also
need to preserve the consistency of the delivered outcome. The
ordinary successful chat timing decision has its own scope. The
deferred timing comparisons remain open.
[#45 ordinary chat timing](45-observe-public-terminal-and-span-before-source-eof.md#accepted-decision)

## Verification

- Revalidated the #71 material bindings, including the original Pi
  lock, toolchains, source trees, driver hashes and Rust executable
  hash. All 92 receipts match their decoded traces and stdout hashes.
  The comparator reproduces all 23 recorded shared passes with zero
  material blockers. Ten deferred differences remain open.
  [Manifest](../evidence/phase5-deferred-next-wait-resume-materials.json) ·
  [Replay](../evidence/phase5-deferred-next-wait-resume-differential.json)
- Ran the existing Pi `lazy-no-consumption` probe on the same snapshot.
  Setup is entered once during construction. The source reaches its
  gate before any outer read and returns the original result after
  release. The public deferred dispatch path is traced above.
- `python -B scripts/opi-doc-check.py` and `git diff --check` pass.

Test impact `retain` for the #71 lifecycle and SDK checks. Worktree
handoff `committed=none-for-this-task`, `staged=none`. The bilingual
guides, spec and contract index are carried in and touched. This
mapping review is a new issue. Real-provider wire behavior and
six-gate acceptance remain open.
