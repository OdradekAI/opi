# opi-implement Init Verify-and-Fold Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a wayfinder-style verify-and-fold stage (`A.init.2e/2f`) to the opi-implement init flow so the draft task graph arrives at the `A.init.3` human review gate pre-corrected, cutting the multi-round `edit-task`/`apply-rule`/`import-draft` rework loop.

**Architecture:** Tiered — a cheap single-agent six-lens audit (default) and a deep multi-lens adversarial Workflow (`--deep-init`). Both run between `A.init.2d` (extract) and `A.init.3` (human gate), fold high-confidence field-edit findings into the draft with `inference_notes` provenance, and leave the gate authoritative. No Rust changes; no `schema_version` bump.

**Tech Stack:** Markdown skill docs (`.claude/skills/opi-implement/`), one JS Workflow script (`scripts/opi-init-verify.workflow.js`), existing doc-guard test suites as the regression gate.

**Spec:** `docs/superpowers/specs/2026-07-12-opi-implement-init-verify-design.md` (normative; hardened by `wf_2e12dfa5-ce6`).

**Commit cadence:** This change is interdependent — `skill.md` and `initializer.md` reference `init-verify.md`, which references the script. Per-task commits would create broken intermediate states. Tasks 1–6 create/edit files without committing; Task 7 verifies and makes ONE consolidated commit. (Per the repo's `CLAUDE.md` git rule, the commit still requires explicit user authorization at execution time.)

---

## File Structure

| File | Responsibility | Task |
|---|---|---|
| `scripts/opi-init-verify.workflow.js` | **New.** Deep-path Workflow: 6 lens agents fan out → deterministic fold → adversarial verify → synthesize. | 1 |
| `.claude/skills/opi-implement/references/init-verify.md` | **New.** Operational reference read at `A.init.2e`: lens charters, finding schema, fold matrix + overrides, cheap/deep protocols, report format, guardrails. | 2 |
| `.claude/skills/opi-implement/references/initializer.md` | Insert `A.init.2e` / `A.init.2f` steps + pointer. | 3 |
| `.claude/skills/opi-implement/references/ledger-schema.md` | Document `init_verification` field + the L5 `inference_notes` encoding. | 4 |
| `.claude/skills/opi-implement/references/anti-patterns.md` | One row: never run verify-and-fold post-confirmation. | 5 |
| `.claude/skills/opi-implement/skill.md` | `--deep-init` in Invocation + one pointer line. | 6 |

No production Rust code. No new crate dependencies. `docs/opi-spec.md` is **not** touched → no phase4/phase6 ledger hash re-sync required, and no existing ledger `spec_files_sha256` drift.

---

## Task 1: Create the deep-path Workflow script

**Files:**
- Create: `scripts/opi-init-verify.workflow.js`

- [ ] **Step 1: Create the script file with exactly this content**

```js
export const meta = {
  name: 'opi-init-verify',
  description: 'Adversarial multi-lens verification of an opi-implement init task-graph draft',
  phases: [
    { title: 'Lens audit' },
    { title: 'Fold' },
    { title: 'Verify' },
    { title: 'Synthesize' },
  ],
}

const draft = args.draftTasks
const sourcePath = args.sourceDesignPath

const FINDINGS_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['findings'],
  properties: {
    findings: {
      type: 'array',
      items: {
        type: 'object',
        additionalProperties: false,
        required: ['lens', 'task_id', 'field', 'problem', 'severity', 'suggested_fix', 'source_citation', 'confidence'],
        properties: {
          lens: { type: 'string' },
          task_id: { type: 'string' },
          field: { type: 'string' },
          problem: { type: 'string' },
          severity: { enum: ['high', 'medium', 'low'] },
          suggested_fix: { type: 'string' },
          source_citation: { type: 'string', pattern: '(§|#)' },
          confidence: { enum: ['high', 'medium', 'low'] },
        },
      },
    },
  },
}

const VERDICT_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['task_id', 'field', 'accepted', 'reason'],
  properties: {
    task_id: { type: 'string' },
    field: { type: 'string' },
    accepted: { type: 'boolean' },
    reason: { type: 'string' },
  },
}

const REPORT_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['summary', 'folded', 'flagged_for_human', 'rejected'],
  properties: {
    summary: { type: 'string' },
    folded: { type: 'array', items: { type: 'object', additionalProperties: true } },
    flagged_for_human: { type: 'array', items: { type: 'object', additionalProperties: true } },
    rejected: { type: 'array', items: { type: 'object', additionalProperties: true } },
  },
}

const LENSES = [
  { key: 'dod-precision', charter: 'L1 DoD precision: detect vague verbs and missing observable assertions; suggest concrete command/API/artifact/call-site/runtime/diagnostics/error expansions.' },
  { key: 'tier-boundary', charter: 'L2 tier/crate-boundary: enforce the opi-ai/opi-agent/opi-coding-agent ownership invariant and task_owned_paths containment.' },
  { key: 'forbidden-scope', charter: 'L3 forbidden-scope: ensure every Non-Goal is a forbidden_scope inference_note and no task risks implementing a non-goal.' },
  { key: 'coverage', charter: 'L4 coverage: every Goal/SC/workflow has an owning task with acceptance_scenarios + production_call_sites; audit composite-row splits.' },
  { key: 'dependency-sequencing', charter: 'L5 dependency/sequencing: depends_on matches Sequencing; no cycles; extract defer/split/residual with re-trigger conditions.' },
  { key: 'substrate-product', charter: 'L6 substrate-vs-product: substrate_only correctness; no product scenario closed by substrate-only evidence.' },
]

phase('Lens audit')
const lensResults = await parallel(LENSES.map((l) => () =>
  agent(
    'You are an init-verify lens auditing an opi-implement task-graph draft.\n' +
    'Read the source phase design doc at ' + sourcePath + ' in full.\n' +
    'Apply lens ' + l.key + ': ' + l.charter + '\n' +
    'Hard rules: never propose implementing a Non-Goal; never invent tasks or scope beyond the source; ' +
    'emit one finding per problem; cite the source section (use § or #) for every finding and verify the cited heading appears verbatim in the source.\n' +
    'Draft task graph JSON:\n' + JSON.stringify(draft),
    { label: 'lens:' + l.key, phase: 'Lens audit', schema: FINDINGS_SCHEMA }
  )
))
const allFindings = lensResults.filter(Boolean).flatMap((r) => r.findings)

const foldable = allFindings.filter((f) =>
  f.severity === 'high' || (f.severity === 'medium' && f.confidence === 'high'))
const flagged = allFindings.filter((f) => !foldable.includes(f))

phase('Verify')
const verdicts = await parallel(foldable.map((f) => () =>
  agent(
    'Adversarially verify this proposed init-verify correction. Try to REJECT it.\n' +
    'Source design doc: ' + sourcePath + '\n' +
    'Task: ' + f.task_id + '  Field: ' + f.field + '\n' +
    'Problem: ' + f.problem + '\n' +
    'Proposed fix: ' + f.suggested_fix + '\n' +
    'Citation: ' + f.source_citation + '\n' +
    'REJECT if the fix contradicts the source, implements a Non-Goal, invents scope beyond the source, ' +
    'or requires task-graph surgery (adding/removing/restructuring tasks). Default to accepted=false if uncertain.',
    { label: 'verify:' + f.task_id + ':' + f.field, phase: 'Verify', schema: VERDICT_SCHEMA }
  )
    .then((v) => ({ finding: f, verdict: v }))
    .catch(() => ({ finding: f, verdict: { task_id: f.task_id, field: f.field, accepted: false, reason: 'verify-agent-error' } }))
))
const confirmed = verdicts.filter(Boolean).filter((v) => v.verdict.accepted).map((v) => v.finding)
const rejected = verdicts.filter(Boolean).filter((v) => !v.verdict.accepted)
  .map((v) => ({ finding: v.finding, reason: v.verdict.reason }))

phase('Synthesize')
const report = await agent(
  'Synthesize the opi-implement init-verify report.\n' +
  'Confirmed folds (apply to draft with inference_notes provenance):\n' + JSON.stringify(confirmed) + '\n' +
  'Flagged for human review (not auto-applied):\n' + JSON.stringify(flagged) + '\n' +
  'Rejected by adversarial verify:\n' + JSON.stringify(rejected) + '\n' +
  'Write a concise summary plus the three lists.',
  { label: 'synthesize', phase: 'Synthesize', schema: REPORT_SCHEMA }
)

return { confirmed_folds: confirmed, flagged_for_human: flagged, rejected, report }
```

This is identical to spec §8 with two hardened refinements folded in: the lens prompt requires `§`/`#` citations whose heading appears verbatim in the source, and the verify prompt also rejects task-graph-surgery fixes (so the deep path enforces the §6 surgery override independently of the calling agent).

- [ ] **Step 2: Smoke-test the script on a synthetic draft**

Invoke the Workflow tool with `scriptPath` and a small synthetic draft. The draft is one deliberately-vague task plus one correct task, so at least L1 (DoD precision) should fire.

```
Workflow({
  scriptPath: "scripts/opi-init-verify.workflow.js",
  args: {
    "phase": 14,
    "sourceDesignPath": "docs/superpowers/specs/2026-07-11-phase14-provider-auth-design.md",
    "draftTasks": [
      {
        "id": "14.1", "phase": 14, "title": "credential store", "crate": "opi-ai",
        "definition_of_done": "supports credentials",
        "depends_on": [], "tier": "library", "commit_type": "feat",
        "evaluator_required": false, "substrate_only": false,
        "acceptance_scenarios": [], "production_call_sites": [],
        "task_owned_paths": ["crates/opi-ai/**"], "inference_notes": []
      },
      {
        "id": "14.2", "phase": 14, "title": "oauth login", "crate": "opi-ai",
        "definition_of_done": "opi auth login <provider> writes an AuthDescriptor; exit 0 on success, exit 3 on keyring failure",
        "depends_on": ["14.1"], "tier": "library", "commit_type": "feat",
        "evaluator_required": false, "substrate_only": false,
        "acceptance_scenarios": [], "production_call_sites": [],
        "task_owned_paths": ["crates/opi-ai/**"], "inference_notes": []
      }
    ]
  }
})
```

Expected: the Workflow completes (does not throw) and returns an object with keys `confirmed_folds`, `flagged_for_human`, `rejected`, `report`. At least one finding references task `14.1` field `definition_of_done` (the vague verb "supports"). Capture the run id from the Workflow tool's result envelope (the `wf_…` value) — this proves the `init_verification.wf_ref` extraction path works.

If the Workflow errors on launch, the most likely cause is a script parse problem (forbidden `Date.now`/`Math.random`, non-literal `meta`, or a schema typo). Re-read the script against the Workflow tool constraints and re-run.

- [ ] **Step 3: Do NOT commit yet** (consolidated commit is Task 7).

---

## Task 2: Create the operational reference `init-verify.md`

**Files:**
- Create: `.claude/skills/opi-implement/references/init-verify.md`

- [ ] **Step 1: Create the file with exactly this content**

````markdown
# Init Verify-and-Fold Reference

Read at `A.init.2e`/`A.init.2f` — the stage between draft extraction
(`A.init.2a/2c/2d`) and the human task-graph review gate (`A.init.3`).

**Hard rule:** verify-and-fold runs ONLY pre-confirmation, on a draft graph that
is not yet a reviewed contract. It MUST NOT run after `A.init.3` confirmation
(anti-pattern row in `anti-patterns.md`). It pre-corrects the draft; it does not
confirm it.

## Inputs

- The draft task array produced by `A.init.2a/2c/2d` (held in memory / written to
  `.opi-impl-state.draft.json`).
- The active phase's registered source design doc path (from the `skill.md`
  registry table).

## Modes

- **Cheap (default):** one agent runs the six lenses sequentially, then folds
  once. Bounded cost; runs on every init.
- **Deep (`--deep-init`):** the Workflow at `scripts/opi-init-verify.workflow.js`
  fans the six lenses out in parallel, adversarially verifies each foldable
  finding, and synthesizes a report. Opt-in.

## The Six Lenses

Every lens MUST: read the source design doc at the registered path in full;
never propose implementing a Non-Goal; never invent tasks or scope beyond the
source; emit one finding per problem with a `source_citation` that follows the
§6 citation grammar and whose heading appears verbatim in the source.

### L1 — DoD precision (kills the 30%)
For each task `definition_of_done`: detect vague verbs (`works`, `supports`,
`loads`, `integrates`, `bridges`, `productizes`, `handles`) and missing
observable assertions. The suggested fix must name the command / API entry
point, persisted artifact, production call site, runtime effect, diagnostics,
and negative / error behavior where relevant. **Severity high** when a vague
verb is present — decoupled from `substrate_only`. Substrate-only tasks are
fold-eligible too: the suggested fix names the observable substrate assertion
(the parsed shape, the persisted struct, the error returned on malformed input).
A vague verb may stand only when the review gate *explicitly* accepts it as
substrate-only — that is an `A.init.3` human decision, not a severity demotion
here.

### L2 — tier / crate-boundary (kills the 21%)
For each task: verify `crate` + `tier` match the load-bearing invariant
(`opi-ai` owns provider/auth wire-up; `opi-agent` owns runtime/sessions/hooks
but must not construct providers; `opi-coding-agent` owns CLI/TUI/RPC wiring).
Verify `task_owned_paths` do not leak into a crate the task does not own.
Cross-reference the design doc's "Implementation Priority and Crate Boundaries"
section. **Severity high** when the invariant is violated.

### L3 — forbidden-scope / non-goal (kills the 16%)
Two sub-checks, non-overlapping with `A.init.2d` (which already converts
Non-Goals into `forbidden_scope` notes) and with the `ledger-schema.md`
validation rule (which accepts a `forbidden_scope` note OR a phase addendum):

- **Coverage** — emit a finding only when a Non-Goal lacks BOTH a
  `forbidden_scope` inference note AND a phase verification addendum (report the
  delta vs the validation rule; an addendum-only Non-Goal must NOT false-positive).
- **Risk** — fire on a token trigger: a task's `acceptance_scenarios` or
  `definition_of_done` contains a token from the Non-Goal vocabulary (`npm`,
  `marketplace`, `OAuth`, `telemetry`, `sandboxing`, `web-UI parity`, `pi session
  compatibility`, `workflow tools`, `MCP core`, `plan mode core`, `sub-agent
  core`). Semantic "resembles a non-goal" hunches without a token match are
  `confidence = low` → flag-only.

**Severity high** on a token match; the suggested fix is the `forbidden_scope`
note text plus citation.

### L4 — coverage / completeness (kills the 12% + gaps)
For each Goal / Success Criterion / named user workflow in the design doc:
confirm there is at least one owning task with an `acceptance_scenarios` entry
and a `production_call_sites` entry. Flag three states: `missing` (no owner),
`substrate-only-owner` (a runtime criterion covered only by a `substrate_only`
task — red flag #11), and `deferred-without-citation`. Also audit composite-row
detection (`A.init.2a`): did any composite row escape splitting?

**Severity** (all three pinned): `missing` = **high**; `substrate-only-owner` =
**high** regardless of confidence (red flag #11); `deferred-without-citation` =
**medium**.

**Fold-vs-flag** is governed by the task-graph-surgery override (§Fold matrix),
not severity alone. A `substrate-only-owner` finding folds at high severity when
its suggested fix is a field edit (e.g. the task is mis-flagged and the lens can
supply the real `production_call_sites`); it is always flag-for-human when the
only fix is adding a vertical-slice task. `missing`-owner and surgery-only
findings are always flag-for-human and surface as REFUSE-triggering items at
`A.init.3` via the existing "REFUSE confirm-all while any source criterion is
missing" rule.

### L5 — dependency / sequencing (+ wayfinder devices)
Verify `depends_on` matches the design doc's Sequencing section; flag missing,
spurious, and cyclic deps. Capture cross-ticket interactions as
`inference_notes`. Extract defers / splits / residuals with this procedure:

1. **Recognize** — pattern-match residual / defer sentences against `deferred to
   <X>`, `re-sharpen when <Y>`, `<Z> appears`, `deferred follow-up`. The matched
   `<Y>`/`<Z>` clause IS the re-trigger condition.
2. **Trigger-less defers** — when a defer states no trigger clause, record the
   re-trigger as `null`, set severity **medium** AND confidence **low** (so the
   fold matrix routes it to flag-for-human). Never invent a trigger.
3. **Encode** — `inference_notes` entry: `field` ∈ {`deferred`, `split`,
   `residual`}, `reason` = `"<verb>: trigger=<clause|null>"`.
4. **Consumer** — `A.init.3` surfaces `null`-trigger defers in the report;
   reinit re-evaluates them.

**Severity high** for cycles or a missing hard dependency.

### L6 — substrate vs. product (red flags #11 / #12)
For each task: verify `substrate_only` is correctly set; verify no
product-facing acceptance scenario relies solely on a `substrate_only` task;
verify every runtime / startup / CLI / session / provider claim has a
`production_call_sites` entry. **Severity high** when a product scenario would
be closed by substrate-only evidence.

## Finding schema

Every finding is:

```json
{
  "lens": "dod-precision | tier-boundary | forbidden-scope | coverage | dependency-sequencing | substrate-product",
  "task_id": "14.1",
  "field": "definition_of_done",
  "problem": "vague verb 'supports' without an observable assertion",
  "severity": "high | medium | low",
  "suggested_fix": "expand to: '<concrete assertion>'",
  "source_citation": "<file> §<section>  |  <file>#<heading>",
  "confidence": "high | medium | low"
}
```

## Fold matrix (applies identically to cheap and deep paths)

| Severity | Confidence | Action |
|---|---|---|
| high | any | **fold** |
| medium | high | **fold** |
| medium | medium / low | flag for human |
| low | any | flag for human |

- **Fold** = apply `suggested_fix` to the named draft field and append an
  `inference_notes` entry
  `{ "field": <field>, "reason": "init-verify <lens>: <problem>", "source": <source_citation> }`.
- **Flag for human** = leave the field unchanged; surface in the report.

**Task-graph-surgery override** (takes precedence over the matrix): a finding
whose `suggested_fix` requires adding, removing, or restructuring tasks — rather
than editing an existing task's field — is **always flag for human** regardless
of severity. This governs L4 `missing`-owner, surgery-only
`substrate-only-owner`, and surgery-only `deferred-without-citation` cases; they
surface as REFUSE-triggering items at `A.init.3`. A high-severity finding whose
fix IS a field edit still folds.

**Citation grammar** (both paths): `source_citation` must follow
`<file>#<heading>` or `<file> §<section>`. The fold step performs the syntactic
grammar check and demotes non-conforming findings to flag-only. Content-existence
(the cited heading appears verbatim in the source) is verified by the lens agent
at emit time on both paths — lens agents hold the source file open; the deep-path
Workflow script has no filesystem access and so does only the syntactic check.

## Cheap-path protocol (`A.init.2e` default)

One agent, sequential:

1. Load the draft graph and the registered phase design doc path.
2. For each lens L1–L6 in order, run the lens charter against the **original
   unmodified draft** (every lens reads the same pre-fold snapshot, for isolation
   and determinism) and collect findings. Each lens verifies its own
   `source_citation`s resolve against the source file before emitting.
3. Apply the fold matrix **once** across all collected findings (plain logic):
   fold high + medium-high-confidence findings whose citation parses and whose
   fix is a field edit; flag the rest. Per-lens folding is intentionally NOT
   used.
4. Record every fold in `inference_notes` with provenance.
5. Emit the report (see Report format) and write the folded draft to
   `.opi-impl-state.draft.json`.

## Deep-path protocol (`A.init.2e` with `--deep-init`)

Invoke the Workflow tool with `scriptPath: "scripts/opi-init-verify.workflow.js"`
and `args = { draftTasks, sourceDesignPath, phase }`. The script fans L1–L6 out
in parallel, deterministically folds, adversarially verifies each foldable
finding (default-reject on uncertainty; surgery fixes rejected), and synthesizes
a report.

The script returns `{ confirmed_folds, flagged_for_human, rejected, report }` —
note the return object does NOT carry the run id. Read the run id from the
Workflow tool's result envelope (the `wf_…` value, conventionally `runId`),
copy it into `init_verification.wf_ref`, then apply `confirmed_folds` to the
draft with `inference_notes` provenance.

## `A.init.2f` Fold + Report

After either path:

1. Apply the confirmed folds to the draft (field edits only), each recorded in
   `inference_notes`.
2. Record `init_verification` on the ledger:
   `{ mode, wf_ref (null on cheap), folded_count, flagged_count, rejected_count, ran_at }`.
3. Write the folded draft to `.opi-impl-state.draft.json`.
4. Emit the report block at `A.init.3`.

## Report format (printed at `A.init.3`, above the draft table)

- mode (`cheap` / `deep` + `wf_ref` if deep);
- folded: N corrections applied, grouped by lens, with source citations;
- flagged for human: M findings not auto-applied (the user's remaining manual
  work — a bounded list, not an open re-edit loop);
- rejected (deep only): K foldable findings the adversarial pass rejected, with
  reasons;
- null-trigger defers (L5): listed for human trigger specification.

The existing `A.init.3` refusal rule still holds: REFUSE `confirm-all` while any
source criterion/workflow is `missing`, or while any runtime criterion is covered
only by a `substrate_only` task.

## Guardrails

- **Pre-confirmation only.** Never run after `A.init.3` confirmation.
- **No silent rewrites.** Every fold is in `inference_notes`; the report lists
  folded, flagged, rejected separately.
- **No scope invention.** Lenses never invent tasks or implement Non-Goals.
  Findings without a resolving citation are flagged, not folded.
- **Deep is opt-in.** `--deep-init` triggers the Workflow; cheap is the default.
- **Adversarial verify before any deep fold.** No foldable finding reaches the
  draft unless an independent agent failed to reject it.
- **`A.init.3` remains authoritative.** The user can still `edit-task` / `abort`
  / re-export the draft.
````

- [ ] **Step 2: Self-check the reference**

Re-read the file and confirm: (a) every lens has an explicit severity rule; (b)
the surgery override and citation grammar appear under "Fold matrix"; (c) the
cheap-path protocol says "original unmodified draft" and "fold matrix once"; (d)
the deep-path protocol names the `wf_ref` extraction from the result envelope;
(e) the L5 procedure has all four steps including the consumer. Fix any gap
inline.

- [ ] **Step 3: Do NOT commit yet.**

---

## Task 3: Wire `A.init.2e`/`A.init.2f` into `initializer.md`

**Files:**
- Modify: `.claude/skills/opi-implement/references/initializer.md` (insert before `### A.init.3 Task-Graph Review Gate`)

- [ ] **Step 1: Insert the two new sub-sections immediately before `### A.init.3 Task-Graph Review Gate`**

Locate the line `### A.init.3 Task-Graph Review Gate` (currently around line 123,
right after the `A.init.2a Composite Row Detection` section ends). Insert this
block directly above it:

```markdown
### A.init.2e Verify

Run the six-lens audit over the draft graph (post-`A.init.2a/2c/2d`,
pre-review) against the active phase's registered source design doc. Cheap path
(default) = one agent; deep path (`--deep-init`) = the Workflow at
`scripts/opi-init-verify.workflow.js`. Read `references/init-verify.md` for the
lens charters, finding schema, fold matrix, citation grammar, and the
cheap/deep protocols.

### A.init.2f Fold + Report

Apply high- and medium-high-confidence findings whose `suggested_fix` is a field
edit on an existing task as draft edits, each recorded in `inference_notes` with
provenance. Findings whose fix requires task-graph surgery, whose citation does
not resolve, or that the deep-path adversarial verify rejected, are flagged for
human (never folded). Write the folded draft to `.opi-impl-state.draft.json`,
record `init_verification` (mode, `wf_ref` on deep, counts, timestamp), and emit
the verification report block at `A.init.3`. This is the only mutation of the
draft between extraction and the human review gate.
```

- [ ] **Step 2: Verify the insertion**

Read the surrounding section and confirm the ordering reads `A.init.2a` → `A.init.2e` → `A.init.2f` → `A.init.3`, and that `A.init.2e` points at `references/init-verify.md`.

- [ ] **Step 3: Do NOT commit yet.**

---

## Task 4: Document the ledger additions in `ledger-schema.md`

**Files:**
- Modify: `.claude/skills/opi-implement/references/ledger-schema.md`

- [ ] **Step 1: Extend the `inference_notes` Field Semantics row**

Locate this row in the **Field Semantics** table:

```
| `tasks[].inference_notes` | array | const | Reasons for inferred fields. Phase non-goal guards are recorded with `field = "forbidden_scope"` and an exact source heading. |
```

Replace it with:

```
| `tasks[].inference_notes` | array | const | Reasons for inferred fields. Phase non-goal guards use `field = "forbidden_scope"` with an exact source heading. Init-verify (L5) defer/split/residual devices use `field` ∈ {`deferred`,`split`,`residual`} with `reason` packed as `"<verb>: trigger=<clause|null>"` (a `null` trigger means the human must specify it at `A.init.3`). |
```

- [ ] **Step 2: Add the `init_verification` row**

In the **Field Semantics** table, immediately after the `task_graph_confirmed_at` row (`| `task_graph_confirmed_at` | string/null | init/reinit | ISO-8601 confirmation time |`), insert:

```
| `init_verification` | object/null | init-only | Written at `A.init.2f`. Shape: `{ mode ("cheap"|"deep"), wf_ref (string/null — null on the cheap path; the Workflow run id on deep), folded_count, flagged_count, rejected_count, ran_at }`. Additive and optional: absent on older ledgers and tolerated; populated on the next init. Does NOT affect `schema_version`. |
```

- [ ] **Step 3: Verify**

Read the Field Semantics table and confirm both rows are present and well-formed. Confirm `schema_version` row text is unchanged (still `2`).

- [ ] **Step 4: Do NOT commit yet.**

---

## Task 5: Add the post-confirmation guard to `anti-patterns.md`

**Files:**
- Modify: `.claude/skills/opi-implement/references/anti-patterns.md`

- [ ] **Step 1: Append one row to the table**

The table's last data row is currently:

```
| Never let sub-agent completion order decide result order | Non-deterministic ordering = unreproducible results. `parallelize` array defines canonical order. |
```

Immediately after it (before the closing prose paragraph "The skill refuses to act if any rule would be violated..."), insert:

```
| Never run verify-and-fold after `A.init.3` confirmation | Verify-and-fold (`A.init.2e`/`A.init.2f`) runs pre-confirmation, where the graph is not yet a reviewed contract. Running it post-confirmation would silently rewrite confirmed metadata, violating red flag #7 (graph is a reviewed contract). The folded draft is an input to the gate, never a mutation of a confirmed graph. |
```

- [ ] **Step 2: Verify**

Confirm the new row is the final table row and the closing prose paragraph still follows the table.

- [ ] **Step 3: Do NOT commit yet.**

---

## Task 6: Add `--deep-init` and the pointer to `skill.md`

**Files:**
- Modify: `.claude/skills/opi-implement/skill.md`

- [ ] **Step 1: Add `--deep-init` to the Invocation block**

Locate the Invocation code block. After the `--reinit` line:

```
opi-implement --reinit                         # re-parse spec, reconcile
```

insert:

```
opi-implement --deep-init                      # init only: multi-lens verify-and-fold (deep Workflow)
```

- [ ] **Step 2: Add the reference pointer**

Locate the pointer lines near the end of the "Six Phases Per Invocation" section:

```
**When init/reinit runs:** Read `references/initializer.md` for the full flow.
```

Immediately after it, insert:

```
**When A.init.2e/2f (verify-and-fold) runs:** Read `references/init-verify.md`
for the six lens charters, fold matrix, citation grammar, and cheap/deep
protocols.
```

- [ ] **Step 3: Verify**

Confirm the Invocation block now lists `--deep-init` and the pointer line sits with the other "When X runs" pointers. No other `skill.md` content changed.

- [ ] **Step 4: Do NOT commit yet.**

---

## Task 7: Verify and commit

**Files:** none new (verification only), then the consolidated commit.

- [ ] **Step 1: Cross-reference integrity check**

Confirm every internal pointer resolves:
- `skill.md` "When A.init.2e/2f runs" → `references/init-verify.md` exists (Task 2).
- `initializer.md` `A.init.2e` → `references/init-verify.md` exists.
- `init-verify.md` deep-path protocol → `scripts/opi-init-verify.workflow.js` exists (Task 1).
- `anti-patterns.md` new row → references `A.init.2e/2f` and red flag #7, both of which exist in `skill.md`.

- [ ] **Step 2: Confirm no ledger/spec drift**

This change does NOT touch `docs/opi-spec.md` or any registered phase design doc. Confirm:
- `git diff --name-only` lists only the 6 files in the File Structure table (plus this plan).
- No `docs/opi-spec.md` change → no phase4/phase6 ledger snapshot re-sync required.
- `.opi-impl-state.json` `spec_files_sha256` is unaffected (skill files are not in `spec_files`).

- [ ] **Step 3: Doc-guard neutrality check**

The changed files are markdown skill docs and one JS script. Confirm none is referenced by any doc-guard test:

```
grep -rn "init-verify" crates/ 2>/dev/null
grep -rn "opi-init-verify.workflow" crates/ 2>/dev/null
grep -rn "init_verification" crates/ 2>/dev/null
```

Expected: no matches (skill files are consumed by the agent at runtime, not `include_str!`'d by any Rust test). If a match appears, that test must be run before committing.

- [ ] **Step 4: Re-run the deep-path script smoke (regression)**

Re-run the Task 1 Step 2 Workflow invocation. Expected: same shape (`confirmed_folds`, `flagged_for_human`, `rejected`, `report`), no launch error. This confirms the committed script file still parses after all edits.

- [ ] **Step 5: Stage and commit (REQUIRES user authorization per CLAUDE.md git rule)**

Before committing, ask the user. Once authorized, stage ONLY the 6 files (never `git add -A`):

```
git add scripts/opi-init-verify.workflow.js \
        .claude/skills/opi-implement/references/init-verify.md \
        .claude/skills/opi-implement/references/initializer.md \
        .claude/skills/opi-implement/references/ledger-schema.md \
        .claude/skills/opi-implement/references/anti-patterns.md \
        .claude/skills/opi-implement/skill.md
git status   # confirm ONLY these 6 files are staged
git commit -m "feat(opi-implement): add wayfinder-style init verify-and-fold stage

Insert A.init.2e/2f between draft extraction and the A.init.3 human gate.
Cheap path (default) runs six lenses in one agent and folds once; deep path
(--deep-init) runs the multi-lens adversarial Workflow. Folds carry
inference_notes provenance; the review gate stays authoritative. Cuts the
~43-corrections/7-tasks rework loop observed on phase 13.

Spec: docs/superpowers/specs/2026-07-12-opi-implement-init-verify-design.md"
```

Do NOT commit the spec or this plan in the same commit unless the user asks — keep the implementation commit focused. (The spec/plan can be committed separately on request.)

---

## Self-Review (completed during authoring)

**1. Spec coverage:**
- G1 (cut rework on DoD/tier/forbidden_scope/production_call_sites + coverage) → L1/L2/L3/L4 (Task 2).
- G2 (wayfinder devices: completeness, non-goal guards, defer/split/residual with re-triggers) → L3/L4/L5 (Task 2).
- G3 (A.init.3 stays authoritative) → guardrails + surgery override (Task 2) + anti-pattern row (Task 5).
- G4 (cheap default bounded, deep opt-in) → modes section + `--deep-init` (Tasks 2, 6).
- G5 (every fold in inference_notes) → fold matrix definition (Task 2) + ledger-schema encoding (Task 4).
- Architecture §4.1 plug-in points → Task 3. §4.2 file list → all six tasks.
- §9 ledger additions → Task 4. §10 report format → Task 2. §11 guardrails → Tasks 2 + 5.
- §8 deep-path script → Task 1. §12 testing → Task 7 (guard neutrality + script smoke).
- 9 hardened folds from `wf_2e12dfa5-ce6` all reflected: L1 severity decoupled; L4 severities pinned + surgery override; L5 procedure + encoding; §7 collect-then-fold-once; L3 concrete sub-checks; citation grammar + pattern; wf_ref extraction + typo fix.

**2. Placeholder scan:** none. Every step shows exact content or an exact runnable command. The script (Task 1) and `init-verify.md` (Task 2) are reproduced in full.

**3. Type/name consistency:** `init_verification` shape (`mode`, `wf_ref`, `folded_count`, `flagged_count`, `rejected_count`, `ran_at`) is identical in spec §9, init-verify.md, ledger-schema.md (Task 4), and initializer.md `A.init.2f` (Task 3). `inference_notes` encoding `"<verb>: trigger=<clause|null>"` is identical in spec §5 L5, init-verify.md L5, and ledger-schema.md (Task 4). The script's `args` keys (`draftTasks`, `sourceDesignPath`, `phase`) match the Task 1 smoke-test args and the init-verify.md deep-path protocol. `--deep-init` spelled identically in skill.md, initializer.md, init-verify.md.

**Out-of-scope observation (do NOT fix in this change):** `ledger-schema.md` line ~160 currently says "for phases 5-14" in one validation rule, which is stale after the 5-13→14-16 registry rotation (that rotation's diff did not touch `ledger-schema.md`). This is unrelated to init-verify and is left for a separate rotation-cleanup change. Flag to the user.
