---
name: opi-workflow
description: Recommend the next Opi workflow for an explicitly requested routing decision.
disable-model-invocation: true
---

# Opi Workflow Router

Return one next action, its owning source and any unmet entry gate. Read only
the selected skill before making claims about its behavior. Routing does not
invoke that skill, edit files or create another delivery ledger.

## Design direction

Inward evidence compares a pinned pi revision with Opi; outward evidence studies
external capabilities. Human shaping turns evidence into reviewed requirements.
`opi-implement` alone owns the implementation ledger and delivery state machine.

## Routes

| Requested outcome | Next action |
|---|---|
| Compare Opi with pi's design or implementation | `opi-realign` with an exact revision |
| Investigate an external capability | `opi-research` |
| Resolve unsettled product meaning | Human-led shaping; optional adapters below |
| Admit a reviewed, registered source and task graph | `opi-implement plan` |
| Execute an admitted task | `opi-implement` |
| Diagnose a hard bug or regression | Evidence-led diagnosis, then the owning repair/delivery route |
| Audit a completed Phase | `opi-audit` |
| Verify and optionally fix normalized findings | `opi-remediate` |
| Run native benchmarks or fidelity cases; repair confirmed eval findings | `opi-eval` |
| Audit/synchronize product documentation | `opi-document` |
| Publish a verified release | `opi-release` |
| Reduce redundant integration-test binaries | `opi-slim-tests` |

A bounded ordinary edit can proceed under AGENTS.md when no specialized
workflow or product-admission gate applies. Do not add lifecycle steps merely
to complete the diagram. Direct explicit invocation of any Opi skill is valid.

## Matt adapter contract

Use these optional shaping adapters only when installed and requested:

- `wayfinder`: a broad, multi-session decision space.
- `grill-with-docs`: bounded ambiguity.
- `to-spec`: materialize settled decisions into a non-normative candidate.
- `diagnosing-bugs`: evidence-led diagnosis, without authorizing delivery.

Before recommending tracker-backed `wayfinder` or `to-spec`, check
`docs/agents/issue-tracker.md`. If absent, identify the missing adapter and
offer explicit `setup-matt-pocock-skills` only if available. Human-led shaping
can proceed in the current conversation without installing a tracker or skill.
Never claim an unavailable command works or silently substitute another package.

Bind domain modeling to `docs/CONTEXT.md` and ADRs to `docs/adr/`.
Tracker artifacts remain evidence until human review materializes and registers
a normative source. Optional adapters cannot bypass that admission.

## Phase boundaries

- Evidence does not authorize product decisions or implementation.
- Plan admission checks readiness; missing meaning returns to human shaping.
- Audit diagnoses without fixes. Eval repairs use only its
  [user-confirmed source-repair branch](../opi-eval/SKILL.md#user-confirmed-source-repairs).
- Documentation does not authorize release. Source repair, live trial budgets,
  graph approval, commit permission and irreversible publication are distinct
  gates owned by their respective skills.
- Preserve existing authorization for the same scope/object; stop only at an
  unmet gate, changed approval binding or material ambiguity. Explain the exact
  source of a blocking rule and the concrete next action.
