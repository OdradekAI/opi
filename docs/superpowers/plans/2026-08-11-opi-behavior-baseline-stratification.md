# Opi Behavior Baseline Stratification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Name existing public-seam tests and CI as Opi's deterministic acceptance baseline while narrowing `opi-eval` to explicitly classified real-provider fidelity canaries with safe comparison and resource-reporting rules.

**Architecture:** Keep the implementation ledger and deterministic tests as the only acceptance authority; do not add a baseline manifest or runtime seam. Extend the existing `opi-eval` Markdown contracts and result schema so generic provider canaries cannot be mistaken for acceptance proof, and guard the cross-file contract with the existing documentation checker.

**Tech Stack:** Markdown skill contracts, JSONL schema documentation, Python `unittest`, `scripts/opi-doc-check.py`.

---

## Preconditions and File Responsibilities

Read the approved design before editing:

- `docs/superpowers/specs/2026-08-11-opi-behavior-baseline-stratification-design.md`

Files and responsibilities:

- `.claude/skills/opi-eval/SKILL.md`: workflow authority, case taxonomy,
  comparison identity, history contract, and resource policy.
- `.claude/skills/opi-eval/references/test-cases.md`: case-local metadata and
  the admission rule for future runtime-fidelity cases.
- `.claude/skills/opi-eval/references/evaluator-prompt.md`: readonly evaluator
  judgment rules, including acceptance and comparison prohibitions.
- `.claude/skills/opi-eval/references/report-template.md`: visible case identity,
  comparison status, and resource record-only reporting.
- `docs/eval/README.md`: persisted JSONL field contract and operator guidance.
- `scripts/opi-doc-check.py`: cross-file semantic-token contract.
- `scripts/test_opi_doc_check.py`: happy-path and token-removal mutation tests.

Do not edit `.claude/skills/opi-eval/agents/openai.yaml`; it already has an
independent working-tree change. Do not create `docs/eval/history.jsonl`, run a
real provider, edit Rust or CI, or touch `.opi-impl-state.json`.

The user has not authorized commits. Omit commit steps during execution. If the
user later requests a commit, stage only the exact files changed by this plan.

### Task 1: Add the failing behavior-baseline documentation contract tests

**Files:**

- Modify: `scripts/test_opi_doc_check.py:12-105`
- Modify: `scripts/test_opi_doc_check.py:130-230`

- [ ] **Step 1: Add the expected cross-file token map**

Insert this constant after `AUDIT_MINIMUM_CHANGE_CONFORMANCE_REQUIRED`:

```python
EVAL_BEHAVIOR_BASELINE_REQUIRED = {
    ".claude/skills/opi-eval/SKILL.md": (
        "sole deterministic acceptance baseline",
        "`provider-fidelity`",
        "`runtime-fidelity`",
        "`case_id@revision + provider:model + OS/arch + run_mode + effective_tools`",
        "`incomparable`",
        "`record-only`",
        "`evaluator_model`",
        "`independence`",
        "Do not create a behavior-baseline manifest",
    ),
    ".claude/skills/opi-eval/references/test-cases.md": (
        "`provider-fidelity`",
        "`runtime-fidelity`",
        "`candy@1`",
        "`tool_chain@1`",
        "`context_retention@1`",
        "`criterion/scenario reference`",
        "`fidelity justification`",
    ),
    ".claude/skills/opi-eval/references/evaluator-prompt.md": (
        "fidelity signal, not deterministic acceptance evidence",
        "same comparison identity",
        "`incomparable`",
        "`record-only`",
        "do not calculate a delta",
        "must not affect the overall verdict",
    ),
    ".claude/skills/opi-eval/references/report-template.md": (
        "**Case class**",
        "**Case revision**",
        "**Criterion/scenario**",
        "**Comparison identity**",
        "**Comparison status**",
        "`record-only`",
    ),
    "docs/eval/README.md": (
        "not deterministic acceptance evidence",
        "`case_id`",
        "`case_class`",
        "`case_revision`",
        "`criterion_source`",
        "`comparison_identity`",
        "`comparison_status`",
        "`evaluator_model`",
        "`independence`",
    ),
}
```

- [ ] **Step 2: Add a fixture writer for the new contract**

Insert this method after
`write_audit_minimum_change_conformance_docs()`:

```python
    def write_eval_behavior_baseline_docs(self) -> None:
        for rel, tokens in EVAL_BEHAVIOR_BASELINE_REQUIRED.items():
            self.write(rel, "\n".join(tokens) + "\n")
```

- [ ] **Step 3: Add happy-path and mutation tests**

Insert these tests after the audit conformance tests:

```python
    def test_eval_behavior_baseline_contract_passes(self) -> None:
        self.write_eval_behavior_baseline_docs()

        checker = getattr(
            doc_check,
            "check_eval_behavior_baseline_contract",
            None,
        )
        self.assertIsNotNone(checker, "eval behavior-baseline checker must exist")
        checker()

        self.assertEqual([], doc_check.ERRORS)

    def test_eval_behavior_baseline_contract_requires_every_token(self) -> None:
        checker = getattr(
            doc_check,
            "check_eval_behavior_baseline_contract",
            None,
        )
        self.assertIsNotNone(checker, "eval behavior-baseline checker must exist")

        for rel, tokens in EVAL_BEHAVIOR_BASELINE_REQUIRED.items():
            for token in tokens:
                with self.subTest(rel=rel, token=token):
                    doc_check.ERRORS = []
                    self.write_eval_behavior_baseline_docs()
                    self.write(
                        rel,
                        "\n".join(item for item in tokens if item != token) + "\n",
                    )

                    checker()

                    self.assertIn(
                        f"{rel}: eval behavior-baseline contract "
                        f"missing semantic tokens {[token]!r}",
                        doc_check.ERRORS,
                    )
```

- [ ] **Step 4: Run only the new tests and verify RED**

Run:

```text
python -m unittest scripts.test_opi_doc_check.SkillContractTests.test_eval_behavior_baseline_contract_passes scripts.test_opi_doc_check.SkillContractTests.test_eval_behavior_baseline_contract_requires_every_token -v
```

Expected: both tests fail at `assertIsNotNone` because
`check_eval_behavior_baseline_contract` does not exist yet.

### Task 2: Implement the focused documentation checker

**Files:**

- Modify: `scripts/opi-doc-check.py:13-105`
- Modify: `scripts/opi-doc-check.py:628-648`

- [ ] **Step 1: Add the production token contract**

Insert this constant after `AUDIT_MINIMUM_CHANGE_CONFORMANCE_CONTRACT`; it must
match the test constant exactly:

```python
EVAL_BEHAVIOR_BASELINE_CONTRACT = {
    ".claude/skills/opi-eval/SKILL.md": (
        "sole deterministic acceptance baseline",
        "`provider-fidelity`",
        "`runtime-fidelity`",
        "`case_id@revision + provider:model + OS/arch + run_mode + effective_tools`",
        "`incomparable`",
        "`record-only`",
        "`evaluator_model`",
        "`independence`",
        "Do not create a behavior-baseline manifest",
    ),
    ".claude/skills/opi-eval/references/test-cases.md": (
        "`provider-fidelity`",
        "`runtime-fidelity`",
        "`candy@1`",
        "`tool_chain@1`",
        "`context_retention@1`",
        "`criterion/scenario reference`",
        "`fidelity justification`",
    ),
    ".claude/skills/opi-eval/references/evaluator-prompt.md": (
        "fidelity signal, not deterministic acceptance evidence",
        "same comparison identity",
        "`incomparable`",
        "`record-only`",
        "do not calculate a delta",
        "must not affect the overall verdict",
    ),
    ".claude/skills/opi-eval/references/report-template.md": (
        "**Case class**",
        "**Case revision**",
        "**Criterion/scenario**",
        "**Comparison identity**",
        "**Comparison status**",
        "`record-only`",
    ),
    "docs/eval/README.md": (
        "not deterministic acceptance evidence",
        "`case_id`",
        "`case_class`",
        "`case_revision`",
        "`criterion_source`",
        "`comparison_identity`",
        "`comparison_status`",
        "`evaluator_model`",
        "`independence`",
    ),
}
```

- [ ] **Step 2: Add the checker function**

Insert this function after
`check_audit_minimum_change_conformance_contract()`:

```python
def check_eval_behavior_baseline_contract() -> None:
    for rel, tokens in EVAL_BEHAVIOR_BASELINE_CONTRACT.items():
        require_tokens(
            rel,
            "eval behavior-baseline contract",
            tokens,
        )
```

- [ ] **Step 3: Register the checker in `main()`**

Add the call immediately after
`check_audit_minimum_change_conformance_contract()`:

```python
    check_eval_behavior_baseline_contract()
```

- [ ] **Step 4: Run the focused tests and verify GREEN**

Run:

```text
python -m unittest scripts.test_opi_doc_check.SkillContractTests.test_eval_behavior_baseline_contract_passes scripts.test_opi_doc_check.SkillContractTests.test_eval_behavior_baseline_contract_requires_every_token -v
```

Expected: `Ran 2 tests` and `OK`.

- [ ] **Step 5: Run the repository checker and verify the intended docs RED**

Run:

```text
python scripts/opi-doc-check.py
```

Expected: exit 1 with `eval behavior-baseline contract missing semantic tokens`
for the current `opi-eval` and `docs/eval` documents. There must be no new
failure outside this contract.

### Task 3: Stratify the eval workflow and case definitions

**Files:**

- Modify: `.claude/skills/opi-eval/SKILL.md:7-11`
- Modify: `.claude/skills/opi-eval/SKILL.md:127-168`
- Modify: `.claude/skills/opi-eval/SKILL.md:187-252`
- Modify: `.claude/skills/opi-eval/references/test-cases.md:1-161`
- Modify: `.claude/skills/opi-eval/references/evaluator-prompt.md:9-155`

- [ ] **Step 1: State the authority split near the top of `SKILL.md`**

Replace the opening description after `# opi-eval` with:

```markdown
End-to-end real-provider fidelity eval for the opi runtime. It compiles opi,
runs structured cases against a real LLM provider, collects NDJSON traces, and
dispatches an independent evaluator subagent to detect fidelity degradation.

Existing public-seam Rust tests and CI remain the
sole deterministic acceptance baseline. `opi-eval` supplements them only when
a registered criterion requires real-provider evidence; a generic canary is
never acceptance proof.

## Evidence authority

- `provider-fidelity` cases detect general model/provider regressions and do
  not cite Opi product criteria.
- `runtime-fidelity` cases cover a registered behavior that deterministic
  providers cannot faithfully reproduce. They must cite that criterion or
  acceptance scenario and state the remaining fidelity gap.
- Do not create a behavior-baseline manifest, copy production call sites into
  this skill, or generate eval cases from the implementation ledger.

Every case supplies metadata from `references/test-cases.md`. Historical
comparison is permitted only for the exact identity
`case_id@revision + provider:model + OS/arch + run_mode + effective_tools`.
```

- [ ] **Step 2: Replace the history example with the expanded per-case schema**

Use this exact example under `### History log`:

```json
{
  "version": "<semver>",
  "commit": "<short hash>",
  "date": "<YYYY-MM-DD>",
  "model": "<provider:model>",
  "platform": "<OS/arch>",
  "run_mode": "json",
  "effective_tools": ["<tool-name>"],
  "cases": {
    "<name>": {
      "case_id": "<name>",
      "case_class": "<provider-fidelity|runtime-fidelity>",
      "case_revision": 1,
      "criterion_source": null,
      "comparison_identity": "<case@revision + subject + environment>",
      "comparison_status": "<comparable|incomparable|record-only>",
      "verdict": "<PASS|DEGRADED|FAIL|ERROR>",
      "tokens_input": 0,
      "tokens_output": 0,
      "time_ms": 0,
      "tool_calls": 0
    }
  },
  "overall": "<PASS|REGRESSION|DEGRADED>",
  "evaluator": "<subagent-type>",
  "evaluator_model": "<provider:model of the evaluator>",
  "independence": "<independent-family|fresh-context-same-family|unknown>"
}
```

After the example, add:

```markdown
Keep `evaluator_model` and `independence` truthful according to the model
independence guardrail below.
```

- [ ] **Step 3: Replace delta analysis with identity-gated comparison**

Replace the current delta paragraph with:

```markdown
### Delta analysis

Compare a case only with prior samples having the same comparison identity.
Mark all other prior samples `incomparable`; do not calculate or narrate a
percentage delta. Opi version and commit remain recorded but are intentionally
outside the identity because cross-version comparison is the purpose.
```

- [ ] **Step 4: Replace the resource dimension policy**

Replace `### 5. Resource consumption` with:

```markdown
### 5. Resource consumption

Always record token usage, elapsed time, and tool-call count. By default score
this dimension `N/A` with resource status `record-only`; it does not change
the case or overall verdict.

A resource threshold is allowed only when a registered performance criterion
defines the budget, or when at least three prior samples share the comparison
identity and the resource policy is explicitly enabled. Historical thresholds
use the comparable cohort median, never the first run.
```

- [ ] **Step 5: Add case admission and comparison guardrails**

Add these bullets under `## Guardrails`:

```markdown
- Generic `provider-fidelity` cases are fidelity signals, not deterministic
  acceptance evidence.
- Admit a `runtime-fidelity` case only for a registered-source fidelity gap;
  do not duplicate a deterministic test for convenience.
- Never compare metrics across different case revisions, provider/models,
  OS/architectures, run modes, or effective tool sets.
```

- [ ] **Step 6: Add the case taxonomy and metadata contract to `test-cases.md`**

Replace the introductory text before the first `---` with:

```markdown
# Test Cases

Eval case definitions for `opi-eval`. These cases measure real-provider
fidelity; deterministic public-seam tests and CI remain the acceptance
baseline.

Each case specifies:

- a unique `case_id` and semantic `revision`;
- a class: `provider-fidelity` or `runtime-fidelity`;
- a `criterion/scenario reference`, or `N/A` for a generic canary;
- a `fidelity justification`;
- the prompt, effective tool set, fixtures, expected behavior, and evaluation
  criteria.

Adding a `provider-fidelity` case requires a distinct general provider risk.
Adding a `runtime-fidelity` case requires a registered criterion or acceptance
scenario plus a fidelity gap that deterministic tests cannot reproduce. Do not
copy production call sites or complete acceptance prose here; resolve them
through the referenced ledger scenario.

Increment the revision only when the prompt, assertions, run mode, or effective
tool set changes semantically. Editorial changes retain the revision.
```

- [ ] **Step 7: Add metadata to each existing case**

Immediately below each case heading, add the corresponding block:

```markdown
**Case ID**: `candy`
**Case identity**: `candy@1`
**Class**: `provider-fidelity`
**Revision**: `1`
**Criterion/scenario reference**: `N/A`
**Fidelity justification**: General real-provider reasoning and answer-format
behavior; it is not an Opi product criterion.
```

```markdown
**Case ID**: `tool_chain`
**Case identity**: `tool_chain@1`
**Class**: `provider-fidelity`
**Revision**: `1`
**Criterion/scenario reference**: `N/A`
**Fidelity justification**: General real-provider tool selection, argument
generation, and result chaining; it is not an Opi product criterion.
```

```markdown
**Case ID**: `context_retention`
**Case identity**: `context_retention@1`
**Class**: `provider-fidelity`
**Revision**: `1`
**Criterion/scenario reference**: `N/A`
**Fidelity justification**: General real-provider long-prompt attention and
detail retention; it is not an Opi product criterion.
```

- [ ] **Step 8: Make all three resource criteria record-only**

Replace the `Resources` row in each case with this wording:

```markdown
| Resources | Record token, timing, and tool-call observations; score `N/A` with `record-only` resource status. |
```

- [ ] **Step 9: Tighten evaluator authority, input, output, and scoring**

Make these exact changes in `evaluator-prompt.md`:

1. Add after the readonly sentence:

```markdown
A generic provider canary is a
fidelity signal, not deterministic acceptance evidence. Only a
`runtime-fidelity` case whose registered criterion requires real-provider
evidence may contribute to that criterion's admission result.
```

2. Add these received fields to the input list:

```markdown
4. **Case metadata** -- case id, class, revision, criterion/scenario reference,
   and fidelity justification.
5. **Comparison data** -- comparison identity, comparison status, and any prior
   samples with the same comparison identity.
```

3. Replace the resource-consumption rules with:

```markdown
### 5. Resource consumption

- Always cite observed tokens, elapsed time, and tool-call count.
- Default to **N/A** with resource status `record-only`.
- A `record-only` resource result must not affect the overall verdict.
- Apply a threshold only when the input identifies a registered performance
  budget, or explicitly enables a median derived from at least three prior
  samples with the same comparison identity.
- Mark a mismatched prior sample `incomparable` and do not calculate a delta.
```

4. Add these lines below each case heading in the output template:

```markdown
**Case class**: <provider-fidelity | runtime-fidelity>
**Case revision**: <positive integer>
**Criterion/scenario**: <registered reference | N/A>
**Comparison identity**: <complete identity>
**Comparison status**: <comparable | incomparable | record-only>
```

5. Add this scoring rule:

```markdown
- Exclude N/A dimensions from the aggregate pass-rate denominator. A
  record-only resource dimension cannot escalate a case or overall verdict.
```

6. Add this constraint:

```markdown
- Compare history only when the complete comparison identity matches. For
  `incomparable` samples, report observed values without a delta claim.
```

- [ ] **Step 10: Run the focused contract tests**

Run:

```text
python -m unittest scripts.test_opi_doc_check.SkillContractTests.test_eval_behavior_baseline_contract_passes scripts.test_opi_doc_check.SkillContractTests.test_eval_behavior_baseline_contract_requires_every_token -v
```

Expected: `Ran 2 tests` and `OK` because the tests use isolated fixtures.

- [ ] **Step 11: Run the repository checker and confirm only report/history fields remain RED**

Run:

```text
python scripts/opi-doc-check.py
```

Expected: exit 1. The remaining eval behavior-baseline failures are limited to
`.claude/skills/opi-eval/references/report-template.md` and
`docs/eval/README.md`.

### Task 4: Synchronize reporting and persisted history documentation

**Files:**

- Modify: `.claude/skills/opi-eval/references/report-template.md:12-120`
- Modify: `docs/eval/README.md:1-82`

- [ ] **Step 1: Add case and comparison fields to the report template**

Add these columns to the summary table before the six dimensions:

```markdown
| Case | Class | Revision | Comparison | Correctness | Tools | Context | Efficiency | Resources | Errors | Overall |
|------|-------|----------|------------|-------------|-------|---------|------------|-----------|--------|---------|
| <name> | <provider-fidelity or runtime-fidelity> | <N> | <comparable, incomparable, or record-only> | <verdict> | <verdict> | <verdict> | <verdict> | <verdict> | <verdict> | <verdict> |
```

Replace the existing summary table rather than retaining two copies.

- [ ] **Step 2: Add case metadata to Detailed Findings**

Insert this block immediately after `### Case: <name>`:

```markdown
**Case class**: <provider-fidelity | runtime-fidelity>
**Case revision**: <positive integer>
**Criterion/scenario**: <registered reference | N/A>
**Comparison identity**: <case@revision + subject + environment>
**Comparison status**: <comparable | incomparable | record-only>
```

Add this sentence after the dimension table:

```markdown
When Resources is `N/A` / `record-only`, retain the observed metrics but do not
include that dimension in pass rate or verdict escalation.
```

- [ ] **Step 3: Gate Version Delta on comparison identity**

Replace the Version Delta instruction with:

```markdown
_Present only when history.jsonl contains a prior sample with the same complete
comparison identity. Mark other samples `incomparable` and omit percentage
deltas._
```

Keep the existing metric table as the display shape for a valid comparable
sample.

- [ ] **Step 4: State the persisted authority in `docs/eval/README.md`**

Replace the opening paragraph with:

```markdown
This directory stores real-provider fidelity results produced by the
`opi-eval` skill (`.claude/skills/opi-eval/SKILL.md`). Generic canaries are
runtime signals, not deterministic acceptance evidence; public-seam tests and
CI remain the acceptance baseline.
```

- [ ] **Step 5: Replace the JSONL example with the synchronized schema**

Use this complete example:

```json
{
  "version": "0.7.3",
  "commit": "abc1234",
  "date": "2026-07-07",
  "model": "anthropic:claude-sonnet-4-5-20250514",
  "platform": "linux/x86_64",
  "run_mode": "json",
  "effective_tools": [],
  "cases": {
    "candy": {
      "case_id": "candy",
      "case_class": "provider-fidelity",
      "case_revision": 1,
      "criterion_source": null,
      "comparison_identity": "candy@1|anthropic:claude-sonnet-4-5-20250514|linux/x86_64|json|none",
      "comparison_status": "record-only",
      "verdict": "PASS",
      "tokens_input": 1200,
      "tokens_output": 450,
      "time_ms": 5600,
      "tool_calls": 0
    }
  },
  "overall": "PASS",
  "evaluator": "readonly-subagent",
  "evaluator_model": "openai:gpt-5.6",
  "independence": "independent-family",
  "compaction_triggered": false,
  "retries": 0
}
```

- [ ] **Step 6: Replace the field-reference table with the synchronized fields**

Use these rows, retaining the existing heading:

```markdown
| Field | Type | Description |
|-------|------|-------------|
| `version` | string | Workspace semver at time of eval; recorded but excluded from comparison identity |
| `commit` | string | Short git hash of the tested binary; recorded but excluded from comparison identity |
| `date` | string | ISO 8601 date (UTC) |
| `model` | string | Subject `provider:model` used for the eval |
| `platform` | string | Subject OS and architecture |
| `run_mode` | string | Opi run mode used by the case |
| `effective_tools` | array | Actual tool names enabled for the case |
| `cases` | object | Per-case results keyed by case id |
| `cases.<name>.case_id` | string | Stable case id |
| `cases.<name>.case_class` | enum | `provider-fidelity` or `runtime-fidelity` |
| `cases.<name>.case_revision` | integer | Positive semantic revision |
| `cases.<name>.criterion_source` | string or null | Registered criterion/scenario reference for runtime-fidelity cases |
| `cases.<name>.comparison_identity` | string | Case revision plus subject and environment identity |
| `cases.<name>.comparison_status` | enum | `comparable`, `incomparable`, or `record-only` |
| `cases.<name>.verdict` | enum | `PASS`, `DEGRADED`, `FAIL`, or `ERROR` |
| `cases.<name>.tokens_input` | number | Input tokens consumed |
| `cases.<name>.tokens_output` | number | Output tokens consumed |
| `cases.<name>.time_ms` | number | Wall-clock milliseconds |
| `cases.<name>.tool_calls` | number | Count of tool executions |
| `overall` | enum | `PASS`, `DEGRADED`, or `REGRESSION` |
| `evaluator` | string | Type of readonly evaluator subagent used |
| `evaluator_model` | string | Evaluator `provider:model` identity |
| `independence` | enum | `independent-family`, `fresh-context-same-family`, or `unknown` |
| `compaction_triggered` | boolean | Whether any case triggered compaction |
| `retries` | number | Total auto-retry count across all cases |
```

- [ ] **Step 7: Add the comparison and resource rules below the table**

Insert:

```markdown
## Comparison rules

Compare history only when `case_id@case_revision`, subject provider/model,
OS/architecture, run mode, and effective tool set all match. Otherwise set
`comparison_status` to `incomparable` and omit percentage deltas.

Resource metrics are `record-only` by default. They become threshold-bearing
only for a registered performance budget or an explicitly enabled median from
at least three comparable prior samples. Do not create an empty
`history.jsonl`; the first real eval creates it.
```

- [ ] **Step 8: Run the complete documentation checker**

Run:

```text
python scripts/opi-doc-check.py
```

Expected: `opi documentation contracts: PASS`.

### Task 5: Final verification and scope audit

**Files:**

- Verify only; no new files.

- [ ] **Step 1: Run the complete documentation-checker unit suite**

Run:

```text
python -m unittest scripts.test_opi_doc_check -v
```

Expected: `Ran 14 tests` and `OK`.

- [ ] **Step 2: Run the repository documentation contract**

Run:

```text
python scripts/opi-doc-check.py
```

Expected: `opi documentation contracts: PASS`.

- [ ] **Step 3: Check whitespace for every planned file**

Run:

```text
git diff --check -- .claude/skills/opi-eval/SKILL.md .claude/skills/opi-eval/references/test-cases.md .claude/skills/opi-eval/references/evaluator-prompt.md .claude/skills/opi-eval/references/report-template.md docs/eval/README.md scripts/opi-doc-check.py scripts/test_opi_doc_check.py
```

Expected: exit 0 with no output.

- [ ] **Step 4: Confirm the protected sidecar was not absorbed into the task**

Run:

```text
git diff -- .claude/skills/opi-eval/agents/openai.yaml
```

Expected: only the pre-existing one-line `default_prompt` change remains; no
other sidecar line was changed by this plan.

- [ ] **Step 5: Review exact task scope**

Run:

```text
git status --short -- .claude/skills/opi-eval docs/eval scripts/opi-doc-check.py scripts/test_opi_doc_check.py docs/superpowers/specs/2026-08-11-opi-behavior-baseline-stratification-design.md docs/superpowers/plans/2026-08-11-opi-behavior-baseline-stratification.md
```

Expected planned modifications:

```text
 M .claude/skills/opi-eval/SKILL.md
 M .claude/skills/opi-eval/references/evaluator-prompt.md
 M .claude/skills/opi-eval/references/report-template.md
 M .claude/skills/opi-eval/references/test-cases.md
 M docs/eval/README.md
 M scripts/opi-doc-check.py
 M scripts/test_opi_doc_check.py
```

The status may also show the pre-existing modified
`.claude/skills/opi-eval/agents/openai.yaml` and the approved design/plan files.
Do not stage or clean any of them without explicit user authorization.

- [ ] **Step 6: Record the test impact in the handoff**

Report:

```text
Test impact: update -- added behavior-baseline documentation-contract happy-path and token-removal mutation coverage; no Rust tests and no real-provider eval.
```
