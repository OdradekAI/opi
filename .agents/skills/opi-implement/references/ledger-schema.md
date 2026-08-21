# Ledger Schema Reference

Path: `.opi-impl-state.json` at repository root. Git-tracked canonical ledger.
Atomic writes use an ignored `.opi-impl-state.json.tmp` plus rename.

## Schema

```json
{
  "schema_version": 2,
  "spec_files": [
    "docs/opi-spec.md",
    "docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md"
  ],
  "spec_files_sha256": {
    "docs/opi-spec.md": "<hash at last init/reinit>",
    "docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md": "<hash at last init/reinit>"
  },
  "current_phase": 17,
  "task_graph_confirmed_at": "2026-08-12T13:10:55+08:00",
  "tasks": [
    {
      "id": "17.5",
      "phase": 17,
      "title": "Wire the Reference Product to dispatchable provider routes",
      "crate": "workspace",
      "definition_of_done": "The Reference Product dispatches registered provider routes and returns owning typed failures before model HTTP dispatch.",
      "definition_source": "inferred",
      "replaces": null,
      "status": "failing",
      "depends_on": ["17.2"],
      "inference_notes": [
        {
          "field": "definition_of_done",
          "reason": "Expanded the registered delivery outcome into an observable Reference Product path.",
          "source": "docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md#Acceptance scenarios and verification"
        }
      ],
      "tier": "workspace",
      "commit_type": "feat",
      "parallelize": [],
      "evaluator_required": false,
      "verification": {
        "library_gates": [
          "cargo test -p opi-coding-agent --test phase17_provider_runtime",
          "cargo clippy --workspace --all-targets -- -D warnings",
          "RUSTDOCFLAGS=\"-D warnings\" cargo doc --workspace --no-deps"
        ],
        "behavioral_tests": ["crates/opi-coding-agent/tests/phase17_provider_runtime.rs"],
        "snapshot_tests": [],
        "smoke_addendum": null
      },
      "acceptance_scenarios": [
        {
          "id": "P17-A02",
          "source": "docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md#Acceptance scenarios and verification",
          "scenario": "An invalid registered provider route returns its owning typed failure before model HTTP dispatch.",
          "verification": [
            "cargo test -p opi-coding-agent --test phase17_provider_runtime phase17_route_and_auth_failures_do_not_dispatch_model_http"
          ],
          "production_call_sites": [
            "opi_coding_agent::CodingHarness::prompt",
            "opi_ai::ProviderCollection::prepare_call"
          ],
          "status": "open"
        }
      ],
      "production_call_sites": [
        "opi_coding_agent::CodingHarness::prompt",
        "opi_ai::ProviderCollection::prepare_call"
      ],
      "substrate_only": false,
      "iteration_count": 0,
      "max_iterations": 5,
      "start_commit": null,
      "baseline_dirty_files": [],
      "task_owned_paths": ["crates/opi-coding-agent/**", "Cargo.toml"],
      "verified_at_commit": null,
      "evidence": null,
      "blocker": null,
      "session_notes": []
    }
  ],
  "phase_exit": {
    "16": {
      "completed_at": "2026-04-12T18:00:00Z",
      "exit_criteria_met": true,
      "evaluator_summary": "all registered Phase 16 exit criteria met; see snapshot",
      "snapshot_path": "docs/snapshots/phase16/opi-impl-state.json",
      "task_summary": [
        { "id": "16.1", "title": "archived task", "status": "passing", "verified_at_commit": "4d9c64..." }
      ]
    }
  }
}
```

## Field Semantics

| Field | Type | Mutability | Notes |
|---|---|---|---|
| `schema_version` | int | reinit-only | Current live value `2`. v2 adds `task_owned_paths`, `definition_source`, `replaces`, `baseline_dirty_files`, `spec_files`, `spec_files_sha256`, `phase_exit[N].snapshot_path`, `phase_exit[N].task_summary`, dotted sub-task IDs, and open-string `crate` values. Live operations accept v2 only; frozen v1 snapshots remain audit-readable historical evidence. |
| `spec_files` | array | const-on-init, reinit-editable | Normative source paths whose drift triggers the plan path. A fresh active graph includes `docs/opi-spec.md` plus exactly the reviewed Phase delivery source files registered in `SKILL.md`. The parent specification is not itself parsed as a roadmap. Adding or removing a path requires a plan-path sync. |
| `spec_files_sha256` | object | reinit-only | Map of file path → its CRLF-normalized SHA-256 (replace `\r\n` with `\n` before hashing) at last init/reinit. The live root `.opi-impl-state.json` is pinned to the current spec by `crates/opi-coding-agent/tests/spec_ledger.rs`; phase-exit snapshots under `docs/snapshots/phaseN/` are historical and are NOT re-synced. Each entry is checked independently; any mismatch triggers the spec-alignment guard. |
| `verify_runs` | array/null | plan+exec+phase-exit | Active-phase verify history. Each entry: `{ stage ("plan"|"exec"|"phase-exit"), wf_ref (string/null — null only when a supported plan fallback has no Workflow id), folded_count, flagged_count, rejected_count, ran_at, task_id (string for risk-gated exec; null for plan/phase-exit), criterion_id (string for phase-exit; null for plan/exec) }`. Tasks with `evaluator_required = false` create no exec entry. Additive/optional within a phase; after a durable pre-archive ledger checkpoint, archive compaction resets it to `[]`. The checkpoint remains the recovery source; do not duplicate the array into a generic history artifact. Does NOT affect `schema_version`. |
| `session_notes` | array/null | plan+runtime+archive | Active-phase root coordination notes. This is distinct from per-task `tasks[].session_notes`. After a durable pre-archive ledger checkpoint, archive compaction resets it to `[]`; archived-phase notes do not accumulate in the live root ledger and remain recoverable from the checkpoint. |
| `current_phase` | int | auto | Phase represented by the active `tasks` array. After archive compaction leaves `tasks = []`, retain the most recently archived phase until the next registered Phase is admitted. |
| `task_graph_confirmed_at` | timestamp/null | plan/reinit | Time of the most recent human graph confirmation. Preserve it in the Phase snapshot; replace it only when a new or reconciled graph is confirmed. |
| `tasks[].id` | string | const | Stable task ID derived from the registered Phase delivery source. Pattern: `^\d+\.\d+(\.\d+)?$`. A third component denotes a reviewed split and uses `parent_spec_row` when the source item has a stable identifier. |
| `tasks[].phase` | int | const | Registered Phase number. |
| `tasks[].title` | string | const | Registered source title or a review-confirmed split title. |
| `tasks[].crate` | string | const | One of opi's six workspace crates (`opi-ai`, `opi-agent`, `opi-coding-agent`, `opi-protocol`, `opi-sandbox`, `opi-tui`), `workspace`, or any free-string packaging identifier (e.g. `examples`, `package-template`) when the reviewed source uses an open identifier. The review gate warns for unknown values but does not refuse solely on that basis. |
| `tasks[].parent_spec_row` | string/null | const | Stable source item ID when a bundled delivery item is split into independently demonstrable tasks. `null` for direct tasks and for reviewed sources without row identifiers. Retained for v2 and snapshot compatibility; it does not imply a parent-spec roadmap table. |
| `tasks[].definition_of_done` | string | const | Observable completion contract from the registered source or confirmed task-graph expansion; provenance is declared by `definition_source`. |
| `tasks[].definition_source` | enum | const | `verbatim`, `inferred`, or `draft-reviewed`; inferred values require review gate confirmation |
| `tasks[].replaces` | string/null | const | Prior task title/meaning superseded during reinit, when the same task ID was repurposed by spec changes |
| `tasks[].status` | enum | runtime | `failing`/`in_progress`/`passing`/`blocked`/`archived` |
| `tasks[].depends_on` | array | const | Task IDs that must be `passing` |
| `tasks[].inference_notes` | array | const | Reasons for inferred fields. Phase non-goal guards use `field = "forbidden_scope"` with an exact source heading. Plan extraction may use `field` ∈ {`deferred`,`split`,`residual`} with `reason` packed as `"<verb>: trigger=<clause|null>"` (a `null` trigger requires a human decision before P.4 confirmation). Inferred placement or public-test-seam choices also record their rationale here. Minimum-change admission standardizes four additional `field` values without changing the note shape or schema version: `reuse_search`, `placement`, `surface_necessity`, and `simplification_ceiling`. |
| `tasks[].tier` | enum | const | `documentation`/`workspace`/`library`/`cli-tool`/`cli-runtime`/`tui` |
| `tasks[].commit_type` | enum | const | `feat`/`fix`/`docs`/`refactor`/`test`/`chore`/`perf` |
| `tasks[].parallelize` | array | const | Sub-unit names for parallel dispatch |
| `tasks[].evaluator_required` | bool | const | Static risk flag |
| `tasks[].verification` | object | const | Tier-specific gate spec |
| `tasks[].acceptance_scenarios` | array | const-on-init, reinit-editable | Product/user-path scenarios owned by this task. Required when the task closes a source-spec goal, success criterion, exit criterion, or workflow. Each scenario has `id`, `source`, `scenario`, `verification`, `production_call_sites`, and runtime `status` (`open`, `met`, or `deferred-by-updated-design`). `scenario` answers what can be demonstrated when the task is complete; `source` cites the reviewed criterion. Component/substrate tasks may use `[]`, but then they cannot close a product acceptance criterion. |
| `tasks[].production_call_sites` | array | const-on-init, append-only during Phase C | Production entry points that must call or exercise this task's implementation before the task can close runtime acceptance. Examples: CLI subcommand handler, harness startup, agent loop hook wrapper, session persistence path. Tests-only helpers do not count. |
| `tasks[].substrate_only` | bool | const-on-init, reinit-editable | `true` means the task intentionally implements a helper/parser/protocol/bridge slice and cannot by itself close product acceptance scenarios. A later vertical-slice task must consume it through a production call site. |
| `tasks[].iteration_count` | int | runtime | Attempts since `in_progress` |
| `tasks[].max_iterations` | int | user-gated runtime | Default 5. `--extend-cap <N>` sets an absolute new cap and requires `N` to be greater than the current value; graph approval does not authorize this mutation. |
| `tasks[].start_commit` | string/null | runtime | HEAD when Phase B confirms |
| `tasks[].baseline_dirty_files` | array | runtime | Files already dirty at Phase B start; used to avoid cleaning or staging unrelated user work |
| `tasks[].task_owned_paths` | array | const-at-Phase-B, append-only during Phase C | Glob patterns the task is allowed to modify. Default derived from `crate` at init/reinit time (e.g. `crate = "opi-agent"` → `["crates/opi-agent/**", "Cargo.toml"]`). Phase C MAY append entries when implementation requires touching outside-prefix files; each append MUST add an `inference_notes` entry with `field = "task_owned_paths"` and a `reason`, written via the atomic ledger write. |
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

Validation rule: minimum-change notes retain `{ field, reason, source }`.
`field = "reuse_search"` requires `searched=`, `reused=`, and `gap=` clauses.
`field = "placement"` requires `target=`, `existing_home=`, and
`cannot_fit_fully=` clauses. `target` MUST be one of `core`,
`reference-product`, `extension`, `plugin`, `package`,
`independent-companion`, or `assurance`; `assurance` is valid only when the task
adds no runtime capability or ownership seam. `field = "surface_necessity"` requires
`public_api=`, `config=`, `state=`, and `dependency_edge=` clauses, each set to
`none` or a necessity rationale. `field = "simplification_ceiling"` requires
`ceiling=` and `revisit_when=` clauses; the revisit trigger must
be observable rather than “when needed”. Missing clauses fail task-graph
admission; explicit `none` or `not-applicable` remains valid when justified.

For every new or reconciled plan draft, `field = "simplification_ceiling"`
also requires `simplification_trigger=` with one of `none`, `unused`,
`duplicate`, `superseded`, `delete`, `merge`, `replace`, or
`dependency-substitution`. `none` records an ordinary task; any other value
activates the conditional validation rule below. On every platform, run
`python .claude/skills/opi-implement/scripts/validate-plan.py .opi-impl-state.draft.json`
before confirmation so missing or duplicated notes and unknown or incomplete
declarations on non-archived tasks fail closed. Retained `status = archived`
reconciliation history is not executable and is excluded. The Windows ledger
guard remains responsible for strict UTF-8 validation and atomic installation;
it does not reinterpret plan semantics during ordinary `Validate` or `Install`
operations.

Conditional validation rule: when a task claims an existing surface is
unused, duplicate, or superseded, or proposes deletion, merging, replacement,
or dependency substitution, `field = "reuse_search"` also requires
`production_consumers=` and `nonproduction_consumers=`, while
`field = "simplification_ceiling"` also requires `net_deletion=` and
`residual_glue=`. These are `reason` subclauses, not new ledger fields, so the
schema version and `{ field, reason, source }` note shape remain unchanged.
An explicit `none` must cite verifiable repository evidence and account for
applicable dynamic loading, configuration, wire or persistent formats, and
public API consumers.

Validation rule: a `substrate_only` task with no owned acceptance scenario
MUST appear in the transitive `depends_on` closure of a later task that owns a
sourced acceptance scenario. Otherwise it is orphan work and the graph cannot
be confirmed. Do not add a fake production call site to a substrate task.

Validation rule: the minimum-change trace is admission-only. Existing
schema-v2 ledgers remain readable and already-confirmed no-drift graphs are not
rewritten. After the next init, reconcile, import, or graph edit, every
executable task must satisfy the trace contract. Phase B labels missing
pre-contract answers `legacy-unrecorded` and never synthesizes them.

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

Validation rule: for every Phase registered in `SKILL.md`, `spec_files` MUST
include the registered delivery source file(s) for the active phase plus the
parent `docs/opi-spec.md`.
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
- Each attempt boundary: append outcome and failing-gate information to the
  task session notes
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

## Legacy Ledger Boundary

There is no v1-to-v2 or `init_verification` migration path. Keep existing
historical ledger snapshots unchanged and inspect them through `opi-audit`;
they are evidence, not live inputs. New and reconciled live ledgers use the
v2 fields defined above.

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
