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
citation grammar below and whose heading appears verbatim in the source.

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

**Fold-vs-flag** is governed by the task-graph-surgery override (Fold matrix),
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
grammar check and demotes non-conforming findings to flag-only.
Content-existence (the cited heading appears verbatim in the source) is verified
by the lens agent at emit time on both paths — lens agents hold the source file
open; the deep-path Workflow script has no filesystem access and so does only
the syntactic check.

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
