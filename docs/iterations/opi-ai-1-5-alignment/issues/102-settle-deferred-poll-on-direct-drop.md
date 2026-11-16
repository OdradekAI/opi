# #102 Settle deferred poll after direct collector Drop

After delivering a successful `Done`, a directly driven deferred collector
releases its source on Drop, then settles its enabled poll once from the
cached message. This applies before or during a pending source tail. The
collector owns both operations on this path.
[#67 original case](67-compare-deferred-stream-source-cleanup.md) ·
[#101 timing and ownership decision](101-decide-deferred-stream-result-and-drop.md)

## Implementation

`ChatStream` takes and releases the retained source before finishing the
poll. The terminal cache supplies the message. Normal EOF or `result()`
takes the observer when settling, so a later collector Drop has no second
completion to record. Handing execution to `.start()` or
`fetch_deferred_ready` leaves settlement with the worker and its join.
The Drop path applies to a cached successful deferred terminal during
ordinary destruction. A preterminal drop still releases the local source
without a success record. This path never dispatches remote cancellation.
[Collector](../../../../crates/opi-ai/src/stream.rs#L627) ·
[Public fetch contract](../../../../crates/opi-ai/src/models.rs#L1032)

## Frozen replay

The expanded Rust driver is identical before and after the repair.
It holds the terminal and source EOF at named gates, records the drop
before any further await, and snapshots the source state when the backend
`finish` method is called. The two early Drop controls use the same ready
message, handle and zero wait as #67. Each resolves auth and dispatches
once and records zero remote cancels. The attempted release of the EOF
gate confirms that its receiver is already gone.

| Direct collector path | Source at Drop | Before repair | After repair |
| --- | --- | --- | --- |
| Drop immediately after `Done` | Released before entering the EOF gate. No natural EOF. | Poll open, no backend finish, one sample. | One finish after release, `stop`, seven samples. |
| Drop after the final `next()` reached the held tail | Released with the gate still closed. No natural EOF. | Poll open, no backend finish, one sample. | One finish after release, `stop`, seven samples. |
| Drop after public EOF | Natural EOF and release precede public EOF. | One finish at EOF. | One finish at EOF, seven samples after Drop. |

The repaired early Drop polls have the same final span and measurement
records as normal EOF. They contain one first-chunk event and duration
sample, one request duration, four token samples and one cost sample.
Elapsed duration values are checked for validity and omitted from the
equality comparison. Pi still completes the source in its independent
forwarder after the public iterator stops. The raw #67 source ownership
and result timing differences remain visible.
[Rust driver](../../../../crates/opi-ai/examples/lifecycle_differential.rs#L1996) ·
[Before and after receipts](../evidence/phase5-deferred-direct-drop-poll-differential.json)

## Verification

- The #89 original-lock Pi snapshot, installation, compiled artifacts,
  catalog and toolchains match the new material manifest. The changed
  bindings are the Rust driver, comparator and `opi-ai` source tree.
  The old manifest returns `STALE_MATERIALS`, exit 2, before any build or
  driver. The original report and executable retain their hashes.
  [#89 materials](../evidence/phase5-codex-websocket-idle-timeout-materials.json) ·
  [Repaired materials](../evidence/phase5-deferred-direct-drop-poll-materials.json)
- The Rust regression fails before the repair with an open poll after
  source Drop. The expanded driver observes that gap at both Drop points.
  After repair the same driver changes only the two post-Drop checkpoints
  per early Drop control. The other 23 Rust traces and all 72 Pi receipts
  match #89. All 24 shared comparisons pass. The raw verdict is
  `DIFFERENCES_REQUIRE_DECISION`, with four passes, 20 differences and
  `strict_alignment: false`.
- The focused `opi-ai` targets pass 503 Rust tests, including 41 in
  `execution_lifecycle`. The differential suite passes 79 Python tests,
  including the subprocess test against the repaired executable and
  mutations for missing, early or duplicate finish, duplicate samples and
  remote cancel. Clippy with `-D warnings`, Rust formatting, the doc checker
  and `git diff --check` pass.
- The repaired material manifest SHA-256 is
  `b70bc6b33bf02d6ab424011ab3c6ef566e91241f0384bbab05254de6b767fea9`.
  The differential receipt SHA-256 is
  `98aa29b1cc004c5cf63bba7a483cf600a65f9e57bcc3c7c734e776f9c10c922a`.
  Both expanded driver executables are archived by SHA-256, and the
  original #89 executable verifies at its recorded path.

Test impact `add` for the Rust regression, held-tail Drop control and
comparator mutations, `update` for the differential driver and subprocess
checks, and `retain` for the original evidence and other controls.
Worktree handoff `committed=none-for-this-task` and `staged=none`.
The runtime, tests, drivers, bilingual guides, changelog and iteration
records are carried in and touched. This issue and the two new receipts
are untracked. The [#94 integration gap](94-record-real-deferred-integration-gap.md)
and the original #59 and #64 differences remain recorded.
