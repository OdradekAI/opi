# Realignment Audit Framework

## Evidence Pass

Collect evidence from both projects before judging drift:

- guidance files: `AGENTS.md`, `CLAUDE.md`, `CONTRIBUTING.md`;
- product docs: README, changelog, docs/specs, ADRs, design notes;
- manifests: workspace files, package manifests, dependency catalogs;
- source topology: packages/crates/modules, binaries, public library surfaces;
- tests and fixtures: contract tests, snapshots, integration tests, mocks;
- roadmap artifacts: phases, PRDs, implementation ledgers, issue lists.

Prefer local evidence over memory. For changing or remote facts, browse or use
the relevant source of truth only when required by the user or task.

## Comparison Dimensions

| Dimension | Questions |
|---|---|
| Product intent | What user workflows does each project optimize for? What is explicitly out of scope? |
| Package boundaries | Which package owns provider/runtime/UI/config/session/tool/plugin concerns? |
| Runtime flow | How do requests, streaming events, tool calls, hooks, cancellation, and retries move through the system? |
| Data formats | What config, session, trace, RPC, cache, or package formats are persisted or exposed? |
| Extension model | Are plugins/extensions in-process, subprocess, dynamic language runtime, RPC, or static compile-time surfaces? |
| Provider/integration model | How are providers registered, authenticated, streamed, tested, diagnosed, and extended? |
| UI model | Which product UI surfaces are core, which are extension surfaces, and which are future ecosystem work? |
| Testing contract | Which behaviors are contract-tested, snapshot-tested, or left as manual workflows? |
| Operations/security | How are credentials, redaction, local files, shells, sandboxing, telemetry, and diagnostics handled? |
| Roadmap/phases | Do planned phases deepen existing seams or chase breadth before foundations are stable? |

## Drift Taxonomy

| Level | Meaning | Typical action |
|---|---|---|
| Aligned | Current behavior matches target semantics or accepted design intent. | Preserve and add regression tests if important. |
| Intentional divergence | Difference is justified by language, runtime, product scope, or explicit non-goal. | Document it and guard against false parity claims. |
| Partial | A seam or subset exists but does not yet cover target semantics. | Deepen the seam before adding new breadth. |
| Missing | Target capability exists and is relevant, but current project lacks it. | Add to roadmap or explicitly defer. |
| Overreach | Current project adds target-adjacent scope that is not justified. | Remove, defer, or move to extension/package layer. |
| Risk | The implementation is in the wrong layer or could block future alignment. | Prioritize architecture adjustment. |

## Evidence Discipline

- Use line-specific citations for consequential claims.
- Mark inference separately from documented evidence.
- Avoid treating examples, demos, or package samples as core product behavior.
- Check changelogs before claiming a capability is current.
- If the target has broader ecosystem features, separate ecosystem parity from
  core semantic alignment.
