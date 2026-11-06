# Batch CHD/DUR — chord host runtime & durable execution (Dims 6, 18)

Measured 2026-11-06 by source inspection of `.pi/pi-1.5.0` (pi 1.5.0, content `8aefc4c29d420c27`) vs Opi `068f1cf` (tree `d382688`).
Contracts measured: `@earendil-works/chord` 1.5.0 (public root export list + `/node /context /delta /output /telemetry-sqlite /sqlite /sqlite/node /sqlite/cloudflare /next` subpath exports; src ≈14.4K lines core + ≈28.6K lines `src/next` v2) and `@earendil-works/pi-durable` 1.5.0 (root export list + `/env /env/node /codemode /tool-search /tools /images /images/node /images/cloudflare /storage/* /testing` subpath exports; harness ≈11.3K lines, session ≈4.9K lines).
Verdict tallies: 20 records — `missing` 13, `partial` 4, `different` 3, `equivalent` 1 (integration-status record only); 0 `opi-only`/`unresolved`.
Biggest gaps: no host/plugin runtime at all in Opi (chord contracts, hosts/facets, links, hot reload, principals — all 0-term absences), and no durable execution layer (no task scheduler, no checkpoint/resume of in-flight work, no replay policy, no crash-loop restart limit; Opi "durable" matches are session-JSONL persistence and "checkpoint" is a file-length savepoint).
Uncertainties: `exactly.?once` (31 hits) and `durable` (152 hits) were classified by sampling only, not exhaustive reading; pi `src/next` (chord v2) measured by module list + design docs, not per-file reading.

## Layer H — authority wording and maturity (recorded first, per duties)

### CHD-H-01 — chord package authority and maturity
- Dimension(s): Layer H
- pi entry: `packages/chord/README.md:1-267` (public), `packages/chord/docs/spec.md:1-9` ("Where it and the code disagree, the disagreement is a bug" — normative), `packages/chord/docs/guide.md:1-5` ("The specification defines its semantics"), `packages/chord/docs/tutorial.md:1` (every example runs as `test/tutorial.test.ts`)
- Opi entry: no counterpart document exists; absence search `grep -rniE 'chord|facet' crates/*/src --include='*.rs'` → 0 matches
- Comparison: missing
- Observable difference: pi publishes a normative spec + usage guide + runnable tutorial for a typed host runtime; Opi has no analogous runtime or its docs.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: no "Experimental" banner anywhere in chord README/spec — chord v1 is presented as decided, normative ("the disagreement is a bug"). Observed maturity: **implemented** — 55 test files in `packages/chord/test/` (incl. `reload.test.ts`, `host-objects-reload.test.ts`, `principals.test.ts`, `delta-tracker/`, `output.test.ts`, `link.test.ts`, `manager.test.ts`).

### CHD-H-02 — chord v2 (`chord/next`) experimental surface and slice-program design docs
- Dimension(s): Layer H (+ Dimension 6)
- pi entry: `packages/chord/src/next/index.ts:1-2` ("Chord v2 (`@earendil-works/chord/next`, experimental)"), exported via `packages/chord/package.json` `exports["./next"]`; design authority `packages/chord/docs/next/design.md:1-4` ("the one current statement of every rule (owner, 2026-10-24)… code follows this document"); slice statuses in `docs/next/slice-*.md` headers (6a/6b/7b/8/10 **built**; 7c **ready to build**; 11 **draft**; layer-2 **superseded**, `docs/next/layer-2.md:3`)
- Opi entry: absence search `grep -rniE 'chord' crates/*/src` → 0; no design-doc analog
- Comparison: missing
- Observable difference: `@earendil-works/chord/next` ships an experimental second-generation runtime (contracts, data, documents, host, jobs, link, registries, state, store ≈28.6K src lines + `src/next-node` http/workers/plugins/compression + tests under `test/next/`, `test/next-node/`); Opi has nothing comparable at any maturity.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: authority wording is owner-approval gated per slice ("approved (owner, …)", "built (…)" with named reviewers); observed maturity **partial** (built slices live behind an explicitly experimental export; slice 11 draft, 7c unbuilt). Also a design TODO marker: `docs/spec.md:7` plugin loader "TODO(loader): replace with an isolating, evicting loader" (owner-approved exception, old module copies leak).

### DUR-H-01 — durable package authority and maturity
- Dimension(s): Layer H (+ Dimension 18)
- pi entry: `packages/durable/README.md:3` ("> **Experimental.** The API changes without notice between releases."), `packages/durable/docs/spec.md:1-2` ("This document is normative", 6531 lines, §1–§13 incl. "Non-goals")
- Opi entry: absence search `grep -rniE 'durable|checkpoint|task.?queue|exactly.?once' crates/*/src` → hits classified below; no normative durable-execution spec in Opi
- Comparison: missing
- Observable difference: pi ships an experimental-but-fully-specified durable agent harness with 121 test files in `test/` plus Cloudflare Workers tests (`test-workers/wake.test.ts`, `test-workers/storage.test.ts`) and 4 storage benchmarks; Opi has no durable-execution contract or tests.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: observed maturity **implemented** (not scaffold): runnable examples `test/examples/00`–`36`, storage conformance suite, workers tests. Authority wording mixes "Experimental" (README) with "normative" (spec §0) — the API is normative within an experimental versioning policy.

### DUR-H-02 — durable group-commit design doc
- Dimension(s): Layer H
- pi entry: `packages/durable/docs/group-commit.md:1-4` ("Status: design, revision 2 (after three reviews). Branch `durable-perf` …")
- Opi entry: no analog (`grep -rniE 'group.?commit' crates/*/src` → 0)
- Comparison: missing
- Observable difference: a designed-not-yet-merged perf change to the Durable Session commit path; Opi has neither the mechanism nor the design.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: maturity **designed-only**; cites uncommitted prerequisites on a branch — evidence that pi 1.5.0's durable layer is under active perf work.

## CHD records — chord host runtime (Dimension 6)

### CHD-01 — contracts (local/remote/method/opaque, bind, coverage)
- Dimension(s): 6
- pi entry: `packages/chord/src/contract.ts:33` (`method`), `:214` (`remote`), `:249` (`local`), `:284` (`coverage`), `:321` (`bind`), opaque via `src/index.ts:16`; schemas are pi-schema JSON Schema (`README.md:11-13`)
- Opi entry: absence search `grep -rniE 'chord|facet|contract' crates/*/src --include='*.rs'` — "contract" appears only as tool-execution contracts (e.g. `crates/opi-coding-agent/src/execution/runtime.rs:738` `BashExecutionContract`), not a cross-process RPC contract type
- Comparison: missing
- Observable difference: pi callers define runtime contract values (name, version, input schema, host placement, state/method schemas) and obtain typed references valid locally and across processes; Opi has no contract mechanism — tool schemas exist in opi-agent (`crates/opi-agent/src/tool.rs`) but only describe a single-process tool call.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: —

### CHD-02 — hosts, facets, registries, object lifetime
- Dimension(s): 6
- pi entry: `packages/chord/src/host.ts:96` (`defineFacet`), `:251` (`createHost`), `HostOptions.loader` `:146`; `src/registry.ts:11` (`defineRegistry`); facet/loader protocol `host.ts:76-90`; child-host process management exported from `src/node.ts` (`createHostManager`, `forkDriver`, `childControl`, `processLock`, `nodeLoader` from `src/loader.ts:2`)
- Opi entry: absence search as CHD-01 (0 chord/facet terms); closest in-tree concept is `crates/opi-agent/src/extension.rs` `ExtensionRegistry` (startup-registered in-process extensions)
- Comparison: missing
- Observable difference: pi objects are created on demand, addressed by `(type, canonicalInput, principalKey)`, live while referenced, survive reloads, and can be served in child host processes; Opi has no object/host model — extensions are plain registries with no lifetime, placement, or process topology.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: —

### CHD-03 — links and wire protocol (memory/IPC/WebSocket)
- Dimension(s): 6
- pi entry: `packages/chord/src/link.ts:272` (`memoryLinkPair`), `:301/:339` (`IpcLink`/`ipcLink`), `:361/:414` (`WebSocketLink`/`webSocketLink`), `:113` (`PACK_THRESHOLD = 64 KiB` binary packing), `:218` (`inflateMessage`); `src/websocket.ts:18` (`acceptWebSockets`); `src/wire.ts:40` (`PROTOCOL_VERSION = 4`)
- Opi entry: word-collision only: `crates/opi-ai/src/ws/frame.rs:1-2` is a WebSocket **codec for provider transports** (Codex/realtime APIs), not a host link; `crates/opi-coding-agent/src/rpc.rs:1-10` is a JSONL-over-stdio RPC, not a frame protocol with links/principals/placement
- Comparison: missing
- Observable difference: pi hosts exchange versioned JSON frames over pluggable links with pack/unpack over 64 KiB; a caller can move a host cluster from in-memory to IPC to WebSocket without touching contracts. Opi's process boundary is exactly two fixed shapes (stdio JSONL RPC; provider HTTP/WS) with no link abstraction.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: —

### CHD-04 — delta tracker, replicated state, watches
- Dimension(s): 6
- pi entry: `packages/chord/src/delta/tracker.ts` (2319-line transactional tracker), `src/delta/index.ts` (apply/diff/encoder/decoder, 742 lines), `src/state.ts:404-413` (`replicatedState` with source-attachment variant), `src/watch.ts:287` (`Watch`), `:406` (`NativeWatch`); ownership contract in `src/delta/README.md`; delivery policies in `README.md` ("Public state subscriptions", "Watches": 100 pending complete-value deliveries, 10,000 pending ops → root replacement)
- Opi entry: absence search `grep -rniE 'replicated|delta|watch' crates/*/src --include='*.rs'` — no state-replication analog; Opi state is the session JSONL + in-memory event stream
- Comparison: missing
- Observable difference: pi publishes atomic revisions with structural ops (string append/front-truncate, splices, permutations) that remote viewers apply incrementally, with defined overflow folding; Opi consumers re-read whole state or receive push events (`crates/opi-agent/src/event.rs:29` `AgentEvent`) with no ops, no folding policy, and no cross-process replication.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: opi-tui redraws from full state snapshots; no delta delivery exists anywhere in the workspace.

### CHD-05 — hot reload
- Dimension(s): 6
- pi entry: `packages/chord/src/reload.ts:23` (`ReloadOptions` incl. graceMs for aborted tasks, host.ts:129), `:119` (`runReload`), `:152` (`endReload`); in-flight work finishes on its code version, state kept, incompatible changes migrate at a safe point (`README.md:7`); tested in `test/reload.test.ts`, `test/host-objects-reload.test.ts`
- Opi entry: absence search `grep -rniE 'hot.?reload' crates/*/src --include='*.rs'` → **0 matches**; `reload` in `crates/opi-coding-agent/src/interactive.rs:80-96` means reloading the package store at startup (word collision)
- Comparison: missing
- Observable difference: pi swaps plugin code inside a running host with defined semantics (references survive, roots abort only when their facet is replaced); Opi cannot change any code without restarting the process.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: —

### CHD-06 — principals
- Dimension(s): 6
- pi entry: `packages/chord/src/host.ts:110-123` (principal is app-supplied, JSON-serializable, never invented by Chord; `principalKey`; routed hosts act as `Route.principal`), `src/contract.ts:360` (`ctx.principal`), principal-scoped object identity `README.md:6`; `PrincipalMatch`/`PrincipalPattern` exported from `host.ts` (`src/index.ts:20`); tested in `test/principals.test.ts`
- Opi entry: closest is `crates/opi-agent/src/authority.rs` (permission boundary) and `crates/opi-coding-agent/src/policy.rs` — trust/permission gating, not per-caller object identity
- Comparison: missing
- Observable difference: in pi, an owner and a guest addressing the same contract input reach different objects with different methods, and hosts can route as other principals; Opi has one implicit principal (the local process) with permission checks, so callers cannot be distinguished at the object layer.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: —

### CHD-07 — bounded output streams and paced progress
- Dimension(s): 6
- pi entry: `packages/chord/src/output.ts:130` (`OutputBuffer`: head-or-tail window, `OutputLimits`, `OutputSkip`), `:52` (`boundOutput`), `:43` (`sanitizeOutput`), `:279` (`utf8ByteLength`), `:328` (`Progress`: adaptive commits of a changing value); exported as `@earendil-works/chord/output` (`package.json` exports)
- Opi entry: real but narrow analog: bash tool output only — `crates/opi-coding-agent/src/tool/operations.rs:1207` `finalize_complete_bash_output` + `StreamCapture` preview cap (`MAX_BASH_OUTPUT_BYTES`), spill-to-temp-file full output, `truncated` flag surfaced at `crates/opi-coding-agent/src/execution/runtime.rs:731`; `crates/opi-agent/src/tool/result.rs:19-24` exposes a generic `truncated` detail on tool results
- Comparison: partial
- Observable difference: chord's is a reusable stream library for any producer with head/tail windows and adaptive progress commits; Opi implements the behavior (bounded preview + retained full output + truncation flag) only for bash streams, hardcoded per tool, with no generic windowed buffer or Progress pacing.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: Opi's `{stdout, stderr, truncated, full_output}` shape mirrors pi-durable's bash tool contract (`{ output, truncated, fullOutputPath?, exitCode }`, `packages/durable/README.md` Tools section) — convergent tool-level design, not a chord/output port.

### CHD-08 — portable SQLite adapters + sqlite telemetry store
- Dimension(s): 6
- pi entry: `packages/chord/src/sqlite/index.ts:1-3` (`SqliteDatabase`/`SqliteExecutor`/`SqliteValue`, `applySqliteMigrations`, `validateSqliteSchemaPrefix`), `src/sqlite/node.ts` (`NodeSqliteDatabase`), `src/sqlite/cloudflare.ts` (`DurableObjectSqliteDatabase`); `src/telemetry-sqlite/` (`createSqliteTelemetry`: retention 7d, `maxQueuedSpans` 10k, metrics min/max/sum/count, `TelemetryStore` queries)
- Opi entry: absence search: no `sqlite`/`rusqlite` in any `crates/*/Cargo.toml` or `crates/*/src` (0 matches)
- Comparison: missing
- Observable difference: pi persists telemetry and host state into SQLite with migration/version-prefix invariants; Opi has no embedded database dependency anywhere (session storage is JSONL only).
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: supply-chain-relevant: pi carries a SQLite adapter family; Opi's absence is a deliberate zero-native-dep posture, recorded here as a gap only under the reproduction lens.

### CHD-09 — Context (cancellation + telemetry propagation)
- Dimension(s): 6
- pi entry: `packages/chord/src/context/index.ts:78-138` (`BACKGROUND_CONTEXT`, `TODO_CONTEXT`, `withTelemetry`, `traceIn`, `withContextValue`, `withAbortSignal`, `withCancel`, `awaitWithContext`); every chord/durable async call carries it
- Opi entry: Rust-native equivalents: `tokio` cancellation (`crates/opi-agent/src/agent.rs:502` `abort()`, `:661` cancellation reset), `crates/opi-ai/src/telemetry.rs` provider telemetry; no context-value propagation type
- Comparison: different
- Observable difference: pi threads one explicit `Context` (abortSignal + telemetry span + typed values) through every call, and adapters derive children without ambient state; Opi distributes the same duties across language-native mechanisms (Drop/cancellation tokens, per-call telemetry params), so callers cannot attach typed per-call values uniformly.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: module-layout/language difference is not itself behavioral; the observable part is the missing typed-value propagation seam.

### CHD-10 — product integration status of chord/durable within the pi snapshot
- Dimension(s): 6, 18 (scope context)
- pi entry: `packages/coding-agent/package.json:44-48` — dependencies are `pi-agent-core`, `pi-ai`, `pi-codemode`, `pi-mcp`, `pi-tui` only; **no** chord/durable/env dependency; `grep '@earendil-works/chord|pi-durable' packages/coding-agent/src` → 0. Consumers found: chord ← durable (27 src files import it) and ← env (type-only: `packages/env/src/list-files.ts:1`, `remote-env.ts:5`, `watch.ts:1`); durable ← env (`packages/env/src/errors.ts:1`, `list-files.ts:12-13`); pi-env itself consumed by **no** package. `packages/agent/CHANGELOG.md:45`: the old experimental harness was *removed* from pi-agent-core — "Use `@earendil-works/pi-durable` for durable sessions."
- Opi entry: no chord/durable analog exists at any integration level (absence searches above), so nothing is or is not wired into `opi-coding-agent`
- Comparison: equivalent
- Observable difference: none at the product level: in both trees, the shipped coding product does not consume the host-runtime/durable-execution layer; the layer exists as a standalone library surface only in pi.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: this record measures integration topology, not capability; the capability gap is recorded in the other CHD/DUR records. Durable is pi's designated successor of agent-core's removed harness, i.e. active upstream direction even though the pi 1.5.0 CLI does not consume it yet.

## DUR records — durable execution (Dimension 18)

### DUR-01 — Harness, Session, commit-before-show
- Dimension(s): 18
- pi entry: `packages/durable/src/harness/harness.ts:209` (`class HarnessImpl … implements HarnessType`), `Harness.open` with `readOnly` `:228-259`, `resume():303`; commit line + "nothing is shown before its commit is stored" `README.md:54` (Concepts); session transaction machinery `src/session/transaction.ts` (1292 lines), `src/session/writer.ts`
- Opi entry: `crates/opi-agent/src/harness.rs:1-3` "Product-neutral durable session facade" — `SessionFacade` owns ordered pending writes (`PendingWriteQueue`, `:53-118`), save points, abort-without-discard (`HarnessError::AbortLeftPending`, `:23`); writes flushed to JSONL (`crates/opi-agent/src/session.rs:459` `SessionWriter`)
- Comparison: different
- Observable difference: pi's unit of durability is the atomic commit (entries + documents + tasks together, published only after storage receipt); Opi's is a queued entry append with priority ordering — persistence happens after emission, there is no commit transaction across multiple record kinds, and in-flight model work is never committed-before-shown.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: this is the deliberate Opi overlap called out in the brief: Opi session persistence overlaps differently (transcript persistence, not durable execution).

### DUR-02 — conversations, submissions, inbox (steer/follow-up/whenBusy)
- Dimension(s): 18
- pi entry: `packages/durable/src/harness/inbox.ts`, `src/harness/submissions.ts:232-241` (requestId idempotency inside admission), `ConversationBusy`/`ConversationIdle` errors `src/errors.ts`; queue modes documented `README.md` "Busy Conversations" (steer placed after current tool round, follow-up after answer, `whenBusy: "reject"`, queued writes visible at once)
- Opi entry: in-memory analog: `crates/opi-agent/src/agent.rs:496-497` `steering_queue`/`follow_up_queue` (`Arc<Mutex<VecDeque<String>>>`), drain/restore during the loop (`agent_loop.rs:310-337`, `:1415-1525`), exposed via RPC commands `steer`/`follow_up`/`abort` (`crates/opi-coding-agent/src/rpc.rs:24-30`)
- Comparison: partial
- Observable difference: Opi steering/follow-up semantics (steer mid-run, follow-up after run) match pi's placement policy, but the queues are in-memory only: a crash loses every queued item, there is no persisted inbox document, no `whenBusy: "reject"`/`whenIdle: "reject"` admission control, no submission handles with `abort()`/`wait()`, and no idempotent `requestId`.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: —

### DUR-03 — tasks, checkpoints, scheduler, sleep/wake, crash-loop limits
- Dimension(s): 18
- pi entry: `packages/durable/src/tasks.ts:4` (`defineTask`), `src/harness/scheduler.ts` (intent commit before execute `:87-88`, replay rule `:196-208`, sleeping-state scheduling `:527-543`, `:1470`), `src/harness/types.ts:772` (`{ kind: "sleeping", until }`), `src/harness/agent.ts:58` (`DEFAULT_RESTART_LIMIT = 5`), `src/timers.ts:13-38` (host `setTimer`/`now`, Workers hibernation rules), `onWake`/`Wake` contract `README.md` "Sleeping and Waking Up" incl. the Cloudflare Durable Object example
- Opi entry: absence searches: `grep -rniE 'task.?queue|scheduler|checkpoint' crates/*/src --include='*.rs'` — `checkpoint` hits are session-file savepoints (`crates/opi-agent/src/session.rs:562-574` `checkpoint()`/`rollback_to()` truncate the JSONL to a byte offset for atomic multi-entry writes), not task checkpoints; `crates/opi-eval/src/agent/process.rs:31-142` spawns agent binaries per run and classifies spawn/exit/crash failures — no checkpoint, no resume; `crates/opi-eval/src/benchmark/process.rs:1814` configures provider retry (`max_retries: 0`), not durable scheduling
- Comparison: missing
- Observable difference: pi work is a stored state machine that survives process death (phase reruns from its top, intent committed before side effects, sleeps stored with host-visible wake times, deterministic crash loops ended at `restartLimit` with `restart_limit` outcomes); Opi work is process-resident: an interrupted run is simply lost, and Opi has no mechanism that resumes, replays, or bounds crashed work.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: opi-eval's per-run failure classification (spawn-refusal, crash, budget-timeout tokens) is the nearest scheduling-adjacent code and is explicitly not durable.

### DUR-04 — tools with external-effect guarantees (replay, nested calls, callers/modelTools, structured output)
- Dimension(s): 18
- pi entry: `packages/durable/src/harness/tool.ts` (`ToolTask`), `src/harness/scheduler.ts:196-208` (rerun only when `replay: "safe"`, else `interrupted` error result with committed output; `restart_limit` after limit, `:431`), nested `api.executeTool()` with key-reattachment and `NestedCallDoc` (`README.md` "Calling tools from tools"), `callers`/`modelTools`/`deferred` (`README.md` "Who may call a tool"), built-ins `src/tools/` (`read`, `write`, `edit`, `bash`, `powershell` per `src/tools/index.ts:29` and `src/tools/bash.ts:19`, `createBashTool` `bash.ts:155`, `createCodingTools` `index.ts:31`), three result channels (`output`/`structuredOutput`/`details`)
- Opi entry: `crates/opi-agent/src/tool.rs` (`Tool` trait, `ExecutionMode`, `ToolResult` with `content`/`details`/`is_error`/`terminate` per `src/tool/result.rs:4`); permission-gated execution `crates/opi-coding-agent/src/execution/permission.rs`; no replay policy, no nested tool calls (`grep -rniE 'executeTool|nested call' crates/*/src` → 0), no callers/modelTools split, no deferred tools
- Comparison: missing
- Observable difference: in pi a tool call's intent is durably committed before `execute()`, a crash mid-call either reruns (declared safe) or yields a committed `interrupted` result, and tools can call tools with reattachment by key; in Opi a crashed tool call is lost entirely, results have two channels not three, and there is no model-vs-tool callability control.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: Opi `details` channel and `terminate` control (`tool/result.rs`) are the convergent pieces.

### DUR-05 — persist/resume, readOnly sessions, provider session identity
- Dimension(s): 18
- pi entry: `packages/durable/README.md` "Persist and Resume" + "Failures and Recovery"; `src/harness/harness.ts:303` (`resume()`), `:228-259` (`readOnly: true` → `ReadOnlySession` on writes/resume), `src/harness/provider.ts` (per-conversation persisted UUIDv7 in `pi.provider` forwarded as `sessionId`, surviving reopen/reset/compaction), `src/harness/submissions.ts:232-241` (same `requestId` returns the existing submission after restart)
- Opi entry: `crates/opi-agent/src/session.rs:590` `read_with_recovery` + `:392` `CrashRecovery` (truncated-line recovery, corrupt/unknown entry counting, fail-closed v2 envelope `:1-38`); resume = reopen for append (`crates/opi-coding-agent/src/session_coordinator.rs:217`); `crates/opi-ai/src/anthropic.rs:1921` / `mistral.rs:388` carry a per-request `session_id` field
- Comparison: missing
- Observable difference: pi reopening storage continues unfinished runs, counts crash attempts, retries the interrupted model request as a failed attempt, and re-finds submissions by id; Opi resume reconstructs the transcript and model/thinking/label metadata (`session_coordinator.rs:876-970`) but restarts nothing — interrupted work is gone, there is no readOnly mode, no idempotent submission identity, and no documented stable per-conversation provider session id across restarts at the agent layer.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: opi-only overlap stated for contrast per brief: Opi's crash recovery is entry-level (a torn last JSONL line), pi's is run-level.

### DUR-06 — multiple conversations, forks, ownership
- Dimension(s): 18
- pi entry: `packages/durable/src/harness/harness.ts:170-171` (`fork(at, …)`), `src/session/session.ts:437-438` (`forkConversation`), ownership kinds (`ownerless`/`task`) with `ConversationRecord.owner` linking children (`README.md` "More Conversations and Forks", "Abort and Subagents"); fork keeps parent agent at the fork entry, fresh provider identity
- Opi entry: `crates/opi-agent/src/session_branch.rs:1-30` — `SessionTree::from_entries` reconstructs branches from the `parent_id` graph inside one append-only file; branch metadata (`BranchInfo` `:9`), active branch via last `Leaf` entry
- Comparison: different
- Observable difference: pi forks are independent conversation records (own id, own provider identity, own ownership edge, own documents) addressable by the harness; Opi branches are alternative paths within one session tree, selectable by tip, with no separate identity or ownership — closer to pi-agent-core's session branching than to durable forks.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: —

### DUR-07 — subagents, child tasks, abort semantics, task graph
- Dimension(s): 18
- pi entry: `packages/durable/src/harness/task-graph.ts` + `harness.ts:382-387` (`observeTaskGraph`), ownership-based abort (bottom-up, `failFast`/`allSettled` waiting, `completing` state) `README.md` "Child Tasks"/"Abort and Subagents"; worked examples `test/examples/21-subagent-foreground.ts`, `22-subagent-background.ts`, `23-child-tasks.ts`
- Opi entry: absence search `grep -rniE 'subagent|child agent|task graph' crates/*/src --include='*.rs'` → 0 relevant (the two hits are a stdout-reader task comment `crates/opi-coding-agent/src/adapter_host.rs:574` and a cache-drop test `crates/opi-ai/src/auth/anthropic_federation.rs:601`); `crates/opi-agent/src/agent.rs:502,769` `abort()` cancels only the local run
- Comparison: missing
- Observable difference: pi can own conversations/tasks by tasks, abort trees bottom-up, keep parents busy until children settle, and expose a live task graph to UIs; Opi has a single agent run with one abort handle and no delegation structure.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: —

### DUR-08 — watching a conversation (windowed view, ops, transcript merge, observers)
- Dimension(s): 18
- pi entry: `packages/durable/src/harness/view.ts` (windowed structural view: newest 50 entries, splice at 100), `src/harness/transcript.ts:51` (`ConversationTranscript.merge/loadEarlier/fillGap/resume`), synchronous observers `harness.ts:382-387+` (`observeTaskGraph`/`observeConversation`/`observeDoc`), watch = chord `WatchHandle` re-export `src/index.ts:1`
- Opi entry: `crates/opi-agent/src/event.rs:29-59` `AgentEvent` push stream (`AgentEventSink`, `subscribe` in `agent.rs`); no windowing, no ops, no late-join/reconnect contract
- Comparison: missing
- Observable difference: a pi UI attaches mid-run and receives committed state plus delta ops with defined folding/overflow, and can page earlier history through gaps; an Opi UI receives ephemeral in-memory events from subscription time and must re-read the session file for history — no delta ops exist.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: —

### DUR-09 — compaction
- Dimension(s): 18
- pi entry: `packages/durable/src/harness/compaction.ts` (`CompactionTask`), policy `README.md` "Compaction": `reserveTokens`/`keepRecentTokens`/`backgroundTokens` (0 disables background), per-model `modelOverrides`, overflow → compact-and-retry-once, `stale` settlement, `beforeCompact` hook, `pi.compaction` entry heads kept entries; defaults `DEFAULT_COMPACTION_POLICY` exported `src/index.ts`
- Opi entry: `crates/opi-agent/src/compaction.rs:13-21` `CompactionConfig { enabled, threshold_tokens: 100_000 }`, `:43-52` `CompactionOutput` with `first_kept_entry_id` + kept entries, `SummarySource::Core|Hook` (`:36-40`), reasons incl. overflow via `crates/opi-agent/src/session_event.rs` `CompactionReason`; exactly-once settlement guarded in `agent_loop.rs:1650`; compaction persisted as session entries (`session.rs:157` `CompactionEntry`)
- Comparison: partial
- Observable difference: both compact by summarizing older entries into a persisted compaction record heading kept entries, with hook-supplied summaries; Opi lacks background compaction, per-model overrides, context-window-aware thresholds, retry-after-overflow semantics, and compaction-as-durable-task (an interrupted Opi compaction is settled in memory, not resumed).
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: —

### DUR-10 — extensions, registry reload, hooks, system-prompt sections, afterRun
- Dimension(s): 18
- pi entry: `packages/durable/src/harness/registry.ts` (`createRegistry`, install/uninstall replaces in place), `src/harness/define.ts` (`defineExtension`, `defineTool`, `hook`, `section`, `wrapTool`, `runEndTask`), generation/tool hook set (`README.md` "Hooks": `beforeRequest`, `beforeProviderRequest`, `afterResponse`, `onYield`, `afterTools`, `afterRun`, `beforeTool`, `afterTool`), stored-by-name selection per conversation
- Opi entry: `crates/opi-agent/src/extension.rs` (`Extension`, `ExtensionRegistry`, `ExtensionCommand`), `crates/opi-agent/src/hooks.rs:12-127` (`BeforeToolCallContext/Result`, `AfterToolCall…`, `ShouldStopAfterTurn`, `PrepareNextTurn`, `InitialTurn`, `PreparationCommit`)
- Comparison: partial
- Observable difference: Opi covers before/after tool calls and turn-boundary hooks in process; it has no runtime install/uninstall-replace (registry is fixed at startup), no system-prompt section model, no tool wrappers, no `afterRun` durable work, and extension selection is not per-conversation state.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: —

### DUR-11 — per-conversation agent + settings
- Dimension(s): 18
- pi entry: `packages/durable/src/harness/agent.ts` (`AgentDoc`, `configure`, `DEFAULT_RETRY_POLICY`, `DEFAULT_PROGRESS_POLICY`), stored `pi.agent` document resolved against the registry at each use, `maxTurns` per conversation (`RangeError` guard), live `settings` getters `README.md` "Settings"
- Opi entry: persisted per-session metadata entries: `crates/opi-agent/src/session.rs:222` `ModelChangeEntry`, `:246` `ThinkingLevelChangeEntry`, `:256` `LabelEntry` — written through `crates/opi-coding-agent/src/session_coordinator.rs:907-970` "so a later resume observes the recorded model"; turn limit `crates/opi-agent/src/loop_types.rs:164` ("max turns exceeded"); usage in `session_event.rs:77`; live settings via `crates/opi-coding-agent/src/config.rs`
- Comparison: partial
- Observable difference: Opi persists model/thinking-level/label choices as session entries and enforces a turn limit, matching the observable "conversation remembers its model" behavior; it lacks per-conversation extension/tool selection, cwd-per-conversation, stored agent documents, retry/progress/restart policy knobs, and settings liveness via getters.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: —

### DUR-12 — code mode (journal-replayed scripting) and tool_search
- Dimension(s): 18
- pi entry: `packages/durable/src/codemode/codemode.ts:255` (`createCodemodeExtension`), deterministic-restart journal documented at `codemode.ts:13-34` (`pi.codemode.journal`, divergence ends the call); `src/tool-search/tool-search.ts:91` (`createToolSearchExtension`, `control.addTools` recording into `modelTools`)
- Opi entry: absence search `grep -rniE 'codemode|tool.?search|deferred tool' crates/*/src --include='*.rs'` → 0
- Comparison: missing
- Observable difference: pi conversations can expose a sandboxed script tool whose execution survives restarts by journal replay, and can scale to huge tool sets via deferred loading; Opi offers neither.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: pi-codemode itself is a separate package (other batch); this record covers only the durable-side extension.

### DUR-13 — storage backends, StoreStorage, conformance, telemetry schema
- Dimension(s): 18
- pi entry: `packages/durable/src/storage/` — `memory.ts`, `indexeddb.ts`, `sqlite/` (node + cloudflare, WAL `synchronous=NORMAL`, table prefixes), `jsonl/` (fsync option), `store/storage.ts:92` (`StoreStorage implements Storage` over a chord v2 store, `:117` `open`), receipt-based `commit(batches, receive, …)` contract (`README.md` "Storage"); `src/testing/index.ts:4` (`registerStorageConformance`, `registerEnvConformance`); `src/harness/telemetry.ts` (`DURABLE_TELEMETRY_SCHEMA`: `pi.durable.submit/phase/report/entry/submission/checkpoint/replay`, metrics)
- Opi entry: `crates/opi-agent/src/session.rs:459` `SessionWriter` (append-only JSONL + fsync, `read_with_recovery`); no pluggable storage trait, no second backend, no conformance suite, no durable telemetry schema (`crates/opi-ai/src/telemetry.rs` covers provider-side only)
- Comparison: missing
- Observable difference: pi storage is a swappable backend contract with atomic batches, durability receipts, five implementations, and a conformance suite any custom backend can run; Opi session storage is one hardcoded JSONL format with entry-level crash recovery, and its telemetry does not model task phases/checkpoints.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: —

## Absence-search commands (exact)

All run from Opi root (`D:\Luiz\Odradek\opi`), `grep -rniE … crates/*/src --include='*.rs'` (`rg` unavailable in this shell; `grep -E` covers the same patterns):

- `grep -rniE 'chord|hot.?reload|replicated|durable|checkpoint|task.?queue|exactly.?once' crates/*/src --include='*.rs'` → 61 files; per-term counts: `chord` 0, `hot.?reload` 0, `replicated` 0, `task.?queue` 0, `facet` 0, `durable` 152, `checkpoint` 43, `exactly.?once` 31 (+1 `exactly_once`).
- Classification: every `durable`/`checkpoint` hit is session-JSONL persistence (opi-agent session.rs/harness.rs/session_context.rs docs and `SessionWriter::checkpoint`, a byte-offset savepoint) or eval/benchmark prose ("durable evidence manifest", "durable cleanup evidence"); every `exactly.?once` hit is prose about single-settlement or dedup behavior (e.g. `agent_loop.rs:1650` compaction settles exactly once) — no execution-semantics mechanism. No real chord/durable analog was found in any of the 61 files beyond those cited in DUR-01/02/05/09/11.
- `grep -rn 'rusqlite|sqlite' crates/*/Cargo.toml Cargo.toml` → 0; `grep -rniE 'sqlite' crates/*/src` → 0.
- `grep -rniE 'codemode|tool.?search|subagent|executeTool|group.?commit' crates/*/src --include='*.rs'` → 0 relevant matches.
- Focused inspection per brief: `crates/opi-agent/src/session.rs`, `session_branch.rs`, `harness.rs`; `crates/opi-eval/src/agent/process.rs`, `benchmark/process.rs` — findings recorded in DUR-03/DUR-05/DUR-06.

## Coverage honesty

- I enumerated both packages' full root export lists (`src/index.ts`) and all subpath exports, but did **not** read all ≈213K source lines. Per-file reading was limited to: both READMEs (fully), chord `index.ts`, `contract.ts`/`host.ts`/`link.ts`/`output.ts`/`context/index.ts`/`state.ts`/`registry.ts`/`reload.ts`/`watch.ts`/`wire.ts` headers and key functions, `next/index.ts`, both `docs/spec.md` headers + section lists, `docs/next/design.md` + `slices.md` + slice status lines, `group-commit.md` header; durable `harness/` key files (harness, scheduler, agent, submissions, tool, view, transcript, types, compaction headers), `session/session.ts` excerpts, `storage/store/storage.ts` exports, `tasks.ts`, `documents.ts`, `codemode/codemode.ts` journal header, `tools/`, `testing/index.ts`, `timers.ts`.
- pi tests were counted and named, not executed (no installs allowed); maturity judgments rest on file presence + README/spec claims.
- The `152 durable` / `43 checkpoint` / `31 exactly.?once` Opi hits were classified by sampling representative files (session.rs, harness.rs, agent_loop.rs, eval process files), not by reading all 61 files line by line.
- chord `src/next` internals (jobs/store/documents layers) were measured by module inventory, export surface, and design-doc statuses, not by reading the ≈28.6K lines; slice-level built/ready/draft claims come from `docs/next/slice-*.md` status lines.
- Environment (`pi-env` / durable `env`) execution environments, images/Photon pipeline, IndexedDB/JSONL storage internals, telemetry-sqlite internals, and pi-codemode itself are out of this batch's scope (env/codemode belong to other packages' batches); only their durable/chord-facing seams were checked.
- I did not run any pi code, consult `docs/opi-spec.md`, `docs/CONTEXT.md`, snapshots, ADRs, or other `docs/realign/` files beyond this evidence directory.
