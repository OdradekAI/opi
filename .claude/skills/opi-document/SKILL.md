---
name: opi-document
disable-model-invocation: true
description: >-
  Update opi documentation so shipped claims remain truthful and English and
  Chinese counterparts stay synchronized. Use for README/doc refreshes,
  documentation drift, localized mirrors, or the documentation phase before a
  release.
---

# Opi Document

Maintain current product documentation from authoritative source. Historical
plans and snapshots remain historical evidence; they are not current-product
truth and are not rewritten merely to make a check pass.

## Inputs

```text
scope=<full | targeted | version-bump>   # default: targeted
files=<path,...>                         # optional exact paths
version=<X.Y.Z>                          # required for version-bump
```

## 1. Establish the documentation delta

Read the affected source, `[Unreleased]` plus the latest release in
`CHANGELOG.md`, the workspace version, and the relevant current docs. Classify
candidate edits as:

- **drift**: a current claim is false, stale, or unverifiable;
- **noise**: internal milestone language that a user cannot act on;
- **gap**: an install, usage, safety, compatibility, or extension fact needed
  to use the shipped product.

Every drift item cites its source of truth. Do not infer shipped behavior from
a design plan or test name.

## 2. Load only applicable constraints

Read [`references/documentation-checks.md`](references/documentation-checks.md). Use source code,
Cargo metadata, behavior tests, and the normative spec as authority. Do not
preserve narrative wording solely because an obsolete prose test once matched
it.

For agent-facing prose, apply the project-local contract directly: keep the
repository as source of truth, expose completion criteria near the action, and
remove duplicated or no-op instructions. For net-new Chinese prose, use
`baoyu-translate`; edit existing Chinese mirrors surgically rather than
regenerating whole files.

## 3. Edit in mirrored units

Update each English file and its `*.zh.md` counterpart together. Preserve
identifiers, CLI flags, provider IDs, schema constants, file paths, and version
numbers exactly. Translate explanations, not code vocabulary.

`AGENTS.md` and `CLAUDE.md` change in lockstep except for their four intentional
Codex/Claude flavor differences. A version bump changes documentation only
after `opi-release` has changed the workspace version.

If `docs/opi-spec.md` changes, also update `docs/opi-spec.zh.md` and route the
live `.opi-impl-state.json` hash through the guarded `opi-implement` plan/reinit
flow. Never hand-edit the canonical ledger or rewrite phase snapshots.

## 4. Verify once

Run:

```text
python scripts/opi-doc-check.py
git diff --check
```

The Python check validates current workspace/crate versions, source-derived
wire-schema versions, EN/ZH counterpart presence for the root/spec and every
Cargo package under `crates/`, project-local skill frontmatter, Codex sidecars,
EN/ZH skill-index membership, local links, root-guidance lockstep, UTF-8, and
selected stale-current-claim exclusions. It does not compile Rust.

Run extra verification only when the edited surface requires it:

- Rust API/rustdoc changed: `cargo test -p <crate> --doc` and
  `RUSTDOCFLAGS="-D warnings" cargo doc -p <crate> --no-deps`.
- Generated reference changed: run its generator/check mode.
- CLI output changed: run the owning focused CLI snapshot/subprocess test.
- Normative spec changed: run the live spec-ledger guard after guarded
  reconciliation.

Do not run the workspace test suite for a prose-only edit.

## Boundaries

- No runtime source, manifest, version, commit, push, PR, or release mutation.
- No weakening behavior tests to accommodate documentation.
- No resurrection of phase-numbered prose guards.
- No full-file Chinese regeneration.
- Report exact checks run and any platform/runtime evidence not exercised.
