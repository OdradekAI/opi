# #103 Compare retained partials on OpenAI Chat and Faux

Keep Opi's event-time snapshots for these routes. A caller can retain a
record of earlier progress and can edit its owned value independently
of the terminal result. Current progress comes from the latest event.
This preserves a concrete ownership guarantee and records the observable
difference under E03 and K02.
[Public contract](../decisions/04-public-contract.md#流事件的-partial-所有权)

## Frozen observations

The compiled Pi `Models.stream` routes a registered OpenAI Chat adapter to a
loopback HTTP peer. Rust `Models::stream_simple` uses the built-in OpenAI Chat
adapter against the same SSE bytes. Raw-event callbacks hold each next frame
before the adapter processes it. At each gate the probe reads the public
events and retains the early message, content array or block, and argument
value. The Pi Faux run gates the real provider's chunk timers. The Opi Faux
control scripts the same deltas through the existing `MockProvider`.

| Checkpoint | Pi OpenAI Chat | Pi Faux | Opi |
| --- | --- | --- | --- |
| Reread retained `start` after completion | Same message and array as the terminal. Two blocks, `toolUse`. | Independent top-level objects. Original `start.content` stays empty when a block is appended. | The opening event keeps its original empty content and pending stop reason. |
| Reread first text delta after completion | Original block now reads `ABCD😀EF`. | Old text array still shares its block, which now reads `ABCD😀EF`. | The original delta still reads `ABCD`. |
| Reread an old tool block and its separately saved argument object | Block points to a later arguments object. Saved `{"a":"BC"}` keeps its original value. | Same replacement pattern within the shared block. | The saved event still owns the text prefix `{"a":"BC`. |
| Edit the old message ID, text block and saved arguments while the source is held | The changed ID and text reach the terminal. Later chunks replace the edited argument object. | Text edit reaches other partials with the same block. The terminal comes from a cloned script and retains its values. | Edits stay in the specific owned events. |
| Read latest observed partial after a usage-only frame, with the next frame held | Input usage is already 7. | No corresponding wire frame. | Input usage is 0 in the retained event and 7 in the final result. |

Both OpenAI adapters make one authenticated POST with equal JSON under the
explicit 4096 output-token limit. The baseline raw text and argument deltas,
content indexes, final text, tool identity, parsed arguments, response ID,
input usage and stop category match. Opi emits `text_end` before starting the
tool block. Pi emits it at the end of the provider stream, after the five
tool deltas. The raw event sequences and both mutation outcomes remain in
the receipt. In a separate setup check, an omitted output-token limit
left the field absent in Pi and sent 16384 in Opi. The paired run uses
the explicit limit to hold the request equal while testing ownership.
[SSE fixture](../../../../crates/opi-ai/tests/data/partial-aliases.sse) ·
[Raw replay](../evidence/phase5-partial-aliases.json)

## Callers and source history

The frozen Pi agent replaces its current partial on each content event
and commits the message from `result()`. Its shallow event projection
can still carry nested references. The interactive coding agent replaces
its displayed message and tool arguments from updates and can reread the
latest message on redraw. Opi's agent publishes text and thinking deltas
with current snapshots, and commits history and selects tools from the
terminal message. The interactive Opi UI appends incoming text deltas.
The callers examined can retain a historical value and continue reading
new events for progress. External extensions may rely on JS aliasing.
[Pi agent loop](https://github.com/earendil-works/pi/blob/f9a26365568127461c47a07a37dcb42c140a3ac5/packages/agent/src/agent-loop.ts#L424-L465) ·
[Pi interactive UI](https://github.com/earendil-works/pi/blob/f9a26365568127461c47a07a37dcb42c140a3ac5/packages/coding-agent/src/modes/interactive/interactive-mode.ts#L3578-L3658) ·
[Opi agent terminal](../../../../crates/opi-agent/src/agent_loop.rs#L672) ·
[Opi progress events](../../../../crates/opi-agent/src/agent_loop.rs#L1891)

Durable stores the latest received partial and copies it synchronously
when the progress timer fires. The probe runs the frozen Harness against
the same controlled text stream with shared and event-time values. It
advances a usage-only update, fires the actual Harness progress timer
while the next text delta is held, and reads the published `pi.live`.
Both modes publish `ABCD`. The shared mode publishes input usage 7 and
the owned mode publishes 0. Both submit one model call, commit the same
final `ABCD😀EF` with usage 7, and settle `done`. The Harness runs with a
controlled message projection to isolate ownership.
[Durable generation](https://github.com/earendil-works/pi/blob/f9a26365568127461c47a07a37dcb42c140a3ac5/packages/durable/src/harness/generation.ts#L699-L753)

Pi v1.3.0 `71023aec1546d6d677a65669c5665c0927ce0dd0` already passed the
same live OpenAI message and spread the Faux top level. Opi's OpenAI
mapper used owned partials from `3ace5b8` and the stream regression
recorded event-time values in `b2ff2f3`. The current write and reread
controls give a reason to retain that behavior with the 1.5.0 source.
The memory cost of queuing long snapshots remains unmeasured.

Pi's `f715a15c1` introduced incremental tool argument parsing in 1.5.0.
The real OpenAI and Faux providers expose parsed object previews.
With only a key received, the tested Pi preview omits it. Opi exposes
raw argument text and its existing `parse_streaming_json` helper
projects that key to `null`. Once the value arrives, the tested string
prefixes and final objects match. Opi's agent also redacts tool data on
its public event projection. The argument preview and `text_end` timing
merit their own consumer and protocol controls.
[Pi parser change](https://github.com/earendil-works/pi/commit/f715a15c101ca8a44d759bceeb7ec2a51024c81c) ·
[Opi display helper](../../../../crates/opi-ai/src/types/message.rs#L442) ·
[Opi agent redaction](../../../../crates/opi-agent/src/event.rs#L478)

## Verification

- The #102 material manifest verifies before and after replay with the same
  Pi v1.5.0 snapshot and original lock. Sixteen relevant source and consumer
  files match Git revision `f9a26365568127461c47a07a37dcb42c140a3ac5`.
  Each side produces three identical traces. The comparison retains
  `strict_alignment: false` and both per-route differences.
  [Materials](../evidence/phase5-deferred-direct-drop-poll-materials.json) ·
  [Pi probe](../tools/observe_partial_aliases.mjs) ·
  [Rust test](../../../../crates/opi-ai/tests/partial_aliases.rs)
- The replay SHA-256 is
  `6c86462f47f9c1747972326d59bcd92bd8f10270eb08050c964fa383ee0e1da1`.
  The #67, #89 and #102 reports retain their original hashes. The Rust
  test executable was sealed by the Cargo-reported path and SHA-256
  `7196a3d0bbec4532961ba9b0854897f8396f18894a567cf497291f32f4b60b11`.
- The pinned Pi targets pass 36 selected AI cases and two selected Durable
  cases. The controlled matrix exercises HTTP, Faux, and the Durable
  progress publication path.
- The `partial_aliases`, `stream_events` and `openai_chat_fixtures` targets
  pass 145 Rust tests. Clippy on those targets with warnings denied,
  Rust formatting, Node syntax, the doc checker and `git diff --check`
  pass. The differential suite passes 85 Python tests. Negative commands
  exit 2 for an existing output path, a mismatched Rust executable hash,
  or an absent Pi snapshot argument. The existing receipt still verifies.

Test impact `add` for the gated integration probe and comparator regressions
and `retain` for the existing lifecycle cases. Worktree handoff
`committed=none-for-this-task` and `staged=none`. The bilingual manuals,
spec, decision and contract index are carried in and touched. This issue,
the raw receipt, fixture and probe files are new untracked paths.

Next, verify OpenAI Chat with text continuing after a tool delta and check
which callers treat `text_end` as authoritative. Compare the current
argument preview in the same trace. The omitted-limit request also needs
its own disposition. Anthropic and other adapters, external extension
behavior, [#94 real deferred integration](94-record-real-deferred-integration-gap.md)
and the six-gate acceptance remain open.

[#104](104-align-openai-chat-interleaved-text.md) resolves the premature
OpenAI Chat text split and records the remaining finish-timing and argument
preview differences. The old-order alias receipt and the original executable
and test source are historical. The current test asserts the repaired order.
The #104 replay uses matching simple entry points. The omitted-limit check
above compared Pi native with Opi simple and needs matching entry points.

[#105](105-compare-chat-output-limits.md) compares the matched entries on the
same complete model record. Both SDKs omit an unset native or direct limit
and compute the simple limit from the model capacity and available context.
The exploratory trace above remains part of #103.
