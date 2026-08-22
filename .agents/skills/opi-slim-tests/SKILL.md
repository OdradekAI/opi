---
name: opi-slim-tests
description: Reduce Rust integration-test binary and link cost while preserving current behavior, architecture, and platform coverage.
disable-model-invocation: true
---

# Opi Slim Tests

Reduce integration-test **binary count**, not useful behavioral coverage. Every
top-level `tests/*.rs` is a separate Cargo target and link step; modules below a
suite directory do not create another binary.

Stop at a verified working-tree diff. Git safety and commit authorization come
from the always-loaded `AGENTS.md` / `CLAUDE.md`; this skill never stages or
commits automatically.

When an `opi-implement` live ledger or Phase snapshot is available, seed the
candidate inventory from non-null `slim_candidate` entries. A candidate is a
search hint, not deletion authority; classify its full body again.

## 1. Establish the baseline

Use `cargo metadata --no-deps --format-version 1` to inventory integration test
targets per crate. Record candidate line count, test count, `cfg`/platform,
fixtures, subprocess use, and representative timing when available. Thin files
are candidates, not automatic deletions.

Use the configured persistent external `CARGO_TARGET_DIR`; resolve it with
`python scripts/opi-cargo-cache.py resolve` when unset. Never run
`cargo clean` to prepare the measurement and never create a disposable target
directory merely to prove isolation.

## 2. Classify from full bodies

Read
[`shared-decision-and-test-stewardship.md`](../_shared/references/shared-decision-and-test-stewardship.md)
in full. Apply its primary candidate classifications to every candidate and
cite the retained Interface or execution contract.

Names are not evidence. Read candidate bodies in full and cite the retained
behavioral seam. “Documentation guard” is not automatically load-bearing.

## 3. Record test impact for the product change

Apply the shared reference's test-impact actions to each product change that
created a candidate. Preserve its replace-don't-layer ordering and proof.

This prevents a refactor from accumulating old and new tests for mutually
exclusive designs. A later phase that replaces an earlier contract should
delete or rewrite the earlier test in the same change.

## 4. Choose the smallest safe form

In preference order:

1. Delete superseded or historical prose assertions after current evidence is
   identified.
2. Parameterize true clones through one public behavior seam.
3. Move distinct cases into modules under one integration binary.
4. Move a pure unit test to the owning module under `#[cfg(test)]` when it does
   not exercise integration/subprocess behavior.
5. Keep genuine platform and helper binaries separate when Cargo execution
   boundaries are part of the test.

Shared test-only helpers belong under `tests/common/`; do not enlarge a
published API to make consolidation easier. Reconcile attributes, imports,
fixtures, environment serialization, snapshots, and test names explicitly;
never concatenate files mechanically.

## 5. Preserve the right guards

Preserve unless explicitly migrated with equivalent proof:

- the live spec-ledger CRLF-normalized SHA-256 contract;
- public protocol/schema behavior tests;
- current safety/security and persistence invariants;
- platform-only process/sandbox behavior;
- reviewed snapshots that assert current UI rendering.

Do not preserve exact narrative phrases, roadmap placeholders, phase numbers,
historical non-goals, released changelog tokens, or test function names as
current Rust assertions. Documentation contracts use
`scripts/opi-doc-check.py`; historical phase artifacts remain frozen.

## 6. Verify proportionally

Cargo metadata and complete test bodies remain the authority for candidate
classification and consolidation. After editing, use the
[shared change-scope reference](../_shared/references/change-scope-and-check-selection.md)
only to inventory the resulting surfaces and select post-change focused gates;
the diff does not decide which tests are duplicate, superseded, or load-bearing.

For each retained/consolidated binary:

```text
cargo clippy -p <crate> --test <name> -- -D warnings
cargo test -p <crate> --test <name>
```

For deleted prose-only binaries, run:

```text
python scripts/opi-doc-check.py
cargo metadata --no-deps --format-version 1
```

Compare before/after target and discovered-test counts. Run broader crate tests
only when shared helpers, library code, or runtime behavior changed. Defer
platform proof to its authoritative CI and say so explicitly. Do not run the
workspace all-target test solely because test files were reorganized.

## 7. Handoff

Report the binary count delta, classification and retained-evidence mapping,
files removed/created/modified, commands and outcomes, unproven platform
coverage, and remaining risks. Leave changes uncommitted unless separately
authorized.
