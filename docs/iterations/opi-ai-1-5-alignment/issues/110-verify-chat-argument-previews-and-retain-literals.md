# #110 Verify Chat argument previews and retain trailing literals

Status: literal-loss repair and bounded preview comparison complete; the wider
incremental-preview contract remains open.

Continuation of [#109](109-repair-baseline-checks-and-refresh-packages.md)
examines the direct SDK preview gap left by [#103](103-compare-stream-partial-aliases.md)
and [#104](104-align-openai-chat-interleaved-text.md). Opi remains a worktree-only
candidate at `ee18dd1323a089d536bdc7e293d0b4869777c344`. Pi is the original-lock
1.5.0 snapshot at `C:/Temp/opi-ai-frozen-lock-20261111`, revision
`f9a26365568127461c47a07a37dcb42c140a3ac5`.

## Finding and repair

These are three distinct interfaces: Pi's Chat adapter uses a per-tool
`ToolCallArgumentsStream`; its older `parseStreamingJson` utility remains
separate; Opi Chat emits owned raw argument strings and recommends
`message::parse_streaming_json` for display. An Opi probe calling that helper
does not establish that the mapper itself parses previews.

The helper discarded a complete trailing literal before the enclosing
container closed. For example, `{"a":true` displayed `{"a":null}`, and
`[1,false` displayed `[1]`. `fix_dangling_token` accepted only *shorter*
literal prefixes and then tried complete literals as numbers. Removing that
length exclusion retains both complete and incomplete literal tokens.
This is the sole production source change. Final argument parsing and
validation retain their authority; tolerant display never authorizes a tool.
[Implementation](../../../../crates/opi-ai/src/types/message.rs) ·
[Regression](../../../../crates/opi-ai/tests/public_utilities.rs)

The new regression failed before the fix on `{"a":true` and passes afterward.
It checks all three literals in object, array and nested-array prefixes,
both before and after closing their containers. Two real adapter controls
cover the helper's other production callers: Pi Messages shows `true` in
the delta and still accepts the server's authoritative final replacement;
native Mistral Conversations shows `true` but retains the incomplete raw
final with `arguments_error`.
[Pi Messages](../../../../crates/opi-ai/tests/chat_pi_messages.rs) ·
[Mistral Conversations](../../../../crates/opi-ai/tests/mistral_fixtures.rs)

## Paired observations and caller impact

The [fixture](../../../../crates/opi-ai/tests/data/chat-argument-preview.json)
contains ten inputs and 65 chunk boundaries: missing/split/empty keys,
nested containers, numeric fractions/exponents, split Unicode escapes,
whitespace/control characters, invalid escapes, duplicate keys and array
literals. Both public simple Chat entries make equal authenticated loopback
requests, consume identical argument chunks and produce equal final objects.
Raw-frame gates hold the next frame until the probe captures the prior delta;
Pi values are copied at event time. No provider credentials or live services
are used.
[Rust probe](../../../../crates/opi-ai/tests/chat_argument_preview.rs) ·
[Pi probe](../tools/observe_chat_argument_preview.mjs)

| Prefix or condition | Frozen Pi Chat | Repaired Opi helper |
| --- | --- | --- |
| `{"a":true` | Keeps `true` | Keeps `true` after the repair |
| `{"na` | `{}` | `{"na":null}` |
| `{"a":1,"a":` | Keeps the earlier `a:1` | Temporarily replaces it with `null` |
| `{"n":1.` | Omits the unfinished number | Shows `n:1` |
| `{"s":"a\u00` | Shows `s:"a"` | Shows the incomplete escape as literal text |
| Unclosed string with trailing space/control or invalid escape | Keeps the streamed text | Keeps it too; Pi's older utility differs here |

The repair closes the two complete-literal mismatches in the shared fixture.
Eighteen stream-preview checkpoints still differ; the older Pi utility has
23 remaining mismatches. These are recorded observations, **not approved
intentional differences**. The current result remains `strict_alignment: false`.

The probe also invokes the frozen Durable `PartialChanges.observe/take/commit`
and Chord `beginChange/prepare/adopt` with each side's previews. Both produce
equal progress commits and final arguments for every input. Durable parses
the raw deltas itself, so this agreement does not imply SDK preview equality.
This is an actual component control, not a new full Harness, tool execution,
task-settlement or durable-storage acceptance run. Each Pi execution's Rust
input file is bound to its same-run Rust trace by path and SHA-256, rechecked
after execution.

Source review confirms that frozen Pi Agent forwards tool updates and its
interactive UI passes `content.arguments` into tool components; direct renderers
can observe these differences. Opi Agent does not forward `ToolCallDelta` to
its public progress projection and executes from the final message. The
existing full Agent/Durable controls in #104/#108 remain historical evidence.
Ten relevant Pi source files are checked against the pinned Git blobs.

## Verification and evidence

- [Final paired receipt](../evidence/phase5-chat-argument-preview-repair-verified.json):
  three stable repetitions for both pre-repair and repaired builds, each
  paired with frozen Pi. Requests, raw prefixes, final objects and both
  Durable projections are checked. [Before materials](../evidence/phase5-chat-argument-preview-materials.json)
  and [after materials](../evidence/phase5-chat-argument-preview-repaired-materials.json)
  differ only in the SDK's `types/message.rs`; the source delta is checked
  against the archived original and is exactly the one-condition repair.
- The [comparator](../../../../scripts/pi-sdk-conformance/differential/compare_chat_argument_preview.py)
  requires both expected repairs and compares JSON with booleans distinct
  from numbers. Eleven tests include partial repair, wrong values, missing
  cases/checkpoints, final drift and consumer drift. A CLI negative check
  rejects an existing output with exit 2 and preserves its hash.
- All **151 tests** across `public_utilities` (54), `chat_pi_messages` (41),
  `mistral_fixtures` (55) and `chat_argument_preview` (one test / ten cases)
  pass. The unchanged Chat terminal target adds five passing tests / 75
  scenarios. Focused library/test Clippy with `-D warnings`, formatting,
  Node syntax, documentation and whitespace checks pass; exact commands and
  outcomes are in the [check receipt](../evidence/phase5-chat-argument-preview-checks.json).
- [Final README package proof](../evidence/phase5-chat-preview-final-readme-package.json)
  passes defaults off/on. An [external package consumer](../evidence/phase5-chat-preview-package-consumer.json)
  runs the archive's exact Chat probe/fixture and the extracted literal
  regression in both profiles: ten cases / 65 checkpoints and one literal
  test per profile. It depends only on the archived SDK, same-run telemetry
  and ordinary pinned support dependencies. Metadata rejects workspace
  source or Agent/coding-agent/protocol/TUI dependencies.
- [Archive/source bindings](../evidence/phase5-chat-preview-package-bindings.json)
  match all 313 original SDK and 17 telemetry payloads to the workspace.
  SDK SHA-256 is `bde05add862c4ced5feb567d7e35c21f654a53284e4d1db9e3c624ee723f9075`;
  telemetry remains `54ffe66285c677661c4f64991b53fa580f69fe8f0a9e370588bc6af285d2fd7a`.
  All 144 carried-in JSON evidence files retain their original bytes.

Raw logs, source backups, both sealed Rust executables and one-shot runners
are retained under `C:/Temp/opi-chat-preview-20261114/`. The first probe build
used a nonexistent convenience method, and the second exposed a fixture's
integer/float representation assertion; both development failures are kept.
The first Node probe used an unavailable Chord dist path; the corrected probe
uses its pinned source under the same source condition as Durable. The initial
comparison receipt and exact old comparator remain historical: review found
that it could overlook one missing repair and confuse booleans with numbers.
The verified receipt supersedes that comparator's conclusion, without
overwriting the original. An earlier successful README package run is also
retained; the final package run includes the completed caller tests and docs.

## Next slice and limits

Implement and validate the missing **incremental argument-preview mapping**
separately from the existing whole-prefix utility. Preserve raw argument
deltas, owned snapshots and strict final validation. First settle the public
Rust mapping for Pi's append/replace/finish behavior, then test missing values,
duplicate keys, split escapes, malformed/non-object finals, interleaved tools,
retained snapshots and long-input cost. Do not make a one-key omission patch
and call it complete incremental parsing. If changing the shared helper too,
include Pi Messages and native Mistral consumers in that contract migration.

This slice does not approve the remaining preview differences or close the
minimal feature configuration, other-platform/MSRV, real Deferred integration
(#94), performance budgets or six-gate milestone. The current scope is Windows
x64. The full #109 workspace checks remain the prior baseline; this local
SDK change runs its affected tests and Clippy rather than claiming a renewed
full-workspace result.

Test impact: `add` literal-loss, native Mistral final-authority, gated Chat
preview and comparator regressions; `update` Pi Messages' existing final
override test with a delta assertion; `retain` terminal-boundary tests and
all prior evidence; `delete=none`. The product utility guide is updated in
English and Chinese; this issue has no language mirror.

Worktree handoff: `committed=none-for-this-task`, `staged=none`; HEAD is
unchanged. Eight existing dirty source/tests/docs are carried-in-and-touched;
the Pi Messages and Mistral test files were initially clean and are modified.
New probes, fixtures, this issue and evidence are untracked. Unrelated work
and legacy workflow state remain preserved.
