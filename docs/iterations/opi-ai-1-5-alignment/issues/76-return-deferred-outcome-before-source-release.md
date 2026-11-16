# Return the deferred outcome before source release

`Models::fetch_deferred_ready` is the single-await entry for a result-only
consumer on Tokio. It returns the first normalized pending message, ready
message or typed failure. The task continues through source release and
settlement of an enabled poll observer. Callers awaiting source release
use `fetch_deferred`, the started `result()` or public stream EOF.
[Public method](../../../../crates/opi-ai/src/models.rs#L1172) ·
[#75 consumer evidence](75-review-deferred-result-consumer.md)

## Decision

Pi `v1.5.0` at `f9a26365568127461c47a07a37dcb42c140a3ac5` fulfills the
result at the first terminal in `packages/ai/src/utils/event-stream.ts:43–87`.
The forwarder in `packages/ai/src/api/lazy.ts:37–85` continues to source EOF
and reads the source result. `packages/durable/src/harness/generation.ts:323–338,758–799`
uses the fulfilled message to classify and schedule work. The frozen #75
application proof observes a second fetch and the ready answer while the
earlier tails are held. It also locates the same result-only call in Pi 1.3.0
at `71023aec1546d6d677a65669c5665c0927ce0dd0`.

The new method uses the existing deferred constructor for routing, expiry,
authentication, provider wait and the local token. The started worker polls
the collector until it normalizes the first terminal. The finisher sends the
cached outcome once before invoking the existing `ChatStream::result()` drain.
This gives a held in-band error terminal the same readiness boundary as a
successful `Done`, using the established Rust `CallFailure` representation.
The waiter releases the event queue and joins the worker if it stops before
sending an outcome. In that case the normal interruption or panic path applies.
[Collector finisher](../../../../crates/opi-ai/src/stream.rs#L584) ·
[Waiter](../../../../crates/opi-ai/src/execution.rs#L225)

The first poll starts the task. Returning from the method or dropping a
polled waiter detaches it. The caller chooses the next fetch, and can retain
the token to release a pending local tail. Remote cancellation uses the
explicit remote operation. A caller needing to observe a tail panic or a
settled poll awaits completion on the cleanup APIs. After the readiness
notification, a tail panic reaches Tokio's panic hook and a runtime shutdown
can interrupt poll settlement.

## Evidence

- `models-deferred-ready-before-source-eof` pairs Pi `fetchDeferred` with
  Rust `fetch_deferred_ready` on the original-lock snapshot. Each side
  dispatches one fetch with the same handle and zero wait. Both results are
  pending at the terminal gate. At the held EOF gate, both deliver the same
  ready payload while the source is retained and the poll is open. Source
  release and one poll settlement follow gate release.
- The original `models-deferred-result-before-source-eof` case still
  observes Rust `fetch_deferred` pending at that same checkpoint. The new
  Rust trace changes only result availability at the gate relative to that
  control. Its Pi trace matches the control apart from the scenario ID.
- All 24 shared comparisons pass. The 23 prior case records match #74
  exactly. The 96 stdout receipts, comparator outputs, executable and
  manifest hashes revalidate. Reusing #74 materials against the new inputs
  exits 2 with `STALE_MATERIALS` before building or executing a case.
  The new manifest has zero material blockers.
  [Materials](../evidence/phase5-deferred-fetch-ready-materials.json) ·
  [Replay](../evidence/phase5-deferred-fetch-ready-differential.json)
- The public Rust tests check a `Deferred` message with the original handle,
  `Stop`, and an in-band error while the tail is held. They check one auth
  resolution and one dispatch with the requested positive wait, no pending
  token or cost measurements, preterminal typed cancellation, local release
  of a held tail after readiness, and zero remote cancels. Separate controls
  drop a polled waiter before the terminal and resume a source panic before
  the readiness notification.
  [Public regression tests](../../../../crates/opi-ai/tests/execution_lifecycle.rs#L1196)

## Verification

- `cargo test -p opi-ai --test execution_lifecycle --test deferred_contract
  --test ai_telemetry --test telemetry_operations --example lifecycle_differential`
  passes 95 tests. The `--no-default-features --test execution_lifecycle`
  run passes all 39 tests. Affected-target Clippy and `cargo doc -p opi-ai
  --no-deps` pass with `-D warnings`.
- `python -B -m unittest discover -s scripts/pi-sdk-conformance/differential
  -p 'test_*.py'` passes 78 tests. `node --check` passes for the upstream
  driver. The negative comparator case rejects a result that stays pending
  at the held EOF gate on either side.
- Cargo format, `python -B scripts/opi-doc-check.py` and `git diff --check`
  pass.

## Handoff

The opt-in maps the controlled result timing. The replay remains
`DIFFERENCES_REQUIRE_DECISION` with ten open deferred differences, including
the completion control, Pi's public stream EOF and failure delivery shapes.
Next, check the pinned streaming consumers for a dependency on public EOF
while the source is still retained. Real-provider wire behavior, npm package
consumption and six-gate acceptance remain open.

Test impact `add` for the four public lifecycle tests, the paired readiness
case and its negative comparison. `update` for the controlled deferred
fixture and driver assertions. `retain` for the existing 23 replay cases
and the established result, EOF, and cancellation controls.

Worktree handoff `committed=none-for-this-task` and `staged=none`. The
production, test, driver, package-guide and iteration edits touch carried-in
paths. The issue and two evidence files are new.
