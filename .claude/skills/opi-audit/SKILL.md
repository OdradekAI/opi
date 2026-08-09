---
name: opi-audit
disable-model-invocation: true
description: >-
  Perform an independent code audit of a specific opi implementation phase.
  Given a phase number, automatically extract the task graph, design spec, and
  commit range from impl-state.json, then systematically compare spec against
  actual implementation. Use this skill whenever the user mentions "audit",
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

## Why independence matters

Audit value comes from fresh eyes. When multiple models audit the same phase
independently, their overlap validates real issues and their divergence surfaces
blind spots. Reading existing audit reports before forming conclusions creates
confirmation bias and reduces the audit's information value. The contamination
rules below exist to protect this independence.

## Workflow

### Phase A: Data acquisition

1. Read `docs/snapshots/phase<N>/opi-impl-state.json`.

2. Extract the spec file paths. The schema has evolved:
   - Schema v1: single `spec_path` string field
   - Schema v2: `spec_files` array of strings
   Handle both. Read all referenced spec files in full.

3. Extract the task graph from `tasks[]`:
   - Task IDs, titles, crates, `definition_of_done`
   - `verified_at_commit` values. Resolve the first task commit's parent as the
     fixed point and the last task commit as HEAD; verify the three-dot diff is
     non-empty before review
   - `depends_on` relationships

4. If `phase_exit` exists for this phase, extract:
   - `completed_at` timestamp
   - `evaluator_summary` (prior evaluator's view -- use as context, not as
     ground truth)
   - Per-task `verified_at_commit` from `task_summary[]`

5. Read project context: `CLAUDE.md` or `AGENTS.md`, `docs/opi-spec.md`.

### Phase B: Dimension inference and interview

Matt `code-review` supplies two mandatory, separate axes at the ledger-derived
fixed commit range:

| Axis | Question |
|---|---|
| Standards | Does the committed phase diff follow `AGENTS.md` / `CLAUDE.md`, other documented repository standards, and the Matt Fowler-smell baseline? |
| Spec | Does the diff implement the registered source without omissions, incorrect behavior, or scope expansion? |

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

**Inference heuristic**: scan task titles and the spec for keywords:
- "export", "redact", "credential", "key", "auth" -> Security
- "invariant", "contract", "guarantee", "must never" -> Invariants
- Multiple crates in task list, or "resume", "fork", "handoff" -> Integration

After inferring, briefly confirm with the user:
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

1. From the task graph, identify affected source files and test files:
   - Task `crate` fields point to the relevant crates
   - Task titles and DoD text often name specific modules
   - Search for test files: `crates/<crate>/tests/` and inline `#[cfg(test)]`

2. For phases touching many files, use parallel subagents organized by file
   group. For example, split by crate or by source-vs-test. This is a
   recommendation for efficiency -- the key requirement is that every relevant
   file gets a full read before findings are written.

3. Also read:
   - Documentation files referenced in the spec (README, opi-spec.md, localized
     counterparts)
   - Any migration or compatibility fixtures mentioned in tests

### Phase D: Audit execution

First open Matt `code-review`. When the phase head is current `HEAD`, invoke it
with the resolved fixed point and registered sources. For a historical phase,
apply its exact two-axis prompts to `git diff <fixed>...<phase-head>` and record
`adapted-historical-range`; do not check out or rewrite the user's working tree.
Its Standards and Spec agents receive this restriction:

```text
Do not invoke code-review, opi-audit, or spawn additional agents.
```

Preserve their results under separate `Standards` and `Spec` report headings.
Then run the applicable opi dimensions over the complete phase, including
unchanged production paths needed to verify the committed diff's integration.

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
- Check priority tiers (P0/P1/P2) against actual completion
- Verify explicit deferrals are documented

**Invariant audit** (when active):
- For each stated invariant, trace the code path that enforces it
- Check whether tests verify the invariant
- Build a matrix: invariant x code-path x test-coverage

**Cross-task integration audit** (when active):
- Check semantic consistency between components built in different tasks
- Look for duplicated logic that could diverge
- Verify handoff points between crates

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
**Scope**: Tasks <first>--<last>, commits `<first-commit>..<last-commit>`
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

## N+2. Residuals and Recommendations

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
- Open Matt `code-review` before running the Standards/Spec axes; this skill's
  summary is not a substitute for the real subskill.
