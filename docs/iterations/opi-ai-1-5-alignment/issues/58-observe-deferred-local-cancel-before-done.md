# Compare local deferred fetch cancellation before Done

Status: replay complete, preterminal local cancellation differences accepted
Date: 2026-11-12
Parent: [alignment specification](../spec.md)
Predecessor: [local cancellation after a ready Done](57-observe-deferred-local-cancel-after-done.md)
Baseline: opi `ee18dd1323a089d536bdc7e293d0b4869777c344` with the #57 worktree inputs

[#99 replay and decision](99-decide-deferred-local-cancel-before-done.md)

## Observations

Both drivers issue one zero-wait fetch on the #57 handle. The source
emits `Start` with empty content and reported usage, then holds the gate
before `Done`. The caller cancels the same in-flight fetch. The Pi
fixture observes the forwarded `AbortSignal` and emits an `error`
event with reason `aborted`. The Rust source stays pending at the
gate and the SDK wrapper observes the caller token. The success gate
stays closed through cleanup and refetch on both sides.
[Pi driver](../tools/observe_upstream.mjs#L870) ·
[Rust driver](../../../../crates/opi-ai/examples/lifecycle_differential.rs#L1660)

| Checkpoint | Pi `Models.fetchDeferred` | Rust `Models::fetch_deferred` |
| --- | --- | --- |
| `Start` emitted, `Done` held | Fetch pending, poll span open with the first-chunk sample | Fetch pending, poll span open with the first-chunk sample |
| Public cancellation result | Promise fulfills with an assistant message with `stopReason: aborted` and `Request was aborted`. The source has run `finally` and reached EOF. Its result has yet to be read and the poll span is open. | `Err(CallFailure)` carries `ProviderError::Cancelled`, the `Aborted` message and `cancelled`. The source has been dropped with the gate held and the poll span is settled. |
| Settled poll and explicit refetch | The forwarder reads the source result once and settles the span. A fresh fetch with the original handle returns the ready message. | The poll state at return is already final. A fresh fetch with the original handle returns the ready message. |

Both sources emit zero `Done` events and both remote cancel entries see
zero calls. The cancellation payloads agree in stop reason, provider,
model, content, handle and the measured usage fields. The settled poll
spans use `stop: aborted` and status `ok`, with the caller as parent.
Both record a first-chunk and request duration and the four token and
one cost samples from the reported snapshot. The drivers validate
duration samples and exclude elapsed values from equality.

## Source and assessment

At frozen Pi revision `f9a26365568127461c47a07a37dcb42c140a3ac5`,
`packages/ai/src/models.ts:856–898,950–975` forwards the signal into
the provider call. Faux's `fetchDeferred` passes the signal to
`streamWithDeltas`, which emits an `error` with `stopReason: aborted`
on cancellation (`packages/ai/src/providers/faux.ts:341–345,358–396,591–644`).
The fixture uses that event shape at a deterministic gate.
`packages/ai/src/api/lazy.ts:37–53` forwards the event before reading
the source result. `packages/ai/src/utils/event-stream.ts:43–48,97–110`
fulfills the public result on the terminal event.

The Rust local cancellation wrapper comes from the original deferred
implementation in opi commit `4b3600fcdeb3ade41bf4f66249337c382630af23`
(AIR-043). The current iteration explicitly reused `ChatStream` and its
`CallFailure` contract for deferred fetch. When cancellation wins while
the source is pending, `ChatStream` drops the source, forms the failure
from the latest partial, and settles telemetry from that message.
[Deferred integration](10-deferred-ai-telemetry.md) ·
[Local cancellation wrapper](../../../../crates/opi-ai/src/models.rs#L1101) ·
[Failure capture](../../../../crates/opi-ai/src/stream.rs#L691) ·
[Result and observer](../../../../crates/opi-ai/src/stream.rs#L560)

Retain the typed cancellation cause so Rust consumers can branch on
`Cancelled` and still inspect the same assistant snapshot. This follows
the iteration's native API decision and the approach already accepted
for the two setup failures. The cleanup evidence supports the Rust SDK's
ownership of the local attempt. The timing of a successful fetch with
a stalled tail was reviewed in #56. [#99](99-decide-deferred-local-cancel-before-done.md)
records the decision on this cancellation case and the caller evidence.
[Public contract](../decisions/04-public-contract.md) ·
[Setup failure decision](43-match-public-models-completion.md#accepted-decisions) ·
[Held-EOF result timing](56-observe-deferred-result-before-source-eof.md)

The transport guides describe the verified Rust failure channel.
This replay covers an in-flight zero-wait call with scripted sources.
Positive waits, real provider HTTP behavior and six-gate acceptance
remain open.
[Transport contract](../../../../crates/opi-ai/docs/transport.md#deferred-transport-semantics) ·
[Explicit remote cancellation](59-verify-deferred-remote-cancel-then-fetch.md)

## Verification

- [Material manifest](../evidence/phase5-deferred-local-cancel-before-done-materials.json), SHA-256 `a5cc92e3a1796f2e8285f5425cf08db544c716e9889fb1099267976c85804593`.
- [Paired replay and raw traces](../evidence/phase5-deferred-local-cancel-before-done-differential.json), SHA-256 `ec11a87804a709bd75af97f50f0b63974429ceb760e8b36211af6f0da69a9c98`.
- The original-lock Windows x64 candidate supplied 48 TypeScript
  invocations and 16 Rust invocations. Each TypeScript triplet has
  identical stdout. All receipts match their raw stdout and hashes.
  The 15 prior case records match #57. All 16 shared comparisons pass
  with zero material blockers. Two cases pass, ten have accepted
  differences, and four deferred differences remain open. The raw
  verdict is `DIFFERENCES_REQUIRE_DECISION` / exit 0 with
  `strict_alignment: false`.
- The #57 manifest returns `STALE_MATERIALS` / exit 2 before build or
  execution. Only the two drivers and comparator differ among bound
  inputs. Pi sources, original and installed locks, dependencies,
  generated inputs, toolchains and Rust library trees match #57.
- The Python differential suite passed 56 tests. The cached Rust
  `test -p opi-ai --example lifecycle_differential --test deferred_contract --test ai_telemetry --offline -j4`
  passed 41 tests. Clippy on the same targets with `-D warnings`, Cargo
  format, documentation and diff checks passed.

Test impact `add` for the paired pre-terminal cancellation case and
comparator regressions. `update` for Rust subprocess assertions.
`retain` for the 15 prior cases and deferred and telemetry regressions.

Worktree handoff `committed=none-for-this-task`, `staged=none`.
The carried-in drivers, comparator, tests, transport guides and
iteration records were updated. This issue and its two evidence JSON
files are new. Links to moved driver code were refreshed.
