# Cargo cache lifecycle

This reference owns repository Cargo cache lifecycle rules. It is shared
reference, not an invocable skill, build gate, or progress ledger. Existing
workflows retain admission, verification, evidence and artifact ownership.

## Build and inspect

Keep incremental compilation and the configured persistent `CARGO_TARGET_DIR`.
When unset, select the persistent worktree/toolchain target with
`python scripts/opi-cargo-cache.py resolve`. Use that exact path as `<target>`
below; never create a per-session target or use `cargo clean` for cleanup.

Before a workflow's first Cargo build, inspect its selected target once:

```text
python scripts/opi-cargo-cache.py status --target <target> --quick --json
```

Quick status reports `disk_free_bytes`, `candidate_files`, `recorded_files` and
`observation_status` with physical coordination metadata, without a recursive
size scan. Use full `status --json` for a root inventory at Phase archive or explicit
maintenance, including unmarked directories; their presence establishes no
ownership. Avoid a full scan after every task. Status, plan-only and docs-only
invocations do not start Cargo builds or maintenance.

Smoke wrappers and the eval runtime own build orchestration. For supported
direct build/check/clippy/test commands, observe exact Cargo outputs with:

```text
python scripts/opi-cargo-cache.py run --target <target> -- test -p <crate> --test <name>
```

Arguments after `--` begin with the Cargo subcommand, without a `cargo` prefix.
Preserve the original command, stdout/stderr and exit as verification evidence;
cache metadata never substitutes for executed-test or source-applicability proof.
For other Cargo writers, coordinate the selected target with `lease start
--target <target> --pid <owner-pid>` and release only that invocation's lease
using `lease end --target <target> --pid <owner-pid>` in a finally/trap path.
These are subcommands of `python scripts/opi-cargo-cache.py`. Configured and
unmarked targets also need coordination; observing one does not claim all bytes.

## Observed-output supersession

Only positively observed regular single-link files directly in Cargo `deps`
directories, superseded by a successful run in the same observation scope, can
become candidates. Scoped package, test, feature,
profile, toolchain and target variants retain their own live observations.
Failed/interrupted runs preserve prior successful observations and their own
outputs, even after a later successful run. Incomplete output inspection also
preserves prior observations and protects every reported admitted path, including
paths whose current identity could not be read. Build completion and test-command
success are separate facts.

This branch protects unobserved/legacy files, inferred filename siblings,
incremental directories, public binaries, build-script outputs and Cargo internal
metadata. Its zero candidate count is not a zero reclaimable-capacity claim.
Old timestamps, a removed test target,
an unmarked directory or a missing current filename match do not prove garbage.
Changes to source/build inputs invalidate affected proof and observations;
they never repair task state or justify a passing checkpoint.

Source-freshness reasoning must bind the exact Cargo unit, its source/dependency
inputs and selected configuration. A changed function does not identify an `.o`
file to delete, and changed workspace source does not invalidate an entire target
root or an independently sealed export. There is no legacy final-artifact
freshness analyzer in this helper; such reasoning is not a supported CLI branch.

## Verified rebuildable incremental eviction

Eviction deliberately trades reuse for space. It does not claim the payload is
stale or unusable. Known compiler-cache ownership with unknown freshness,
including legacy sessions without observation receipts, can qualify. Keep
incremental compilation enabled and persistent targets in use; later compilation
may rebuild an evicted session and take longer.

The planner uses offline/locked metadata with all workspace features and no
platform filter. It compares package identities across the current resolved
dependency graph, excluding target names shared with any unselected package,
including external dependencies. This graph does not identify removed or inactive
historical dependencies. Review scope accordingly: names delimit scope, not
freshness or sufficient ownership proof. Target ownership and
finalized rustc session structure, recognizable cache headers and an existing
regular session lock must also validate. Select the complete `.bin`/`.o` payload
set for each admitted session; preserve metadata, locks and all other contents.
Working sessions, unknown layouts, explicit protections and uncertain ownership
stay excluded. This is deliberate payload eviction, not rustc's normal GC or a
promise that a partially retained session remains fully reusable.

Prepare an external protection manifest with the actual evidence/replay pins:

```json
{
  "schema": 1,
  "files": ["<absolute-file-path>"],
  "trees": ["<absolute-tree-path>"],
  "bindings": [{"path": "<absolute-file-path>", "sha256": "<sha256>"}]
}
```

Use empty arrays where appropriate after reviewing evidence ownership. Bindings
are exact content checks; files and trees exclude those paths from eviction.
The plan binds the protection file and selected workspace/target identities.

```text
python scripts/opi-cargo-cache.py plan-eviction --target <target> --workspace <workspace> --package opi-ai --package opi-agent --protection <protection.json> --output <plan.json>
```

Repeat `--package` for each selected package. Keep protection and plan files
outside the target. `--output` must name a new, nonexistent file. An interrupted
write is not an applicable plan; use a new path and regenerate for review.
Review selected/excluded sessions, unresolved leases, exact
paths and identities, and allocation/link accounting. Count allocation as
reclaimable only when all hardlinks to that file belong to the deletion set;
logical file sizes or directory totals alone are not guaranteed freed space.

## Evidence protection

Preserve raw command evidence, failed runs, frozen eval executables/materials
and release artifacts under their owning contracts. Shared caches are outside
eval retention/cleanup. Ordinary log path mentions and basename matches are
investigation leads, not permanent whole-unit/root pins. Retain original sealed
bytes and real replay/regrade dependencies. A rebuild recipe is not a replacement
for required original bytes; an evidence-preserving copy/archive must retain
identities and any required original-path restoration without rewriting seals.
Protect an export's actual evidence dependencies rather than assuming every
byte of its shared target is immutable. Resolve ambiguous dependencies before
selection. Never infer deletion authority from a retention count, successful
test, task checkpoint or Phase archive.

## Maintenance cost and recovery

Keep routine build-workflow checks to quick status. When checkpoint changes or
capacity pressure warrant inspection, select changed workspace packages and
their reverse dependencies from the current graph; toolchain, configuration and
feature changes may broaden that scope. This is scope selection, not automatic
source-freshness analysis or a persistent incremental index in the helper.

After establishing a historical baseline, reuse inventories, protections and
reviewed plans while their bindings remain valid. A task ending or a new chat
does not by itself require another full inventory. Refresh the affected inputs
when they drift; mandatory apply-time revalidation still covers every selected
file. Stabilize protection inputs before a large plan/apply sequence, and retain
the bound rule bytes before updating rules after execution.

On Windows, the helper queries identities/allocation with attribute-only handles
and reads headers through one eight-worker pool per plan/apply invocation, in
batches of at most 256. Reads finish and handles close before the session lock
is released. Use this bounded path instead of adding a separate whole-target
prefetch. It preserves format, identity, membership, protection and lock checks;
it does not make selected-file validation optional.

Report planning and application (including preflight) durations separately,
with per-target completion, deleted-file counts and free space before/after.
Distinguish candidate allocation estimates from the observed net free-space
change, which can include other disk activity. Successful exact-plan execution
does not require another recursive inventory merely to repeat the count.
After interruption or failure, reconcile the exact listed paths before claiming
zero or partial deletion. A confirmed zero-deletion attempt may reuse an unchanged
plan after ownership/quiescence recovery and revalidation; actual partial deletion
requires a newly reviewed plan for the remaining eligible files. Retain failure
receipts rather than replaying a partially applied plan.

## Separate explicit maintenance

At task/checkpoint or Phase boundaries, or under actual capacity pressure,
propose the appropriate branch when inspection is warranted. These events
authorize proposals only. Keep plans outside sealed evidence and
the implementation ledger, and outside the selected target directory:

```text
python scripts/opi-cargo-cache.py plan --target <target> --output <plan.json>
```

Review the exact candidate paths, file identities and sizes against this
reference's protection rules. A plan enumerates candidates, not every protected
file; protected reasons are policy, not an exhaustive protected-file inventory.
An empty plan, protected legacy bytes or non-actionable cache state does not
block task passing or mutate `.opi-impl-state.json`. Report a cache failure
separately; a failed required Cargo command still fails its normal gate.

Only an explicit maintenance request authorizes applying the reviewed plan:

```text
python scripts/opi-cargo-cache.py apply --plan <plan.json> --sha256 <digest> --quiescent
```

Carry forward execution authorization within its reviewed scope. A changed
digest requires renewed review, not by itself another permission request; ask
when the authorized scope expands or a necessary user decision changes. A
plan-only or no-deletion instruction remains effective until execution is requested.

The digest binds the reviewed plan bytes. `--quiescent` asserts that unmanaged
Cargo/IDE/other writers have stopped; leases alone cannot establish that.
Application rechecks exact file/state identities, linked paths, ownership and
coordinated writer inactivity. Same-host verifiable ownership is required;
unknown foreign/legacy leases and changed plans/files block deletion. Resolve
the uncertainty or regenerate and review a fresh plan; never force it through.
There is no blanket age/TTL eviction, and legacy `prune --execute` cannot bypass
these requirements. Workflows observe and propose; they never apply automatically.

The existing apply entry dispatches the plan kind. Incremental eviction also
requires nonblocking native session locks and complete revalidation of payload
membership, protections, workspace/target identity and file identities before
the first unlink. New working sessions, replaced roots, links/reparse points,
unresolved leases or any drift reject application; regenerate and review after
resolving the cause. Only exact listed payload files are deleted.

Windows and WSL coordinate through an atomic directory-creation guard on the
common filesystem. A busy/stale guard fails closed and is never reclaimed
automatically. Interrupted writes retain unresolved writer leases even after
the parent exits. An operator must verify that all descendants stopped, then
explicitly end the exact same-host PID lease; another namespace's PID is not
proof of inactivity. Resolve stale guards only after verifying writer inactivity
and ownership, never as routine workflow cleanup.

## Workflow hooks

| Event | Cache action |
|---|---|
| Implement A, before runtime boot | Quick selected-target check |
| Implement D; audit/remediate/slim verification | Existing gates with coordinated observation |
| Implement E.3, after completed checkpoint | Optional plan on candidates/pressure; separate report |
| Implement F, before archive | One full physical inventory; preserve ledger/archive authority |
| Task/Phase boundary or actual capacity pressure | Propose supersession or incremental eviction; no automatic apply |
| Eval preparation/cleanup | Runtime owns builds and freezes binaries; release owned leases, retain caches |
| Release preparation/build/finalization | Coordinate local writers; retain versioned artifacts and failure evidence |

Audit independent context may receive only physical cache metadata needed for
build coordination. Keep implementation ledger, task history and earlier
assessment conclusions outside that context; cache inspection never opens a
history channel or expands audit authority.
