# Task Source and Completion Proof

Use `scripts/task_completion.py` at the existing B, D and E boundaries. It reads
Git and external input files, writes immutable receipts outside the repository,
and never stages, commits, runs tests or modifies the ledger. Successful proof
does not authorize any of those actions. The ledger byte/atomic-write guard
remains responsible for safe installation after E's semantic checks.

## Baseline and verification input

After B.3, serialize the selected complete task object to an external UTF-8 JSON
file. From the repository root, capture the pre-implementation state:

```text
python .agents/skills/opi-implement/scripts/task_completion.py baseline --task <task.json> --source-design-path <registered-phase-source> --source docs/opi-spec.md --output <external-baseline.json>
```

Repeat `--source` for every other registered source. Keep the emitted raw-byte
SHA-256. The receipt binds the task, full `start_commit`, registered source
bytes, index and tracked/nonignored-untracked working content. It preserves
carried-in work and rejects overlapping task-owned dirty paths. Receipts are
evidence, not a second progress ledger. Preserve their original bytes.

Registered sources remain admission inputs. The existing documentation/alignment
exception in [verification tiers](verification-tiers.md#task-graph-verification-checks)
permits a source edit only when the original frozen task has `tier: "documentation"`,
owns that exact path and explicitly names its update in the DoD. For either
`docs/opi-spec.md` or `docs/opi-spec.zh.md`, both exact paths must be owned and
named in the DoD. The helper checks the tier, exact paths and DoD path mentions;
the existing admission review establishes their meaning and authorization.
A later C.1a ownership append cannot grant this exception. Other source drift
remains an error; this helper never revises the admitted task or reconciles the
ledger/source registry.

Before running the final D verification union, capture its source input:

```text
python .agents/skills/opi-implement/scripts/task_completion.py capture <external-baseline.json> --sha256 <baseline-sha256> --mode worktree --output <external-candidate.json>
```

The worktree mode requires HEAD to remain `start_commit`. Its file inventory
includes additions, modifications and deletions, whether staged, unstaged or
untracked. The staged version is recorded separately from the working version.
Resolve index-only changes that were undone in the working tree before capture;
they cannot be reviewed as the tested working implementation.
The baseline receipt's `sources` map keeps the original source identities.
The candidate's `sources` map records every registered source at D capture,
including an admitted documentation edit. Those final bytes are the verification
input and must remain unchanged through review and completion. Evaluate the
source edit against the frozen task and original authority; its proposed text
does not authorize additional implementation.
An authorized C.1a ownership append uses `capture --task <current-task.json>`;
the original baseline remains fixed, and the append must retain the original
task contract and record its reason under the existing inference-note rule.
The existing Phase C append rule for `production_call_sites` also remains;
completion cannot extend these fields after review. Runtime fields retain
their existing authority rules, including explicit `--extend-cap` for a higher
`max_iterations`; the helper does not authorize those changes.

For `--resume-from-manual`, use `capture --mode commit`. Review the actual task commit
against `start_commit`, require exactly that sole-parent commit, and keep
task-owned paths clean. When entering recovery without an earlier receipt,
`baseline --mode commit` can bind that existing manual commit and current
carried-in state for a new verification run; it does not attest a historical dirty state
or an earlier test execution. Preserve unrelated current work.
For a source changed by that manual commit, the recovery baseline retains its
original `start_commit` blob identity; capture still checks the same admitted
documentation exception before binding the current source bytes.

## Verification facts and planned evidence

Run the required commands against the captured input and retain their original
exits and raw logs. Use an external JSON object with a nonempty `checks` list:

```json
{
  "checks": [
    {
      "command": "<exact original command>",
      "exit_code": 0,
      "stdout": "<absolute raw stdout path>",
      "stderr": "<absolute raw stderr path>",
      "kind": "gate"
    }
  ]
}
```

Use `kind: "behavioral"` for behavior evidence. Retain the existing scenario
and `gate_results` details in this object: public entry, production
consumer/adapter, discriminating assertion, proof class and applicability.
Shared proof may cover several scenarios; do not duplicate a protocol matrix.
The helper requires successful original exits and seals the named log bytes;
it cannot authenticate a caller's recorded command or classify an assertion.
A helper-only or stale-client test cannot close an integration criterion.

A behavioral `cargo test` check also carries a `rust` object, including when
the recorded command invokes it through `opi-cargo-cache.py run`. Quoted paths,
Windows executable paths and the wrapper's `--target=<path>` form do not remove
the execution-proof requirement:

```json
{
  "list_output": "<absolute complete inventory log>",
  "ignored_list_output": "<absolute ignored inventory log>",
  "run_output": "<absolute original execution log>",
  "list_exit_code": 0,
  "ignored_list_exit_code": 0,
  "filter": "<actual filter>",
  "exact": true,
  "selection": "normal",
  "expected_count": 1
}
```

Record the original inventory-command exit codes; `run_output` must identify
the behavioral check's captured stdout. Match options to the actual execution. Omit `filter`, `exact` or
`expected_count` where not applicable. The helper reuses the
[Rust execution checker](../../opi-audit/references/test-execution-check.md);
its single-target limits and evidence boundaries apply. Compile-only and broad
workspace gates remain `gate` checks; they do not substitute for the owned
scenario's behavioral proof. For other runners retain runner-native case
identities and counts with the original evidence.

Create a separate JSON dictionary of the planned exact `Opi-*` footer values
from the [durable evidence contract](ledger-schema.md#durable-evidence-contract).
The DoD hash uses the complete decoded string's UTF-8 bytes without trimming or
newline normalization. `Opi-Evaluator` is the planned `passed` or `not-required`
value; writing it is not itself evidence that evaluation ran.

```text
python .agents/skills/opi-implement/scripts/task_completion.py review <external-candidate.json> --sha256 <candidate-sha256> --evidence <verification.json> --footers <footers.json> --output <external-review.json>
```

This checks source stability since capture and binds the task, source, file
inventory, verification facts, original log hashes and proposed footers into
one D.2 input. It refuses failed checks and invalid Rust execution evidence.
The review receipt exposes `baseline_sources` and final `sources`; its
`source_sha256` is the final `source_design_path` identity from D capture.
The bound baseline receipt preserves the original authority independently.
When proof changes, create a new review receipt; never relabel an old review
result with a new digest. Source drift requires a new candidate capture and
rerunning affected verification; do not recapture after testing and claim the
new input was the one tested.

## Review and completion

For a required D.2 Workflow pass the complete review JSON as `candidate`, its
exact task as `task`, its `source_design_path` as `sourceDesignPath`, and the
emitted review hash as `review_sha256`. The Workflow returns the two receipt
identities and the complete actual `reviewed_input` with its deterministic
findings. E compares that input with the original receipt, so a copied digest
cannot disguise a truncated or changed review object. Save that result with the actual
runtime `wf_ref` as an additional top-level field. No fabricated run ID,
exception result or missing run can replace completed required evaluation.

After review, verify that its source and proof still match:

```text
python .agents/skills/opi-implement/scripts/task_completion.py verify <external-review.json> --sha256 <review-sha256> --stage worktree --workflow-result <workflow-result.json>
```

Omit `--workflow-result` only when the frozen task has
`evaluator_required=false`. A required result must match both receipt hashes,
include the identical complete review input and consistent valid report/lists,
have no must-fix finding, and identify the actual runtime run. Receipt checks
do not authenticate the runtime; the caller retains its original execution
proof. `REVIEW-INCOMPLETE` stops D/E regardless of any empty partial list.

After authorized staging, immediately before E.1:

```text
python .agents/skills/opi-implement/scripts/task_completion.py verify <external-review.json> --sha256 <review-sha256> --stage index --workflow-result <workflow-result.json>
```

The entire index must contain exactly the reviewed task delta, with no
unrelated staged content or canonical ledger. After the authorized task commit,
or when recovering the reviewed manual commit, verify its parent, actual
contents and parsed Git footers:

```text
python .agents/skills/opi-implement/scripts/task_completion.py verify <external-review.json> --sha256 <review-sha256> --stage commit --workflow-result <workflow-result.json>
```

Before E.2 installs a passing ledger candidate, verify that candidate's task
identity, DoD, delivery commit and evidence mirror against the actual commit:

```text
python .agents/skills/opi-implement/scripts/task_completion.py verify <external-review.json> --sha256 <review-sha256> --stage checkpoint --workflow-result <workflow-result.json> --ledger <proposed-ledger.json>
```

Apply the existing atomic ledger write and separate checkpoint commit only
after this succeeds. The check is about the selected task's completion; it is
not a replacement for whole-ledger safety/graph validation. On mismatch keep
the current state and original proof, resolve the identified gap, and repeat
only affected checks. Historical mistakes are corrected by attributed
append-only notes, never by rewriting original footers or inventing old runs.

## Proof boundary

The helper compares observations without locking concurrent editors. It covers
tracked and nonignored-untracked source, with the canonical ledger handled
separately. It rejects changed symlinks, unsupported Git modes and external
Git clean filters for index/commit comparison. Ignored inputs, external
dependencies, client/material identity, platform conditions, assertion quality
and actual command provenance need their declared proof. A matching receipt
establishes content association, not product acceptance.
