# #104 Keep one OpenAI Chat text block across tool deltas

OpenAI Chat now appends text that resumes after tool arguments to the same
content index. The mapper closes the text block at completion with its full
text. Raw deltas still arrive in wire order. The change applies to the shared
Chat Completions mapper, including the OpenRouter and legacy Mistral and Azure
Chat routes. Native Mistral Conversations and Azure Responses have separate
mappers.
[Chat mapper](../../../../crates/opi-ai/src/api/openai_completions.rs#L935) ·
[Azure Chat](../../../../crates/opi-ai/src/api/azure_openai.rs#L226) ·
[Mistral routes](../../../../crates/opi-ai/src/api/mistral.rs#L22)

## Why change the mapper

The wire has one `choice.delta.content` text stream and separate indexed
`tool_calls` streams. A tool delta carries no text boundary. The frozen Pi
v1.5.0 adapter keeps the first text block and appends all later text to it.
Pi v1.3.0 (`71023aec1546d6d677a65669c5665c0927ce0dd0`) already did the same.
Opi's early text close came from the initial Chat adapter `3ace5b8b`.
The replay shows a lost whitespace suffix in the next model request and
a different ordered transcript after a provider switch. Keeping the text
block open fixes both at the point where the blocks are created.
[Pinned Pi mapper](https://github.com/earendil-works/pi/blob/f9a26365568127461c47a07a37dcb42c140a3ac5/packages/ai/src/api/openai-completions.ts#L407-L471) ·
[Opi Chat replay](../../../../crates/opi-ai/src/api/openai_completions.rs#L2311) ·
[Opi Anthropic replay](../../../../crates/opi-ai/src/api/anthropic.rs#L2470)

## Frozen replay

The Pi and Rust public simple entry points receive the same SSE bytes from
loopback HTTP peers. One fixture puts a text delta between the first tool
argument delta and the remaining arguments. The second substitutes a space
and newline for that resumed text. The same Rust driver runs on both mapper
revisions. A raw-frame gate holds the usage trailer before normalization.
Each SDK makes the original request and a real Chat continuation with the
terminal message and a fixture tool result. The Anthropic control captures
the assistant portion of the compiled continuation payload.
[Fixture](../../../../crates/opi-ai/tests/data/openai-chat-interleaved.sse) ·
[Rust probe](../../../../crates/opi-ai/tests/openai_chat_interleaved.rs) ·
[Pi probe](../tools/observe_chat_interleaving.mjs)

| Observation | Pi v1.5.0 | Opi before | Opi after |
| --- | --- | --- | --- |
| Text delta indexes in arrival order | 0, 0 | 0, 2 | 0, 0 |
| Text end events | One, full text | Two, at tool start and finish | One, full text at finish |
| Final blocks | Text AB, tool | Text A, tool, text B | Text AB, tool |
| Next Chat request with nonblank B | `ABCD😀EF` | `ABCD😀EF` | `ABCD😀EF` |
| Next Chat request with whitespace-only B | `"ABCD \n"` | `"ABCD"` | `"ABCD \n"` |
| Next Anthropic assistant content | Text AB, tool use | Text A, tool use, text B | Text AB, tool use |

The repaired event kinds, indexes, deltas, and final content agree for the
two inputs. Both complete Chat request bodies agree with the explicit 4096
output limit and the same tool declaration. The Anthropic comparison covers
the assistant content block array. Tool identity, parsed final arguments,
response ID, usage and stop reason agree. The existing replay serializers
still consume the blocks recorded in historical transcripts.
[Before and after traces](../evidence/phase5-chat-interleaving.json)

At the held usage frame, Pi has emitted neither block end. Repaired Opi
has already emitted `TextEnd` and `ToolCallEnd` in response to
`finish_reason`. Both still await the terminal message. Pi closes blocks
when its OpenAI SDK iterator ends, which can occur at the `[DONE]`
sentinel. The fixture releases the usage frame before comparing `Done`
with `result()`. The finish-to-iterator-end timing remains open.
[Pi termination](https://github.com/earendil-works/pi/blob/f9a26365568127461c47a07a37dcb42c140a3ac5/packages/ai/src/api/openai-completions.ts#L672-L693) ·
[Opi source identity](../evidence/phase5-chat-interleaving-materials.json)

## Caller impact

Durable's `PartialChanges` clears a text block's pending append plan when it
receives `text_end`. It independently parses each tool's raw deltas to
publish argument changes. The frozen Harness consumes each recorded
nonblank stream through a controlled event projection and flushes progress
while the next event is held. Old Opi's resumed text appears as a third
block and stays third in the stored and replayed message. The repaired
stream produces the same progress and stored/replayed block order as Pi.
All three inputs run the tool once with `{"a":"BCDEF","n":2}`, dispatch
twice including the follow-up, and settle `done` with the same answer.
[Durable changes](https://github.com/earendil-works/pi/blob/f9a26365568127461c47a07a37dcb42c140a3ac5/packages/durable/src/harness/partial-changes.ts#L46-L104) ·
[Durable progress flush](https://github.com/earendil-works/pi/blob/f9a26365568127461c47a07a37dcb42c140a3ac5/packages/durable/src/harness/generation.ts#L699-L753)

For the unfinished keys `"a"` and `"n"`, Pi's SDK preview omits the key
and Opi's display helper supplies `null`. The controlled Harness shows an
omitted key for all three streams because it applies the raw argument
deltas. This accounts for Durable's result. Direct SDK preview callers can
still observe the difference. The Opi agent updates its TUI via text
deltas and commits the final blocks for session replay. AI telemetry
records each block end, so the repaired text block produces one text end
record with the full UTF-16 length.
[Opi agent](../../../../crates/opi-agent/src/agent_loop.rs#L1891) ·
[Opi telemetry](../../../../crates/opi-ai/src/telemetry.rs#L190)

## Materials and verification

- The #102 original-lock snapshot, installed dependencies, catalog, and
  compiled Pi files are identical. The only source change between the two
  material manifests is `api/openai.rs`. Twelve selected Pi source files
  match Git revision `f9a26365568127461c47a07a37dcb42c140a3ac5`. Each of
  the three builds produces three equal traces.
  [Before materials](../evidence/phase5-deferred-direct-drop-poll-materials.json) ·
  [After materials](../evidence/phase5-chat-interleaving-materials.json) ·
  [Replay driver](../../../../scripts/pi-sdk-conformance/differential/replay_chat_interleaving.py)
- The raw receipt SHA-256 is
  `91629a0598035d939325170c6ea85486163297f4bcdf0cac13e3ba9b272baddd`.
  The pre-repair test executable is
  `5b9ba6f48b563d23c5078eee317998ecf732248f8fe0d62744b45e8f41f2534e`
  and the repaired one is
  `bbe0123fb20f283fddcaee54818148172efd6344f2f99b55a04218e253491e2d`.
  Both are sealed by hash. The #103 raw receipt, original test source
  and executable are preserved. The live #103 alias test follows the
  repaired event order.
- The frozen Pi tests pass 188 AI and seven Durable cases. The Python
  differential suite passes 91 tests. Negative CLI checks reject an
  existing receipt, a changed executable hash and missing Pi probe inputs.
- The 11 scoped `opi-ai` test binaries pass 372 cases, including 116 in
  `openai_chat_fixtures`. They cover the shared Chat routes, replay,
  HTTP tails and AI telemetry. Clippy with warnings denied on the library
  and those test targets, Rust formatting, Node syntax, the doc checker
  and `git diff --check` pass.

Test impact `add` for event-index, whitespace continuation and paired
consumer controls, `update` for the mixed and live partial-alias
expectations, and `retain` for the frozen historical traces. Worktree
handoff `committed=none-for-this-task` and `staged=none`.

Next, replay omitted output limits with identical model records and compare
Pi and Opi native, simple, and direct entry points. The #103 exploratory
comparison used Pi `stream` and Opi `stream_simple`. Pinned Pi
`streamSimple` also defaults to the model cap. The native comparison
is still needed. The held-finish difference, SDK argument preview,
[#94 real deferred integration](94-record-real-deferred-integration-gap.md),
and six-gate acceptance remain open.

[#105](105-compare-chat-output-limits.md) completes the matched output-limit
replay. The finish-to-iterator-end timing is the next open Chat observation.
