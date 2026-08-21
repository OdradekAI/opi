---
name: opi-audit
disable-model-invocation: true
description: >-
  Perform an independent code audit of a specific opi implementation phase.
  Given a phase number, extract the task graph, registered specs, definitions
  of done, and claimed evidence from `.opi-impl-state.json`, pin the repository's
  current committed HEAD, then verify every requirement against the complete
  relevant implementation at that HEAD. Use this skill whenever the user
  mentions "audit",
  "code review", "审计", "审查", "review phase N", "compare spec and
  implementation", or asks to verify whether a phase was implemented correctly.
  Also use when the user wants to check spec compliance, find implementation
  gaps, or produce a structured audit report for any phase.
---

# Opi Audit

Independent, phase-level code audit for the opi project. Each audit compares a
design specification against the actual implementation, producing a structured
findings report with severity classifications.

## Inputs

```text
phase=<N>           # required; the phase number to audit (e.g. 13)
focus=<text>        # optional; specific dimensions, tasks, or concerns
```

If the user says "audit phase 13" or "审查 phase 12", extract the phase number.
If no phase number is provided, ask for it.

## Current-HEAD authority

An `opi-audit` always evaluates the repository's current committed `HEAD`. The
implementation ledger and its registered specs define the requirements to
verify. They do not define a changed-file or commit-range coverage boundary.
Auditing only a historical diff would re-report defects that later remediation
already fixed, miss defects introduced by that remediation, and omit relevant
code that did not change inside the selected range.

At audit start, resolve and retain:

```text
requirement_set   = registered specs + task claims + DoD + evidence claims
phase_exit_commit = last task verified_at_commit (provenance only)
audit_head        = git rev-parse HEAD (the sole audit endpoint)
```

There is no historical audit mode. Do not offer or infer a phase-exit scope.
Git commits and diffs are optional provenance and discovery aids only. Never
use them to decide which requirements, source files, tests, or findings are in
scope. The audit object is the complete relevant implementation at `audit_head`.
When the worktree is dirty, audit the committed objects at `audit_head`, not the
working-tree copies. The report itself may be written to the worktree after the
evidence has been derived from the pinned commit. Run any build, test, or
reproduction command against an isolated checkout of `audit_head`; never treat
execution from a dirty working tree as audit evidence.

## Why independence matters

Audit value comes from fresh eyes. When multiple models audit the same phase
independently, their overlap validates real issues and their divergence surfaces
blind spots. Reading existing audit reports before forming conclusions creates
confirmation bias and reduces the audit's information value. The contamination
rules below exist to protect this independence.

## Workflow

### Phase A: Data acquisition

1. Pin `audit_head = git rev-parse HEAD`. All ledger, spec, context, source, and
   test reads for the audit must come from this commit (for example,
   `git show <audit_head>:<path>`), never from dirty working-tree copies.

2. Read `docs/snapshots/phase<N>/opi-impl-state.json` at `audit_head`.

3. Extract the spec file paths. The schema has evolved:
   - Schema v1: single `spec_path` string field
   - Schema v2: `spec_files` array of strings
   Handle both. Read all referenced spec files in full at `audit_head`.

4. Extract the task graph from `tasks[]`:
   - Task IDs, titles, crates, `definition_of_done`
   - Claimed evidence, acceptance commands, and `verified_at_commit` values
   - The last task commit as `phase_exit_commit`; other commits remain
     provenance or discovery aids only
   - `depends_on` relationships

5. Build a requirements matrix from every registered spec criterion, task
   claim, DoD item, and evidence claim. Merge duplicates but do not silently
   drop conflicts. A ledger claim or cited test is something to verify, not
   proof that the requirement passes.

5a. Derive a `minimum-change conformance matrix` from committed ledger objects
    at `audit_head`. For every task, collect:
    - `acceptance_scenarios[].id` and `.source`, or the later scenario owner
      whose dependency closure contains a substrate task;
    - `inference_notes` entries `reuse_search`, `placement`,
      `surface_necessity`, and `simplification_ceiling`;
    - scenario/task `production_call_sites` and
      `verification.behavioral_tests`.

    Classify trace availability without applying a newer admission contract
    retroactively to a frozen snapshot:
    - if no task records `simplification_trigger=`, treat the graph as
      pre-contract; classify absent notes or clauses as `not-recorded`, inspect
      any evidence that is present, and do not create a finding solely for the
      historical omission;
    - if at least one task records `simplification_trigger=`, the graph claims
      the current contract; a missing required note or clause is `drifted` and
      adds a Spec requirement for incomplete admission evidence;
    - when the complete required trace exists, compare it with the complete
      relevant implementation at the current committed `audit_head`.

    When a task claims an existing surface is unused, duplicate, or
    superseded, or proposes deletion, merging, replacement, or dependency
    substitution, also collect `production_consumers`,
    `nonproduction_consumers`, `net_deletion`, and `residual_glue` from the
    existing Reuse and Ceiling/trigger notes. Do not reconstruct these
    conditional answers for a legacy `not-recorded` graph.

6. If `phase_exit` exists for this phase, extract:
   - `completed_at` timestamp
   - `evaluator_summary` (prior evaluator's view -- use as context, not as
     ground truth)
   - Per-task `verified_at_commit` from `task_summary[]`

7. Record dirty worktree paths for isolation only. Do not read their contents as
   implementation evidence and do not include them in findings unless they are
   also present in `audit_head`.

8. Read project context at `audit_head`: `CLAUDE.md` or `AGENTS.md`,
   `docs/opi-spec.md`.

9. Use commit history or diffs only when helpful for locating implementation or
   understanding provenance. Do not require a non-empty diff and do not derive
   audit coverage from changed paths.

### Phase B: Dimension inference and interview

Matt `code-review` supplies two mandatory, separate axes and its Standards smell
baseline. Its fixed-point diff workflow does not define an `opi-audit` scope:

| Axis | Question |
|---|---|
| Standards | Does the complete relevant implementation at `audit_head` follow `AGENTS.md` / `CLAUDE.md`, other documented repository standards, and the Matt Fowler-smell baseline? |
| Spec | Does the complete relevant implementation at `audit_head` satisfy every registered spec criterion, task claim, and DoD item without omissions, incorrect behavior, or scope expansion? |

Do not merge or rerank these axes; a phase may pass one and fail the other.
Opi then adds the applicable phase-wide dimensions below.

| Dimension | When it applies |
|-----------|----------------|
| Correctness | Always |
| Security / redaction | Tasks involving export, user data, credentials, network I/O |
| Test quality | Always (but depth varies) |
| Invariants | When the spec defines explicit invariants or contracts |
| Cross-task integration | Phases with 4+ tasks or multi-crate changes |
| Residuals | Always (catch-all for issues outside other dimensions) |

**Minimum-change conformance overlay:** Activate this overlay when at least one
audited task contains a standardized minimum-change note. It is not a
selectable dimension and does not add an axis, severity, or verdict. Report it
as an overlay on Standards, Spec, and Integration.

The overlay compares the admitted trace with the complete relevant
implementation at the current committed `audit_head`. Trigger evidence is
limited to committed source/configuration, tests/fixtures, checked-in
platform/build matrices, registered specs, and archived task evidence.
External usage metrics, telemetry, provider dashboards, and dirty working-tree
content are outside audit authority; classify those triggers
`not-assessable` rather than inventing a finding.

For triggered simplification claims, verify production consumers separately
from tests, docs, and examples. Check applicable dynamic loading,
configuration, wire or persistent formats, and public API use before accepting
`production_consumers=none`. Verify that `net_deletion` subtracts new glue and
that `residual_glue` names any surviving adapters, shims, or duplicate paths.
Finding routing remains on existing axes; incomplete post-contract evidence is
Spec drift, while demonstrated duplication or shallow seams remain Standards
or Integration findings as applicable.

**Inference heuristic**: scan task titles and the spec for keywords:
- "export", "redact", "credential", "key", "auth" -> Security
- "invariant", "contract", "guarantee", "must never" -> Invariants
- Multiple crates in task list, or "resume", "fork", "handoff" -> Integration

After inferring, briefly confirm with the user:
- The pinned `audit_head` and requirement sources
- Which dimensions apply
- Any specific areas of concern or focus
- Whether any dimensions should be added or dropped

If the user provided a `focus` parameter, weight those dimensions higher but
still cover the basics (Standards, Spec, correctness, and test quality).

### Phase C: Deep read

Thorough auditing requires reading complete source files, not just search
snippets. Partial reads miss context like error handling paths, type
definitions, and import relationships that are critical for correctness
judgments.

1. From the requirements matrix, identify every relevant source, configuration,
   documentation, fixture, and test file at `audit_head`:
   - Task `crate` fields point to the relevant crates
   - Spec criteria, task titles, DoD text, evidence claims, public entry points,
     call paths, and repository search identify additional implementation
   - Search for test files: `crates/<crate>/tests/` and inline `#[cfg(test)]`
   - Do not use a changed-file list to decide relevance
   - Read every selected file in full from `audit_head`, even when the worktree
     has a different copy

2. For phases touching many files, use parallel subagents organized by file
   group. For example, split by crate or by source-vs-test. This is a
   recommendation for efficiency -- the key requirement is that every relevant
   file gets a full read before findings are written.

3. Also read:
   - Documentation files referenced in the spec (README, opi-spec.md, localized
     counterparts)
   - Any migration or compatibility fixtures mentioned in tests

### Phase D: Audit execution

First verify that `git rev-parse HEAD` still equals the pinned `audit_head`. If
it changed, discard partial conclusions and restart Phase A with the new HEAD.

Open Matt `code-review` to load its two-axis definitions and full Standards
smell baseline. Do not invoke or inherit its fixed-point diff coverage model.
Run separate Standards and Spec reviewers against the requirements matrix and
the complete relevant implementation at `audit_head`. Instruct every reviewer
to read committed objects from `audit_head`, ignore uncommitted working-tree
content, and treat history/diffs only as non-authoritative discovery aids.
Their prompts receive this restriction:

```text
Do not invoke code-review, opi-audit, or spawn additional agents.
```

Preserve their results under separate `Standards` and `Spec` report headings.
Then run the applicable opi dimensions over every row of the requirements
matrix and all relevant current implementation, including unchanged and
pre-existing paths. A finding cannot be excluded merely because its file or
line is absent from a Git diff.

Before writing the verdict, verify `git rev-parse HEAD` against `audit_head`
again. A changed HEAD invalidates the pinned review and requires a restart.

Work through each active dimension. For each finding, follow the template in
`references/finding-template.md` (read it now if you haven't). The template is
a guide for narrative clarity. Every actionable finding also emits the exact
normalized block from `../_shared/references/finding-contract.md`, using the
canonical severity definitions in
`../_shared/references/finding-contract.md`.

**Correctness audit**:
- Trace each task's DoD claims against the actual code
- Check serde round-trips, error handling paths, boundary conditions
- Verify algorithm correctness (e.g., tree walks, chain reconstruction)
- Look for off-by-one errors, missing `None`/`Err` handling, silent failures

**Security / redaction audit** (when active):
- Trace data flow from input to output for every export/display path
- Verify redaction is applied to all text fields, not just obvious ones
- Check that source data is not mutated by read operations
- Look for partial-write leaks on error paths

**Test quality audit**:
- Build a coverage matrix: each feature x each operation
- Assess assertion strength: does the test verify behavior or just "no panic"?
- Check fixture realism: do test inputs resemble real-world data?
- Verify isolation: temp directories, no shared state, no test ordering deps
- Look for missing negative tests (error paths, rejection paths)

**Spec axis follow-through**:
- Map each Success Criterion from the spec to code evidence
- Verify each Non-Goal is not accidentally implemented
- Check source-declared priority or risk tiers, when present, against actual
  completion; do not invent a fixed P0/P1/P2 taxonomy
- Verify explicit deferrals are documented

**Invariant audit** (when active):
- For each stated invariant, trace the code path that enforces it
- Check whether tests verify the invariant
- Build a matrix: invariant x code-path x test-coverage

**Cross-task integration audit** (when active):
- Check semantic consistency between components built in different tasks
- Look for duplicated logic that could diverge
- Verify handoff points between crates

**Minimum-change conformance audit** (when the overlay is active):
- Verify the actual committed module interface, placement, configuration,
  state, dependency edges, and production callers against the task trace.
- Search the complete relevant implementation for each recorded reuse target
  and for competing duplicate helpers, seams, packages, or protocols.
- Treat shallow modules, hypothetical seams, and adapters without leverage as
  Standards concerns; do not use implementation-line/interface-line ratios.
- Treat unadmitted public/config/state/dependency surface or core placement as
  Spec concerns.
- Treat cross-task duplicate logic, divergent protocol handling, or
  inconsistent handoffs as Integration concerns.
- Verify that substrate work reaches the later scenario-owning task through
  the recorded dependency closure and production call path.
- Mark a repository-observable simplification trigger `triggered`. It becomes
  a finding only when the implementation still exceeds its recorded ceiling
  or the registered source requires an action that did not occur.

Finding routing remains on existing axes: `standards`, `spec`, and
`integration`. Complexity alone is never a Blocker. Apply the existing
severity definitions to the observed behavior or contract impact.

**Residuals**:
- Anything that doesn't fit other dimensions
- Concurrency concerns, performance cliffs, API ergonomics
- Items carried forward from evaluator summaries that need verification

### Phase E: Report

Output file: `docs/snapshots/phase<N>/audit.<model-id>.md`

The model-id is a short identifier for the auditing model (e.g., `opus4.6`,
`codex`, `glm5.2`, `gpt5.5`). Determine it from your own model identity. If
uncertain, ask the user.

**Report structure**:

```markdown
# Phase <N> <Title> -- Independent Code Audit

**Auditor**: <model-id> (independent, no prior audit reports consulted)
**Date**: <YYYY-MM-DD>
**Scope**: Phase <N> registered requirements and Tasks <first>--<last>
**Implementation target**: `<audit-head>` (current committed implementation)
**Phase exit commit**: `<last-task-commit>` (provenance only)
**History use**: provenance and discovery only; no diff coverage boundary
**Method**: <brief description of audit approach>

---

## 1. Executive Summary

**Verdict: <PASS | PASS-WITH-FINDINGS | FAIL>**

| Severity | Count |
|----------|-------|
| Blocker  | N     |
| Major    | N     |
| Minor    | N     |
| Info     | N     |

<2-3 sentence summary of overall quality and top concerns>

### Per-task summary

| Task | Title | Verdict |
|------|-------|---------|
| ...  | ...   | ...     |

---

## 2-N. <Dimension> Findings

### X.Y <SEVERITY>: <Short title>

**File:** `<path>`
**Lines:** <range>
**Cause:** <what is wrong and why>
**Impact:** <consequences if unfixed>
**Fix:** <specific suggested remediation>

```yaml
id: <source-stable identifier>
source_kind: audit
source_path: docs/snapshots/phase<N>/audit.<model-id>.md
source_model: <model-id>
independence: <independent-family | fresh-context-same-family | unknown>
axis: <standards | spec | security | test-quality | invariants | integration | residuals>
severity: <Blocker | Major | Minor | Info>
title: <short title>
claim: <falsifiable problem statement>
evidence:
  - location: <file:line or command>
    detail: <observed evidence>
criterion_source: <spec/rule citation or null>
reproduction: [<command>]
confidence: <high | medium | low>
status: unverified
```

---

## N+1. Invariant Verification (if applicable)

| Invariant | Code evidence | Test coverage |
|-----------|--------------|---------------|
| ...       | ...          | ...           |

---

## N+2. Minimum-change Conformance

| Task | Scenario/source | Reuse | Placement | Surface | Production slice | Ceiling/trigger | Status |
|------|-----------------|-------|-----------|---------|------------------|-----------------|--------|
| ...  | ...             | ...   | ...       | ...     | ...              | ...             | `conforming` |

Allowed primary statuses are `conforming`, `drifted`, `triggered`,
`not-recorded`, and `not-assessable`. Cells cite committed code, tests, task
evidence, or the applicable non-assessable/legacy state. Actionable rows link
to their ordinary normalized finding under Standards, Spec, or Integration.

---

## N+3. Residuals and Recommendations

### Priority recommendations
1. ...
```

Verdicts:
- **PASS**: No blockers or majors, minors are low-risk
- **PASS-WITH-FINDINGS**: No blockers, but majors need attention before next phase
- **FAIL**: Blockers present, or majors indicate systemic implementation problems

## Contamination isolation

These rules protect audit independence. They exist because the audit's value is
proportional to its independence from prior reviews.

- Do not read, search, grep for, or reference any `audit.*.md` files in
  `docs/snapshots/phase<N>/` before completing the audit report.
- Do not read evaluator reports, AI review results, or human review records
  for the phase being audited.
- The `evaluator_summary` field in `phase_exit` is acceptable context (it's
  structural metadata, not a detailed review), but do not seek out the full
  evaluator transcript.
- If you accidentally encounter existing audit content during file searches,
  state "Audit context contaminated" and disclose what was seen. Then proceed
  with extra care to ensure your findings are independently derived.
- Base all conclusions on: source code, tests, configuration, documentation,
  git history, and the design specification.

## Guardrails

- Do not modify source code, specs, tests, or documentation unless the user
  explicitly asks. This is an audit, not a fix-up session.
- Do not commit or push unless the user explicitly asks.
- Always audit the pinned current committed `audit_head`; never substitute the
  ledger's last task commit or a historical phase snapshot as the endpoint.
- Derive coverage from the registered specs, task claims, DoD, evidence claims,
  and their complete relevant implementation. Never derive coverage from a
  commit range, diff, or changed-file list.
- Exclude uncommitted worktree changes from evidence. Do not check out, stash,
  reset, or rewrite the user's worktree to achieve this; read Git objects.
- Execute verification commands against an isolated checkout of `audit_head`
  when the user's worktree is dirty.
- Distinguish "spec deviation" from "reasonable implementation evolution". When
  the implementation differs from the spec but the choice is defensible, note
  it as Info rather than Major.
- Do not reduce audit depth because other audit reports exist. Each audit stands
  alone.
- When findings overlap with `evaluator_summary` items from `phase_exit`, note
  the overlap but provide your own independent evidence and assessment.

## References

- Read `references/finding-template.md` for the finding format, severity
  definitions with examples, and a complete finding example drawn from a real
  audit.
- Read `../_shared/references/finding-contract.md` for the machine-stable
  interchange consumed by `opi-remediate`.
- Open Matt `code-review` before running the Standards/Spec axes to load its
  axis definitions and complete smell baseline. Do not inherit its diff-bound
  review scope.
