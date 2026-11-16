# Observe the public Models result before source EOF

Status: done
Owner: Codex for user
Parent: [alignment specification](../spec.md)
Predecessor: [matched public Models completion](43-match-public-models-completion.md)
Baseline: opi `ee18dd1323a089d536bdc7e293d0b4869777c344` with the #39–#43 worktree changes

## Frozen replay

`models-result-before-source-eof` runs one successful controlled provider
call on each public Models entry point. The driver starts observing
`result()` with the source terminal held at the first gate. The second
gate is reached when the execution asks the source for another item after
`Done`. It holds EOF while the Pi promise handlers get an event-loop
checkpoint and the Rust driver polls the pinned result future. The drivers
then release EOF and collect the same result operation.

| Observation | Pi `Models.streamSimple` | Rust `Models::stream_simple` |
| --- | --- | --- |
| Before source terminal emission | Result pending | Result pending |
| One terminal emitted, source EOF held | Result fulfilled | Result pending |
| After source EOF release | EOF and `stop` for `lifecycle-differential:mock-model` | EOF, source Drop and `stop` for `lifecycle-differential:mock-model` |

The comparator requires one dispatch, one source terminal, the named
checkpoint with EOF still held, actual EOF after release, and matching
stop reason and message IDs. An early ready result must agree with the
final result on the compared fields. The readiness difference is recorded
as `result-before-source-eof`. The scope is the public result of a successful
call with a controlled finite source tail.

On the frozen Pi candidate, `packages/ai/src/api/lazy.ts:37–52` forwards
the terminal into the outer event stream while continuing to read the source.
`packages/ai/src/utils/event-stream.ts:43–57` resolves the public result
promise as it accepts that terminal. Rust waits for the execution task's
outcome. The #37 repair drains the retained attempt to EOF before
settlement, so the public result waits while the gate holds the source.
[#37 source tail repair](37-ordinary-source-tail-settlement.md) · [Rust execution owner](../../../../crates/opi-ai/src/execution.rs)

## Accepted decision

On 2026-11-11, the user replied “继续推进” to the recommendation to retain
Rust's result timing. For the ordinary successful call in this case,
`ChatCall::result()` waits for the execution task to finish. A successful
result means the call-owned source has reached EOF and been released. This
wait includes the finite source tail after `Done`. The observed result timing
is an intentional difference.
[Lifecycle decision](../decisions/11-telemetry-lifecycle.md#普通-models-的成功结果与源流-eof)

With the #43 setup decisions, the five cases have two passes and three cases
with passing shared assertions and accepted differences. The follow-up measures
public terminal delivery and telemetry settlement at the source EOF gate.
[#43 setup decisions](43-match-public-models-completion.md#accepted-decisions) · [#45 terminal and span observations](45-observe-public-terminal-and-span-before-source-eof.md)

## Verification

- [Material manifest](../evidence/phase5-result-before-source-eof-materials.json), SHA-256 `55e619ef3d57bed1a42136052dbe9594cf7ef8c57b564da74238b726037777f1`.
- [Replay commands and raw traces](../evidence/phase5-result-before-source-eof-differential.json), SHA-256 `534c107e643ac5181f1b3166dd22b0651f82615ff698e6a5315ade2f51b66d57`.
- The original-lock candidate at `C:/Temp/opi-ai-frozen-lock-20261111`
  supplied 15 TypeScript and five Rust invocations. All five shared
  comparisons passed, with zero material blockers and raw verdict
  `DIFFERENCES_REQUIRE_DECISION` / exit 0 and `strict_alignment: false`.
  The four prior cases reproduce
  the #43 comparison records and driver stdout byte for byte.
- The runner rejects the #43 manifest with exit 2 before driver execution.
  The only frozen input drift is in the Rust example, TypeScript observer,
  comparator and runner. The upstream material and Rust library hashes match #43.
- The Python differential suite passed 24 tests. Cached
  `test -p opi-ai --example lifecycle_differential --offline -j4` passed two
  Drop tests. Cached Clippy on the example passed with `-D warnings`.
  Cargo format, the documentation checker and diff checks passed.

Test impact `add` for the new result timing case and comparator negative
paths. `update` for the completion drivers and their assertions. `retain`
for the four prior cases, Drop regressions and historical evidence.

Worktree handoff `committed=none-for-this-task`, `staged=none`. The modified
probe, comparator, runner, tests, differential README, spec and index were
carried in and updated. This issue and its two JSON records are new
untracked paths. The replay and checks ran on Windows x64.

The decision review verified the record hashes, original-lock candidate,
bound Rust and probe inputs, recorded toolchain versions and driver executable
hash. Test impact of recording the decision `none`.
