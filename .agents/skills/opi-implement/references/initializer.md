# Initializer & Reinit Reference

## Init Mode (A.init)

Triggered when `.opi-impl-state.json` is absent (fresh) or when the unified plan
path detects spec drift. The `plan` verb forces the plan path; a bare or
run-specific invocation enters it only on drift.

### A.init.1 Pre-flight
- Record current branch, baseline dirty files, and `opi-spec.md` presence.
  Refuse only when dirty files would be overwritten by init/reinit outputs;
  do not require unrelated user changes to be cleaned.

### A.init.2 Select the Registered Phase Source

`docs/opi-spec.md` is the durable parent specification. It supplies stable
clauses, invariants, and strategic goals, but it is not parsed as a task
roadmap. A fresh plan requires one human-reviewed Phase delivery source that:

- cites stable clauses and one strategic goal from `docs/opi-spec.md`;
- defines the Phase problem, scope, non-goals, success/exit criteria, and
  delivery decomposition;
- is explicitly registered in `SKILL.md` before admission.

If no registered Phase delivery source exists, return
`DESIGN_DECISION_REQUIRED`; do not infer a roadmap from the parent spec or scan
arbitrary files under `docs/superpowers/specs/`.

For each task described by the registered source, extract id or source order,
title, owning crate/surface, definition of done, phase number, dependencies,
and verification evidence. Infer tier, commit type, evaluator requirement, and
any missing execution metadata. Attach `inference_notes` with an exact source
heading for every non-verbatim field. An inferred or review-expanded DoD stays
non-executable until the task-graph gate confirms it.

When one source item bundles independently demonstrable deliverables, split it
into separate `<phase>.<N>` tasks and retain the source item's stable identifier
in `parent_spec_row` when one exists. Do not create a placeholder parent task.
The graph review shows the split as a unit so the user can accept or revise it.

Documentation/alignment tasks whose source explicitly forbids runtime behavior
may use tier `documentation`. A task that owns Rust source, Cargo manifests,
runtime scripts, fixtures, snapshots, or generated artifacts uses the relevant
non-documentation tier.

### A.init.2a Registered Phase Delivery Sources

Phase delivery sources come only from the reviewed registry in `SKILL.md`.

No supplemental Phase source is currently registered. Completed Phase 17 and
Phase 18 delivery specifications are frozen under `docs/snapshots/phase17/`
and `docs/snapshots/phase18/`; they are historical evidence and MUST NOT be
admitted into the live `spec_files` set.

For each active phase:

- Include `docs/opi-spec.md` and the phase's registered source in `spec_files`.
- Hash both files in `spec_files_sha256` using the CRLF-normalized SHA-256
  (replace `\r\n` with `\n` before hashing; see `SKILL.md`).
- Derive task IDs as `<phase>.<N>` in source order unless the reviewed source
  already names a stricter sequence.
- Convert success criteria into `acceptance_scenarios` before graph review.
- Convert Non-Goals into `forbidden_scope` review notes and phase-specific
  verification addenda. A task cannot satisfy a criterion by implementing a
  non-goal.
- Preserve explicit handoff sections as dependency hints for the next phase,
  not as executable work in the current phase.
- If `docs/opi-spec.md` and the registered phase design conflict on phase
  scope, stop for graph review instead of choosing silently.

### A.init.2b Design Acceptance Extraction

For every active phase whose source files include goals, success criteria, exit
criteria, or named user workflows, extract them before task-graph review.

For each criterion/workflow:

- Create or assign at least one `acceptance_scenarios` entry.
- Record `source` as an exact file path plus section/heading.
- Write the scenario as an observable user or integration path, not a module
  description.
- Assign it to the task that will actually close the path. If no task closes it,
  add a vertical-slice task or stop for human graph review.
- Add `production_call_sites` for runtime/startup/CLI/session/adapter/provider
  claims. Tests-only helpers do not count.
- Mark helper/parser/protocol/bridge tasks `substrate_only = true` unless their
  verification proves a production call path.

DoD lint: if a DoD contains vague verbs such as `works`, `supports`, `loads`,
`integrates`, `bridges`, `productizes`, or `handles`, expand it into concrete
assertions before the task is executable. The expansion must name the command or
API entry point, persisted artifact, production call site, runtime effect,
diagnostics, and negative/error behavior where relevant.

Vertical-slice rule: every product-facing phase must include at least one task
whose acceptance scenario starts at a real user/API entry point and ends at the
runtime effect claimed by the design. Component tasks may pass as substrate, but
they cannot by themselves satisfy this rule.

### P.0 Source Admission

Run after extraction has identified the registered source files and before a
candidate graph can replace the canonical ledger. Verify that:

- every source is reviewed and registered;
- problem, solution, out-of-scope, success, and exit criteria are explicit;
- evidence provenance is identified as inward pi alignment, outward research,
  or both;
- Rust-native divergence has a recorded rationale where relevant;
- a new capability explains its core, extension-seam, or plugin/package
  placement;
- changed domain terms agree with `docs/CONTEXT.md`;
- public acceptance and test seams are explicit enough to plan.

Source admission does not edit the source. Return one of these stop verdicts
when evidence or product meaning is missing:

- `RESEARCH_REQUIRED` — route to `opi-research` or `opi-realign`.
- `DESIGN_DECISION_REQUIRED` — apply the `SKILL.md` Source-return rule and
  recommend the exact explicit user invocation of Matt `wayfinder` or
  `grill-with-docs`.

Do not write `.opi-impl-state.json` on either verdict.

### P.1 Draft Graph

Write the extracted candidate only to `.opi-impl-state.draft.json`. Every task
must be an independently demonstrable vertical slice unless it is an explicitly
justified expand-contract migration step. Existing fields carry the admission
evidence without a schema bump:

- `acceptance_scenarios[].scenario` names what can be demonstrated;
- `acceptance_scenarios[].source` cites the reviewed criterion;
- `production_call_sites` names the real production path;
- `verification.behavioral_tests` exercises the agreed public seam;
- `inference_notes` records an inferred seam or placement rationale.

The draft is the only mutable plan artifact before confirmation. The canonical
ledger remains unchanged.

#### Minimum-change trace

For every executable draft task, construct the six-answer trace from existing
source and repository evidence already read during admission; do not start a
second research workflow solely to fill this trace. Product/vertical-slice
tasks cite `acceptance_scenarios[].id` and `.source`. A `substrate_only` task
may own no scenario only when a later scenario-owning task's
transitive `depends_on` closure contains it; otherwise
the substrate is orphan work and fails admission.

Add these four standardized notes using the existing
`{ field, reason, source }` shape:

- `field = "reuse_search"`: `reason` is
  `searched=<symbols/paths/packages/protocols>; reused=<items|none>;
  gap=<smallest missing capability>`;
- `field = "placement"`: `reason` is
  `target=<core|reference-product|extension|plugin|package|independent-companion|assurance>; existing_home=<id|none>;
  cannot_fit_fully=<reason|not-applicable>`;
- `field = "surface_necessity"`: `reason` is
  `public_api=<none|necessity>; config=<none|necessity>;
  state=<none|necessity>; dependency_edge=<none|necessity>`;
- `field = "simplification_ceiling"`: `reason` is
  `ceiling=<known limit>;
  revisit_when=<observable condition>;
  simplification_trigger=<none|unused|duplicate|superseded|delete|merge|replace|dependency-substitution>`.

When the task claims an existing surface is unused, duplicate, or superseded,
or proposes deletion, merging, replacement, or dependency substitution,
append conditional simplification evidence to those same notes:

- `reuse_search.reason` also includes
  `production_consumers=<items|none>; nonproduction_consumers=<tests/docs/examples|none>`;
- `simplification_ceiling.reason` also includes
  `net_deletion=<removed surfaces minus new glue>; residual_glue=<items|none>`.

Use `simplification_trigger=none` for an ordinary task; the four consumer and
deletion subclauses are required only for another trigger value. A `none`
answer must cite verifiable repository evidence.
Check dynamic loading, configuration lookup, wire and persistent formats, and
public API consumers when those mechanisms could hide a live dependency.

When the registered source or repository evidence shows intrinsic Agent state,
two production task participants sharing one rule, an expand-contract
transition, or a recurrent decision finding, read
`../../_shared/references/shared-decision-and-test-stewardship.md` in full.
Add one `field = "shared_decision"` inference note per participating task using
its exact eleven-clause plan-note contract. Do not add the note for an ordinary
one-consumer helper. Every participating task sets `evaluator_required = true`.

Every `source` cites the registered source heading, reviewed decision, or
repository evidence used for that answer. `none` and `not-applicable` are
valid only with a reason. `revisit_when` must name an observable workflow,
threshold, platform capability, or failure condition; “when needed” is not
admissible.

Use `assurance` only for tests, documentation, CI, or audit work that introduces
no runtime capability or ownership seam. Reference Product assembly and policy
work uses `reference-product`, even when it consumes or narrows an Agent Core
interface in the same compiling cutover.

The production-slice answer reuses
`acceptance_scenarios[].verification`, scenario/task
`production_call_sites`, and `verification.behavioral_tests`. Documentation
tasks may answer `not-applicable` and use their documentation-contract gate;
runtime tasks require both a production caller and behavioral proof.

### P.2 Adversarial Review

Review the registered sources and original draft in a fresh context. Use the
Workflow at `.claude/skills/opi-implement/scripts/plan.workflow.js` when the
runtime supports its bounded parallel agents; otherwise run the same lenses in
one fresh reviewer. Both paths return the same schema and disclose whether
independence is cross-model or fresh-context same-model.

The two non-collapsible axes are:

- **design readiness** — pi direction, justified Rust divergence, plugin-first
  placement, domain language, deep interfaces, public seams, shared decision
  ownership and closure, contradictions, and unstated assumptions;
- **execution readiness** — criterion coverage, demonstrable vertical slices,
  real dependencies, owned paths, production wiring, proportional verification,
  replace-don't-layer test impact, and forbidden scope.

Review all six minimum-change answers explicitly. When the conditional
simplification trigger applies, also verify `production_consumers=`,
`nonproduction_consumers=`, `net_deletion=`, and `residual_glue=`. An omitted
answer with otherwise sufficient source material is `GRAPH_REVISION_REQUIRED`;
a missing fact is `RESEARCH_REQUIRED`; an unsettled placement, public surface,
or simplification decision is `DESIGN_DECISION_REQUIRED`. Do not let generic
prose such as “reuse considered” satisfy a standardized note.

Reviewers report findings and try to reject unsupported findings. They do not
edit the source or draft, and no finding is auto-folded.

### P.3 Admission Verdict

The plan review returns exactly one primary verdict while retaining every
finding in the report:

- `READY` — no blocking design or graph finding remains;
- `RESEARCH_REQUIRED` — a blocking evidence gap remains;
- `DESIGN_DECISION_REQUIRED` — a blocking product/architecture decision
  remains;
- `GRAPH_REVISION_REQUIRED` — the reviewed source is adequate but the draft
  graph must change.

Verdict precedence when several blocking findings coexist is research, then
design decision, then graph revision. A non-`READY` verdict stops without
writing the canonical ledger. `GRAPH_REVISION_REQUIRED` may revise only the
draft, then must repeat P.2.

### P.4 Task-Graph Review Gate

Render complete draft as table with: id, title, tier, `task_owned_paths`
(default derived from `crate`, editable), commit_type, depends_on,
execution order, evaluator_required, acceptance scenario count,
production call-site count, `substrate_only`, phase source files,
forbidden-scope notes (stored as `inference_notes` entries with
`field = "forbidden_scope"`), inference_notes.

Also render an acceptance coverage table:

- source criterion/workflow;
- owning task;
- verification command/test;
- production call sites;
- forbidden-scope guard when the criterion is near a phase non-goal;
- status (`covered`, `substrate-only`, `missing`, `deferred`).

REFUSE `confirm-all` while any source criterion/workflow is `missing`, or while
any runtime criterion is covered only by a `substrate_only` task.

Also render a six-row minimum-change trace per task. For substrate tasks, show
the later scenario owner whose transitive `depends_on` closure contains the
substrate. Before graph confirmation, run
`python .claude/skills/opi-implement/scripts/validate-plan.py .opi-impl-state.draft.json`
on every platform. The validator excludes retained `status = archived`
reconciliation history. REFUSE `confirm-all` when that command fails,
a required answer is absent, any `surface_necessity` clause is missing,
`simplification_ceiling` omits
`revisit_when`, a triggered simplification claim omits its four conditional
consumer/deletion subclauses, a substrate has no later scenario owner, or a
runtime claim lacks either a production call site or behavioral verification.
This is evidence inside the existing human graph gate, not an additional gate.

Gate options after a `READY` adversarial verdict:
- **confirm-all** — accept the graph as shown
- **edit-task `<id>`** — modify one task's inferred fields
- **apply-rule `<selector>` `<field>` `<value>`** — batch edit (show before/after diff)
- **export-draft** — write `.opi-impl-state.draft.json` for human editing
- **import-draft** — validate schema, uniqueness, deps, cycles, tiers; re-render
- **abort** — stop without writing

Every edit or import invalidates `READY`, re-runs P.2, and then re-renders before
confirmation.
REFUSE to proceed until whole graph is confirmed.
MUST NOT silently apply inferred changes.

Graph confirmation authorizes installation of the canonical ledger; it does
not authorize a Git commit. After confirmation, present a separate checkpoint
choice:

- **write-only** — install the canonical ledger and leave the change
  uncommitted; this is the default for `opi-implement plan`;
- **commit-checkpoint** — explicitly authorize the narrowly scoped bootstrap or
  reconciliation commit described in A.init.6;
- **abort-before-write** — stop with only the ignored draft changed.

### P.5 Write Ledger
- Write `.opi-impl-state.json` atomically
- Ensure `.opi-impl-state.json` is tracked
- Add `.opi-impl-state.json.tmp`, `.opi-impl-state.draft.json`, candidate,
  backup, and corrupt ledger patterns to `.gitignore` if missing
- After the canonical install is verified, remove the ignored draft. Retain it
  only when the user exported it, stopped before installation, or explicitly
  needs it to resume graph review.

### A.init.5 Write Smoke Script
- `scripts/opi-impl-smoke.sh` (+ `.ps1` sibling on Windows)

### A.init.6 Commit

**Note:** This is the bootstrap checkpoint outside normal task Phase E. It
commits harness infrastructure and the confirmed canonical ledger, not task
implementation code.

Run this step only when the user selected **commit-checkpoint** at the separate
checkpoint choice. `confirm-all`, `plan`, or approval of the task graph alone
does not authorize a commit.

- Commit ONLY the canonical ledger, smoke scripts, and any `.gitignore` update
- Message: `chore: bootstrap opi-implement ledger and smoke`

### A.init.7 Print Summary
- Success message + next-task hint

## Schema Version Boundary

Live plan and execution paths accept `schema_version == 2` only. Refuse a
missing, v1, or future version with the offending value. Historical v1
snapshots remain readable by `opi-audit`; do not route them through a live
plan, mutate them, or promise an automatic migration.

## Reinit Reconciliation (drift branch of the plan path)

This is the drift branch of the unified plan path required by `AUTH-003`,
reached when the
plan path detects a `spec_files_sha256` mismatch — not via a separate `--reinit`
flag. When drift is detected on a bare (make-progress) or run-specific
invocation, the harness runs this reconciliation, then P.0 source admission and
P.1/P.2 draft review, then PRESENTS the P.4 gate only on a `READY` verdict and
pauses — it does not
auto-pick or run a task until the human confirms the reconciled graph. The
`plan` verb stops at the gate regardless.

When drift is detected against an existing ledger:

1. For each path in `spec_files`, recompute its SHA-256 and compare with
   `spec_files_sha256`. If every entry matches → refuse, suggest `--status`.
   If any differs → proceed.
2. Re-derive a fresh graph from the registered Phase delivery source while
   checking it against the parent specification.
3. Reconcile field-by-field:
   - **Both:** preserve `status`, `verified_at_commit`, `iteration_count`,
     `session_notes`, `blocker`
   - **Only in old:** warn, ask "keep history, mark `archived`?"
   - **Only in new:** add with `status: failing`
   - **DoD changed for passing task:** warn, ask preserve-as-passing (cosmetic)
     or demote-to-failing (substantive)
   - **depends_on/tier/commit_type/evaluator_required/acceptance_scenarios/production_call_sites/substrate_only/forbidden-scope inference_notes changed:** re-run
     task-graph review gate with row-level diff, require confirmation
4. Update every entry in `spec_files_sha256` to the freshly recomputed
   CRLF-normalized hash after confirmation.
5. Install the reconciled canonical ledger and any changed harness files. Only
   if the user separately selected **commit-checkpoint**, commit them with
   `chore: reconcile opi-implement harness files with registered source changes`.
6. The ignored draft is never committed. If reconciliation produces no
   canonical-ledger or harness-file change, do not create an empty commit.

### Changed Task Meaning

If a task ID is present in both old and new graphs but the title or DoD changes
substantively, show a row-level diff and default to:

- preserve runtime history in `session_notes`;
- keep `status = failing` unless the user explicitly confirms the old passing
  evidence still satisfies the new DoD;
- record an `inference_notes` entry with `field = "replaces"` when the new task
  intentionally supersedes an old one under the same ID.

Do not embed old Phase-specific meaning changes in the active initializer.
Frozen snapshots and Git history retain those examples.

## Draft Export/Import

**export-draft:** Writes `.opi-impl-state.draft.json` (gitignored scratch).

**import-draft:** Validates:
- Schema version
- Task ID uniqueness
- Dependency references exist
- No cycles
- Known tier names

Import never counts as confirmation by itself. If a draft promotes a deferred
row into executable, it must supply `definition_of_done` + inference note.

## apply-rule Examples

Batch graph edits for tedious one-by-one changes:
- Add a named substrate task as a dependency of every task that consumes it.
- Change every task owning `opi-tui` runtime paths to tier `tui`.
- Mark public-protocol tasks as `evaluator_required = true`.
- Mark every task participating in a `shared_decision` as
  `evaluator_required = true`.

Always show before/after diff for affected rows, invalidate the prior `READY`
verdict, repeat P.2, then return to P.4.
