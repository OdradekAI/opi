# opi-implement Init Verify-and-Fold — Design

**Status:** Draft for review
**Date:** 2026-07-12
**Author:** opi-implement skill hardening
**Supersedes / extends:** `docs/superpowers/specs/2026-05-20-opi-implement-skill-design.md`
  (extends the A.init flow; does not change Phase B–F execution semantics)

## 1. Overview

`opi-implement`'s spec-init flow (`A.init.2` parse → `A.init.2a/2c/2d` extract →
`A.init.3` human task-graph review gate) is a **single-pass inference** from a
phase design doc into a task ledger. The archived `docs/snapshots/phase13/`
ledger shows the parser produced **43 inference corrections across 7 tasks**,
concentrated in four fields:

| Inferred field | Corrections | Share |
|---|---|---|
| `definition_of_done` | 13 | 30% |
| `tier` / crate-boundary | 9 | 21% |
| `forbidden_scope` (non-goal extraction) | 7 | 16% |
| `production_call_sites` | 5 | 12% |

These four fields account for **~79% of all manual rework**, performed by the
user round-tripping `edit-task` / `apply-rule` / `export-draft` / `import-draft`
at the `A.init.3` gate. The Phase 14–16 design docs have **no Success Criteria
or Testing Strategy section**, so the parser synthesizes DoD / tier / scope from
scattered prose and guesses wrong.

The **wayfinder** planning methodology (see
`docs/superpowers/plans/2026-07-10-phase-roadmap-redesign-map.md`) avoids this
by running `draft → adversarial multi-lens fan-out → fold corrections → review`,
recording `wf_*` verification refs, premise corrections, and devices a single
pass never produces: defers **with re-trigger conditions**, splits **with
verified crux facts**, residuals carry-over, and non-goal guards.

### Proposal

Insert a wayfinder-style **verify-and-fold stage between `A.init.2` (parse) and
`A.init.3` (human gate)**. The human review gate stays (the graph is a reviewed
contract — red flag #7), but the draft arriving at it is pre-corrected, so the
user confirms once instead of iterating N times.

The stage is **tiered**:
- **Cheap path (default)** — a single agent runs six lenses sequentially, folds
  high-confidence corrections, and writes a verification report. Bounded cost
  on every init.
- **Deep path (`--deep-init`)** — a Workflow fans the six lenses out in
  parallel, adversarially verifies each fold, and synthesizes a final report
  with a `wf_*` ref. Opt-in, for wayfinder-grade thoroughness.

Scope of this change: **A.init only**. Reinit reconciliation (same single-pass
re-parse, plus preserve-history concerns) is a follow-up (§13).

## 2. Goals

- G1. Cut human rework at `A.init.3` by pre-folding the four high-error inferred
  fields (`definition_of_done`, `tier`, `forbidden_scope`,
  `production_call_sites`) plus acceptance coverage gaps, with provenance.
- G2. Port wayfinder's signature devices into init: completeness audit (every
  Goal/SC has an owning task), non-goal guards, and defer / split / residual
  extraction **with re-trigger conditions**.
- G3. Preserve the `A.init.3` human gate as the final authority; verify-and-fold
  never silently confirms the graph.
- G4. Keep the default cheap path bounded and predictable; make wayfinder-grade
  depth opt-in via `--deep-init`.
- G5. Record every fold as an `inference_notes` entry so the graph remains
  auditable and the existing "graph is a reviewed contract" invariant holds.

## 3. Non-Goals

- N1. Changing Phase B–F execution semantics, verification tiers, or commit
  evidence format.
- N2. Replacing the `A.init.3` human review gate. Verify-and-fold pre-corrects;
  it does not confirm.
- N3. Touching reinit reconciliation in this change (follow-up, §13).
- N4. Auto-implementing Non-Goals, inventing tasks beyond the source design, or
  expanding phase scope.
- N5. Bumping `schema_version` (the additions are additive, optional, and only
  written at init — §9).
- N6. Making the deep path the default. Token / time cost of a Workflow fan-out
  is opt-in by design.

## 4. Architecture

### 4.1 Plug-in points

Two new steps between the existing `A.init.2d` and `A.init.3`, defined in
`references/initializer.md` and detailed in a new reference
`references/init-verify.md`:

| Step | Name | Action |
|---|---|---|
| `A.init.2e` | **Verify** | Run the six-lens audit over the draft graph against the phase design doc. Cheap path = one agent; deep path = `scripts/opi-init-verify.workflow.js`. |
| `A.init.2f` | **Fold + report** | Apply high / medium-high-confidence findings as draft edits, each recorded in `inference_notes`. Emit a verification report. Hand the pre-corrected draft to `A.init.3`. |

`skill.md` gains one pointer line under "When init/reinit runs" referencing the
new `references/init-verify.md`. The six-phase-per-invocation section (§"Six
Phases Per Invocation") is unchanged — `A.init.2e/2f` are sub-steps of Phase A,
like `A.init.2a/2c/2d`.

### 4.2 New and modified files

| File | Change |
|---|---|
| `.claude/skills/opi-implement/references/init-verify.md` | **New.** Lens charters, finding schema, severity/fold matrix, cheap-path protocol, deep-path protocol, report format, guardrails. |
| `.claude/skills/opi-implement/references/initializer.md` | Insert `A.init.2e` / `A.init.2f` between `A.init.2d` and `A.init.3`; one-line pointer to `init-verify.md`. |
| `.claude/skills/opi-implement/skill.md` | One pointer line; add `--deep-init` to the Invocation block; note in Red Flags / anti-patterns that verify-and-fold must not run post-confirmation. |
| `.claude/skills/opi-implement/references/anti-patterns.md` | One row: "Never run verify-and-fold after `A.init.3` confirmation" (prevents silent contract rewrite). |
| `.claude/skills/opi-implement/references/ledger-schema.md` | Document the additive `init_verification` top-level field (§9). |
| `scripts/opi-init-verify.workflow.js` | **New.** The deep-path Workflow script (§8). Invoked via the Workflow tool's `scriptPath` by the opi-implement agent when `--deep-init` is passed. |

## 5. The Six Lenses

Each lens is an independent audit of the draft graph against the phase design
doc. Every lens must: read the source design doc at the registered path, never
propose implementing a Non-Goal, never invent scope beyond the source, and emit
one finding per problem using the schema in §6.

### L1 — DoD precision (kills the 30%)
For each task `definition_of_done`: detect vague verbs (`works`, `supports`,
`loads`, `integrates`, `bridges`, `productizes`, `handles`) and missing
observable assertions. Suggested fix must name the command / API entry point,
persisted artifact, production call site, runtime effect, diagnostics, and
negative / error behavior where relevant. Promotes the existing "DoD precision
rule" (skill.md) from a human-enforced rule to a mechanically-enforced check.
**Severity high** when a vague verb is present — decoupled from
`substrate_only`. Substrate-only tasks are fold-eligible too: the suggested fix
names the observable substrate assertion (the parsed shape, the persisted
struct, the error returned on malformed input). The only carve-out in skill.md's
DoD precision rule — a vague verb may stand when the review gate *explicitly*
accepts it as substrate-only — is an `A.init.3` human decision, not an automatic
severity demotion at this stage. (The prior "not `substrate_only`" qualifier
contradicted the unconditional DoD lint in `A.init.2c` / anti-pattern #14 and
systematically under-folded the verb-heavy helper / parser / bridge tasks that
most need expansion.)

### L2 — tier / crate-boundary (kills the 21%)
For each task: verify `crate` + `tier` match the load-bearing invariant
(`opi-ai` owns provider/auth wire-up; `opi-agent` owns runtime/sessions/hooks
but must not construct providers; `opi-coding-agent` owns CLI/TUI/RPC wiring).
Verify `task_owned_paths` do not leak into a crate the task does not own.
Cross-reference the design doc's "Implementation Priority and Crate Boundaries"
section. **Severity high** when the invariant is violated.

### L3 — forbidden-scope / non-goal (kills the 16%)
Two concrete sub-checks, deliberately non-overlapping with `A.init.2d` (which
already converts Non-Goals into `forbidden_scope` notes) and with the
`ledger-schema.md` validation rule (which accepts a `forbidden_scope` note
**or** a phase verification addendum):

- **Coverage sub-check** — emit a finding only when a Non-Goal lacks **both** a
  `forbidden_scope` inference note **and** a phase verification addendum (i.e.
  report the *delta* vs the existing validation rule; do not re-run `A.init.2d`
  or stricter-than-run it — an addendum-only Non-Goal must not false-positive).
- **Risk sub-check** — fire on a concrete token trigger: a task's
  `acceptance_scenarios` or `definition_of_done` contains a token from the
  Non-Goal vocabulary (`npm`, `marketplace`, `OAuth`, `telemetry`, `sandboxing`,
  `web-UI parity`, `pi session compatibility`, `workflow tools`, `MCP core`,
  `plan mode core`, `sub-agent core`). Semantic "resembles a non-goal" hunches
  without a token match are `confidence = low` → flag-only (never folded).

**Severity high** on a token match (a non-goal is at risk); the suggested fix is
the `forbidden_scope` note text plus citation.

### L4 — coverage / completeness (kills the 12% + gaps)
For each Goal / Success Criterion / named user workflow in the design doc:
confirm there is at least one owning task with an `acceptance_scenarios` entry
and a `production_call_sites` entry. Flag three states: `missing` (no owner), `substrate-only-owner` (a runtime
criterion covered only by a `substrate_only` task — red flag #11), and
`deferred-without-citation` (a criterion deferred without an exact source
citation). Also audit composite-row detection (`A.init.2a`): did any composite
row escape splitting?

**Severity** (all three states pinned, since the fold matrix is severity-driven
and determinism requires it): `missing` owner of a runtime criterion = **high**;
`substrate-only-owner` of a runtime criterion = **high** regardless of
confidence (red flag #11); `deferred-without-citation` = **medium**.

**Fold-vs-flag for these states** is governed by the §6 *task-graph-surgery
override*, not the severity matrix alone. A `substrate-only-owner` finding folds
at high severity when its `suggested_fix` is a field edit on the existing task
(e.g. the task is mis-flagged and the lens can supply the real
`production_call_sites` that promote it to product); it is **always flag for
human** when the only fix is adding a new vertical-slice task. `missing`-owner
and surgery-only findings are likewise always flag for human (§11 forbids
inventing tasks) and surface as REFUSE-triggering items at `A.init.3` via the
existing "REFUSE confirm-all while any source criterion is missing" rule.

### L5 — dependency / sequencing (+ wayfinder devices)
Verify `depends_on` matches the design doc's Sequencing section; flag missing
deps, spurious deps, and cycles. Capture cross-ticket interactions as
`inference_notes`. **Extract** explicit defers / splits / residuals from the design doc using a
concrete procedure (not aspirational):

1. **Recognize** — pattern-match residual / defer sentences against the templates
   `deferred to <X>`, `re-sharpen when <Y>`, `<Z> appears`, `deferred follow-up`.
   The matched `<Y>` / `<Z>` clause IS the re-trigger condition.
2. **Trigger-less defers** — when a defer states no trigger clause (e.g.
   "deferred follow-up, not Phase 14"), record the re-trigger as `null`, set
   severity **medium** AND confidence **low**, so the §6 fold matrix routes it to
   *flag for human* (forcing the human to specify the trigger). Never invent a
   trigger — that violates §11 (no scope invention) and recreates the
   silent-drop failure this device exists to prevent.
3. **Encode** — record each as an `inference_notes` entry: `field` ∈ {`deferred`,
   `split`, `residual`}, `reason` = `"<verb>: trigger=<clause|null>"` (the
   re-trigger is packed into the existing `reason` key — no new task field, per
   §9; the convention is documented in `ledger-schema.md`).
4. **Consumer** — `A.init.3` surfaces defers with `null` triggers in the report;
   reinit (R1 follow-up) re-evaluates them. Without this, the device is
   write-only.

**Severity high** for cycles or a missing hard dependency.

### L6 — substrate vs. product (red flags #11 / #12)
For each task: verify `substrate_only` is correctly set. Verify no
product-facing acceptance scenario relies solely on a `substrate_only` task.
Verify every runtime / startup / CLI / session / provider claim has a
`production_call_sites` entry. **Severity high** when a product scenario would
be closed by substrate-only evidence.

## 6. Finding schema and severity

Every finding is a structured object:

```json
{
  "lens": "dod-precision | tier-boundary | forbidden-scope | coverage | dependency-sequencing | substrate-product",
  "task_id": "14.1",
  "field": "definition_of_done",
  "problem": "vague verb 'supports' without an observable assertion",
  "severity": "high | medium | low",
  "suggested_fix": "expand to: 'opi auth login <provider> writes an AuthDescriptor to ~/.config/opi/credentials.toml; exit 0 on success, exit 3 on keyring failure'",
  "source_citation": "phase14-design.md §T1 > backend strategy; §Goals bullet 2",
  "confidence": "high | medium | low"
}
```

### Fold matrix (applies identically to cheap and deep paths)

| Severity | Confidence | Action |
|---|---|---|
| high | any | **fold** |
| medium | high | **fold** |
| medium | medium / low | flag for human |
| low | any | flag for human |

"Fold" = apply `suggested_fix` to the named draft field and append an
`inference_notes` entry `{ "field": <field>, "reason": "init-verify <lens>: <problem>", "source": <source_citation> }`.
"Flag for human" = leave the field unchanged; surface in the report.

**Task-graph-surgery override** (takes precedence over the severity matrix): a
finding whose `suggested_fix` requires adding, removing, or restructuring tasks
— rather than editing an existing task's field — is **always flag for human**
regardless of severity, because §11 / N4 forbid inventing tasks. This governs
the L4 `missing`-owner, surgery-only `substrate-only-owner`, and
`deferred-without-citation` cases; they surface as REFUSE-triggering items at
`A.init.3` via the existing "REFUSE confirm-all while any source criterion is
missing" rule. A high-severity finding whose fix IS a field edit still folds.

**Citation grammar** (applies to both paths): `source_citation` must follow
`<file>#<heading>` or `<file> §<section>`. The fold step performs the syntactic
grammar check and demotes non-conforming findings to flag-only.
Content-existence (the cited heading actually appears in the source file) is
verified by the lens agent at emit time on both paths — lens agents hold the
source file open; the deep-path Workflow script has no filesystem access and so
does only the syntactic check. This makes §11's "no fold without an exact
citation" guardrail mechanical, matching anti-pattern #16.

## 7. Cheap path (default) protocol

`A.init.2e` (cheap): one agent, sequential.

1. Load the draft graph produced by `A.init.2a/2c/2d` and the registered phase
   design doc path.
2. For each lens L1–L6 in order, run the lens charter against the **original
   unmodified draft** (every lens reads the same pre-fold snapshot, for isolation
   and determinism) and collect findings. Each lens verifies its own
   `source_citation`s resolve against the source file before emitting.
3. Apply the §6 fold matrix **once** across all collected findings (plain logic,
   not a model call): fold high + medium-high-confidence findings whose citation
   parses, flag the rest. Per-lens folding is intentionally NOT used — the §6
   header ("applies identically to cheap and deep paths") and the §8 deep-path
   Fold phase both use this collect-then-fold-once model.
4. Record every fold in `inference_notes` with provenance.
5. Emit the verification report (§10) and write the folded draft to
   `.opi-impl-state.draft.json`.

Cost: bounded — six lens passes in one agent. Runs on every init. No fan-out.

## 8. Deep path (`--deep-init`) protocol

`A.init.2e` (deep): the Workflow at `scripts/opi-init-verify.workflow.js`.

The opi-implement agent invokes the Workflow tool with `scriptPath` pointing at
the script and `args = { draftTasks, sourceDesignPath, phase }`, where
`draftTasks` is the array of draft task objects (post-`A.init.2d`,
pre-`A.init.3`). The script:

1. **Lens audit** — fans L1–L6 out in `parallel`; each lens agent reads the
   source design doc at `sourceDesignPath` and the draft (passed in the prompt),
   and returns findings via `FINDINGS_SCHEMA`.
2. **Fold** — deterministic JS (no agent): apply the §6 fold matrix; collect
   `foldable` and `flagged`.
3. **Verify** — adversarial: for each foldable finding, an independent agent
   tries to **reject** it (contradicts source, implements a non-goal, invents
   scope, conflicts with another task; default rejected if uncertain). Errored
   verify agents count as rejected so the finding surfaces in the report rather
   than vanishing.
4. **Synthesize** — one agent emits the final report (`REPORT_SCHEMA`) including
   folded, flagged-for-human, and rejected-with-reason lists.
5. Returns `{ confirmed_folds, flagged_for_human, rejected, report }` — note
   the return object does **not** carry the run id. The calling opi-implement
   agent reads the run id from the Workflow tool's result envelope (the `wf_…`
   value, conventionally exposed as `runId`), copies it verbatim into
   `init_verification.wf_ref`, applies `confirmed_folds` to
   `.opi-impl-state.draft.json` (with `inference_notes` provenance), and renders
   at `A.init.3`.

### Reference script

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
    'emit one finding per problem; cite the source section for every finding.\n' +
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
    'or conflicts with another task. Default to accepted=false if uncertain.',
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

Notes on the script:
- No `Date.now()` / `Math.random()`; the run id is produced by the Workflow tool
  itself and read from its result envelope by the calling agent (not from the
  script's return object, which omits it).
- The Fold phase is deterministic JS with **no filesystem access**; it performs
  only the syntactic citation-grammar check. Citation content-existence is the
  lens agents' responsibility (they hold the source file open).
- `parallel` is a barrier by design here: fold and verify need **all** lens
  findings together (cross-lens dedup, whole-draft adversarial check). A
  `pipeline` would be wrong — there is no per-lens downstream stage.
- Errored verify agents resolve to a rejected verdict so no finding silently
  disappears (matters for the §11 guardrail on silent folds).

## 9. Ledger additions (additive, no schema bump)

A new **optional** top-level field on `.opi-impl-state.json`, written only at
init:

```json
"init_verification": {
  "mode": "cheap | deep",
  "wf_ref": "<run id from the Workflow tool result envelope, or null on the cheap path>",
  "folded_count": 9,
  "flagged_count": 4,
  "rejected_count": 1,
  "ran_at": "<ISO timestamp>"
}
```

This is additive and optional. It does **not** require `schema_version` to move
off 2: the schema-version migration logic in `initializer.md` keys only on
`schema_version`; a missing `init_verification` on older ledgers is tolerated
(it is populated on the next init). Per-task folds live in the existing
`inference_notes` array — no new task field.

## 10. `A.init.3` output changes

The gate itself is unchanged (same options: `confirm-all`, `edit-task`,
`apply-rule`, `export-draft`, `import-draft`, `abort`). What changes is the
draft it renders and an added report block:

1. **Verified draft table** — same columns as today, but fields already carry
   the folded corrections with their `inference_notes` provenance visible.
2. **Verification report block** (new) — printed above the table:
   - mode (`cheap` / `deep` + `wf_ref` if deep);
   - folded: N corrections applied, grouped by lens, with source citations;
   - flagged for human: M findings not auto-applied (the user's remaining
     manual work, now a bounded list instead of an open-ended re-edit loop);
   - rejected (deep only): K foldable findings the adversarial pass rejected,
     with reasons.

`REFUSE confirm-all while any source criterion/workflow is missing, or while any
runtime criterion is covered only by a substrate_only task` — the existing
`A.init.3` refusal rule still holds; L4 surfaces these as flagged-for-human so
they are visible before confirmation rather than discovered mid-execution.

## 11. Guardrails

- **Pre-confirmation only.** `A.init.2e/2f` run before `A.init.3` confirmation.
  The graph is not yet a reviewed contract, so folding is safe. After
  confirmation, reinit rules apply; verify-and-fold MUST NOT run again on a
  confirmed graph (new anti-pattern row, §4.2).
- **No silent rewrites.** Every fold is recorded in `inference_notes` with
  `field` / `reason` / `source`. The report lists folded, flagged, and rejected
  separately.
- **No scope invention.** Lenses never invent tasks, never implement a Non-Goal,
  never broaden phase scope. Findings without an exact `source_citation` are
  low-confidence and flagged, not folded.
- **Deep is opt-in.** `--deep-init` triggers the Workflow; the cheap path is the
  default. Token / time cost of fan-out is never incurred without the flag.
- **Adversarial verify before any deep fold.** In the deep path, no foldable
  finding reaches the draft unless an independent agent failed to reject it.
- **`A.init.3` remains authoritative.** Verify-and-fold reduces rework; it does
  not replace review. The user can still `edit-task` / `abort` / re-export the
  draft.

## 12. Testing / acceptance strategy

This is a harness skill change (markdown + one JS Workflow script), so
"tests" are a mix of doc guards and an empirical acceptance check at the next
real init:

- **Doc-guard neutrality.** `docs/superpowers/specs/*.md` design docs are not
  pinned by any guard suite (per the README doc-guard audit). Adding this spec
  trips no guard. (The skill-doc edits to `skill.md` / `initializer.md` /
  `anti-patterns.md` are also guard-neutral — no test `include_str`s them.)
- **Workflow script static check.** `scripts/opi-init-verify.workflow.js` must
  pass the Workflow tool's own parse (pure-literal `meta`, no
  `Date.now`/`Math.random`, valid `parallel`/schema usage). Confirmed by running
  it once on a synthetic draft in implementation.
- **Empirical acceptance (the real proof).** The next fresh `opi-implement` init
  (or `--deep-init`) of an active phase (14/15/16) reaches `A.init.3` with the
  four high-error fields pre-folded. Success criterion: the user confirms or
  makes a **bounded** number of edits (the flagged-for-human list), versus the
  prior ~43 inference corrections / 7 tasks observed on phase 13. The
  `init_verification.folded_count` / `flagged_count` fields make this
  measurable.
- **Self-demonstration.** This spec was hardened by a 5-lens adversarial
  Workflow (`wf_2e12dfa5-ce6`; 28 findings → 13 foldable → 9 confirmed and
  folded here, 4 rejected by adversarial verify) before review, applying the
  same verify-and-fold discipline to the design itself.

## 13. Residuals / follow-ups

- **R1 (deferred — this change's follow-up): reinit reconciliation.** `--reinit`
  re-parses with the same single-pass logic and would benefit equally, but it
  carries preserve-history constraints (`status`, `verified_at_commit`,
  `session_notes`, `blocker`). Verify-and-fold on reinit must diff against the
  existing confirmed graph and route field changes through the existing
  row-level-diff review path. Scoped out of this design.
- **R2 (deferred): lens tuning.** The six-lens set and the fold matrix are
  calibrated from the phase-13 evidence. After two or three real inits, the
  `init_verification` counters should be re-read to see which lenses over- or
  under-fold and the matrix adjusted.
- **R3 (deferred): cross-lens dedup.** The deep path collects findings per lens
  independently; two lenses can flag the same task/field. Today the report lists
  both and the human dedups. A deterministic dedup pass between Fold and Verify
  is a small future refinement.
- **R4 (deferred): second fold-aware pass.** The cheap path folds once after all
  lenses run against the same pre-fold snapshot (§7), so lens order is
  immaterial. A possible future refinement is a second, fold-aware pass so later
  lenses can react to earlier folds (e.g. L1 DoD expansion changes what L4
  coverage sees) — not implemented in this change.

## 14. Implementation surface (for the writing-plans step)

Touched files, in likely commit order (the plan will finalize):

1. `.claude/skills/opi-implement/references/init-verify.md` (new — the bulk of
   the content: lens charters, schema, fold matrix, protocols, report, guards).
2. `scripts/opi-init-verify.workflow.js` (new — the §8 script).
3. `.claude/skills/opi-implement/references/initializer.md` (insert
   `A.init.2e` / `A.init.2f`, pointer).
4. `.claude/skills/opi-implement/references/ledger-schema.md` (document
   `init_verification`).
5. `.claude/skills/opi-implement/references/anti-patterns.md` (one row).
6. `.claude/skills/opi-implement/skill.md` (pointer line, `--deep-init` in
   Invocation).

No production Rust code changes. No `schema_version` bump. No new crate deps.
