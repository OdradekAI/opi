# Observe the public Models terminal and request span at source EOF

Status: done
Owner: Codex for user
Parent: [alignment specification](../spec.md)
Predecessor: [public result timing](44-observe-public-result-before-source-eof.md)
Baseline: opi `ee18dd1323a089d536bdc7e293d0b4869777c344` with the #39–#44 worktree changes

## Frozen replay

`models-terminal-and-span-before-source-eof` uses the same original-lock
candidate and controlled source as #44. Each public Models call reads
`Start` and starts one next operation while the source terminal is held.
The second gate holds source EOF after a single source `Done`. The drivers
sample the same next operation and the recorded `pi.ai.request` span at
that gate. After release they observe actual source EOF, a single public
`Done` followed by outer EOF, and the final result from the retained call.

| Checkpoint | Pi `Models.streamSimple` | Rust `Models::stream_simple` |
| --- | --- | --- |
| After public `Start`, source terminal held | Next pending, one open request span | Next pending, one open request span |
| Source `Done` emitted, source EOF held | Public `Done` ready, request span open | Next pending, request span open |
| Source EOF and public stream completion observed | One public `Done`, outer EOF, one settled request span with `stop` | One public `Done`, outer EOF, source Drop, one settled request span with `stop` |

The Pi driver passes an actual parent from the frozen telemetry package's
in-memory recorder. It captures the delegated request-span promise and
awaits it after source EOF to sample the settled child span. The Rust driver
polls the pinned `next()` and reads `Recorder::spans()` before release, then
reads it again after source Drop and public EOF. Both public results match
the observed terminal. The comparator checks source and public stream
completion, terminal identity, and one matching request span with its final
`stop`. The differing public `Done` readiness is recorded as
`terminal-and-span-before-source-eof`.

In the frozen Pi `v1.5.0` source at `f9a26365568127461c47a07a37dcb42c140a3ac5`,
`packages/ai/src/api/lazy.ts:37–52` forwards `Done` as it reads it. The
forwarder reads to EOF and awaits the source result before calling the
observer's `onResult`. That callback releases the span promise in
`packages/ai/src/telemetry.ts:181–207`, and the memory adapter records
settlement in `packages/telemetry/src/memory.ts:232–245`. Rust's execution
task awaits the finish path, which drains the retained source, before
sending the public terminal. [Rust execution](../../../../crates/opi-ai/src/execution.rs#L105)

## Accepted decision

On 2026-11-11, the user replied “继续推进” to the recommendation to retain
Rust's public `Done` timing. For the successful call with a finite tail in
this case, Rust delivers public `Done` after the call-owned source reaches
EOF and is released. The caller waits for the tail and can treat the
terminal as completion of the call's source work, consistent with the #44
result contract. The public terminal readiness at the EOF gate is an
intentional difference.
[Lifecycle decision](../decisions/11-telemetry-lifecycle.md#普通-models-的公开终态与源流-eof) · [#44 result decision](44-observe-public-result-before-source-eof.md#accepted-decision) · [#37 source tail repair](37-ordinary-source-tail-settlement.md)

With the #43 and #44 decisions, the six cases have two passes and four
cases with passing shared assertions and accepted differences. The
follow-up probes a result attribute write failure at the public Models
entry points. [#46 result telemetry fault](46-observe-public-result-attribute-write-failure.md)

## Verification

- [Material manifest](../evidence/phase5-terminal-and-span-before-source-eof-materials.json), SHA-256 `07059d515ff976036c673bb39b2a699c8d8873a5122ba3604ffc57701ac1ca6d`.
- [Replay commands and raw traces](../evidence/phase5-terminal-and-span-before-source-eof-differential.json), SHA-256 `8f50deada026fb4185264f73794e40ab24be8df25f3237da701f7d46432607ac`.
- The candidate at `C:/Temp/opi-ai-frozen-lock-20261111` supplied 18
  TypeScript and six Rust invocations. All six shared comparisons passed
  with zero material blockers. The raw verdict is
  `DIFFERENCES_REQUIRE_DECISION` / exit 0 with `strict_alignment: false`.
  The five prior comparison records and driver stdout match #44 byte for
  byte. The two standalone Pi `lazyStream` completion cases reproduce
  the six recorded #42 stdout streams.
- The runner rejects the #44 manifest before driver execution with
  `STALE_MATERIALS` / exit 2. The Rust example, TypeScript observer,
  comparator and runner are the four changed bound inputs. The upstream
  source, locks, installed/generated materials, Rust library trees and
  runtime versions match #44.
- The Python differential suite passed 27 tests. Cached
  `test -p opi-ai --example lifecycle_differential --offline -j4` passed two
  Drop tests. Cached Clippy on the example passed with `-D warnings`.
  Cargo format, the documentation checker and diff checks passed.

Test impact `add` for public terminal and request-span timing and invalid
trace coverage. `update` for both drivers and their comparator and runtime
assertions. `retain` for the five prior cases, standalone probes and Drop
regressions.

Worktree handoff `committed=none-for-this-task`, `staged=none`. The probes,
comparator, runner, tests, differential README, spec, index and #44 issue
were carried in and updated. This issue and its two JSON records are new
untracked paths. The replay and checks ran on Windows x64.

The decision review verified the record hashes, original-lock candidate,
bound Rust and probe inputs, recorded toolchain versions and driver executable
hash. Test impact of recording the decision `none`.
