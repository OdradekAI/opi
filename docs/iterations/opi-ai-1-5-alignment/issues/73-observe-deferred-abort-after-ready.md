# Observe interruption after a deferred fetch delivers Done

At #73 the started execution could replace a successful deferred outcome
after the caller has already read it. The controlled sequence reads
`Done` directly, calls `.start()` on that same collector and holds the
source at the EOF gate. This exercises the interval that early started
`Done` would expose.
[#72 entry mapping](72-map-pi-eager-deferred-stream.md) ·
[#74 cached-outcome implementation](74-deliver-started-deferred-done-before-source-release.md)

## Observed outcomes

| Control with source EOF held | Source release | Public outcome | Poll |
| --- | --- | --- | --- |
| Fresh `.start()` and force abort | `abort()` joins source Drop before returning. | The source has produced `Done` and the next public read is still pending. `abort()` returns `Err(Cancelled)` with `Aborted`, and `result()` returns the same failure. | Settled as aborted. |
| Direct successful `Done`, then `.start()` and force abort | `abort()` joins source Drop before returning. | `abort()` and `result()` return `Err(Cancelled)` with `Aborted` after the caller received `Done` with `Stop`. | Settled as aborted. |
| Direct successful `Done`, then `.start()` and Tokio shutdown | Shutdown drops the task and its source. | The started iterator emits a typed `StreamError` with `Error` on a second runtime. `result()` returns that failure. | Open. |

The EOF gate remains held in each control. All three release the source
without natural EOF, perform one auth resolution and one fetch with the
original handle and zero wait, and leave the request token uncancelled.
Any invocation of the backend's remote cancel entry fails the test.
The #74 integration tests now exercise these same held-tail paths with
the committed result contract.

The #73 worker waits for its finisher before sending a terminal.
The iterator returns EOF as soon as it has a terminal cache. Moving the
send earlier therefore also requires waiting for the join before public
EOF. `result()` already waits for that join. An interrupted join has only
the produced partial available, so `abort()` caches a typed cancellation
and the ordinary interruption path synthesizes a stream failure. The
direct-`Done` handoff gives the worker the collector with its result but
leaves the consumer without the previously delivered outcome on an
interrupted join.

## Proposed result contract

Keep force abort responsible for stopping the task and waiting for
source release. Return the typed cancellation when that stop occurs
before the fetch has cached a successful `Done`. Once the SDK has
normalized and cached a successful `Done`, preserve that outcome when
cancelling its tail. This covers a terminal already read directly and a
terminal queued by the started task. The latter is an observable change
from the first control above, which reports cancellation during the
held tail even after source `Done`.

A shared cached outcome allows iteration and `result()` to agree if the
task is stopped with an unread terminal. Both `result()` and `abort()`
can discard the event queue. Define the commitment when the execution
caches the successful terminal, before publishing it, so queue access
does not determine the business outcome. Continue to surface task
panics through the existing join paths. The existing typed interruption
before a successful terminal remains applicable.

The next held-tail proof should show early `Done`, then EOF and
`result()` waiting for source release and a single poll settlement.
Verify detached cleanup, force abort before and after the cached
terminal, shutdown with a buffered or delivered terminal, and a caller
that abandons the `abort()` wait and joins through `next()` or
`result()`. Use the existing request-token behavior as a control.
The ordinary chat terminal decision continues to own ordinary timing.
[#68 local cancellation](68-drain-started-deferred-source-tail.md) ·
[#45 ordinary timing](45-observe-public-terminal-and-span-before-source-eof.md#accepted-decision)

## Verification

- `cargo test -p opi-ai --test execution_lifecycle --offline -j4`
  passed 31 tests, including both new tests. Targeted Clippy passed
  with `-D warnings`.
- The #71 material and replay hashes, original lock, bound source and
  driver inputs, executable, all 92 receipts and 23 shared comparisons
  were verified. The replay records zero material blockers and ten
  open deferred differences. The new Rust integration controls use
  the existing replay for normal, token-cancelled and detached paths.
  [Manifest](../evidence/phase5-deferred-next-wait-resume-materials.json) ·
  [Replay](../evidence/phase5-deferred-next-wait-resume-differential.json)
- Cargo format, `python -B scripts/opi-doc-check.py` and
  `git diff --check` pass.

Test impact `add` for the two interruption tests and `retain` for
the prior execution lifecycle and paired replay. Worktree handoff
`committed=none-for-this-task` and `staged=none`. The tests, spec and
index are carried in and touched. This review is a new issue. The
early delivery and cached-outcome behavior is a proposal for the
next implementation slice.
