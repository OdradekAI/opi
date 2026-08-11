# Opi Minimum-Change Trace Admission Design

**Date:** 2026-08-11

**Status:** Approved

**Scope:** `.claude/skills/opi-implement` plan admission and task handoff

## Problem

`opi-implement` already records criterion coverage, production call sites,
plugin-first placement, public test seams, and inferred decisions. Those facts
are distributed across the task graph and reviewer prose, so a human cannot
quickly verify that an admitted task is the smallest justified change.

The admission gate needs one compact trace that answers six questions without
introducing another planning artifact or expanding the ledger schema.

## Decision

Use the existing schema-v2 task fields as the canonical minimum-change trace.
Do not add a `minimum_change_trace` object and do not bump `schema_version`.

Four answers use standardized `inference_notes` entries. Criterion ownership
and the production vertical slice continue to use the existing acceptance,
call-site, and verification fields. Missing is not an answer; explicit `none`
or `not-applicable` is valid when justified.

| Admission question | Canonical evidence |
|---|---|
| Which registered criterion or acceptance scenario does this work serve? | `acceptance_scenarios[].id` and `.source`; for a substrate task, the later scenario-owning task whose transitive `depends_on` closure contains it, rendered through the P.4 acceptance coverage table |
| Were existing helpers, runtime seams, packages, or protocols searched and reused? | `inference_notes[field = "reuse_search"]` |
| Why can the work not live completely in an existing plugin or package? | `inference_notes[field = "placement"]` |
| Is each new public API, config item, state field, and dependency edge necessary? | `inference_notes[field = "surface_necessity"]` |
| What is the smallest vertical slice that proves the production call chain? | `acceptance_scenarios[].verification`, `acceptance_scenarios[].production_call_sites`, task-level `production_call_sites`, and `verification.behavioral_tests` |
| If a simplification is accepted, what is its ceiling and observable revisit trigger? | `inference_notes[field = "simplification_ceiling"]`, including `revisit_when` |

## Standard Note Contract

Keep the existing `{ field, reason, source }` note shape. Standardize only the
`field` values and the key-value clauses inside `reason`:

```json
{
  "field": "reuse_search",
  "reason": "searched=<symbols/paths/packages/protocols>; reused=<items|none>; gap=<smallest missing capability>",
  "source": "<registered source heading or repository evidence>"
}
```

```json
{
  "field": "placement",
  "reason": "target=<core|extension|plugin|package>; existing_home=<id|none>; cannot_fit_fully=<reason|not-applicable>",
  "source": "<registered source heading>"
}
```

```json
{
  "field": "surface_necessity",
  "reason": "public_api=<none|necessity>; config=<none|necessity>; state=<none|necessity>; dependency_edge=<none|necessity>",
  "source": "<registered source heading or repository evidence>"
}
```

```json
{
  "field": "simplification_ceiling",
  "reason": "accepted=<none|simplification>; ceiling=<known limit>; revisit_when=<observable condition>",
  "source": "<registered source heading or reviewed decision>"
}
```

`revisit_when` must be observable: a missing capability encountered in a named
workflow, a measured threshold, a newly supported platform, or a concrete
failure mode. A calendar guess or “when needed” is not sufficient.

If an existing plugin/package can own the work completely, `placement` says so
and no core seam is admitted. If no new public/config/state/dependency surface
is needed, every `surface_necessity` clause says `none`; the gate does not
reward inventing a surface to fill the trace.

## Applicability

The trace is evaluated for each executable draft task and for the graph's
criterion coverage as a whole.

- A product or vertical-slice task owns at least one sourced acceptance
  scenario and must identify the production entry point and behavioral proof.
- A pure helper/parser/protocol task may keep `acceptance_scenarios = []` only
  when `substrate_only = true`; P.4 must show the later scenario-owning task.
  Derive that owner through the graph's `depends_on` closure. An orphan
  substrate task fails admission. It must not invent a production caller
  merely to fill the trace.
- A documentation-only task may answer the production-slice question as
  `not-applicable` and use its documentation-contract verification. It still
  needs a registered source and explicit surface/placement answers.
- `none` and `not-applicable` require a reason. Empty notes, absent clauses, or
  generic assertions such as “reuse considered” fail admission.

## Workflow Changes

### P.1 Draft Construction

Populate the four standardized notes while deriving the task graph. Reuse the
existing source and repository evidence already read during source admission;
do not start a second research workflow solely to fill the trace.

### P.2 Adversarial Admission

The plan reviewers check all six answers explicitly:

- design readiness checks reuse search, placement, surface necessity, and the
  simplification ceiling;
- execution readiness checks criterion ownership and the demonstrable
  production vertical slice.

An omitted answer with otherwise sufficient source material routes to
`GRAPH_REVISION_REQUIRED`. Missing facts route to `RESEARCH_REQUIRED`.
Unsettled placement, public surface, or simplification decisions route to
`DESIGN_DECISION_REQUIRED`.

### P.4 Human Graph Gate

Render a six-row minimum-change trace for each task beside the existing task
and acceptance coverage tables. Refuse `confirm-all` when a required answer is
missing, a surface note omits one of its four clauses, a simplification note
omits `revisit_when`, or a runtime claim lacks either a production call site or
behavioral verification.

The human confirms the graph once. The trace is evidence for that existing
gate, not a seventh gate.

### Phase B Handoff

Before the existing proceed/commit question, print the selected task's six
answers. Phase B does not reinterpret them. If implementation discovery would
add an unadmitted API, config item, state field, dependency edge, or placement
change, stop and return to graph review; the Phase C `task_owned_paths`
append-only exception remains the only in-task const-field mutation.

## Rollout and Compatibility

This is an admission contract, not a schema migration.

- Do not rewrite `.opi-impl-state.json` as part of this change.
- Already-confirmed, no-drift task graphs are not invalidated retroactively.
- The next init, reconcile, draft import, or graph edit must produce the full
  trace before confirmation.
- Phase B may label absent entries on a pre-contract task as
  `legacy-unrecorded`; it must not fabricate answers. Once that graph returns
  through the plan path, the exemption ends.
- Existing schema-v2 readers and ledger-guard logic remain unchanged.

This rollout avoids changing task meaning or creating ledger churn while work
is in progress. It also ensures the rule becomes mandatory at the next normal
admission boundary.

## Files to Change

- `.claude/skills/opi-implement/skill.md`: state the rule and add the Phase B
  rendering requirement.
- `.claude/skills/opi-implement/references/initializer.md`: define P.1
  construction, P.2 rejection, and P.4 rendering/refusal behavior.
- `.claude/skills/opi-implement/references/ledger-schema.md`: define the four
  standardized note semantics and validation rules without a schema bump.
- `.claude/skills/opi-implement/references/verify-engine.md`: map the six
  questions onto the existing design/execution readiness lenses.
- `.claude/skills/opi-implement/scripts/plan.workflow.js`: make the applicable
  lens charters inspect the trace explicitly.
- `scripts/opi-doc-check.py` and `scripts/test_opi_doc_check.py`: extend the
  existing skill-contract check so the required field names and cross-file
  obligations cannot silently drift.

No Rust code, Cargo metadata, registered phase source paths, or canonical
ledger content changes.

## Verification

Test impact: `update` the existing documentation-contract tests; no Rust tests.

Run:

```text
python -m unittest scripts.test_opi_doc_check
python scripts/opi-doc-check.py
node --check .claude/skills/opi-implement/scripts/plan.workflow.js
git diff --check -- .claude/skills/opi-implement/skill.md
git diff --check -- .claude/skills/opi-implement/references/initializer.md
git diff --check -- .claude/skills/opi-implement/references/ledger-schema.md
git diff --check -- .claude/skills/opi-implement/references/verify-engine.md
git diff --check -- .claude/skills/opi-implement/scripts/plan.workflow.js
git diff --check -- scripts/opi-doc-check.py scripts/test_opi_doc_check.py
```

The tests must fail when any standardized field is removed from its required
skill/reference location and pass only when the admission, rendering, ledger
semantics, and reviewer charters remain synchronized.

## Rejected Alternatives

### Add `tasks[].minimum_change_trace`

Rejected because it duplicates acceptance and call-site data, requires a
schema bump and migration, and creates two sources of truth.

### Store the six answers only in reviewer prose

Rejected because prose cannot be rendered consistently at P.4 or handed off
reliably at Phase B.

### Rewrite the current ledger immediately

Rejected because it would alter an already-confirmed graph outside the plan
path and risk colliding with active implementation state.
