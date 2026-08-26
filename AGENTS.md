# AGENTS.md

This file provides guidance to Codex (Codex.ai/code) when working with code in
this repository.

## Project

`opi` is a Rust AI Agent toolkit with a terminal-first coding Agent as its
Reference Product. It reimplements selected ideas from
[earendil-works/pi](https://github.com/earendil-works/pi), but it is not a
line-by-line port or a pi compatibility layer.

## Sources of truth

Use the narrowest authoritative source for each claim:

- `docs/opi-spec.md` is the normative source for durable product direction,
  architecture invariants, admission gates, and strategic priority.
- `docs/CONTEXT.md` owns the domain language for architecture, extension
  runtime, command execution, evidence, and authority boundaries.
- `README.md`, generated `opi --help`, crate documentation, manifests, schemas,
  fixtures, and source own current product and protocol facts.
- `Cargo.toml` and crate manifests own workspace topology, versions, Rust
  edition, MSRV, and dependency declarations.
- `CHANGELOG.md` owns release history and unreleased user-visible changes.
- `docs/realign/` and `.repo/pi-0.84.1` are non-normative inward evidence;
  `docs/research/` is non-normative outward evidence.
- `.opi-impl-state.json` and `docs/snapshots/` own implementation progress and
  completed delivery history. Do not record progress in `docs/opi-spec.md`.

When documentation has an English/Chinese counterpart, update both in the same
change or state why synchronization is unnecessary. `CLAUDE.md` is a symlink to
`AGENTS.md`, so the repository keeps exactly one guidance file; edit
`AGENTS.md` and never `CLAUDE.md` directly. On a filesystem without symlink
support a fresh clone materializes `CLAUDE.md` as a text file containing
`AGENTS.md`; `scripts/opi-doc-check.py` warns on that state and fails on real
content drift.

## Design boundaries

- Keep the Agent Core small and deep. Mechanism belongs below policy; terminal,
  workflow, benchmark, and user-policy opinions do not belong in core crates.
- Dependencies point inward toward the smallest stable interface. A new public
  seam needs intrinsic state-machine value or at least two real adapters or
  consumers with shared conformance tests.
- Optional Opi workflows belong in the Extension Ecosystem. Agent-neutral
  capabilities should begin as Independent Companions with Agent-neutral
  contracts.
- Prefer Rust-native correctness: enums for closed states, explicit ownership,
  typed errors, bounded concurrency, and fail-closed validation at authority,
  protocol, adapter, and permission boundaries.
- Do not add a feature flag, trait, crate, config key, compatibility layer, or
  abstraction for hypothetical future use.

Consult the spec before answering scope or architecture questions. If a request
would contradict a normative clause, stop and ask the user whether they intend
to revise the specification.

## Workspace layout

All crates use lockstep workspace versioning and Rust edition 2024.

```text
opi-ai       (no internal deps)       - provider-neutral LLM API
opi-tui      (no internal deps)       - terminal UI components
opi-eval     (no internal deps)       - unpublished Independent Companion eval toolkit
opi-agent    -> opi-ai                - product-neutral Agent runtime
opi-protocol (no internal deps)       - command-execution protocol
opi-sandbox  -> opi-protocol          - standalone restriction SDK/CLI
opi-coding-agent -> opi-ai, opi-agent, opi-protocol, opi-tui - opi binary and coding harness
```

Internal dependencies must be declared in root `[workspace.dependencies]` and
referenced with `{ workspace = true }`. Publishable path dependencies also need
a version. Do not duplicate workspace-owned package metadata in crate manifests.

## Dependency and supply-chain safety

- Treat changes to `Cargo.toml`, `Cargo.lock`, Cargo features, `build.rs`, native
  dependencies, and proc-macro dependencies as reviewed code.
- Verify third-party APIs against the exact locked crate version or its
  authoritative documentation; do not guess from another version.
- Never hand-edit `Cargo.lock`. Regenerate it with Cargo and review unexpected
  direct and transitive changes.
- Do not introduce or broaden default features, build scripts, native code, or
  proc macros without explaining the portability, authority, and supply-chain
  impact.

## Project workflow

The canonical workflow and skill-selection policy live in
`.claude/skills/README.md` and `.claude/skills/README.zh.md`. All `opi-*` skills
require explicit user invocation; use `opi-workflow` only when the user asks for
workflow routing.

- `opi-realign` gathers pinned pi alignment evidence.
- `opi-research` gathers outward capability evidence.
- Human-led shaping updates the normative spec or a registered supplemental
  source.
- `opi-implement plan` is the admission gate for implementation work;
  `opi-implement` alone owns `.opi-impl-state.json`.
- `opi-audit`, `opi-eval`, and `opi-remediate` provide assurance and correction.
- `opi-document`, `opi-release`, and `opi-slim-tests` own their named workflows.

Do not hand-edit `.opi-impl-state.json`, create a competing implementation
ledger, or treat arbitrary `docs/superpowers/specs/` files as normative.

## Working principles

- Answer questions before editing or running commands.
- State assumptions that affect the outcome. If materially different
  interpretations remain, present them instead of choosing silently. If the
  request is still unclear, name the uncertainty and ask before proceeding.
- Prefer the simplest approach that satisfies the request. Surface a simpler
  alternative and push back when the proposed path adds avoidable complexity.
- Make the minimum change that solves the request. Every changed line must
  trace to the requested outcome; do not refactor, reformat, or harden adjacent
  code without evidence that it is required.
- Do not add unrequested features, one-use abstractions, speculative
  configurability, or error handling for states the design makes impossible.
  If a solution is much larger than its essential behavior, simplify it before
  continuing.
- Read relevant files in full before broad investigation or wide-ranging edits.
  Preserve unrelated working-tree changes and clean up only changes you caused.
- Match the surrounding style even when you would structure new code
  differently. Mention unrelated dead code instead of deleting it; remove only
  imports, variables, functions, tests, or docs made unused by your change.
- Comments and rustdoc describe current contracts, invariants, failure behavior,
  timing, ownership, and safe use. Do not preserve Phase, task, PR, or review
  history in source comments; keep history in `docs/snapshots/`, registered
  ADRs, `CHANGELOG.md`, or Git and link it only when needed.
- Ask before removing intentional behavior.
- While public crates, CLI surfaces, wire formats, and on-disk formats remain
  0.x, breaking changes are allowed only when deliberately scoped and recorded
  under `## [Unreleased]`. Do not add compatibility shims unless the owning
  specification, issue, or release policy requires them. Update affected
  callers, schemas, fixtures, tests, and documentation together. Revisit this
  policy before 1.0.
- Prefer safe Rust. Avoid `unsafe` unless there is no sound safe alternative;
  keep any required unsafe boundary narrow and justified.
- Use `thiserror` for library error types and `anyhow` only in binaries or tests.
  Match the surrounding module's established style.
- Trait objects are appropriate at crate boundaries; prefer generics within a
  crate when the concrete type is known.

## Verification

Define the success criterion before implementation and run the narrowest
sufficient check. Translate vague requests into observable outcomes: reproduce
a bug before fixing it, exercise invalid inputs when adding validation, and
compare focused tests before and after a refactor. For multi-step work, state
each item as `step -> verify: <check>` and loop until every check passes or the
remaining gap is explicit.

Match verification to the changed surface: focused tests for local behavior;
integration or snapshot tests for CLI, TUI, or model-visible output; fixture and
conformance tests for protocols, schemas, and durable formats; the documentation
check for documentation, skills, and metadata; artifact or package smoke checks
for published paths; and workspace gates for cross-crate runtime behavior, Phase
exit, CI, and release. Report the exact commands run and their results. Do not
claim that a gate passed unless that command was actually run.

For ordinary non-skill work, use
[the shared change-scope and check-selection contract](.agents/skills/_shared/references/change-scope-and-check-selection.md)
to inventory outgoing changes and select the verification union. Its authority
is `check-selection-only`; an active `opi-*` workflow keeps its own scope,
admission, audit, remediation, and release authority.

Runtime Rust or Cargo changes require the relevant `opi-implement` tier gate
when that workflow is active. Documentation, skill, and metadata-only work uses
the no-compile documentation check.

```sh
# Fast documentation contract
python scripts/opi-doc-check.py

# Focused Rust test
cargo test -p <crate> -- <test_name>

# Workspace gates (cross-crate changes, phase exit, CI, or release)
cargo fmt --check --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo test --workspace --doc
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

If a test file is created or modified, run that exact test binary/filter and
iterate until it passes. Use `opi_ai::test_support::MockProvider` and local
fixtures; tests must not call paid providers, require live credentials, or
activate live-provider behavior merely because credentials exist in the
environment. Filesystem and session tests use isolated temp directories.
Serialize tests that mutate process environment variables.

Record test impact in the handoff as `add`, `update`, `delete`, `retain`, or
`none`. Do not encode prose wording, phase status, roadmap text, changelog
tokens, or test-function names as Rust documentation tests; use
`scripts/opi-doc-check.py` for stable source-derived documentation contracts.

Keep incremental compilation enabled and use the repository's external Cargo
cache workflow. Do not run `cargo clean`, create per-session target directories,
or delete caches as task cleanup. Inspect with
`python scripts/opi-cargo-cache.py status`.

## Git safety

Assume multiple users or agents may modify this worktree concurrently.
Unrelated staged, unstaged, and untracked changes are not yours; do not move,
rewrite, stage, or delete them.

- NEVER commit unless the user asks.
- Never commit credentials or `.env` files.
- Commit only files changed for the current task. Stage them by explicit path;
  never use `git add -A` or `git add .`.
- Run `git status` before staging and again before committing.
- Use Conventional Commits. Include `fixes #<number>` or `closes #<number>`
  when a related issue exists.
- NEVER add `Co-Authored-By` trailers. Do not add
  `Co-Authored-By: Codex` or similar text.
- Never bypass hooks or rewrite shared history.

Forbidden operations include `git reset --hard`, `git checkout .`,
`git clean -fd`, `git stash`, `git commit --no-verify`, and
`git push --force`. If a rebase conflicts in a file you did not modify, abort
and ask the user. Never discard unrelated worktree changes.

Review pull requests without pulling or changing the current branch or
worktree. Prefer `gh pr view`, `gh pr diff`, `gh api`, and
`git show <ref>:<path>`. Do not use `gh pr checkout`, `git switch`, or an
equivalent branch-changing command unless the user explicitly asks. Create or
publish a branch, commit, push, PR, or release only when the user explicitly
asks. Automation-created branch names should use the `codex/` prefix unless the
user specifies another name.

## Changelog and release

Read the complete `## [Unreleased]` section before editing it. Add user-visible
changes only there, reuse existing subsections, and never create duplicate
subsection headings. Released sections are immutable. Release work must use the
explicitly invoked `opi-release <version>` workflow; never improvise crates.io
publication, tags, or rollback. Public release rollback uses a revert and tag
deletion, never a hard reset or force push.

## Editing these instructions

Keep each rule self-contained, link volatile facts to their authoritative source
instead of copying them here, and run `python scripts/opi-doc-check.py` after
every change.

## Communication

Keep responses concise and technical. No fluff and no emojis in commits,
issues, PR comments, or code. Lead with the result, then state verification and
any remaining risk or unverified work. When responding to review feedback or a
proposed diagnosis, state whether you agree, disagree, or partially agree and
cite the governing evidence before making changes.
