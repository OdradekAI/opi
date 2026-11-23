# #107 Observe Chat finish, usage, sentinel and caller settlement

Status: historical investigation and decision complete; repair follows in [#108](108-repair-chat-terminal-boundary.md).

Do **not** accept the current timing as an intentional difference. Opi ends
blocks at `finish_reason`, but the larger defect is success at usage: a later
provider error or request cancellation cannot change the cached success.
Real Agent execution can run a tool and finish before that tail arrives.
Without usage, Opi instead ignores `[DONE]` and waits for HTTP EOF. This
task establishes the decision and reproducible evidence; it does not change
production behavior or claim protocol alignment.

## Paired observations

The original-lock Pi 1.5.0 snapshot is
`C:/Temp/opi-ai-frozen-lock-20261111`, pinned to
`f9a26365568127461c47a07a37dcb42c140a3ac5`. Source blobs are checked against
`D:/Luiz/Odradek/pi`. Its installed OpenAI SDK is 7.19.0. Opi remains the
dirty candidate at HEAD `ee18dd1323a089d536bdc7e293d0b4869777c344`.
Material manifests and executable/source hashes, rather than HEAD alone,
identify both inputs.

Each public simple entry consumes the same fixture from its own loopback
HTTP peer. All eight request bodies agree. Raw no-op frame gates establish
that finish and usage have been processed; the peer then holds the response
open. A 200 ms bounded observation records public events, `result()`, EOF,
and peer disconnect before natural HTTP EOF. Three independent runs per
side produce identical observations. This is a bounded scheduling check,
not a latency benchmark or an assertion that arbitrary slow runtimes always
settle within 200 ms.

| Boundary or tail | Frozen Pi | Current Opi |
| --- | --- | --- |
| Finish, before usage | No block ends; pending | Text/tool ends; pending |
| Usage trailer, before `[DONE]` | Pending | Success, public EOF, response released |
| Usage on the finish frame | Pending | Immediate success and response release |
| `[DONE]` without usage, HTTP still open | Success and response release | Still pending, response retained |
| Natural EOF after finish/usage, no sentinel | Success at EOF | Already succeeded at usage |
| Error before usage | Error, no block ends | Error, blocks already ended |
| Error after usage | Error | Cached `tool_use` success |
| Cancel before usage | Aborted; closes blocks during SDK unwind | Aborted; blocks already ended |
| Cancel after usage | Aborted | Cached `tool_use` success |

Final content, tool identity/arguments, response ID, raw finish reason and
token counts agree in these fixtures. The final stop reason differs in the
two after-usage failure cases. Equality checks remove only Opi's additional
raw argument JSON field; the evidence retains it. They do not normalize away
timing, result classification, usage or block-end differences.

[Shared fixture](../../../../crates/opi-ai/tests/data/openai-chat-finish-timing.json) 路
[Rust probe](../../../../crates/opi-ai/tests/openai_chat_finish_timing.rs) 路
[Pi probe](../tools/observe_chat_finish_timing.mjs) 路
[Three-run paired evidence](../evidence/phase5-chat-finish-timing.json) 路
[Materials](../evidence/phase5-chat-finish-before-materials.json)

## Actual callers

The real Opi `agent_loop` uses `Models` and the HTTP Chat provider, with a
harmless terminating tool. In done/error/cancel variants, it has already
emitted `MessageEnd`, invoked the tool once and successfully returned while
the peer still holds the protocol tail. Releasing an error or cancelling
the same Agent token afterwards cannot retract that completed run. This is
an actual Agent call, not a synthetic event projection.
[Agent probe](../../../../crates/opi-agent/tests/agent_chat_finish_timing.rs)

Separately, the frozen real Durable Harness consumes the recorded Pi and
Opi SDK event/message projections. At the held usage boundary, Pi-derived
streams remain pending with zero tools and no assistant history. Opi-derived
streams have executed the tool once, dispatched the follow-up, stored both
assistant messages and settled. After releasing error/aborted tails, the
Pi-derived runs settle `unanswered/model_error`, execute no tools and store
the failed assistant; Opi-derived runs remain successful.
[Durable probe](../tools/observe_chat_finish_consumers.mjs) 路
[Three-run caller evidence](../evidence/phase5-chat-finish-consumers-verified.json)

That Durable control is a trace projection into the real Harness, **not** a
live Pi-to-Rust bridge. It uses Faux identity and a controlled progress timer;
diagnostic text and reference aliasing are not projected. The aborted trace
models a provider-aborted response, not cancellation of Durable's own token.
It therefore proves the tool and storage consequences of the observed
terminal classification, not all retry or application cancellation policy.

Source inspection explains these observations:

- Opi `consume_event` ignores `TextEnd`/`ToolCallEnd`; `Done` supplies the
  final message, emits `MessageEnd`, and permits tool execution. Moving only
  the two block ends cannot repair premature tool execution.
  [Agent consumption](../../../../crates/opi-agent/src/agent_loop.rs#L1891) 路
  [Final message and tools](../../../../crates/opi-agent/src/agent_loop.rs#L672)
- `ThinkingEnd` is forwarded to Agent consumers, and AI telemetry records
  every block end. Their timing still matters even where text/tool end events
  are ignored. This fixture does not contain a thinking block.
  [Thinking update](../../../../crates/opi-agent/src/agent_loop.rs#L1915) 路
  [Telemetry](../../../../crates/opi-ai/src/telemetry.rs#L200)
- The coding harness commits a successful run; the session coordinator then
  appends messages and updates cumulative usage. Early success can therefore
  propagate into a successful session commit. This last statement follows
  the inspected call chain; this task did **not** add an on-disk coding-agent
  session probe.
  [Harness commit](../../../../crates/opi-coding-agent/src/harness.rs#L3130) 路
  [Session commit](../../../../crates/opi-coding-agent/src/session_coordinator.rs#L448)
- Pi closes blocks after its provider iterator ends. Its OpenAI SDK breaks
  on `[DONE]` and releases the response; ordinary SSE errors skip the block
  close loop, whereas transport abort can unwind normally before Pi checks
  the signal. Durable applies block ends to its progress bookkeeping, then
  awaits the final result before tool execution.
  [Pinned Chat source](https://github.com/earendil-works/pi/blob/f9a26365568127461c47a07a37dcb42c140a3ac5/packages/ai/src/api/openai-completions.ts#L672) 路
  [Pinned generation](https://github.com/earendil-works/pi/blob/f9a26365568127461c47a07a37dcb42c140a3ac5/packages/durable/src/harness/generation.ts#L740)

## Diagnosis and next repair

The three hypotheses were premature success, merely early observation with
callers still waiting, and completed results with cleanup still waiting.
The first explains the usage cases: public EOF, peer disconnect and real
tool execution all occur early. The no-usage case is the opposite error:
neither result nor response release occurs at the sentinel.

The causes are local and directly observable: the parser discards `[DONE]`,
the mapper finalizes blocks on Finish, and usage flushes a pending successful
terminal. Once `saw_done` is true, later provider events are ignored.
The recorded source locations are `api/openai.rs:76` for the sentinel parser,
`api/openai.rs:950` for the finish mapper and `api/openai.rs:1072` for usage
settlement. [Recorded source identity](../evidence/phase5-chat-finish-before-materials.json)

Next, repair this boundary as one scoped change:

1. Treat finish/usage as accumulated metadata. Finalize successful blocks
   and publish `Done` at the protocol sentinel or valid natural EOF.
2. Preserve tail errors/cancellation until that terminal, and release the
   response at `[DONE]` without requiring HTTP EOF, including absent usage.
3. Keep missing-finish compatibility explicit; check grammar/strict argument
   finalization and failure partials before moving the existing finish code.
4. Cover the shared Chat routes, including Azure's separate transport loop,
   and verify tool suppression, usage, raw callbacks and one telemetry
   settlement. Decide cancellation-only block-end details separately from
   success; do not copy the upstream SDK's unwind artifact blindly.

The proposed equal-timing assertion was run before any production edit and
failed at the early block ends. Its exact source and red log are retained
under `C:/Temp/opi-chat-finish-20261114/` and bound by the checks receipt.
It is not installed as a permanently failing CI test. The checked-in
observation/comparison tests instead verify the known mismatches and report
`repair-required` / `strict_alignment=false`; passing them does not approve
the defect. The repair must update this explicit comparator and retain the
historical evidence.

## Verification and boundaries

- `cargo test -p opi-ai --test openai_chat_finish_timing -- --nocapture`:
  one passing observation test, eight cases; paired replay repeats each
  side three times with identical traces.
- `cargo test -p opi-agent --test agent_chat_finish_timing -- --nocapture`:
  one passing observation test, three cases; real Agent and Durable
  consumer replays each repeat three times.
- `python -B -m unittest discover -s scripts/pi-sdk-conformance/differential -p test_chat_finish_timing.py`:
  eleven passing comparator tests, including subprocess refusal to overwrite
  existing evidence before loading inputs.
- Affected test-target Clippy with `-D warnings`, `cargo fmt --check --all`,
  both Node syntax checks, doc checker and `git diff --check` pass.
  [Commands, hashes and red/green checks](../evidence/phase5-chat-finish-checks.json)

Cargo commands use the existing shared cache wrapper and retain their raw
logs/executables. No provider credentials, paid calls, production edits,
staging or commits were made. Test impact: `add` for two Rust probes and
comparison/consumer controls; `retain` for historical fixtures and evidence;
`update/delete=none`. The iteration-only report has no pre-existing language
mirror; the owning Chinese spec is updated. No user-visible runtime change
requires a changelog entry in this diagnostic slice.

Thinking blocks, incomplete/strict/grammar arguments, provider-specific Chat
routes, malformed/transport tails and every public entry are not newly
validated by these eight cases. The next repair must select their affected
checks. Direct SDK argument preview, #94 real Deferred integration, minimal
features, other platforms and six-gate acceptance remain open.
