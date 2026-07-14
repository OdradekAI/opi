# Verify Engine Reference

Read at three lifecycle points: the **plan** path (`A.init.2e/2f`, shared by init
and reinit), **Phase D.2** (exec), and **Phase F.1a** (phase-exit). This is the
single stage-aware adversarial-verify engine; the old `init-verify.md`
plan-only stage is now the plan stage of this engine.

## 1. Purpose / hard rule

One stage-aware adversarial-verify engine runs at three points:

- **plan** — init and reinit merged into one auto-detecting path, deep auto-decided
  by drift magnitude;
- **exec** — Phase D, deep risk-gated;
- **phase-exit** — Phase F, deep, wrapping the existing F.1a evaluator.

**Hard rule — the engine pre-corrects / pre-reviews; it never auto-overrides a
gate it already fired.** Plan verify runs ONLY pre-`A.init.3`-confirmation, on a
draft graph that is not yet a reviewed contract. Exec verify gates Phase D
(must-fix routes to Phase C). Phase-exit verify gates `F.1b` archive. Re-running
any stage to override its own gate silently rewrites confirmed/shipped state and
is forbidden (see `anti-patterns.md`). The engine never silently confirms a
graph, passes a task, or archives a phase.

## 2. Inputs

- **plan** — the draft task array produced by `A.init.2a/2c/2d` (held in memory /
  written to `.opi-impl-state.draft.json`) + the active phase's registered source
  design doc path (from the `skill.md` registry table).
- **exec** — the task object (`definition_of_done`, `evidence`,
  `acceptance_scenarios`, `task_owned_paths`, `production_call_sites`) + the HEAD
  commit being verified + the phase design doc.
- **phase-exit** — F.1a's `criteria_trace[]` array + the phase design doc + the
  phase number.

## 3. Modes

- **plan — auto-deep (§5.4 classifier).** Deep (full multi-lens Workflow at
  `.claude/skills/opi-implement/scripts/plan.workflow.js`) for first-init-of-a-phase or substantive
  spec-section change; single-agent verify for routine drift; defaults deep when
  uncertain. No `--deep-init` flag.
- **exec — risk-gated (§7.2).** `evaluator_required = true` → full 6-lens deep
  Workflow (`.claude/skills/opi-implement/scripts/exec.workflow.js`). All other tasks → 2-lens single-agent
  pass (L-D1 + L-D5), no script.
- **phase-exit — always deep (§8.3).** `.claude/skills/opi-implement/scripts/phase-exit.workflow.js`, all 5
  lenses, every time (runs once per phase).

### 3.1 Plan auto-deep magnitude classifier (concrete procedure)

Inside the plan path's verify step, the harness classifies the sync by a
**section-level diff** (not whole-file hash):

1. **Re-parse** each `spec_files` entry into named sections (by heading) and hash
   each section. Compare to the section hashes captured at the last plan run.
2. **Substantive change** (→ deep): any of these sections changed — Goals,
   Non-Goals, Sequencing, per-ticket design sections (T1/T2/...), Success/Exit
   Criteria, Load-bearing invariant.
3. **Routine drift** (→ single-agent): only non-substantive sections changed
   (Residuals wording, typo, single-DoD-wording tweak, or only the
   `spec_files_sha256` whole-file hash moved without a substantive-section diff).
4. **First-init-of-a-phase** (→ deep): no task in the active `tasks` array has
   `phase == current_phase` AND `phase_exit[current_phase]` is absent. (Distinct
   from a `fresh` ledger, which is ledger-absent.)
5. **Default deep** when the classifier is uncertain (rigor-favoring).

**Token-budget guard:** no hard per-run token ceiling is enforced in v1; the
section-diff classifier is the cost control (deep only on substantive /
first-init, single-agent otherwise). A budget guard is a deferred follow-up if
deep-run cost proves excessive in practice. No `--quick` flag (preserves the
"fewer concepts" goal).

## 4. The shared harness

Documented once here; each stage script re-implements it (~40 lines).

**Constraint (fact):** Workflow scripts cannot import a shared module, so the
harness is a documented convention. DRY lives at the spec/convention level, not
runtime code-sharing.

### 4.1 The verify loop

`lenses fan out → deterministic disposition → adversarial verify → synthesize`.

1. Run every lens charter (in parallel under the deep Workflow, sequentially
   under a single-agent path) against the input. Each lens emits findings.
2. Deterministic disposition (plain logic, no model): split findings into the
   foldable set and the flag-for-human set using the severity/disposition matrix
   below.
3. Adversarial verify: each foldable finding goes to an independent agent that
   tries to REJECT it. Only findings that survive (the verifier failed to reject)
   proceed. Default to rejected on uncertainty or verifier error.
4. Synthesize a report (summary + the stage's outcome lists).

### 4.2 `FINDINGS_SCHEMA`

```json
{
  "lens": "<lens-key>",
  "task_id": "<string at plan/exec; null at phase-exit>",
  "criterion_id": "<string at phase-exit; null at plan/exec>",
  "field": "<draft field or subject>",
  "problem": "<what is wrong>",
  "severity": "high | medium | low",
  "suggested_fix": "<concrete fix>",
  "source_citation": "<file>#<heading>  |  <file> §<section>",
  "confidence": "high | medium | low"
}
```

The subject is **split, not overloaded**: `task_id` is populated at plan/exec,
`criterion_id` at phase-exit. Cross-stage queries need no stage-conditional
parsing.

### 4.3 Severity / disposition matrix (the foldable set)

| Severity | Confidence | Disposition |
|---|---|---|
| high | any | foldable (verify, then apply/block per stage) |
| medium | high | foldable |
| medium | medium / low | flag for human |
| low | any | flag for human |

The **stage-specific action** on a foldable finding that survives adversarial
verify differs by stage (see §5 / §6 / §7). Flag-for-human findings never mutate
state; they surface in the report.

### 4.4 Task-graph-surgery override (plan stage)

Takes precedence over the matrix at plan: a finding whose `suggested_fix`
requires **adding, removing, or restructuring tasks** — rather than editing an
existing task's field — is **always flag for human** regardless of severity. It
surfaces as a REFUSE-triggering item at `A.init.3`. A high-severity finding whose
fix IS a field edit still folds.

### 4.5 Citation grammar

`source_citation` must follow `<file>#<heading>` or `<file> §<section>` (regex
`(§|#)`). The disposition step performs the syntactic grammar check and demotes
non-conforming findings to flag-only. **Content-existence** — the cited heading
appears verbatim in the source — is verified by the lens agent at emit time on
both paths (lens agents hold the source file open; the deep-path Workflow scripts
have no filesystem access and so do only the syntactic check).

## 5. Plan stage (migrated init-verify)

The shipped init-verify becomes the plan stage. It runs inside the unified plan
path at `A.init.2e/2f` (preserved step names), pre-`A.init.3` confirmation. Mode
is auto-deep (§3.1). Outcome = `draft-field-edit` (apply `suggested_fix` to a
draft field + an `inference_notes` provenance entry).

Every lens MUST: read the source design doc at the registered path in full;
never propose implementing a Non-Goal; never invent tasks or scope beyond the
source; emit one finding per problem with a `source_citation` whose heading
appears verbatim in the source.

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
  delta vs the validation rule; an addendum-only Non-Goal must NOT
  false-positive).
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

Fold-vs-flag is governed by the task-graph-surgery override (§4.4), not severity
alone. A `substrate-only-owner` finding folds at high severity when its suggested
fix is a field edit (e.g. the task is mis-flagged and the lens can supply the
real `production_call_sites`); it is always flag-for-human when the only fix is
adding a vertical-slice task. `missing`-owner and surgery-only findings are
always flag-for-human and surface as REFUSE-triggering items at `A.init.3` via
the existing "REFUSE confirm-all while any source criterion is missing" rule.

### L5 — dependency / sequencing (+ wayfinder devices)

Verify `depends_on` matches the design doc's Sequencing section; flag missing,
spurious, and cyclic deps. Capture cross-ticket interactions as
`inference_notes`. Extract defers / splits / residuals with this procedure:

1. **Recognize** — pattern-match residual / defer sentences against `deferred to
   <X>`, `re-sharpen when <Y>`, `<Z> appears`, `deferred follow-up`. The matched
   `<Y>`/`<Z>` clause IS the re-trigger condition.
2. **Trigger-less defers** — when a defer states no trigger clause, record the
   re-trigger as `null`, set severity **medium** AND confidence **low** (so the
   matrix routes it to flag-for-human). Never invent a trigger.
3. **Encode** — `inference_notes` entry: `field` ∈ {`deferred`, `split`,
   `residual`}, `reason` = `"<verb>: trigger=<clause|null>"`.
4. **Consumer** — `A.init.3` surfaces `null`-trigger defers in the report;
   re-plan re-evaluates them.

**Severity high** for cycles or a missing hard dependency.

### L6 — substrate vs. product (red flags #11 / #12)

For each task: verify `substrate_only` is correctly set; verify no
product-facing acceptance scenario relies solely on a `substrate_only` task;
verify every runtime / startup / CLI / session / provider claim has a
`production_call_sites` entry. **Severity high** when a product scenario would
be closed by substrate-only evidence.

### Plan-stage protocols

- **Deep path** — invoke the Workflow tool with
  `scriptPath: ".claude/skills/opi-implement/scripts/plan.workflow.js"` and
  `args = { draftTasks, sourceDesignPath, phase }`. The script fans L1–L6 out in
  parallel, deterministically folds, adversarially verifies each foldable finding
  (default-reject on uncertainty; surgery fixes rejected), and synthesizes a
  report. It returns `{ confirmed_folds, flagged_for_human, rejected, report }`.
  The return object does NOT carry the run id — read it from the Workflow tool's
  result envelope and copy it into the `verify_runs` entry's `wf_ref`, then apply
  `confirmed_folds` to the draft with `inference_notes` provenance.
- **Single-agent path** — one agent runs L1–L6 sequentially against the original
  unmodified draft (every lens reads the same pre-fold snapshot, for isolation
  and determinism), applies the fold matrix once across all collected findings,
  records every fold in `inference_notes`, and emits the report.

After either path, `A.init.2f` applies the confirmed folds (field edits only),
records a `verify_runs` entry (`stage = "plan"`), writes the folded draft to
`.opi-impl-state.draft.json`, and emits the report block at `A.init.3`.

## 6. Exec stage (Phase D)

The adversarial-judgment layer on top of the D.1 mechanical gates. Outcome =
`must-fix-block`: a bounded must-fix list that BLOCKS Phase D pass and routes to
Phase C; the agent addresses findings via TDD. Each C→D cycle increments
`iteration_count` against `max_iterations` (5). The must-fix list is recorded in
`session_notes[].gate_results`, not a new field. Persistent must-fix growth hits
the failure gate (`references/failure-gate.md`). The engine never auto-edits
code.

### 6.1 Lens set (6)

| Lens | Catches |
|---|---|
| L-D1 Implementation-matches-DoD | stubs/TODOs passing a real DoD assertion (red flag #6) |
| L-D2 Tests-non-vacuous | tautological / always-pass / over-mocked tests |
| L-D3 Production-call-site-proven | runtime claims with no real production call site (red flags #11/#12) |
| L-D4 Evidence-truthfulness | `Opi-*` footers / evidence not matching reality (judgment analog of D.0a) |
| L-D5 Non-goal-leak | implementation drifting into a phase Non-Goal (red flag #15) |
| L-D6 Workspace-deps-honored | bare path deps, missing `[workspace.dependencies]` (red flag #9) |

### 6.2 Risk-gating

- `evaluator_required = true` → full 6-lens deep Workflow (fan-out + adversarial
  verify) via `.claude/skills/opi-implement/scripts/exec.workflow.js`.
- All other tasks → **2-lens single-agent pass: L-D1 + L-D5.** L-D3
  (production-call-site) is already mechanically enforced by the D.0 Product
  Acceptance addendum + the ledger `production_call_sites` validation rule, so it
  is the redundant lens for the light path; **L-D5 (non-goal-leak) has no
  mechanical exec backstop** (the Non-Goal validation rule runs at
  task-graph-confirmation, not exec), so it must be in the light path — non-goal
  drift is a tier-agnostic hard-refuse that a non-`evaluator_required` `library`
  task could otherwise pass cleanly. No fan-out, no adversarial-verify stage.

### 6.3 Light-path protocol (single agent, L-D1 + L-D5)

1. Load the task object, the HEAD commit being verified, and the phase design
   doc. Run `git show --stat <commit>` and read each changed file.
2. **L-D1** — every observable assertion in the `definition_of_done` is actually
   implemented (no stubs/TODOs/placeholders passing a real assertion).
3. **L-D5** — the implementation does not drift into a phase Non-Goal
   (token-trigger: `npm`, `marketplace`, `OAuth`, `telemetry`, `sandboxing`,
   `web-UI parity`, `pi session compatibility`, `workflow tools`, `MCP core`,
   `plan mode core`, `sub-agent core`).
4. Findings become the must-fix list that blocks D and routes to C. Each finding
   cites the source section (`§` or `#`) and verifies the heading appears
   verbatim.

### 6.4 Deep-path protocol

Invoke the Workflow tool with `scriptPath: ".claude/skills/opi-implement/scripts/exec.workflow.js"` and
`args = { task, sourceDesignPath, commit }`. The script fans the 6 lenses out in
parallel, adversarially verifies each foldable finding, and synthesizes the
report. It returns `{ must_fix, flagged_for_human, rejected, report }`. Read the
run id from the Workflow result envelope into the `verify_runs` entry's `wf_ref`.

## 7. Phase-exit stage (Phase F)

Runs once per phase (infrequent); all 5 lenses run as the full deep Workflow
every time. Outcome = `not-met-block`.

### 7.1 Lens set (5) — adversarial audit of F.1a's criteria trace

| Lens | audits |
|---|---|
| L-F1 Every-criterion-traced-to-code | each success/exit criterion maps to real code |
| L-F2 Every-criterion-traced-to-test | each criterion has an exercising test |
| L-F3 Non-goals-respected | no Non-Goal implemented to satisfy a criterion |
| L-F4 Residuals-exactly-cited | every `deferred-by-updated-design` has an exact current-spec citation |
| L-F5 Substrate-vs-product-honest | no product criterion closed by substrate-only tasks across the phase |

### 7.2 Wraps F.1a (not replace)

F.1a produces the criteria trace (working machinery, phases 1–13); the 5 lenses
take that trace as input and adversarially audit it. F.1a's output → lenses'
input. Invoke the Workflow tool with
`scriptPath: ".claude/skills/opi-implement/scripts/phase-exit.workflow.js"` and
`args = { criteriaTrace, sourceDesignPath, phase }`. The script returns
`{ not_met, flagged_for_human, rejected, report }`.

### 7.3 Finding → trace write-back

The engine emits findings, but `F.1b` REFUSE fires on `criteria_trace[].status`.
The mapping: a finding that survives adversarial verify — i.e. in the foldable
set (severity **high**, or **medium** with **high** confidence) — against
`criterion_id` C **upserts `criteria_trace[C].status = not-met`**, with the
finding's `source_citation` as the evidence pointer. `F.1b`'s existing REFUSE
rule then fires on that row. Medium/low findings that are not in the foldable set
do NOT mutate the trace; they are surfaced in the report for the human.

`deferred-by-updated-design` survives only with an exact citation (L-F4 enforces
adversarially); a lens finding of "deferred-without-citation" upserts
`criteria_trace[C].status = not-met` (F.1b refuses uncited deferrals).

## 8. Report format

- **plan** — mode (`single-agent` / `deep` + `wf_ref` if deep); folded: N
  corrections applied, grouped by lens, with source citations; flagged for human:
  M findings not auto-applied (a bounded list, not an open re-edit loop);
  rejected (deep only): K foldable findings the adversarial pass rejected, with
  reasons; null-trigger defers (L5): listed for human trigger specification.
- **exec** — must-fix (block Phase D pass; route to Phase C); flagged for human;
  rejected.
- **phase-exit** — not-met (the calling agent upserts
  `criteria_trace[C].status = not-met` for each; F.1b then REFUSEs archive);
  flagged for human (do not mutate trace); rejected.

## 9. Guardrails

- **Human gate authoritative at every stage.** Plan verify pre-corrects the draft
  (gate still confirms); exec must-fix blocks D but the agent/human fixes;
  phase-exit not-met blocks archive but the human decides the remedy.
- **Pre-confirmation only at plan.** Plan verify runs on an unconfirmed graph.
  Exec and phase-exit run on implemented/shipped work — their findings block
  progression, they don't auto-rewrite.
- **No scope invention / no auto-edit.** Lenses never invent tasks (plan) or
  auto-edit code (exec). Findings without a resolving citation are flagged.
- **Deep is auto/default, never a flag.** No `--quick`, no `--deep-init`.
- **Adversarial verify before any deep disposition.** No foldable finding reaches
  the draft / must-fix / not-met list unless an independent agent failed to
  reject it.
