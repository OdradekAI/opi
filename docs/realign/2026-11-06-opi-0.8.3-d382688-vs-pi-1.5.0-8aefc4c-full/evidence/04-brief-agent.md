# Batch AGT — agent loop & harness (Dim 5)

pi side: `packages/agent/src/{agent.ts, agent-loop.ts, proxy.ts,
stream-fn.ts, types.ts}` + `packages/agent/test/`.

Opi side: `crates/opi-agent/src/{agent.rs, agent_loop.rs, loop_types.rs,
streaming_proxy.rs, event.rs, session.rs, session_branch.rs,
session_context.rs, session_event.rs, harness.rs, hooks.rs, authority.rs,
sdk.rs, extension.rs, message.rs, evidence.rs, validation.rs, compaction.rs,
diagnostic.rs, diagnostic_sink.rs, tool.rs, tool/}`.

Measure:
1. Stateful AgentSession wrapper: construction params, `prompt()` workflow,
   completed-turn decision (queued input vs model decides), steering vs
   follow-up queue semantics, when queued input becomes next-turn prompt vs
   in-flight steering.
2. Tool continuation: loop_tool_call/deferred tools, result submission,
   parallel tools.
3. Termination/abort: cancellation tokens, drop semantics, in-flight request
   behavior.
4. Transactional next-turn replacement vs opi session replacement semantics.
5. Context/model updates mid-session (both sides).
6. `proxy.ts` event proxying vs `streaming_proxy.rs`.
7. opi-agent modules with no pi-agent counterpart (authority, evidence,
   validation, diagnostic_sink, sdk, session_branch): describe behavior, mark
   opi-only.

Trace full caller flow both sides:
caller -> request prep -> stream -> tool calls -> completion -> cleanup.

Output `evidence/agent.md`, prefix `AGT`. Tallies + coverage honesty.
