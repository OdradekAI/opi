# Initializer & Reinit Reference

## Init Mode (A.init)

Triggered when `.opi-impl-state.json` is absent (fresh) or when the unified plan
path detects spec drift. The `plan` verb forces the plan path; a bare or
run-specific invocation enters it only on drift.

### A.init.1 Pre-flight
- Record current branch, baseline dirty files, and `opi-spec.md` presence.
  Refuse only when dirty files would be overwritten by init/reinit outputs;
  do not require unrelated user changes to be cleaned.

### A.init.2 Parse Spec
Parse `opi-spec.md` §15 roadmap tables. For each task row extract:
- id, title, crate, DoD (when present), phase number
- **Infer:** tier (from crate + description), commit_type (from task verbs),
  depends_on (from ordering + DoD references), evaluator_required (from risk rules)
- Attach `inference_notes` for every non-verbatim field
- Documentation/alignment rows whose source phase explicitly forbids runtime
  behavior or code migration use tier `documentation`. If any inferred task
  owns Rust source, Cargo manifests, runtime scripts, fixtures, snapshots, or
  generated artifacts, promote it to the relevant non-documentation tier before
  graph confirmation.
- Rows without explicit DoD:
  - Phase 1 rows with a "Definition of done" column use that text verbatim.
  - Phase 2+ rows may receive a draft `definition_of_done` inferred from the
    roadmap row, feature parity matrix, relevant crate section, security
    requirements, and phase exit criteria.
  - Every inferred DoD MUST include `inference_notes` with source section names.
  - The task remains non-executable until the task-graph review gate confirms
    the inferred DoD.

### A.init.2d Reviewed Supplemental Task Sources

Supplemental phases are sourced only from the reviewed
design registry in `skill.md`. Do not scan arbitrary `docs/superpowers/specs/` files.

| Phase | Registered source | Draft task extraction |
|---:|---|---|
| 14 | `docs/superpowers/specs/2026-07-11-phase14-provider-auth-design.md` | Goals, Non-Goals, T1 Credential store, T2 OAuth + per-request auth, T3 Request enrichment, Sequencing, Residuals |
| 14 | `docs/superpowers/specs/2026-07-14-phase14-exit-remediation-design.md` | Phase-exit remediation criteria and residuals |
| 15 | `docs/superpowers/specs/2026-07-11-phase15-safety-sandbox-design.md` | Goals, Non-Goals, T4 OS-native sandbox, T5 Operations seam, T6 Project-trust gate, Sequencing, Cross-ticket interactions, Residuals |
| 16 | `docs/superpowers/specs/2026-07-28-phase16-pluggable-extension-command-execution-design.md` | Goals, Non-Goals, Product Contract, Configuration and Routing, Executable Package Lifecycle, Protocol, standalone SDK/CLI, Testing and Acceptance, Phase Integration |
| 18 | `docs/superpowers/specs/2026-07-11-phase18-agent-intelligence-design.md` | Goals, Non-Goals, T7 Skills/templates runtime, T8 LLM compaction + branch-summary, T9 Read-tool inline image, Sequencing, Cross-ticket interactions, Residuals |

For each active phase:

- Include `docs/opi-spec.md` and the phase's registered source in `spec_files`.
- Hash both files in `spec_files_sha256` using the CRLF-normalized SHA-256
  (replace `\r\n` with `\n` before hashing; see `skill.md`).
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

### A.init.2c Design Acceptance Extraction

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

### A.init.2a Composite Row Detection

Some spec roadmap rows describe N independent deliverables in one line
(e.g. `4.6 | extension examples: permission gate, sub-agent, plan mode, todo, MCP adapter`).
These rows MUST NOT become a single ledger task.

Trigger heuristic: a roadmap row is composite when any of these is true:

- the row title contains `:` followed by at least two comma-separated items;
- the row title begins with `examples:` or `task family:`;
- the row title is a Phase 4 resource-family row listing at least three independent resource nouns joined by commas or `and`, such as `skills, prompt fragments, themes, and packages`;
- the row's crate column is an open packaging identifier such as `examples / package template` and the title lists at least two deliverables.

Do not split a row merely because the DoD contains commas. The split decision is based on the roadmap row title and crate column.

For each composite row:

- Generate sub-tasks with IDs `<row>.1`, `<row>.2`, ..., `<row>.N`.
- Set `parent_spec_row = "<row>"` on each.
- Independent draft DoD per item, drawn from the item phrase plus relevant
  spec sections.
- Each sub-task inherits the parent's `depends_on` unless review-gate narrows.
- Each sub-task inherits the parent's `crate` (or `"package-template"` /
  `"examples"` when the row's crate column is non-standard).
- `definition_source = "inferred"` for every sub-task (composite rows never
  produce verbatim DoDs).
- The composite row itself does NOT produce a parent task; there is no
  placeholder entry with id `<row>`.
- The task-graph review gate MUST surface composite decompositions in a
  dedicated section so the user reviews them as a unit before confirmation.

Phase 4 examples:

- `4.7 | skills, prompt fragments, themes, and packages with progressive discovery` becomes `4.7.1` skills, `4.7.2` prompt fragments/templates, `4.7.3` themes, and `4.7.4` packages.
- `4.8 | extension/package examples: permission gate, protected paths, sub-agent, plan mode, todo, MCP adapter` becomes six package/example tasks; the parent row is not executable.

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
- `DESIGN_DECISION_REQUIRED` — route to Matt `wayfinder` or
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

### P.2 Adversarial Review

Review the registered sources and original draft in a fresh context. Use the
Workflow at `.claude/skills/opi-implement/scripts/plan.workflow.js` when the
runtime supports its bounded parallel agents; otherwise run the same lenses in
one fresh reviewer. Both paths return the same schema and disclose whether
independence is cross-model or fresh-context same-model.

The two non-collapsible axes are:

- **design readiness** — pi direction, justified Rust divergence, plugin-first
  placement, domain language, deep interfaces, public seams, contradictions,
  and unstated assumptions;
- **execution readiness** — criterion coverage, demonstrable vertical slices,
  real dependencies, owned paths, production wiring, proportional verification,
  and forbidden scope.

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

### P.5 Write Ledger
- Write `.opi-impl-state.json` atomically
- Ensure `.opi-impl-state.json` is tracked
- Add `.opi-impl-state.json.tmp`, `.opi-impl-state.draft.json`, candidate,
  backup, and corrupt ledger patterns to `.gitignore` if missing

### A.init.5 Write Smoke Script
- `scripts/opi-impl-smoke.sh` (+ `.ps1` sibling on Windows)

### A.init.6 Commit

**Note:** This is the bootstrap checkpoint outside normal task Phase E. It
commits harness infrastructure and the confirmed canonical ledger, not task
implementation code.

- Commit ONLY the canonical ledger, smoke scripts, and any `.gitignore` update
- Message: `chore: bootstrap opi-implement ledger and smoke`

### A.init.7 Print Summary
- Success message + next-task hint

## Schema Version Migration

On every invocation, inspect `schema_version` from the ledger before any other
step.

- `schema_version == 2` (current): proceed.
- `schema_version == 1`: route the ledger into the unified plan path's
  fresh/drift detection. Print "Ledger is v1; running v1 → v2 migration as part
  of plan-path sync." Apply the v1 → v2 migration documented in
  `ledger-schema.md`, then continue with the rest of plan-path sync (drift
  reconciliation below + P.0 source admission + P.1/P.2 draft review + P.4
  gate).
- `schema_version > 2` or missing: refuse with an explicit message identifying
  the offending value.

## Reinit Reconciliation (drift branch of the plan path)

This is the drift branch of the unified plan path (spec §5.3), reached when the
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
2. Re-parse spec into fresh ledger.
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
5. Commit the reconciled canonical ledger and any changed harness files
   (`.gitignore`, smoke) with
   `chore: reconcile opi-implement harness files with opi-spec.md changes`
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

Examples from the 2026-05-25 spec adjustment:

- `3.7 OPI.md context loading` becomes `3.7 AGENTS.md / CLAUDE.md context loading`;
- `3.8 permission profiles and policy system` becomes `3.8 pi-style tool selection and safety hooks`;
- `3.9 MCP client adapter` becomes `3.9 find / ls built-in tool parity`;
- MCP moves to Phase 4 as an extension/package example, not a Phase 3 core task.

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
- Add `1.17` as dep to every task using `MockProvider`
- Change all `opi-tui` rows to tier `tui`
- Mark public-protocol rows as `evaluator_required = true`

Always show before/after diff for affected rows, invalidate the prior `READY`
verdict, repeat P.2, then return to P.4.
