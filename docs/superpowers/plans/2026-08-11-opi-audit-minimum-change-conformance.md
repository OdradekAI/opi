# Opi Audit Minimum-Change Conformance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `opi-audit` compare current committed implementation complexity with admitted minimum-change traces while preserving its existing finding axes, severities, verdicts, and pinned-HEAD authority.

**Architecture:** Extend the existing audit workflow with a derived conformance matrix rather than a new audit axis. A focused Python documentation-contract checker keeps matrix extraction, status semantics, evidence limits, existing-axis routing, and report rendering synchronized across the audit skill and finding template.

**Tech Stack:** Markdown skill contracts, Python 3.11+ standard library, `unittest`.

---

## Design Reference

Implement the approved design in
`docs/superpowers/specs/2026-08-11-opi-audit-minimum-change-conformance-design.md`.

## File Map

| File | Responsibility in this change |
|---|---|
| `scripts/test_opi_doc_check.py` | Independently specify and mutation-test the audit overlay's cross-file tokens. |
| `scripts/opi-doc-check.py` | Run the focused audit minimum-change conformance contract. |
| `.claude/skills/opi-audit/SKILL.md` | Extract, activate, execute, route, and report the conformance overlay. |
| `.claude/skills/opi-audit/references/finding-template.md` | Define the report matrix and show how actionable rows retain existing axes. |

## Execution Constraints

- Preserve the existing uncommitted default-prompt change in
  `.claude/skills/opi-audit/agents/openai.yaml`; do not edit or revert it.
- Preserve the already implemented minimum-change admission contract and skill
  consistency checks in `scripts/opi-doc-check.py` and
  `scripts/test_opi_doc_check.py`.
- Do not modify `opi-implement`, `.opi-impl-state.json`, the shared finding
  contract, Rust code, Cargo metadata, or historical audit reports.
- Do not run Rust compilation for this documentation/skill-only change.
- Do not stage or commit unless the user separately gives explicit commit
  authorization.

### Task 1: Add Red Audit-Overlay Contract Tests

**Files:**

- Modify: `scripts/test_opi_doc_check.py:16-174`
- Test: `scripts/test_opi_doc_check.py`

- [ ] **Step 1: Add the independent audit-overlay fixture**

Insert this constant immediately after `MINIMUM_CHANGE_TRACE_REQUIRED` and
before `class SkillContractTests`:

```python
AUDIT_MINIMUM_CHANGE_CONFORMANCE_REQUIRED = {
    ".claude/skills/opi-audit/SKILL.md": (
        "minimum-change conformance matrix",
        "`reuse_search`",
        "`surface_necessity`",
        "`simplification_ceiling`",
        "`conforming`",
        "`drifted`",
        "`triggered`",
        "`not-recorded`",
        "`not-assessable`",
        "current committed `audit_head`",
        "Finding routing remains on existing axes",
        "## Minimum-change Conformance",
    ),
    ".claude/skills/opi-audit/references/finding-template.md": (
        "## Minimum-change Conformance",
        "| Task | Scenario/source | Reuse | Placement | Surface | Production slice | Ceiling/trigger | Status |",
        "`not-recorded`",
        "`not-assessable`",
        "`standards`",
        "`spec`",
        "`integration`",
    ),
}
```

- [ ] **Step 2: Add a fixture writer**

Add this method after `write_minimum_change_trace_docs`:

```python
    def write_audit_minimum_change_conformance_docs(self) -> None:
        for rel, tokens in AUDIT_MINIMUM_CHANGE_CONFORMANCE_REQUIRED.items():
            self.write(rel, "\n".join(tokens) + "\n")
```

- [ ] **Step 3: Add happy-path and token-removal tests**

Add these methods immediately after the two existing minimum-change trace
contract tests:

```python
    def test_audit_minimum_change_conformance_contract_passes(self) -> None:
        self.write_audit_minimum_change_conformance_docs()

        checker = getattr(
            doc_check,
            "check_audit_minimum_change_conformance_contract",
            None,
        )
        self.assertIsNotNone(checker, "audit conformance checker must exist")
        checker()

        self.assertEqual([], doc_check.ERRORS)

    def test_audit_minimum_change_conformance_requires_every_token(self) -> None:
        checker = getattr(
            doc_check,
            "check_audit_minimum_change_conformance_contract",
            None,
        )
        self.assertIsNotNone(checker, "audit conformance checker must exist")

        for rel, tokens in AUDIT_MINIMUM_CHANGE_CONFORMANCE_REQUIRED.items():
            for token in tokens:
                with self.subTest(rel=rel, token=token):
                    doc_check.ERRORS = []
                    self.write_audit_minimum_change_conformance_docs()
                    self.write(
                        rel,
                        "\n".join(item for item in tokens if item != token) + "\n",
                    )

                    checker()

                    self.assertIn(
                        f"{rel}: audit minimum-change conformance contract "
                        f"missing semantic tokens {[token]!r}",
                        doc_check.ERRORS,
                    )
```

- [ ] **Step 4: Run the focused tests and verify RED**

Run:

```text
python -m unittest scripts.test_opi_doc_check -v
```

Expected: 12 tests run; the two new audit-overlay tests fail only because
`check_audit_minimum_change_conformance_contract` does not exist. The existing
10 tests remain passing.

### Task 2: Implement the Focused Documentation Checker

**Files:**

- Modify: `scripts/opi-doc-check.py:15-104`
- Test: `scripts/test_opi_doc_check.py`

- [ ] **Step 1: Add the checker-owned contract map**

Insert this constant immediately after `MINIMUM_CHANGE_TRACE_CONTRACT`:

```python
AUDIT_MINIMUM_CHANGE_CONFORMANCE_CONTRACT = {
    ".claude/skills/opi-audit/SKILL.md": (
        "minimum-change conformance matrix",
        "`reuse_search`",
        "`surface_necessity`",
        "`simplification_ceiling`",
        "`conforming`",
        "`drifted`",
        "`triggered`",
        "`not-recorded`",
        "`not-assessable`",
        "current committed `audit_head`",
        "Finding routing remains on existing axes",
        "## Minimum-change Conformance",
    ),
    ".claude/skills/opi-audit/references/finding-template.md": (
        "## Minimum-change Conformance",
        "| Task | Scenario/source | Reuse | Placement | Surface | Production slice | Ceiling/trigger | Status |",
        "`not-recorded`",
        "`not-assessable`",
        "`standards`",
        "`spec`",
        "`integration`",
    ),
}
```

- [ ] **Step 2: Add the non-mutating checker function**

Insert this function immediately after
`check_minimum_change_trace_contract`:

```python
def check_audit_minimum_change_conformance_contract() -> None:
    for rel, tokens in AUDIT_MINIMUM_CHANGE_CONFORMANCE_CONTRACT.items():
        require_tokens(
            rel,
            "audit minimum-change conformance contract",
            tokens,
        )
```

Do not call the function from `main()` until Task 3 updates the two audit
documents.

- [ ] **Step 3: Run the focused tests and verify GREEN**

Run:

```text
python -m unittest scripts.test_opi_doc_check -v
```

Expected: `Ran 12 tests` and `OK`.

### Task 3: Encode the Audit Conformance Overlay

**Files:**

- Modify: `.claude/skills/opi-audit/SKILL.md:70-345`
- Modify: `.claude/skills/opi-audit/references/finding-template.md:67-83`
- Modify: `scripts/opi-doc-check.py:590-603`
- Test: `scripts/test_opi_doc_check.py`

- [ ] **Step 1: Add Phase A matrix extraction**

In `.claude/skills/opi-audit/SKILL.md`, insert this numbered step immediately
after current Phase A step 5:

```markdown
5a. Derive a `minimum-change conformance matrix` from committed ledger objects
    at `audit_head`. For every task, collect:
    - `acceptance_scenarios[].id` and `.source`, or the later scenario owner
      whose dependency closure contains a substrate task;
    - `inference_notes` entries `reuse_search`, `placement`,
      `surface_necessity`, and `simplification_ceiling`;
    - scenario/task `production_call_sites` and
      `verification.behavioral_tests`.

    Classify trace availability structurally:
    - none of the four standardized notes exists -> `not-recorded`; continue
      the ordinary audit without reconstructing legacy answers;
    - at least one note exists but a required note or clause is absent ->
      `drifted`; add a Spec requirement for incomplete admission evidence;
    - all required trace evidence exists -> compare it with the complete
      relevant implementation at the current committed `audit_head`.
```

Do not renumber the existing Phase A steps. The insertion must not change
current-HEAD authority or allow working-tree evidence.

- [ ] **Step 2: Add Phase B overlay activation and evidence limits**

Insert this subsection after the existing dimension table and before the
inference heuristic:

```markdown
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
```

- [ ] **Step 3: Add the Phase D audit checklist and routing**

Insert this section after `Cross-task integration audit` and before
`Residuals`:

```markdown
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
```

- [ ] **Step 4: Add the report section and status contract**

In the Phase E report example, insert this section before Residuals and change
the Residuals heading from `N+2` to `N+3`:

```markdown
## N+2. Minimum-change Conformance

| Task | Scenario/source | Reuse | Placement | Surface | Production slice | Ceiling/trigger | Status |
|------|-----------------|-------|-----------|---------|------------------|-----------------|--------|
| ...  | ...             | ...   | ...       | ...     | ...              | ...             | `conforming` |

Allowed primary statuses are `conforming`, `drifted`, `triggered`,
`not-recorded`, and `not-assessable`. Cells cite committed code, tests, task
evidence, or the applicable non-assessable/legacy state. Actionable rows link
to their ordinary normalized finding under Standards, Spec, or Integration.
```

- [ ] **Step 5: Extend the finding-template reference**

In
`.claude/skills/opi-audit/references/finding-template.md`, insert this section
before `## Complete example`:

```markdown
## Minimum-change Conformance

When the audited phase contains standardized minimum-change notes, include
this table before Residuals:

| Task | Scenario/source | Reuse | Placement | Surface | Production slice | Ceiling/trigger | Status |
|------|-----------------|-------|-----------|---------|------------------|-----------------|--------|
| ...  | ...             | ...   | ...       | ...     | ...              | ...             | `conforming` |

Allowed statuses are `conforming`, `drifted`, `triggered`, `not-recorded`,
and `not-assessable`. `not-recorded` is a legacy state and
`not-assessable` means required evidence is outside pinned-HEAD authority;
neither creates a finding by itself.

Actionable rows still emit the shared normalized block. Use axis `standards`
for shallow interfaces, duplicate implementation, or hypothetical seams; axis
`spec` for unadmitted placement/surface or incomplete post-contract evidence;
and axis `integration` for cross-task duplication or divergent handoffs.
```

- [ ] **Step 6: Activate the deterministic contract check**

In `scripts/opi-doc-check.py`, add this call in `main()` immediately after
`check_minimum_change_trace_contract()`:

```python
    check_audit_minimum_change_conformance_contract()
```

- [ ] **Step 7: Run focused and integrated checks**

Run:

```text
python -m unittest scripts.test_opi_doc_check -v
python scripts/opi-doc-check.py
```

Expected:

- unit tests: `Ran 12 tests` and `OK`;
- documentation contract: `opi documentation contracts: PASS`.

### Task 4: Verify Scope and Handoff

**Files:**

- Verify only; no new files.

- [ ] **Step 1: Check whitespace in tracked task-owned files**

Run:

```text
git diff --check -- .claude/skills/opi-audit/SKILL.md .claude/skills/opi-audit/references/finding-template.md scripts/opi-doc-check.py
```

Expected: exit code 0. CRLF conversion warnings are informational; trailing
whitespace errors are not.

- [ ] **Step 2: Check whitespace in the untracked test and design artifacts**

Run each command and inspect that it reports no whitespace error; exit code 1
is expected because each file is untracked:

```text
git diff --no-index --check -- /dev/null scripts/test_opi_doc_check.py
git diff --no-index --check -- /dev/null docs/superpowers/specs/2026-08-11-opi-audit-minimum-change-conformance-design.md
git diff --no-index --check -- /dev/null docs/superpowers/plans/2026-08-11-opi-audit-minimum-change-conformance.md
```

- [ ] **Step 3: Re-run the authoritative documentation checks**

Run:

```text
python -m unittest scripts.test_opi_doc_check -v
python scripts/opi-doc-check.py
```

Expected: 12 tests pass and the documentation checker prints PASS.

- [ ] **Step 4: Audit the exact diff and protected files**

Run:

```text
git diff -- .claude/skills/opi-audit/SKILL.md .claude/skills/opi-audit/references/finding-template.md scripts/opi-doc-check.py
git diff --no-index -- /dev/null scripts/test_opi_doc_check.py
git status --short -- .opi-impl-state.json .claude/skills/opi-audit .claude/skills/opi-implement scripts/opi-doc-check.py scripts/test_opi_doc_check.py docs/superpowers/specs/2026-08-11-opi-audit-minimum-change-conformance-design.md docs/superpowers/plans/2026-08-11-opi-audit-minimum-change-conformance.md
```

Verify manually:

- `opi-audit/agents/openai.yaml` retains its pre-existing prompt-only change;
- no task-owned diff changes `opi-implement`, the canonical ledger, the shared
  finding contract, Rust code, or audit history;
- the audit overlay does not add a finding axis, severity, verdict, audit mode,
  network read, or working-tree evidence source;
- unrelated worktree changes remain untouched and unstaged.

- [ ] **Step 5: Report completion without committing**

Report test impact as `update`, list the two verification commands and their
results, and state explicitly that the ledger and agent sidecar were not
modified by this work. Do not stage or commit.

If the user later explicitly authorizes one commit, first re-run `git status`,
stage only the six files changed for this work, verify the staged diff, and
use:

```text
git commit -m "docs(opi-audit): verify minimum-change conformance"
```
