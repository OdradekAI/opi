# Deliver started deferred Done before source release

`Models::stream_deferred(...).start()` can now deliver a successful `Done`
while the source tail is pending. Its existing execution task continues
through `ChatStream::result()`. Public EOF and `result()` still wait for
source release and poll settlement. Consumer Drop leaves the task driving
the same source.
[#72 entry mapping](72-map-pi-eager-deferred-stream.md) ·
[#73 interruption baseline](73-observe-deferred-abort-after-ready.md) ·
[Transport contract](../../../../crates/opi-ai/docs/transport.md#deferred-transport-semantics)

## Outcome and cleanup

The task caches the first normalized successful `Done` before putting it
in the event queue. `result()` and `abort()` can discard the queue, so the
cache must survive separately through the join. Starting after a direct
read of `Done` seeds the same cache and takes over only the tail. The
iterator joins the task before returning EOF after the early `Done`.
[Worker](../../../../crates/opi-ai/src/execution.rs#L105) ·
[Collector handoff](../../../../crates/opi-ai/src/stream.rs#L435) ·
[Join](../../../../crates/opi-ai/src/execution.rs#L193)

| Interruption while the source is pending | Before successful `Done` is cached | After successful `Done` is cached |
| --- | --- | --- |
| Force abort | Join source release and return typed `Cancelled` with `Aborted`. | Join source release and return the cached message, whether buffered, read from the started stream, or read directly before `.start()`. |
| Tokio runtime shutdown | The joined task reports typed `StreamError`. In the tested path the poll stays open. | After shutdown releases the source, a retained handle can join and recover the same message. Iteration delivers any unread `Done` once and then EOF. |

Joining an interrupted tail after successful `Done` settles the existing
poll with that result. If the caller abandons an already-polled `abort()`
wait, `next()` and `result()` can still join and settle the observer. A
source panic still propagates through the join. Task cancellation releases
the call-owned source at its current position. After runtime shutdown,
settlement from the cached outcome requires a retained handle to join.
The controls resolve auth and dispatch once with the original handle and
zero wait. Force abort leaves the request token and remote job alone.
The request-token controls still yield typed cancellation before `Done`
and the cached result after it. The ordinary timing decision covers chat.
[Force abort](../../../../crates/opi-ai/tests/execution_lifecycle.rs#L921) ·
[Resumed join](../../../../crates/opi-ai/tests/execution_lifecycle.rs#L957) ·
[Runtime shutdown](../../../../crates/opi-ai/tests/execution_lifecycle.rs#L1015) ·
[#45 ordinary timing](45-observe-public-terminal-and-span-before-source-eof.md#accepted-decision)

## Paired replay

The upstream material remains Pi `v1.5.0` at
`f9a26365568127461c47a07a37dcb42c140a3ac5` with the original
`package-lock.json` blob `a0749b93b38ead1c75351751883ed95dec5cb32f`.
`packages/ai/src/utils/event-stream.ts:43–87` fulfills the result and marks
the outer iterator done at the first terminal. The forwarder continues
through source EOF and result in `packages/ai/src/api/lazy.ts:37–85`.
Pi `v1.3.0` already splits delivery and cleanup in those paths. Opi's
previous late started `Done` came from awaiting its own task finisher.
The new delivery point matches business readiness and preserves the
source-release guarantee for callers awaiting completion.
[#72 source revisions](72-map-pi-eager-deferred-stream.md#source-revisions-and-rationale)

The comparison pairs the Pi eager entry with the Rust started entry.
Both deliver the same ready `Done` at the held EOF gate with the poll
open. Pi can also expose public EOF and a fulfilled result while the
source is still pending. Rust's next read stays pending at that gate.
After release the Rust worker reaches natural EOF, drops the source,
settles the poll and returns public EOF and the matching result. The
detached consumer and direct-to-start handoff retain their cleanup proofs.
[#56 result timing](56-observe-deferred-result-before-source-eof.md) ·
[Rust checkpoint](../../../../crates/opi-ai/examples/lifecycle_differential.rs#L1825) ·
[Comparator](../../../../scripts/pi-sdk-conformance/differential/compare_lifecycle.py#L1211)

The original-lock replay passes all 23 shared comparisons with zero
material blockers. All 22 other cases, the target's three Pi receipts,
and 44 of its 46 Rust checkpoints match #71. The other two checkpoints
record early started `Done` and the following pending `next()`.
[Manifest](../evidence/phase5-deferred-started-done-materials.json) ·
[Replay](../evidence/phase5-deferred-started-done-differential.json)

## Verification and remaining work

- The updated held-tail driver failed on the old worker with
  `started Done is available while source EOF is held`. The new replay
  reaches that assertion and the subsequent EOF and result checks.
- The 90 focused Rust tests in `execution_lifecycle`, `ai_telemetry`,
  `deferred_contract`, `telemetry_operations` and the lifecycle example pass.
  All 34 `execution_lifecycle` tests also pass with `--no-default-features`.
  Affected-target Clippy and `cargo doc -p opi-ai --no-deps --offline`
  pass with warnings denied.
- The 77 differential Python tests pass, including the real Rust subprocess.
  The runner rejects the #71 manifest with `STALE_MATERIALS` before any
  case runs. All 92 new receipts and the bound executable match their hashes.
- `cargo fmt --all --check`, `python -B scripts/opi-doc-check.py` and
  `git diff --check` pass.

Ten deferred comparisons remain open. EOF and `result()` still include
tail latency for a caller who awaits completion. Next compare Pi's
result-only consumer with Rust's release guarantee when the tail stays
pending, then decide the exact EOF and result timing contract. Real-provider
wire evidence and six-gate acceptance remain open.

Test impact `add` for resumed abort, preterminal shutdown and tail panic,
`update` for the two #73 interruption tests and the started replay,
and `retain` for the token, detached cleanup and ordinary controls.
Worktree handoff `committed=none-for-this-task` and `staged=none`. The
execution owner is task-only tracked work. Other modified paths were
carried in and touched. This issue and the two sealed evidence files are new.
