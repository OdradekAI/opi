# #105 Match OpenAI Chat output limits by SDK entry

The 42 matched OpenAI Chat calls send the same JSON bodies and produce the
same terminal results. Each SDK resolves route auth once for native and
simple, and uses the injected credential for direct. The #103 exploratory
request used Pi native with a 4096 model capacity and Opi simple with a
16384 catalog capacity. Matching both the entry and the record resolves
that comparison. [#103 observation](103-compare-stream-partial-aliases.md#frozen-observations) ·
[Paired request and result receipt](../evidence/phase5-chat-output-limits.json)

## Requests on the wire

Pi uses `Models.stream`, `Models.streamSimple` and the public
`@earendil-works/pi-ai/api/openai-completions` native `stream` with an
explicit key and normalized transcript. Opi uses `Models::stream`,
`Models::stream_simple` and `Models::stream_direct` with the registered
OpenAI Chat adapter. All calls go to a loopback HTTP peer. The shared model
has a context window of 10000, an output capacity of 4096 and thinking
disabled. The replay selects `max_completion_tokens` and `max_tokens`
in turn with the same compatibility settings on both sides.
[Input and SSE replies](../../../../crates/opi-ai/tests/data/openai-chat-output-limits.json) ·
[Pi probe](../tools/observe_chat_output_limits.mjs) ·
[Rust probe](../../../../crates/opi-ai/tests/openai_chat_output_limits.rs)

| Input | Native and direct on both SDKs | Simple on both SDKs |
| --- | --- | --- |
| Unset limit, ample context | Omit the field | 4096 |
| Explicit 1024 | 1024 | 1024 |
| Explicit 5000, ample context | 5000 | 5000 |
| Unset limit, estimated context 2286 | Omit the field | 3618 |
| Explicit 7000, estimated context 2286 | 7000 | 3618 |
| Unset limit, estimated context 6002 | Omit the field | 1 |
| Explicit zero, estimated context 6002 | Omit the field | Omit the field |

The tight-context inputs use a previous successful assistant message with
reported total usage and a two-token estimated user tail. The simple
compiler subtracts the estimate and the 4096 reserve from the context
window, floors the available space at one, and takes the smaller of that
space and the supplied ceiling. It uses the model output capacity when the
ceiling is unset. Native and direct preserve the caller's positive value
and let the provider decide how to handle it. Zero survives the simple
clamp and Chat omits the field. The peers return a one-token `length`
response for a serialized limit of one. Otherwise they return a two-token
`stop` response. Both SDKs return the same text, raw and normalized stop
reason, response ID and usage for each reply.
[Pi budget compiler](https://github.com/earendil-works/pi/blob/f9a26365568127461c47a07a37dcb42c140a3ac5/packages/ai/src/api/simple-options.ts#L15-L55) ·
[Opi compiler](../../../../crates/opi-ai/src/models.rs#L4102) ·
[Opi Chat projection](../../../../crates/opi-ai/src/openai_chat.rs#L1849)

## Source and callers

Pi added the simple context reserve in `09f105957`. Both v1.3.0
(`71023aec1546d6d677a65669c5665c0927ce0dd0`) and the frozen v1.5.0
(`f9a26365568127461c47a07a37dcb42c140a3ac5`) have the native serializer
and the simple default and clamp used here. Opi `f70dfb328` introduced
the prepared default and reserve. `fb13da43` switched typed native calls
to adapter validation and added wire tests for omission and explicit
values. The matched replay supports keeping that behavior. Native gives
callers access to provider defaults and exact options. Simple provides a
budget for callers that work across providers.
[Pi history](https://github.com/earendil-works/pi/commit/09f10595755040963796cbd4d925a0f56a7f4b97) ·
[Pi native and simple dispatch](https://github.com/earendil-works/pi/blob/f9a26365568127461c47a07a37dcb42c140a3ac5/packages/ai/src/models.ts#L904-L947) ·
[Opi native and simple dispatch](../../../../crates/opi-ai/src/models.rs#L5304) ·
[Existing Opi wire tests](../../../../crates/opi-ai/tests/node4_preparation.rs#L157)

The Pi coding agent delegates ordinary requests through
`modelRuntime.streamSimple`, and Durable also calls `Models.streamSimple`.
Opi's agent uses `prepare_call_with_policy` for its retry admission. That
entry uses the neutral compiler and freezes the resolved budget for each
permitted attempt. On tool continuation or a model switch, the product
default follows the selected model. The coding-agent configuration checks
its explicit ceiling against that model and the enabled thinking budget
before dispatch. This configuration policy covers a different entry from
the public native SDK call that sends 7000 unchanged.
[Pi coding agent](https://github.com/earendil-works/pi/blob/f9a26365568127461c47a07a37dcb42c140a3ac5/packages/coding-agent/src/core/sdk.ts#L412-L423) ·
[Pi Durable](https://github.com/earendil-works/pi/blob/f9a26365568127461c47a07a37dcb42c140a3ac5/packages/durable/src/harness/generation.ts#L739-L749) ·
[Opi agent request](../../../../crates/opi-agent/src/agent_loop.rs#L518) ·
[Opi product config](../../../../crates/opi-coding-agent/README.md#config-sessions-and-context-files)

The English and Chinese provider-auth manuals describe the available-space
floor and Chat zero handling. The replay covers valid complete Chat model
records with thinking disabled. Numeric thinking budget projection and
Pi's helper behavior for a nonpositive context window have separate
contracts. Opi's model validation requires positive context and output
capacities.
[Provider auth](../../../../crates/opi-ai/docs/provider-auth.md) ·
[中文](../../../../crates/opi-ai/docs/provider-auth.zh.md) ·
[Opi model validation](../../../../crates/opi-ai/src/types/model.rs#L1376)

## Materials

- The [#104 material manifest](../evidence/phase5-chat-interleaving-materials.json)
  remains applicable. It binds the original package lock, installed
  dependency resolution, generated catalog, compiled Pi trees, and current
  Rust sources. Seven selected Pi files match the pinned Git blobs. The
  replay rechecks all material and probe bindings before and after running
  each side three times. [Replay](../../../../scripts/pi-sdk-conformance/differential/replay_chat_output_limits.py)
- The build receipt records the cache-wrapper command, exit, Cargo
  JSON stdout and stderr, material manifest digest, fixture and Rust
  probe digests, Cargo-reported executable and archived executable with
  its digest. The local receipt is `C:/Temp/opi-105-chat-output-verified-build.json`
  with SHA-256
  `4fc06a48dc451c19c24951498ec0aae86aec6982d76a034ae2401d6664341edb`.
  The archived executable has SHA-256
  `c6365e30ead65580d34fd9beb5e044675fe56307fb150bddcd22d8624c45d451`.
  The [raw replay](../evidence/phase5-chat-output-limits.json) has SHA-256
  `7c30bd11f2fcd7479ec59953473a223631341c791276770601f6bb9694949874`.

## Verification

- The `openai_chat_output_limits`, `openai_chat_fixtures` and
  `node4_preparation` targets pass 143 Rust tests. The targeted
  `provider_collection` prepared-budget test passes. The frozen Pi Chat and
  context-estimate suites pass 14 tests. The differential suite passes 96
  tests. Negative replay commands reject an existing output, a mismatched
  executable hash and missing Pi probe arguments.
- The coding-agent regression initially failed to compile because the
  partial usage aggregate did not initialize `Usage.cost`. The aggregate
  now starts from `Usage::unknown()` and retains the token counts it has.
  Eight output-limit tests and four failed-run usage tests pass.
  [Usage aggregation](../../../../crates/opi-coding-agent/src/harness.rs#L3190)
- The session-runtime target passes 63 cases. Its
  `phase14_usage_subsets_survive_session_resume` case still fails on a
  `cost` field in session JSONL. It writes a reported assistant usage directly
  through the session coordinator. The pricing serialization contract
  remains open. [Failing assertion](../../../../crates/opi-coding-agent/tests/session_runtime.rs#L3275)
- Affected-target Clippy with warnings denied, Rust formatting, Node
  syntax, the doc checker and whitespace checks pass. The Phase 1
  live-tree verifier reports drift against its historical source inventory.
  Its original index content and the current #104 material manifest each
  verify against their own recorded inputs.

Test impact `add` for the loopback and comparator checks and `retain`
for the existing budget, usage and historical lifecycle evidence.
Worktree handoff `committed=none-for-this-task` and `staged=none`.

Next, gate OpenAI Chat at the finish frame and at `[DONE]` to compare
block ends, early consumer actions and source cleanup. SDK tool-argument
preview, other adapter and observer cases, [#94 real deferred integration](94-record-real-deferred-integration-gap.md),
and six-gate acceptance remain open.
