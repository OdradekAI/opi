# Compare a ready deferred fetch while source EOF is held

Status: replay complete, successful held-EOF result timing accepted
Date: 2026-11-12
Parent: [alignment specification](../spec.md)
Predecessor: [two zero-wait fetches](55-observe-deferred-wait0-pending-then-ready.md)
Baseline: opi `ee18dd1323a089d536bdc7e293d0b4869777c344` with the #55 worktree inputs

[#97 replay and decision](97-decide-deferred-fetch-held-eof.md)

## Observations

Both public fetch helpers make one explicit zero-wait dispatch with
the same seeded handle and ready response used in #55. The driver
holds the ready `Done` and then holds the source EOF at a separate
gate. It observes the same public fetch operation throughout.
[Pi driver](../tools/observe_upstream.mjs#L840) ·
[Rust driver](../../../../crates/opi-ai/examples/lifecycle_differential.rs#L1582)

| Checkpoint | Pi `Models.fetchDeferred` | Rust `Models::fetch_deferred` |
| --- | --- | --- |
| Before source `Done` | Public result pending | Public result pending |
| After `Done`, with source EOF held | Public result fulfilled with `stop` and text `ready` | Public result pending, source still held |
| After releasing EOF | Natural EOF, one iterator `finally` and one source `result()` read. Public result matches the value already returned. | Natural EOF and source Drop observed when the same fetch returns the matching ready message. |

Both poll spans remain open at the two held checkpoints, with one
first-chunk duration measurement each. After release, both have one
settled `pi.ai.poll` child span with `stop: stop`, both duration
measurements, four token counter samples and one cost counter sample. The compared
result fields, arguments and measurement values other than durations
agree. The duration samples are checked for valid values and the
elapsed times are excluded from equality.

The named gates establish the dependency on the source tail without
using elapsed time or a timeout as a readiness test. The Pi driver gives the
public promise handlers one event-loop checkpoint while the gate
remains closed. Rust polls its pinned fetch future while the gate is
closed. Coverage is a ready outcome from scripted sources.

## Source and assessment

The frozen Pi revision `f9a26365568127461c47a07a37dcb42c140a3ac5`
resolves the public result from the first terminal push in
`packages/ai/src/utils/event-stream.ts:43–47`. The forwarder continues
to EOF and observes the source result at
`packages/ai/src/api/lazy.ts:37–53`.

Rust's deferred fetch uses the `ChatStream::result()` completion
boundary adopted in #10. After the terminal it drains and releases
the source, completes the observer and then returns. That order gives
callers a ready message with the local stream already released and
the poll record available when the source terminates normally. The
gate also exposes the cost to callers. A custom source that stalls
after `Done` delays an otherwise ready fetch, including at zero wait.
[Deferred integration](10-deferred-ai-telemetry.md) ·
[Rust fetch](../../../../crates/opi-ai/src/models.rs#L1143) ·
[Rust completion boundary](../../../../crates/opi-ai/src/stream.rs#L560)

The recommendation at this replay was to retain the completion order
for finite tails. [#97](97-decide-deferred-fetch-held-eof.md) records
the decision after comparing both Rust fetch entries. The
[local cancellation follow-up](57-observe-deferred-local-cancel-after-done.md)
checks the ready result, source release, poll settlement and remote
cancel count while EOF is held after `Done`.

The product READMEs now describe the existing pending and ready
outcomes and the source drain. This replay covers one ready fetch
and normal EOF release. Positive waits, cancellation, real provider
HTTP and six-gate acceptance remain open.

## Verification

- [Material manifest](../evidence/phase5-deferred-result-before-source-eof-materials.json), SHA-256 `e4ef102c998be11b98d496ba8963fcf619493443305f3ddd079574e3bd4420ef`.
- [Paired replay and raw traces](../evidence/phase5-deferred-result-before-source-eof-differential.json), SHA-256 `610127cedba605456ba406ac83a47fff94bfd9cb381bea36c916c27f674e274b`.
- The original-lock Windows x64 candidate supplied 42 TypeScript
  invocations and 14 Rust invocations. Each TypeScript triplet has
  identical stdout. The 13 prior case records match #55. All 14
  shared comparisons pass with zero material blockers. Two cases
  pass, ten have accepted differences, and the two deferred timing
  differences remain open. The raw verdict is
  `DIFFERENCES_REQUIRE_DECISION` / exit 0 with `strict_alignment: false`.
- The #55 manifest returns `STALE_MATERIALS` / exit 2 before build
  or execution. Only the two drivers and comparator differ among
  bound inputs. Source, original and installed locks, dependencies,
  generated inputs, toolchains and Rust library trees match #55.
- The Python differential suite passed 50 tests. The cached Rust
  `test -p opi-ai --example lifecycle_differential --test deferred_contract --test ai_telemetry --offline -j4`
  passed 41 tests. Clippy on the same targets with `-D warnings`, Cargo
  format, documentation and diff checks passed.

Test impact `add` for the paired held-EOF case and comparator
regressions. `update` for the Rust subprocess assertions. `retain`
for the 13 prior cases and deferred and telemetry regressions.

Worktree handoff `committed=none-for-this-task`, `staged=none`.
The drivers, comparator, subprocess and comparator tests, product
READMEs and iteration records were carried in and updated. Moved
driver links were refreshed. This issue and its two evidence JSON
files are new.
