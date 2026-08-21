# Execution Protocol

Layer-by-layer fix execution, verification gates, and failure handling for
Phase F of `opi-remediate`.

## Layer derivation

### From cargo metadata

```bash
cargo metadata --no-deps --format-version 1
```

Parse the `packages[].dependencies` to build the internal dependency graph.
Only consider workspace-internal dependencies (those with a `path` field
pointing within the workspace).

### From Cargo manifests (fallback)

If `cargo metadata` is unavailable, parse workspace members and internal
dependencies from the root and crate `Cargo.toml` files. Manifests own the
current graph; do not copy a static crate table from `AGENTS.md` or
`CLAUDE.md` into this skill.

### Layer assignment

1. Crates with no internal dependencies = Layer 1.
2. Crates whose dependencies are all in Layer 1 = Layer 2.
3. Continue recursively.
4. Documentation = final layer (always last).

Example for the current workspace:

| Layer | Crates | Reason |
|---|---|---|
| 1 | opi-ai, opi-tui, opi-protocol | No internal deps |
| 2 | opi-agent, opi-sandbox | opi-agent -> opi-ai; opi-sandbox -> opi-protocol |
| 3 | opi-coding-agent | Depends on opi-ai, opi-agent, opi-protocol, opi-tui |
| 4 | Documentation | Always last |

If a fix spans multiple crates in different layers, split it into per-layer
parts. The substrate part goes in the earlier layer; the consumer part goes
in the later layer.

## Execution loop

For each layer, in order:

### Step 1: Apply changes

Apply all fix items assigned to this layer:
1. Code changes that introduce new public APIs or types (other fixes may
   depend on these within the same layer).
2. Code changes that modify existing behavior.
3. Test additions and modifications.

### Step 2: Verify

Run the layer's one authoritative tier-scoped verification. For one affected
crate:

```bash
scripts/opi-impl-smoke.sh scoped --crate <crate> --test <affected-test-binary>
```

On Windows PowerShell use the `.ps1` sibling. Omit `--test` only when the
affected behavior is fully covered by library tests. Use smoke `full` only for
cross-crate/workspace-tier remediation.

For documentation layers, run `python scripts/opi-doc-check.py` and verify:
- Localized counterparts are updated (EN + ZH).
- No broken internal references.
- Terminology is consistent with the code changes made in previous layers.

### Step 3: Gate

- **All pass**: Proceed to the next layer.
- **fmt fails**: Identify every path the formatter would change. Format only
  remediation-owned paths that were not dirty at the baseline, using the
  narrowest package/file target, then re-verify. If formatting would touch a
  carried-in or unowned path, stop and report the conflict; never run an
  automatic workspace-wide rewrite.
- **clippy fails**: Fix the warning, re-verify. If evidence shows the warning
  predates the remediation and is outside owned scope, stop and ask whether to
  record a scoped exclusion or expand the remediation; do not call the layer
  passing while silently continuing.
- **test fails**: Investigate. If the failure is in a test you added or
  modified, fix it. If the failure is in an existing test broken by your
  change, fix the change. If evidence shows the failure is pre-existing and
  outside owned scope, stop for the same explicit exclusion/expansion decision.

## Final verification

After all layers pass individually, run only the deduplicated union of missing
acceptance/platform gates. Do not rerun scoped gates. A cross-crate/workspace
remediation uses `scripts/opi-impl-smoke.sh full` once as its layer gate;
doctests are added only when Rust API documentation changed.

### Platform detection

Detect the host platform for smoke script selection:

| Host | Smoke script |
|---|---|
| Linux / macOS | `scripts/opi-impl-smoke.sh` |
| Windows PowerShell | `scripts/opi-impl-smoke.ps1` |
| Windows Git Bash / MSYS / WSL | `scripts/opi-impl-smoke.sh` |

The full smoke mode bundles formatting, all-target clippy, rustdoc, and
workspace tests. It has no redundant standalone build. Scoped remediation does
not escalate to full mode without a cross-crate semantic reason.

## Failure handling

### Layer verification failure

If a layer's verification commands fail after two fix attempts:

1. Record the failure (command, output, affected test/lint).
2. Stop execution.
3. Report to the user:
   - Which layer failed.
   - Which fix item caused the failure.
   - The error output.
   - Suggestion: "Fix the issue manually or re-run with adjusted scope."

Do NOT proceed to the next layer with a broken previous layer.

### Final verification failure

If workspace-wide tests fail after all layers passed individually:

1. Identify which test(s) fail.
2. Determine if the failure is caused by a cross-crate interaction from the
   remediation changes.
3. If fixable, apply the fix and re-run.
4. If not fixable in the current scope, record it and report to the user.

### Rollback

The execution protocol does not perform automatic rollback. If fixes need to
be reverted:

1. Use `git diff` to identify the changed files.
2. Present the list to the user.
3. The user decides whether to preserve the changes or authorizes a recoverable,
   file-specific reversal. The protocol does not print or execute a reversal
   command because files may contain pre-existing user work.

Never run `git reset --hard`, `git checkout .`, or `git clean -fd`.

## Progress tracking

During execution, maintain a running status:

```text
Layer 1 (opi-agent):
  [x] Fix 1.1: expose unified active-chain API
  [x] Fix 1.2: collect_metadata rootless guard
  [ ] Fix 1.3: rename BranchSummary test
  Verification: pending

Layer 2 (opi-coding-agent):
  [ ] Fix 2.1: refactor select_ordered_entries
  ...
```

Update this status as each fix item completes. Present the status to the
user at each layer gate.
