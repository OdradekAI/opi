# Compare local deferred fetch cancellation after a ready Done

Status: replay complete, post-terminal local cleanup difference accepted
Date: 2026-11-12
Parent: [alignment specification](../spec.md)
Predecessor: [ready fetch with source EOF held](56-observe-deferred-result-before-source-eof.md)
Baseline: opi `ee18dd1323a089d536bdc7e293d0b4869777c344` with the #56 worktree inputs

[#98 replay and decision](98-decide-deferred-local-cancel-after-done.md)

## Observations

The drivers use the #56 handle, ready message and zero-wait dispatch.
They wait for the source to emit `Done` and reach the held EOF gate.
Then they trigger the local cancel token or signal while the gate is
still closed. Both backends receive the caller's cancellation state.
The scripted source waits at the gate without acting on that state.
[Pi driver](../tools/observe_upstream.mjs#L840) ·
[Rust driver](../../../../crates/opi-ai/examples/lifecycle_differential.rs#L1582)

| Checkpoint | Pi `Models.fetchDeferred` | Rust `Models::fetch_deferred` |
| --- | --- | --- |
| `Done` emitted, EOF held | Ready result available, poll span open | Public result pending, poll span open |
| Local cancel with EOF held | The ready result remains available. The source stays at the gate and the span remains open with only the first-chunk duration. | The same fetch returns the cached ready result. The source is dropped before natural EOF and the poll span is settled with `stop: stop`. |
| Cleanup and explicit refetch | The driver releases EOF. The iterator runs `finally` and its result is read once. The span settles and the same handle yields the ready message on one new dispatch. | The EOF gate stays closed. The same handle yields the ready message on one new dispatch. |

Both sides record zero calls to their remote cancel entry. The initial
ready result and explicit refetch agree in stop reason, provider/model,
content, handle and usage. The settled initial poll span and metric
samples agree on names, counts, context, attributes and counter values.
The drivers validate the duration samples and exclude elapsed values
from equality.

## Source and assessment

At frozen Pi revision `f9a26365568127461c47a07a37dcb42c140a3ac5`,
`packages/ai/src/models.ts:856–898` carries the caller signal into
provider options. The deferred fetch passes those options to the provider
at `models.ts:950–975`. The forwarder at `api/lazy.ts:37–53` continues to
source EOF and reads its result. The observed cleanup timing applies
to the scripted source that waits for the gate independently of the signal.

Rust observes the token in its deferred source wrapper. Cancelling a
held tail ends that wrapper's drain and releases its source. The
success cached by `ChatStream::result()` stays authoritative and the
observer completes from it before the fetch returns.
[Local cancel wrapper](../../../../crates/opi-ai/src/models.rs#L1116) ·
[Cached result and drain](../../../../crates/opi-ai/src/stream.rs#L560)

The Rust completion order gives callers a way to stop a stalled local
tail while retaining the ready outcome and the remote handle. This
supports retaining the order for finite tails and explicit local
cancellation. [#98](98-decide-deferred-local-cancel-after-done.md)
records the decision on this case. The caller must trigger cancellation
to get this behavior on a stalled tail. The follow-up replay checks the failure channel,
cleanup and handle reuse when cancellation arrives before `Done`.
[#58 pre-terminal cancellation](58-observe-deferred-local-cancel-before-done.md)

The transport guides describe the verified post-terminal behavior.
Coverage is a ready response from the scripted backend. Positive
waits, real provider HTTP behavior and six-gate acceptance remain open.
[Transport contract](../../../../crates/opi-ai/docs/transport.md#deferred-transport-semantics)

## Verification

- [Material manifest](../evidence/phase5-deferred-local-cancel-after-done-materials.json), SHA-256 `883c6ae9d01b048db0bb1c9351a29eda4688a4e020adbcb652883c87cbf3459a`.
- [Paired replay and raw traces](../evidence/phase5-deferred-local-cancel-after-done-differential.json), SHA-256 `566ef32eddb62e4013ffa3a088d17cc83f2a8d5c724385d4885861df8d8e40f5`.
- The original-lock Windows x64 candidate supplied 45 TypeScript
  invocations and 15 Rust invocations. Each TypeScript triplet has
  identical stdout. The 14 prior case records match #56. All 15
  shared comparisons pass with zero material blockers. Two cases
  pass, ten have accepted differences, and three deferred differences
  remain open. The raw verdict is `DIFFERENCES_REQUIRE_DECISION` /
  exit 0 with `strict_alignment: false`.
- The #56 manifest returns `STALE_MATERIALS` / exit 2 before build
  or execution. Only the two drivers and comparator differ among
  bound inputs. Source, original and installed locks, dependencies,
  generated inputs, toolchains and Rust library trees match #56.
- The Python differential suite passed 53 tests. The cached Rust
  `test -p opi-ai --example lifecycle_differential --test deferred_contract --test ai_telemetry --offline -j4`
  passed 41 tests. Clippy on the same targets with `-D warnings`, Cargo
  format, documentation and diff checks passed.

Test impact `add` for the paired local-cancel case and comparator
regressions. `update` for Rust subprocess assertions and the
pre-poll cancellation test description. `retain` for the 14 prior
cases and deferred and telemetry regressions.

Worktree handoff `committed=none-for-this-task`, `staged=none`.
The carried-in drivers, comparator, subprocess and comparator tests,
and iteration records were updated. Moved driver links were refreshed.
The two transport guides and the test description are task-owned
tracked edits. This issue and its two evidence JSON files are new.
