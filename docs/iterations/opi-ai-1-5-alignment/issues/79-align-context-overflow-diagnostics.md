# Align context-overflow diagnostics with Pi 1.5.0

The two deferred failure messages recorded in #78 now classify as overflow
through `message::is_context_overflow`. Both diagnostic forms cause the
frozen Durable harness to compact eligible history and submit a new job.
[Application and SDK observations](78-review-deferred-failure-classification.md)

## Source and behavior

The Rust table originated at Pi `v1.3.0` (`71023aec1546d6d677a65669c5665c0927ce0dd0`).
The frozen `v1.5.0` (`f9a26365568127461c47a07a37dcb42c140a3ac5`) added two
alternatives in `packages/ai/src/utils/overflow.ts:39–69`.

- `d3398f826ae2654b52a7988830d533a354754c13` recognizes LiteLLM's
  `Context Window exceeded` pre-call diagnostic.
- `73f8de80bc87021375f8b525292ac07492253002` recognizes request-size overflow
  from raw HTTP `413` or `<provider> API error (413):` at the start of the
  diagnostic. The raw status ends at whitespace, a colon or end of string.

Both alternatives use the existing Rust predicate and its error stop gate.
The status pattern preserves the JavaScript ASCII word class and whitespace
boundary. Throttling and rate-limit exclusions take precedence. The
Cerebras fallback covers bodyless 400 and compact `413(no body)` forms.
[Predicate](../../../../crates/opi-ai/src/types/message.rs#L781) ·
[Status controls](../../../../crates/opi-ai/tests/public_utilities.rs#L578)

The whole-call helper already consults the predicate before matching
transient diagnostics. The test for `custom-provider API error (413):
upstream returned 503` returns the failed message after one producer
invocation and zero retry callbacks. A 503 that merely mentions 413 in
its body classifies as transient, as does the rate-limit control.
[Whole-call test](../../../../crates/opi-ai/tests/whole_call_retry.rs#L464)

## Verification

- The frozen Pi `packages/ai/test/overflow.test.ts` passes all 32 tests with
  `vitest run --config vitest.config.ts --no-cache test/overflow.test.ts`.
  The predicate source and test file match the pinned Git bytes. Ten
  additional calls on the frozen predicate confirm the boundary controls,
  including JavaScript whitespace, the provider label and rate limiting.
- Before the repair, the two focused Rust targets fail four assertions
  covering the missing LiteLLM and 413 forms and the retry veto. After the
  repair, `execution_lifecycle` (40), `public_utilities` (53) and
  `whole_call_retry` (21) pass. The nine deferred observations match #78
  with `contextOverflow` set to `true` for `context-window` and
  `request-size`. Each message retains the original fixture diagnostic.
  [Deferred failure inputs](../../../../crates/opi-ai/tests/execution_lifecycle.rs#L1384)
- The seven overflow tests pass with `--no-default-features`. Affected-target
  Clippy and `cargo doc -p opi-ai --no-deps` pass with warnings denied.
- `cargo fmt --all --check`, `python -B scripts/opi-doc-check.py` and
  `git diff --check` pass.
- The old material manifest rejects the updated `crates/opi-ai/src` tree
  before building. The new freeze binds the same original lock and Pi
  snapshot with zero material blockers. All 24 shared comparisons pass and
  all 96 trace receipts validate. The 24 case records match #76 exactly.
  [Materials](../evidence/phase5-deferred-overflow-materials.json) ·
  [Replay](../evidence/phase5-deferred-overflow-differential.json)
- The #76 driver was copied before rebuilding the shared executable path.
  The copy is at
  `C:/Temp/opi-ai-lifecycle-executables/c5e13996fbd2e091969cc92b5e65607f4cf19779a06fb20188175a0c108fc4fb/lifecycle_differential.exe`.

The two pattern changes close the overflow source drift from #78. Next,
preserve legacy provider `Aborted` terminals, then compare the existing
message-level retry classification with the pinned Durable predicate.
The replay records ten deferred difference cases. Real-provider wire
behavior and six-gate acceptance remain open.

Test impact `add` for the leading 413 status regression and `update` for
the provider pattern, exclusion, deferred failure and whole-call controls.
`retain` for the usage-based and 24 paired lifecycle checks. Worktree
handoff `committed=none-for-this-task` and `staged=none`.
The lifecycle test, changelog, iteration brief and index, and replay guide
are carried-in and touched. The #79 issue, material manifest and replay are
new untracked files.
