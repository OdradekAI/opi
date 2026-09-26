---
name: opi-implement
disable-model-invocation: true
description: Admit and execute the canonical Opi implementation ledger one reviewed task at a time.
---

# opi-implement

Admit a reviewed task graph or execute one admitted task through verification
and a canonical ledger checkpoint. Invoke explicitly. Code tasks use TDD;
documentation-only tasks use the no-compile documentation gate. Graph approval,
task commits, and phase archive retain the explicit gates below.

**Spec alignment rule:** Before executing any task whose `phase >= current_phase`,
compare each entry in the ledger `spec_files_sha256` map with the current
CRLF-normalized hash of the corresponding file in `spec_files`. If any entry differs, auto-enter the
plan path's drift branch (Reinit Reconciliation + source admission + adversarial
draft review + the human graph gate) per `references/initializer.md` and
`AUTH-003`/`PHASE-001` in `docs/opi-spec.md`; do not
auto-pick or run a task until the human confirms the reconciled graph. Only
`--status` bypasses drift handling. A retry for an archived phase is allowed
only when its frozen snapshot and `Opi-DoD-SHA256` commit footer identify the
same shipped contract; it never reopens or rewrites the snapshot. Do not run
stale ledger tasks whose title or DoD contradicts the current registered source.

**Source-return rule:** The harness never repairs missing or wrong product
meaning while initializing a graph or executing a task. Return missing evidence
to `opi-research` / `opi-realign`; return unresolved product decisions to
human-led shaping, which can continue in the current conversation. Suggest
`wayfinder` or `grill-with-docs` only when installed and requested by the user;
their absence does not block shaping. Adapter tracker artifacts never enter
`spec_files`; only human-reviewed results materialized in `docs/opi-spec.md`
or a registered supplemental source may be admitted by plan. Frozen copies
under `docs/snapshots/phaseN/` are never edited.

**Reviewed supplemental sources:** Supplemental tasks come only from this
registry. Do not auto-parse arbitrary files from `docs/superpowers/specs/`.

| Phase | Reviewed source | Localized counterpart |
|---|---|---|
| 19 | [Local Native Agent Evaluation](../../../docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md) | [简体中文](../../../docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.zh.md) |

For Phase 19, include both listed files alongside `docs/opi-spec.md` in
`spec_files`; derive criteria and source outcomes once from the English source,
and check the Chinese counterpart for equivalent meaning. Registration does
not admit the graph or authorize live provider execution. The source's local
operator acceptance handoff preserves this harness's provider/auth-file limits.

Completed Phase 17 and Phase 18 delivery specifications are frozen under
`docs/snapshots/phase17/` and `docs/snapshots/phase18/`; they are historical
evidence and MUST NOT enter the live `spec_files` set.

When a ledger is initialized or reconciled for a supplemental phase, the
phase's registered source files MUST be included in `spec_files` and hashed in
`spec_files_sha256` alongside `docs/opi-spec.md`. Registered supplemental
phases derive draft task rows from the registered design's goals, per-ticket
design sections, sequencing, and residuals, then require task-graph review
before execution.

**Product-loop integrity rule:** Every source goal, success/exit criterion, and
named workflow needs an owned `acceptance_scenarios` entry with executable
verification and production call-site evidence for runtime claims. Substrate
tests cannot close product acceptance. Expand vague DoDs into observable
entry points, artifacts, effects, diagnostics, and relevant negative behavior
during admission; [initializer.md](references/initializer.md) owns extraction.

**Minimum-change trace rule:** Admission requires six answers: sourced
criterion, reuse search, placement, surface necessity, smallest production
slice, and simplification ceiling with an observable revisit trigger. Use the
existing scenario/verification/dependency fields and `inference_notes` keys
`reuse_search`, `placement`, `surface_necessity`, and `simplification_ceiling`;
do not add another trace object or change the schema. Each substrate must be
in a later scenario owner's transitive dependency closure.

Every new or reconciled draft declares
`simplification_trigger=none|unused|duplicate|superseded|delete|merge|replace|dependency-substitution`.
A non-`none` trigger requires `production_consumers`, `nonproduction_consumers`,
`net_deletion`, and `residual_glue`, backed by repository evidence including
applicable dynamic/configuration/wire/persistent/public-API use. The exact
note grammar and controlled placement vocabulary live in
[initializer.md](references/initializer.md#minimum-change-trace). Before graph
confirmation on every platform, run
`python .claude/skills/opi-implement/scripts/validate-plan.py .opi-impl-state.draft.json`;
missing, duplicated, or malformed declarations block admission. Retained
`status = archived` history is excluded because it is not executable.
Already-confirmed no-drift graphs remain executable; report absent pre-contract
answers as `legacy-unrecorded` until their next plan admission.

When plan admission detects a shared semantic decision, read
`../_shared/references/shared-decision-and-test-stewardship.md` in full and
apply its trigger and plan-note contract before P.4. Do not manufacture a
shared seam for an ordinary one-consumer helper.

## Invocation

```text
opi-implement                                  # make-progress: sync-if-needed -> auto-pick -> run (no-drift); plan+pause (drift)
opi-implement plan                             # sync-only plan path (init or drift-reconcile), don't run
opi-implement <task-id>                        # specific task (sync-if-needed first; validates deps)
opi-implement --status                         # print ledger summary
opi-implement <task-id> --resume-from-manual   # verify a manual commit
opi-implement <task-id> --extend-cap <N>       # set a higher absolute iteration cap
opi-implement --clear-blocker <id> --because <text>  # unblock a task
```

## Mode Detection

Dispatch order (first match wins): `--status` → status mode; `--clear-blocker`
→ clear-blocker mode; `<task-id>` with `--resume-from-manual` / `--extend-cap`
→ task-lifecycle mode; `plan` verb → plan-only mode (sync, then stop at a
non-`READY` admission verdict or the P.4 gate); `<task-id>` → run-specific mode
(sync-if-needed first);
bare → make-progress mode (sync-if-needed → auto-pick → run). Only `--status`
bypasses drift handling.

Apply the Spec alignment rule before mutation or task selection. No-drift
make-progress and run-specific modes continue to execution; drift enters
plan+pause at P.4. `--status` is read-only. Plan, status, clear-blocker, and
lifecycle modes run only their applicable steps, not the full task sequence.

**Auto-pick rule:** Lowest task ID (lexicographic, numerically aware) whose
`status` is `failing` AND every `depends_on` entry is `passing`. A dependency
is satisfied if it appears as `passing` in the active `tasks` array OR in any
`phase_exit[*].task_summary` entry. Tasks with `status: blocked` are skipped
until `--clear-blocker`.

**User-override rule:** Refuse if any `depends_on` is not satisfied by the
active tasks or archived phase summaries; print which dep is missing.

## Six Phases Per Invocation

For task execution or manual-commit recovery, read
[execution.md](references/execution.md) before starting. It owns A–F: bootstrap,
reviewed task contract and commit authorization, implementation, verification,
separate task/ledger commits, and independently verified phase exit/archive.
Continue through the selected task within existing authorization until those
completion conditions pass or a documented gate blocks progress.

For plan/init/drift reconciliation, read
[initializer.md](references/initializer.md) instead. Plan admission has separate
source review, adversarial review, human graph, and checkpoint-commit gates.
Status and clear-blocker modes use the procedures below.

## Task Selection

Before selecting another task, resolve any interrupted `in_progress` task with
no recorded commit through
[Interrupt Recovery](references/ledger-schema.md#interrupt-recovery). Preserve
its work and offer the recorded continue/block/manual choices. Otherwise use
the auto-pick and dependency rules above.

## Composition With Sub-Skills

Use `codebase-design` only when the public seam is unclear, `tdd` for code
behavior, and `diagnosing-bugs` at C.2's failure trigger. These supply local
techniques; they do not create another task, commit, or progress state machine.
Plan, exec, and phase-exit review use the bounded protocols in
[verify-engine.md](references/verify-engine.md). Read that reference only for
the stage being run.

Resolve a named helper from the available skill/tool catalog before invoking
it. If a required helper is unavailable, use only a documented fallback or
report the affected gate as blocked; never claim an invocation that did not run.

## Parallel Sub-Unit Contract

Dispatch only the admitted disjoint units in `parallelize`; follow
[execution.md](references/execution.md#parallel-sub-unit-contract) for ownership,
ordering, assembled verification, and failure handling. Sub-agents never commit.

## Commit Evidence Format

Before Phase E, use the exact `Opi-Task`, `Opi-DoD-SHA256`, `Opi-Verification`,
`Opi-Evaluator`, and conditional `Opi-Acceptance` footers from
[Durable Evidence Contract](references/ledger-schema.md#durable-evidence-contract).
Derive commit type from `commit_type` and scope from the owning crate. Stage
implementation files separately from the canonical ledger.

## Ledger Location & Safety

Read [ledger-schema.md](references/ledger-schema.md) before any ledger mutation,
interrupt recovery, conflict reconciliation, or phase archive. It owns schema
v2, field mutability, strict UTF-8/size/secret validation, atomic writes,
checkpoint boundaries, and phase-local compaction. Frozen snapshots stay frozen.

- `.opi-impl-state.json` is the tracked canonical ledger. Temp, draft,
  candidate, backup, and corrupt copies stay ignored and uncommitted.
- Use structured JSON and optimistic concurrency. On Windows, validate/install
  through `.claude/skills/opi-implement/scripts/ledger-guard.ps1`, passing the
  observed hash as `-ExpectedTargetSha256`; on mismatch stop and re-read.
  Recovery also passes `-BackupPath`. Never use Windows PowerShell 5.1 default
  text encoding for ledger data.
- Capture baseline dirty files before implementation; preserve unrelated work.
  Apply the staging overlap checks in `references/verification-tiers.md`.
- Refuse worktree removal while the canonical ledger is dirty, staged, or
  untracked, a temp remains, or required checkpoints are absent from the
  destination branch. Reconcile conflicts through both branches' `Opi-*`
  evidence instead of choosing one side.

## Platform Detection

Use the host-native smoke wrapper: `scripts/opi-impl-smoke.ps1` for native
PowerShell, `scripts/opi-impl-smoke.sh` for Linux/macOS or Windows bash with
forward-slash paths. Modes are `boot` at A.4 for runtime tasks, `full` for
workspace D.1, and `scoped --crate <crate> [--test <name>...]` for focused Rust
D.1. Read [verification-tiers.md](references/verification-tiers.md) for the
selected tier and applicable addenda.

Respect `CARGO_TARGET_DIR`; if unset use
`python scripts/opi-cargo-cache.py resolve`. Smoke wrappers hold Cargo process
leases. Reuse the persistent worktree/toolchain cache, keep incremental
compilation enabled, and leave cache reclamation to separate maintenance.
Inspect with `python scripts/opi-cargo-cache.py status`.

For `spec_files_sha256`, hash file bytes after CRLF→LF normalization. Historical
snapshot hashes are never resynced. Toolchain/MSRV requirements come from the
workspace manifest; `gh` is needed by release, not implementation.

## Red Flags - STOP Immediately

Read `references/anti-patterns.md` for the full reasoned table. Stop
immediately when work would weaken tests, self-grade evidence, rewrite the
confirmed graph, touch unrelated user changes, commit transient ledgers, close
a product scenario with substrate-only proof, implement a registered Non-Goal,
or admit an unregistered source.

The skill refuses to act if any rule would be violated, even if the user
requests it during a failure-decision gate.

## Status Mode (`--status`)

Print a summary table of all tasks:
- id, title, status, tier, depends_on (with pass/fail indicators)
- Current phase number
- Current `spec_files` and any hash drift warning
- Next unblocked task hint
- Any blocked tasks with blocker text
- Phase-exit status for completed phases

## Clear-Blocker Mode (`--clear-blocker <id> --because <text>`)

1. Validate task exists and `status = blocked`
2. Append `--because` text to `session_notes`
3. Clear `blocker` field
4. Set `status = failing`
5. Write ledger
6. Print confirmation + next-task hint

## Scope Boundaries (Never Cross)

- Editing `docs/opi-spec.md` except for a reviewed documentation/alignment task
  whose DoD explicitly owns `docs/opi-spec.md` and its localized counterpart
- Pushing commits or tags to `origin`
- Publishing to crates.io
- Building cross-platform binaries
- Network calls to any provider API
- Opening GitHub issues, PRs, or releases
- Reading/writing user runtime data such as `~/.config/opi/`, real auth files,
  or real session storage. Editing source code for config/session behavior is
  allowed only when the selected spec task owns that behavior.
