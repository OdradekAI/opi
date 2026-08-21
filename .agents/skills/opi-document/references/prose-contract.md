# Prose contract

Apply this reference only to the requested human-facing prose or agent-facing
instruction scope. Write
enough to preserve the current contract, then remove repetition, authoring
history, and narration that does not help a reader use or maintain Opi.

## Scope and exclusions

In scope are current-product documentation, project-local skills and agent
instructions, Rust comments and rustdoc, prompts, diagnostics, CLI/TUI strings,
and other model-visible or user-visible prose selected by the invocation.

Exclude released changelog sections, `docs/snapshots/`, generated artifacts,
fixtures, recorded runtime output, and outward or inward evidence whose original
wording is itself the historical record. Edit a generator or owning source
instead of its derivative. A targeted invocation never becomes a repository-wide
prose audit merely because an analogous passage may exist elsewhere.

## Preserve the complete proposition

Before editing, identify every relevant actor, action, condition, timing or
ordering rule, modal strength, negative guarantee, exception, ownership or side
effect, failure mode, and consequence. Preserve each factual clause unless the
owning source proves that it is obsolete. Concision alone does not justify
weakening or deleting a proposition.

Classify each candidate explicitly:

- `keep`: the current wording is accurate, complete, and appropriately placed;
- `add`: code or authoritative documentation leaves a required contract unstated;
- `trim`: remove repetition or decoration while preserving every proposition;
- `restore`: return a contract fact lost by an earlier edit;
- `restructure`: keep the facts but move detail to its owning document or section;
- `defer`: semantic or source authority is unresolved and the edit must wait.

Current-state surfaces describe what Opi does, requires, rejects, or guarantees
at the current revision. Replace PR, review, task, phase, or authoring-session
vantage with current behavior. Keep resolvable spec, ADR, issue, standard, and
counterfactual regression references when they provide durable authority or
rationale.

## Owner-first editing

Update the narrowest authoritative owner before derivative prose: source and
behavior before README claims, source rustdoc before generated API material,
and generator inputs before generated catalogs. Keep one home for extended
rationale; repeat only the local contract facts needed for safe use.

Treat prompts, diagnostics, and model-visible or user-visible strings as
behavior. Wording changes on those surfaces require the owning snapshot,
subprocess, integration, or runtime evidence when one exists. Do not silently
rewrite such text as editorial cleanup.

Semantic review remains human or model judgment. The documentation checker may
verify that this reference and its routes remain present.
Mechanical checks do not prove semantic quality, factual completeness, or
translation fidelity.

Report the inspected scope, edits made, deliberate keeps, deferred cases, and
the exact verification commands run.
