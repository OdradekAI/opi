---
name: opi-document
disable-model-invocation: true
description: >-
  Independently audit every maintained current-product README against shipped
  implementation by default, repair documentation drift, and keep English and
  Chinese counterparts synchronized. Use for full README truth audits,
  targeted README/doc refreshes, localized mirrors, or the documentation phase
  before a release.
---

# Opi Document

Maintain current product documentation from authoritative source. Historical
plans and snapshots remain historical evidence; they are not current-product
truth and are not rewritten merely to make a check pass.

## Inputs

```text
scope=<full | targeted | version-bump>   # default: full
files=<path,...>                         # targeted only; optional exact paths
version=<X.Y.Z>                          # required for version-bump
```

## Scope semantics

A bare invocation is `scope=full`. Do not infer `scope=targeted` from the dirty
worktree, the current branch diff, or the most recent implementation change.

- `scope=full`: read [`references/readme-audit.md`](references/readme-audit.md)
  and independently audit every maintained current-product README against its
  owning implementation evidence. A diff or prior audit may suggest risky
  claims but never narrows coverage.
- `scope=targeted`: limit semantic review to `files`, or, when `files` is
  omitted, use the
  [shared change-scope reference](../_shared/references/change-scope-and-check-selection.md)
  for candidate discovery only. Source ownership and this skill's inputs retain
  documentation authority; the diff does not expand the requested scope or
  authorize a repository-wide rewrite.
- `scope=version-bump`: update version-bearing documentation only after
  `opi-release` has changed the workspace version; `version` is required.

## 1. Establish the documentation delta

For `scope=full`, first build the complete README coverage matrix required by
`references/readme-audit.md`, then read every included README in full. For
other scopes, read the affected source and relevant current docs. In every
scope, also read `[Unreleased]` plus the latest release in `CHANGELOG.md` and
the workspace version. Classify candidate edits as:

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
exclusions for frozen or derivative material. Load it for the human-facing or
agent-facing prose selected by the active scope; a targeted scope never expands
into an automatic repository-wide prose audit.

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

## 4. Report semantic coverage

For `scope=full`, report the complete coverage matrix from
`references/readme-audit.md`, including unchanged `keep` rows, edited rows,
explicit exclusions, and every `defer` limitation. Do not claim that the full
README set is truthful while any included row is unreviewed or deferred.

For `scope=targeted` or `scope=version-bump`, report the exact inspected paths
and state explicitly that repository-wide README truth was not assessed.

## 5. Verify once

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
