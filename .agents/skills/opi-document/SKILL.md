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

When `scope=targeted` omits `files`, use the
[shared change-scope reference](../_shared/references/change-scope-and-check-selection.md)
for candidate discovery only. It may locate outgoing documentation surfaces,
but source ownership and this skill's inputs retain documentation authority;
the diff does not expand the requested scope or authorize a repository-wide
rewrite.

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

When the requested scope contains human-facing prose, read the affected passage
as a complete proposition before editing it. Preserve its actor, behavior,
conditions, timing, modality, negative guarantees, exceptions, ownership,
failure behavior, and consequences. Classify the candidate as keep, add, trim,
restore, restructure, or defer; a smaller word count is not evidence of a
better contract.

## 2. Load only applicable constraints

Read [`references/documentation-checks.md`](references/documentation-checks.md). Use source code,
Cargo metadata, behavior tests, and the normative spec as authority. Do not
preserve narrative wording solely because an obsolete prose test once matched
it.

For human-facing prose or agent-facing instructions, also read
[`references/prose-contract.md`](references/prose-contract.md). It owns
semantic judgment, owner-first editing, current-state wording, and the
exclusions for frozen or derivative material. Load it only for the targeted
scope; a targeted scope never expands into an automatic repository-wide prose
audit.

For agent-facing instructions, also apply the project-local contract: keep the
repository as source of truth, expose completion criteria near the action, and
remove duplicated or no-op instructions. For net-new Chinese prose, use
`baoyu-translate`; edit existing Chinese mirrors surgically rather than
regenerating whole files.

## 3. Edit in mirrored units

Update each English file and its `*.zh.md` counterpart together. Preserve
identifiers, CLI flags, provider IDs, schema constants, file paths, and version
numbers exactly. Translate explanations, not code vocabulary.

`CLAUDE.md` is a symlink to `AGENTS.md`: edit `AGENTS.md` only and never
`CLAUDE.md` directly. A version bump changes documentation only after
`opi-release` has changed the workspace version.

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
- No claim that a mechanical prose or documentation check proves semantic
  quality.
- Report exact checks run and any platform/runtime evidence not exercised.
