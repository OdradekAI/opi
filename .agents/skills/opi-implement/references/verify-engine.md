# Verify Engine Reference

Read at three lifecycle points: plan admission P.2, task verification Phase D.2,
and phase-exit verification Phase F.1a. The stages share evidence and
adversarial-verification conventions, but their authority differs.

## Hard boundaries

- **Plan** reviews a draft and returns a routing verdict. It never edits the
  source or draft and never writes the canonical ledger.
- **Exec** produces must-fix findings that block Phase D and route to Phase C.
  It never edits code.
- **Phase exit** produces not-met findings that block archive. It never rewrites
  a success criterion or accepts an uncited deferral.
- Re-running a stage to erase a gate it already fired is forbidden.
- No stage silently confirms a graph, passes a task, or archives a phase.

## Inputs

- **plan** — registered source paths, the original draft task array, current
  phase, and reported reviewer independence;
- **exec** — task object, HEAD commit, and registered phase source;
- **phase-exit** — F.1a `criteria_trace[]`, completed phase tasks, registered
  phase source, and phase.

## Dispatch modes

Plan dispatch is capability-sensitive, not based on section hashes that the
ledger does not store:

- when the runtime supports bounded agents, invoke
  `scripts/plan.workflow.js` and limit concurrent lenses to available slots;
- otherwise one fresh reviewer runs the same charters sequentially against the
  same original draft;
- use another model family when available; otherwise report
  `fresh-context-same-family` degraded independence;
- both paths return the same result schema.

Exec remains risk-gated: `evaluator_required = true` uses the seven-lens Workflow
in `scripts/exec.workflow.js`; other tasks skip exec review because their
D.0/D.1/D.3 proof is deterministic. Phase exit always uses
`scripts/phase-exit.workflow.js` once per phase.

## Common finding rules

Every finding contains:

```json
{
  "lens": "<lens-key>",
  "task_id": "<string at plan/exec; null at phase-exit>",
  "criterion_id": "<string at phase-exit; null at plan/exec>",
  "decision_id": "<stable shared decision id or null>",
  "field": "<field or subject>",
  "problem": "<falsifiable problem>",
  "severity": "high | medium | low",
  "suggested_fix": "<bounded correction or next decision>",
  "source_citation": "<file>#<heading> | <file> §<section>",
  "confidence": "high | medium | low"
}
```

Plan findings additionally carry:

```json
{
  "axis": "design-readiness | execution-readiness",
  "route": "RESEARCH_REQUIRED | DESIGN_DECISION_REQUIRED | GRAPH_REVISION_REQUIRED",
  "blocking": true
}
```

`source_citation` must match `<file>#<heading>` or `<file> §<section>`, and the
emitting reviewer verifies that the cited heading exists. An adversarial
verifier tries to reject each blocking or must-fix finding. Uncertain verifier
results are rejected rather than used to mutate or block state.

Use `decision_id` only for a declared `shared_decision`; ordinary findings use
`null`. Accepted exec decision findings are copied into the task attempt's
`session_notes[].gate_results` so recurrence is detectable across active phase
tasks.

For exec and phase exit, high findings and medium/high-confidence findings enter
the blocking verification set; other findings are reported for human review.
Plan does not use a fold matrix: every surviving finding is reported and no
finding is auto-applied.

## Plan admission stage

The plan review has two non-collapsible axes.

### Design readiness

| Lens | Checks |
|---|---|
| P-D1 Pi lineage and Rust divergence | The source preserves pi's design direction or records why a Rust-native divergence is intentional. |
| P-D2 Plugin-first placement | Optional/non-pi capability defaults to plugin/package; `reuse_search` names what was inspected and `placement` proves any core change is the smallest missing extension seam. |
| P-D3 Domain and interface clarity | Terms agree with `docs/CONTEXT.md`; modules expose deep interfaces at explicit seams. |
| P-D4 Acceptance/test seam | User-visible behavior, its registered scenario source, production call sites, and the highest practical public behavioral test seam are explicit. |
| P-D5 Source completeness | Problem, solution, out-of-scope, success, exit, evidence provenance, `surface_necessity`, and any `simplification_ceiling` are present without contradictions or silent assumptions. |

Route missing facts to `RESEARCH_REQUIRED`. Route unresolved product,
architecture, terminology, placement, or seam decisions to
`DESIGN_DECISION_REQUIRED`.

### Minimum-change trace overlay

The existing lenses jointly inspect the six answers; this does not add a new
lens or verdict. Design readiness verifies `reuse_search`, `placement`,
`surface_necessity`, and `simplification_ceiling`, including an observable
`revisit_when`. Execution readiness verifies the sourced criterion/scenario,
the smallest production vertical slice, and the later scenario owner for each
substrate task.

When the draft claims an existing surface is unused, duplicate, or superseded,
or proposes deletion, merging, replacement, or dependency substitution, the
same lenses also inspect `production_consumers`, `nonproduction_consumers`,
`net_deletion`, and `residual_glue`. They verify that production use is not
inferred from tests, docs, or examples and that the stated deletion accounts
for newly introduced glue. Every new or reconciled draft declares
`simplification_trigger`; `none` leaves these subclauses unnecessary, while a
listed non-`none` trigger requires them. The cross-platform
`scripts/validate-plan.py` gate rejects missing, duplicated, or malformed
declared fields on non-archived tasks before graph confirmation; reviewers
still assess whether the cited evidence is true.

When the shared-decision trigger fires, P-D3 also verifies one stable decision
identity, exactly one owner, one typed Interface and representation, agreeing
production consumers, and explicit temporary-path closure. P-D4 verifies that
the owner's closure test exercises that Interface and that the planned test
impact follows replace-don't-layer. The deterministic validator checks the note
grammar and graph relationships; reviewers check repository truth.

If the source is sufficient but the draft omits or malforms an answer, return
`GRAPH_REVISION_REQUIRED`. Missing facts return `RESEARCH_REQUIRED`.
Unsettled placement, surface, or simplification choices return
`DESIGN_DECISION_REQUIRED`. A triggered claim with sufficient source evidence
but missing conditional subclauses returns `GRAPH_REVISION_REQUIRED`.

### Execution readiness

| Lens | Checks |
|---|---|
| P-E1 Criterion coverage | Every goal/workflow/criterion owns an acceptance scenario and production path where applicable; each substrate is contained by a later scenario owner's dependency closure. |
| P-E2 Demonstrable vertical slices | Each task answers what can be demonstrated through scenario verification, production call sites, and behavioral tests; substrate tasks cannot close product criteria alone. |
| P-E3 Dependencies and sequencing | Edges are real blockers, cycles are absent, and expand-contract is used only for justified wide refactors. |
| P-E4 Ownership and verification | Owned paths, necessary public/config/state/dependency surfaces, public behavioral seam, verification tier/addenda, and forbidden scope are proportional and consistent. |
| P-E5 DoD precision | Observable commands, interfaces, artifacts, runtime effects, diagnostics, and negative behavior replace vague verbs. |

Route every blocking task-graph defect to `GRAPH_REVISION_REQUIRED`.

### Plan protocol

The deep path invokes:

```text
scriptPath: .claude/skills/opi-implement/scripts/plan.workflow.js
args: { draftTasks, sourceDesignPath, phase, independence }
```

The script runs the plan lenses against the same original draft, adversarially
checks their blocking findings, and returns:

```text
{
  verdict,
  design_findings,
  graph_findings,
  flagged_for_human,
  rejected,
  resource_summary,
  report
}
```

Verdict precedence is:

```text
RESEARCH_REQUIRED
DESIGN_DECISION_REQUIRED
GRAPH_REVISION_REQUIRED
READY
```

The single-reviewer path returns the same object. Record a plan `verify_runs`
entry and `wf_ref` when available, but do not apply findings to the draft. A
non-`READY` verdict stops before P.4. A graph edit invalidates `READY` and repeats
this stage.

## Exec stage (Phase D.2)

Exec is an adversarial judgment layer on top of the D.0/D.1 mechanical gates.
Surviving must-fix findings block Phase D and route to Phase C; the agent fixes
them through Matt `tdd` or, for a hard bug, Matt `diagnosing-bugs`.

| Lens | Checks |
|---|---|
| L-D1 Implementation matches DoD | No stub, placeholder, or partial behavior passes an observable assertion. |
| L-D2 Tests are non-vacuous | No tautology, always-pass assertion, implementation-coupled test, or avoidable internal mock. |
| L-D3 Production call site is proven | Runtime claims have a real production caller exercised through the acceptance seam. |
| L-D4 Evidence is truthful | Artifacts and `Opi-*` evidence match the commands and behavior actually observed. |
| L-D5 Non-goals remain excluded | The implementation does not drift into a registered source Non-Goal. |
| L-D6 Workspace dependency rules hold | Internal dependencies use the workspace and publishable path+version rules. |
| L-D7 Decision locality and test stewardship | Declared decisions have one owning typed Interface, every production consumer routes through it, legacy paths close, and test disposition neither layers equivalent tests nor pins a superseded Interface. |

Deep-path protocol:

```text
scriptPath: .claude/skills/opi-implement/scripts/exec.workflow.js
args: { task, sourceDesignPath, commit }
result: { must_fix, flagged_for_human, rejected, report }
```

Store the run id in `verify_runs[].wf_ref` when available.

After an accepted finding with non-null `decision_id`, append that ID and the
finding summary to the current attempt's `gate_results`. If an accepted finding
with the same ID already exists in any active phase task note, stop task-local
repair and return to graph review with `GRAPH_REVISION_REQUIRED`.

## Phase-exit stage (Phase F.1a)

F.1a first constructs the current criteria trace. The Workflow then audits it:

| Lens | Checks |
|---|---|
| L-F1 Criterion traced to code | Every criterion maps to real production code. |
| L-F2 Criterion traced to test | Every criterion has an exercising behavioral test. |
| L-F3 Non-goals respected | No Non-Goal was implemented to satisfy a criterion. |
| L-F4 Residuals exactly cited | Every deferral cites the exact current source. |
| L-F5 Substrate/product distinction honest | No product criterion is closed by substrate-only work. |
| L-F6 Shared decisions closed | Completed tasks and assembled code prove one owner and Interface, all production consumers, closed legacy paths, the closure test, and truthful exhaustive test dispositions. |

Invoke:

```text
scriptPath: .claude/skills/opi-implement/scripts/phase-exit.workflow.js
args: { criteriaTrace, phaseTasks, sourceDesignPath, phase }
result: { not_met, flagged_for_human, rejected, report }
```

A surviving blocking finding for criterion C upserts
`criteria_trace[C].status = not-met` with the source citation as evidence.
An L-F6 finding uses a criterion declared by its decision note and carries the
stable `decision_id`.
`F.1b` then refuses archive. An uncited `deferred-by-updated-design` is also
`not-met`. Non-blocking findings do not mutate the trace.

## Report and guardrails

- **plan** — independence, primary verdict, design findings, graph findings,
  non-blocking human flags, rejected findings, and a deterministic resource
  summary including total reviewers and `max_parallel_agents`;
- **exec** — must-fix, human flags, and rejected findings;
- **phase exit** — not-met, human flags, and rejected findings.

Do not invent scope, suppress a finding to save a loop, or ask a reviewer to
mutate the artifact it reviews. Reviewers never invoke `opi-implement`, the
current verify stage, or another reviewer recursively. The human graph gate,
mechanical tests, task evidence gate, and phase archive gate remain
authoritative.
