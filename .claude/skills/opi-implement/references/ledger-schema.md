# Ledger Schema Reference

Path: `.opi-impl-state.json` at repository root. Git-tracked canonical ledger.
Atomic writes use an ignored `.opi-impl-state.json.tmp` plus rename.

## Schema

```json
{
  "schema_version": 2,
  "spec_files": ["docs/opi-spec.md"],
  "spec_files_sha256": {
    "docs/opi-spec.md": "<hash at last init/reinit>"
  },
  "task_graph_confirmed_at": "2026-05-20T09:30:00Z",
  "current_phase": 1,
  "tasks": [
    {
      "id": "1.6",
      "phase": 1,
      "title": "agent_loop",
      "crate": "opi-agent",
      "definition_of_done": "mock tests cover no-tool and tool-use turns",
      "definition_source": "verbatim",
      "replaces": null,
      "status": "failing",
      "depends_on": ["1.1", "1.2", "1.5"],
      "inference_notes": [
        {
          "field": "depends_on",
          "reason": "agent_loop consumes provider and tool traits",
          "source": "opi-spec.md §15 + DoD references"
        }
      ],
      "tier": "library",
      "commit_type": "feat",
      "parallelize": [],
      "evaluator_required": false,
      "verification": {
        "library_gates": [
          "cargo test -p opi-agent",
          "cargo clippy -p opi-agent -- -D warnings",
          "RUSTDOCFLAGS=\"-D warnings\" cargo doc -p opi-agent --no-deps"
        ],
        "behavioral_tests": ["crates/opi-agent/tests/agent_loop_mock.rs"],
        "snapshot_tests": [],
        "smoke_addendum": null
      },
      "acceptance_scenarios": [
        {
          "id": "phase1-agent-loop-tool-use",
          "source": "docs/opi-spec.md §15 Phase 1 exit criteria",
          "scenario": "Mock provider requests a tool, the production agent loop validates and executes it, and the next provider turn receives the result.",
          "verification": [
            "cargo test -p opi-agent --test agent_loop_mock tool_use_turn"
          ],
          "production_call_sites": [
            "opi_agent::agent_loop"
          ],
          "status": "open"
        }
      ],
      "production_call_sites": ["opi_agent::agent_loop"],
      "substrate_only": false,
      "iteration_count": 0,
      "max_iterations": 5,
      "start_commit": null,
      "baseline_dirty_files": [],
      "task_owned_paths": ["crates/opi-agent/**", "Cargo.toml"],
      "last_attempt": null,
      "verified_at_commit": null,
      "evidence": null,
      "blocker": null,
      "session_notes": []
    }
  ],
  "phase_exit": {
    "1": {
      "completed_at": "2026-04-12T18:00:00Z",
      "exit_criteria_met": true,
      "evaluator_summary": "all Phase 1 exit criteria met; see commit 4d9c64...",
      "snapshot_path": "docs/snapshots/phase1/opi-impl-state.json",
      "task_summary": [
        { "id": "1.0", "title": "introduce Phase 1 dependencies", "status": "passing", "verified_at_commit": "4d9c64..." }
      ]
    }
  }
}
```

## Field Semantics

| Field | Type | Mutability | Notes |
|---|---|---|---|
| `schema_version` | int | reinit-only | Current value `2`. v2 adds `task_owned_paths`, `definition_source`, `replaces`, `baseline_dirty_files`, `spec_files`, `spec_files_sha256`, `phase_exit[N].snapshot_path`, `phase_exit[N].task_summary`, dotted sub-task IDs, and open-string `crate` values. Reading a v1 ledger requires explicit reinit-time migration; refuse unknown versions. |
| `spec_files` | array | const-on-init, reinit-editable | Normative spec file paths whose drift triggers the plan path's drift branch. Default `["docs/opi-spec.md"]`. Supplemental phases MUST include only the reviewed source files registered in `skill.md` for the active phase, plus `docs/opi-spec.md`. Adding or removing a path requires a plan-path sync. |
| `spec_files_sha256` | object | reinit-only | Map of file path → its CRLF-normalized SHA-256 (replace `\r\n` with `\n` before hashing) at last init/reinit. The live root `.opi-impl-state.json` is pinned to the current spec by `crates/opi-coding-agent/tests/spec_ledger.rs`; phase-exit snapshots under `docs/snapshots/phaseN/` are historical and are NOT re-synced. Each entry is checked independently; any mismatch triggers the spec-alignment guard. |
| `task_graph_confirmed_at` | string/null | init/reinit | ISO-8601 confirmation time |
| `verify_runs` | array/null | plan+exec+phase-exit | Active-phase verify history. Each entry: `{ stage ("plan"|"exec"|"phase-exit"), wf_ref (string/null — null only when a supported plan fallback has no Workflow id), folded_count, flagged_count, rejected_count, ran_at, task_id (string for risk-gated exec; null for plan/phase-exit), criterion_id (string for phase-exit; null for plan/exec) }`. Tasks with `evaluator_required = false` create no exec entry. Additive/optional within a phase; after a durable pre-archive ledger checkpoint, archive compaction resets it to `[]`. The checkpoint remains the recovery source; do not duplicate the array into a generic history artifact. Does NOT affect `schema_version`. |
| `session_notes` | array/null | plan+runtime+archive | Active-phase root coordination notes. This is distinct from per-task `tasks[].session_notes`. After a durable pre-archive ledger checkpoint, archive compaction resets it to `[]`; archived-phase notes do not accumulate in the live root ledger and remain recoverable from the checkpoint. |
| `current_phase` | int | auto | Lowest phase with non-`passing` task |
| `tasks[].id` | string | const | Matches a row in `opi-spec.md` §15 OR a sub-task expansion. Pattern: `^\d+\.\d+(\.\d+)?$`. Sub-task IDs carry a third component (e.g. `4.6.1`) and MUST also set `parent_spec_row`. |
| `tasks[].phase` | int | const | From row's phase grouping |
| `tasks[].title` | string | const | Spec row title |
| `tasks[].crate` | string | const | One of opi's six workspace crates (`opi-ai`, `opi-agent`, `opi-coding-agent`, `opi-protocol`, `opi-sandbox`, `opi-tui`), `workspace`, or any free-string packaging identifier (e.g. `examples`, `package-template`) when the reviewed source uses an open identifier. The review gate warns for unknown values but does not refuse solely on that basis. |
| `tasks[].parent_spec_row` | string/null | const | Source spec row ID when this task is a sub-task expansion (e.g. `"4.7"` for `4.7.1`). Direct spec rows MUST use `null`, not an empty string. |
| `tasks[].definition_of_done` | string | const | Verbatim from spec |
| `tasks[].definition_source` | enum | const | `verbatim`, `inferred`, or `draft-reviewed`; inferred values require review gate confirmation |
| `tasks[].replaces` | string/null | const | Prior task title/meaning superseded during reinit, when the same task ID was repurposed by spec changes |
| `tasks[].status` | enum | runtime | `failing`/`in_progress`/`passing`/`blocked`/`archived` |
| `tasks[].depends_on` | array | const | Task IDs that must be `passing` |
| `tasks[].inference_notes` | array | const | Reasons for inferred fields. Phase non-goal guards use `field = "forbidden_scope"` with an exact source heading. Plan extraction may use `field` ∈ {`deferred`,`split`,`residual`} with `reason` packed as `"<verb>: trigger=<clause|null>"` (a `null` trigger requires a human decision before P.4 confirmation). Inferred placement or public-test-seam choices also record their rationale here. |
| `tasks[].tier` | enum | const | `documentation`/`workspace`/`library`/`cli-tool`/`cli-runtime`/`tui` |
| `tasks[].commit_type` | enum | const | `feat`/`fix`/`docs`/`refactor`/`test`/`chore`/`perf` |
| `tasks[].parallelize` | array | const | Sub-unit names for parallel dispatch |
| `tasks[].evaluator_required` | bool | const | Static risk flag |
| `tasks[].verification` | object | const | Tier-specific gate spec |
| `tasks[].acceptance_scenarios` | array | const-on-init, reinit-editable | Product/user-path scenarios owned by this task. Required when the task closes a source-spec goal, success criterion, exit criterion, or workflow. Each scenario has `id`, `source`, `scenario`, `verification`, `production_call_sites`, and runtime `status` (`open`, `met`, or `deferred-by-updated-design`). `scenario` answers what can be demonstrated when the task is complete; `source` cites the reviewed criterion. Component/substrate tasks may use `[]`, but then they cannot close a product acceptance criterion. |
| `tasks[].production_call_sites` | array | const-on-init, append-only during Phase C | Production entry points that must call or exercise this task's implementation before the task can close runtime acceptance. Examples: CLI subcommand handler, harness startup, agent loop hook wrapper, session persistence path. Tests-only helpers do not count. |
| `tasks[].substrate_only` | bool | const-on-init, reinit-editable | `true` means the task intentionally implements a helper/parser/protocol/bridge slice and cannot by itself close product acceptance scenarios. A later vertical-slice task must consume it through a production call site. |
| `tasks[].iteration_count` | int | runtime | Attempts since `in_progress` |
| `tasks[].max_iterations` | int | const | Default 5 |
| `tasks[].start_commit` | string/null | runtime | HEAD when Phase B confirms |
| `tasks[].baseline_dirty_files` | array | runtime | Files already dirty at Phase B start; used to avoid cleaning or staging unrelated user work |
| `tasks[].task_owned_paths` | array | const-at-Phase-B, append-only during Phase C | Glob patterns the task is allowed to modify. Default derived from `crate` at init/reinit time (e.g. `crate = "opi-agent"` → `["crates/opi-agent/**", "Cargo.toml"]`). Phase C MAY append entries when implementation requires touching outside-prefix files; each append MUST add an `inference_notes` entry with `field = "task_owned_paths"` and a `reason`, written via the atomic ledger write. |
| `tasks[].last_attempt` | object/null | runtime | `{attempt, started_at, ended_at, outcome, failing_gate, touched_files}` |
| `tasks[].verified_at_commit` | string | runtime | Set in Phase E.2 |
| `tasks[].evidence` | object/null | runtime | Mirror of `Opi-*` commit footers |
| `tasks[].blocker` | string | runtime | Populated when `status = blocked` |
| `tasks[].session_notes` | array | runtime | Append-only `{timestamp, attempt, summary, gate_results}` |
| `phase_exit[N]` | object | runtime | Before archive it may carry evaluator working detail. After archive the root entry contains only `completed_at`, `exit_criteria_met`, `evaluator_summary`, `snapshot_path`, and `task_summary`; `evaluator_summary` is at most 256 characters and points to the snapshot for detail. |
| `phase_exit[N].snapshot_path` | string/null | runtime | Path to a committed phase-local snapshot at the moment phase `N` exited. `null` while the phase is incomplete. Written under `docs/snapshots/phase<N>/`. |
| `phase_exit[N].criteria_trace` | array | runtime/archive | Phase-exit evaluator's independent trace from current source-spec success/exit criteria to evidence. Every item uses `status = met`, `deferred-by-updated-design`, or `not-met`. Phase archive is refused if any item is `not-met` or if a deferral lacks an exact current-spec citation. Preserve the detailed trace in the phase-local snapshot; remove it from the root entry during archive compaction. |
| `phase_exit[N].task_summary` | array | runtime | `[{id, title, status, verified_at_commit}]` for every task that belonged to phase `N` at exit time. Lets `--status` report completed phases without reading the snapshot file. |

Archive snapshots are intentionally phase-local: they include top-level
schema/spec metadata, the archived phase's completed `tasks`, and only that
phase's `phase_exit[N]` record. Do not copy older `phase_exit` records into new
snapshots or create a generic root-history artifact. The full pre-archive
ledger checkpoint is the recovery source for pruned coordination journals. The
root ledger remains the compact index for dependency checks and status
reporting through `phase_exit[*].task_summary`; it holds only the five compact
exit fields and active-phase working history, not expanded evidence tables or
prior-phase journals.

Validation rule: every path listed in `tasks[].verification.behavioral_tests` MUST be matched by at least one `task_owned_paths` glob before the task graph is confirmed. This prevents Phase C from needing an immediate ownership expansion just to create the task's declared tests.

Validation rule: `tasks[].verification.behavioral_tests` MUST exercise the
pre-agreed highest practical public seam for each owned acceptance scenario.
When the seam is inferred rather than verbatim from the source, record the seam
and rationale in `inference_notes`. A private helper test may supplement but
cannot replace the public behavioral seam.

Validation rule: when `behavioral_tests` references more than one crate, either `tier` MUST be `workspace` or `verification.library_gates` MUST include mechanical gates for every referenced crate. Snapshot-bearing tests also require `snapshot_tests` and explicit snapshot approval under the `tui` rules.

Validation rule: `task_owned_paths` MUST NOT include broad documentation globs
such as `docs/**` when a narrower subtree can satisfy the task. Use a
purpose-specific path such as `docs/extension-examples/**` for example
packages. `docs/opi-spec.md` is normative input and MUST NOT be task-owned
unless the task is a reviewed documentation/alignment task whose DoD explicitly
requires updating `docs/opi-spec.md` and its localized counterpart.

Validation rule: every source-spec success criterion, exit criterion, goal, or
named user workflow for the active phase MUST be represented by at least one
`acceptance_scenarios` entry before the task graph is confirmed. If the
criterion is intentionally deferred, the scenario must be assigned to a
documentation/alignment task that updates the source spec or records an exact
current-spec citation for the deferral.

Validation rule: for every supplemental phase registered in `skill.md`,
`spec_files` MUST include the registered source file(s) for the active phase.
Unregistered design docs, snapshot files, skill source files, `AGENTS.md`, and
`CLAUDE.md` MUST NOT be added to `spec_files`.

Validation rule: every Non-Goal in the registered active phase source MUST be
represented either by a `forbidden_scope` inference note on the relevant task
family or by a phase-specific verification addendum. A task that implements a
phase non-goal cannot be marked passing unless the source spec was updated and
the ledger was reconciled through the plan path.

Validation rule: a task with non-empty `acceptance_scenarios` MUST include at
least one behavioral, subprocess, harness, or integration verification command
for each scenario. Pure parser/helper/unit tests may supplement but cannot be
the only evidence for a user-facing runtime workflow.

Validation rule: a runtime, startup, CLI, session, adapter, provider, or
extension claim MUST list `production_call_sites`. If the implementation has no
production call site yet, set `substrate_only = true`, keep acceptance scenarios
open, and create or retain a later vertical-slice task.

## Durable Evidence Contract

The ledger is mutable but Git-tracked coordination state. Every successful task
commit MUST include parseable footers:

```text
Opi-Task: <id>
Opi-DoD-SHA256: <sha256 of definition_of_done>
Opi-Verification: <tier>; <short command/result summary>
Opi-Evaluator: <not-required | passed>
```

These values are copied into `tasks[].evidence` after the task commit. The
resulting canonical ledger is then committed separately. Git footers remain the
authoritative recovery source if a checkpoint is lost or conflicted.

Tasks with non-empty `acceptance_scenarios` also include:

```text
Opi-Acceptance: <scenario ids>; <command/test/call-site evidence summary>
```

## Atomic Write Protocol

1. Read the target as strict UTF-8 and record its SHA-256 for optimistic
   concurrency. Never use the Windows PowerShell 5.1 default text encoding.
2. Serialize full JSON with a structured writer (not shell echo/string concat).
3. Validate the candidate before replacement: schema v2, strict BOM-less UTF-8,
   valid JSON, at most 16 MiB total, at most 65,536 characters in any string,
   no known repeated UTF-8/GB2312 mojibake markers, and no forbidden sensitive
   property names, bearer credentials, or private-key material.
4. Write to `.opi-impl-state.json.tmp` in repo root and flush it. Fsync the
   parent directory when the platform exposes that operation.
5. Recheck the target SHA-256. If it changed, preserve both files and stop so
   the caller can re-read the newer ledger.
6. Atomically rename `.opi-impl-state.json.tmp` over `.opi-impl-state.json`.
7. On failure, leave the previous ledger intact and print the tmp path for
   inspection.

On Windows, use
`.claude/skills/opi-implement/scripts/ledger-guard.ps1` for candidate validation
and installation. Do not use default `Get-Content`, `Set-Content`, `Out-File`,
or PowerShell `>` redirection for ledger data; Windows PowerShell 5.1 otherwise
decodes BOM-less UTF-8 through the system ANSI code page. Recovery writes pass
`-BackupPath` to retain the corrupt target as the atomic replacement backup;
normal checkpoint writes use the guard's transient replacement backup.

**Write boundaries** (the only times the ledger is written):
- End of Phase B (user confirms): mark `in_progress`, record `start_commit`
- Each attempt boundary: record start, failing gate, touched files
- Failure decision gate: mark `blocked`, extend cap, or record handoff
- End of Phase E: mark `passing`, record commit + evidence
- Reinit after task-graph review gate confirmed

**Checkpoint boundaries:**
- Phase B and failed-attempt writes may remain dirty and are not committed as
  standalone progress noise.
- A successful task first commits task-owned files, then records that commit SHA
  and commits only the canonical ledger as
  `chore(opi-implement): checkpoint task <id> ledger`.
- Blocked handoffs, phase-exit updates, reviewed graph reconciliation, and
  phase archival are durable boundaries and must checkpoint the canonical
  ledger. A durable pre-archive checkpoint is required before pruning;
  phase archival then checkpoints the phase snapshot together with the
  compacted canonical ledger.
- Temporary, draft, candidate, backup, and corrupt ledger files are never
  tracked.

Before worktree removal, the canonical ledger must be clean, no temp candidate
may remain, and every required checkpoint must be contained in the destination
branch. Resolve a ledger merge conflict by plan-path reconciliation against
both branches' `Opi-*` evidence, never by accepting one side wholesale.

## v1 → v2 Migration

Reinit MUST run this migration before any reconciliation when it loads a
ledger with `schema_version < 2`. The migration is applied to a draft copy;
nothing is overwritten until the task-graph review gate confirms.

Per-task rules:

- `definition_source`: compute by re-parsing the spec roadmap row.
  - Spec row has explicit DoD column AND the v1 DoD string matches → `"verbatim"`.
  - Spec row has no DoD column AND the v1 DoD's SHA-256 matches the
    `Opi-DoD-SHA256` footer of the task's `verified_at_commit` → `"draft-reviewed"`
    (work shipped under that DoD; preserve).
  - Anything else → `"inferred"` AND demote `status` to `failing` AND require
    re-confirmation at the task-graph review gate.
- `replaces`: when the same task ID exists in v1 ledger and current spec but
  title or DoD changed substantively (string distance beyond trivial whitespace
  or punctuation), fill with the v1 title. Else `null`.
- `baseline_dirty_files`: always `[]` at migration time; populated fresh at the
  next Phase B.
- `task_owned_paths`: derive from `tasks[].crate` per the rules in the field
  definition (e.g. `crate = "opi-agent"` → `["crates/opi-agent/**", "Cargo.toml"]`).
- `acceptance_scenarios`: set to `[]` for legacy tasks, then require reinit
  task-graph review to populate scenarios for any current source-spec criteria
  the task claims to close.
- `production_call_sites`: set to `[]` for legacy tasks; task-graph review must
  populate it before runtime/startup/CLI/session/adapter/provider claims can be
  executable.
- `substrate_only`: set to `false` by default; review may set `true` for
  helper/parser/protocol/bridge tasks that intentionally do not close a product
  acceptance scenario.

Top-level field rules:

- `spec_sha256` (v1 string) → `spec_files` = `["docs/opi-spec.md"]` AND
  `spec_files_sha256` = `{"docs/opi-spec.md": <existing-hash>}`. Delete the
  v1 `spec_sha256` key.
- `phase{N}_summary` (v1 informal top-level array) → `phase_exit["<N>"].task_summary`.
  Delete the v1 top-level keys.
- `phase{N}_snapshot` (v1 informal top-level string) → `phase_exit["<N>"].snapshot_path`.
  Delete the v1 top-level keys.

After successful migration AND task-graph review gate confirmation, write
`schema_version: 2` via the atomic write protocol.

## verify_runs migration

A ledger carrying the legacy single `init_verification` object is migrated on
first run by pushing one `verify_runs` entry: `stage = "plan"`, `task_id = null`,
`criterion_id = null`, and the legacy `wf_ref` / `folded_count` / `flagged_count`
/ `rejected_count` / `ran_at` copied verbatim. **The legacy `mode` field
("cheap"|"deep") is dropped** — cheap-vs-deep is no longer recorded at the
ledger level after the auto-deep unification; `wf_ref` is the only surviving
distinguisher. (Consumer-visible break; CHANGELOG'd.)

## Interrupt Recovery

On invocation, if a task has `status = in_progress` AND `verified_at_commit = null`:

**No task-owned dirty files beyond `baseline_dirty_files`:** Prompt:
> "Task X was marked `in_progress` but no commit was recorded. Was the prior
> session interrupted? Reset to `failing` and retry, or investigate first?"

**Task-owned dirty files present:** MUST NOT reset/restore/clean/discard. Print:
- `start_commit`
- `baseline_dirty_files`
- `git status --short`
- Task-owned files changed since `start_commit`
- Last failing gate + reproduction commands

Offer only: continue investigation, mark blocked with text, or drop to manual.
