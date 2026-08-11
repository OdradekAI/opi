# Opi Minimum-Change Trace Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the six-question minimum-change trace a reviewable, drift-checked admission contract in `opi-implement` without changing ledger schema v2 or rewriting the current ledger.

**Architecture:** Reuse existing acceptance, call-site, verification, dependency, and `inference_notes` fields as the sole trace storage. Add one focused Python documentation-contract checker to keep the skill, initializer, ledger semantics, verify reference, and Workflow charters synchronized; the runtime ledger guard and Rust workspace remain unchanged.

**Tech Stack:** Markdown skill contracts, Workflow JavaScript, Python 3.11+ standard library, `unittest`.

---

## Design Reference

Implement the approved design in
`docs/superpowers/specs/2026-08-11-opi-minimum-change-trace-design.md`.

## File Map

| File | Responsibility in this change |
|---|---|
| `scripts/test_opi_doc_check.py` | Test the cross-file minimum-change trace contract, including mutation cases for every required token. |
| `scripts/opi-doc-check.py` | Define and run the deterministic cross-file contract check. |
| `.claude/skills/opi-implement/skill.md` | State the top-level rule and make Phase B render the six answers. |
| `.claude/skills/opi-implement/references/initializer.md` | Construct the trace at P.1, review it at P.2, and refuse incomplete P.4 confirmation. |
| `.claude/skills/opi-implement/references/ledger-schema.md` | Standardize the four `inference_notes` reason grammars and validation semantics without a schema bump. |
| `.claude/skills/opi-implement/references/verify-engine.md` | Map the six answers to existing design- and execution-readiness lenses and verdict routes. |
| `.claude/skills/opi-implement/scripts/plan.workflow.js` | Make the current lens charters inspect the standardized trace explicitly. |

## Execution Constraints

- Preserve the existing uncommitted Phase 14-16 registered-source path changes
  in `skill.md` and `initializer.md`; do not edit or revert those rows.
- Preserve the current skill-contract work already present in
  `scripts/opi-doc-check.py` and `scripts/test_opi_doc_check.py`.
- Do not modify `.opi-impl-state.json`, `schema_version`, ledger-guard scripts,
  Rust code, Cargo metadata, or registered phase source paths.
- Do not run Rust compilation for this documentation/skill-only change.
- Do not commit unless the user separately gives explicit commit authorization.

### Task 1: Add Red Contract Tests

**Files:**

- Modify: `scripts/test_opi_doc_check.py:9-184`
- Test: `scripts/test_opi_doc_check.py`

- [ ] **Step 1: Add an independent expected-token fixture**

Insert this module-level constant after `doc_check` is loaded and before
`class SkillContractTests`:

```python
MINIMUM_CHANGE_TRACE_REQUIRED = {
    ".claude/skills/opi-implement/skill.md": (
        "**Minimum-change trace rule:**",
        "`reuse_search`",
        "`surface_necessity`",
        "`simplification_ceiling`",
        "`legacy-unrecorded`",
    ),
    ".claude/skills/opi-implement/references/initializer.md": (
        "#### Minimum-change trace",
        '`field = "reuse_search"`',
        '`field = "placement"`',
        '`field = "surface_necessity"`',
        '`field = "simplification_ceiling"`',
        "`revisit_when`",
        "transitive `depends_on` closure",
        "REFUSE `confirm-all`",
    ),
    ".claude/skills/opi-implement/references/ledger-schema.md": (
        '`field = "reuse_search"`',
        "`searched=`",
        "`reused=`",
        "`gap=`",
        '`field = "placement"`',
        "`cannot_fit_fully=`",
        '`field = "surface_necessity"`',
        "`public_api=`",
        "`config=`",
        "`state=`",
        "`dependency_edge=`",
        '`field = "simplification_ceiling"`',
        "`ceiling=`",
        "`revisit_when=`",
    ),
    ".claude/skills/opi-implement/references/verify-engine.md": (
        "### Minimum-change trace overlay",
        '`reuse_search`',
        '`placement`',
        '`surface_necessity`',
        '`simplification_ceiling`',
        '`revisit_when`',
        '`GRAPH_REVISION_REQUIRED`',
        '`RESEARCH_REQUIRED`',
        '`DESIGN_DECISION_REQUIRED`',
    ),
    ".claude/skills/opi-implement/scripts/plan.workflow.js": (
        '"reuse_search"',
        '"placement"',
        '"surface_necessity"',
        '"simplification_ceiling"',
        "revisit_when",
        "transitive depends_on closure",
    ),
}
```

- [ ] **Step 2: Add a fixture writer to `SkillContractTests`**

Add this method after `write_indexes`:

```python
    def write_minimum_change_trace_docs(self) -> None:
        for rel, tokens in MINIMUM_CHANGE_TRACE_REQUIRED.items():
            self.write(rel, "\n".join(tokens) + "\n")
```

- [ ] **Step 3: Add the happy-path and mutation tests**

Add these methods before the existing
`test_matching_skill_contract_passes` test:

```python
    def test_minimum_change_trace_contract_passes(self) -> None:
        self.write_minimum_change_trace_docs()

        checker = getattr(doc_check, "check_minimum_change_trace_contract", None)
        self.assertIsNotNone(checker, "minimum-change trace checker must exist")
        checker()

        self.assertEqual([], doc_check.ERRORS)

    def test_minimum_change_trace_contract_requires_every_token(self) -> None:
        checker = getattr(doc_check, "check_minimum_change_trace_contract", None)
        self.assertIsNotNone(checker, "minimum-change trace checker must exist")

        for rel, tokens in MINIMUM_CHANGE_TRACE_REQUIRED.items():
            for token in tokens:
                with self.subTest(rel=rel, token=token):
                    doc_check.ERRORS = []
                    self.write_minimum_change_trace_docs()
                    self.write(
                        rel,
                        "\n".join(item for item in tokens if item != token) + "\n",
                    )

                    checker()

                    self.assertIn(
                        f"{rel}: minimum-change trace contract "
                        f"missing semantic tokens {[token]!r}",
                        doc_check.ERRORS,
                    )
```

- [ ] **Step 4: Run the focused tests and verify RED**

Run:

```text
python -m unittest scripts.test_opi_doc_check -v
```

Expected: the two new tests fail because
`check_minimum_change_trace_contract` does not exist; the eight existing skill
contract tests remain passing.

### Task 2: Implement the Focused Contract Checker

**Files:**

- Modify: `scripts/opi-doc-check.py:13-35`
- Test: `scripts/test_opi_doc_check.py`

- [ ] **Step 1: Add the checker-owned contract map**

Insert this constant after `ERRORS`:

```python
MINIMUM_CHANGE_TRACE_CONTRACT = {
    ".claude/skills/opi-implement/skill.md": (
        "**Minimum-change trace rule:**",
        "`reuse_search`",
        "`surface_necessity`",
        "`simplification_ceiling`",
        "`legacy-unrecorded`",
    ),
    ".claude/skills/opi-implement/references/initializer.md": (
        "#### Minimum-change trace",
        '`field = "reuse_search"`',
        '`field = "placement"`',
        '`field = "surface_necessity"`',
        '`field = "simplification_ceiling"`',
        "`revisit_when`",
        "transitive `depends_on` closure",
        "REFUSE `confirm-all`",
    ),
    ".claude/skills/opi-implement/references/ledger-schema.md": (
        '`field = "reuse_search"`',
        "`searched=`",
        "`reused=`",
        "`gap=`",
        '`field = "placement"`',
        "`cannot_fit_fully=`",
        '`field = "surface_necessity"`',
        "`public_api=`",
        "`config=`",
        "`state=`",
        "`dependency_edge=`",
        '`field = "simplification_ceiling"`',
        "`ceiling=`",
        "`revisit_when=`",
    ),
    ".claude/skills/opi-implement/references/verify-engine.md": (
        "### Minimum-change trace overlay",
        '`reuse_search`',
        '`placement`',
        '`surface_necessity`',
        '`simplification_ceiling`',
        '`revisit_when`',
        '`GRAPH_REVISION_REQUIRED`',
        '`RESEARCH_REQUIRED`',
        '`DESIGN_DECISION_REQUIRED`',
    ),
    ".claude/skills/opi-implement/scripts/plan.workflow.js": (
        '"reuse_search"',
        '"placement"',
        '"surface_necessity"',
        '"simplification_ceiling"',
        "revisit_when",
        "transitive depends_on closure",
    ),
}
```

- [ ] **Step 2: Add the non-mutating checker function**

Insert this function immediately after `require_tokens`:

```python
def check_minimum_change_trace_contract() -> None:
    for rel, tokens in MINIMUM_CHANGE_TRACE_CONTRACT.items():
        require_tokens(rel, "minimum-change trace contract", tokens)
```

Do not call the function from `main()` yet. The repository documents do not
satisfy it until Task 3.

- [ ] **Step 3: Run the focused tests and verify GREEN**

Run:

```text
python -m unittest scripts.test_opi_doc_check -v
```

Expected: `Ran 10 tests` and `OK`.

### Task 3: Encode the Six Questions in the Admission Workflow

**Files:**

- Modify: `.claude/skills/opi-implement/skill.md:56-72,171-176`
- Modify: `.claude/skills/opi-implement/references/initializer.md:153-224`
- Modify: `.claude/skills/opi-implement/references/ledger-schema.md:112,145-190`
- Modify: `.claude/skills/opi-implement/references/verify-engine.md:81-112`
- Modify: `.claude/skills/opi-implement/scripts/plan.workflow.js:92-116`
- Modify: `scripts/opi-doc-check.py:531-543`
- Test: `scripts/test_opi_doc_check.py`

- [ ] **Step 1: State the top-level rule and Phase B handoff**

In `skill.md`, insert this section after the existing DoD precision rule:

```markdown
**Minimum-change trace rule:** Every task graph admitted after this contract
MUST answer six questions using existing ledger fields: registered criterion
or scenario; reuse search; plugin/package placement; necessity of public API,
config, state, and dependency surfaces; the smallest production vertical
slice; and any accepted simplification's ceiling plus observable revisit
trigger. Reuse `acceptance_scenarios`, `production_call_sites`,
`verification.behavioral_tests`, `depends_on`, and standardized
`inference_notes` fields `reuse_search`, `placement`, `surface_necessity`, and
`simplification_ceiling`. Do not add a duplicate trace object or bump the
ledger schema. A substrate task must be in the transitive dependency closure
of a later scenario-owning task; otherwise refuse it as orphan work.
```

Extend B.1 with this exact continuation:

```markdown
     + the six-answer minimum-change trace. For a graph confirmed before this
     contract, print absent answers as `legacy-unrecorded`; never fabricate
     them. The exemption ends when the graph next enters the plan path. Phase B
     does not reinterpret the answers. If implementation would change an
     admitted API, config item, state field, dependency edge, placement, or
     simplification limit, stop and return to graph review; the Phase C
     `task_owned_paths` append-only exception remains the sole in-task mutation
     of a const field.
```

Keep the current Phase 14-16 source registry rows byte-for-byte unchanged.

- [ ] **Step 2: Add P.1 construction rules**

In `initializer.md`, insert this subsection at the end of P.1, immediately
before P.2:

```markdown
#### Minimum-change trace

For every executable draft task, construct the six-answer trace from existing
source and repository evidence already read during admission; do not start a
second research workflow solely to fill this trace. Product/vertical-slice
tasks cite `acceptance_scenarios[].id` and
`.source`. A `substrate_only` task may own no scenario only when a later
scenario-owning task's transitive `depends_on` closure contains it; otherwise
the substrate is orphan work and fails admission.

Add these four standardized notes using the existing
`{ field, reason, source }` shape:

- `field = "reuse_search"`: `reason` is
  `searched=<symbols/paths/packages/protocols>; reused=<items|none>;
  gap=<smallest missing capability>`;
- `field = "placement"`: `reason` is
  `target=<core|extension|plugin|package>; existing_home=<id|none>;
  cannot_fit_fully=<reason|not-applicable>`;
- `field = "surface_necessity"`: `reason` is
  `public_api=<none|necessity>; config=<none|necessity>;
  state=<none|necessity>; dependency_edge=<none|necessity>`;
- `field = "simplification_ceiling"`: `reason` is
  `accepted=<none|simplification>; ceiling=<known limit>;
  revisit_when=<observable condition>`.

Every `source` cites the registered source heading, reviewed decision, or
repository evidence used for that answer. `none` and `not-applicable` are
valid only with a reason. `revisit_when` must name an observable workflow,
threshold, platform capability, or failure condition; “when needed” is not
admissible.

The production-slice answer reuses
`acceptance_scenarios[].verification`, scenario/task
`production_call_sites`, and `verification.behavioral_tests`. Documentation
tasks may answer `not-applicable` and use their documentation-contract gate;
runtime tasks require both a production caller and behavioral proof.
```

- [ ] **Step 3: Add P.2 routes and P.4 refusal rules**

Append this paragraph to P.2 after the two readiness-axis bullets:

```markdown
Review all six minimum-change answers explicitly. An omitted answer with
otherwise sufficient source material is `GRAPH_REVISION_REQUIRED`; a missing
fact is `RESEARCH_REQUIRED`; an unsettled placement, public surface, or
simplification decision is `DESIGN_DECISION_REQUIRED`. Do not let generic
prose such as “reuse considered” satisfy a standardized note.
```

Append this paragraph to P.4 immediately before the gate-options list:

```markdown
Also render a six-row minimum-change trace per task. For substrate tasks, show
the later scenario owner whose transitive `depends_on` closure contains the
substrate. REFUSE `confirm-all` when a required answer is absent, any
`surface_necessity` clause is missing, `simplification_ceiling` omits
`revisit_when`, a substrate has no later scenario owner, or a runtime claim
lacks either a production call site or behavioral verification. This is
evidence inside the existing human graph gate, not an additional gate.
```

Keep the current Phase 14-16 source registry rows byte-for-byte unchanged.

- [ ] **Step 4: Standardize ledger note semantics without changing schema**

In `ledger-schema.md`, extend the `tasks[].inference_notes` field description
with this sentence:

```markdown
Minimum-change admission standardizes four additional `field` values without
changing the note shape or schema version: `reuse_search`, `placement`,
`surface_necessity`, and `simplification_ceiling`.
```

Add these validation rules after the existing public behavioral seam rule:

```markdown
Validation rule: minimum-change notes retain `{ field, reason, source }`.
`field = "reuse_search"` requires `searched=`, `reused=`, and `gap=` clauses.
`field = "placement"` requires `target=`, `existing_home=`, and
`cannot_fit_fully=` clauses. `field = "surface_necessity"` requires
`public_api=`, `config=`, `state=`, and `dependency_edge=` clauses, each set to
`none` or a necessity rationale. `field = "simplification_ceiling"` requires
`accepted=`, `ceiling=`, and `revisit_when=` clauses; the revisit trigger must
be observable rather than “when needed”. Missing clauses fail task-graph
admission; explicit `none` or `not-applicable` remains valid when justified.

Validation rule: a `substrate_only` task with no owned acceptance scenario
MUST appear in the transitive `depends_on` closure of a later task that owns a
sourced acceptance scenario. Otherwise it is orphan work and the graph cannot
be confirmed. Do not add a fake production call site to a substrate task.

Validation rule: the minimum-change trace is admission-only. Existing
schema-v2 ledgers remain readable and already-confirmed no-drift graphs are not
rewritten. After the next init, reconcile, import, or graph edit, every
executable task must satisfy the trace contract. Phase B labels missing
pre-contract answers `legacy-unrecorded` and never synthesizes them.
```

Do not change the example ledger JSON, `schema_version`, v1-to-v2 migration, or
atomic-write protocol.

- [ ] **Step 5: Map the trace onto existing review axes**

In `verify-engine.md`, add this subsection after the verdict-routing paragraph
under Design readiness:

```markdown
### Minimum-change trace overlay

The existing lenses jointly inspect the six answers; this does not add a new
lens or verdict. Design readiness verifies `reuse_search`, `placement`,
`surface_necessity`, and `simplification_ceiling`, including an observable
`revisit_when`. Execution readiness verifies the sourced criterion/scenario,
the smallest production vertical slice, and the later scenario owner for each
substrate task.

If the source is sufficient but the draft omits or malforms an answer, return
`GRAPH_REVISION_REQUIRED`. Missing facts return `RESEARCH_REQUIRED`.
Unsettled placement, surface, or simplification choices return
`DESIGN_DECISION_REQUIRED`.
```

Then make these existing table checks explicit:

```markdown
| P-D2 Plugin-first placement | Optional/non-pi capability defaults to plugin/package; `reuse_search` names what was inspected and `placement` proves any core change is the smallest missing extension seam. |
| P-D4 Acceptance/test seam | User-visible behavior, its registered scenario source, production call sites, and the highest practical public behavioral test seam are explicit. |
| P-D5 Source completeness | Problem, solution, out-of-scope, success, exit, evidence provenance, `surface_necessity`, and any `simplification_ceiling` are present without contradictions or silent assumptions. |
| P-E1 Criterion coverage | Every goal/workflow/criterion owns an acceptance scenario and production path where applicable; each substrate is contained by a later scenario owner's dependency closure. |
| P-E2 Demonstrable vertical slices | Each task answers what can be demonstrated through scenario verification, production call sites, and behavioral tests; substrate tasks cannot close product criteria alone. |
| P-E4 Ownership and verification | Owned paths, necessary public/config/state/dependency surfaces, public behavioral seam, verification tier/addenda, and forbidden scope are proportional and consistent. |
```

Replace the corresponding six rows; do not add duplicate rows.

- [ ] **Step 6: Tighten the current Workflow charters**

In `plan.workflow.js`, replace only these three `charter` strings:

```javascript
charter: 'Check pi design lineage, justified Rust-native divergence, evidence provenance, plugin-first placement, and whether any proposed core work is only the smallest missing extension seam. Require inference_notes fields "reuse_search", "placement", "surface_necessity", and "simplification_ceiling" with an observable revisit_when.',
```

```javascript
charter: 'Check criterion coverage, demonstrable vertical slices, substrate/product honesty, acceptance scenarios, and production call sites. Every substrate task must be contained by a later scenario owner through the transitive depends_on closure.',
```

```javascript
charter: 'Check observable DoDs, agreed behavioral seams, proportional verification tiers/addenda, forbidden-scope guards, non-goal leakage, and whether every runtime claim has both a production caller and behavioral verification.',
```

The first replaces `design-lineage-placement`, the second replaces
`execution-coverage-slices`, and the third replaces
`execution-verification-scope`. Do not alter schemas, result folding, verdict
precedence, or agent dispatch.

- [ ] **Step 7: Activate the deterministic contract check**

In `scripts/opi-doc-check.py`, add this call in `main()` immediately after
`skill_docs = check_skill_contracts()`:

```python
    check_minimum_change_trace_contract()
```

- [ ] **Step 8: Run focused and integrated checks**

Run:

```text
python -m unittest scripts.test_opi_doc_check -v
python scripts/opi-doc-check.py
node --check .claude/skills/opi-implement/scripts/plan.workflow.js
```

Expected:

- unit tests: `Ran 10 tests` and `OK`;
- documentation contract: `opi documentation contracts: PASS`;
- JavaScript syntax check: exit code 0 with no output.

### Task 4: Verify Scope and Handoff

**Files:**

- Verify only; no new files.

- [ ] **Step 1: Check whitespace in tracked task-owned files**

Run:

```text
git diff --check -- scripts/opi-doc-check.py scripts/test_opi_doc_check.py .claude/skills/opi-implement/skill.md .claude/skills/opi-implement/references/initializer.md .claude/skills/opi-implement/references/ledger-schema.md .claude/skills/opi-implement/references/verify-engine.md .claude/skills/opi-implement/scripts/plan.workflow.js
```

Expected: exit code 0. CRLF conversion warnings are informational; trailing
whitespace errors are not.

- [ ] **Step 2: Re-run the authoritative documentation checks**

Run:

```text
python -m unittest scripts.test_opi_doc_check -v
python scripts/opi-doc-check.py
node --check .claude/skills/opi-implement/scripts/plan.workflow.js
```

Expected: 10 tests pass, the documentation checker prints PASS, and Node exits
0 without output.

- [ ] **Step 3: Audit the exact diff and protected files**

Run:

```text
git diff -- scripts/opi-doc-check.py scripts/test_opi_doc_check.py .claude/skills/opi-implement/skill.md .claude/skills/opi-implement/references/initializer.md .claude/skills/opi-implement/references/ledger-schema.md .claude/skills/opi-implement/references/verify-engine.md .claude/skills/opi-implement/scripts/plan.workflow.js
git diff --no-index -- /dev/null scripts/test_opi_doc_check.py
git diff --no-index -- /dev/null docs/superpowers/specs/2026-08-11-opi-minimum-change-trace-design.md
git diff --no-index -- /dev/null docs/superpowers/plans/2026-08-11-opi-minimum-change-trace.md
git status --short -- .opi-impl-state.json .claude/skills/opi-implement scripts/opi-doc-check.py scripts/test_opi_doc_check.py docs/superpowers/specs/2026-08-11-opi-minimum-change-trace-design.md docs/superpowers/plans/2026-08-11-opi-minimum-change-trace.md
```

Each `git diff --no-index` command is expected to exit 1 because the file is
untracked; inspect its output as the file's complete proposed addition.

Verify manually:

- no task-owned diff changes the Phase 14-16 registered-source rows;
- `.opi-impl-state.json` has no change caused by this implementation;
- no `schema_version` or ledger-guard code changed;
- the diff contains only the approved trace contract, its tests, and the two
  approved design/plan artifacts;
- unrelated worktree changes remain untouched and unstaged.

- [ ] **Step 4: Report completion without committing**

Report test impact as `update`, list the three verification commands and their
results, and state explicitly that the canonical ledger was not modified. Do
not stage or commit.

If the user later explicitly authorizes one commit, first re-run `git status`,
stage only the nine files changed for this work, verify the staged diff, and
use:

```text
git commit -m "docs(opi-implement): add minimum-change admission trace"
```
