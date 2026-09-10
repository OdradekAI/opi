# Post-evaluation environment cleanup

Read before evaluation preparation to record ownership, and after report
preparation to perform cleanup. This also applies to failed/cancelled runs,
fidelity cases and completed repair checks. Retain reusable materials and
evidence that supports report traceability, verification, diagnosis or
reproduction; remove only disposable resources attributable to this invocation.
Routine cleanup within that scope needs no second blanket confirmation.

## Establish ownership and retention

Before acquisition, builds or execution, record the selected Docker context,
existing full image IDs, tags/digests, containers, volumes/networks and relevant
disk usage. Record the exact private workspace/temp roots, shared cache roots
and evidence locations. During execution, tie created resources to actual
commands, plans, build outputs and owned-resource receipts. Inventory only safe
metadata; Docker inspect/environment output can contain credentials.

A before/after difference alone does not prove ownership on a shared host.
Neither a name prefix, an old timestamp nor `<none>`/dangling status authorizes
deletion. If the baseline is missing, recover ownership from retained creation
records; leave ambiguous resources intact and explain the gap.

| Resource | Retention decision |
|---|---|
| Reusable base/runtime, benchmark environment, runner, proxy, verifier and shared dependency images | Keep the exact identities needed by maintained eval profiles or future runs, including task images reused across evaluations. Being task-specific does not make an image disposable. |
| Shared material/source/package downloads and persistent Cargo caches | Keep. Release this invocation's leases; cache eviction is separate maintenance. |
| Original run trees, seals, failed/unsealed attempts, native inputs/results, captures, delivered patches/files, controls/plans/materials, build/runtime/source identities and frozen executables, command/check logs and reports | Keep under the evidence-root contract, including private evidence and material required for offline regrading. Failed results are evidence, not garbage. |
| Trial-derived images, build intermediates and other non-reusable Docker artifacts | Remove only when owned by this invocation, unused, and unnecessary for evidence, a pending rerun/regrade or supported reproduction. Preserve any unique needed contents first and verify their retained identity. An image digest alone does not preserve image contents. |
| Scratch workspaces, test homes, temporary scripts/download staging, duplicate copies, stopped helper containers, owned volumes/networks | Remove after all writers are gone and required contents have been preserved. A temp directory containing the only trace, patch or failure log remains evidence. |
| Temporary projected credentials, sockets and process/lease residue | Remove this invocation's disposable copies/resources when no longer in use. Preserve credential references and cleanup outcomes, never credential values or user-owned key/config stores. |

Reusable means useful across the maintained evaluation workflow, not literally
required in every trial. Retain a per-run resource when it is still an evidence
or reproducibility dependency. A recipe, tag or possible future download is
not proof that deleting the only retained artifact preserves offline capability.
Follow [evidence root](evidence-root.md) for preservation/privacy; cleanup does
not authorize resealing, rewriting paths in frozen claims, or unsafe exports.

## Inspect and remove disposable resources

1. Quiesce owned writers and preserve required container/volume contents before
   destructive removal; then finish container cleanup before deleting images.
   Use Companion cleanup receipts plus live checks; a successful trial exit
   alone does not prove cleanup. Keep any resource required by active work,
   a resumable run, or another user's process.
2. Reinspect Docker images on the recorded context, including untagged images.
   Resolve candidate tags to full immutable IDs, check every alias, container
   reference (including stopped containers), shared profile/material dependency
   and evidence dependency. Revalidate immediately before removal. Remove only
   individually identified disposable images without force; a conflict leaves
   the resource retained with a reason. A run-specific tag on a shared image
   may be removed only by that exact tag, preserving the shared image/aliases.
3. Check other run-created residue: containers and helper services, unused owned
   volumes/networks, disposable builder state, temp homes/worktrees, download
   staging, scratch scripts, sockets and expired owned leases. Apply the same
   ownership, inactivity and evidence checks to each item. Preserve shared
   build caches; clean builder state only when its builder/storage is proven
   exclusive to this invocation. Do not use global `docker system prune`,
   blanket image/builder/volume pruning, `cargo clean`, or whole-cache deletion.
4. Resolve filesystem deletion targets to absolute paths within the exact
   owned disposable roots. Reject escapes and linked ancestors; do not follow
   symlinks/junctions into another tree. On Windows, validate final targets and
   use native PowerShell operations with `-LiteralPath` end-to-end. Keep source
   worktrees with user changes and every preserved evidence root intact.
5. Record each action and actual result. A missing resource after reinspection
   is a no-op; a failed deletion is not success. Continue with independent safe
   candidates, retain ambiguous/dependent items, and report remaining residue
   without expanding scope or adding paid reruns to repair a cleanup failure.

## Verify and report

Keep the cleanup record beside the external operation evidence, outside sealed
trees and any finalized acceptance manifest. Include before/after inventories,
exact resource IDs or private paths, ownership evidence, keep/remove decisions
and reasons, sanitized commands/exits, failures, retained locations and measured
space reclaimed. Shared image layers mean summed image sizes are not reclaimed
bytes; report an actual measurement or `unknown`.

Compare protected file inventories/digests before and after cleanup. Re-run
offline verification/report reconstruction when a referenced material path or
resource was affected, using fresh output paths. Preserve prior nonzero checks
and invalid trials; cleanup must not erase or upgrade them. Verify removed
containers/helpers and disposable resources are absent, while retained image
identities, caches and evidence remain available. A surviving ambiguous resource
is a documented retention decision, not permission to force removal.

Add a cleanup summary and record link to the final report/handoff. State
`completed`, `partial` or `not-applicable` with the inspected scope and reasons;
use `not-applicable` only when no disposable resources were created. Missing
ownership, inaccessible Docker or failed removal leaves explicit partial
cleanup. A read-only verify invocation may inventory/report shared resources,
but cleans only its own scratch and never the supplied evidence or shared Docker
state.
