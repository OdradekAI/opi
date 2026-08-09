---
name: opi-workflow
description: Route opi work among inward pi realignment, outward capability research, human-led shaping, delivery, assurance, documentation, and release without creating a second state machine.
disable-model-invocation: true
---

# Opi Workflow Router

Route work to the skill that owns the next decision or artifact. This skill is
an index, not an orchestrator: open the selected skill before making any
load-bearing claim, and stop at human decision and irreversible boundaries.

## Design direction

- **Inward:** pi is opi's design reference. Use `opi-realign` to measure the
  latest pi direction against the Rust implementation and selected opi scope.
- **Outward:** optional capabilities beyond or poorly served by pi are researched
  for plugin/package placement before expanding core. Use `opi-research`.
- **Implementation:** `.opi-impl-state.json` and `opi-implement` are the only
  delivery state machine. Do not introduce another ticket, plan, worktree, or
  commit protocol inside it.

## Routes

| Situation | Open and use |
|---|---|
| Compare current opi with the latest pi design or implementation | `opi-realign` |
| Investigate an external or non-pi capability | `opi-research` |
| Resolve a large, multi-session decision space with fog | Recommend explicit Matt `wayfinder`, after the tracker adapter check below |
| Sharpen bounded ambiguity in the current session | Recommend explicit Matt `grill-with-docs`, with the opi glossary adapter below |
| Synthesize settled decisions when no reviewed spec exists | Recommend explicit Matt `to-spec`, producing a non-normative tracker candidate |
| Admit a reviewed, registered source and construct its task graph | `opi-implement plan` |
| Execute an admitted ledger task | `opi-implement` |
| Diagnose a hard bug or performance regression | Matt `diagnosing-bugs`, then return to the owning delivery route |
| Audit a completed phase | `opi-audit` |
| Validate and optionally fix normalized findings | `opi-remediate` |
| Gather release-candidate runtime evidence | `opi-eval` |
| Synchronize product documentation | `opi-document` |
| Publish a verified release | `opi-release` |
| Reduce integration-test binary count | `opi-slim-tests` |

Direct invocation of any routed skill is valid. Do not call a skill merely to
make the route look complete: skip `to-spec` when wayfinding already produced a
reviewed spec, and skip assurance or release work whose entry gate is not met.

## Matt adapter contract

- `wayfinder`, `grill-with-docs`, `to-spec`, and
  `setup-matt-pocock-skills` are user-invoked Matt skills. Return the exact
  recommended invocation and stop; do not claim to invoke or compose them from
  this router.
- Before recommending `wayfinder` or `to-spec`, require
  `docs/agents/issue-tracker.md`. If it is absent, recommend explicit Matt
  `setup-matt-pocock-skills` and stop until the tracker is configured. Do not
  invent tracker conventions inside an opi skill.
- When `wayfinder` or `grill-with-docs` invokes Matt `domain-modeling`, bind its
  glossary target to opi's existing `docs/CONTEXT.md` and its ADR target to
  `docs/adr/`. Do not create a competing root `CONTEXT.md`.
- A wayfinder map, decision ticket, or `to-spec` tracker issue is shaping
  evidence, not an `opi-implement` source. Human review must materialize the
  settled result into `docs/opi-spec.md` or a supplemental spec registered by
  `opi-implement`; only then may `opi-implement plan` admit it.

## Phase boundaries

- Research and realignment produce evidence, not product decisions.
- Human-led shaping may return to either evidence path repeatedly.
- `opi-implement plan` is an adversarial admission check, not a shaping tool.
- `opi-implement` cannot silently amend a normative source when meaning is
  missing or wrong; it returns to the owning shaping artifact.
- Audit does not fix, eval does not fix, documentation does not release, and
  release does not cross an irreversible boundary without explicit approval.

When a required routed skill is unavailable, stop with a setup error. Do not
silently substitute a similarly named workflow from another skill package.
