# Remediation Plan Template

Output format for `docs/snapshots/phase<N>/remediation-plan.md`.

## File structure

```markdown
# Phase <N> Remediation Plan

**Date**: <YYYY-MM-DD>
**Audit sources**: <list of audit files consumed>
**Commit range**: `<first>..<last>`
**Design spec**: <spec file path(s)>

---

## Audit cross-reference summary

| Cluster | Theme | Auditors | Consensus | Unified severity | Verification |
|---------|-------|----------|-----------|------------------|-------------|
| ... | ... | ... | ... | ... | Confirmed / Partially / Refuted |

## Decision record

| ID | Finding cluster(s) | Decision | Rationale | Decided by |
|----|-------------------|----------|-----------|------------|
| D1 | C1, C3 | ... | ... | auto / user |

## Remediation layers

### Layer 1: <crate-name> (substrate)

**Verification**:

    cargo fmt --all
    cargo clippy -p <crate> --all-targets -- -D warnings
    cargo test -p <crate> --all-targets

#### Fix 1.1: <short title>

- **Audit source**: <auditor IDs and finding IDs>
- **Cluster**: C<N>
- **Decision**: D<N>
- **Verification status**: Confirmed
- **File(s)**: `<path>` ~L<range>
- **Change**: <description of what to modify>
- **Test plan**: <new test / modified test / existing test name>

#### Fix 1.2: ...

### Layer 2: <crate-name> (product)

...

### Layer N: Documentation

...

## Final verification

    cargo test --workspace --all-targets
    cargo test --workspace --doc

## Scope exclusions

Findings that were refuted, downgraded to Info, or deferred:

| Finding | Status | Reason |
|---------|--------|--------|
| ... | Refuted | <evidence> |
| ... | Deferred to Phase <M> | <rationale> |
```

## Required fields per fix item

Every fix item in the plan MUST include these fields:

| Field | Description |
|---|---|
| Audit source | Which auditor(s) and finding ID(s) identified this issue |
| Cluster | The cross-reference cluster ID (omit if single-source) |
| Decision | The decision record ID that resolved the fix direction |
| Verification status | Confirmed / Partially confirmed (from Phase C) |
| File(s) | Exact file path(s) and approximate line numbers |
| Change | What to modify -- specific enough that a developer can act on it |
| Test plan | How the fix will be verified (new test name, or existing test) |

## Decision record format

Each decision in the table must specify:

| Field | Description |
|---|---|
| ID | Sequential identifier (D1, D2, ...) |
| Finding cluster(s) | Which cluster(s) this decision addresses |
| Decision | The chosen approach, stated concisely |
| Rationale | Why this approach was chosen over alternatives |
| Decided by | `auto` (clear fix direction) or `user` (escalated) |

For user-decided items, also record the alternatives that were presented and
the user's selection.

## Layer ordering rules

1. Layers follow the workspace dependency graph (leaf crates first).
2. Within a layer, fixes are ordered:
   - New public APIs or type changes (other fixes may depend on these)
   - Behavioral code changes
   - Test additions
3. Documentation fixes are always the final layer.
4. Each layer's verification commands must pass before the next layer starts.

## Scope exclusions section

Every finding that does NOT produce a fix item must appear in the exclusions
table with one of:

- **Refuted**: Phase C verification showed the finding is incorrect.
- **Deferred to Phase M**: Valid finding but intentionally left for a future
  phase (must cite the rationale).
- **Info/No action**: Finding is informational and does not require a code
  change.
- **Duplicate**: Finding is a subset of another cluster's fix.
