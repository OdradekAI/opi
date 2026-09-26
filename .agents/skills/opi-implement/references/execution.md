# Task Execution (Phases A–F)

For normal task execution, complete A–F for the selected task. E owns the task
and ledger commits; F has a separate archive gate. The plan path uses
[initializer.md](initializer.md), including separate graph and
checkpoint-commit authorization.

1. **Phase A: Bootstrap**
   - A.1 Detect mode (status / clear-blocker / task-lifecycle / plan / run-specific / make-progress)
   - A.2 Load `.opi-impl-state.json`; if absent, enter the initializer's
     admission gates before creating the canonical ledger.
   - A.3 Record `HEAD`, branch, and baseline dirty files. Handle interrupted
     tasks through [ledger-schema.md](ledger-schema.md#interrupt-recovery).
   - A.4 Select the task and validate dependencies. For Rust/runtime tasks,
     run smoke `boot` (format + production lib/bin clippy); documentation tasks
     need no Cargo bootstrap. Status and plan modes never run task smoke.

2. **Phase B: Plan-the-task**
   - B.1 Print task DoD + verification tier + parallelize plan + owned
     acceptance scenarios + required production call-site traces + phase
     source files + phase-specific forbidden-scope guards + the six-answer
     minimum-change trace + every participating shared-decision note and its
     owner/consumer/Interface/closure-test map. For a graph confirmed before
     this contract, print
     absent answers as `legacy-unrecorded`; never fabricate them. The exemption
     ends when the graph next enters the plan path. Phase B does not reinterpret
     the answers. If implementation would change an admitted API, config item,
     state field, dependency edge, placement, or simplification limit, stop and
     return to graph review; the Phase C `task_owned_paths` append-only
     exception remains the sole in-task mutation of a const field.
   - B.1a If the task's product meaning is fuzzy — an unset scope boundary,
     contradictory source, or unresolved domain term — stop before marking it
     `in_progress` and apply the Source-return rule. Implementation-detail
     questions may be clarified here only when they do not change the reviewed
     DoD, acceptance scenario, or forbidden scope.
   - B.2 Obtain explicit authorization for this task and both the task commit
     and separate ledger-checkpoint commit after verification. Existing
     authorization for this exact task and both commits satisfies the gate;
     graph confirmation alone does not.
   - B.3 If confirmed: mark `in_progress`, record `start_commit`, write ledger

3. **Phase C: Implement**
   - C.1 For a code task, use Matt `tdd`. Before the first test for a runtime task,
     read the shared decision and test stewardship reference, record and
     confirm the highest practical public seam in the task plan, read existing
     tests at that Interface, and choose the smallest
     `add|update|delete|retain|none` disposition. Work one vertical red-green
     slice at a time; do not bulk-write imagined tests.
     - If `parallelize` contains disjoint owned units ->
       `superpowers:dispatching-parallel-agents`
   - C.1a If implementation requires modifying files outside
     `tasks[].task_owned_paths`, the harness MUST append the new glob to
     `task_owned_paths` and record an `inference_notes` entry
     (`field = "task_owned_paths"`, `reason = "<why>"`) via the atomic ledger
     write BEFORE the file is edited. Append is the only Phase C mutation of a
     const field; it never silently expands ownership.
   - C.2 For a hard bug, performance regression, nondeterministic failure, or a
     third unsuccessful attempt, open Matt `diagnosing-bugs` and establish its
     tight red-capable feedback loop before another fix.
   - C.3 At `max_iterations` (default 5), apply the failure decision gate.

4. **Phase D: Verify**
   - D.0 Product acceptance checks:
     - Run every `acceptance_scenarios` verification owned by the task.
     - For runtime/startup/CLI claims, prove the production call site exists and
       is exercised by the scenario. A direct helper/unit test is not enough.
     - If the scenario cannot be exercised yet, the task may pass only as
       substrate coverage and must leave the scenario open on a later vertical
       slice task.
     - Account for every test added, updated, deleted, or intentionally retained
       in `session_notes[].gate_results.test_disposition` using the shared
       reference. A task-local deletion requires a passing equal-or-stronger
       Interface replacement; an out-of-scope duplicate or superseded test
       remains unchanged with a non-null slim candidate.
   - D.0a Artifact truthfulness gate:
     - If a task claims runtime, CLI, JSON/NDJSON, RPC, session, provider, tool,
       browser, or generated-artifact behavior, read
       [artifact-truthfulness.md](artifact-truthfulness.md) before verification.
     - Preserve command, stdout, stderr, exit code, session/NDJSON artifacts,
       and provider/browser captures needed by the claim.
     - Run `scripts/opi-artifact-audit.py` or its platform wrapper when an
       artifact directory exists.
     - Classify each claim as `verified`, `observed-unpreserved`,
       `source-inferred`, or `not-opi`; only `verified` closes runtime
       acceptance criteria.
   - D.1 Run the task's one authoritative tier gate. `workspace` uses `smoke
     full`; focused Rust tiers use `smoke scoped --crate <crate> --test
     <name>...`; documentation uses `python scripts/opi-doc-check.py` plus diff
     checks. Phase addenda extend this command set without rerunning it.
   - D.2 Run the task-level risk evaluator only when
     `evaluator_required = true`. It invokes
     `.claude/skills/opi-implement/scripts/exec.workflow.js` (full 7-lens deep).
     Deterministic documentation, skill, test-only, mechanical, and
     behavior-preserving internal-refactor tasks skip D.2. Must-fix findings
     block Phase D and route to Phase C (incrementing `iteration_count`).
     Accepted findings with a non-null `decision_id` are appended to the current
     attempt's gate results. If that ID already appears in an accepted finding
     anywhere in the active phase task notes, stop local repair and return to
     graph review with `GRAPH_REVISION_REQUIRED`.
   - D.3 Run only acceptance, production-call-site, generated-artifact, or
     authoritative-platform checks still missing after D.0/D.1. Record the
     union of commands; never rerun D.1 under a second label. Then apply the
     commit-staging rules in [verification-tiers.md](verification-tiers.md).
   - D.4 If any fail -> back to Phase C within the iteration cap. After a fix,
     rerun affected checks and retain still-valid evidence for unchanged state.
     Never run `cargo clean` during or after a task.

5. **Phase E: Task Commit & Ledger Checkpoint**
   - E.1 Commit only task-owned implementation files with `Opi-*` evidence
     footers; never stage the dirty canonical ledger in this commit
   - E.2 Capture the task commit SHA + evidence, flip status to `passing`, and
     append the session note through the atomic ledger protocol
   - E.3 Validate and commit only `.opi-impl-state.json` with message
     `chore(opi-implement): checkpoint task <id> ledger`
   - E.4 No push (push is separate human action)

6. **Phase F: Phase-Exit Check**
   - F.1 If all phase tasks passing -> run phase-exit evaluator
   - F.1a Phase-exit evaluator MUST rebuild the source spec's success/exit
     criteria, goals, non-goals, and named workflows from the current spec
     files, inspect code/tests independently of ledger claims, and produce a
     criteria trace with one of:
     `met`, `deferred-by-updated-design`, or `not-met`. It then invokes
     `.claude/skills/opi-implement/scripts/phase-exit.workflow.js` (6-lens audit of the trace and completed phase tasks per
     [verify-engine.md](verify-engine.md)); accepted findings upsert
     `criteria_trace[C].status = not-met`, and F.1b REFUSEs archive.
   - F.1b REFUSE phase archive when any criterion is `not-met`, or when
     `deferred-by-updated-design` lacks an exact source citation from the
     current spec/plan.
   - F.2 Print phase-complete report; no auto-release
   - F.3 Else -> print "next unblocked: X.Y" hint
   - F.4 If F.1 passed, run the archive gate:
     - F.4a User gate: "Archive phase `<N>` ledger to
       `docs/snapshots/phase<N>/opi-impl-state.json` as a phase-local snapshot
       and compact `tasks` array into `phase_exit[<N>].task_summary`?"
     - F.4b If confirmed: write a phase-local snapshot containing the top-level
       schema/spec metadata, only the completed tasks for phase `<N>`, and only
       `phase_exit[<N>]` (including any detailed `criteria_trace`; do not copy
       prior phases' `phase_exit` records into the snapshot). Refuse pruning
       unless the full pre-archive ledger state is already present in a durable
       checkpoint commit. Then mutate the root ledger via atomic protocol:
       move completed tasks into `phase_exit[<N>].task_summary`, set
       `phase_exit[<N>].snapshot_path`, keep only the five compact phase-exit
       fields, clear the root `session_notes` and `verify_runs` arrays, and
       remove those tasks from the active `tasks` array. Commit ONLY the new
       snapshot and canonical ledger with message
       `chore: archive opi-implement phase <N> ledger snapshot`.
     - F.4c If declined: leave tasks array intact; no snapshot written.

**When D.2 or F.1a runs:** Read [verify-engine.md](verify-engine.md) for that stage.

**When Phase D runs:** Read [verification-tiers.md](verification-tiers.md) for gate details.

**When iteration cap hits:** Read [failure-gate.md](failure-gate.md) for the protocol.

## Parallel Sub-Unit Contract

When `parallelize` is non-empty:

- Sub-agents work on disjoint files; MUST NOT create commits
- Parent integrates results in ledger order, then runs the task verification
  union on the assembled result; rerun only evidence invalidated by later edits
- Completion events may arrive out of order; persisted evidence uses `parallelize` array order
- Conflict or overlapping edit -> fail attempt -> normal debug/failure path
