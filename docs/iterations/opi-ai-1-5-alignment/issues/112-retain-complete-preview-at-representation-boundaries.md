# #112 Retain complete previews at representation boundaries

Status: approved preview boundary repair and bounded verification complete;
final numeric decoding and product performance acceptance remain separate.

Final numeric decoding is continued in [#113](113-preserve-final-numeric-decoding.md);
this issue and its receipts retain their historical scope.

Continuation of [#111](111-map-incremental-argument-previews.md), using worktree
HEAD `ee18dd1323a089d536bdc7e293d0b4869777c344` and frozen Pi 1.5.0 revision
`f9a26365568127461c47a07a37dcb42c140a3ac5` at
`C:/Temp/opi-ai-frozen-lock-20261111`. Original lock and OpenAI SDK 7.19.0 remain
pinned. No provider calls, staging, commit, branch, push or release.

## Decision and repair

The old preview omitted an overflowing numeric value. For example,
`{"a":[1e400,true]}` became `{"a":[true]}`; a duplicate overflowing member could
leave its superseded value visible while other members advanced. The user
explicitly selected whole-snapshot holding and the 128-container limit on
2026-11-14. See the [owning decision](../decisions/04-public-contract.md#非有限数字及快照深度的完整保留).

`ToolCallArgumentsStream` now retains non-finite scalars as internal nodes in
their actual slots. A reachable non-finite value holds the entire previous
representable snapshot, including while the scalar is pending. Valid subsequent
overwrites can recover even before their delimiter; replaced unreachable nodes
do not block publication. Non-prefix replacement resets the snapshot to `{}`.
Invalid numeric grammar remains tolerant preview input rather than an overflow
node. Raw text, UTF-16 handling, independent snapshot ownership, copy credit,
128-container bound and strict finalization retain their existing contracts.

The locked serde_json version is **1.0.149**. `IgnoredAny` validates JSON grammar
without imposing the finite `Number` range, then Rust's f64 parser classifies
the value. No dependency, feature, provider, Agent or persistence code changes.
An earlier overflow in a duplicate member still fails strict final validation,
even when the preview has recovered through a later valid member. Approved
Unicode/numeric/depth representation differences remain outside strict Pi
equivalence; final large-number rounding has not been approved as a difference.

## Frozen comparison and real callers

The [final paired receipt](../evidence/phase5-preview-boundaries-verified.json)
binds [current materials](../evidence/phase5-preview-boundaries-materials.json),
Cargo-reported sealed executables, fixtures, probes and comparator. Three runs
are stable except elapsed timing fields. The SDK production tree changes only
`streaming_json.rs` from #111; other material trees and frozen source bindings
are identical.

- All original **65 Chat checkpoints** still match Pi through the new owner;
  the original 18 closures, request/final values and actual Durable/Chord ordinary
  commits remain verified. The sealed #110 executable still returns the exact
  historical legacy-helper trace.
- **23 interface cases / 124 operations** include non-finite arrays, pending
  values, duplicate recovery, cross-tool isolation and replacement reset, in
  addition to the existing Unicode/depth/key-order/final controls. All 1,200
  array lengths and copy-credit controls match. Twenty-eight difference records
  remain explicit: approved representation/final rejection boundaries plus one
  existing derived final-number difference; the receipt is not strict alignment.
- [Actual caller controls](../evidence/phase5-preview-boundaries-callers.json)
  invoke frozen `ToolCallArgumentsStream`, Chord `copyJson`, and Durable
  `PartialChanges`. Pi accepts overflow, while Chord rejects non-finite JSON.
  Supplying a constructed finite `{}` preview to `PartialChanges` still fails
  its commit because it independently parses the raw overflow delta. That
  constructed control is not a Rust invocation or a whole-Harness test.
- Actual `ToolExecutionComponent` control batches 202 argument updates across
  two explicit frames into two renderer calls; an unchanged frame adds none,
  and completion adds one. Frozen `pi-tui/dist` is absent, so the successful
  component probe uses an explicit resolver to the frozen TUI source. Missing
  dist and invalid Windows import attempts remain in the evidence; this is
  neither an upstream package smoke nor a real terminal timing measurement.

The comparator now binds both final raw strings back to the accumulated input
for the exact overflow controls. Review found that equal forged finals on both
sides could otherwise pass. A synchronized mutation now rejects this case,
alongside coherent-but-wrong held snapshots. The
[initial paired receipt](../evidence/phase5-preview-boundaries-differential.json)
and its comparator bytes are preserved; the final receipt supersedes its
comparison assurance after the fix and a fresh three-run replay.

## Display cost calibration

Frozen Pi `packages/tui/src/tui.ts` coalesces ordinary render requests at 16 ms;
forced/input rendering can bypass that delay. The tool component stores the
latest arguments and updates its display on render. Durable's
`packages/durable/src/harness/agent.ts` defaults to 100 ms trailing partial
commits; `generation.ts` allows one commit in flight and settles it at exit.

opi's current `agent_loop.rs` does not forward tool argument deltas to the
product TUI. `interactive.rs` displays tool arguments from `MessageEnd`; its
50 ms input poll does not establish a preview cadence. No product throttling
or arbitrary performance threshold is added.

The public SDK test uses **virtual 1 ms arrivals**, 32-character fragments,
and 1/16/100 ms snapshot intervals with a final flush. Every delta is parsed.
It verifies final values and retained snapshot independence, recording owned
snapshot calls, returned character volume and host debug timings separately.

| Input characters | Snapshot interval | Calls including final flush | Returned string characters |
| --- | --- | --- | --- |
| 25,000 | 1 ms | 783 | 9,821,872 |
| 25,000 | 16 ms | 49 | 627,112 |
| 25,000 | 100 ms | 8 | 114,600 |
| 250,000 | 1 ms | 7,814 | 977,062,496 |
| 250,000 | 16 ms | 489 | 61,339,792 |
| 250,000 | 100 ms | 79 | 10,109,200 |

These are controlled output-volume observations, not allocator totals, actual
UI elapsed time or equal-work comparison with Pi's structural sharing. The
append-only and per-chunk controls from #111 are also rerun. A performance
budget needs a selected real embedding and measured arrival/render workload.

## Verification and handoff

The [check receipt](../evidence/phase5-preview-boundaries-checks.json) records
commands, source applicability, logs and worktree inventories:

- **76 top-level Rust tests plus one depth child**: owner 12, interface/cost 3,
  Chat preview 1, unchanged helper utilities 54, shared terminal route 5
  (75 scenarios), Agent caller 1 (three scenarios).
- **15 comparator tests**, including 44 mutation subcases, five consistent
  incorrect-snapshot controls, synchronized forged finals, input drift and
  CLI no-overwrite/negative cases. CLI success and rejection logs are retained.
- Four SDK doctests, affected-target Clippy and rustdoc with warnings denied,
  workspace formatting, documentation and whitespace checks pass.
- [README package consumption](../evidence/phase5-preview-boundaries-readme-package.json)
  and the [independent archive consumer](../evidence/phase5-preview-boundaries-package-consumer.json)
  pass defaults off/on. Each profile runs all **16 tests plus its depth child**
  copied from the actual archive, with no workspace source or product crate
  dependency. [Archive bindings](../evidence/phase5-preview-boundaries-package-bindings.json)
  verify all original SDK/telemetry payloads against current workspace bytes.

All 216 pre-existing JSON evidence files retain their original bytes. New raw
logs, backups, sealed executables and runners live under
`C:/Temp/opi-preview-boundaries-20261114/`; actual archives and consumers live
under `C:/Temp/opi-preview-boundaries-package-20261114/`. #111's package and
sealed executable evidence remains historical and unchanged. English/Chinese
utility guides and the existing Unreleased Added entry change together; README
API usage remains accurate, and the iteration issue has no language mirror.

Test impact: `add` overflow recovery/grammar controls, controlled cadence and
comparator rejection cases; `update` overflow regression, interface fixture and
comparator expectations; `retain` raw/final/helper/terminal/Agent tests and all
old evidence; `delete=none`. Worktree-only delivery, `committed=none-for-this-task`,
`staged=none`, HEAD unchanged. No full-workspace, MSRV/other-platform,
minimal-feature, real Deferred (#94), performance-budget or six-gate conclusion.

## Next slice

Prioritize the existing **final numeric decoding** difference: the raw integer
one followed by 64 zeroes becomes Pi `1e64` versus serde_json
`9.999999999999999e63`, adjacent f64 values. The new preview `finish` returns
text/error and does not create this gap, but actual `opi-agent` final parsing
feeds hooks, schema validation, authorization and tool execution.
`session_coordinator::tool_arguments_for_session` also re-decodes and serializes
the value for persistence. Thus the difference has real caller impact, beyond
the original probe observation. Add caller/session-resume controls and assess
the smallest decoding fix separately; do not silently broaden workspace serde
features or label the rounding difference intentional. Select a real preview
display consumer before defining any performance acceptance budget.
