---
name: opi-workflow
description: Recommend one next action for current Opi work or an explicitly selected legacy workflow.
disable-model-invocation: true
---

# Opi Workflow Router

Return one next action, its owning source and any real unmet condition. Read
only the selected skill before making claims about it. Routing does not invoke
that skill, edit files or create another delivery ledger.

## Routes

| Requested outcome | Next action |
|---|---|
| Build a clear feature or repair ordinary code | Follow [task-scoped context](../../../docs/agents/context.md); inspect, implement and verify |
| Resolve bounded product ambiguity | Clarify with the user; record meaningful terms/decisions at their owner |
| Map a large unresolved multi-session decision space | Requested Matt `wayfinder`, using the adapter below |
| Compare Opi with pi, including gap-closing advice and acceptance | `opi-realign` with the supplied snapshot path |
| Research external designs and current Opi counterparts | `opi-research` |
| Diagnose a hard bug or regression | Evidence-led diagnosis, then the user-agreed repair |
| Resume a specific legacy Phase ledger/graph | Read the legacy boundary, then `opi-implement` only if that contract is intended |
| Audit a legacy Phase | Legacy boundary, then `opi-audit` |
| Repair findings from a legacy indexed audit set | Legacy boundary, then `opi-remediate` |
| Run native benchmarks/fidelity cases or repair confirmed eval findings | `opi-eval` |
| Audit/synchronize documentation | `opi-document` |
| Publish a verified release | `opi-release` |
| Reduce redundant integration-test binaries | `opi-slim-tests` |

New features do not need global spec revision, Phase registration or
`opi-implement plan`. A clear user request may be sufficient; multi-session work
uses a focused brief with acceptance and unresolved decisions. Do not route a
new task through a retired Phase merely because a ledger exists.

## Matt adapter contract

For requested Matt flows, read [tracker configuration](../../../docs/agents/issue-tracker.md)
and [domain configuration](../../../docs/agents/domain.md). They select
`docs/iterations/`, `docs/GLOSSARY.md` and `docs/adr/`, including the explicit
translation of Matt's root-glossary and `.scratch/` planning defaults.

Check availability and read the actual skill before recommending it. `ask-matt`
is a router; `wayfinder` resolves decisions; `grill-with-docs` sharpens bounded
ambiguity; `to-spec` captures settled scope. Their outputs do not automatically
approve implementation. Do not invoke setup just because the default root
files are absent; this project intentionally keeps them in `docs/`.
If an installed tool cannot honor the adapter, report the mismatch rather than
creating duplicate contexts. Missing optional tools do not block conversation.

## Boundaries

- [Legacy Phase procedures](../_shared/references/legacy-phase-workflows.md)
  retain their selected-source, ledger, review and digest gates only for explicit
  legacy work. Missing historical inputs do not block a new independent task.
- Research supplies evidence, not permission. Confirm material product meaning
  with the user and record the result in the selected task/decision.
- Eval source repair, paid trials, commits and irreversible publication are
  separate authorizations. Documentation does not authorize release.
- Preserve existing authorization for the same scope/object. Explain the exact
  conflict and concrete next action instead of adding lifecycle ceremonies.
