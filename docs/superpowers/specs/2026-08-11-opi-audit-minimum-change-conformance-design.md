# Opi Audit Minimum-Change Conformance Design

**Date:** 2026-08-11

**Status:** Approved

**Scope:** `opi-audit` consumption of admitted minimum-change traces

## Problem

`opi-implement` now records a six-question minimum-change trace at graph
admission. `opi-audit` already verifies Standards, Spec, Correctness, Test
quality, Invariants, Integration, and Residuals, but complexity remains
implicit in the smell baseline and residual findings. It does not compare the
implemented module interfaces, placement, reuse, or simplification limits with
the admitted trace.

Adding an independent complexity axis would duplicate existing authority and
make finding ownership ambiguous. The audit instead needs a small conformance
overlay that deepens its current Standards, Spec, and Integration axes.

## Decision

Derive a task-level `minimum-change conformance matrix` from the archived
schema-v2 ledger and registered sources, then compare it with the complete
relevant implementation at the pinned committed `audit_head`.

Do not add a finding axis, severity, verdict, ledger field, or audit mode.
Normalized findings retain the existing interchange contract.

## Authority and Evidence

The current committed HEAD remains the sole implementation endpoint. The
admitted trace is a requirement claim to verify, not proof that the
implementation conforms.

Allowed trigger evidence is limited to committed, reproducible repository
state:

- production source and configuration;
- tests and fixtures;
- platform/build matrices and checked-in support declarations;
- registered specifications and archived task evidence.

External usage metrics, telemetry, provider dashboards, and uncommitted
working-tree content are outside audit authority. A trigger that depends on
such evidence is `not-assessable`, not a finding.

## Matrix Construction

During Phase A, derive one row per audited task:

| Trace question | Ledger evidence | Audit comparison |
|---|---|---|
| Registered criterion/scenario | `acceptance_scenarios[].id` and `.source`; downstream scenario owner for substrate tasks | Current implementation still serves the registered behavior |
| Reuse search | `inference_notes[field = "reuse_search"]` | No duplicate helper, runtime seam, package, or protocol was introduced |
| Placement | `inference_notes[field = "placement"]` | Core/plugin/package placement matches the admitted home |
| Surface necessity | `inference_notes[field = "surface_necessity"]` | Actual public interface, config, state, and dependency edges do not exceed the admitted surface |
| Production slice | scenario verification, `production_call_sites`, behavioral tests, and dependency closure | The production path exists and has not expanded into unnecessary bypasses |
| Simplification ceiling | `inference_notes[field = "simplification_ceiling"]` | The ceiling is respected and any repository-observable `revisit_when` trigger is identified |

The conformance matrix supplements the existing requirements matrix. It does
not replace source criteria, task DoDs, or acceptance evidence.

## Trace Availability

Audit determines trace availability structurally rather than by timestamps or
schema migration markers:

- none of the four standardized notes exists: `not-recorded`; treat the task
  as legacy and continue the ordinary Standards/Spec audit;
- at least one standardized note exists but a required note or clause is
  missing: the task adopted the contract incompletely; record `drifted` and
  emit a Spec finding;
- all notes and existing acceptance/call-site fields are present: perform the
  full comparison.

The audit never reconstructs missing legacy answers or rewrites the ledger.

## Conformance Status

Each task row uses exactly one primary status:

| Status | Meaning |
|---|---|
| `conforming` | Current committed implementation remains inside the admitted trace |
| `drifted` | Actual placement, surface, reuse, or production path exceeds or contradicts the trace |
| `triggered` | Repository evidence proves the recorded `revisit_when` condition is now true |
| `not-recorded` | No standardized trace notes exist; legacy compatibility applies |
| `not-assessable` | The trigger requires evidence outside pinned-HEAD audit authority |

`triggered` is not automatically a defect. It becomes a finding only when the
implementation still relies on the accepted simplification beyond its stated
ceiling or when the source requires action that has not occurred.

## Finding Routing

Findings remain on existing axes:

| Observation | Axis |
|---|---|
| Unadmitted public interface, config, state, dependency edge, or core placement | `spec` |
| Duplicate implementation despite recorded reuse; shallow module; hypothetical seam; adapter without leverage | `standards` |
| Cross-task duplicate logic, divergent protocol handling, or inconsistent handoff | `integration` |
| Incomplete post-contract trace that obscures an admitted decision | `spec` |

Complexity alone is never a Blocker. Use the existing severity definitions:

- Blocker only when the resulting defect exposes credentials/data, destroys
  data, deadlocks, crashes on expected input, or makes a core path unusable;
- Major for material product/spec expansion or a triggered ceiling causing a
  significant behavior gap;
- Minor for bounded unnecessary interface, dependency, duplication, or
  incomplete trace evidence without current incorrect behavior;
- Info for a defensible design evolution or future concern that is not a
  defect.

`not-recorded` and `not-assessable` appear in the matrix only and do not create
findings by themselves.

## Workflow Changes

### Phase A: Data acquisition

After extracting tasks and dependencies, extract the four standardized notes,
acceptance scenarios, production call sites, behavioral verification, and the
downstream scenario owner for substrate tasks. Build the conformance matrix
from committed ledger objects at `audit_head`.

### Phase B: Dimension inference

Activate the overlay when at least one audited task contains a standardized
minimum-change note. Report it as an overlay on Standards, Spec, and
Integration, not as a selectable dimension. Legacy-only phases retain the
existing dimension interview unchanged.

### Phase D: Audit execution

For every complete trace row:

1. locate the actual committed module interface, placement, dependencies, and
   production callers;
2. search the complete relevant implementation for the recorded reuse targets
   and competing duplicates;
3. compare actual public/config/state/dependency surfaces with the admitted
   `surface_necessity` clauses;
4. verify that substrate work reaches its downstream scenario owner;
5. assess repository-observable simplification triggers;
6. route actionable differences to the existing finding axes.

The full-read and pinned-HEAD rules remain unchanged.

### Phase E: Report

Add a `Minimum-change Conformance` section before Residuals:

| Task | Scenario/source | Reuse | Placement | Surface | Production slice | Ceiling/trigger | Status |
|---|---|---|---|---|---|---|---|

Cells cite committed code, tests, task evidence, or `not-recorded` /
`not-assessable`. Actionable rows link to their ordinary normalized finding.

## Files to Change

- `.claude/skills/opi-audit/SKILL.md`: extract, activate, execute, and report
  the conformance overlay.
- `.claude/skills/opi-audit/references/finding-template.md`: document the
  conformance table and existing-axis routing examples.
- `scripts/opi-doc-check.py`: add a focused cross-file audit-overlay contract.
- `scripts/test_opi_doc_check.py`: add happy-path and token-removal mutation
  tests for the overlay contract.

Do not modify `opi-audit/agents/openai.yaml`, the shared finding schema,
`opi-implement`, ledger files, Rust code, or historical audit reports.

## Verification

Test impact: update documentation-contract tests; no Rust tests.

Run:

```text
python -m unittest scripts.test_opi_doc_check -v
python scripts/opi-doc-check.py
git diff --check -- .claude/skills/opi-audit/SKILL.md
git diff --check -- .claude/skills/opi-audit/references/finding-template.md
git diff --check -- scripts/opi-doc-check.py scripts/test_opi_doc_check.py
```

The contract test must detect removal of trace extraction, the five statuses,
existing-axis routing, pinned-HEAD evidence limits, legacy handling, or the
report table.

## Rollout

The change applies to future audits immediately:

- archived phases without standardized notes remain auditable as legacy;
- phases with a partial trace surface an incomplete-contract Spec finding;
- phases with a complete trace receive full conformance comparison;
- no historical ledger or report is rewritten.

## Rejected Alternatives

### Independent complexity dimension

Rejected because surface drift is a Spec concern, shallow interfaces and
duplicate implementation are Standards concerns, and cross-task duplication
is already Integration. A new dimension would duplicate all three.

### Separate complexity-audit skill

Rejected because it would repeat source acquisition, pinned-HEAD isolation,
requirements construction, and finding normalization while exposing almost no
new interface. The complexity would reappear in callers rather than remain
local to `opi-audit`.

### Reconstruct legacy traces

Rejected because inference after implementation would convert an audit into a
new admission decision and could falsely legitimize existing complexity.
