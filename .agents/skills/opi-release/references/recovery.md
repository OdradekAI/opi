# Release recovery

Read only for an incomplete or interrupted release. Reconcile the requested
version, local state and current GitHub/crates.io facts before mutating anything.
A remembered command exit or a state boolean is not remote proof.

| Observed state | Next action |
|---|---|
| Local preparation only | Match the recorded base and exact task-owned diff; continue preparation without demanding a clean tree. Unknown edits require reconciliation. |
| Candidate committed/pushed, no tag | Validate the candidate and its CI/platform runs; resume the public candidate gate, not fresh preflight. |
| Tag exists, draft missing | Verify annotated tag target equals the candidate. With the public operation authorization still applicable, create the draft and recover the retained build/upload stage. A mismatched tag stops. |
| Draft/assets exist | Verify draft state, candidate, complete expected asset names and downloaded digests. Reuse matching bytes; conflicting bytes need explicit replacement review. |
| Some crates published | Query every graph member/version, reconcile verified successes and unknown outcomes, then resume only missing crates after the fresh publication gate. Existing matching versions are recovery facts, not fresh-entry conflicts. |
| Draft already public | Verify registry and downloaded assets. Reconcile completion; do not republish or overwrite. |

Checkpoint each verified transition atomically. If a process died between an
external success and its checkpoint, reconstruct that fact from the exact tag,
release, run and registry identities. Preserve old state/evidence when facts
conflict. A changed candidate, asset set, strategy or publication scope needs
review and renewed authorization for the affected public/irreversible gate.

Before public changes, retain the local delta and fix forward within scope;
revert only exact known paths when the user chooses that operation. After
public Git changes, abandonment requires confirmed draft/tag deletion and a
revert commit, never history rewriting. After crates.io publication, versions
remain immutable; yanking requires explicit confirmation. Prefer a new fixed
version when a published artifact is wrong.

A completed release is reconstructed from public Git/registries. Delete the
temporary state only after final verification, retaining non-empty artifacts
and original failure evidence. Cache maintenance remains a separate action.
