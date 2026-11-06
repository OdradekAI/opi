# Batch CHD/DUR — chord host runtime & durable execution (Dims 6,18 + Layer H)

pi side:
- `packages/chord` (122K loc): typed host runtime — objects with principals,
  references, hot reload, replicated JSON state + deltas, contracts
  (local/remote/method/opaque), createHost/defineFacet, links
  (memory/ipc/websocket), delta transactional tracker, output bounds, sqlite
  adapters, telemetry-sqlite. Read README fully; enumerate exports from
  src/index.ts.
- `packages/durable` (91K loc): durable conversation/task/document runtime —
  commit-before-show, persist/resume with failed-attempt retry, extensions,
  tools, code mode, per-conversation agent, settings, reload, watching, busy
  conversations, reset/handoff, compaction, agent events, hooks, forks,
  abort/subagents. Read README fully; enumerate exports from src/index.ts.

Steps:
1. Enumerate public exports of both packages (src/index.ts).
2. Bounded absence searches in opi: `rg -li
   'chord|hot.?reload|replicated|durable|checkpoint|task.?queue|exactly.?once'
   crates/*/src` — classify word-collision vs real analog; inspect
   opi-agent session.rs/session_branch.rs and opi-eval process/benchmark for
   durable-scheduling analogs.
3. Integration status inside the snapshot: coding-agent package.json does NOT
   depend on chord/durable/env (verify); find whether any product surface
   consumes durable (`rg 'pi-durable' across snapshot`).
4. Layer H: record authority wording (durable README: "Experimental. The API
   changes without notice"; chord README status) and observed maturity from
   source/tests (implemented/partial/scaffold).
5. Per major capability — chord: contracts, hosts/facets, links/IPC,
   delta/state replication, hot reload, principals, output bounds, sqlite;
   durable: conversations, tasks/checkpoints, tools with external-effect
   guarantees, resume/replay, forks, subagents, watching, compaction,
   extensions — write a record: pi anchor + opi absence/analogy + verdict
   (likely `missing`; `opi-only` where opi session persistence overlaps
   differently — describe opi-agent session.rs persistence semantics for
   contrast).

Output `evidence/chord-durable.md`, prefixes `CHD`, `DUR`. Tallies + coverage
honesty; exact absence-search commands.
