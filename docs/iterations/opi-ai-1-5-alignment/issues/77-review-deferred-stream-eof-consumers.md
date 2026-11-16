# Review callers of deferred stream EOF

Keep public EOF as the cleanup boundary of an Opi started deferred stream.
The pinned Pi callers either await a result or iterate a test fixture that
ends its source immediately. The controlled replay establishes an EOF timing
difference after a successful `Done`. The two SDKs deliver the same ready
payload from one fetch in that comparison.
[#74 delivery](74-deliver-started-deferred-done-before-source-release.md) ·
[#76 result-only mapping](76-return-deferred-outcome-before-source-release.md)

## Pinned callers

The audit uses Pi `v1.5.0` at `f9a26365568127461c47a07a37dcb42c140a3ac5`
and the comparison revision `v1.3.0` at `71023aec1546d6d677a65669c5665c0927ce0dd0`.
Run the following from a local Pi repository with the two revisions.

```text
git grep -n -I -e streamDeferred -e fetchDeferred f9a26365568127461c47a07a37dcb42c140a3ac5 --
git grep -n -I -e streamDeferred -e fetchDeferred 71023aec1546d6d677a65669c5665c0927ce0dd0 --
```

The `streamDeferred` search has 13 matching lines in six files at 1.5.0
and six lines in three files at 1.3.0. The audit follows both public entry
points, the identity wrapper, provider delegation, in-tree tests and the
application call to `fetchDeferred`.

| Pi 1.5.0 location | Continuation boundary |
| --- | --- |
| `packages/ai/src/models.ts:338–348,950–975` | Public stream entry and the `.result()` helper. The helper awaits the result without reading the iterator. |
| `packages/ai/src/utils/identity.ts:149–181` | Adds identity options and delegates the two entries. |
| `packages/coding-agent/src/core/model-runtime.ts:763–784` and `packages/coding-agent/src/core/provider-composer.ts:651–657` | Prepares and delegates the fetch. `ModelRuntime.fetchDeferred` uses `.result()`. |
| `packages/coding-agent/test/model-runtime-images.test.ts:245–283` and `packages/coding-agent/test/model-runtime-modify-models-compat.test.ts:181–230` | Checks result-based rejection of image models before dispatch, and request option forwarding for a native deferred provider. |
| `packages/ai/test/telemetry.test.ts:71–115` | The deferred row collects all events with `for await`, checks the events and `result()`, then uses `vi.waitFor` for span settlement. The fixture pushes its events and calls `end(result)` in the same function. |
| `packages/ai/README.md:1118–1134` | Documents the `pi.ai.poll` observer. The deferred entry is mentioned in the telemetry contract. |
| `packages/durable/src/harness/generation.ts:323–338,758–799` | The shipped application awaits `fetchDeferred`, then classifies the message and schedules another poll, writes an answer or starts tools. [#75 observed continuation](75-review-deferred-result-consumer.md) |

The telemetry case is the only pinned in-tree use found that iterates
`streamDeferred` to EOF. Its assertions depend on full event delivery and
eventual telemetry settlement. In 1.3.0, the Models and coding-agent helpers
already use `.result()` (`packages/ai/src/models.ts:924–945` and
`packages/coding-agent/src/core/model-runtime.ts:755–776`). Durable uses the
same result-only continuation (`packages/durable/src/harness/generation.ts:236–244`).
These findings cover the pinned Git trees. Third-party consumers remain
outside the audit.

## Timing and ownership

At both Pi revisions, `packages/ai/src/utils/event-stream.ts:43–87` marks
the outer stream done and fulfills the result at the first terminal. The
forwarder continues through source EOF and reads the source result in
`packages/ai/src/api/lazy.ts:31–39` at 1.3.0 and `:37–52` at 1.5.0. The
1.5.0 forwarder also invokes the poll observer on completion. The EOF
timing follows from the outer stream's terminal behavior in both versions.
Opi's source drain and poll observer came from
`1495260d1d3be40cab957b5558da57fe8494d937` and
`80e706b9e2f93b21cd185929dd0d4947d044e010`. The #74 change subsequently
advanced the successful `Done` delivery of the started worker while
keeping its join on the last read.
[#66 source history](66-review-deferred-fetch-result-and-source-cleanup.md#source-history) ·
[Rust EOF join](../../../../crates/opi-ai/src/execution.rs#L271)

The `models-deferred-stream-source-cleanup` replay holds source EOF after
`Done`. The two entries dispatch once with the same handle and zero wait.

| Checkpoint | Pi `Models.streamDeferred` | Rust `Models::stream_deferred(...).start()` |
| --- | --- | --- |
| At `Done` | Ready message delivered. Source and poll are open. | Same ready message. Source and poll are open. |
| Last read with source EOF held | Public EOF. Source `finally` and result reads are both zero. Poll is open. | Pending. Source remains owned by the task and the poll is open. |
| After gate release | Source EOF, one `finally`, one source result read and one poll settlement. | Source EOF and Drop, one poll settlement, then public EOF and the same result. |

Rust code can continue inside the loop at `Done` and let the task drain,
including after dropping the consumer. When a caller waits for public EOF
or `result()`, the join also observes a tail panic. This gives streaming
callers a successful terminal for business work and a final read that can
confirm resource release. `fetch_deferred_ready` provides the single-await
boundary for the result-only caller.
[Replay](../evidence/phase5-deferred-fetch-ready-differential.json) ·
[Tail panic test](../../../../crates/opi-ai/tests/execution_lifecycle.rs#L1111) ·
[Transport contract](../../../../crates/opi-ai/docs/transport.md#deferred-transport-semantics)

## Decision and verification

The existing Rust entries cover the audited consumers. Retain the
source-release boundary for public EOF after a successful started `Done`
as a deliberate timing difference. The full `deferred-stream-source-cleanup`
case stays in the replay and includes direct and detached consumer behavior.
The replay retains ten deferred difference cases and reports
`DIFFERENCES_REQUIRE_DECISION`.

- The frozen `packages/ai/test/telemetry.test.ts` row `observes streamDeferred
  without changing delivered events or results` passes on the original-lock
  installation. From `packages/ai`, the command is `node --conditions=source
  ../../node_modules/vitest/vitest.mjs run test/telemetry.test.ts -t "observes
  streamDeferred without changing delivered events or results" --configLoader
  runner --no-cache --maxWorkers 1`. The controlled driver environment
  supplies only the system path, temp and time zone variables. One test
  passes and 21 unselected tests are skipped.
- Twelve inspected Pi files match the pinned Git bytes after the test.
  The #76 manifest and replay still bind the original lock. All 24 shared
  comparisons, 96 stdout receipts and the Rust executable hash revalidate.
  The held-EOF case passes its payload, dispatch and cleanup assertions.
  [Bound materials](../evidence/phase5-deferred-fetch-ready-materials.json)
- `python -B scripts/opi-doc-check.py` and `git diff --check` pass.
- Test impact `retain`. Reused the existing regression and replay evidence.

Next, trace how Durable classifies deferred error and abort results. Verify
whether a migration receiving a typed `CallFailure` can keep the same retry,
compaction and cancellation behavior.

Worktree handoff `committed=none-for-this-task` and `staged=none`. The spec,
contract index, lifecycle decision and bilingual transport guides are
carried in and touched. This issue is new.
